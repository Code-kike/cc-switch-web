//! Root router assembly + SPA fallback.

use axum::{
    body::Body,
    extract::DefaultBodyLimit,
    http::{header, HeaderValue, Method, StatusCode, Uri},
    middleware::from_fn,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use std::path::{Path, PathBuf};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

use super::{handlers, middleware as mw, ApiState};

/// Generous request-body ceiling for the JSON/multipart API (item 12). axum's
/// 2 MiB default rejected realistic SQLite config exports / skill / prompt
/// uploads with an opaque 500; raising the ceiling lets legit uploads succeed
/// while genuinely-oversized requests get a clean 413 from the body-limit layer.
const MAX_API_BODY_BYTES: usize = 64 * 1024 * 1024;

pub fn build_router(state: ApiState) -> Router {
    let api = api_router(state);
    Router::new()
        .nest("/api", api)
        .fallback(serve_spa_fallback)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(from_fn(mw::security_headers::add_security_headers))
                // Unauthenticated Web API: keep only the browser same-origin
                // intent guard for state-changing /api/* requests. SPA assets
                // and /api/health stay public.
                .layer(from_fn(mw::intent::require_same_origin_intent)),
        )
}

/// Minimal router served when the database is too new to initialize (M7).
///
/// Building the full API needs a compatible DB to construct `ApiState`, so on a
/// `db_version_too_new` failure the server would otherwise `return Err` and exit
/// before ever binding a listener — leaving the purpose-built `DatabaseUpgrade`
/// recovery screen (which the SPA renders from the `get_init_error` probe)
/// unreachable and systemd in a connection-refused restart loop. This degraded
/// router requires no `ApiState`/DB: it binds normally and serves ONLY the
/// init-error probe, `/api/health`, and the SPA static assets, so the browser
/// can display the headless recovery UI. Every other `/api/*` path 404s.
pub fn build_degraded_router() -> Router {
    let api = Router::new()
        .merge(handlers::health::router())
        // The SPA bootstrap probes this (POST per web-commands.ts); also accept
        // GET so a direct browser hit works.
        .route(
            "/system/get_init_error",
            post(degraded_get_init_error).get(degraded_get_init_error),
        )
        .layer(mw::cors::layer())
        .fallback(api_404);
    Router::new()
        .nest("/api", api)
        .fallback(serve_spa_fallback)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(from_fn(mw::security_headers::add_security_headers))
                .layer(from_fn(mw::intent::require_same_origin_intent)),
        )
}

async fn degraded_get_init_error() -> Json<Option<crate::init_status::InitErrorPayload>> {
    Json(crate::init_status::get_init_error())
}

fn api_router(state: ApiState) -> Router {
    Router::new()
        .merge(handlers::health::router())
        .merge(handlers::system::router(state.clone()))
        .merge(handlers::auth::router(state.clone()))
        .merge(handlers::backups::router(state.clone()))
        .merge(handlers::config::router(state.clone()))
        .merge(handlers::deeplink::router(state.clone()))
        .merge(handlers::env::router(state.clone()))
        .merge(handlers::failover::router(state.clone()))
        .merge(handlers::global_proxy::router(state.clone()))
        .merge(handlers::hermes::router(state.clone()))
        .merge(handlers::mcp::router(state.clone()))
        .merge(handlers::omo::router(state.clone()))
        .merge(handlers::openclaw::router(state.clone()))
        .merge(handlers::parity::router(state.clone()))
        .merge(handlers::prompts::router(state.clone()))
        .merge(handlers::providers::router(state.clone()))
        .merge(handlers::proxy::router(state.clone()))
        .merge(handlers::s3::router(state.clone()))
        .merge(handlers::sessions::router(state.clone()))
        .merge(handlers::settings::router(state.clone()))
        .merge(handlers::skills::router(state.clone()))
        .merge(handlers::subscription::router(state.clone()))
        .merge(handlers::usage::router(state.clone()))
        .merge(handlers::webdav::router(state.clone()))
        .merge(handlers::workspace::router(state.clone()))
        .layer(mw::cors::layer())
        // Raise the body ceiling above axum's 2 MiB default so real config /
        // skill / prompt uploads succeed; oversized requests get a clean 413 (item 12).
        .layer(DefaultBodyLimit::max(MAX_API_BODY_BYTES))
        .fallback(api_404)
}

async fn api_404(uri: Uri) -> Response {
    (
        StatusCode::NOT_FOUND,
        [(header::CONTENT_TYPE, "application/json")],
        format!(
            "{{\"code\":\"NOT_FOUND\",\"message\":\"No API route for {}\"}}",
            uri.path()
        ),
    )
        .into_response()
}

