use axum::{
    extract::State,
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;

use super::super::ApiState;
use super::common::{json_ok, ApiError, ApiResult};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveSettingsRequest {
    settings: crate::settings::AppSettings,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveWebDavSyncSettingsRequest {
    settings: crate::settings::WebDavSyncSettings,
    password_touched: Option<bool>,
}

fn merge_settings_for_save(
    mut incoming: crate::settings::AppSettings,
    existing: &crate::settings::AppSettings,
) -> crate::settings::AppSettings {
    match (&mut incoming.webdav_sync, &existing.webdav_sync) {
        (None, _) => {
            incoming.webdav_sync = existing.webdav_sync.clone();
        }
        (Some(incoming_sync), Some(existing_sync))
            if incoming_sync.password.is_empty() && !existing_sync.password.is_empty() =>
        {
            incoming_sync.password = existing_sync.password.clone();
        }
        _ => {}
    }
    incoming
}

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/settings/get-settings", get(get_settings))
        .route("/settings/save-settings", put(save_settings))
        .route(
            "/settings/webdav-sync-save-settings",
            post(save_webdav_sync_settings),
        )
        .with_state(state)
}

async fn get_settings() -> ApiResult<crate::settings::AppSettings> {
    Ok(json_ok(crate::settings::get_settings_for_frontend()))
}

async fn save_settings(
    State(_state): State<ApiState>,
    Json(request): Json<SaveSettingsRequest>,
) -> Result<Json<bool>, ApiError> {
    let existing = crate::settings::get_settings();
    let merged = merge_settings_for_save(request.settings, &existing);
    crate::settings::update_settings(merged).map_err(ApiError::from_anyhow)?;
    Ok(json_ok(true))
}

async fn save_webdav_sync_settings(
    State(_state): State<ApiState>,
    Json(request): Json<SaveWebDavSyncSettingsRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let password_touched = request.password_touched.unwrap_or(false);
    let existing = crate::settings::get_webdav_sync_settings();
    let mut sync_settings = request.settings;

    if let Some(existing_settings) = existing.clone() {
        if !password_touched && sync_settings.password.is_empty() {
            sync_settings.password = existing_settings.password;
        }
        sync_settings.status = existing_settings.status;
    }

    sync_settings.normalize();
    sync_settings.validate().map_err(ApiError::from_anyhow)?;
    crate::settings::set_webdav_sync_settings(Some(sync_settings))
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(json!({ "success": true })))
}
