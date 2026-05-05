//! `model_test` Web handler — Layer 2 stub.
//!
//! Real implementation arrives in Layer 3 / Task model_test-owner; for now the
//! router is empty and every `/api/model_test/...` request falls through to the
//! API 404 handler. The frontend adapter sees these as `WEB_NOT_SUPPORTED`
//! errors when it tries to invoke an unmapped command.

use axum::Router;

use super::super::ApiState;

#[allow(unused_variables, dead_code)]
pub fn router(state: ApiState) -> Router {
    Router::new()
}
