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
struct MemoryQuery {
    kind: crate::hermes_config::MemoryKind,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetMemoryRequest {
    kind: crate::hermes_config::MemoryKind,
    content: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetMemoryEnabledRequest {
    kind: crate::hermes_config::MemoryKind,
    enabled: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenHermesWebUiRequest {
    path: Option<String>,
}

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route(
            "/hermes/get-hermes-live-provider-ids",
            get(get_hermes_live_provider_ids),
        )
        .route(
            "/hermes/get-hermes-live-provider",
            get(get_hermes_live_provider),
        )
        .route(
            "/hermes/import-hermes-providers-from-live",
            post(import_hermes_providers_from_live),
        )
        .route(
            "/config/get-hermes-model-config",
            get(get_hermes_model_config),
        )
        .route("/hermes/get-hermes-memory", get(get_hermes_memory))
        .route("/hermes/set-hermes-memory", put(set_hermes_memory))
        .route(
            "/hermes/get-hermes-memory-limits",
            get(get_hermes_memory_limits),
        )
        .route(
            "/hermes/set-hermes-memory-enabled",
            put(set_hermes_memory_enabled),
        )
        .route("/hermes/open-hermes-web-ui", post(open_hermes_web_ui))
        .route(
            "/hermes/launch-hermes-dashboard",
            post(launch_hermes_dashboard),
        )
        .with_state(state)
}

async fn import_hermes_providers_from_live(State(state): State<ApiState>) -> ApiResult<usize> {
    let count =
        crate::services::provider::import_hermes_providers_from_live(state.app_state.as_ref())
            .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(count))
}

async fn get_hermes_live_provider_ids() -> ApiResult<Vec<String>> {
    let ids = crate::hermes_config::get_providers()
        .map(|providers| providers.keys().cloned().collect())
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(ids))
}

async fn get_hermes_live_provider(
    Query(query): Query<ProviderIdQuery>,
) -> ApiResult<Option<serde_json::Value>> {
    let provider =
        crate::hermes_config::get_provider(&query.provider_id).map_err(ApiError::from_anyhow)?;
    Ok(json_ok(provider))
}

async fn get_hermes_model_config() -> ApiResult<Option<crate::hermes_config::HermesModelConfig>> {
    let config = crate::hermes_config::get_model_config().map_err(ApiError::from_anyhow)?;
    Ok(json_ok(config))
}

async fn get_hermes_memory(Query(query): Query<MemoryQuery>) -> ApiResult<String> {
    let content = crate::hermes_config::read_memory(query.kind).map_err(ApiError::from_anyhow)?;
    Ok(json_ok(content))
}

async fn set_hermes_memory(Json(request): Json<SetMemoryRequest>) -> ApiResult<()> {
    crate::hermes_config::write_memory(request.kind, &request.content)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(()))
}

async fn get_hermes_memory_limits() -> ApiResult<crate::hermes_config::HermesMemoryLimits> {
    let limits = crate::hermes_config::read_memory_limits().map_err(ApiError::from_anyhow)?;
    Ok(json_ok(limits))
}

async fn set_hermes_memory_enabled(
    Json(request): Json<SetMemoryEnabledRequest>,
) -> ApiResult<crate::hermes_config::HermesWriteOutcome> {
    let result = crate::hermes_config::set_memory_enabled(request.kind, request.enabled)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(result))
}

async fn open_hermes_web_ui(
    State(state): State<ApiState>,
    Json(request): Json<OpenHermesWebUiRequest>,
) -> ApiResult<()> {
    let port = std::env::var("HERMES_WEB_PORT")
        .ok()
        .and_then(|raw| raw.trim().parse::<u16>().ok())
        .unwrap_or(9119);
    let base = format!("http://127.0.0.1:{port}");
    let target = match request.path.as_deref() {
        Some(path) if path.starts_with('/') => format!("{base}{path}"),
        Some(path) if !path.is_empty() => format!("{base}/{path}"),
        _ => format!("{base}/"),
    };
    state
        .sink
        .open_url(&target)
        .map_err(ApiError::from_service_message)?;
    Ok(json_ok(()))
}

async fn launch_hermes_dashboard() -> Result<Json<()>, ApiError> {
    Err(ApiError::not_supported(
        "Hermes dashboard launch is not supported in web-server mode",
    ))
}
