//! Root router assembly + SPA fallback.

use axum::{
    body::Body,
    http::{header, HeaderValue, Method, StatusCode, Uri},
    middleware::from_fn,
    response::{IntoResponse, Response},
    Router,
};
use std::path::{Path, PathBuf};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

use super::{handlers, middleware as mw, ApiState};

pub fn build_router(state: ApiState) -> Router {
    let api = api_router(state);
    Router::new()
        .nest("/api", api)
        .fallback(serve_spa_fallback)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(from_fn(mw::security_headers::add_security_headers)),
        )
}

fn api_router(state: ApiState) -> Router {
    Router::new()
        .merge(handlers::health::router())
        .merge(handlers::system::router(state.clone()))
        .merge(handlers::auth::router(state.clone()))
        .merge(handlers::backups::router(state.clone()))
        .merge(handlers::config::router(state.clone()))
        .merge(handlers::copilot::router(state.clone()))
        .merge(handlers::deeplink::router(state.clone()))
        .merge(handlers::env::router(state.clone()))
        .merge(handlers::failover::router(state.clone()))
        .merge(handlers::global_proxy::router(state.clone()))
        .merge(handlers::hermes::router(state.clone()))
        .merge(handlers::mcp::router(state.clone()))
        .merge(handlers::model_fetch::router(state.clone()))
        .merge(handlers::model_test::router(state.clone()))
        .merge(handlers::omo::router(state.clone()))
        .merge(handlers::openclaw::router(state.clone()))
        .merge(handlers::parity::router(state.clone()))
        .merge(handlers::prompts::router(state.clone()))
        .merge(handlers::providers::router(state.clone()))
        .merge(handlers::proxy::router(state.clone()))
        .merge(handlers::sessions::router(state.clone()))
        .merge(handlers::settings::router(state.clone()))
        .merge(handlers::skills::router(state.clone()))
        .merge(handlers::subscription::router(state.clone()))
        .merge(handlers::universal::router(state.clone()))
        .merge(handlers::usage::router(state.clone()))
        .merge(handlers::vscode::router(state.clone()))
        .merge(handlers::webdav::router(state.clone()))
        .merge(handlers::workspace::router(state.clone()))
        .layer(mw::cors::layer())
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
/// Layer 2 / Task 4 — placeholder; rust-embed integration happens once
/// `dist-web/` is built.
async fn serve_spa_fallback(method: Method, uri: Uri) -> Response {
    if method != Method::GET && method != Method::HEAD {
        return (StatusCode::METHOD_NOT_ALLOWED, "method not allowed").into_response();
    }

    if let Some(response) = try_serve_dist_web_asset(uri.path()).await {
        return response;
    }

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

async fn try_serve_dist_web_asset(path: &str) -> Option<Response> {
    let rel = path.trim_start_matches('/');
    let dist_root = dist_web_root();

    if rel.is_empty() || !rel.contains('.') {
        return read_dist_web_file(&dist_root.join("index.html")).await;
    }

    let candidate = dist_root.join(rel);
    if let Some(resp) = read_dist_web_file(&candidate).await {
        return Some(resp);
    }

    read_dist_web_file(&dist_root.join("index.html")).await
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
