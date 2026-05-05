use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

use super::super::ApiState;
use super::common::{json_ok, ApiError, ApiResult};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageQuery {
    start_date: Option<i64>,
    end_date: Option<i64>,
    app_type: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestLogsRequest {
    filters: crate::services::usage_stats::LogFilters,
    page: u32,
    page_size: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestDetailRequest {
    request_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelPricingRequest {
    model_id: String,
    display_name: String,
    input_cost: String,
    output_cost: String,
    cache_read_cost: String,
    cache_creation_cost: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelIdRequest {
    model_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestUsageScriptRequest {
    provider_id: String,
    app: String,
    script_code: String,
    timeout: Option<u64>,
    api_key: Option<String>,
    base_url: Option<String>,
    access_token: Option<String>,
    user_id: Option<String>,
    template_type: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ModelPricingInfo {
    model_id: String,
    display_name: String,
    input_cost_per_million: String,
    output_cost_per_million: String,
    cache_read_cost_per_million: String,
    cache_creation_cost_per_million: String,
}

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/usage/get-usage-summary", get(get_usage_summary))
        .route("/usage/get-usage-trends", get(get_usage_trends))
        .route("/usage/get-usage-data-sources", get(get_usage_data_sources))
        .route("/system/get_model_stats", post(get_model_stats))
        .route("/system/get_request_logs", post(get_request_logs))
        .route("/system/get_request_detail", post(get_request_detail))
        .route("/system/get_model_pricing", post(get_model_pricing))
        .route("/system/update_model_pricing", post(update_model_pricing))
        .route("/system/delete_model_pricing", post(delete_model_pricing))
        .route("/usage/testusagescript", post(test_usage_script))
        .route("/sessions/sync-session-usage", post(sync_session_usage))
        .with_state(state)
}

async fn get_usage_summary(
    State(state): State<ApiState>,
    Query(query): Query<UsageQuery>,
) -> ApiResult<crate::services::usage_stats::UsageSummary> {
    let summary = state
        .app_state
        .db
        .get_usage_summary(query.start_date, query.end_date, query.app_type.as_deref())
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(summary))
}

async fn get_usage_trends(
    State(state): State<ApiState>,
    Query(query): Query<UsageQuery>,
) -> ApiResult<Vec<crate::services::usage_stats::DailyStats>> {
    let trends = state
        .app_state
        .db
        .get_daily_trends(query.start_date, query.end_date, query.app_type.as_deref())
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(trends))
}

async fn get_model_stats(
    State(state): State<ApiState>,
    Json(query): Json<UsageQuery>,
) -> ApiResult<Vec<crate::services::usage_stats::ModelStats>> {
    let stats = state
        .app_state
        .db
        .get_model_stats(query.start_date, query.end_date, query.app_type.as_deref())
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(stats))
}

async fn get_request_logs(
    State(state): State<ApiState>,
    Json(request): Json<RequestLogsRequest>,
) -> ApiResult<crate::services::usage_stats::PaginatedLogs> {
    let logs = state
        .app_state
        .db
        .get_request_logs(&request.filters, request.page, request.page_size)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(logs))
}

async fn get_request_detail(
    State(state): State<ApiState>,
    Json(request): Json<RequestDetailRequest>,
) -> ApiResult<Option<crate::services::usage_stats::RequestLogDetail>> {
    let detail = state
        .app_state
        .db
        .get_request_detail(&request.request_id)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(detail))
}

async fn sync_session_usage(
    State(state): State<ApiState>,
) -> ApiResult<crate::services::session_usage::SessionSyncResult> {
    let db = state.app_state.db.clone();
    let result = tokio::task::spawn_blocking(move || {
        let mut result = crate::services::session_usage::sync_claude_session_logs(&db)?;
        match crate::services::session_usage_codex::sync_codex_usage(&db) {
            Ok(codex) => {
                result.imported += codex.imported;
                result.skipped += codex.skipped;
                result.files_scanned += codex.files_scanned;
                result.errors.extend(codex.errors);
            }
            Err(err) => result.errors.push(format!("Codex sync failed: {err}")),
        }
        match crate::services::session_usage_gemini::sync_gemini_usage(&db) {
            Ok(gemini) => {
                result.imported += gemini.imported;
                result.skipped += gemini.skipped;
                result.files_scanned += gemini.files_scanned;
                result.errors.extend(gemini.errors);
            }
            Err(err) => result.errors.push(format!("Gemini sync failed: {err}")),
        }
        Ok::<_, crate::error::AppError>(result)
    })
    .await
    .map_err(ApiError::from_anyhow)?
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(result))
}

