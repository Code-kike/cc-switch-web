use axum::{
    extract::{Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;

use super::super::ApiState;
use super::common::{json_ok, ApiError, ApiResult};

#[derive(Deserialize)]
struct AppQuery {
    app: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportDefaultRequest {
    app: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderIdQuery {
    app: String,
    id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddProviderRequest {
    app: String,
    provider: crate::provider::Provider,
    add_to_live: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateProviderRequest {
    app: String,
    provider: crate::provider::Provider,
    original_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SwitchProviderRequest {
    app: String,
    id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateSortOrderRequest {
    app: String,
    updates: Vec<crate::services::ProviderSortUpdate>,
}

#[derive(Deserialize)]
struct IdQuery {
    id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UniversalProviderRequest {
    provider: crate::provider::UniversalProvider,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CustomEndpointRequest {
    app: String,
    provider_id: String,
    url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CustomEndpointQuery {
    app: String,
    provider_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderHealthQuery {
    provider_id: String,
    app_type: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageQuery {
    start_date: Option<i64>,
    end_date: Option<i64>,
    app_type: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct QueryProviderUsageRequest {
    provider_id: String,
    app: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StreamCheckProviderRequest {
    app_type: String,
    provider_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StreamCheckAllProvidersRequest {
    app_type: String,
    proxy_targets_only: Option<bool>,
}

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/providers/get-providers", get(get_providers))
        .route("/providers/get-current-provider", get(get_current_provider))
        .route("/providers/add-provider", post(add_provider))
        .route("/providers/update-provider", put(update_provider))
        .route("/providers/delete-provider", delete(delete_provider))
        .route(
            "/config/remove-provider-from-live-config",
            delete(remove_provider_from_live_config),
        )
        .route("/config/import-default-config", post(import_default_config))
        .route("/providers/switch-provider", post(switch_provider))
        .route(
            "/providers/update-providers-sort-order",
            put(update_providers_sort_order),
        )
        .route(
            "/providers/import-opencode-providers-from-live",
            post(import_opencode_providers_from_live),
        )
        .route(
            "/providers/get-opencode-live-provider-ids",
            get(get_opencode_live_provider_ids),
        )
        .route(
            "/providers/read-live-provider-settings",
            get(read_live_provider_settings),
        )
        .route(
            "/providers/sync-current-providers-live",
            post(sync_current_providers_live),
        )
        .route(
            "/providers/get-universal-providers",
            get(get_universal_providers),
        )
        .route(
            "/providers/get-universal-provider",
            get(get_universal_provider),
        )
        .route(
            "/providers/upsert-universal-provider",
            post(upsert_universal_provider),
        )
        .route(
            "/providers/delete-universal-provider",
            delete(delete_universal_provider),
        )
        .route(
            "/providers/sync-universal-provider",
            post(sync_universal_provider),
        )
        .route("/system/get_custom_endpoints", post(get_custom_endpoints))
        .route("/system/add_custom_endpoint", post(add_custom_endpoint))
        .route(
            "/system/remove_custom_endpoint",
            post(remove_custom_endpoint),
        )
        .route(
            "/system/update_endpoint_last_used",
            post(update_endpoint_last_used),
        )
        .route("/system/update_tray_menu", post(update_tray_menu))
        .route("/providers/get-provider-health", get(get_provider_health))
        .route("/providers/get-provider-stats", get(get_provider_stats))
        .route("/providers/queryproviderusage", post(query_provider_usage))
        .route(
            "/providers/check-provider-limits",
            get(check_provider_limits),
        )
        .route(
            "/providers/stream-check-provider",
            post(stream_check_provider),
        )
        .route(
            "/providers/stream-check-all-providers",
            post(stream_check_all_providers),
        )
        .with_state(state)
}

fn parse_app_type(app: &str) -> Result<crate::app_config::AppType, ApiError> {
    use std::str::FromStr;
    crate::app_config::AppType::from_str(app).map_err(|err| ApiError::bad_request(err.to_string()))
}

async fn get_providers(
    State(state): State<ApiState>,
    Query(query): Query<AppQuery>,
) -> Result<Json<indexmap::IndexMap<String, crate::provider::Provider>>, ApiError> {
    let app_type = parse_app_type(&query.app)?;
    let providers = crate::services::ProviderService::list(state.app_state.as_ref(), app_type)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(providers))
}

async fn get_current_provider(
    State(state): State<ApiState>,
    Query(query): Query<AppQuery>,
) -> ApiResult<String> {
    let app_type = parse_app_type(&query.app)?;
    let current = crate::services::ProviderService::current(state.app_state.as_ref(), app_type)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(current))
}

async fn add_provider(
    State(state): State<ApiState>,
    Json(request): Json<AddProviderRequest>,
) -> ApiResult<bool> {
    let app_type = parse_app_type(&request.app)?;
    let result = crate::services::ProviderService::add(
        state.app_state.as_ref(),
        app_type,
        request.provider,
        request.add_to_live.unwrap_or(true),
    )
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(result))
}

async fn update_provider(
    State(state): State<ApiState>,
    Json(request): Json<UpdateProviderRequest>,
) -> ApiResult<bool> {
    let app_type = parse_app_type(&request.app)?;
    let result = crate::services::ProviderService::update(
        state.app_state.as_ref(),
        app_type,
        request.original_id.as_deref(),
        request.provider,
    )
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(result))
}

async fn delete_provider(
    State(state): State<ApiState>,
    Query(query): Query<ProviderIdQuery>,
) -> ApiResult<bool> {
    let app_type = parse_app_type(&query.app)?;
    crate::services::ProviderService::delete(state.app_state.as_ref(), app_type, &query.id)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(true))
}

async fn remove_provider_from_live_config(
    State(state): State<ApiState>,
    Query(query): Query<ProviderIdQuery>,
) -> ApiResult<bool> {
    let app_type = parse_app_type(&query.app)?;
    crate::services::ProviderService::remove_from_live_config(
        state.app_state.as_ref(),
        app_type,
        &query.id,
    )
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(true))
}

async fn import_default_config(
    State(state): State<ApiState>,
    Json(request): Json<ImportDefaultRequest>,
) -> ApiResult<bool> {
    let app_type = parse_app_type(&request.app)?;
    let imported =
        crate::services::ProviderService::import_default_config(state.app_state.as_ref(), app_type)
            .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(imported))
}

async fn switch_provider(
    State(state): State<ApiState>,
    Json(request): Json<SwitchProviderRequest>,
) -> Result<Json<crate::services::SwitchResult>, ApiError> {
    let app_type = parse_app_type(&request.app)?;
    let result = crate::services::ProviderService::switch(
        state.app_state.as_ref(),
        app_type.clone(),
        &request.id,
    )
    .map_err(ApiError::from_anyhow)?;
    state.sink.emit_json(
        "provider-switched",
        json!({
            "appType": request.app,
            "providerId": request.id,
        }),
    );
    Ok(json_ok(result))
}

async fn update_providers_sort_order(
    State(state): State<ApiState>,
    Json(request): Json<UpdateSortOrderRequest>,
) -> ApiResult<bool> {
    let app_type = parse_app_type(&request.app)?;
    let result = crate::services::ProviderService::update_sort_order(
        state.app_state.as_ref(),
        app_type,
        request.updates,
    )
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(result))
}

async fn import_opencode_providers_from_live(State(state): State<ApiState>) -> ApiResult<usize> {
    let count =
        crate::services::provider::import_opencode_providers_from_live(state.app_state.as_ref())
            .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(count))
}

async fn get_opencode_live_provider_ids() -> ApiResult<Vec<String>> {
    let ids = crate::opencode_config::get_providers()
        .map(|providers| providers.keys().cloned().collect())
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(ids))
}

async fn read_live_provider_settings(
    Query(query): Query<AppQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let app_type = parse_app_type(&query.app)?;
    let value = crate::services::ProviderService::read_live_settings(app_type)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(value))
}

async fn sync_current_providers_live(
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    crate::services::ProviderService::sync_current_to_live(state.app_state.as_ref())
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(json!({
        "success": true,
        "message": "Live configuration synchronized"
    })))
}

async fn get_universal_providers(
    State(state): State<ApiState>,
) -> Result<Json<std::collections::HashMap<String, crate::provider::UniversalProvider>>, ApiError> {
    let providers = crate::services::ProviderService::list_universal(state.app_state.as_ref())
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(providers))
}

async fn get_universal_provider(
    State(state): State<ApiState>,
    Query(query): Query<IdQuery>,
) -> ApiResult<Option<crate::provider::UniversalProvider>> {
    let provider =
        crate::services::ProviderService::get_universal(state.app_state.as_ref(), &query.id)
            .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(provider))
}

async fn upsert_universal_provider(
    State(state): State<ApiState>,
    Json(request): Json<UniversalProviderRequest>,
) -> ApiResult<bool> {
    let result = crate::services::ProviderService::upsert_universal(
        state.app_state.as_ref(),
        request.provider,
    )
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(result))
}

