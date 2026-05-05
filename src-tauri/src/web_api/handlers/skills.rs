use axum::{
    extract::{Multipart, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;
use std::path::Path;

use super::super::ApiState;
use super::common::{json_ok, ApiError, ApiResult};

#[derive(Deserialize)]
struct AppQuery {
    app: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstallUnifiedRequest {
    skill: crate::services::skill::DiscoverableSkill,
    current_app: String,
}

#[derive(Deserialize)]
struct IdRequest {
    id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurrentAppRequest {
    current_app: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestoreBackupRequest {
    backup_id: String,
    current_app: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToggleSkillAppRequest {
    id: String,
    app: String,
    enabled: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportSkillsFromAppsRequest {
    imports: Vec<crate::services::skill::ImportSkillSelection>,
}

#[derive(Deserialize)]
struct MigrateSkillStorageRequest {
    target: crate::services::skill::SkillStorageLocation,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacySkillRequest {
    app: Option<String>,
    directory: String,
}

#[derive(Deserialize)]
struct AddSkillRepoRequest {
    repo: crate::services::skill::SkillRepo,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchSkillsShRequest {
    query: String,
    limit: usize,
    offset: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoveSkillRepoQuery {
    owner: String,
    name: String,
}

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/skills/get-installed-skills", get(get_installed_skills))
        .route("/skills/get-skill-repos", get(get_skill_repos))
        .route(
            "/skills/get-skills-migration-result",
            get(get_skills_migration_result),
        )
        .route("/skills/get-skills", get(get_skills))
        .route("/skills/get-skills-for-app", get(get_skills_for_app))
        .route("/skills/install-skill-unified", post(install_skill_unified))
        .route(
            "/skills/uninstall-skill-unified",
            post(uninstall_skill_unified),
        )
        .route("/skills/restore-skill-backup", post(restore_skill_backup))
        .route("/skills/toggle-skill-app", post(toggle_skill_app))
        .route("/skills/scan-unmanaged-skills", get(scan_unmanaged_skills))
        .route(
            "/skills/import-skills-from-apps",
            post(import_skills_from_apps),
        )
        .route(
            "/skills/discover-available-skills",
            post(discover_available_skills),
        )
        .route("/skills/check-skill-updates", get(check_skill_updates))
        .route("/skills/update-skill", put(update_skill))
        .route("/skills/migrate-skill-storage", post(migrate_skill_storage))
        .route("/skills/install-skill", post(install_skill))
        .route("/skills/install-skill-for-app", post(install_skill_for_app))
        .route("/skills/install-skills-upload", post(install_skills_upload))
        .route("/skills/uninstall-skill", post(uninstall_skill))
        .route(
            "/skills/uninstall-skill-for-app",
            post(uninstall_skill_for_app),
        )
        .route("/skills/add-skill-repo", post(add_skill_repo))
        .route("/skills/search-skills-sh", post(search_skills_sh))
        .route("/skills/remove-skill-repo", delete(remove_skill_repo))
        .with_state(state)
}

fn parse_app_type(app: &str) -> Result<crate::app_config::AppType, ApiError> {
    use std::str::FromStr;
    crate::app_config::AppType::from_str(app).map_err(|err| ApiError::bad_request(err.to_string()))
}

async fn get_installed_skills(
    State(state): State<ApiState>,
) -> ApiResult<Vec<crate::app_config::InstalledSkill>> {
    let skills = crate::services::skill::SkillService::get_all_installed(&state.app_state.db)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(skills))
}

async fn get_skill_repos(
    State(state): State<ApiState>,
) -> ApiResult<Vec<crate::services::skill::SkillRepo>> {
    let repos = state
        .app_state
        .db
        .get_skill_repos()
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(repos))
}

async fn get_skills_migration_result(
) -> ApiResult<Option<crate::init_status::SkillsMigrationPayload>> {
    Ok(json_ok(crate::init_status::take_skills_migration_result()))
}

async fn get_skills(
    State(state): State<ApiState>,
) -> Result<Json<Vec<crate::services::skill::Skill>>, ApiError> {
    let repos = state
        .app_state
        .db
        .get_skill_repos()
        .map_err(ApiError::from_anyhow)?;
    let service = crate::services::skill::SkillService::new();
    let skills = service
        .list_skills(repos, &state.app_state.db)
        .await
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(skills))
}

async fn get_skills_for_app(
    State(state): State<ApiState>,
    Query(_query): Query<AppQuery>,
) -> Result<Json<Vec<crate::services::skill::Skill>>, ApiError> {
    get_skills(State(state)).await
}

async fn install_skill_unified(
    State(state): State<ApiState>,
    Json(request): Json<InstallUnifiedRequest>,
) -> Result<Json<crate::app_config::InstalledSkill>, ApiError> {
    let app = parse_app_type(&request.current_app)?;
    let service = crate::services::skill::SkillService::new();
    let skill = service
        .install(&state.app_state.db, &request.skill, &app)
        .await
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(skill))
}

async fn uninstall_skill_unified(
    State(state): State<ApiState>,
    Json(request): Json<IdRequest>,
) -> Result<Json<crate::services::skill::SkillUninstallResult>, ApiError> {
    let result = crate::services::skill::SkillService::uninstall(&state.app_state.db, &request.id)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(result))
}

async fn restore_skill_backup(
    State(state): State<ApiState>,
    Json(request): Json<RestoreBackupRequest>,
) -> Result<Json<crate::app_config::InstalledSkill>, ApiError> {
    let app = parse_app_type(&request.current_app)?;
    let skill = crate::services::skill::SkillService::restore_from_backup(
        &state.app_state.db,
        &request.backup_id,
        &app,
    )
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(skill))
}

async fn toggle_skill_app(
    State(state): State<ApiState>,
    Json(request): Json<ToggleSkillAppRequest>,
) -> ApiResult<bool> {
    let app = parse_app_type(&request.app)?;
    crate::services::skill::SkillService::toggle_app(
        &state.app_state.db,
        &request.id,
        &app,
        request.enabled,
    )
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(true))
}

async fn scan_unmanaged_skills(
    State(state): State<ApiState>,
) -> Result<Json<Vec<crate::app_config::UnmanagedSkill>>, ApiError> {
    let skills = crate::services::skill::SkillService::scan_unmanaged(&state.app_state.db)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(skills))
}

async fn import_skills_from_apps(
    State(state): State<ApiState>,
    Json(request): Json<ImportSkillsFromAppsRequest>,
) -> Result<Json<Vec<crate::app_config::InstalledSkill>>, ApiError> {
    let skills = crate::services::skill::SkillService::import_from_apps(
        &state.app_state.db,
        request.imports,
    )
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(skills))
}

