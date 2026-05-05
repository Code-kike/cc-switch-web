//! CSRF middleware — verifies `X-CSRF-Token` header on non-safe methods.
//!
//! Layer 2 stub. Real implementation in Round 4 P1-5:
//!   - bind CSRF token to session_id at login
//!   - skip safe methods (`GET`, `HEAD`, `OPTIONS`)
//!   - skip `/api/events` SSE endpoint (Round 5 P1-2)

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    middleware::Next,
    response::Response,
};

#[allow(dead_code)]
pub async fn verify_csrf(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let method = req.method();
    let path = req.uri().path();
    if matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS) || path == "/api/events" {
        return Ok(next.run(req).await);
    }
    // Permissive stub for now; production code looks up session.csrf_token
    // and rejects 403 on mismatch.
    Ok(next.run(req).await)
}
