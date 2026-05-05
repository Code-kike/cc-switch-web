use axum::{
    extract::{Query, State},
    routing::{delete, get, put},
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
struct SetOpenClawEnvRequest {
    env: crate::openclaw_config::OpenClawEnvConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteEnvVarsRequest {
    conflicts: Vec<crate::services::env_checker::EnvConflict>,
}

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/env/check-env-conflicts", get(check_env_conflicts))
        .route("/env/delete-env-vars", delete(delete_env_vars))
        .route("/env/platform", get(platform_passthrough))
        .route("/env/get-openclaw-env", get(get_openclaw_env))
        .route("/env/set-openclaw-env", put(set_openclaw_env))
        .with_state(state)
}

async fn check_env_conflicts(
    Query(query): Query<AppQuery>,
) -> ApiResult<Vec<crate::services::env_checker::EnvConflict>> {
    let conflicts = crate::services::env_checker::check_env_conflicts(&query.app)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(conflicts))
}

async fn delete_env_vars(
    Json(request): Json<DeleteEnvVarsRequest>,
) -> ApiResult<crate::services::env_manager::BackupInfo> {
    let backup = crate::services::env_manager::delete_env_vars(request.conflicts)
        .map_err(ApiError::bad_request)?;
    Ok(json_ok(backup))
}

async fn platform_passthrough() -> ApiResult<serde_json::Value> {
    let (os, is_wsl) = detect_platform();
    let home = crate::config::get_home_dir();
    Ok(json_ok(serde_json::json!({
        "os": os,
        "isWsl": is_wsl,
        "defaultPaths": {
            "appConfig": home.join(".cc-switch").to_string_lossy().to_string(),
            "claude": home.join(".claude").to_string_lossy().to_string(),
            "codex": home.join(".codex").to_string_lossy().to_string(),
            "gemini": home.join(".gemini").to_string_lossy().to_string(),
            "opencode": home.join(".config").join("opencode").to_string_lossy().to_string(),
            "openclaw": home.join(".openclaw").to_string_lossy().to_string(),
            "hermes": home.join(".hermes").to_string_lossy().to_string(),
            "omo": home.join(".config").join("opencode").to_string_lossy().to_string(),
        }
    })))
}

async fn get_openclaw_env() -> ApiResult<crate::openclaw_config::OpenClawEnvConfig> {
    let env = crate::openclaw_config::get_env_config().map_err(ApiError::from_anyhow)?;
    Ok(json_ok(env))
}

async fn set_openclaw_env(
    State(_state): State<ApiState>,
    Json(request): Json<SetOpenClawEnvRequest>,
) -> ApiResult<crate::openclaw_config::OpenClawWriteOutcome> {
    let result =
        crate::openclaw_config::set_env_config(&request.env).map_err(ApiError::from_anyhow)?;
    Ok(json_ok(result))
}

#[cfg(target_os = "linux")]
fn detect_platform() -> (&'static str, bool) {
    let is_wsl = std::fs::read_to_string("/proc/version")
        .map(|s| s.to_lowercase().contains("microsoft"))
        .unwrap_or(false);
    ("linux", is_wsl)
}

#[cfg(target_os = "macos")]
fn detect_platform() -> (&'static str, bool) {
    ("macos", false)
}

#[cfg(target_os = "windows")]
fn detect_platform() -> (&'static str, bool) {
    ("windows", false)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn detect_platform() -> (&'static str, bool) {
    ("unknown", false)
}
