use axum::{
    extract::{Query, State},
    routing::{delete, get},
    Json, Router,
};
use serde::Deserialize;

use super::super::ApiState;
use super::common::{json_ok, web_desktop_only, ApiError, ApiResult};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionMessagesQuery {
    provider_id: String,
    source_path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteSessionQuery {
    provider_id: String,
    session_id: String,
    source_path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteSessionsRequest {
    items: Vec<crate::session_manager::DeleteSessionRequest>,
}

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/sessions/list-sessions", get(list_sessions))
        .route("/sessions/get-session-messages", get(get_session_messages))
        .route("/sessions/delete-session", delete(delete_session))
        .route("/sessions/delete-sessions", delete(delete_sessions))
        .route(
            "/sessions/launch-session-terminal",
            axum::routing::post(web_desktop_only),
        )
        .with_state(state)
}

async fn list_sessions() -> ApiResult<Vec<crate::session_manager::SessionMeta>> {
    let sessions = tokio::task::spawn_blocking(crate::session_manager::scan_sessions)
        .await
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(sessions))
}

async fn get_session_messages(
    Query(query): Query<SessionMessagesQuery>,
) -> ApiResult<Vec<crate::session_manager::SessionMessage>> {
    let provider_id = query.provider_id;
    let source_path = query.source_path;
    let messages = tokio::task::spawn_blocking(move || {
        crate::session_manager::load_messages(&provider_id, &source_path)
    })
    .await
    .map_err(ApiError::from_anyhow)?
    .map_err(ApiError::from_service_message)?;
    Ok(json_ok(messages))
}

async fn delete_session(Query(query): Query<DeleteSessionQuery>) -> ApiResult<bool> {
    let deleted = tokio::task::spawn_blocking(move || {
        crate::session_manager::delete_session(
            &query.provider_id,
            &query.session_id,
            &query.source_path,
        )
    })
    .await
    .map_err(ApiError::from_anyhow)?
    .map_err(ApiError::from_service_message)?;
    Ok(json_ok(deleted))
}

async fn delete_sessions(
    Json(request): Json<DeleteSessionsRequest>,
) -> ApiResult<Vec<crate::session_manager::DeleteSessionOutcome>> {
    let outcomes = tokio::task::spawn_blocking(move || {
        crate::session_manager::delete_sessions(&request.items)
    })
    .await
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(outcomes))
}
