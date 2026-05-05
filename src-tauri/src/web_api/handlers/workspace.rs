use axum::{
    extract::Query,
    routing::{get, post, put},
    Json, Router,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

use super::super::ApiState;
use super::common::{json_ok, ApiError, ApiResult};

const ALLOWED_FILES: &[&str] = &[
    "AGENTS.md",
    "SOUL.md",
    "USER.md",
    "IDENTITY.md",
    "TOOLS.md",
    "MEMORY.md",
    "HEARTBEAT.md",
    "BOOTSTRAP.md",
    "BOOT.md",
];

static DAILY_MEMORY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\d{4}-\d{2}-\d{2}\.md$").unwrap());

#[derive(Deserialize)]
struct FilenameQuery {
    filename: String,
}

#[derive(Deserialize)]
struct DailyMemoryQuery {
    query: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileWriteRequest {
    filename: String,
    content: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DailyMemoryFileInfo {
    filename: String,
    date: String,
    size_bytes: u64,
    modified_at: u64,
    preview: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DailyMemorySearchResult {
    filename: String,
    date: String,
    size_bytes: u64,
    modified_at: u64,
    snippet: String,
    match_count: usize,
}

pub fn router(_state: ApiState) -> Router {
    Router::new()
        .route("/workspace/read-workspace-file", get(read_workspace_file))
        .route("/workspace/write-workspace-file", put(write_workspace_file))
        .route(
            "/system/list_daily_memory_files",
            post(list_daily_memory_files),
        )
        .route(
            "/system/read_daily_memory_file",
            post(read_daily_memory_file),
        )
        .route(
            "/system/write_daily_memory_file",
            post(write_daily_memory_file),
        )
        .route(
            "/system/delete_daily_memory_file",
            post(delete_daily_memory_file),
        )
        .route(
            "/system/search_daily_memory_files",
            post(search_daily_memory_files),
        )
}

fn validate_filename(filename: &str) -> Result<(), ApiError> {
    if !ALLOWED_FILES.contains(&filename) {
        return Err(ApiError::bad_request(format!(
            "Invalid workspace filename: {filename}. Allowed: {}",
            ALLOWED_FILES.join(", ")
        )));
    }
    Ok(())
}

fn validate_daily_memory_filename(filename: &str) -> Result<(), ApiError> {
    if !DAILY_MEMORY_RE.is_match(filename) {
        return Err(ApiError::bad_request(format!(
            "Invalid daily memory filename: {filename}. Expected: YYYY-MM-DD.md"
        )));
    }
    Ok(())
}

fn workspace_dir() -> std::path::PathBuf {
    crate::openclaw_config::get_openclaw_dir().join("workspace")
}

fn memory_dir() -> std::path::PathBuf {
    workspace_dir().join("memory")
}

async fn read_workspace_file(Query(query): Query<FilenameQuery>) -> ApiResult<Option<String>> {
    validate_filename(&query.filename)?;
    let path = workspace_dir().join(&query.filename);
    if !path.exists() {
        return Ok(json_ok(None));
    }
    let content = std::fs::read_to_string(&path)
        .map(Some)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(content))
}

async fn write_workspace_file(Json(request): Json<FileWriteRequest>) -> ApiResult<()> {
    validate_filename(&request.filename)?;
    let dir = workspace_dir();
    std::fs::create_dir_all(&dir).map_err(ApiError::from_anyhow)?;
    let path = dir.join(&request.filename);
    crate::config::write_text_file(&path, &request.content).map_err(ApiError::from_anyhow)?;
    Ok(json_ok(()))
}

async fn list_daily_memory_files() -> ApiResult<Vec<DailyMemoryFileInfo>> {
    let dir = memory_dir();
    if !dir.exists() {
        return Ok(json_ok(Vec::new()));
    }

    let entries = std::fs::read_dir(&dir).map_err(ApiError::from_anyhow)?;
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".md") {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }

        let preview = std::fs::read_to_string(entry.path())
            .unwrap_or_default()
            .chars()
            .take(200)
            .collect::<String>();

        files.push(DailyMemoryFileInfo {
            date: name.trim_end_matches(".md").to_string(),
            filename: name,
            size_bytes: meta.len(),
            modified_at: modified_at_secs(&meta),
            preview,
        });
    }
    files.sort_by(|a, b| b.filename.cmp(&a.filename));
    Ok(json_ok(files))
}

async fn read_daily_memory_file(Json(request): Json<FilenameQuery>) -> ApiResult<Option<String>> {
    validate_daily_memory_filename(&request.filename)?;
    let path = memory_dir().join(&request.filename);
    if !path.exists() {
        return Ok(json_ok(None));
    }
    let content = std::fs::read_to_string(&path)
        .map(Some)
        .map_err(ApiError::from_anyhow)?;
    Ok(json_ok(content))
}

async fn write_daily_memory_file(Json(request): Json<FileWriteRequest>) -> ApiResult<()> {
    validate_daily_memory_filename(&request.filename)?;
    let dir = memory_dir();
    std::fs::create_dir_all(&dir).map_err(ApiError::from_anyhow)?;
    let path = dir.join(&request.filename);
    crate::config::write_text_file(&path, &request.content).map_err(ApiError::from_anyhow)?;
    Ok(json_ok(()))
}

async fn delete_daily_memory_file(Json(request): Json<FilenameQuery>) -> ApiResult<()> {
    validate_daily_memory_filename(&request.filename)?;
    let path = memory_dir().join(&request.filename);
    if path.exists() {
        std::fs::remove_file(&path).map_err(ApiError::from_anyhow)?;
    }
    Ok(json_ok(()))
}

async fn search_daily_memory_files(
    Json(request): Json<DailyMemoryQuery>,
) -> ApiResult<Vec<DailyMemorySearchResult>> {
    let dir = memory_dir();
    if !dir.exists() || request.query.is_empty() {
        return Ok(json_ok(Vec::new()));
    }

    let query_lower = request.query.to_lowercase();
    let mut results = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(ApiError::from_anyhow)?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".md") {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }

        let date = name.trim_end_matches(".md").to_string();
        let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
        let content_lower = content.to_lowercase();
        let matches = content_lower
            .match_indices(&query_lower)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let date_matches = date.to_lowercase().contains(&query_lower);
        if matches.is_empty() && !date_matches {
            continue;
        }

        results.push(DailyMemorySearchResult {
            filename: name,
            date,
            size_bytes: meta.len(),
            modified_at: modified_at_secs(&meta),
            snippet: build_snippet(&content, matches.first().copied()),
            match_count: matches.len(),
        });
    }
    results.sort_by(|a, b| b.filename.cmp(&a.filename));
    Ok(json_ok(results))
}

fn modified_at_secs(meta: &std::fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn build_snippet(content: &str, first_match: Option<usize>) -> String {
    let start = first_match
        .map(|index| index.saturating_sub(50))
        .unwrap_or(0);
    let start = floor_char_boundary(content, start);
    let end_hint = first_match.map(|index| index + 70).unwrap_or(120);
    let end = ceil_char_boundary(content, end_hint.min(content.len()));
    let mut snippet = String::new();
    if start > 0 {
        snippet.push_str("...");
    }
    snippet.push_str(&content[start..end]);
    if end < content.len() {
        snippet.push_str("...");
    }
    snippet
}

fn floor_char_boundary(s: &str, mut index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    while !s.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn ceil_char_boundary(s: &str, mut index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    while !s.is_char_boundary(index) {
        index += 1;
    }
    index
}