async fn discover_available_skills(
    State(state): State<ApiState>,
) -> Result<Json<Vec<crate::services::skill::DiscoverableSkill>>, ApiError> {
    let repos = state
        .app_state
        .db
        .get_skill_repos()
        .map_err(ApiError::from_anyhow)?;
    let service = crate::services::skill::SkillService::new();
    let skills = service
        .discover_available(repos)
        .await
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(skills))
}

async fn check_skill_updates(
    State(state): State<ApiState>,
) -> Result<Json<Vec<crate::services::skill::SkillUpdateInfo>>, ApiError> {
    let service = crate::services::skill::SkillService::new();
    let updates = service
        .check_updates(&state.app_state.db)
        .await
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(updates))
}

async fn update_skill(
    State(state): State<ApiState>,
    Json(request): Json<IdRequest>,
) -> Result<Json<crate::app_config::InstalledSkill>, ApiError> {
    let service = crate::services::skill::SkillService::new();
    let skill = service
        .update_skill(&state.app_state.db, &request.id)
        .await
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(skill))
}

async fn migrate_skill_storage(
    State(state): State<ApiState>,
    Json(request): Json<MigrateSkillStorageRequest>,
) -> Result<Json<crate::services::skill::MigrationResult>, ApiError> {
    let result =
        crate::services::skill::SkillService::migrate_storage(&state.app_state.db, request.target)
            .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(result))
}

async fn install_skill(
    State(state): State<ApiState>,
    Json(request): Json<LegacySkillRequest>,
) -> ApiResult<bool> {
    install_skill_for_app(State(state), Json(request)).await
}

