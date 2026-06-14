use axum::{
    extract::Query,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

use super::super::ApiState;
use super::common::{json_ok, validate_outbound_url, ApiResult};

#[derive(Deserialize)]
struct ToolQuery {
    tool: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BalanceQuery {
    base_url: String,
    api_key: String,
}

pub fn router(_state: ApiState) -> Router {
    Router::new()
        .route(
            "/subscription/get-subscription-quota",
            get(get_subscription_quota),
        )
        .route("/usage/get-balance", post(get_balance))
        .route("/usage/get-coding-plan-quota", post(get_coding_plan_quota))
}

async fn get_subscription_quota(
    Query(query): Query<ToolQuery>,
) -> ApiResult<crate::services::subscription::SubscriptionQuota> {
    let quota = crate::services::subscription::get_subscription_quota(&query.tool)
        .await
        .map_err(super::common::ApiError::from_service_message)?;
    Ok(json_ok(quota))
}

async fn get_balance(Json(query): Json<BalanceQuery>) -> ApiResult<crate::provider::UsageResult> {
    // Audit F4: reject internal/private targets before dialing the user-supplied base_url.
    validate_outbound_url(&query.base_url).await?;
    let result = crate::services::balance::get_balance(&query.base_url, &query.api_key)
        .await
        .map_err(super::common::ApiError::from_service_message)?;
    Ok(Json(result))
}

async fn get_coding_plan_quota(
    Json(query): Json<BalanceQuery>,
) -> ApiResult<crate::services::subscription::SubscriptionQuota> {
    // Audit F4: ZenMux and other coding-plan providers dial base_url; guard it.
    validate_outbound_url(&query.base_url).await?;
    let quota =
        crate::services::coding_plan::get_coding_plan_quota(&query.base_url, &query.api_key)
            .await
            .map_err(super::common::ApiError::from_service_message)?;
    Ok(json_ok(quota))
}
