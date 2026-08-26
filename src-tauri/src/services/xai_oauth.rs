use std::time::Duration;

use serde::Deserialize;

use crate::proxy::providers::xai_oauth_auth::XaiOAuthManager;
use crate::proxy::providers::XAI_API_BASE_URL;
use crate::services::model_fetch::FetchedModel;
use crate::services::subscription::{CredentialStatus, SubscriptionQuota};

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    #[serde(default)]
    data: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    id: String,
    #[serde(default)]
    owned_by: Option<String>,
}

/// Query the SuperGrok subscription quota for a managed xAI OAuth account.
///
/// Keep this in the tauri-free service layer so the desktop command, Web API,
/// and provider usage/tray paths all use the same account resolution and
/// credential error semantics.
pub async fn query_quota(
    manager: &XaiOAuthManager,
    account_id: Option<&str>,
) -> Result<SubscriptionQuota, String> {
    let resolved = match account_id.map(str::trim).filter(|id| !id.is_empty()) {
        Some(id) => Some(id.to_string()),
        None => manager.default_account_id().await,
    };
    let Some(id) = resolved else {
        return Ok(SubscriptionQuota::not_found("xai_oauth"));
    };

    let token = match manager.get_valid_token_for_account(&id).await {
        Ok(token) => token,
        Err(error) => {
            return Ok(SubscriptionQuota::error(
                "xai_oauth",
                CredentialStatus::Expired,
                format!("xAI OAuth token unavailable: {error}"),
            ));
        }
    };

    crate::services::subscription_grok::query_grok_quota(
        &token,
        "xai_oauth",
        "Please re-login via cc-switch.",
    )
    .await
}

pub async fn fetch_models(
    manager: &XaiOAuthManager,
    account_id: Option<&str>,
) -> Result<Vec<FetchedModel>, String> {
    let resolved = match account_id.map(str::trim).filter(|id| !id.is_empty()) {
        Some(id) => Some(id.to_string()),
        None => manager.default_account_id().await,
    };
    let account_id = resolved.ok_or_else(|| "No usable xAI account available".to_string())?;
    let token = manager
        .get_valid_token_for_account(&account_id)
        .await
        .map_err(|error| format!("xAI OAuth token unavailable: {error}"))?;

    let response = crate::proxy::http_client::get()
        .get(format!("{XAI_API_BASE_URL}/models"))
        .bearer_auth(token)
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|error| format!("xAI models request failed: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("xAI models request failed: HTTP {status}"));
    }
    let payload: ModelsResponse = response
        .json()
        .await
        .map_err(|_| "xAI models response was not valid JSON".to_string())?;
    let mut models: Vec<FetchedModel> = payload
        .data
        .into_iter()
        .map(|model| FetchedModel {
            id: model.id,
            owned_by: model.owned_by,
        })
        .collect();
    models.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(models)
}
