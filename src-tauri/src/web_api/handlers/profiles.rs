//! Project Profile Web API parity.
//!
//! Upstream exposes these operations only as Tauri commands. The web-first fork
//! mirrors the same Claude/Codex-scoped contracts over Axum and emits the same
//! cache-refresh events through the Web SSE sink.

use axum::{
    extract::{Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use super::super::ApiState;
use super::common::{json_ok, ApiError, ApiResult};
use crate::database::Profile;
use crate::error::AppError;
use crate::services::profile::{ProfilePayload, ProfileScope, ProfileService};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileDto {
    id: String,
    name: String,
    payload: ProfilePayload,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at: Option<i64>,
}

impl From<Profile> for ProfileDto {
    fn from(profile: Profile) -> Self {
        // Match the desktop command: one damaged payload must not make the
        // complete project list unavailable.
        let payload = serde_json::from_str(&profile.payload).unwrap_or_else(|error| {
            log::warn!(
                "Failed to parse profile '{}' payload in Web API; using defaults: {error}",
                profile.id
            );
            ProfilePayload::default()
        });
        Self {
            id: profile.id,
            name: profile.name,
            payload,
            created_at: profile.created_at,
            updated_at: profile.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CurrentProfileIds {
    claude: Option<String>,
    codex: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfilesResponse {
    profiles: Vec<ProfileDto>,
    current_ids: CurrentProfileIds,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateProfileRequest {
    name: String,
    scope: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateProfileRequest {
    id: String,
    name: Option<String>,
    resnapshot: Option<bool>,
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProfileIdQuery {
    id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScopeRequest {
    scope: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApplyProfileRequest {
    id: String,
    scope: String,
}

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/profiles/list-profiles", get(list_profiles))
        .route("/profiles/create-profile", post(create_profile))
        .route("/profiles/update-profile", put(update_profile))
        .route("/profiles/delete-profile", delete(delete_profile))
        .route(
            "/profiles/clear-current-profile",
            delete(clear_current_profile),
        )
        .route("/profiles/apply-profile", post(apply_profile))
        .with_state(state)
}

fn parse_scope(raw: &str) -> Result<ProfileScope, ApiError> {
    ProfileScope::parse(raw).map_err(map_profile_error)
}

fn map_profile_error(error: AppError) -> ApiError {
    match error {
        AppError::InvalidInput(_) | AppError::Config(_) => ApiError::bad_request(error.to_string()),
        _ => ApiError::from_anyhow(error),
    }
}

async fn list_profiles(State(state): State<ApiState>) -> ApiResult<ProfilesResponse> {
    let profiles = ProfileService::list(state.app_state.as_ref()).map_err(map_profile_error)?;
    let current_ids = CurrentProfileIds {
        claude: state
            .app_state
            .db
            .get_current_profile_id(ProfileScope::Claude.as_str())
            .map_err(map_profile_error)?,
        codex: state
            .app_state
            .db
            .get_current_profile_id(ProfileScope::Codex.as_str())
            .map_err(map_profile_error)?,
    };
    Ok(json_ok(ProfilesResponse {
        profiles: profiles.into_iter().map(ProfileDto::from).collect(),
        current_ids,
    }))
}

async fn create_profile(
    State(state): State<ApiState>,
    Json(request): Json<CreateProfileRequest>,
) -> ApiResult<ProfileDto> {
    let scope = parse_scope(&request.scope)?;
    let profile = ProfileService::create(state.app_state.as_ref(), &request.name, scope)
        .map_err(map_profile_error)?;
    Ok(json_ok(ProfileDto::from(profile)))
}

async fn update_profile(
    State(state): State<ApiState>,
    Json(request): Json<UpdateProfileRequest>,
) -> ApiResult<ProfileDto> {
    let scope = request.scope.as_deref().map(parse_scope).transpose()?;
    let profile = ProfileService::update(
        state.app_state.as_ref(),
        &request.id,
        request.name,
        request.resnapshot.unwrap_or(false),
        scope,
    )
    .map_err(map_profile_error)?;
    Ok(json_ok(ProfileDto::from(profile)))
}

async fn delete_profile(
    State(state): State<ApiState>,
    Query(query): Query<ProfileIdQuery>,
) -> ApiResult<bool> {
    ProfileService::delete(state.app_state.as_ref(), &query.id).map_err(map_profile_error)?;
    Ok(json_ok(true))
}

async fn clear_current_profile(
    State(state): State<ApiState>,
    Query(request): Query<ScopeRequest>,
) -> ApiResult<bool> {
    let scope = parse_scope(&request.scope)?;
    state
        .app_state
        .db
        .set_current_profile_id(scope.as_str(), None)
        .map_err(map_profile_error)?;
    state.sink.emit_json(
        "profile-applied",
        serde_json::json!({ "profileId": null, "scope": scope.as_str() }),
    );
    Ok(json_ok(true))
}

async fn apply_profile(
    State(state): State<ApiState>,
    Json(request): Json<ApplyProfileRequest>,
) -> ApiResult<Vec<String>> {
    let scope = parse_scope(&request.scope)?;
    let profile_id = request.id;
    let app_state = state.app_state.clone();
    let profile_id_for_apply = profile_id.clone();

    // ProviderService::switch uses a blocking bridge internally. Keep it away
    // from Tokio workers, matching the desktop command's synchronous execution.
    let (warnings, should_stop_proxy) = tokio::task::spawn_blocking(move || {
        ProfileService::apply(app_state.as_ref(), &profile_id_for_apply, scope)
    })
    .await
    .map_err(|error| ApiError::internal(format!("Profile apply task failed: {error}")))?
    .map_err(map_profile_error)?;

    if should_stop_proxy {
        // `stop` reports "not running" when there is already no listener. That
        // is an idempotent final state for profile switching, so log and keep
        // the successful apply result just like the desktop command.
        if let Err(error) = state.app_state.proxy_service.stop().await {
            log::warn!("Failed to stop proxy after Web profile switch: {error}");
        }
    }

    emit_profile_apply_events(&state, &profile_id, scope);
    Ok(json_ok(warnings))
}

fn emit_profile_apply_events(state: &ApiState, profile_id: &str, scope: ProfileScope) {
    for app_type in scope.apps() {
        let app_type_str = app_type.as_str();
        let (proxy_enabled, auto_failover_enabled) =
            state.app_state.db.get_proxy_flags_sync(app_type_str);
        let provider_id =
            crate::settings::get_effective_current_provider(&state.app_state.db, app_type)
                .ok()
                .flatten()
                .unwrap_or_default();
        state.sink.emit_json(
            "provider-switched",
            serde_json::json!({
                "appType": app_type_str,
                "proxyEnabled": proxy_enabled,
                "autoFailoverEnabled": auto_failover_enabled,
                "providerId": provider_id,
            }),
        );
    }
    state.sink.emit_json(
        "profile-applied",
        serde_json::json!({ "profileId": profile_id, "scope": scope.as_str() }),
    );
}