/// SPA fallback: every non-API GET returns index.html so client-side routing
/// works on direct URL hits / refreshes (Round 5 P0-3).
///
/// Assets are served from disk (`dist-web/`, see `read_dist_web_file`); the
/// previously-planned rust-embed integration was dropped and the dep removed (item 16).
async fn serve_spa_fallback(method: Method, uri: Uri) -> Response {
    if method != Method::GET && method != Method::HEAD {
        return (StatusCode::METHOD_NOT_ALLOWED, "method not allowed").into_response();
    }

    try_serve_dist_web_asset(uri.path()).await
}

async fn try_serve_dist_web_asset(path: &str) -> Response {
    let rel = path.trim_start_matches('/');
    let dist_root = dist_web_root();

    if rel.is_empty() {
        return read_dist_web_file(&dist_root.join("index.html"))
            .await
            .unwrap_or_else(spa_placeholder_response);
    }

    // Path-traversal guard (audit C1): `uri.path()` is NOT dot-segment-normalized,
    // so a request like `/../../../../etc/passwd` would otherwise escape `dist_root`
    // via `Path::join` and be read+served. Only read the candidate when every path
    // component is a plain file/dir name (no `..`, absolute root, or Windows prefix).
    // Verified exploit before this guard: `curl --path-as-is .../../../../etc/hostname`
    // returned the host's /etc/hostname.
    if is_safe_relative_asset(rel) {
        let candidate = dist_root.join(rel);
        if let Some(resp) = read_dist_web_file(&candidate).await {
            return resp;
        }
    }

    if !is_static_asset_path(rel) {
        return read_dist_web_file(&dist_root.join("index.html"))
            .await
            .unwrap_or_else(spa_placeholder_response);
    }

    (StatusCode::NOT_FOUND, "asset not found").into_response()
}

/// True only when `rel` is a safe relative asset path: every component is a plain
/// file/dir name or `.` — no `..` (ParentDir), absolute root, or Windows prefix.
/// Path-traversal guard for the hand-rolled static server (audit C1). `tower_http::
/// ServeDir` has equivalent protection, but keeping the hand-rolled branch lets the
/// SPA index fallback handle extensionless client routes uniformly.
fn is_safe_relative_asset(rel: &str) -> bool {
    use std::path::Component;
    !rel.is_empty()
        && Path::new(rel)
            .components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

fn is_static_asset_path(path: &str) -> bool {
    let Some(extension) = Path::new(path).extension().and_then(|ext| ext.to_str()) else {
        return false;
    };

    matches!(
        extension.to_ascii_lowercase().as_str(),
        "avif"
            | "bmp"
            | "css"
            | "eot"
            | "gif"
            | "html"
            | "ico"
            | "jpeg"
            | "jpg"
            | "js"
            | "json"
            | "map"
            | "mjs"
            | "otf"
            | "png"
            | "svg"
            | "ttf"
            | "txt"
            | "wasm"
            | "webmanifest"
            | "webp"
            | "woff"
            | "woff2"
            | "xml"
    )
}

fn spa_placeholder_response() -> Response {
    let body = "<!DOCTYPE html><html><head><title>cc-switch-web</title></head>\
<body><div id=\"root\"></div><script>\
console.warn('SPA assets not found — run `pnpm build:web` from repo root.');\
</script></body></html>";

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"))
        .body(Body::from(body))
        .unwrap_or_else(|_| {
            (StatusCode::INTERNAL_SERVER_ERROR, "fallback build failed").into_response()
        })
}

fn dist_web_root() -> PathBuf {
    if let Ok(path) = std::env::var("CC_SWITCH_WEB_DIST_DIR") {
        return PathBuf::from(path);
    }

    let mut candidates = vec![PathBuf::from("dist-web"), PathBuf::from("../dist-web")];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            candidates.push(exe_dir.join("dist-web"));
            candidates.push(exe_dir.join("../../../dist-web"));
        }
    }

    candidates
        .into_iter()
        .find(|path| path.join("index.html").is_file())
        .unwrap_or_else(|| PathBuf::from("dist-web"))
}

