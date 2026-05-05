use axum::{
    extract::{Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::super::ApiState;
use super::common::{json_ok, ApiError, ApiResult};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToggleMcpAppRequest {
    server_id: String,
    app: String,
    enabled: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpsertMcpServerRequest {
    server: crate::app_config::McpServer,
}

#[derive(Deserialize)]
struct AppQuery {
    app: String,
}

#[derive(Deserialize)]
struct IdQuery {
    id: String,
}

#[derive(Deserialize)]
struct CmdQuery {
    cmd: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpsertClaudeMcpServerRequest {
    id: String,
    spec: serde_json::Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpsertMcpServerInConfigRequest {
    app: String,
    id: String,
    spec: serde_json::Value,
    sync_other_side: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetMcpEnabledRequest {
    app: String,
    id: String,
    enabled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct McpConfigResponse {
    config_path: String,
    servers: HashMap<String, serde_json::Value>,
}

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/mcp/get-claude-mcp-status", get(get_claude_mcp_status))
        .route(
            "/mcp/upsert-claude-mcp-server",
            post(upsert_claude_mcp_server),
        )
        .route(
            "/mcp/delete-claude-mcp-server",
            delete(delete_claude_mcp_server),
        )
        .route("/mcp/validate-mcp-command", post(validate_mcp_command))
        .route("/mcp/set-mcp-enabled", put(set_mcp_enabled))
        .route("/mcp/get-mcp-servers", get(get_mcp_servers))
        .route("/mcp/upsert-mcp-server", post(upsert_mcp_server))
        .route("/mcp/delete-mcp-server", delete(delete_mcp_server))
        .route("/mcp/toggle-mcp-app", post(toggle_mcp_app))
        .route("/mcp/import-mcp-from-apps", post(import_mcp_from_apps))
        .route(
            "/config/read-claude-mcp-config",
            get(read_claude_mcp_config),
        )
        .route("/config/get-mcp-config", get(get_mcp_config))
        .route(
            "/config/upsert-mcp-server-in-config",
            post(upsert_mcp_server_in_config),
        )
        .route(
            "/config/delete-mcp-server-in-config",
            delete(delete_mcp_server_in_config),
        )
        .with_state(state)
}

fn parse_app_type(app: &str) -> Result<crate::app_config::AppType, ApiError> {
    use std::str::FromStr;
    crate::app_config::AppType::from_str(app).map_err(|err| ApiError::bad_request(err.to_string()))
}

async fn get_claude_mcp_status() -> ApiResult<crate::claude_mcp::McpStatus> {
    let status = crate::claude_mcp::get_mcp_status().map_err(ApiError::from_anyhow)?;
    Ok(json_ok(status))
}

async fn read_claude_mcp_config() -> ApiResult<Option<String>> {
    let config = crate::claude_mcp::read_mcp_json().map_err(ApiError::from_anyhow)?;
    Ok(json_ok(config))
}

async fn upsert_claude_mcp_server(
    Json(request): Json<UpsertClaudeMcpServerRequest>,
) -> ApiResult<bool> {
    let updated = crate::claude_mcp::upsert_mcp_server(&request.id, request.spec)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(updated))
}

async fn delete_claude_mcp_server(Query(query): Query<IdQuery>) -> ApiResult<bool> {
    let deleted = crate::claude_mcp::delete_mcp_server(&query.id).map_err(ApiError::from_anyhow)?;
    Ok(json_ok(deleted))
}

async fn validate_mcp_command(Json(request): Json<CmdQuery>) -> ApiResult<bool> {
    let valid =
        crate::claude_mcp::validate_command_in_path(&request.cmd).map_err(ApiError::from_anyhow)?;
    Ok(json_ok(valid))
}

async fn get_mcp_servers(
    State(state): State<ApiState>,
) -> Result<Json<indexmap::IndexMap<String, crate::app_config::McpServer>>, ApiError> {
    let servers = crate::services::McpService::get_all_servers(state.app_state.as_ref())
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(servers))
}

async fn upsert_mcp_server(
    State(state): State<ApiState>,
    Json(request): Json<UpsertMcpServerRequest>,
) -> ApiResult<()> {
    crate::services::McpService::upsert_server(state.app_state.as_ref(), request.server)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(()))
}

async fn delete_mcp_server(
    State(state): State<ApiState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<bool> {
    let id = params
        .get("id")
        .cloned()
        .ok_or_else(|| ApiError::bad_request("missing id"))?;
    let deleted = crate::services::McpService::delete_server(state.app_state.as_ref(), &id)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(deleted))
}

async fn toggle_mcp_app(
    State(state): State<ApiState>,
    Json(request): Json<ToggleMcpAppRequest>,
) -> ApiResult<()> {
    use std::str::FromStr;
    let app = crate::app_config::AppType::from_str(&request.app)
        .map_err(|err| ApiError::bad_request(err.to_string()))?;
    crate::services::McpService::toggle_app(
        state.app_state.as_ref(),
        &request.server_id,
        app,
        request.enabled,
    )
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(()))
}

async fn import_mcp_from_apps(State(state): State<ApiState>) -> ApiResult<usize> {
    let total = crate::services::McpService::import_from_all_apps(state.app_state.as_ref())
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(total))
}

async fn get_mcp_config(
    State(state): State<ApiState>,
    Query(query): Query<AppQuery>,
) -> ApiResult<McpConfigResponse> {
    let app_type = parse_app_type(&query.app)?;
    let servers = crate::services::McpService::get_servers(state.app_state.as_ref(), app_type)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(McpConfigResponse {
        config_path: crate::config::get_app_config_path()
            .to_string_lossy()
            .to_string(),
        servers,
    }))
}

async fn upsert_mcp_server_in_config(
    State(state): State<ApiState>,
    Json(request): Json<UpsertMcpServerInConfigRequest>,
) -> ApiResult<bool> {
    let app_type = parse_app_type(&request.app)?;
    let existing = state
        .app_state
        .db
        .get_all_mcp_servers()
        .map_err(ApiError::from_anyhow)?
        .get(&request.id)
        .cloned();

    let mut server = if let Some(mut existing) = existing {
        existing.server = request.spec.clone();
        existing.apps.set_enabled_for(&app_type, true);
        existing
    } else {
        let mut apps = crate::app_config::McpApps::default();
        apps.set_enabled_for(&app_type, true);
        let name = request
            .spec
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&request.id)
            .to_string();
        crate::app_config::McpServer {
            id: request.id.clone(),
            name,
            server: request.spec,
            apps,
            description: None,
            homepage: None,
            docs: None,
            tags: Vec::new(),
        }
    };

    if request.sync_other_side.unwrap_or(false) {
        server.apps.claude = true;
        server.apps.codex = true;
        server.apps.gemini = true;
        server.apps.opencode = true;
    }

    crate::services::McpService::upsert_server(state.app_state.as_ref(), server)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(true))
}

async fn delete_mcp_server_in_config(
    State(state): State<ApiState>,
    Query(query): Query<IdQuery>,
) -> ApiResult<bool> {
    let deleted = crate::services::McpService::delete_server(state.app_state.as_ref(), &query.id)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(deleted))
}

async fn set_mcp_enabled(
    State(state): State<ApiState>,
    Json(request): Json<SetMcpEnabledRequest>,
) -> ApiResult<bool> {
    let app_type = parse_app_type(&request.app)?;
    let updated = crate::services::McpService::set_enabled(
        state.app_state.as_ref(),
        app_type,
        &request.id,
        request.enabled,
    )
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(updated))
}
