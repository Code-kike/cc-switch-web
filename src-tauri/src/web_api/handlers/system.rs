//! System endpoints: CSRF token issuance, web credentials management, and
//! web-mode system utilities.

use std::collections::HashMap;
use std::convert::Infallible;

use async_stream::stream;
use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use super::super::ApiState;
use super::common::{json_ok, ApiError, ApiResult};

#[derive(Serialize)]
struct CsrfTokenResponse {
    token: String,
}

#[derive(Deserialize)]
struct WebCredentials {
    #[allow(dead_code)]
    new_password: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenExternalRequest {
    url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetToolVersionsRequest {
    tools: Option<Vec<String>>,
    wsl_shell_by_tool: Option<HashMap<String, crate::services::WslShellPreferenceInput>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestApiEndpointsRequest {
    urls: Vec<String>,
    timeout_secs: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CircuitBreakerActionRequest {
    provider_id: String,
    app_type: String,
}

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/system/csrf-token", get(csrf_token))
        .route("/system/web-credentials", put(web_credentials))
        .route("/system/get_migration_result", post(get_migration_result))
        .route("/system/get_init_error", post(get_init_error))
        .route(
            "/system/is_live_takeover_active",
            post(is_live_takeover_active),
        )
        .route("/system/is_portable_mode", post(is_portable_mode))
        .route("/system/restart_app", post(restart_app))
        .route("/system/open_external", post(open_external))
        .route(
            "/system/apply_claude_onboarding_skip",
            post(apply_claude_onboarding_skip),
        )
        .route(
            "/system/clear_claude_onboarding_skip",
            post(clear_claude_onboarding_skip),
        )
        .route("/system/check_for_updates", post(check_for_updates))
        .route("/system/get_update_info", get(get_update_info))
        .route("/system/get_tool_versions", post(get_tool_versions))
        .route("/system/test_api_endpoints", post(test_api_endpoints))
        .route("/system/reset_circuit_breaker", post(reset_circuit_breaker))
        .route(
            "/system/get_circuit_breaker_stats",
            post(get_circuit_breaker_stats),
        )
        .route(
            "/system/enter-lightweight-mode",
            post(enter_lightweight_mode),
        )
        .route("/system/exit-lightweight-mode", post(exit_lightweight_mode))
        .route("/system/is-lightweight-mode", get(is_lightweight_mode))
        .route("/events", get(events))
        .with_state(state)
}

async fn csrf_token() -> Json<CsrfTokenResponse> {
    // Placeholder: real implementation derives token from session cookie
    // (Round 4 P1-5: CSRF bound to session_id, stored in web_sessions table).
    Json(CsrfTokenResponse {
        token: "stub-csrf-token".to_string(),
    })
}

async fn web_credentials(Json(_creds): Json<WebCredentials>) -> axum::http::StatusCode {
    // Placeholder: real implementation rotates `~/.cc-switch/web_password`
    // and invalidates all sessions (`DELETE FROM web_sessions`).
    axum::http::StatusCode::NOT_IMPLEMENTED
}

async fn get_migration_result() -> ApiResult<bool> {
    Ok(json_ok(crate::init_status::take_migration_success()))
}

async fn get_init_error() -> ApiResult<Option<crate::init_status::InitErrorPayload>> {
    Ok(json_ok(crate::init_status::get_init_error()))
}

async fn is_live_takeover_active(State(state): State<ApiState>) -> ApiResult<bool> {
    let active = state
        .app_state
        .proxy_service
        .is_takeover_active()
        .await
        .map_err(ApiError::from_service_message)?;
    Ok(json_ok(active))
}

async fn is_portable_mode() -> ApiResult<bool> {
    let exe_path = std::env::current_exe().map_err(ApiError::from_anyhow)?;
    let portable = exe_path
        .parent()
        .map(|dir| dir.join("portable.ini").is_file())
        .unwrap_or(false);
    Ok(json_ok(portable))
}

async fn restart_app() -> ApiResult<bool> {
    Ok(json_ok(true))
}

async fn open_external(
    State(state): State<ApiState>,
    Json(request): Json<OpenExternalRequest>,
) -> ApiResult<bool> {
    let url = if request.url.starts_with("http://") || request.url.starts_with("https://") {
        request.url
    } else {
        format!("https://{}", request.url)
    };
    state
        .sink
        .open_url(&url)
        .map_err(ApiError::from_service_message)?;
    Ok(json_ok(true))
}

async fn apply_claude_onboarding_skip() -> ApiResult<bool> {
    crate::claude_mcp::set_has_completed_onboarding()
        .map(json_ok)
        .map_err(ApiError::from_anyhow)
}

async fn clear_claude_onboarding_skip() -> ApiResult<bool> {
    crate::claude_mcp::clear_has_completed_onboarding()
        .map(json_ok)
        .map_err(ApiError::from_anyhow)
}

async fn check_for_updates() -> ApiResult<bool> {
    Ok(json_ok(
        crate::services::web_update::get_web_update_info()
            .await
            .available,
    ))
}

async fn get_update_info() -> ApiResult<crate::services::WebUpdateInfo> {
    Ok(json_ok(crate::services::web_update::get_web_update_info().await))
}

async fn get_tool_versions(
    Json(request): Json<GetToolVersionsRequest>,
) -> ApiResult<Vec<crate::services::ToolVersion>> {
    let versions =
        crate::services::tool_version::get_tool_versions(request.tools, request.wsl_shell_by_tool)
            .await;
    Ok(json_ok(versions))
}

async fn test_api_endpoints(
    Json(request): Json<TestApiEndpointsRequest>,
) -> ApiResult<Vec<crate::services::EndpointLatency>> {
    let results =
        crate::services::SpeedtestService::test_endpoints(request.urls, request.timeout_secs)
            .await
            .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(results))
}

async fn reset_circuit_breaker(
    State(state): State<ApiState>,
    Json(request): Json<CircuitBreakerActionRequest>,
) -> ApiResult<()> {
    state
        .app_state
        .db
        .update_provider_health(&request.provider_id, &request.app_type, true, None)
        .await
        .map_err(ApiError::from_anyhow)?;

    state
        .app_state
        .proxy_service
        .reset_provider_circuit_breaker(&request.app_type, &request.provider_id)
        .await
        .map_err(ApiError::from_service_message)?;

    Ok(json_ok(()))
}

async fn get_circuit_breaker_stats(
    Json(_request): Json<CircuitBreakerActionRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Err(ApiError::not_supported(
        "Circuit breaker runtime stats are unavailable in web-server mode",
    ))
}

async fn enter_lightweight_mode() -> Result<Json<serde_json::Value>, ApiError> {
    Err(ApiError::desktop_only(
        "Lightweight mode is only available in the desktop runtime",
    ))
}

async fn exit_lightweight_mode() -> Result<Json<serde_json::Value>, ApiError> {
    Err(ApiError::desktop_only(
        "Lightweight mode is only available in the desktop runtime",
    ))
}

async fn is_lightweight_mode() -> Result<Json<serde_json::Value>, ApiError> {
    Err(ApiError::desktop_only(
        "Lightweight mode state is only available in the desktop runtime",
    ))
}

async fn events(
    State(state): State<ApiState>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.events.subscribe();
    let stream = stream! {
        loop {
            match rx.recv().await {
                Ok(env) => {
                    let payload = serde_json::to_string(&env).unwrap_or_else(|_| "{}".to_string());
                    yield Ok(Event::default().data(payload));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    yield Ok(Event::default().event("lagged").data("null"));
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}
