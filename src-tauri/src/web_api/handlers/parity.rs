//! Parity catch-all routes for mapped Web commands that have not been migrated
//! to concrete handlers yet.
//!
//! These routes deliberately return Web-specific errors instead of falling
//! through to the generic `/api` 404. Concrete handlers should keep owning real
//! implementations; this module only closes UI-visible gaps while migration
//! continues page by page.

use axum::{
    routing::{any, get, post},
    Router,
};

use super::super::ApiState;
use super::common::{web_desktop_only, web_not_supported, web_upload_required};

pub fn router(state: ApiState) -> Router {
    Router::new()
        // Browser upload/download replacements.
        .route("/config/export-config-to-file", post(web_upload_required))
        .route("/config/import-config-from-file", post(web_upload_required))
        .route(
            "/prompts/import-prompt-from-file",
            post(web_upload_required),
        )
        .route("/skills/install-skills-from-zip", post(web_upload_required))
        .route("/system/open_file_dialog", post(web_upload_required))
        .route("/system/open_zip_file_dialog", post(web_upload_required))
        .route("/system/save_file_dialog", post(web_upload_required))
        // Desktop shell / OS integration.
        .route("/config/open-app-config-folder", post(web_desktop_only))
        .route("/config/open-config-folder", post(web_desktop_only))
        .route("/providers/open-provider-terminal", post(web_desktop_only))
        .route("/system/pick_directory", post(web_desktop_only))
        .route(
            "/workspace/open-workspace-directory",
            post(web_desktop_only),
        )
        // Resource-level fallbacks for command paths still under migration.
        .route("/auth/*path", any(web_not_supported))
        .route("/backups/*path", any(web_not_supported))
        .route("/config/*path", any(web_not_supported))
        .route("/deeplink/*path", any(web_not_supported))
        .route("/env/*path", any(web_not_supported))
        .route("/failover/*path", any(web_not_supported))
        .route("/global-proxy/*path", any(web_not_supported))
        .route("/mcp/*path", any(web_not_supported))
        .route("/omo/*path", any(web_not_supported))
        .route("/prompts/*path", any(web_not_supported))
        .route("/providers/*path", any(web_not_supported))
        .route("/proxy/*path", any(web_not_supported))
        .route("/sessions/*path", any(web_not_supported))
        .route("/settings/*path", any(web_not_supported))
        .route("/skills/*path", any(web_not_supported))
        .route("/subscription/*path", any(web_not_supported))
        .route("/system/*path", any(web_not_supported))
        .route("/usage/*path", any(web_not_supported))
        .route("/webdav/*path", any(web_not_supported))
        .route("/workspace/*path", any(web_not_supported))
        .route("/healthz/parity", get(|| async { "ok" }))
        .with_state(state)
}
