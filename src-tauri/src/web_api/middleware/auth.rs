//! Authentication middleware — HTTP Basic Auth for the Web API (audit fix C2).
//!
//! The Web API can read provider secrets, import/export the SQLite config, and
//! toggle proxy takeover. On a non-loopback bind (e.g. the Tailscale deployment)
//! it must not be reachable unauthenticated. This middleware enforces HTTP Basic
//! Auth on `/api/*` (except `/api/health`) whenever credentials are configured:
//!
//!   `CC_SWITCH_WEB_AUTH_PASSWORD`  — required to ENABLE auth
//!   `CC_SWITCH_WEB_AUTH_USER`      — optional, default `cc-switch`
//!
//! When no password is configured the middleware is a no-op (loopback-only dev
//! posture). `examples/server.rs` REFUSES to bind a non-loopback address unless a
//! password is set, so the unauthenticated path can only occur on loopback.
//!
//! Static SPA assets stay public: the browser loads the app, then its `/api` calls
//! trigger the native Basic-Auth prompt — no frontend changes required. Basic Auth
//! over plain HTTP is acceptable here because the transport is the encrypted
//! Tailscale tunnel; front with `tailscale serve`/TLS for stronger posture.

use std::sync::OnceLock;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use base64::Engine as _;

struct WebAuth {
    user: String,
    password: String,
}

/// Resolved-once web Basic Auth credentials. `None` ⇒ auth disabled (no password).
fn web_auth() -> Option<&'static WebAuth> {
    static AUTH: OnceLock<Option<WebAuth>> = OnceLock::new();
    AUTH.get_or_init(|| {
        let password = std::env::var("CC_SWITCH_WEB_AUTH_PASSWORD")
            .ok()
            .filter(|p| !p.is_empty())?;
        let user = std::env::var("CC_SWITCH_WEB_AUTH_USER")
            .ok()
            .filter(|u| !u.is_empty())
            .unwrap_or_else(|| "cc-switch".to_string());
        Some(WebAuth { user, password })
    })
    .as_ref()
}

/// True when web Basic Auth credentials are configured. `examples/server.rs` uses
/// this to refuse an unauthenticated non-loopback bind.
pub fn is_configured() -> bool {
    web_auth().is_some()
}

/// Constant-time byte comparison to avoid credential timing oracles. (Length is
/// not secret-constant, but that leaks only credential length.)
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Validate an `Authorization: Basic <base64(user:pass)>` header against `expected`.
fn credentials_match(header_value: Option<&header::HeaderValue>, expected: &WebAuth) -> bool {
    let Some(value) = header_value.and_then(|v| v.to_str().ok()) else {
        return false;
    };
    let Some(token) = value
        .strip_prefix("Basic ")
        .or_else(|| value.strip_prefix("basic "))
    else {
        return false;
    };
    let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(token.trim()) else {
        return false;
    };
    let Some(colon) = decoded.iter().position(|&b| b == b':') else {
        return false;
    };
    let user = &decoded[..colon];
    let pass = &decoded[colon + 1..];
    // Evaluate both unconditionally so a fast user mismatch can't short-circuit the
    // password comparison into a timing oracle.
    let user_ok = ct_eq(user, expected.user.as_bytes());
    let pass_ok = ct_eq(pass, expected.password.as_bytes());
    user_ok & pass_ok
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [(
            header::WWW_AUTHENTICATE,
            "Basic realm=\"cc-switch-web\", charset=\"UTF-8\"",
        )],
        "unauthorized",
    )
        .into_response()
}

fn forbidden(reason: &'static str) -> Response {
    (StatusCode::FORBIDDEN, reason).into_response()
}

/// Whether `method` is state-changing (subject to the FIX 5 cross-site intent
/// check). GET/HEAD/OPTIONS are safe and exempt.
fn is_state_changing(method: &Method) -> bool {
    matches!(
        *method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    )
}

