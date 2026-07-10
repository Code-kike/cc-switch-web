use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::AppError;

/// 获取用户主目录，带回退和日志
///
/// ## Windows 注意事项
///
/// - `dirs::home_dir()` 在 Windows 上使用 `SHGetKnownFolderPath(FOLDERID_Profile)`，
///   返回的是真实用户目录（类似 `C:\\Users\\Alice`），与 v3.10.2 行为一致。
/// - 不要直接使用 `HOME` 环境变量：它可能由 Git/Cygwin/MSYS 等第三方工具注入，
///   且不一定等于用户目录，可能导致 `.cc-switch/cc-switch.db` 路径变化，从而“看起来像数据丢失”。
///
/// ## 测试隔离
///
/// 为了让 Windows CI/本地测试能稳定隔离真实用户数据，可通过 `CC_SWITCH_TEST_HOME`
/// 显式覆盖 home dir（仅用于测试/调试场景）。
pub fn get_home_dir() -> PathBuf {
    if let Ok(home) = std::env::var("CC_SWITCH_TEST_HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    dirs::home_dir().unwrap_or_else(|| {
        log::warn!("无法获取用户主目录，回退到当前目录");
        PathBuf::from(".")
    })
}

/// 获取 Claude Code 配置目录路径
pub fn get_claude_config_dir() -> PathBuf {
    if let Some(custom) = crate::settings::get_claude_override_dir() {
        return custom;
    }

    get_home_dir().join(".claude")
}

/// 默认 Claude MCP 配置文件路径 (~/.claude.json)
pub fn get_default_claude_mcp_path() -> PathBuf {
    get_home_dir().join(".claude.json")
}

fn derive_mcp_path_from_override(dir: &Path) -> Option<PathBuf> {
    let file_name = dir
        .file_name()
        .map(|name| name.to_string_lossy().to_string())?
        .trim()
        .to_string();
    if file_name.is_empty() {
        return None;
    }
    let parent = dir.parent().unwrap_or_else(|| Path::new(""));
    Some(parent.join(format!("{file_name}.json")))
}

/// 获取 Claude MCP 配置文件路径，若设置了目录覆盖则与覆盖目录同级
pub fn get_claude_mcp_path() -> PathBuf {
    if let Some(custom_dir) = crate::settings::get_claude_override_dir() {
        if let Some(path) = derive_mcp_path_from_override(&custom_dir) {
            return path;
        }
    }
    get_default_claude_mcp_path()
}

/// 获取 Claude Code 主配置文件路径
pub fn get_claude_settings_path() -> PathBuf {
    let dir = get_claude_config_dir();
    let settings = dir.join("settings.json");
    if settings.exists() {
        return settings;
    }
    // 兼容旧版命名：若存在旧文件则继续使用
    let legacy = dir.join("claude.json");
    if legacy.exists() {
        return legacy;
    }
    // 默认新建：回落到标准文件名 settings.json（不再生成 claude.json）
    settings
}

/// 获取应用配置目录路径 (~/.cc-switch)
pub fn get_app_config_dir() -> PathBuf {
    if let Some(custom) = crate::app_store::get_app_config_dir_override() {
        return custom;
    }

    let default_dir = get_home_dir().join(".cc-switch");

    // 兼容 v3.10.3：当用户环境存在 `HOME` 且与真实用户目录不同，
    // v3.10.3 可能在 `HOME/.cc-switch/` 下创建/使用了数据库。
    // 这里仅在“默认位置没有数据库”时回退到旧位置，避免再次出现“供应商消失”问题，
    // 同时也避免新安装因为 `HOME` 被设置而写入非预期路径。
    #[cfg(windows)]
    {
        let default_db = default_dir.join("cc-switch.db");
        if !default_db.exists() {
            if let Ok(home_env) = std::env::var("HOME") {
                let trimmed = home_env.trim();
                if !trimmed.is_empty() {
                    let legacy_dir = PathBuf::from(trimmed).join(".cc-switch");
                    if legacy_dir.join("cc-switch.db").exists() {
                        log::info!(
                            "Detected v3.10.3 legacy database at {}, using it instead of {}",
                            legacy_dir.display(),
                            default_dir.display()
                        );
                        return legacy_dir;
                    }
                }
            }
        }
    }

    default_dir
}