async fn delete_universal_provider(
    State(state): State<ApiState>,
    Query(query): Query<IdQuery>,
) -> ApiResult<bool> {
    let result =
        crate::services::ProviderService::delete_universal(state.app_state.as_ref(), &query.id)
            .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(result))
}

async fn sync_universal_provider(
    State(state): State<ApiState>,
    Json(query): Json<IdQuery>,
) -> ApiResult<bool> {
    let result = crate::services::ProviderService::sync_universal_to_apps(
        state.app_state.as_ref(),
        &query.id,
    )
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(result))
}

async fn get_custom_endpoints(
    State(state): State<ApiState>,
    Json(request): Json<CustomEndpointQuery>,
) -> Result<Json<Vec<crate::settings::CustomEndpoint>>, ApiError> {
    let app_type = parse_app_type(&request.app)?;
    let endpoints = crate::services::ProviderService::get_custom_endpoints(
        state.app_state.as_ref(),
        app_type,
        &request.provider_id,
    )
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(endpoints))
}

async fn add_custom_endpoint(
    State(state): State<ApiState>,
    Json(request): Json<CustomEndpointRequest>,
) -> ApiResult<()> {
    let app_type = parse_app_type(&request.app)?;
    crate::services::ProviderService::add_custom_endpoint(
        state.app_state.as_ref(),
        app_type,
        &request.provider_id,
        request.url,
    )
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(()))
}

