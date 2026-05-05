use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};

use super::super::ApiState;
use super::common::{json_ok, ApiError, ApiResult};

const AUTH_PROVIDER_GITHUB_COPILOT: &str = "github_copilot";
const AUTH_PROVIDER_CODEX_OAUTH: &str = "codex_oauth";

#[derive(Debug, Clone, serde::Serialize)]
pub struct ManagedAuthAccount {
    pub id: String,
    pub provider: String,
    pub login: String,
    pub avatar_url: Option<String>,
    pub authenticated_at: i64,
    pub is_default: bool,
    pub github_domain: String,
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
        .with_state(state)
}

fn ensure_auth_provider(auth_provider: &str) -> Result<&'static str, ApiError> {
    match auth_provider {
        AUTH_PROVIDER_GITHUB_COPILOT => Ok(AUTH_PROVIDER_GITHUB_COPILOT),
        AUTH_PROVIDER_CODEX_OAUTH => Ok(AUTH_PROVIDER_CODEX_OAUTH),
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
        _ => unreachable!(),
    }
    Ok(json_ok(()))
}
