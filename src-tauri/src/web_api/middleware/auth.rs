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
    http::{header, Request, StatusCode},
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

/// Enforce Basic Auth on `/api/*` (except `/api/health`) when credentials are
/// configured. Static SPA assets and the health probe stay public.
pub async fn require_auth(req: Request<Body>, next: Next) -> Response {
    let path = req.uri().path();
    let is_public = !path.starts_with("/api/") || path == "/api/health";
    if is_public {
        return next.run(req).await;
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
}