/// FIX 5 (CSRF defense for no-body POSTs): cached Basic credentials are
/// auto-attached cross-site like cookies, and CORS does not block *simple*
/// cross-origin requests from executing. For state-changing `/api/*` methods we
/// therefore require a same-origin request intent: either `Sec-Fetch-Site` is
/// `same-origin`/`none`, or — when an `Origin` header is present — its host
/// matches the request `Host`. This costs no token plumbing and does not affect
/// the same-origin SPA (its fetches are same-origin) nor the browser's Basic
/// auth replay. Returns Ok(()) when the request intent is acceptable.
fn check_same_origin_intent(req: &Request<Body>) -> Result<(), Response> {
    let headers = req.headers();

    // Prefer the fetch-metadata signal when the browser sends it.
    if let Some(site) = headers
        .get("sec-fetch-site")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
    {
        return match site {
            // same-origin: SPA fetch; none: user-initiated (address bar, bookmark).
            "same-origin" | "none" => Ok(()),
            // same-site / cross-site: a sibling/foreign page initiated this.
            _ => Err(forbidden("cross-site request rejected")),
        };
    }

    // No fetch-metadata (older browser / non-browser client): fall back to an
    // Origin↔Host host comparison. Absent Origin (e.g. curl, server-to-server)
    // is allowed — the Basic credential is the control there, not CSRF.
    let Some(origin) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) else {
        return Ok(());
    };
    // "null" Origin (sandboxed iframe / opaque origin) is not same-origin.
    if origin.eq_ignore_ascii_case("null") {
        return Err(forbidden("cross-site request rejected"));
    }
    let origin_host = url::Url::parse(origin)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_ascii_lowercase()));
    let request_host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        // Host header may include a port; compare host portion only.
        .map(|h| h.split(':').next().unwrap_or(h).to_ascii_lowercase());

    match (origin_host, request_host) {
        (Some(o), Some(h)) if o == h => Ok(()),
        _ => Err(forbidden("cross-site request rejected")),
    }
}

