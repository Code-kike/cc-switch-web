use axum::{extract::State, routing::post, Json, Router};
use serde::Deserialize;

use super::super::ApiState;
use super::common::{json_ok, ApiError, ApiResult};

#[derive(Deserialize)]
struct ParseDeeplinkRequest {
    url: String,
}

#[derive(Deserialize)]
struct DeeplinkRequest {
    request: crate::deeplink::DeepLinkImportRequest,
}

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/deeplink/parse-deeplink", post(parse_deeplink))
        .route("/config/merge-deeplink-config", post(merge_deeplink_config))
        .route("/deeplink/import-from-deeplink", post(import_from_deeplink))
        .route(
            "/deeplink/import-from-deeplink-unified",
            post(import_from_deeplink_unified),
        )
        .with_state(state)
}

async fn parse_deeplink(
    Json(request): Json<ParseDeeplinkRequest>,
) -> ApiResult<crate::deeplink::DeepLinkImportRequest> {
    let parsed =
        crate::deeplink::parse_deeplink_url(&request.url).map_err(ApiError::from_anyhow)?;
    Ok(json_ok(parsed))
}

async fn merge_deeplink_config(
    Json(request): Json<DeeplinkRequest>,
) -> ApiResult<crate::deeplink::DeepLinkImportRequest> {
    let merged =
        crate::deeplink::parse_and_merge_config(&request.request).map_err(ApiError::from_anyhow)?;
    Ok(json_ok(merged))
}

async fn import_from_deeplink(
    State(state): State<ApiState>,
    Json(request): Json<DeeplinkRequest>,
) -> ApiResult<String> {
    let provider_id =
        crate::deeplink::import_provider_from_deeplink(state.app_state.as_ref(), request.request)
            .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(provider_id))
}

async fn import_from_deeplink_unified(
    State(state): State<ApiState>,
    Json(request): Json<DeeplinkRequest>,
) -> ApiResult<serde_json::Value> {
    let request = request.request;
    let value = match request.resource.as_str() {
        "provider" => {
            let id =
                crate::deeplink::import_provider_from_deeplink(state.app_state.as_ref(), request)
                    .map_err(ApiError::from_anyhow)?;
            serde_json::json!({ "type": "provider", "id": id })
        }
        "prompt" => {
            let id =
                crate::deeplink::import_prompt_from_deeplink(state.app_state.as_ref(), request)
                    .map_err(ApiError::from_anyhow)?;
            serde_json::json!({ "type": "prompt", "id": id })
        }
        "mcp" => {
            let result =
                crate::deeplink::import_mcp_from_deeplink(state.app_state.as_ref(), request)
                    .map_err(ApiError::from_anyhow)?;
            serde_json::json!({
                "type": "mcp",
                "importedCount": result.imported_count,
                "importedIds": result.imported_ids,
                "failed": result.failed,
            })
        }
        "skill" => {
            let key =
                crate::deeplink::import_skill_from_deeplink(state.app_state.as_ref(), request)
                    .map_err(ApiError::from_anyhow)?;
            serde_json::json!({ "type": "skill", "key": key })
        }
        other => {
            return Err(ApiError::bad_request(format!(
                "Unsupported resource type: {other}"
            )))
        }
    };
    Ok(json_ok(value))
}
