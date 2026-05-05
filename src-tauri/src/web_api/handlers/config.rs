use axum::{
    extract::{Multipart, Query, State},
    http::{header, HeaderMap, HeaderValue},
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;

use super::super::ApiState;
use super::common::{json_ok, ApiError, ApiResult};

#[derive(Deserialize)]
struct AppQuery {
    app: String,
}

#[derive(Deserialize)]
struct ClaudeConfigStatusQuery {
    #[allow(dead_code)]
    app: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetAppConfigDirOverrideRequest {
    path: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetRectifierConfigRequest {
    config: crate::proxy::types::RectifierConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetOptimizerConfigRequest {
    config: crate::proxy::types::OptimizerConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetLogConfigRequest {
    config: crate::proxy::types::LogConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateCircuitBreakerConfigRequest {
    config: crate::proxy::circuit_breaker::CircuitBreakerConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetClaudeCommonConfigSnippetRequest {
    snippet: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommonConfigSnippetQuery {
    app_type: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetCommonConfigSnippetRequest {
    app_type: String,
    snippet: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExtractCommonConfigSnippetRequest {
    app_type: String,
    settings_config: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FetchModelsForConfigQuery {
    base_url: String,
    api_key: String,
    is_full_url: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveStreamCheckConfigRequest {
    config: crate::services::stream_check::StreamCheckConfig,
}

fn parse_app_type(app: &str) -> Result<crate::app_config::AppType, ApiError> {
    use std::str::FromStr;
    crate::app_config::AppType::from_str(app).map_err(|err| ApiError::bad_request(err.to_string()))
}

fn invalid_json_format_error(error: serde_json::Error) -> String {
    let lang = crate::settings::get_settings()
        .language
        .unwrap_or_else(|| "zh".to_string());

    match lang.as_str() {
        "en" => format!("Invalid JSON format: {error}"),
        "ja" => format!("JSON形式が無効です: {error}"),
        _ => format!("无效的 JSON 格式: {error}"),
    }
}

fn invalid_toml_format_error(error: toml_edit::TomlError) -> String {
    let lang = crate::settings::get_settings()
        .language
        .unwrap_or_else(|| "zh".to_string());

    match lang.as_str() {
        "en" => format!("Invalid TOML format: {error}"),
        "ja" => format!("TOML形式が無効です: {error}"),
        _ => format!("无效的 TOML 格式: {error}"),
    }
}

fn normalize_common_config_app_type(app_type: &str) -> &str {
    match app_type {
        "omo_slim" => "omo-slim",
        _ => app_type,
    }
}

fn validate_common_config_snippet(app_type: &str, snippet: &str) -> Result<(), ApiError> {
    if snippet.trim().is_empty() {
        return Ok(());
    }

    match app_type {
        "claude" | "gemini" | "omo" | "omo-slim" => {
            serde_json::from_str::<serde_json::Value>(snippet)
                .map_err(|err| ApiError::bad_request(invalid_json_format_error(err)))?;
        }
        "codex" => {
            snippet
                .parse::<toml_edit::DocumentMut>()
                .map_err(|err| ApiError::bad_request(invalid_toml_format_error(err)))?;
        }
        _ => {}
    }

    Ok(())
}

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route(
            "/config/get-app-config-dir-override",
            get(get_app_config_dir_override),
        )
        .route(
            "/config/set-app-config-dir-override",
            put(set_app_config_dir_override),
        )
        .route("/config/get-config-dir", get(get_config_dir))
        .route("/config/get-app-config-path", get(get_app_config_path))
        .route(
            "/config/get-claude-code-config-path",
            get(get_claude_code_config_path),
        )
        .route(
            "/config/get-claude-config-status",
            get(get_claude_config_status),
        )
        .route(
            "/config/get-claude-common-config-snippet",
            get(get_claude_common_config_snippet),
        )
        .route(
            "/config/set-claude-common-config-snippet",
            put(set_claude_common_config_snippet),
        )
        .route(
            "/config/get-common-config-snippet",
            get(get_common_config_snippet),
        )
        .route(
            "/config/set-common-config-snippet",
            put(set_common_config_snippet),
        )
        .route(
            "/config/extract-common-config-snippet",
            post(extract_common_config_snippet),
        )
        .route(
            "/config/fetch-models-for-config",
            get(fetch_models_for_config),
        )
        .route(
            "/config/get-stream-check-config",
            get(get_stream_check_config),
        )
        .route(
            "/config/save-stream-check-config",
            put(save_stream_check_config),
        )
        .route(
            "/config/read-claude-plugin-config",
            get(read_claude_plugin_config),
        )
        .route(
            "/config/apply-claude-plugin-config",
            post(apply_claude_plugin_config),
        )
        .route("/config/get-config-status", get(get_config_status))
        .route("/config/get-rectifier-config", get(get_rectifier_config))
        .route("/config/set-rectifier-config", put(set_rectifier_config))
        .route("/config/get-optimizer-config", get(get_optimizer_config))
        .route("/config/set-optimizer-config", put(set_optimizer_config))
        .route("/config/get-log-config", get(get_log_config))
        .route("/config/set-log-config", put(set_log_config))
        .route(
            "/config/get-circuit-breaker-config",
            get(get_circuit_breaker_config),
        )
        .route(
            "/config/update-circuit-breaker-config",
            put(update_circuit_breaker_config),
        )
        .route(
            "/config/import-config-upload",
            axum::routing::post(import_config_upload),
        )
        .route(
            "/config/export-config-download",
            get(export_config_download),
        )
        .with_state(state)
}

async fn get_app_config_dir_override() -> ApiResult<Option<String>> {
    Ok(json_ok(
        crate::app_store::get_app_config_dir_override().map(|p| p.to_string_lossy().to_string()),
    ))
}

async fn set_app_config_dir_override(
    State(_state): State<ApiState>,
    Json(request): Json<SetAppConfigDirOverrideRequest>,
) -> ApiResult<bool> {
    crate::app_store::set_app_config_dir_override_web(request.path.as_deref())
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(true))
}

async fn get_config_dir(Query(query): Query<AppQuery>) -> ApiResult<String> {
    let dir = match parse_app_type(&query.app)? {
        crate::app_config::AppType::Claude => crate::config::get_claude_config_dir(),
        crate::app_config::AppType::Codex => crate::codex_config::get_codex_config_dir(),
        crate::app_config::AppType::Gemini => crate::gemini_config::get_gemini_dir(),
        crate::app_config::AppType::OpenCode => crate::opencode_config::get_opencode_dir(),
        crate::app_config::AppType::OpenClaw => crate::openclaw_config::get_openclaw_dir(),
        crate::app_config::AppType::Hermes => crate::hermes_config::get_hermes_dir(),
    };
    Ok(json_ok(dir.to_string_lossy().to_string()))
}

async fn get_app_config_path() -> ApiResult<String> {
    Ok(json_ok(
        crate::config::get_app_config_path()
            .to_string_lossy()
            .to_string(),
    ))
}

async fn get_claude_code_config_path() -> ApiResult<String> {
    Ok(json_ok(
        crate::config::get_claude_settings_path()
            .to_string_lossy()
            .to_string(),
    ))
}

async fn get_claude_config_status(
    Query(_query): Query<ClaudeConfigStatusQuery>,
) -> ApiResult<crate::config::ConfigStatus> {
    Ok(json_ok(crate::config::get_claude_config_status()))
}

async fn get_claude_common_config_snippet(
    State(state): State<ApiState>,
) -> ApiResult<Option<String>> {
    let snippet = state
        .app_state
        .db
        .get_config_snippet("claude")
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(snippet))
}

async fn set_claude_common_config_snippet(
    State(state): State<ApiState>,
    Json(request): Json<SetClaudeCommonConfigSnippetRequest>,
) -> ApiResult<()> {
    validate_common_config_snippet("claude", &request.snippet)?;
    let is_cleared = request.snippet.trim().is_empty();
    let value = if is_cleared {
        None
    } else {
        Some(request.snippet)
    };

    state
        .app_state
        .db
        .set_config_snippet("claude", value)
        .map_err(ApiError::from_anyhow)?;
    state
        .app_state
        .db
        .set_config_snippet_cleared("claude", is_cleared)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(()))
}

async fn get_common_config_snippet(
    State(state): State<ApiState>,
    Query(query): Query<CommonConfigSnippetQuery>,
) -> ApiResult<Option<String>> {
    let app_type = normalize_common_config_app_type(&query.app_type);
    let snippet = state
        .app_state
        .db
        .get_config_snippet(app_type)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(snippet))
}

async fn set_common_config_snippet(
    State(state): State<ApiState>,
    Json(request): Json<SetCommonConfigSnippetRequest>,
) -> ApiResult<()> {
    let app_type = normalize_common_config_app_type(&request.app_type).to_string();
    let is_cleared = request.snippet.trim().is_empty();
    let old_snippet = state
        .app_state
        .db
        .get_config_snippet(&app_type)
        .map_err(ApiError::from_anyhow)?;

    validate_common_config_snippet(&app_type, &request.snippet)?;

    let value = if is_cleared {
        None
    } else {
        Some(request.snippet)
    };

    if matches!(app_type.as_str(), "claude" | "codex" | "gemini") {
        if let Some(legacy_snippet) = old_snippet
            .as_deref()
            .filter(|existing| !existing.trim().is_empty())
        {
            let app = parse_app_type(&app_type)?;
            crate::services::ProviderService::migrate_legacy_common_config_usage(
                state.app_state.as_ref(),
                app,
                legacy_snippet,
            )
            .map_err(ApiError::from_anyhow)?;
        }
    }

    state
        .app_state
        .db
        .set_config_snippet(&app_type, value)
        .map_err(ApiError::from_anyhow)?;
    state
        .app_state
        .db
        .set_config_snippet_cleared(&app_type, is_cleared)
        .map_err(ApiError::from_anyhow)?;

    if matches!(app_type.as_str(), "claude" | "codex" | "gemini") {
        let app = parse_app_type(&app_type)?;
        crate::services::ProviderService::sync_current_provider_for_app(
            state.app_state.as_ref(),
            app,
        )
        .map_err(ApiError::from_anyhow)?;
    }

    if app_type == "omo"
        && state
            .app_state
            .db
            .get_current_omo_provider("opencode", "omo")
            .map_err(ApiError::from_anyhow)?
            .is_some()
    {
        crate::services::OmoService::write_config_to_file(
            state.app_state.as_ref(),
            &crate::services::omo::STANDARD,
        )
        .map_err(ApiError::from_anyhow)?;
    }
    if app_type == "omo-slim"
        && state
            .app_state
            .db
            .get_current_omo_provider("opencode", "omo-slim")
            .map_err(ApiError::from_anyhow)?
            .is_some()
    {
        crate::services::OmoService::write_config_to_file(
            state.app_state.as_ref(),
            &crate::services::omo::SLIM,
        )
        .map_err(ApiError::from_anyhow)?;
    }

    Ok(json_ok(()))
}

async fn extract_common_config_snippet(
    State(state): State<ApiState>,
    Json(request): Json<ExtractCommonConfigSnippetRequest>,
) -> ApiResult<String> {
    let app_type = normalize_common_config_app_type(&request.app_type);
    let app = parse_app_type(app_type)?;

    if let Some(settings_config) = request
        .settings_config
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        let settings = serde_json::from_str::<serde_json::Value>(settings_config)
            .map_err(|err| ApiError::bad_request(invalid_json_format_error(err)))?;
        let snippet =
            crate::services::provider::ProviderService::extract_common_config_snippet_from_settings(
                app, &settings,
            )
            .map_err(ApiError::from_anyhow)?;
        return Ok(json_ok(snippet));
    }

    let snippet = crate::services::provider::ProviderService::extract_common_config_snippet(
        state.app_state.as_ref(),
        app,
    )
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(snippet))
}

async fn fetch_models_for_config(
    Query(query): Query<FetchModelsForConfigQuery>,
) -> ApiResult<Vec<crate::services::model_fetch::FetchedModel>> {
    let models = crate::services::model_fetch::fetch_models(
        &query.base_url,
        &query.api_key,
        query.is_full_url.unwrap_or(false),
    )
    .await
    .map_err(ApiError::bad_request)?;
    Ok(json_ok(models))
}

async fn get_stream_check_config(
    State(state): State<ApiState>,
) -> ApiResult<crate::services::stream_check::StreamCheckConfig> {
    let config = state
        .app_state
        .db
        .get_stream_check_config()
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(config))
}

async fn save_stream_check_config(
    State(state): State<ApiState>,
    Json(request): Json<SaveStreamCheckConfigRequest>,
) -> ApiResult<()> {
    state
        .app_state
        .db
        .save_stream_check_config(&request.config)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(()))
}

async fn read_claude_plugin_config() -> ApiResult<Option<String>> {
    let config = crate::claude_plugin::read_claude_config().map_err(ApiError::from_anyhow)?;
    Ok(json_ok(config))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApplyClaudePluginConfigRequest {
    official: bool,
}

async fn apply_claude_plugin_config(
    Json(request): Json<ApplyClaudePluginConfigRequest>,
) -> ApiResult<bool> {
    let changed = if request.official {
        crate::claude_plugin::clear_claude_config()
    } else {
        crate::claude_plugin::write_claude_config()
    }
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(changed))
}

async fn get_config_status(
    Query(query): Query<AppQuery>,
) -> ApiResult<crate::config::ConfigStatus> {
    let status = match parse_app_type(&query.app)? {
        crate::app_config::AppType::Claude => crate::config::get_claude_config_status(),
        crate::app_config::AppType::Codex => {
            let auth_path = crate::codex_config::get_codex_auth_path();
            crate::config::ConfigStatus {
                exists: auth_path.exists(),
                path: crate::codex_config::get_codex_config_dir()
                    .to_string_lossy()
                    .to_string(),
            }
        }
        crate::app_config::AppType::Gemini => {
            let env_path = crate::gemini_config::get_gemini_env_path();
            crate::config::ConfigStatus {
                exists: env_path.exists(),
                path: crate::gemini_config::get_gemini_dir()
                    .to_string_lossy()
                    .to_string(),
            }
        }
        crate::app_config::AppType::OpenCode => {
            let config_path = crate::opencode_config::get_opencode_config_path();
            crate::config::ConfigStatus {
                exists: config_path.exists(),
                path: crate::opencode_config::get_opencode_dir()
                    .to_string_lossy()
                    .to_string(),
            }
        }
        crate::app_config::AppType::OpenClaw => {
            let config_path = crate::openclaw_config::get_openclaw_config_path();
            crate::config::ConfigStatus {
                exists: config_path.exists(),
                path: crate::openclaw_config::get_openclaw_dir()
                    .to_string_lossy()
                    .to_string(),
            }
        }
        crate::app_config::AppType::Hermes => {
            let config_path = crate::hermes_config::get_hermes_config_path();
            crate::config::ConfigStatus {
                exists: config_path.exists(),
                path: crate::hermes_config::get_hermes_dir()
                    .to_string_lossy()
                    .to_string(),
            }
        }
    };
    Ok(json_ok(status))
}

async fn get_rectifier_config(
    State(state): State<ApiState>,
) -> ApiResult<crate::proxy::types::RectifierConfig> {
    let config = state
        .app_state
        .db
        .get_rectifier_config()
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(config))
}

async fn set_rectifier_config(
    State(state): State<ApiState>,
    Json(request): Json<SetRectifierConfigRequest>,
) -> ApiResult<bool> {
    state
        .app_state
        .db
        .set_rectifier_config(&request.config)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(true))
}

async fn get_optimizer_config(
    State(state): State<ApiState>,
) -> ApiResult<crate::proxy::types::OptimizerConfig> {
    let config = state
        .app_state
        .db
        .get_optimizer_config()
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(config))
}

