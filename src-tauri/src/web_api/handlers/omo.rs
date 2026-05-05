use axum::{
    extract::State,
    routing::{get, post},
    Router,
};

use super::super::ApiState;
use super::common::{json_ok, ApiError, ApiResult};

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/omo/read-omo-local-file", get(read_omo_local_file))
        .route(
            "/omo/get-current-omo-provider-id",
            get(get_current_omo_provider_id),
        )
        .route("/omo/disable-current-omo", post(disable_current_omo))
        .route(
            "/omo/read-omo-slim-local-file",
            get(read_omo_slim_local_file),
        )
        .route(
            "/omo/get-current-omo-slim-provider-id",
            get(get_current_omo_slim_provider_id),
        )
        .route(
            "/omo/disable-current-omo-slim",
            post(disable_current_omo_slim),
        )
        .with_state(state)
}

async fn read_omo_local_file() -> ApiResult<crate::services::omo::OmoLocalFileData> {
    let data = crate::services::OmoService::read_local_file(&crate::services::omo::STANDARD)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(data))
}

async fn get_current_omo_provider_id(State(state): State<ApiState>) -> ApiResult<String> {
    let provider = state
        .app_state
        .db
        .get_current_omo_provider("opencode", "omo")
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(provider.map(|p| p.id).unwrap_or_default()))
}

async fn disable_current_omo(State(state): State<ApiState>) -> ApiResult<()> {
    disable_omo_variant(state, "omo", &crate::services::omo::STANDARD)
}

async fn read_omo_slim_local_file() -> ApiResult<crate::services::omo::OmoLocalFileData> {
    let data = crate::services::OmoService::read_local_file(&crate::services::omo::SLIM)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(data))
}

async fn get_current_omo_slim_provider_id(State(state): State<ApiState>) -> ApiResult<String> {
    let provider = state
        .app_state
        .db
        .get_current_omo_provider("opencode", "omo-slim")
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(provider.map(|p| p.id).unwrap_or_default()))
}

async fn disable_current_omo_slim(State(state): State<ApiState>) -> ApiResult<()> {
    disable_omo_variant(state, "omo-slim", &crate::services::omo::SLIM)
}

fn disable_omo_variant(
    state: ApiState,
    category: &str,
    variant: &crate::services::omo::OmoVariant,
) -> ApiResult<()> {
    let providers = state
        .app_state
        .db
        .get_all_providers("opencode")
        .map_err(ApiError::from_anyhow)?;

    for (id, provider) in &providers {
        if provider.category.as_deref() == Some(category) {
            state
                .app_state
                .db
                .clear_omo_provider_current("opencode", id, category)
                .map_err(ApiError::from_anyhow)?;
        }
    }

    crate::services::OmoService::delete_config_file(variant).map_err(ApiError::from_anyhow)?;
    Ok(json_ok(()))
}
