//! xAI OAuth state and xAI-specific commands.

use crate::proxy::providers::xai_oauth_auth::XaiOAuthManager;
use crate::services::model_fetch::FetchedModel;
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

pub struct XaiOAuthState(pub Arc<RwLock<XaiOAuthManager>>);

#[tauri::command(rename_all = "camelCase")]
pub async fn get_xai_oauth_models(
    account_id: Option<String>,
    state: State<'_, XaiOAuthState>,
) -> Result<Vec<FetchedModel>, String> {
    let manager = state.0.read().await;
    crate::services::xai_oauth::fetch_models(&manager, account_id.as_deref()).await
}
