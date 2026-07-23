use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

use super::super::ApiState;
use super::common::{json_ok, ApiError, ApiResult};

const AUTH_PROVIDER_GITHUB_COPILOT: &str = "github_copilot";
const AUTH_PROVIDER_CODEX_OAUTH: &str = "codex_oauth";
const AUTH_PROVIDER_XAI_OAUTH: &str = "xai_oauth";

#[derive(Debug, Clone, serde::Serialize)]
pub struct ManagedAuthAccount {
    pub id: String,
    pub provider: String,
    pub login: String,
    pub avatar_url: Option<String>,
    pub authenticated_at: i64,
    pub is_default: bool,
    pub github_domain: String,
    pub requires_reauth: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ManagedAuthStatus {
    pub provider: String,
    pub authenticated: bool,
    pub default_account_id: Option<String>,
    pub migration_error: Option<String>,
    pub accounts: Vec<ManagedAuthAccount>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ManagedAuthDeviceCodeResponse {
    pub provider: String,
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthProviderRequest {
    auth_provider: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthStartLoginRequest {
    auth_provider: String,
    github_domain: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthPollRequest {
    auth_provider: String,
    device_code: String,
    github_domain: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthAccountRequest {
    auth_provider: String,
    account_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CodexOauthModelsQuery {
    account_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CodexOauthQuotaQuery {
    account_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct XaiOauthModelsQuery {
    account_id: Option<String>,
}

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/auth/auth-start-login", post(auth_start_login))
        .route("/auth/auth-poll-for-account", post(auth_poll_for_account))
        .route("/auth/auth-list-accounts", post(auth_list_accounts))
        .route("/auth/auth-get-status", post(auth_get_status))
        .route("/auth/auth-remove-account", post(auth_remove_account))
        .route(
            "/auth/auth-set-default-account",
            post(auth_set_default_account),
        )
        .route("/auth/auth-logout", post(auth_logout))
        .route("/auth/get-codex-oauth-models", get(get_codex_oauth_models))
        .route("/auth/get-codex-oauth-quota", get(get_codex_oauth_quota))
        .route("/auth/get-xai-oauth-models", get(get_xai_oauth_models))
        .route("/auth/get-xai-oauth-quota", get(get_xai_oauth_quota))
        .with_state(state)
}

fn ensure_auth_provider(auth_provider: &str) -> Result<&'static str, ApiError> {
    match auth_provider {
        AUTH_PROVIDER_GITHUB_COPILOT => Ok(AUTH_PROVIDER_GITHUB_COPILOT),
        AUTH_PROVIDER_CODEX_OAUTH => Ok(AUTH_PROVIDER_CODEX_OAUTH),
        AUTH_PROVIDER_XAI_OAUTH => Ok(AUTH_PROVIDER_XAI_OAUTH),
        _ => Err(ApiError::bad_request(format!(
            "Unsupported auth provider: {auth_provider}"
        ))),
    }
}

fn map_account(
    provider: &str,
    account: crate::proxy::providers::copilot_auth::GitHubAccount,
    default_account_id: Option<&str>,
) -> ManagedAuthAccount {
    ManagedAuthAccount {
        is_default: default_account_id == Some(account.id.as_str()),
        id: account.id,
        provider: provider.to_string(),
        login: account.login,
        avatar_url: account.avatar_url,
        authenticated_at: account.authenticated_at,
        github_domain: account.github_domain,
        requires_reauth: false,
    }
}

fn map_xai_account(
    account: crate::proxy::providers::xai_oauth_auth::XaiOAuthAccount,
    default_account_id: Option<&str>,
) -> ManagedAuthAccount {
    ManagedAuthAccount {
        is_default: default_account_id == Some(account.id.as_str()),
        id: account.id,
        provider: AUTH_PROVIDER_XAI_OAUTH.to_string(),
        login: account.login,
        avatar_url: account.avatar_url,
        authenticated_at: account.authenticated_at,
        github_domain: account.github_domain,
        requires_reauth: account.requires_reauth,
    }
}

fn map_xai_status(
    status: crate::proxy::providers::xai_oauth_auth::XaiOAuthStatus,
) -> ManagedAuthStatus {
    let default_account_id = status.default_account_id.clone();
    ManagedAuthStatus {
        provider: AUTH_PROVIDER_XAI_OAUTH.to_string(),
        authenticated: status.authenticated,
        default_account_id: default_account_id.clone(),
        migration_error: None,
        accounts: status
            .accounts
            .into_iter()
            .map(|account| map_xai_account(account, default_account_id.as_deref()))
            .collect(),
    }
}

fn map_device_code_response(
    provider: &str,
    response: crate::proxy::providers::copilot_auth::GitHubDeviceCodeResponse,
) -> ManagedAuthDeviceCodeResponse {
    ManagedAuthDeviceCodeResponse {
        provider: provider.to_string(),
        device_code: response.device_code,
        user_code: response.user_code,
        verification_uri: response.verification_uri,
        expires_in: response.expires_in,
        interval: response.interval,
    }
}

async fn auth_start_login(
    State(state): State<ApiState>,
    Json(request): Json<AuthStartLoginRequest>,
) -> ApiResult<ManagedAuthDeviceCodeResponse> {
    let auth_provider = ensure_auth_provider(&request.auth_provider)?;
    let response = match auth_provider {
        AUTH_PROVIDER_GITHUB_COPILOT => {
            let manager = state.copilot_auth.read().await;
            manager
                .start_device_flow(request.github_domain.as_deref())
                .await
                .map_err(ApiError::from_anyhow)?
        }
        AUTH_PROVIDER_CODEX_OAUTH => {
            let manager = state.codex_oauth.read().await;
            manager
                .start_device_flow()
                .await
                .map_err(ApiError::from_anyhow)?
        }
        AUTH_PROVIDER_XAI_OAUTH => {
            let manager = state.xai_oauth.read().await;
            manager
                .start_device_flow()
                .await
                .map_err(ApiError::from_anyhow)?
        }
        _ => unreachable!(),
    };
    Ok(json_ok(map_device_code_response(auth_provider, response)))
}

async fn auth_poll_for_account(
    State(state): State<ApiState>,
    Json(request): Json<AuthPollRequest>,
) -> ApiResult<Option<ManagedAuthAccount>> {
    let auth_provider = ensure_auth_provider(&request.auth_provider)?;
    let account = match auth_provider {
        AUTH_PROVIDER_GITHUB_COPILOT => {
            let manager = state.copilot_auth.write().await;
            match manager
                .poll_for_token(&request.device_code, request.github_domain.as_deref())
                .await
            {
                Ok(account) => {
                    let default_account_id = manager.get_status().await.default_account_id;
                    account.map(|account| {
                        map_account(auth_provider, account, default_account_id.as_deref())
                    })
                }
                Err(
                    crate::proxy::providers::copilot_auth::CopilotAuthError::AuthorizationPending,
                ) => None,
                Err(err) => return Err(ApiError::from_anyhow(err)),
            }
        }
        AUTH_PROVIDER_CODEX_OAUTH => {
            let manager = state.codex_oauth.write().await;
            match manager.poll_for_token(&request.device_code).await {
                Ok(account) => {
                    let default_account_id = manager.get_status().await.default_account_id;
                    account.map(|account| {
                        map_account(auth_provider, account, default_account_id.as_deref())
                    })
                }
                Err(crate::proxy::providers::codex_oauth_auth::CodexOAuthError::AuthorizationPending) => None,
                Err(err) => return Err(ApiError::from_anyhow(err)),
            }
        }
        AUTH_PROVIDER_XAI_OAUTH => {
            let manager = state.xai_oauth.write().await;
            match manager.poll_for_token(&request.device_code).await {
                Ok(account) => {
                    let default_account_id = manager.get_status().await.default_account_id;
                    account.map(|account| map_xai_account(account, default_account_id.as_deref()))
                }
                Err(
                    crate::proxy::providers::xai_oauth_auth::XaiOAuthError::AuthorizationPending,
                ) => None,
                Err(err) => return Err(ApiError::from_anyhow(err)),
            }
        }
        _ => unreachable!(),
    };

    Ok(json_ok(account))
}

async fn auth_list_accounts(
    State(state): State<ApiState>,
    Json(request): Json<AuthProviderRequest>,
) -> ApiResult<Vec<ManagedAuthAccount>> {
    let auth_provider = ensure_auth_provider(&request.auth_provider)?;
    let accounts = match auth_provider {
        AUTH_PROVIDER_GITHUB_COPILOT => {
            let manager = state.copilot_auth.read().await;
            let status = manager.get_status().await;
            let default_account_id = status.default_account_id.clone();
            status
                .accounts
                .into_iter()
                .map(|account| map_account(auth_provider, account, default_account_id.as_deref()))
                .collect()
        }
        AUTH_PROVIDER_CODEX_OAUTH => {
            let manager = state.codex_oauth.read().await;
            let status = manager.get_status().await;
            let default_account_id = status.default_account_id.clone();
            status
                .accounts
                .into_iter()
                .map(|account| map_account(auth_provider, account, default_account_id.as_deref()))
                .collect()
        }
        AUTH_PROVIDER_XAI_OAUTH => {
            let manager = state.xai_oauth.read().await;
            let status = manager.get_status().await;
            let default_account_id = status.default_account_id.clone();
            status
                .accounts
                .into_iter()
                .map(|account| map_xai_account(account, default_account_id.as_deref()))
                .collect()
        }
        _ => unreachable!(),
    };
    Ok(json_ok(accounts))
}

async fn auth_get_status(
    State(state): State<ApiState>,
    Json(request): Json<AuthProviderRequest>,
) -> ApiResult<ManagedAuthStatus> {
    let auth_provider = ensure_auth_provider(&request.auth_provider)?;
    let status = match auth_provider {
        AUTH_PROVIDER_GITHUB_COPILOT => {
            let manager = state.copilot_auth.read().await;
            let status = manager.get_status().await;
            let default_account_id = status.default_account_id.clone();
            ManagedAuthStatus {
                provider: auth_provider.to_string(),
                authenticated: status.authenticated,
                default_account_id: default_account_id.clone(),
                migration_error: status.migration_error,
                accounts: status
                    .accounts
                    .into_iter()
                    .map(|account| {
                        map_account(auth_provider, account, default_account_id.as_deref())
                    })
                    .collect(),
            }
        }
        AUTH_PROVIDER_CODEX_OAUTH => {
            let manager = state.codex_oauth.read().await;
            let status = manager.get_status().await;
            let default_account_id = status.default_account_id.clone();
            ManagedAuthStatus {
                provider: auth_provider.to_string(),
                authenticated: status.authenticated,
                default_account_id: default_account_id.clone(),
                migration_error: None,
                accounts: status
                    .accounts
                    .into_iter()
                    .map(|account| {
                        map_account(auth_provider, account, default_account_id.as_deref())
                    })
                    .collect(),
            }
        }
        AUTH_PROVIDER_XAI_OAUTH => {
            let manager = state.xai_oauth.read().await;
            map_xai_status(manager.get_status().await)
        }
        _ => unreachable!(),
    };
    Ok(json_ok(status))
}

async fn auth_remove_account(
    State(state): State<ApiState>,
    Json(request): Json<AuthAccountRequest>,
) -> ApiResult<()> {
    let auth_provider = ensure_auth_provider(&request.auth_provider)?;
    match auth_provider {
        AUTH_PROVIDER_GITHUB_COPILOT => {
            let manager = state.copilot_auth.write().await;
            manager
                .remove_account(&request.account_id)
                .await
                .map_err(ApiError::from_anyhow)?;
        }
        AUTH_PROVIDER_CODEX_OAUTH => {
            let manager = state.codex_oauth.write().await;
            manager
                .remove_account(&request.account_id)
                .await
                .map_err(ApiError::from_anyhow)?;
        }
        AUTH_PROVIDER_XAI_OAUTH => {
            let manager = state.xai_oauth.write().await;
            manager
                .remove_account(&request.account_id)
                .await
                .map_err(ApiError::from_anyhow)?;
        }
        _ => unreachable!(),
    }
    Ok(json_ok(()))
}

async fn auth_set_default_account(
    State(state): State<ApiState>,
    Json(request): Json<AuthAccountRequest>,
) -> ApiResult<()> {
    let auth_provider = ensure_auth_provider(&request.auth_provider)?;
    match auth_provider {
        AUTH_PROVIDER_GITHUB_COPILOT => {
            let manager = state.copilot_auth.write().await;
            manager
                .set_default_account(&request.account_id)
                .await
                .map_err(ApiError::from_anyhow)?;
        }
        AUTH_PROVIDER_CODEX_OAUTH => {
            let manager = state.codex_oauth.write().await;
            manager
                .set_default_account(&request.account_id)
                .await
                .map_err(ApiError::from_anyhow)?;
        }
        AUTH_PROVIDER_XAI_OAUTH => {
            let manager = state.xai_oauth.write().await;
            manager
                .set_default_account(&request.account_id)
                .await
                .map_err(ApiError::from_anyhow)?;
        }
        _ => unreachable!(),
    }
    Ok(json_ok(()))
}

async fn auth_logout(
    State(state): State<ApiState>,
    Json(request): Json<AuthProviderRequest>,
) -> ApiResult<()> {
    let auth_provider = ensure_auth_provider(&request.auth_provider)?;
    match auth_provider {
        AUTH_PROVIDER_GITHUB_COPILOT => {
            let manager = state.copilot_auth.write().await;
            manager.clear_auth().await.map_err(ApiError::from_anyhow)?;
        }
        AUTH_PROVIDER_CODEX_OAUTH => {
            let manager = state.codex_oauth.write().await;
            manager.clear_auth().await.map_err(ApiError::from_anyhow)?;
        }
        AUTH_PROVIDER_XAI_OAUTH => {
            let manager = state.xai_oauth.write().await;
            manager.clear_auth().await.map_err(ApiError::from_anyhow)?;
        }
        _ => unreachable!(),
    }
    Ok(json_ok(()))
}

async fn get_codex_oauth_models(
    State(state): State<ApiState>,
    Query(query): Query<CodexOauthModelsQuery>,
) -> ApiResult<Vec<crate::services::model_fetch::FetchedModel>> {
    let manager = state.codex_oauth.read().await;
    let resolved = match query
        .account_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    {
        Some(id) => Some(id.to_string()),
        None => manager.default_account_id().await,
    };
    let Some(id) = resolved else {
        return Err(ApiError::bad_request("No ChatGPT account available"));
    };

    let token = manager
        .get_valid_token_for_account(&id)
        .await
        .map_err(|e| ApiError::bad_request(format!("Codex OAuth token unavailable: {e}")))?;

    let models = crate::services::codex_oauth_models::fetch_models_with_token(&token, &id)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(json_ok(models))
}

async fn get_xai_oauth_models(
    State(state): State<ApiState>,
    Query(query): Query<XaiOauthModelsQuery>,
) -> ApiResult<Vec<crate::services::model_fetch::FetchedModel>> {
    let manager = state.xai_oauth.read().await;
    let models = crate::services::xai_oauth::fetch_models(&manager, query.account_id.as_deref())
        .await
        .map_err(ApiError::bad_request)?;
    Ok(json_ok(models))
}

async fn get_xai_oauth_quota(
    State(state): State<ApiState>,
    Query(query): Query<XaiOauthModelsQuery>,
) -> ApiResult<crate::services::subscription::SubscriptionQuota> {
    let manager = state.xai_oauth.read().await;
    let quota = crate::services::xai_oauth::query_quota(&manager, query.account_id.as_deref())
        .await
        .map_err(ApiError::from_service_message)?;
    Ok(json_ok(quota))
}

async fn get_codex_oauth_quota(
    State(state): State<ApiState>,
    Query(query): Query<CodexOauthQuotaQuery>,
) -> ApiResult<crate::services::subscription::SubscriptionQuota> {
    use crate::services::subscription::{CredentialStatus, SubscriptionQuota};

    let manager = state.codex_oauth.read().await;

    // 解析最终使用的账号 ID：显式 > 默认账号 > 无账号 (not_found)
    let resolved = match query
        .account_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    {
        Some(id) => Some(id.to_string()),
        None => manager.default_account_id().await,
    };
    let Some(id) = resolved else {
        return Ok(json_ok(SubscriptionQuota::not_found("codex_oauth")));
    };

    // 获取（必要时自动刷新）access_token
    let token = match manager.get_valid_token_for_account(&id).await {
        Ok(t) => t,
        Err(e) => {
            return Ok(json_ok(SubscriptionQuota::error(
                "codex_oauth",
                CredentialStatus::Expired,
                format!("Codex OAuth token unavailable: {e}"),
            )));
        }
    };

    let quota = crate::services::subscription::query_codex_quota(
        &token,
        Some(&id),
        "codex_oauth",
        "Codex OAuth access token expired or rejected. Please re-login via cc-switch.",
    )
    .await
    .map_err(super::common::ApiError::from_service_message)?;
    Ok(json_ok(quota))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::providers::xai_oauth_auth::{XaiOAuthAccount, XaiOAuthStatus};

    #[test]
    fn xai_auth_provider_is_supported_by_web_dispatch() {
        assert_eq!(
            ensure_auth_provider(AUTH_PROVIDER_XAI_OAUTH).unwrap(),
            AUTH_PROVIDER_XAI_OAUTH
        );
    }

    #[test]
    fn xai_status_mapping_preserves_default_and_reauth_state() {
        let status = map_xai_status(XaiOAuthStatus {
            authenticated: true,
            default_account_id: Some("ready".to_string()),
            username: Some("ready@example.com".to_string()),
            accounts: vec![
                XaiOAuthAccount {
                    id: "ready".to_string(),
                    login: "ready@example.com".to_string(),
                    avatar_url: None,
                    authenticated_at: 20,
                    github_domain: "x.ai".to_string(),
                    requires_reauth: false,
                },
                XaiOAuthAccount {
                    id: "expired".to_string(),
                    login: "expired@example.com".to_string(),
                    avatar_url: None,
                    authenticated_at: 10,
                    github_domain: "x.ai".to_string(),
                    requires_reauth: true,
                },
            ],
        });

        assert_eq!(status.provider, AUTH_PROVIDER_XAI_OAUTH);
        assert!(status.authenticated);
        assert_eq!(status.default_account_id.as_deref(), Some("ready"));
        assert!(status.accounts[0].is_default);
        assert!(!status.accounts[0].requires_reauth);
        assert!(!status.accounts[1].is_default);
        assert!(status.accounts[1].requires_reauth);
    }
}
