//! CORS middleware — same-origin by default; explicit allow-list via
//! `CORS_ALLOW_ORIGINS` env var. Round 4 P1-2: LAN auto-allow removed
//! to avoid CSRF attack surface when running cookie sessions.

use axum::http::{header, HeaderName, HeaderValue, Method};
use tower_http::cors::{AllowOrigin, CorsLayer};

const ENV_ALLOW: &str = "CORS_ALLOW_ORIGINS";

#[allow(dead_code)]
pub fn layer() -> CorsLayer {
    let mut cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            HeaderName::from_static("x-csrf-token"),
            HeaderName::from_static("x-request-id"),
        ])
        .allow_credentials(true);

    if let Ok(raw) = std::env::var(ENV_ALLOW) {
        let origins: Vec<HeaderValue> = raw
            .split(',')
            .filter_map(|s| HeaderValue::from_str(s.trim()).ok())
            .collect();
        if !origins.is_empty() {
            cors = cors.allow_origin(AllowOrigin::list(origins));
        }
    }
    // Default: no allow_origin set → same-origin only (browser blocks others).
    cors
}