/// Enforce Basic Auth on `/api/*` (except `/api/health`) when credentials are
/// configured. Static SPA assets and the health probe stay public.
///
/// FIX 7 (CORS preflight): an uncredentialed `OPTIONS` + `Origin` preflight is
/// passed through to the inner `CorsLayer` so `CORS_ALLOW_ORIGINS` can actually
/// negotiate; the real cross-origin GET/POST that follows still carries
/// credentials and is auth-checked (+ the FIX 5 intent check for mutations).
///
/// FIX 5 (CSRF): state-changing `/api/*` methods additionally require a
/// same-origin request intent (see `check_same_origin_intent`).
pub async fn require_auth(req: Request<Body>, next: Next) -> Response {
    let path = req.uri().path();
    let is_public = !path.starts_with("/api/") || path == "/api/health";
    if is_public {
        return next.run(req).await;
    }

    // FIX 7: let the CORS preflight reach the CorsLayer (it carries no
    // credentials and changes no state; the actual request is still gated).
    if req.method() == Method::OPTIONS && req.headers().contains_key(header::ORIGIN) {
        return next.run(req).await;
    }

    // FIX 5: cross-site intent check for mutating methods, BEFORE auth so a
    // forged cross-site POST with cached creds is rejected with 403.
    if is_state_changing(req.method()) {
        if let Err(resp) = check_same_origin_intent(&req) {
            return resp;
        }
    }

    match web_auth() {
        // No credentials configured: loopback-only dev posture (server.rs guarantees
        // a non-loopback bind cannot reach here without credentials).
        None => next.run(req).await,
        Some(expected) => {
            if credentials_match(req.headers().get(header::AUTHORIZATION), expected) {
                next.run(req).await
            } else {
                unauthorized()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn basic(user: &str, pass: &str) -> HeaderValue {
        let token =
            base64::engine::general_purpose::STANDARD.encode(format!("{user}:{pass}").as_bytes());
        HeaderValue::from_str(&format!("Basic {token}")).unwrap()
    }

    #[test]
    fn ct_eq_basic() {
        assert!(ct_eq(b"abc", b"abc"));
        assert!(!ct_eq(b"abc", b"abd"));
        assert!(!ct_eq(b"abc", b"abcd"));
    }

    #[test]
    fn credentials_match_accepts_correct_and_rejects_wrong() {
        let expected = WebAuth {
            user: "cc-switch".to_string(),
            password: "s3cr3t".to_string(),
        };
        assert!(credentials_match(
            Some(&basic("cc-switch", "s3cr3t")),
            &expected
        ));
        assert!(!credentials_match(
            Some(&basic("cc-switch", "wrong")),
            &expected
        ));
        assert!(!credentials_match(
            Some(&basic("other", "s3cr3t")),
            &expected
        ));
        assert!(!credentials_match(None, &expected));
        assert!(!credentials_match(
            Some(&HeaderValue::from_static("Bearer x")),
            &expected
        ));
        assert!(!credentials_match(
            Some(&HeaderValue::from_static("Basic !!!notbase64")),
            &expected
        ));
    }

    fn req_with(method: Method, path: &str, headers: &[(&'static str, &str)]) -> Request<Body> {
        let mut builder = Request::builder().method(method).uri(path);
        for (k, v) in headers {
            builder = builder.header(*k, *v);
        }
        builder.body(Body::empty()).unwrap()
    }

    #[test]
    fn is_state_changing_classifies_methods() {
        assert!(is_state_changing(&Method::POST));
        assert!(is_state_changing(&Method::PUT));
        assert!(is_state_changing(&Method::PATCH));
        assert!(is_state_changing(&Method::DELETE));
        assert!(!is_state_changing(&Method::GET));
        assert!(!is_state_changing(&Method::HEAD));
        assert!(!is_state_changing(&Method::OPTIONS));
    }

    #[test]
    fn cross_site_origin_post_is_rejected() {
        // FIX 5: a forged cross-site POST (Origin host != Host) → 403.
        let req = req_with(
            Method::POST,
            "/api/proxy/start-proxy-server",
            &[
                ("origin", "http://evil.example.com"),
                ("host", "100.75.197.120:3010"),
            ],
        );
        assert!(check_same_origin_intent(&req).is_err());
    }

    #[test]
    fn cross_site_fetch_metadata_post_is_rejected() {
        // FIX 5: Sec-Fetch-Site: cross-site → 403 even without an Origin host check.
        let req = req_with(
            Method::POST,
            "/api/proxy/start-proxy-server",
            &[("sec-fetch-site", "cross-site")],
        );
        assert!(check_same_origin_intent(&req).is_err());
    }

    #[test]
    fn same_origin_post_passes_intent_check() {
        // Matching Origin/Host host → pass.
        let req = req_with(
            Method::POST,
            "/api/proxy/start-proxy-server",
            &[
                ("origin", "http://100.75.197.120:3010"),
                ("host", "100.75.197.120:3010"),
            ],
        );
        assert!(check_same_origin_intent(&req).is_ok());

        // Sec-Fetch-Site: same-origin → pass.
        let req = req_with(
            Method::POST,
            "/api/proxy/start-proxy-server",
            &[("sec-fetch-site", "same-origin")],
        );
        assert!(check_same_origin_intent(&req).is_ok());

        // No Origin + no fetch-metadata (curl / server-to-server) → pass (the
        // Basic credential is the control there, not CSRF).
        let req = req_with(Method::POST, "/api/proxy/start-proxy-server", &[]);
        assert!(check_same_origin_intent(&req).is_ok());

        // Sec-Fetch-Site: none (address bar / bookmark) → pass.
        let req = req_with(
            Method::POST,
            "/api/proxy/start-proxy-server",
            &[("sec-fetch-site", "none")],
        );
        assert!(check_same_origin_intent(&req).is_ok());
    }

    #[test]
    fn opaque_null_origin_post_is_rejected() {
        let req = req_with(
            Method::POST,
            "/api/proxy/start-proxy-server",
            &[("origin", "null")],
        );
        assert!(check_same_origin_intent(&req).is_err());
    }
}