async fn read_dist_web_file(path: &Path) -> Option<Response> {
    let bytes = match tokio::fs::read(path).await {
        Ok(bytes) => bytes,
        Err(_) => return None,
    };

    let mime = mime_guess::from_path(path).first_or_octet_stream();
    Some(
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime.as_ref())
            .header(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"))
            .body(Body::from(bytes))
            .unwrap_or_else(|_| {
                (StatusCode::INTERNAL_SERVER_ERROR, "asset response failed").into_response()
            }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn degraded_router_serves_init_error_probe() {
        use http_body_util::BodyExt;
        use tower::ServiceExt;

        crate::init_status::set_init_error(crate::init_status::InitErrorPayload {
            path: "/tmp/cc-switch.db".to_string(),
            error: "database version is too new".to_string(),
            kind: Some("db_version_too_new".to_string()),
            db_version: Some(999),
            supported_version: Some(1),
        });

        let router = build_degraded_router();
        // Direct client (no Origin / Sec-Fetch-*) passes the same-origin intent
        // guard, mirroring the SPA bootstrap probe.
        let response = router
            .oneshot(
                axum::http::Request::builder()
                    .method(Method::POST)
                    .uri("/api/system/get_init_error")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("degraded router response");

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        let body = String::from_utf8_lossy(&bytes);
        assert!(
            body.contains("db_version_too_new"),
            "degraded init-error probe should surface the recovery payload: {body}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn missing_asset_returns_404_instead_of_index() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(
            temp_dir.path().join("index.html"),
            "<div id=\"root\"></div>",
        )
        .expect("write index");
        std::env::set_var("CC_SWITCH_WEB_DIST_DIR", temp_dir.path());

        let response = try_serve_dist_web_asset("/assets/missing.js").await;

        std::env::remove_var("CC_SWITCH_WEB_DIST_DIR");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    #[serial]
    async fn route_path_without_extension_falls_back_to_index() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(
            temp_dir.path().join("index.html"),
            "<div id=\"root\"></div>",
        )
        .expect("write index");
        std::env::set_var("CC_SWITCH_WEB_DIST_DIR", temp_dir.path());

        let response = try_serve_dist_web_asset("/settings/providers").await;

        std::env::remove_var("CC_SWITCH_WEB_DIST_DIR");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    #[serial]
    async fn dotted_route_without_asset_extension_falls_back_to_index() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(
            temp_dir.path().join("index.html"),
            "<div id=\"root\"></div>",
        )
        .expect("write index");
        std::env::set_var("CC_SWITCH_WEB_DIST_DIR", temp_dir.path());

        let response = try_serve_dist_web_asset("/providers/openai.com").await;

        std::env::remove_var("CC_SWITCH_WEB_DIST_DIR");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn is_safe_relative_asset_rejects_traversal() {
        // Legit relative asset paths (post leading-slash trim) are allowed.
        assert!(is_safe_relative_asset("assets/index-abc123.js"));
        assert!(is_safe_relative_asset("favicon.ico"));
        assert!(is_safe_relative_asset("./assets/app.css"));
        // Traversal / escape attempts are rejected (audit C1).
        assert!(!is_safe_relative_asset("../../../../etc/passwd"));
        assert!(!is_safe_relative_asset("assets/../../../etc/passwd"));
        assert!(!is_safe_relative_asset("/etc/passwd")); // RootDir component
        assert!(!is_safe_relative_asset("")); // empty handled by caller, but reject here too
    }

    #[tokio::test]
    #[serial]
    async fn path_traversal_does_not_read_outside_dist_root() {
        // Regression for audit C1: before the guard, this returned the host's
        // /etc/hostname. It must now fall back to the SPA index (the payload has no
        // asset extension), never the traversed host file.
        let temp_dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(
            temp_dir.path().join("index.html"),
            "<div id=\"root\">SPA</div>",
        )
        .expect("write index");
        std::env::set_var("CC_SWITCH_WEB_DIST_DIR", temp_dir.path());

        let response = try_serve_dist_web_asset("/../../../../../../etc/hostname").await;
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let text = String::from_utf8_lossy(&body);

        std::env::remove_var("CC_SWITCH_WEB_DIST_DIR");

        assert_eq!(status, StatusCode::OK);
        assert!(
            text.contains("id=\"root\""),
            "traversal must fall back to SPA index, got: {text}"
        );
        // /etc/hostname / /etc/passwd content must never leak through.
        assert!(
            !text.contains("root:"),
            "must not leak /etc/passwd-style content"
        );
    }

    #[tokio::test]
    async fn body_limit_rejects_oversize_with_413_and_allows_under_limit() {
        use axum::{body::Bytes, routing::post};
        use http::Request;
        use tower::ServiceExt;

        // Mirror the production api_router body-limit layer with a small cap so the
        // boundary is cheap to exercise; the handler consumes the full body.
        const LIMIT: usize = 1024;
        let app = Router::new()
            .route("/u", post(|_b: Bytes| async { StatusCode::OK }))
            .layer(DefaultBodyLimit::max(LIMIT));

        let under = app
            .clone()
            .oneshot(
                Request::post("/u")
                    .header(header::CONTENT_LENGTH, "16")
                    .body(Body::from(vec![b'x'; 16]))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(under.status(), StatusCode::OK);

        let too_big = vec![b'x'; LIMIT + 1];
        let over = app
            .oneshot(
                Request::post("/u")
                    .header(header::CONTENT_LENGTH, too_big.len().to_string())
                    .body(Body::from(too_big))
                    .unwrap(),
            )
            .await
            .unwrap();
        // Clean 413 from the body-limit layer, not an opaque 500 (item 12).
        assert_eq!(over.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[test]
    fn api_body_ceiling_exceeds_axum_default() {
        // Regression guard: the ceiling must exceed axum's 2 MiB default — the bug
        // was real config exports (> 2 MiB) failing with an opaque 500 (item 12).
        assert!(MAX_API_BODY_BYTES > 2 * 1024 * 1024);
    }
}
