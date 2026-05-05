use axum::{
    extract::{Multipart, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;

use super::super::ApiState;
use super::common::{json_ok, ApiError, ApiResult};

#[derive(Deserialize)]
struct AppQuery {
    app: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PromptIdQuery {
    app: String,
    id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpsertPromptRequest {
    app: String,
    id: String,
    prompt: crate::prompt::Prompt,
}

fn parse_app_type(app: &str) -> Result<crate::app_config::AppType, ApiError> {
    use std::str::FromStr;
    crate::app_config::AppType::from_str(app).map_err(|err| ApiError::bad_request(err.to_string()))
}

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/prompts/get-prompts", get(get_prompts))
        .route("/prompts/upsert-prompt", post(upsert_prompt))
        .route("/prompts/delete-prompt", delete(delete_prompt))
        .route("/prompts/enable-prompt", post(enable_prompt))
        .route("/prompts/import-prompt-upload", post(import_prompt_upload))
        .route(
            "/prompts/get-current-prompt-file-content",
            get(get_current_prompt_file_content),
        )
        .with_state(state)
}

async fn get_prompts(
    State(state): State<ApiState>,
    Query(query): Query<AppQuery>,
) -> Result<Json<indexmap::IndexMap<String, crate::prompt::Prompt>>, ApiError> {
    let app = parse_app_type(&query.app)?;
    let prompts = crate::services::PromptService::get_prompts(state.app_state.as_ref(), app)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(prompts))
}

async fn upsert_prompt(
    State(state): State<ApiState>,
    Json(request): Json<UpsertPromptRequest>,
) -> ApiResult<()> {
    let app = parse_app_type(&request.app)?;
    crate::services::PromptService::upsert_prompt(
        state.app_state.as_ref(),
        app,
        &request.id,
        request.prompt,
    )
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(()))
}

async fn delete_prompt(
    State(state): State<ApiState>,
    Query(query): Query<PromptIdQuery>,
) -> ApiResult<()> {
    let app = parse_app_type(&query.app)?;
    crate::services::PromptService::delete_prompt(state.app_state.as_ref(), app, &query.id)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(()))
}

async fn enable_prompt(
    State(state): State<ApiState>,
    Json(request): Json<PromptIdQuery>,
) -> ApiResult<()> {
    let app = parse_app_type(&request.app)?;
    crate::services::PromptService::enable_prompt(state.app_state.as_ref(), app, &request.id)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(()))
}

async fn get_current_prompt_file_content(
    Query(query): Query<AppQuery>,
) -> ApiResult<Option<String>> {
    let app = parse_app_type(&query.app)?;
    let content = crate::services::PromptService::get_current_file_content(app)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(content))
}

async fn import_prompt_upload(
    State(state): State<ApiState>,
    Query(query): Query<AppQuery>,
    mut multipart: Multipart,
) -> ApiResult<String> {
    let app = parse_app_type(&query.app)?;
    let mut content = None;
    let mut filename = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(ApiError::from_anyhow)?
    {
        if field.name() == Some("file") {
            filename = field.file_name().map(ToString::to_string);
            let bytes = field.bytes().await.map_err(ApiError::from_anyhow)?;
            let text = String::from_utf8(bytes.to_vec()).map_err(|err| {
                ApiError::bad_request(format!("Invalid UTF-8 prompt upload: {err}"))
            })?;
            content = Some(text);
            break;
        }
    }

    let content = content.ok_or_else(|| ApiError::upload_required("Missing upload field: file"))?;
    let timestamp = chrono::Utc::now().timestamp();
    let id = format!("imported-{timestamp}");
    let display_name = filename
        .as_deref()
        .and_then(|name| name.strip_suffix(".md").or(Some(name)))
        .unwrap_or("Imported Prompt")
        .to_string();
    let prompt = crate::prompt::Prompt {
        id: id.clone(),
        name: display_name,
        content,
        description: Some("Imported from Web upload".to_string()),
        enabled: false,
        created_at: Some(timestamp),
        updated_at: Some(timestamp),
    };
    crate::services::PromptService::upsert_prompt(state.app_state.as_ref(), app, &id, prompt)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(id))
}
