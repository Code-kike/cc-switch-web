use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::{json, Value};
use url::{Host, Url};

pub type ApiResult<T> = Result<Json<T>, ApiError>;

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
    details: Option<Value>,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(
        status: StatusCode,
        code: &'static str,
        message: impl Into<String>,
        details: Value,
    ) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            details: Some(details),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "BAD_REQUEST", message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", message)
    }

    pub fn not_supported(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_IMPLEMENTED, "WEB_NOT_SUPPORTED", message)
    }

    pub fn desktop_only(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_IMPLEMENTED, "WEB_DESKTOP_ONLY", message)
    }

    pub fn upload_required(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "WEB_UPLOAD_REQUIRED", message)
    }

    pub fn from_anyhow<E: std::fmt::Display>(error: E) -> Self {
        Self::internal(error.to_string())
    }

    /// Map an axum multipart error to the status it actually warrants instead of
    /// a blanket 500. Critically this preserves `413 Payload Too Large` when a
    /// body exceeds the configured `DefaultBodyLimit` (item 12) — `from_anyhow`
    /// would otherwise flatten an over-limit upload into an opaque 500.
    pub fn from_multipart(error: axum::extract::multipart::MultipartError) -> Self {
        let status = error.status();
        let code = if status == StatusCode::PAYLOAD_TOO_LARGE {
            "WEB_PAYLOAD_TOO_LARGE"
        } else {
            "WEB_MULTIPART_ERROR"
        };
        Self::new(status, code, error.body_text())
    }

    pub fn from_service_message(message: String) -> Self {
        if message.contains("unavailable in web-server mode")
            || message.contains("not supported in this runtime")
        {
            return Self::not_supported(message);
        }
        Self::bad_request(message)
    }

    /// Borrow the human-readable error message (used when an SSRF rejection is
    /// surfaced inline as a per-URL result rather than a top-level error).
    pub fn message(&self) -> String {
        self.message.clone()
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = json!({
            "code": self.code,
            "message": self.message,
            "details": self.details,
        });
        (self.status, Json(body)).into_response()
    }
}

pub fn json_ok<T: Serialize>(value: T) -> Json<T> {
    Json(value)
}

pub async fn web_not_supported() -> Result<Json<Value>, ApiError> {
    Err(ApiError::not_supported(
        "This command is not implemented in Web mode yet",
    ))
}

pub async fn web_desktop_only() -> Result<Json<Value>, ApiError> {
    Err(ApiError::desktop_only(
        "This desktop-only command is not available in Web mode",
    ))
}

pub async fn web_upload_required() -> Result<Json<Value>, ApiError> {
    Err(ApiError::upload_required(
        "Use the Web upload/download endpoint for this file operation",
    ))
}

/// Env var holding a comma-separated allow-list of hostnames that bypass the
/// SSRF guard (e.g. an internal endpoint the operator deliberately exposes).
const SSRF_ALLOW_ENV: &str = "CC_SWITCH_WEB_SSRF_ALLOW";

/// Returns true if `host` is present (case-insensitively) in the
/// `CC_SWITCH_WEB_SSRF_ALLOW` env allow-list.
fn ssrf_host_allowed(host: &str) -> bool {
    match std::env::var(SSRF_ALLOW_ENV) {
        Ok(list) => list
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .any(|entry| entry.eq_ignore_ascii_case(host)),
        Err(_) => false,
    }
}

/// Returns true if the IPv4 address falls in a range the web server must never
/// dial outbound: loopback (127.0.0.0/8), link-local (169.254.0.0/16) or any
/// private range (10/8, 172.16/12, 192.168/16).
fn is_blocked_ipv4(ip: &Ipv4Addr) -> bool {
    ip.is_loopback() || ip.is_link_local() || ip.is_private()
}

/// Returns true if the IPv6 address falls in a blocked range: loopback (::1),
/// link-local (fe80::/10) or unique-local / ULA (fc00::/7). The `is_*` helpers
/// for the latter two are unstable on stable Rust, so they are checked manually.
fn is_blocked_ipv6(ip: &Ipv6Addr) -> bool {
    if ip.is_loopback() {
        return true;
    }
    let segments = ip.segments();
    // Link-local fe80::/10
    if (segments[0] & 0xffc0) == 0xfe80 {
        return true;
    }
    // Unique-local / ULA fc00::/7
    if (segments[0] & 0xfe00) == 0xfc00 {
        return true;
    }
    false
}

/// Returns true if the IP address is in any range disallowed for outbound
/// requests from the web server. IPv4-mapped IPv6 addresses are unwrapped so a
/// mapped private/loopback v4 cannot slip through the v6 path.
fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_blocked_ipv4(&v4),
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => is_blocked_ipv4(&v4),
            None => is_blocked_ipv6(&v6),
        },
    }
}

