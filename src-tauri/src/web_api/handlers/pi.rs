use axum::{
    extract::{Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;

use super::super::ApiState;
use super::common::{json_ok, ApiError, ApiResult};

// ── get_pi_current_state ────────────────────────────────────────────────────
pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/pi/get-pi-current-state", get(get_pi_current_state))
        .route(
            "/pi/update-pi-provider-usage-script",
            post(update_pi_provider_usage_script),
        )
        .route(
            "/pi/get-pi-session-discovery",
            get(get_pi_session_discovery),
        )
        .route("/pi/get-pi-prompt-file", get(get_pi_prompt_file))
        .route("/pi/replace-pi-prompt-file", post(replace_pi_prompt_file))
        .route("/pi/delete-pi-prompt-file", delete(delete_pi_prompt_file))
        .route(
            "/pi/list-pi-prompt-templates",
            get(list_pi_prompt_templates),
        )
        .route(
            "/pi/upsert-pi-prompt-template",
            post(upsert_pi_prompt_template),
        )
        .route(
            "/pi/delete-pi-prompt-template",
            delete(delete_pi_prompt_template),
        )
        .with_state(state)
}

async fn get_pi_current_state(
    State(state): State<ApiState>,
) -> Result<Json<crate::services::pi_state::PiCurrentState>, ApiError> {
    let current = crate::services::pi_state::PiStateService::current(state.app_state.as_ref())
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(current))
}

// ── update_pi_provider_usage_script ────────────────────────────────────────
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdatePiUsageScriptRequest {
    id: String,
    usage_script: crate::provider::UsageScript,
}

async fn update_pi_provider_usage_script(
    State(state): State<ApiState>,
    Json(request): Json<UpdatePiUsageScriptRequest>,
) -> ApiResult<bool> {
    let ok = crate::services::ProviderService::update_pi_usage_script(
        state.app_state.as_ref(),
        &request.id,
        request.usage_script,
    )
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(ok))
}

// ── get_pi_session_discovery ────────────────────────────────────────────────
async fn get_pi_session_discovery(
) -> Json<crate::session_manager::providers::pi::PiSessionDiscovery> {
    json_ok(crate::session_manager::providers::pi::session_discovery())
}

// ── pi prompt files (SYSTEM.md / APPEND_SYSTEM.md) ──────────────────────────
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PiPromptFileQuery {
    kind: crate::services::pi_prompt_files::PiPromptFileKind,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReplacePiPromptFileRequest {
    kind: crate::services::pi_prompt_files::PiPromptFileKind,
    expected_revision: String,
    content: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeletePiPromptFileQuery {
    kind: crate::services::pi_prompt_files::PiPromptFileKind,
    expected_revision: String,
}

async fn get_pi_prompt_file(
    Query(query): Query<PiPromptFileQuery>,
) -> ApiResult<crate::services::pi_prompt_files::PiPromptFileSnapshot> {
    let snapshot = crate::services::pi_prompt_files::PiPromptFileService::read(query.kind)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(snapshot))
}

async fn replace_pi_prompt_file(
    Json(request): Json<ReplacePiPromptFileRequest>,
) -> ApiResult<crate::services::pi_prompt_files::PiPromptFileSnapshot> {
    let snapshot = crate::services::pi_prompt_files::PiPromptFileService::replace(
        request.kind,
        &request.expected_revision,
        &request.content,
    )
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(snapshot))
}

async fn delete_pi_prompt_file(Query(query): Query<DeletePiPromptFileQuery>) -> ApiResult<bool> {
    let ok = crate::services::pi_prompt_files::PiPromptFileService::delete(
        query.kind,
        &query.expected_revision,
    )
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(ok))
}

// ── pi prompt templates (prompts/*.md) ─────────────────────────────────────
async fn list_pi_prompt_templates(
) -> ApiResult<Vec<crate::services::pi_prompt_files::PiPromptTemplate>> {
    let templates = crate::services::pi_prompt_files::PiPromptTemplateService::list()
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(templates))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpsertPiPromptTemplateRequest {
    slug: String,
    original_slug: Option<String>,
    expected_revision: String,
    content: String,
}

async fn upsert_pi_prompt_template(
    Json(request): Json<UpsertPiPromptTemplateRequest>,
) -> ApiResult<crate::services::pi_prompt_files::PiPromptTemplate> {
    let template = crate::services::pi_prompt_files::PiPromptTemplateService::upsert(
        &request.slug,
        request.original_slug.as_deref(),
        &request.expected_revision,
        &request.content,
    )
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(template))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeletePiPromptTemplateQuery {
    slug: String,
    expected_revision: String,
}

async fn delete_pi_prompt_template(
    Query(query): Query<DeletePiPromptTemplateQuery>,
) -> ApiResult<bool> {
    let ok = crate::services::pi_prompt_files::PiPromptTemplateService::delete(
        &query.slug,
        &query.expected_revision,
    )
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(ok))
}
