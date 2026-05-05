use axum::{
    extract::{Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;

use super::super::ApiState;
use super::common::{json_ok, web_not_supported, ApiError, ApiResult};

#[derive(Deserialize)]
struct BackupFilenameQuery {
    filename: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenameBackupRequest {
    old_filename: String,
    new_name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkillBackupIdQuery {
    backup_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestoreSkillBackupRequest {
    backup_id: String,
    current_app: String,
}

fn parse_app_type(app: &str) -> Result<crate::app_config::AppType, ApiError> {
    use std::str::FromStr;
    crate::app_config::AppType::from_str(app).map_err(|err| ApiError::bad_request(err.to_string()))
}

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/backups/create-db-backup", post(create_db_backup))
        .route("/backups/list-db-backups", get(list_db_backups))
        .route("/backups/restore-db-backup", post(restore_db_backup))
        .route("/backups/rename-db-backup", post(rename_db_backup))
        .route("/backups/delete-db-backup", delete(delete_db_backup))
        .route("/backups/get-skill-backups", get(get_skill_backups))
        .route("/backups/delete-skill-backup", delete(delete_skill_backup))
        .route("/backups/restore-skill-backup", post(restore_skill_backup))
        .route("/backups/restore-env-backup", post(web_not_supported))
        .with_state(state)
}

async fn create_db_backup(State(state): State<ApiState>) -> ApiResult<String> {
    let db = state.app_state.db.clone();
    let filename = tokio::task::spawn_blocking(move || match db.backup_database_file()? {
        Some(path) => Ok(path
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default()),
        None => Err(crate::error::AppError::Config(
            "Database file not found, backup skipped".to_string(),
        )),
    })
    .await
    .map_err(ApiError::from_anyhow)?
    .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(filename))
}

async fn list_db_backups() -> ApiResult<Vec<crate::database::backup::BackupEntry>> {
    let backups = crate::database::Database::list_backups().map_err(ApiError::from_anyhow)?;
    Ok(json_ok(backups))
}

async fn restore_db_backup(
    State(state): State<ApiState>,
    Json(request): Json<BackupFilenameQuery>,
) -> ApiResult<String> {
    let db = state.app_state.db.clone();
    let restored = tokio::task::spawn_blocking(move || db.restore_from_backup(&request.filename))
        .await
        .map_err(ApiError::from_anyhow)?
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(restored))
}

async fn rename_db_backup(Json(request): Json<RenameBackupRequest>) -> ApiResult<String> {
    let renamed =
        crate::database::Database::rename_backup(&request.old_filename, &request.new_name)
            .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(renamed))
}

async fn delete_db_backup(Query(query): Query<BackupFilenameQuery>) -> ApiResult<()> {
    crate::database::Database::delete_backup(&query.filename).map_err(ApiError::from_anyhow)?;
    Ok(json_ok(()))
}

async fn get_skill_backups() -> ApiResult<Vec<crate::services::skill::SkillBackupEntry>> {
    let backups =
        crate::services::skill::SkillService::list_backups().map_err(ApiError::from_anyhow)?;
    Ok(json_ok(backups))
}

async fn delete_skill_backup(Query(query): Query<SkillBackupIdQuery>) -> ApiResult<bool> {
    crate::services::skill::SkillService::delete_backup(&query.backup_id)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(true))
}

async fn restore_skill_backup(
    State(state): State<ApiState>,
    Json(request): Json<RestoreSkillBackupRequest>,
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