async fn get_usage_data_sources(
    State(state): State<ApiState>,
) -> ApiResult<Vec<crate::services::session_usage::DataSourceSummary>> {
    let sources = crate::services::session_usage::get_data_source_breakdown(&state.app_state.db)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(sources))
}

async fn get_model_pricing(State(state): State<ApiState>) -> ApiResult<Vec<ModelPricingInfo>> {
    state
        .app_state
        .db
        .ensure_model_pricing_seeded()
        .map_err(ApiError::from_anyhow)?;

    let db = state.app_state.db.clone();
    let pricing = tokio::task::spawn_blocking(move || {
        let conn = crate::database::lock_conn!(db.conn);
        let table_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='model_pricing'",
                [],
                |row| row.get::<_, i64>(0).map(|count| count > 0),
            )
            .unwrap_or(false);
        if !table_exists {
            return Ok(Vec::new());
        }

        let mut stmt = conn.prepare(
            "SELECT model_id, display_name, input_cost_per_million, output_cost_per_million,
                    cache_read_cost_per_million, cache_creation_cost_per_million
             FROM model_pricing
             ORDER BY display_name",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ModelPricingInfo {
                    model_id: row.get(0)?,
                    display_name: row.get(1)?,
                    input_cost_per_million: row.get(2)?,
                    output_cost_per_million: row.get(3)?,
                    cache_read_cost_per_million: row.get(4)?,
                    cache_creation_cost_per_million: row.get(5)?,
                })
            })
            .map_err(|err| AppError::Database(err.to_string()))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|err| AppError::Database(err.to_string()))
    })
    .await
    .map_err(ApiError::from_anyhow)?
    .map_err(ApiError::from_anyhow)?;

    Ok(json_ok(pricing))
}

async fn update_model_pricing(
    State(state): State<ApiState>,
    Json(request): Json<ModelPricingRequest>,
) -> ApiResult<()> {
    let db = state.app_state.db.clone();
    tokio::task::spawn_blocking(move || {
        let conn = crate::database::lock_conn!(db.conn);
        conn.execute(
            "INSERT OR REPLACE INTO model_pricing (
                model_id, display_name, input_cost_per_million, output_cost_per_million,
                cache_read_cost_per_million, cache_creation_cost_per_million
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                request.model_id,
                request.display_name,
                request.input_cost,
                request.output_cost,
                request.cache_read_cost,
                request.cache_creation_cost,
            ],
        )
        .map(|_| ())
        .map_err(|err| AppError::Database(err.to_string()))
    })
    .await
    .map_err(ApiError::from_anyhow)?
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(()))
}

async fn delete_model_pricing(
    State(state): State<ApiState>,
    Json(request): Json<ModelIdRequest>,
) -> ApiResult<()> {
    let db = state.app_state.db.clone();
    tokio::task::spawn_blocking(move || {
        let conn = crate::database::lock_conn!(db.conn);
        conn.execute(
            "DELETE FROM model_pricing WHERE model_id = ?1",
            rusqlite::params![request.model_id],
        )
        .map(|_| ())
        .map_err(|err| AppError::Database(err.to_string()))
    })
    .await
    .map_err(ApiError::from_anyhow)?
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(()))
}

async fn test_usage_script(
    State(state): State<ApiState>,
    Json(request): Json<TestUsageScriptRequest>,
) -> ApiResult<crate::provider::UsageResult> {
    use std::str::FromStr;

    let app_type = crate::app_config::AppType::from_str(&request.app)
        .map_err(|err| ApiError::bad_request(err.to_string()))?;
    let result = crate::services::ProviderService::test_usage_script(
        state.app_state.as_ref(),
        app_type,
        &request.provider_id,
        &request.script_code,
        request.timeout.unwrap_or(10),
        request.api_key.as_deref(),
        request.base_url.as_deref(),
        request.access_token.as_deref(),
        request.user_id.as_deref(),
        request.template_type.as_deref(),
    )
    .await
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(result))
}