async fn install_skill_for_app(
    State(state): State<ApiState>,
    Json(request): Json<LegacySkillRequest>,
) -> ApiResult<bool> {
    let app = parse_app_type(request.app.as_deref().unwrap_or("claude"))?;
    let repos = state
        .app_state
        .db
        .get_skill_repos()
        .map_err(ApiError::from_anyhow)?;
    let service = crate::services::skill::SkillService::new();
    let skills = service
        .discover_available(repos)
        .await
        .map_err(ApiError::from_anyhow)?;
    let skill = skills
        .into_iter()
        .find(|s| {
            let install_name = std::path::Path::new(&s.directory)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| s.directory.clone());
            install_name.eq_ignore_ascii_case(&request.directory)
                || s.directory.eq_ignore_ascii_case(&request.directory)
        })
        .ok_or_else(|| ApiError::bad_request(format!("Skill not found: {}", request.directory)))?;
    service
        .install(&state.app_state.db, &skill, &app)
        .await
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(true))
}

async fn uninstall_skill(
    State(state): State<ApiState>,
    Json(request): Json<LegacySkillRequest>,
) -> Result<Json<crate::services::skill::SkillUninstallResult>, ApiError> {
    uninstall_skill_for_app(State(state), Json(request)).await
}

async fn uninstall_skill_for_app(
    State(state): State<ApiState>,
    Json(request): Json<LegacySkillRequest>,
) -> Result<Json<crate::services::skill::SkillUninstallResult>, ApiError> {
    let _ = request.app.as_deref().map(parse_app_type).transpose()?;
    let installed = crate::services::skill::SkillService::get_all_installed(&state.app_state.db)
        .map_err(ApiError::from_anyhow)?;
    let skill = installed
        .into_iter()
        .find(|s| s.directory.eq_ignore_ascii_case(&request.directory))
        .ok_or_else(|| {
            ApiError::bad_request(format!("Skill not installed: {}", request.directory))
        })?;
    let result = crate::services::skill::SkillService::uninstall(&state.app_state.db, &skill.id)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(result))
}

async fn add_skill_repo(
    State(state): State<ApiState>,
    Json(request): Json<AddSkillRepoRequest>,
) -> ApiResult<bool> {
    state
        .app_state
        .db
        .save_skill_repo(&request.repo)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(true))
}

async fn install_skills_upload(
    State(state): State<ApiState>,
    Query(query): Query<AppQuery>,
    mut multipart: Multipart,
) -> Result<Json<Vec<crate::app_config::InstalledSkill>>, ApiError> {
    let app = parse_app_type(query.app.as_deref().unwrap_or("claude"))?;
    let mut upload = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(ApiError::from_anyhow)?
    {
        if field.name() == Some("file") {
            let file_name = field
                .file_name()
                .map(ToString::to_string)
                .unwrap_or_else(|| "skills.zip".to_string());
            let bytes = field.bytes().await.map_err(ApiError::from_anyhow)?;
            upload = Some((file_name, bytes));
            break;
        }
    }

    let (file_name, bytes) =
        upload.ok_or_else(|| ApiError::upload_required("Missing upload field: file"))?;
    let suffix = if file_name.ends_with(".skill") {
        ".skill"
    } else {
        ".zip"
    };
    let upload_name = Path::new(&file_name)
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::trim)
        .filter(|name| !name.is_empty() && *name != "." && *name != "..")
        .map(|name| {
            if name.ends_with(suffix) {
                name.to_string()
            } else {
                format!("{name}{suffix}")
            }
        })
        .unwrap_or_else(|| format!("skills{suffix}"));
    let temp_dir = tempfile::tempdir().map_err(ApiError::from_anyhow)?;
    let upload_path = temp_dir.path().join(upload_name);
    std::fs::write(&upload_path, &bytes).map_err(ApiError::from_anyhow)?;
    let installed = crate::services::skill::SkillService::install_from_zip(
        &state.app_state.db,
        &upload_path,
        &app,
    )
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(installed))
}

async fn search_skills_sh(
    Json(request): Json<SearchSkillsShRequest>,
) -> Result<Json<crate::services::skill::SkillsShSearchResult>, ApiError> {
    let result = crate::services::skill::SkillService::search_skills_sh(
        &request.query,
        request.limit,
        request.offset,
    )
    .await
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(result))
}

async fn remove_skill_repo(
    State(state): State<ApiState>,
    Query(query): Query<RemoveSkillRepoQuery>,
) -> ApiResult<bool> {
    state
        .app_state
        .db
        .delete_skill_repo(&query.owner, &query.name)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(true))
}