/// 获取应用配置文件路径
pub fn get_app_config_path() -> PathBuf {
    get_app_config_dir().join("config.json")
}

/// 清理供应商名称，确保文件名安全
#[allow(dead_code)]
pub fn sanitize_provider_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '-',
            _ => c,
        })
        .collect::<String>()
        .to_lowercase()
}

/// 获取供应商配置文件路径
#[allow(dead_code)]
pub fn get_provider_config_path(provider_id: &str, provider_name: Option<&str>) -> PathBuf {
    let base_name = provider_name
        .map(sanitize_provider_name)
        .unwrap_or_else(|| sanitize_provider_name(provider_id));

    get_claude_config_dir().join(format!("settings-{base_name}.json"))
}

/// 读取 JSON 配置文件
pub fn read_json_file<T: for<'a> Deserialize<'a>>(path: &Path) -> Result<T, AppError> {
    if !path.exists() {
        return Err(AppError::Config(format!("文件不存在: {}", path.display())));
    }

    let content = fs::read_to_string(path).map_err(|e| AppError::io(path, e))?;

    serde_json::from_str(&content).map_err(|e| AppError::json(path, e))
}

/// 递归排序 JSON 对象的键（按字母顺序），确保序列化输出是确定性的
fn sort_json_keys(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted_map = Map::new();
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            for key in keys {
                sorted_map.insert(key.clone(), sort_json_keys(&map[key]));
            }
            Value::Object(sorted_map)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(sort_json_keys).collect()),
        other => other.clone(),
    }
}

/// 写入 JSON 配置文件（键按字母排序，确保确定性输出）
pub fn write_json_file<T: Serialize>(path: &Path, data: &T) -> Result<(), AppError> {
    write_json_file_with_mode(path, data, AtomicWriteMode::RejectFinalSymlink)
}

/// 写入由外部应用拥有的 JSON 配置文件。
///
/// 若最终路径是一个现有且有效的文件符号链接，则原子替换其解析后的目标，保留链接本身。
pub fn write_json_file_managed<T: Serialize>(path: &Path, data: &T) -> Result<(), AppError> {
    write_json_file_with_mode(path, data, AtomicWriteMode::FollowManagedSymlink)
}

fn write_json_file_with_mode<T: Serialize>(
    path: &Path,
    data: &T,
    mode: AtomicWriteMode,
) -> Result<(), AppError> {
    // 确保目录存在
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }

    let value = serde_json::to_value(data).map_err(|e| AppError::JsonSerialize { source: e })?;
    let sorted_value = sort_json_keys(&value);
    let json = serde_json::to_string_pretty(&sorted_value)
        .map_err(|e| AppError::JsonSerialize { source: e })?;

    atomic_write_with_mode(path, json.as_bytes(), mode)
}

/// 原子写入文本文件（用于 TOML/纯文本）
pub fn write_text_file(path: &Path, data: &str) -> Result<(), AppError> {
    atomic_write_with_mode(path, data.as_bytes(), AtomicWriteMode::RejectFinalSymlink)
}

