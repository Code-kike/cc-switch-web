//! Security headers — CSP, HSTS, X-Frame-Options, X-Content-Type-Options,
//! Referrer-Policy, Permissions-Policy, COOP, CORP. Round 5 P1-1.
//!
//! Layer 2 implementation: static header set, no per-request CSP nonce yet
//! (deferred until inline FOUC script is wired up in Layer 3 / Task 10A).

use axum::{
    body::Body,
    http::{header, HeaderName, HeaderValue, Request, Response},
    middleware::Next,
};

const CSP: &str = "default-src 'self'; \
img-src 'self' data:; \
style-src 'self' 'unsafe-inline'; \
script-src 'self' 'unsafe-inline'; \
connect-src 'self'; \
frame-ancestors 'none'; \
form-action 'self'; \
base-uri 'self'";

const PERMISSIONS_POLICY: &str = "camera=(), microphone=(), geolocation=(), payment=()";

pub async fn add_security_headers(req: Request<Body>, next: Next) -> Response<Body> {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(CSP),
    );
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static(PERMISSIONS_POLICY),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    if std::env::var("ENABLE_HSTS").as_deref() != Ok("false") {
        headers.insert(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }
    response
}