async fn remove_custom_endpoint(
    State(state): State<ApiState>,
    Json(request): Json<CustomEndpointRequest>,
) -> ApiResult<()> {
    let app_type = parse_app_type(&request.app)?;
    crate::services::ProviderService::remove_custom_endpoint(
        state.app_state.as_ref(),
        app_type,
        &request.provider_id,
        request.url,
    )
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(()))
}

async fn update_endpoint_last_used(
    State(state): State<ApiState>,
    Json(request): Json<CustomEndpointRequest>,
) -> ApiResult<()> {
    let app_type = parse_app_type(&request.app)?;
    crate::services::ProviderService::update_endpoint_last_used(
        state.app_state.as_ref(),
        app_type,
        &request.provider_id,
        request.url,
    )
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(()))
}

async fn update_tray_menu() -> ApiResult<bool> {
    Ok(json_ok(true))
}

async fn get_provider_health(
    State(state): State<ApiState>,
    Query(query): Query<ProviderHealthQuery>,
) -> ApiResult<crate::proxy::types::ProviderHealth> {
    let health = state
        .app_state
        .db
        .get_provider_health(&query.provider_id, &query.app_type)
        .await
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(health))
}

async fn get_provider_stats(
    State(state): State<ApiState>,
    Query(query): Query<UsageQuery>,
) -> ApiResult<Vec<crate::services::usage_stats::ProviderStats>> {
    let stats = state
        .app_state
        .db
        .get_provider_stats(query.start_date, query.end_date, query.app_type.as_deref())
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(stats))
}

async fn check_provider_limits(
    State(state): State<ApiState>,
    Query(query): Query<ProviderHealthQuery>,
) -> ApiResult<crate::services::usage_stats::ProviderLimitStatus> {
    let status = state
        .app_state
        .db
        .check_provider_limits(&query.provider_id, &query.app_type)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(status))
}

