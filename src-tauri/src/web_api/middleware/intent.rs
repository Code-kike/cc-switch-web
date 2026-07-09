//! Same-origin intent middleware for mutating Web API requests.
//!
//! The standalone Web API is unauthenticated by design. This middleware is not
//! an access-control or identity layer; it only rejects browser-initiated
//! cross-site writes so unrelated web pages cannot trigger state-changing
//! `/api/*` calls. Direct clients such as curl/scripts and same-origin SPA calls
//! pass through.

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

fn forbidden(reason: &'static str) -> Response {
    (StatusCode::FORBIDDEN, reason).into_response()
}

/// Whether `method` is state-changing and needs a same-origin intent check.
fn is_state_changing(method: &Method) -> bool {
    matches!(
        *method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    )
}

/// Require a same-origin request intent for state-changing browser requests.
///
/// Accepted signals:
/// - `Sec-Fetch-Site: same-origin` or `none`
/// - absent Fetch Metadata with `Origin` host matching `Host`
/// - absent `Origin` and absent Fetch Metadata, for direct clients like curl
fn check_same_origin_intent(req: &Request<Body>) -> Result<(), Response> {
    let headers = req.headers();

    if let Some(site) = headers
        .get("sec-fetch-site")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
    {
        return match site {
            "same-origin" | "none" => Ok(()),
            _ => Err(forbidden("cross-site request rejected")),
        };
    }

    let Some(origin) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) else {
        return Ok(());
    };
    if origin.eq_ignore_ascii_case("null") {
        return Err(forbidden("cross-site request rejected"));
    }

    let origin_host = url::Url::parse(origin)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_ascii_lowercase()));
    let request_host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .and_then(|h| {
            url::Url::parse(&format!("http://{h}"))
                .ok()
                .and_then(|u| u.host_str().map(|s| s.to_ascii_lowercase()))
        });

    match (origin_host, request_host) {
        (Some(o), Some(h)) if o == h => Ok(()),
        _ => Err(forbidden("cross-site request rejected")),
    }
}

/// Apply the same-origin intent check to state-changing `/api/*` requests.
///
/// Static SPA assets and `/api/health` remain public. `OPTIONS` preflight
/// requests with an `Origin` header pass through to the CORS layer; the actual
/// mutating request that follows is still checked.
pub async fn require_same_origin_intent(req: Request<Body>, next: Next) -> Response {
    let path = req.uri().path();
    let is_public = !path.starts_with("/api/") || path == "/api/health";
    if is_public {
        return next.run(req).await;
    }

    if req.method() == Method::OPTIONS && req.headers().contains_key(header::ORIGIN) {
        return next.run(req).await;
    }

    if is_state_changing(req.method()) {
        if let Err(resp) = check_same_origin_intent(&req) {
            return resp;
        }
    }

    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let req = req_with(
            Method::POST,
            "/api/proxy/start-proxy-server",
            &[("sec-fetch-site", "cross-site")],
        );
        assert!(check_same_origin_intent(&req).is_err());
    }

    #[test]
    fn same_origin_post_passes_intent_check() {
        let req = req_with(
            Method::POST,
            "/api/proxy/start-proxy-server",
            &[
                ("origin", "http://100.75.197.120:3010"),
                ("host", "100.75.197.120:3010"),
            ],
        );
        assert!(check_same_origin_intent(&req).is_ok());

        let req = req_with(
            Method::POST,
            "/api/proxy/start-proxy-server",
            &[("sec-fetch-site", "same-origin")],
        );
        assert!(check_same_origin_intent(&req).is_ok());

        let req = req_with(Method::POST, "/api/proxy/start-proxy-server", &[]);
        assert!(check_same_origin_intent(&req).is_ok());

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

    #[test]
    fn ipv6_same_origin_host_passes_intent_check() {
        let req = req_with(
            Method::POST,
            "/api/proxy/start-proxy-server",
            &[("origin", "http://[::1]:3010"), ("host", "[::1]:3010")],
        );
        assert!(check_same_origin_intent(&req).is_ok());
    }

    #[test]
    fn ipv6_cross_site_origin_post_is_rejected() {
        let req = req_with(
            Method::POST,
            "/api/proxy/start-proxy-server",
            &[
                ("origin", "http://[2001:db8::1]:3010"),
                ("host", "[::1]:3010"),
            ],
        );
        assert!(check_same_origin_intent(&req).is_err());
    }
}