async fn set_optimizer_config(
    State(state): State<ApiState>,
    Json(request): Json<SetOptimizerConfigRequest>,
) -> ApiResult<bool> {
    match request.config.cache_ttl.as_str() {
        "5m" | "1h" => {}
        other => {
            return Err(ApiError::bad_request(format!(
                "Invalid cache_ttl value: '{other}'. Allowed values: '5m', '1h'"
            )));
        }
    }
    state
        .app_state
        .db
        .set_optimizer_config(&request.config)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(true))
}

async fn get_log_config(
    State(state): State<ApiState>,
) -> ApiResult<crate::proxy::types::LogConfig> {
    let config = state
        .app_state
        .db
        .get_log_config()
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(config))
}

async fn set_log_config(
    State(state): State<ApiState>,
    Json(request): Json<SetLogConfigRequest>,
) -> ApiResult<bool> {
    state
        .app_state
        .db
        .set_log_config(&request.config)
        .map_err(ApiError::from_anyhow)?;
    log::set_max_level(request.config.to_level_filter());
    Ok(json_ok(true))
}

async fn get_circuit_breaker_config(
    State(state): State<ApiState>,
) -> ApiResult<crate::proxy::circuit_breaker::CircuitBreakerConfig> {
    let config = state
        .app_state
        .db
        .get_circuit_breaker_config()
        .await
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(config))
}

