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
struct ProviderIdQuery {
    provider_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetDefaultModelRequest {
    model: crate::openclaw_config::OpenClawDefaultModel,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetModelCatalogRequest {
    catalog: std::collections::HashMap<String, crate::openclaw_config::OpenClawModelCatalogEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetAgentsDefaultsRequest {
    defaults: crate::openclaw_config::OpenClawAgentsDefaults,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetToolsRequest {
    tools: crate::openclaw_config::OpenClawToolsConfig,
}

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route(
            "/openclaw/get-openclaw-live-provider-ids",
            get(get_openclaw_live_provider_ids),
        )
        .route(
            "/openclaw/get-openclaw-live-provider",
            get(get_openclaw_live_provider),
        )
        .route(
            "/openclaw/import-openclaw-providers-from-live",
            post(import_openclaw_providers_from_live),
        )
        .route(
            "/openclaw/get-openclaw-default-model",
            get(get_openclaw_default_model),
        )
        .route(
            "/openclaw/set-openclaw-default-model",
            put(set_openclaw_default_model),
        )
        .route(
            "/openclaw/get-openclaw-model-catalog",
            get(get_openclaw_model_catalog),
        )
        .route(
            "/openclaw/set-openclaw-model-catalog",
            put(set_openclaw_model_catalog),
        )
        .route(
            "/openclaw/get-openclaw-agents-defaults",
            get(get_openclaw_agents_defaults),
        )
        .route(
            "/openclaw/set-openclaw-agents-defaults",
            put(set_openclaw_agents_defaults),
        )
        .route("/openclaw/get-openclaw-tools", get(get_openclaw_tools))
        .route("/openclaw/set-openclaw-tools", put(set_openclaw_tools))
        .route(
            "/config/scan-openclaw-config-health",
            get(scan_openclaw_config_health),
        )
        .with_state(state)
}

async fn import_openclaw_providers_from_live(State(state): State<ApiState>) -> ApiResult<usize> {
    let count =
        crate::services::provider::import_openclaw_providers_from_live(state.app_state.as_ref())
            .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(count))
}

async fn get_openclaw_live_provider_ids() -> ApiResult<Vec<String>> {
    let ids = crate::openclaw_config::get_providers()
        .map(|providers| providers.keys().cloned().collect())
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(ids))
}

async fn get_openclaw_live_provider(
    Query(query): Query<ProviderIdQuery>,
) -> ApiResult<Option<serde_json::Value>> {
    let provider =
        crate::openclaw_config::get_provider(&query.provider_id).map_err(ApiError::from_anyhow)?;
    Ok(json_ok(provider))
}

async fn scan_openclaw_config_health(
) -> ApiResult<Vec<crate::openclaw_config::OpenClawHealthWarning>> {
    let warnings =
        crate::openclaw_config::scan_openclaw_config_health().map_err(ApiError::from_anyhow)?;
    Ok(json_ok(warnings))
}

async fn get_openclaw_default_model(
) -> ApiResult<Option<crate::openclaw_config::OpenClawDefaultModel>> {
    let model = crate::openclaw_config::get_default_model().map_err(ApiError::from_anyhow)?;
    Ok(json_ok(model))
}

async fn set_openclaw_default_model(
    Json(request): Json<SetDefaultModelRequest>,
) -> ApiResult<crate::openclaw_config::OpenClawWriteOutcome> {
    let result =
        crate::openclaw_config::set_default_model(&request.model).map_err(ApiError::from_anyhow)?;
    Ok(json_ok(result))
}

async fn get_openclaw_model_catalog() -> ApiResult<
    Option<std::collections::HashMap<String, crate::openclaw_config::OpenClawModelCatalogEntry>>,
> {
    let catalog = crate::openclaw_config::get_model_catalog().map_err(ApiError::from_anyhow)?;
    Ok(json_ok(catalog))
}

async fn set_openclaw_model_catalog(
    Json(request): Json<SetModelCatalogRequest>,
) -> ApiResult<crate::openclaw_config::OpenClawWriteOutcome> {
    let result = crate::openclaw_config::set_model_catalog(&request.catalog)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(result))
}

async fn get_openclaw_agents_defaults(
) -> ApiResult<Option<crate::openclaw_config::OpenClawAgentsDefaults>> {
    let defaults = crate::openclaw_config::get_agents_defaults().map_err(ApiError::from_anyhow)?;
    Ok(json_ok(defaults))
}

async fn set_openclaw_agents_defaults(
    Json(request): Json<SetAgentsDefaultsRequest>,
) -> ApiResult<crate::openclaw_config::OpenClawWriteOutcome> {
    let result = crate::openclaw_config::set_agents_defaults(&request.defaults)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(result))
}

async fn get_openclaw_tools() -> ApiResult<crate::openclaw_config::OpenClawToolsConfig> {
    let tools = crate::openclaw_config::get_tools_config().map_err(ApiError::from_anyhow)?;
    Ok(json_ok(tools))
}

async fn set_openclaw_tools(
    Json(request): Json<SetToolsRequest>,
) -> ApiResult<crate::openclaw_config::OpenClawWriteOutcome> {
    let result =
        crate::openclaw_config::set_tools_config(&request.tools).map_err(ApiError::from_anyhow)?;
    Ok(json_ok(result))
}
