//! xAI OAuth state and xAI-specific commands.

use crate::proxy::providers::xai_oauth_auth::XaiOAuthManager;
use crate::services::model_fetch::FetchedModel;
use crate::services::subscription::SubscriptionQuota;
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

pub struct XaiOAuthState(pub Arc<RwLock<XaiOAuthManager>>);

/// Query the quota belonging to the selected managed SuperGrok account.
/// The shared service keeps this command equivalent to the Web handler and
/// usage/tray refresh paths.
#[tauri::command(rename_all = "camelCase")]
pub async fn get_xai_oauth_quota(
    account_id: Option<String>,
    state: State<'_, XaiOAuthState>,
) -> Result<SubscriptionQuota, String> {
    let manager = state.0.read().await;
    crate::services::xai_oauth::query_quota(&manager, account_id.as_deref()).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_xai_oauth_models(
    account_id: Option<String>,
    state: State<'_, XaiOAuthState>,
) -> Result<Vec<FetchedModel>, String> {
    let manager = state.0.read().await;
    crate::services::xai_oauth::fetch_models(&manager, account_id.as_deref()).await
}
