use axum::{extract::State, routing::post, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use std::sync::Arc;

use crate::error::AppError;
use crate::settings::{self, S3SyncSettings};

use super::super::ApiState;
use super::common::{json_ok, validate_outbound_url, ApiError, ApiResult};

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

/// Normalize an S3 endpoint to a full URL with a scheme, mirroring
/// `services::s3::split_scheme_host`: a bare `minio.example.com:9000` is treated
/// as `https://minio.example.com:9000`. Without this, `Url::parse` reads the
/// host as the scheme (`minio.example.com:9000` → scheme `minio.example.com`)
/// and `validate_outbound_url` 400s a perfectly valid MinIO endpoint (P4-B
/// regression).
fn normalize_s3_endpoint_for_guard(endpoint: &str) -> String {
    let trimmed = endpoint.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    }
}

/// Audit F3: reject custom S3 endpoints that resolve to internal/private targets
/// before dialing. An empty endpoint means the AWS default (public) — nothing to
/// guard. Web-server only; the shared sync service stays unrestricted for desktop.
async fn guard_s3_endpoint(settings: &S3SyncSettings) -> Result<(), ApiError> {
    if settings.endpoint.trim().is_empty() {
        return Ok(());
    }
    // P4-B: schemeless endpoints (the documented MinIO `host:port` form) must be
    // normalized to a scheme so the guard validates the host, not a phantom one.
    let normalized = normalize_s3_endpoint_for_guard(&settings.endpoint);
    validate_outbound_url(&normalized).await
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
    guard_s3_endpoint(&resolved).await?;
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
    guard_s3_endpoint(&settings).await?;
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
    guard_s3_endpoint(&settings).await?;
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
    guard_s3_endpoint(&settings).await?;
    let info = crate::services::s3_sync::fetch_remote_info(&settings)
        .await
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(info.unwrap_or(json!({ "empty": true }))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings_with_endpoint(endpoint: &str) -> S3SyncSettings {
        S3SyncSettings {
            endpoint: endpoint.to_string(),
            ..S3SyncSettings::default()
        }
    }

    #[test]
    fn normalize_defaults_schemeless_to_https() {
        // P4-B: a bare `host:port` must become a parseable https URL, not be
        // mis-read as scheme `minio.example.com`.
        assert_eq!(
            normalize_s3_endpoint_for_guard("minio.example.com:9000"),
            "https://minio.example.com:9000"
        );
        assert_eq!(
            normalize_s3_endpoint_for_guard("  minio.local:9000  "),
            "https://minio.local:9000"
        );
        // Explicit schemes are preserved untouched.
        assert_eq!(
            normalize_s3_endpoint_for_guard("http://minio:9000"),
            "http://minio:9000"
        );
        assert_eq!(
            normalize_s3_endpoint_for_guard("https://storage.example.com"),
            "https://storage.example.com"
        );
    }

    #[tokio::test]
    async fn guard_accepts_schemeless_public_endpoint() {
        // P4-B regression: a schemeless endpoint pointing at a public address
        // must pass the guard (it previously 400'd on a phantom scheme). Use a
        // public IP literal so the assertion does not depend on DNS.
        let settings = settings_with_endpoint("1.1.1.1:9000");
        assert!(guard_s3_endpoint(&settings).await.is_ok());
    }

    #[tokio::test]
    async fn guard_blocks_schemeless_internal_endpoint() {
        // The normalization must not weaken the guard: a schemeless loopback
        // endpoint is still rejected.
        let settings = settings_with_endpoint("127.0.0.1:9000");
        assert!(guard_s3_endpoint(&settings).await.is_err());
    }

    #[tokio::test]
    async fn guard_skips_empty_endpoint() {
        // Empty endpoint = AWS default (public); nothing to guard.
        let settings = settings_with_endpoint("");
        assert!(guard_s3_endpoint(&settings).await.is_ok());
    }
}