/// SSRF guard for user-supplied outbound URLs reaching shared services.
/// Parses `raw`, rejects non-http(s) schemes, and blocks targets that resolve
/// to loopback / link-local / private / ULA addresses. Hostnames are resolved
/// via DNS and rejected if ANY resolved IP is blocked. A hostname listed in the
/// `CC_SWITCH_WEB_SSRF_ALLOW` env var bypasses these checks.
///
/// Note: this guard is web-server-only; the desktop runtime keeps dialing local
/// endpoints through the same shared services without this restriction.
///
/// Async (audit F11): DNS resolution uses non-blocking `tokio::net::lookup_host`
/// rather than the blocking `std` resolver, so it never stalls a Tokio worker.
pub async fn validate_outbound_url(raw: &str) -> Result<(), ApiError> {
    let url = Url::parse(raw)
        .map_err(|err| ApiError::bad_request(format!("Invalid URL '{raw}': {err}")))?;

    match url.scheme() {
        "http" | "https" => {}
        other => {
            return Err(ApiError::bad_request(format!(
                "Unsupported URL scheme '{other}': only http and https are allowed"
            )));
        }
    }

    let host = url
        .host()
        .ok_or_else(|| ApiError::bad_request(format!("URL '{raw}' has no host")))?;

    match host {
        Host::Ipv4(ip) => {
            if !ssrf_host_allowed(&ip.to_string()) && is_blocked_ipv4(&ip) {
                return Err(ApiError::bad_request(format!(
                    "Refusing to reach internal address '{ip}'"
                )));
            }
        }
        Host::Ipv6(ip) => {
            if !ssrf_host_allowed(&ip.to_string()) && is_blocked_ip(IpAddr::V6(ip)) {
                return Err(ApiError::bad_request(format!(
                    "Refusing to reach internal address '{ip}'"
                )));
            }
        }
        Host::Domain(domain) => {
            if ssrf_host_allowed(domain) {
                return Ok(());
            }
            // Resolve the hostname and reject if any resolved IP is blocked.
            let port = url.port_or_known_default().unwrap_or(0);
            let addrs = tokio::net::lookup_host((domain, port))
                .await
                .map_err(|err| {
                    ApiError::bad_request(format!("Failed to resolve host '{domain}': {err}"))
                })?;
            for addr in addrs {
                if is_blocked_ip(addr.ip()) {
                    return Err(ApiError::bad_request(format!(
                        "Refusing to reach internal address for host '{domain}'"
                    )));
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        extract::{DefaultBodyLimit, Multipart},
        routing::post,
        Router,
    };
    use http::{header::CONTENT_TYPE, Request};
    use tower::ServiceExt;

    // Mirrors the production multipart handlers (config/prompts/skills): read
    // fields, mapping errors via ApiError::from_multipart. An over-DefaultBodyLimit
    // body must surface as 413, not the opaque 500 that from_anyhow produced (item 12).
    async fn read_all(mut mp: Multipart) -> Result<StatusCode, ApiError> {
        while let Some(field) = mp.next_field().await.map_err(ApiError::from_multipart)? {
            let _ = field.bytes().await.map_err(ApiError::from_multipart)?;
        }
        Ok(StatusCode::OK)
    }

    fn multipart_body(boundary: &str, payload_len: usize) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            b"Content-Disposition: form-data; name=\"file\"; filename=\"x.bin\"\r\n\r\n",
        );
        body.extend(std::iter::repeat(b'x').take(payload_len));
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        body
    }

    async fn upload(limit: usize, payload_len: usize) -> StatusCode {
        let boundary = "ccswMultipartBoundaryTest";
        let app = Router::new()
            .route("/u", post(read_all))
            .layer(DefaultBodyLimit::max(limit));
        let body = multipart_body(boundary, payload_len);
        app.oneshot(
            Request::post("/u")
                .header(
                    CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
    }

    #[tokio::test]
    async fn oversized_multipart_upload_maps_to_413_not_500() {
        // Production handlers used ApiError::from_anyhow → 500 on overflow; the
        // from_multipart helper must preserve MultipartError::status() == 413.
        assert_eq!(upload(1024, 1024 * 8).await, StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn under_limit_multipart_upload_succeeds() {
        // The raised ceiling must let realistic (> axum's 2 MiB default) uploads through.
        assert_eq!(
            upload(64 * 1024 * 1024, 4 * 1024 * 1024).await,
            StatusCode::OK
        );
    }
}
