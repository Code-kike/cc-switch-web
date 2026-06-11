use axum::{extract::State, routing::post, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use std::sync::Arc;

use crate::error::AppError;
use crate::settings::{self, S3SyncSettings};

use super::super::ApiState;
use super::common::{json_ok, ApiError, ApiResult};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct S3TestConnectionRequest {
    settings: S3SyncSettings,
    preserve_empty_password: Option<bool>,
}

fn persist_sync_error(settings: &mut S3SyncSettings, error: &AppError, source: &str) {
    settings.status.last_error = Some(error.to_string());
    settings.status.last_error_source = Some(source.to_string());
    let _ = settings::update_s3_sync_status(settings.status.clone());
}

fn s3_not_configured_error() -> ApiError {
    ApiError::bad_request(
        AppError::localized(
            "s3.sync.not_configured",
            "未配置 S3 同步",
            "S3 sync is not configured.",
        )
        .to_string(),
    )
}

fn s3_sync_disabled_error() -> ApiError {
    ApiError::bad_request(
        AppError::localized("s3.sync.disabled", "S3 同步未启用", "S3 sync is disabled.")
            .to_string(),
    )
}

fn require_enabled_s3_settings() -> Result<S3SyncSettings, ApiError> {
    let settings = settings::get_s3_sync_settings().ok_or_else(s3_not_configured_error)?;
    if !settings.enabled {
        return Err(s3_sync_disabled_error());
    }
    Ok(settings)
}

fn resolve_secret_for_request(
    mut incoming: S3SyncSettings,
    existing: Option<S3SyncSettings>,
    preserve_empty_secret: bool,
) -> S3SyncSettings {
    if let Some(existing_settings) = existing {
        if preserve_empty_secret && incoming.secret_access_key.is_empty() {
            incoming.secret_access_key = existing_settings.secret_access_key;
        }
    }
    incoming
}

async fn run_with_s3_lock<T, Fut>(operation: Fut) -> Result<T, AppError>
where
    Fut: std::future::Future<Output = Result<T, AppError>>,
{
    crate::services::s3_sync::run_with_sync_lock(operation).await
}

fn run_post_import_sync(db: Arc<crate::database::Database>) -> Result<(), AppError> {
    let app_state = crate::store::AppState::new(db);
    crate::services::ProviderService::sync_current_to_live(&app_state)?;
    crate::settings::reload_settings()?;
    Ok(())
}

fn post_sync_warning<E: std::fmt::Display>(err: E) -> String {
    AppError::localized(
        "sync.post_operation_sync_failed",
        format!("后置同步状态失败: {err}"),
        format!("Post-operation synchronization failed: {err}"),
    )
    .to_string()
}

fn post_sync_warning_from_result(result: Result<Result<(), AppError>, String>) -> Option<String> {
    match result {
        Ok(Ok(())) => None,
        Ok(Err(err)) => Some(post_sync_warning(err)),
        Err(err) => Some(post_sync_warning(err)),
    }
}

fn attach_warning(mut value: Value, warning: Option<String>) -> Value {
    if let Some(message) = warning {
        if let Some(obj) = value.as_object_mut() {
            obj.insert("warning".to_string(), Value::String(message));
        }
    }
    value
}

fn map_sync_result<T, F>(result: Result<T, AppError>, on_error: F) -> Result<T, ApiError>
where
    F: FnOnce(&AppError),
{
    match result {
        Ok(value) => Ok(value),
        Err(err) => {
            on_error(&err);
            Err(ApiError::from_anyhow(err))
        }
    }
}

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/s3/s3-test-connection", post(s3_test_connection))
        .route("/s3/s3-sync-upload", post(s3_sync_upload))
        .route("/s3/s3-sync-download", post(s3_sync_download))
        .route(
            "/s3/s3-sync-fetch-remote-info",
            post(s3_sync_fetch_remote_info),
        )
        .with_state(state)
}

async fn s3_test_connection(Json(request): Json<S3TestConnectionRequest>) -> ApiResult<Value> {
    let preserve_empty = request.preserve_empty_password.unwrap_or(true);
    let resolved = resolve_secret_for_request(
        request.settings,
        settings::get_s3_sync_settings(),
        preserve_empty,
    );
    crate::services::s3_sync::check_connection(&resolved)
        .await
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(json!({
        "success": true,
        "message": "S3 connection ok"
    })))
}

async fn s3_sync_upload(State(state): State<ApiState>) -> ApiResult<Value> {
    let db = state.app_state.db.clone();
    let mut settings = require_enabled_s3_settings()?;
    let result = run_with_s3_lock(crate::services::s3_sync::upload(&db, &mut settings)).await;
    map_sync_result(result, |error| {
        persist_sync_error(&mut settings, error, "manual")
    })
    .map(json_ok)
}

async fn s3_sync_download(State(state): State<ApiState>) -> ApiResult<Value> {
    let db = state.app_state.db.clone();
    let db_for_sync = db.clone();
    let mut settings = require_enabled_s3_settings()?;
    let _auto_sync_suppression = crate::services::s3_auto_sync::AutoSyncSuppressionGuard::new();

    let sync_result =
        run_with_s3_lock(crate::services::s3_sync::download(&db, &mut settings)).await;
    let mut result = map_sync_result(sync_result, |error| {
        persist_sync_error(&mut settings, error, "manual")
    })?;

    let warning = post_sync_warning_from_result(
        tokio::task::spawn_blocking(move || run_post_import_sync(db_for_sync))
            .await
            .map_err(|e| e.to_string()),
    );
    result = attach_warning(result, warning);

    Ok(json_ok(result))
}

async fn s3_sync_fetch_remote_info() -> ApiResult<Value> {
    let settings = require_enabled_s3_settings()?;
    let info = crate::services::s3_sync::fetch_remote_info(&settings)
        .await
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(info.unwrap_or(json!({ "empty": true }))))
}
