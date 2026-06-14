use axum::{
    extract::{Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;

use super::super::ApiState;
use super::common::{json_ok, ApiError, ApiResult};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppTypeQuery {
    app_type: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderQuery {
    app_type: String,
    provider_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetAutoFailoverRequest {
    app_type: String,
    enabled: bool,
}

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/failover/get-failover-queue", get(get_failover_queue))
        .route(
            "/failover/get-available-providers-for-failover",
            get(get_available_providers_for_failover),
        )
        .route(
            "/failover/add-to-failover-queue",
            post(add_to_failover_queue),
        )
        .route(
            "/failover/remove-from-failover-queue",
            delete(remove_from_failover_queue),
        )
        .route(
            "/failover/get-auto-failover-enabled",
            get(get_auto_failover_enabled),
        )
        .route(
            "/failover/set-auto-failover-enabled",
            put(set_auto_failover_enabled),
        )
        .with_state(state)
}

async fn get_failover_queue(
    State(state): State<ApiState>,
    Query(query): Query<AppTypeQuery>,
) -> ApiResult<Vec<crate::database::FailoverQueueItem>> {
    let queue = state
        .app_state
        .db
        .get_failover_queue(&query.app_type)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(queue))
}

async fn get_available_providers_for_failover(
    State(state): State<ApiState>,
    Query(query): Query<AppTypeQuery>,
) -> ApiResult<Vec<crate::provider::Provider>> {
    let providers = state
        .app_state
        .db
        .get_available_providers_for_failover(&query.app_type)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(providers))
}

async fn add_to_failover_queue(
    State(state): State<ApiState>,
    Json(request): Json<ProviderQuery>,
) -> ApiResult<()> {
    state
        .app_state
        .db
        .add_to_failover_queue(&request.app_type, &request.provider_id)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(()))
}

async fn remove_from_failover_queue(
    State(state): State<ApiState>,
    Query(query): Query<ProviderQuery>,
) -> ApiResult<()> {
    state
        .app_state
        .db
        .remove_from_failover_queue(&query.app_type, &query.provider_id)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(()))
}

async fn get_auto_failover_enabled(
    State(state): State<ApiState>,
    Query(query): Query<AppTypeQuery>,
) -> ApiResult<bool> {
    let enabled = state
        .app_state
        .db
        .get_proxy_config_for_app(&query.app_type)
        .await
        .map_err(ApiError::from_anyhow)?
        .auto_failover_enabled;
    Ok(json_ok(enabled))
}

async fn set_auto_failover_enabled(
    State(state): State<ApiState>,
    Json(request): Json<SetAutoFailoverRequest>,
) -> ApiResult<()> {
    // F9: align web with desktop semantics via the shared, runtime-neutral
    // service method. On enable it auto-adds the current provider when the queue
    // would be empty, switches the proxy target to P1, and emits
    // `provider-switched` through the injected ChannelEventSink (SSE) so the web
    // UI refreshes — instead of the old "reject empty queue + only flip the flag".
    state
        .app_state
        .proxy_service
        .set_auto_failover_enabled(&request.app_type, request.enabled)
        .await
        .map_err(ApiError::from_service_message)?;
    Ok(json_ok(()))
}