/// 原子写入由外部应用拥有的文本配置文件，并保留受支持的最终符号链接。
pub fn write_text_file_managed(path: &Path, data: &str) -> Result<(), AppError> {
    atomic_write_with_mode(path, data.as_bytes(), AtomicWriteMode::FollowManagedSymlink)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AtomicWriteMode {
    RejectFinalSymlink,
    FollowManagedSymlink,
}

fn resolve_atomic_write_path(path: &Path, mode: AtomicWriteMode) -> Result<PathBuf, AppError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => Some(metadata),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(AppError::io(path, error)),
    };

    let Some(metadata) = metadata else {
        return Ok(path.to_path_buf());
    };

    if !metadata.file_type().is_symlink() {
        return Ok(path.to_path_buf());
    }

    if mode == AtomicWriteMode::RejectFinalSymlink {
        return Err(AppError::Config(format!(
            "拒绝原子替换符号链接路径: {}",
            path.display()
        )));
    }

    let resolved = fs::canonicalize(path).map_err(|error| AppError::IoContext {
        context: format!("解析受管配置符号链接失败: {}", path.display()),
        source: error,
    })?;
    let target_metadata =
        fs::metadata(&resolved).map_err(|error| AppError::io(&resolved, error))?;
    if !target_metadata.is_file() {
        return Err(AppError::Config(format!(
            "受管配置符号链接目标不是普通文件: {} -> {}",
            path.display(),
            resolved.display()
        )));
    }

    Ok(resolved)
}

fn atomic_write_with_mode(path: &Path, data: &[u8], mode: AtomicWriteMode) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }

    let write_path = resolve_atomic_write_path(path, mode)?;
    atomic_write_resolved(&write_path, data)
}

/// 原子写入：写入临时文件后 rename 替换，避免半写状态
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<(), AppError> {
    atomic_write_with_mode(path, data, AtomicWriteMode::RejectFinalSymlink)
}

/// 原子写入由外部应用管理的配置目标，保留有效的最终符号链接。
pub fn atomic_write_managed(path: &Path, data: &[u8]) -> Result<(), AppError> {
    atomic_write_with_mode(path, data, AtomicWriteMode::FollowManagedSymlink)
}

/// Ensure a write target resolves inside the effective allowed root.
///
/// The root itself may be a user-managed directory symlink. Existing targets
/// are fully resolved; new targets resolve their parent before the filename is
/// appended. This prevents an allowlisted filename from escaping through a
/// nested/final symlink.
pub fn ensure_write_path_within_root(root: &Path, path: &Path) -> Result<(), AppError> {
    let canonical_root = fs::canonicalize(root).map_err(|error| AppError::IoContext {
        context: format!("解析允许写入目录失败: {}", root.display()),
        source: error,
    })?;

    let resolved_path = match fs::symlink_metadata(path) {
        Ok(_) => fs::canonicalize(path).map_err(|error| AppError::IoContext {
            context: format!("解析写入目标失败: {}", path.display()),
            source: error,
        })?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let parent = path.parent().ok_or_else(|| {
                AppError::Config(format!("写入目标缺少父目录: {}", path.display()))
            })?;
            let canonical_parent =
                fs::canonicalize(parent).map_err(|error| AppError::IoContext {
                    context: format!("解析写入目标父目录失败: {}", parent.display()),
                    source: error,
                })?;
            let filename = path.file_name().ok_or_else(|| {
                AppError::Config(format!("写入目标缺少文件名: {}", path.display()))
            })?;
            canonical_parent.join(filename)
        }
        Err(error) => return Err(AppError::io(path, error)),
    };

    if !resolved_path.starts_with(&canonical_root) {
        return Err(AppError::Config(format!(
            "写入目标超出允许目录: {} -> {}",
            path.display(),
            resolved_path.display()
        )));
    }

    Ok(())
}

