//! Health check — public, unauthenticated, used by load balancers and
//! container orchestrators. Returns a static JSON snapshot of build info.

use axum::{routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    name: &'static str,
    version: &'static str,
}

pub fn router() -> Router {
    Router::new().route("/health", get(health))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
    })
}
