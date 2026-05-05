use axum::{
    extract::{Query, State},
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;

use super::super::ApiState;
use super::common::{json_ok, ApiError, ApiResult};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetTakeoverRequest {
    app_type: String,
    enabled: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SwitchProxyProviderRequest {
    app_type: String,
    provider_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppTypeQuery {
    app_type: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateProxyConfigRequest {
    config: crate::proxy::types::ProxyConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateGlobalProxyConfigRequest {
    config: crate::proxy::types::GlobalProxyConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateAppProxyConfigRequest {
    config: crate::proxy::types::AppProxyConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CostMultiplierRequest {
    app_type: String,
    value: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PricingSourceRequest {
    app_type: String,
    value: String,
}

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/proxy/get-proxy-status", get(get_proxy_status))
        .route(
            "/proxy/get-proxy-takeover-status",
            get(get_proxy_takeover_status),
        )
        .route("/proxy/is-proxy-running", get(is_proxy_running))
        .route("/proxy/start-proxy-server", post(start_proxy_server))
        .route(
            "/proxy/stop-proxy-with-restore",
            post(stop_proxy_with_restore),
        )
        .route(
            "/proxy/set-proxy-takeover-for-app",
            put(set_proxy_takeover_for_app),
        )
        .route(
            "/providers/switch-proxy-provider",
            post(switch_proxy_provider),
        )
        .route("/config/get-proxy-config", get(get_proxy_config))
        .route("/config/update-proxy-config", put(update_proxy_config))
        .route(
            "/config/get-global-proxy-config",
            get(get_global_proxy_config),
        )
        .route(
            "/config/update-global-proxy-config",
            put(update_global_proxy_config),
        )
        .route(
            "/config/get-proxy-config-for-app",
            get(get_proxy_config_for_app),
        )
        .route(
            "/config/update-proxy-config-for-app",
            put(update_proxy_config_for_app),
        )
        .route(
            "/system/get_default_cost_multiplier",
            post(get_default_cost_multiplier),
        )
        .route(
            "/system/set_default_cost_multiplier",
            post(set_default_cost_multiplier),
        )
        .route(
            "/system/get_pricing_model_source",
            post(get_pricing_model_source),
        )
        .route(
            "/system/set_pricing_model_source",
            post(set_pricing_model_source),
        )
        .with_state(state)
}

async fn get_proxy_status(
    State(state): State<ApiState>,
) -> ApiResult<crate::proxy::types::ProxyStatus> {
    let status = state
        .app_state
        .proxy_service
        .get_status()
        .await
        .map_err(ApiError::from_service_message)?;
    Ok(json_ok(status))
}

async fn get_proxy_takeover_status(
    State(state): State<ApiState>,
) -> ApiResult<crate::proxy::types::ProxyTakeoverStatus> {
    let status = state
        .app_state
        .proxy_service
        .get_takeover_status()
        .await
        .map_err(ApiError::from_service_message)?;
    Ok(json_ok(status))
}

async fn is_proxy_running(State(state): State<ApiState>) -> ApiResult<bool> {
    Ok(json_ok(state.app_state.proxy_service.is_running().await))
}

async fn start_proxy_server(
    State(state): State<ApiState>,
) -> Result<Json<crate::proxy::types::ProxyServerInfo>, ApiError> {
    let info = state
        .app_state
        .proxy_service
        .start()
        .await
        .map_err(ApiError::from_service_message)?;
    Ok(json_ok(info))
}

async fn stop_proxy_with_restore(State(state): State<ApiState>) -> Result<Json<()>, ApiError> {
    state
        .app_state
        .proxy_service
        .stop_with_restore()
        .await
        .map_err(ApiError::from_service_message)?;
    Ok(json_ok(()))
}

async fn set_proxy_takeover_for_app(
    State(state): State<ApiState>,
    Json(request): Json<SetTakeoverRequest>,
) -> Result<Json<()>, ApiError> {
    state
        .app_state
        .proxy_service
        .set_takeover_for_app(&request.app_type, request.enabled)
        .await
        .map_err(ApiError::from_service_message)?;
    Ok(json_ok(()))
}

async fn switch_proxy_provider(
    State(state): State<ApiState>,
    Json(request): Json<SwitchProxyProviderRequest>,
) -> Result<Json<()>, ApiError> {
    state
        .app_state
        .proxy_service
        .switch_proxy_target(&request.app_type, &request.provider_id)
        .await
        .map_err(ApiError::from_service_message)?;
    Ok(json_ok(()))
}

async fn get_proxy_config(
    State(state): State<ApiState>,
) -> ApiResult<crate::proxy::types::ProxyConfig> {
    let config = state
        .app_state
        .db
        .get_proxy_config()
        .await
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(config))
}

async fn update_proxy_config(
    State(state): State<ApiState>,
    Json(request): Json<UpdateProxyConfigRequest>,
) -> ApiResult<()> {
    state
        .app_state
        .db
        .update_proxy_config(request.config)
        .await
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(()))
}

async fn get_global_proxy_config(
    State(state): State<ApiState>,
) -> ApiResult<crate::proxy::types::GlobalProxyConfig> {
    let config = state
        .app_state
        .db
        .get_global_proxy_config()
        .await
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(config))
}

async fn update_global_proxy_config(
    State(state): State<ApiState>,
    Json(request): Json<UpdateGlobalProxyConfigRequest>,
) -> ApiResult<()> {
    state
        .app_state
        .db
        .update_global_proxy_config(request.config)
        .await
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(()))
}

async fn get_proxy_config_for_app(
    State(state): State<ApiState>,
    Query(query): Query<AppTypeQuery>,
) -> ApiResult<crate::proxy::types::AppProxyConfig> {
    let config = state
        .app_state
        .db
        .get_proxy_config_for_app(&query.app_type)
        .await
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(config))
}

async fn update_proxy_config_for_app(
    State(state): State<ApiState>,
    Json(request): Json<UpdateAppProxyConfigRequest>,
) -> ApiResult<()> {
    state
        .app_state
        .db
        .update_proxy_config_for_app(request.config)
        .await
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(()))
}

async fn get_default_cost_multiplier(
    State(state): State<ApiState>,
    Json(request): Json<AppTypeQuery>,
) -> ApiResult<String> {
    let value = state
        .app_state
        .db
        .get_default_cost_multiplier(&request.app_type)
        .await
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(value))
}

async fn set_default_cost_multiplier(
    State(state): State<ApiState>,
    Json(request): Json<CostMultiplierRequest>,
) -> ApiResult<()> {
    state
        .app_state
        .db
        .set_default_cost_multiplier(&request.app_type, &request.value)
        .await
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(()))
}

async fn get_pricing_model_source(
    State(state): State<ApiState>,
    Json(request): Json<AppTypeQuery>,
) -> ApiResult<String> {
    let value = state
        .app_state
        .db
        .get_pricing_model_source(&request.app_type)
        .await
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(value))
}

async fn set_pricing_model_source(
    State(state): State<ApiState>,
    Json(request): Json<PricingSourceRequest>,
) -> ApiResult<()> {
    state
        .app_state
        .db
        .set_pricing_model_source(&request.app_type, &request.value)
        .await
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(()))
}