fn atomic_write_resolved(path: &Path, data: &[u8]) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }

    let parent = path
        .parent()
        .ok_or_else(|| AppError::Config("无效的路径".to_string()))?;
    let mut tmp = parent.to_path_buf();
    let file_name = path
        .file_name()
        .ok_or_else(|| AppError::Config("无效的文件名".to_string()))?
        .to_string_lossy()
        .to_string();
    tmp.push(format!("{file_name}.tmp.{}", uuid::Uuid::new_v4().simple()));

    #[cfg(unix)]
    let existing_mode = {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path)
            .ok()
            .map(|metadata| metadata.permissions().mode())
    };

    let write_result = (|| -> Result<(), AppError> {
        // 临时文件总是新建文件：在写入凭证/配置内容前就以 0600 打开，
        // 消除 create(0644)→write→chmod 之间的全局可读竞态窗口。
        #[cfg(unix)]
        let mut f = {
            use std::os::unix::fs::OpenOptionsExt;
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&tmp)
                .map_err(|e| AppError::io(&tmp, e))?
        };
        #[cfg(not(unix))]
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
            .map_err(|e| AppError::io(&tmp, e))?;
        f.write_all(data).map_err(|e| AppError::io(&tmp, e))?;
        f.flush().map_err(|e| AppError::io(&tmp, e))?;
        Ok(())
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_file(&tmp);
        return Err(error);
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Some(mode) = existing_mode {
            // 已存在文件：保留原有权限位
            if let Err(error) = fs::set_permissions(&tmp, fs::Permissions::from_mode(mode)) {
                let _ = fs::remove_file(&tmp);
                return Err(AppError::io(&tmp, error));
            }
        }
        // 首次创建：临时文件已在上方以 0600 打开。
    }

    #[cfg(windows)]
    {
        // Windows 上 rename 目标存在会失败，先移除再重命名（尽量接近原子性）
        if path.exists() {
            let _ = fs::remove_file(path);
        }
        fs::rename(&tmp, path).map_err(|e| AppError::IoContext {
            context: format!("原子替换失败: {} -> {}", tmp.display(), path.display()),
            source: e,
        })?;
    }

    #[cfg(not(windows))]
    {
        if let Err(error) = fs::rename(&tmp, path) {
            let _ = fs::remove_file(&tmp);
            return Err(AppError::IoContext {
                context: format!("原子替换失败: {} -> {}", tmp.display(), path.display()),
                source: error,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_mcp_path_from_override_preserves_folder_name() {
        let override_dir = PathBuf::from("/tmp/profile/.claude");
        let derived = derive_mcp_path_from_override(&override_dir)
            .expect("should derive path for nested dir");
        assert_eq!(derived, PathBuf::from("/tmp/profile/.claude.json"));
    }

    #[test]
    fn derive_mcp_path_from_override_handles_non_hidden_folder() {
        let override_dir = PathBuf::from("/data/claude-config");
        let derived = derive_mcp_path_from_override(&override_dir)
            .expect("should derive path for standard dir");
        assert_eq!(derived, PathBuf::from("/data/claude-config.json"));
    }

    #[test]
    fn derive_mcp_path_from_override_supports_relative_rootless_dir() {
        let override_dir = PathBuf::from("claude");
        let derived = derive_mcp_path_from_override(&override_dir)
            .expect("should derive path for single segment");
        assert_eq!(derived, PathBuf::from("claude.json"));
    }

    #[test]
    fn derive_mcp_path_from_root_like_dir_returns_none() {
        let override_dir = PathBuf::from("/");
        assert!(derive_mcp_path_from_override(&override_dir).is_none());
    }

    #[test]
    fn sort_json_keys_sorts_top_level_object() {
        let input = serde_json::json!({
            "z": 1,
            "a": 2,
            "m": 3,
        });
        let sorted = sort_json_keys(&input);
        let serialized = serde_json::to_string(&sorted).unwrap();
        assert_eq!(serialized, r#"{"a":2,"m":3,"z":1}"#);
    }

    #[test]
    fn sort_json_keys_recurses_into_nested_objects() {
        let input = serde_json::json!({
            "outer_b": {"z": 1, "a": 2},
            "outer_a": {"y": 3, "b": 4},
        });
        let sorted = sort_json_keys(&input);
        let serialized = serde_json::to_string(&sorted).unwrap();
        assert_eq!(
            serialized,
            r#"{"outer_a":{"b":4,"y":3},"outer_b":{"a":2,"z":1}}"#
        );
    }

    #[test]
    fn sort_json_keys_preserves_array_order() {
        let input = serde_json::json!([3, 1, 2]);
        let sorted = sort_json_keys(&input);
        let serialized = serde_json::to_string(&sorted).unwrap();
        assert_eq!(serialized, "[3,1,2]");
    }

    #[test]
    fn sort_json_keys_sorts_objects_inside_arrays_but_keeps_array_order() {
        let input = serde_json::json!([
            {"z": 1, "a": 2},
            {"y": 3, "b": 4},
        ]);
        let sorted = sort_json_keys(&input);
        let serialized = serde_json::to_string(&sorted).unwrap();
        assert_eq!(serialized, r#"[{"a":2,"z":1},{"b":4,"y":3}]"#);
    }

    #[test]
    fn sort_json_keys_passes_through_primitives() {
        let cases = vec![
            serde_json::json!("hello"),
            serde_json::json!(42),
            serde_json::json!(3.5),
            serde_json::json!(true),
            serde_json::json!(null),
        ];
        for value in cases {
            let sorted = sort_json_keys(&value);
            assert_eq!(sorted, value);
        }
    }

    #[test]
    fn sort_json_keys_handles_empty_collections() {
        let empty_obj = serde_json::json!({});
        assert_eq!(
            serde_json::to_string(&sort_json_keys(&empty_obj)).unwrap(),
            "{}"
        );

        let empty_arr = serde_json::json!([]);
        assert_eq!(
            serde_json::to_string(&sort_json_keys(&empty_arr)).unwrap(),
            "[]"
        );
    }

    #[test]
    fn sort_json_keys_produces_identical_output_for_different_insertion_orders() {
        // 核心保证：同一逻辑配置无论键的插入顺序如何，写出的字节序列必须一致。
        let mut a = Map::new();
        a.insert("env".to_string(), serde_json::json!({"PATH": "/usr/bin"}));
        a.insert("model".to_string(), serde_json::json!("claude-sonnet-4-5"));
        a.insert("permissions".to_string(), serde_json::json!({"allow": []}));

        let mut b = Map::new();
        b.insert("permissions".to_string(), serde_json::json!({"allow": []}));
        b.insert("model".to_string(), serde_json::json!("claude-sonnet-4-5"));
        b.insert("env".to_string(), serde_json::json!({"PATH": "/usr/bin"}));

        let sorted_a = sort_json_keys(&Value::Object(a));
        let sorted_b = sort_json_keys(&Value::Object(b));

        assert_eq!(
            serde_json::to_string(&sorted_a).unwrap(),
            serde_json::to_string(&sorted_b).unwrap(),
        );
    }

    #[cfg(unix)]
    #[test]
    fn default_atomic_write_rejects_final_symlink_without_replacing_it() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.toml");
        let link = dir.path().join("config.toml");
        fs::write(&target, "old").unwrap();
        symlink(&target, &link).unwrap();

        let error = atomic_write(&link, b"new").expect_err("restricted write must reject link");

        assert!(error.to_string().contains("符号链接"));
        assert!(fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(fs::read_to_string(&target).unwrap(), "old");
    }

    #[cfg(unix)]
    #[test]
    fn managed_atomic_write_preserves_absolute_symlink_and_updates_target() {
        use std::os::unix::fs::symlink;
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.toml");
        let link = dir.path().join("config.toml");
        fs::write(&target, "old").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o640)).unwrap();
        symlink(&target, &link).unwrap();

        atomic_write_managed(&link, b"new").unwrap();

        assert!(fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(fs::read_to_string(&target).unwrap(), "new");
        assert_eq!(fs::read_to_string(&link).unwrap(), "new");
        assert_eq!(
            fs::metadata(&target).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_creates_new_file_with_private_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("credentials.json");

        atomic_write(&path, b"secret").unwrap();

        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[cfg(unix)]
    #[test]
    fn managed_atomic_write_preserves_relative_symlink() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.json");
        let link = dir.path().join("config.json");
        fs::write(&target, "old").unwrap();
        symlink("target.json", &link).unwrap();

        write_text_file_managed(&link, "new").unwrap();

        assert!(fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(fs::read_to_string(&target).unwrap(), "new");
    }

    #[cfg(unix)]
    #[test]
    fn managed_atomic_write_rejects_dangling_symlink() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let link = dir.path().join("config.toml");
        symlink("missing.toml", &link).unwrap();

        let error = atomic_write_managed(&link, b"new").expect_err("dangling link must fail");

        assert!(error.to_string().contains("解析受管配置符号链接失败"));
        assert!(fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
    }

    #[cfg(unix)]
    #[test]
    fn managed_atomic_write_rejects_directory_symlink() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let target_dir = dir.path().join("target-dir");
        let link = dir.path().join("config.toml");
        fs::create_dir(&target_dir).unwrap();
        symlink(&target_dir, &link).unwrap();

        let error = atomic_write_managed(&link, b"new").expect_err("directory link must fail");

        assert!(error.to_string().contains("不是普通文件"));
        assert!(fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
    }

    #[cfg(unix)]
    #[test]
    fn containment_check_rejects_final_symlink_outside_root() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("workspace");
        let outside = dir.path().join("outside.md");
        let link = root.join("AGENTS.md");
        fs::create_dir(&root).unwrap();
        fs::write(&outside, "outside").unwrap();
        symlink(&outside, &link).unwrap();

        let error = ensure_write_path_within_root(&root, &link)
            .expect_err("outside symlink must fail containment");

        assert!(error.to_string().contains("超出允许目录"));
    }

    #[cfg(unix)]
    #[test]
    fn restricted_write_rejects_final_symlink_even_when_target_is_inside_root() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("workspace");
        let target = root.join("target.md");
        let link = root.join("AGENTS.md");
        fs::create_dir(&root).unwrap();
        fs::write(&target, "old").unwrap();
        symlink(&target, &link).unwrap();

        ensure_write_path_within_root(&root, &link).unwrap();
        let error = write_text_file(&link, "new")
            .expect_err("restricted workspace writes must not follow final links");

        assert!(error.to_string().contains("符号链接"));
        assert!(fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(fs::read_to_string(&target).unwrap(), "old");
    }

    #[cfg(unix)]
    #[test]
    fn containment_check_accepts_new_file_under_symlinked_root() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let target_root = dir.path().join("workspace-target");
        let root_link = dir.path().join("workspace");
        fs::create_dir(&target_root).unwrap();
        symlink(&target_root, &root_link).unwrap();

        let path = root_link.join("AGENTS.md");
        ensure_write_path_within_root(&root_link, &path).unwrap();
        write_text_file(&path, "managed workspace").unwrap();

        assert!(fs::symlink_metadata(&root_link)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(
            fs::read_to_string(target_root.join("AGENTS.md")).unwrap(),
            "managed workspace"
        );
    }
}

/// 复制文件
pub fn copy_file(from: &Path, to: &Path) -> Result<(), AppError> {
    fs::copy(from, to).map_err(|e| AppError::IoContext {
        context: format!("复制文件失败 ({} -> {})", from.display(), to.display()),
        source: e,
    })?;
    Ok(())
}

/// 删除文件
pub fn delete_file(path: &Path) -> Result<(), AppError> {
    if path.exists() {
        fs::remove_file(path).map_err(|e| AppError::io(path, e))?;
    }
    Ok(())
}

/// 检查 Claude Code 配置状态
#[derive(Serialize, Deserialize)]
pub struct ConfigStatus {
    pub exists: bool,
    pub path: String,
}

/// 获取 Claude Code 配置状态
pub fn get_claude_config_status() -> ConfigStatus {
    let path = get_claude_settings_path();
    ConfigStatus {
        exists: path.exists(),
        path: path.to_string_lossy().to_string(),
    }
}