async fn update_circuit_breaker_config(
    State(state): State<ApiState>,
    Json(request): Json<UpdateCircuitBreakerConfigRequest>,
) -> ApiResult<()> {
    state
        .app_state
        .db
        .update_circuit_breaker_config(&request.config)
        .await
        .map_err(ApiError::from_anyhow)?;
    state
        .app_state
        .proxy_service
        .update_circuit_breaker_configs(&request.config)
        .await
        .map_err(ApiError::from_service_message)?;
    Ok(json_ok(()))
}

async fn import_config_upload(
    State(state): State<ApiState>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut sql_content = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(ApiError::from_anyhow)?
    {
        if field.name() == Some("file") {
            let bytes = field.bytes().await.map_err(ApiError::from_anyhow)?;
            let text = String::from_utf8(bytes.to_vec())
                .map_err(|err| ApiError::bad_request(format!("Invalid UTF-8 SQL upload: {err}")))?;
            sql_content = Some(text);
            break;
        }
    }

    let sql_content =
        sql_content.ok_or_else(|| ApiError::upload_required("Missing upload field: file"))?;
    let db = state.app_state.db.clone();
    let db_for_sync = db.clone();
    let backup_id = tokio::task::spawn_blocking(move || {
        let backup_id = db.import_sql_string(&sql_content)?;
        let warning = match crate::services::ProviderService::sync_current_to_live(
            &crate::store::AppState::new(db_for_sync),
        ) {
            Ok(()) => None,
            Err(err) => Some(err.to_string()),
        };
        Ok::<_, crate::error::AppError>((backup_id, warning))
    })
    .await
    .map_err(ApiError::from_anyhow)?
    .map_err(ApiError::from_anyhow)?;

    Ok(json_ok(json!({
        "success": true,
        "message": "SQL imported successfully",
        "backupId": backup_id.0,
        "warning": backup_id.1,
    })))
}

async fn export_config_download(
    State(state): State<ApiState>,
) -> Result<impl IntoResponse, ApiError> {
    let db = state.app_state.db.clone();
    let sql = tokio::task::spawn_blocking(move || db.export_sql_string())
        .await
        .map_err(ApiError::from_anyhow)?
        .map_err(ApiError::from_anyhow)?;
    let stamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("cc-switch-export-{stamp}.sql");
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/sql; charset=utf-8"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
            .map_err(ApiError::from_anyhow)?,
    );
    Ok((headers, sql))
}
