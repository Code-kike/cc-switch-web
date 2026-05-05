//! Authentication middleware — Basic Auth fallback + cookie session check.
//!
//! Layer 2 stub. Real implementation in Round 3 P0-2 + Round 4 P0-2:
//!   - opaque session cookie (`cc_switch_session=<id>; HttpOnly; SameSite=Lax`)
//!   - Basic Auth retained as fallback for `curl` / scripted access
//!   - sessions stored in `web_sessions` table (Round 5 P0-1 schema)

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

#[allow(dead_code)]
pub async fn require_auth(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    // Permissive stub: in production, verify cookie session ID against
    // the `web_sessions` table; fall back to Basic Auth header check.
    Ok(next.run(req).await)
}
