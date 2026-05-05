use axum::{extract::State, routing::post, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use std::sync::Arc;

use crate::error::AppError;
use crate::settings::{self, WebDavSyncSettings};

use super::super::ApiState;
use super::common::{json_ok, ApiError, ApiResult};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebDavTestConnectionRequest {
    settings: WebDavSyncSettings,
    preserve_empty_password: Option<bool>,
}

fn persist_sync_error(settings: &mut WebDavSyncSettings, error: &AppError, source: &str) {
    settings.status.last_error = Some(error.to_string());
    settings.status.last_error_source = Some(source.to_string());
    let _ = settings::update_webdav_sync_status(settings.status.clone());
}

fn webdav_not_configured_error() -> ApiError {
    ApiError::bad_request(
        AppError::localized(
            "webdav.sync.not_configured",
            "未配置 WebDAV 同步",
            "WebDAV sync is not configured.",
        )
        .to_string(),
    )
}

fn webdav_sync_disabled_error() -> ApiError {
    ApiError::bad_request(
        AppError::localized(
            "webdav.sync.disabled",
            "WebDAV 同步未启用",
            "WebDAV sync is disabled.",
        )
        .to_string(),
    )
}

fn require_enabled_webdav_settings() -> Result<WebDavSyncSettings, ApiError> {
    let settings = settings::get_webdav_sync_settings().ok_or_else(webdav_not_configured_error)?;
    if !settings.enabled {
        return Err(webdav_sync_disabled_error());
    }
    Ok(settings)
}

fn resolve_password_for_request(
    mut incoming: WebDavSyncSettings,
    existing: Option<WebDavSyncSettings>,
    preserve_empty_password: bool,
) -> WebDavSyncSettings {
    if let Some(existing_settings) = existing {
        if preserve_empty_password && incoming.password.is_empty() {
            incoming.password = existing_settings.password;
        }
    }
    incoming
}

async fn run_with_webdav_lock<T, Fut>(operation: Fut) -> Result<T, AppError>
where
    Fut: std::future::Future<Output = Result<T, AppError>>,
{
    crate::services::webdav_sync::run_with_sync_lock(operation).await
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
        .route(
            "/webdav/webdav-test-connection",
            post(webdav_test_connection),
        )
        .route("/webdav/webdav-sync-upload", post(webdav_sync_upload))
        .route("/webdav/webdav-sync-download", post(webdav_sync_download))
        .route(
            "/webdav/webdav-sync-fetch-remote-info",
            post(webdav_sync_fetch_remote_info),
        )
        .with_state(state)
}

async fn webdav_test_connection(
    Json(request): Json<WebDavTestConnectionRequest>,
) -> ApiResult<Value> {
    let preserve_empty = request.preserve_empty_password.unwrap_or(true);
    let resolved = resolve_password_for_request(
        request.settings,
        settings::get_webdav_sync_settings(),
        preserve_empty,
    );
    crate::services::webdav_sync::check_connection(&resolved)
        .await
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(json!({
        "success": true,
        "message": "WebDAV connection ok"
    })))
}

async fn webdav_sync_upload(State(state): State<ApiState>) -> ApiResult<Value> {
    let db = state.app_state.db.clone();
    let mut settings = require_enabled_webdav_settings()?;
    let result =
        run_with_webdav_lock(crate::services::webdav_sync::upload(&db, &mut settings)).await;
    map_sync_result(result, |error| {
        persist_sync_error(&mut settings, error, "manual")
    })
    .map(json_ok)
}

async fn webdav_sync_download(State(state): State<ApiState>) -> ApiResult<Value> {
    let db = state.app_state.db.clone();
    let db_for_sync = db.clone();
    let mut settings = require_enabled_webdav_settings()?;
    let _auto_sync_suppression = crate::services::webdav_auto_sync::AutoSyncSuppressionGuard::new();

    let sync_result =
        run_with_webdav_lock(crate::services::webdav_sync::download(&db, &mut settings)).await;
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

async fn webdav_sync_fetch_remote_info() -> ApiResult<Value> {
    let settings = require_enabled_webdav_settings()?;
    let info = crate::services::webdav_sync::fetch_remote_info(&settings)
        .await
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(info.unwrap_or(json!({ "empty": true }))))
}