async fn query_provider_usage(
    State(state): State<ApiState>,
    Json(request): Json<QueryProviderUsageRequest>,
) -> ApiResult<crate::provider::UsageResult> {
    let app_type = parse_app_type(&request.app)?;
    let result = crate::services::ProviderService::query_usage(
        state.app_state.as_ref(),
        app_type,
        &request.provider_id,
    )
    .await
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(result))
}

async fn stream_check_provider(
    State(state): State<ApiState>,
    Json(request): Json<StreamCheckProviderRequest>,
) -> ApiResult<crate::services::stream_check::StreamCheckResult> {
    use crate::services::stream_check::StreamCheckService;
    use std::str::FromStr;

    let app_type = crate::app_config::AppType::from_str(&request.app_type)
        .map_err(|err| ApiError::bad_request(err.to_string()))?;
    let config = state
        .app_state
        .db
        .get_stream_check_config()
        .map_err(ApiError::from_anyhow)?;
    let providers = state
        .app_state
        .db
        .get_all_providers(app_type.as_str())
        .map_err(ApiError::from_anyhow)?;
    let provider = providers
        .get(&request.provider_id)
        .ok_or_else(|| ApiError::bad_request(format!("供应商 {} 不存在", request.provider_id)))?;

    let result =
        StreamCheckService::check_with_retry(&app_type, provider, &config, None, None, None)
            .await
            .map_err(ApiError::from_anyhow)?;

    let _ = state.app_state.db.save_stream_check_log(
        &request.provider_id,
        &provider.name,
        app_type.as_str(),
        &result,
    );

    Ok(json_ok(result))
}

async fn stream_check_all_providers(
    State(state): State<ApiState>,
    Json(request): Json<StreamCheckAllProvidersRequest>,
) -> ApiResult<Vec<(String, crate::services::stream_check::StreamCheckResult)>> {
    use crate::services::stream_check::{HealthStatus, StreamCheckResult, StreamCheckService};
    use std::collections::HashSet;
    use std::str::FromStr;

    let app_type = crate::app_config::AppType::from_str(&request.app_type)
        .map_err(|err| ApiError::bad_request(err.to_string()))?;
    let config = state
        .app_state
        .db
        .get_stream_check_config()
        .map_err(ApiError::from_anyhow)?;
    let providers = state
        .app_state
        .db
        .get_all_providers(app_type.as_str())
        .map_err(ApiError::from_anyhow)?;

    let allowed_ids: Option<HashSet<String>> = if request.proxy_targets_only.unwrap_or(false) {
        let mut ids = HashSet::new();
        if let Ok(Some(current_id)) = state.app_state.db.get_current_provider(app_type.as_str()) {
            ids.insert(current_id);
        }
        if let Ok(queue) = state.app_state.db.get_failover_queue(app_type.as_str()) {
            for item in queue {
                ids.insert(item.provider_id);
            }
        }
        Some(ids)
    } else {
        None
    };

    let mut results = Vec::new();
    for (id, provider) in providers {
        if let Some(ids) = &allowed_ids {
            if !ids.contains(&id) {
                continue;
            }
        }

        let result =
            StreamCheckService::check_with_retry(&app_type, &provider, &config, None, None, None)
                .await
                .unwrap_or_else(|e| {
                    let (http_status, message) = match &e {
                        crate::error::AppError::HttpStatus { status, .. } => (
                            Some(*status),
                            StreamCheckService::classify_http_status(*status).to_string(),
                        ),
                        _ => (None, e.to_string()),
                    };
                    StreamCheckResult {
                        status: HealthStatus::Failed,
                        success: false,
                        message,
                        response_time_ms: None,
                        http_status,
                        model_used: String::new(),
                        tested_at: chrono::Utc::now().timestamp(),
                        retry_count: 0,
                        error_category: None,
                    }
                });

        let _ = state.app_state.db.save_stream_check_log(
            &id,
            &provider.name,
            app_type.as_str(),
            &result,
        );
        results.push((id, result));
    }

    Ok(json_ok(results))
}
