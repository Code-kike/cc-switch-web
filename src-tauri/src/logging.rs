//! 日志脱敏与前端错误日志落盘（tauri-free，桌面与 Web 双运行时共用）。
//!
//! URL 脱敏 API 供所有会把外部 URL 写进日志的路径使用（proxy 转发、webdav、
//! model fetch、deeplink 等）；`append_frontend_error` 是前端错误日志的统一
//! 落盘出口（桌面 Tauri command 与 Web API handler 都汇聚到这里）。

use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::error::AppError;

pub(crate) struct RedactedUrl<'a> {
    url: &'a str,
    known_secrets: &'a [String],
}

impl fmt::Display for RedactedUrl<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&redact_url_for_log_with_secrets(
            self.url,
            self.known_secrets,
        ))
    }
}

/// 为日志提供惰性 URL 脱敏包装；只有日志实际输出时才解析和重建 URL。
pub(crate) fn url_for_log(url: &str) -> RedactedUrl<'_> {
    RedactedUrl {
        url,
        known_secrets: &[],
    }
}

/// 为持有确切认证材料的调用方提供优先精确匹配、再启发式兜底的 URL 脱敏。
pub(crate) fn url_for_log_with_secrets<'a>(
    url: &'a str,
    known_secrets: &'a [String],
) -> RedactedUrl<'a> {
    RedactedUrl { url, known_secrets }
}

/// 已知密钥参与子串脱敏的最短长度：过短的值(如 "api")当作子串会误伤无关文本，
/// 所以只对足够长、几乎不可能是普通词的值做替换。
const MIN_KNOWN_SECRET_LEN: usize = 8;

/// 唯一的密钥脱敏原语：把字符串里出现的、我们确切握有的密钥值替换为 [REDACTED]。
/// 不做任何“看起来像密钥”的形状猜测——只隐藏已知值，天然收敛、不误伤正常路径。
fn redact_known_secrets(text: &str, known_secrets: &[String]) -> String {
    let mut output = text.to_string();
    for secret in known_secrets {
        if secret.chars().count() >= MIN_KNOWN_SECRET_LEN {
            output = output.replace(secret.as_str(), "[REDACTED]");
        }
    }
    output
}

/// 无 scheme 的裸 authority 形态(如 `user:pass@host/path`)剥掉 userinfo：
/// 仅当 `@` 出现在第一个 `/` 之前时才视为凭据。
fn strip_bare_userinfo(input: &str) -> &str {
    let authority_end = input.find('/').unwrap_or(input.len());
    match input[..authority_end].rfind('@') {
        Some(at) => &input[at + 1..],
        None => input,
    }
}

pub(crate) fn redact_url_for_log(url_str: &str) -> String {
    redact_url_for_log_with_secrets(url_str, &[])
}

/// 为日志脱敏 URL：剥掉 userinfo(user:pass@) 与整个 query/fragment，保留
/// scheme/host/port/path 供诊断(如 base_url 配错路径导致 404)，最后再抹掉已知密钥值。
pub(crate) fn redact_url_for_log_with_secrets(url_str: &str, known_secrets: &[String]) -> String {
    let scheme_relative = url_str.starts_with("//");
    let parsed = if scheme_relative {
        url::Url::parse(&format!("https:{url_str}"))
    } else {
        url::Url::parse(url_str)
    };

    let sanitized = match parsed {
        Ok(mut url) if url.has_host() => {
            let _ = url.set_username("");
            let _ = url.set_password(None);
            url.set_query(None);
            url.set_fragment(None);
            let rendered = url.as_str();
            if scheme_relative {
                rendered
                    .strip_prefix("https:")
                    .unwrap_or(rendered)
                    .to_string()
            } else {
                rendered.to_string()
            }
        }
        _ => {
            // 解析失败(相对路径、含裸 userinfo 的非法 URL 等)：丢掉 query/fragment，
            // 尽力剥掉 userinfo，其余原样保留。
            let without_tail = url_str.split(['?', '#']).next().unwrap_or(url_str);
            strip_bare_userinfo(without_tail).to_string()
        }
    };

    redact_known_secrets(&sanitized, known_secrets)
}

/// 只保留 `scheme://host:port`，丢掉 path/query/userinfo。用于我们手里没有任何
/// 已知密钥可脱敏 path 的场景——凭据可能整个内嵌在 base_url 的 path 里，此时
/// 记录 path 无法保证不泄漏，只能退回到 origin。
pub(crate) fn redact_url_origin_for_log(url_str: &str) -> String {
    let scheme_relative = url_str.starts_with("//");
    let parsed = if scheme_relative {
        url::Url::parse(&format!("https:{url_str}"))
    } else {
        url::Url::parse(url_str)
    };

    match parsed {
        Ok(url) if url.has_host() => {
            let authority = &url[url::Position::BeforeHost..url::Position::AfterPort];
            if scheme_relative {
                format!("//{authority}")
            } else {
                format!("{}://{authority}", url.scheme())
            }
        }
        _ => "[invalid target]".to_string(),
    }
}

// ---------------------------------------------------------------------------
// 大小轮转（crash.log / frontend.log 共用）
// ---------------------------------------------------------------------------

pub(crate) fn rotated_log_path(path: &Path, index: usize) -> PathBuf {
    let mut rotated = path.as_os_str().to_os_string();
    rotated.push(format!(".{index}"));
    PathBuf::from(rotated)
}

/// 达到 `max_size` 时把 `path` 轮转为 `path.1`（旧的 `path.1` 顺移为
/// `path.2` …），最多保留 `archives_to_keep` 个归档。调用方负责并发互斥。
pub(crate) fn rotate_log_if_needed_with_limit(
    path: &Path,
    max_size: u64,
    archives_to_keep: usize,
) -> std::io::Result<()> {
    let size = match fs::metadata(path) {
        Ok(metadata) => metadata.len(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    if size < max_size || archives_to_keep == 0 {
        return Ok(());
    }

    for index in (1..=archives_to_keep).rev() {
        let source = if index == 1 {
            path.to_path_buf()
        } else {
            rotated_log_path(path, index - 1)
        };
        if !source.exists() {
            continue;
        }

        let destination = rotated_log_path(path, index);
        if destination.exists() {
            fs::remove_file(&destination)?;
        }
        fs::rename(source, destination)?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// 前端错误日志落盘（frontend.log）
// ---------------------------------------------------------------------------

const FRONTEND_LOG_FILE_NAME: &str = "frontend.log";
const FRONTEND_LOG_MAX_SIZE: u64 = 5 * 1024 * 1024;
const FRONTEND_LOG_ARCHIVES_TO_KEEP: usize = 2;
/// 服务端兜底上限：前端自身已把消息截到 ~12K 字符，这里只防御异常客户端。
const FRONTEND_LOG_MAX_MESSAGE_CHARS: usize = 20_000;

static FRONTEND_LOG_LOCK: Mutex<()> = Mutex::new(());

fn bound_frontend_message(message: &str) -> String {
    let mut bounded: String = message
        .chars()
        .take(FRONTEND_LOG_MAX_MESSAGE_CHARS)
        .collect();
    if message
        .chars()
        .nth(FRONTEND_LOG_MAX_MESSAGE_CHARS)
        .is_some()
    {
        bounded.push_str("\n[truncated by backend]");
    }
    bounded
}

fn open_frontend_log_for_append(path: &Path) -> std::io::Result<fs::File> {
    let mut options = fs::OpenOptions::new();
    options.create(true).append(true);
    // 新建时使用私有权限；已存在的文件保留其权限位。
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

/// 把（前端已脱敏的）错误消息追加写入 `<app_config_dir>/logs/frontend.log`，
/// 并镜像到进程日志（桌面 → cc-switch.log/stdout，Web → stdout/journald）。
///
/// 桌面 Tauri command 与 Web API handler 是本函数仅有的两个入口；
/// 落盘不受动态日志级别影响，白屏崩溃在任何配置下都留痕。
pub fn append_frontend_error(message: &str) -> Result<(), AppError> {
    let bounded = bound_frontend_message(message);

    let log_dir = crate::config::get_app_config_dir().join("logs");
    fs::create_dir_all(&log_dir).map_err(|e| AppError::io(&log_dir, e))?;
    let path = log_dir.join(FRONTEND_LOG_FILE_NAME);

    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f %:z");
    let entry = format!("[{timestamp}] {bounded}\n");

    // size check、轮转与追加在同一临界区内，避免并发上报竞争 rename 丢归档。
    {
        let _guard = FRONTEND_LOG_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _ = rotate_log_if_needed_with_limit(
            &path,
            FRONTEND_LOG_MAX_SIZE,
            FRONTEND_LOG_ARCHIVES_TO_KEEP,
        );
        let mut file = open_frontend_log_for_append(&path).map_err(|e| AppError::io(&path, e))?;
        file.write_all(entry.as_bytes())
            .map_err(|e| AppError::io(&path, e))?;
        file.flush().map_err(|e| AppError::io(&path, e))?;
    }

    log::error!(target: "frontend", "{bounded}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // serial：与其他读写进程级 CC_SWITCH_TEST_HOME 的测试互斥。
    #[serial_test::serial]
    fn append_frontend_error_persists_to_frontend_log_on_disk() {
        struct EnvGuard(Option<std::ffi::OsString>);
        impl Drop for EnvGuard {
            fn drop(&mut self) {
                match self.0.take() {
                    Some(value) => std::env::set_var("CC_SWITCH_TEST_HOME", value),
                    None => std::env::remove_var("CC_SWITCH_TEST_HOME"),
                }
            }
        }
        let temp = tempfile::tempdir().unwrap();
        let _guard = EnvGuard(std::env::var_os("CC_SWITCH_TEST_HOME"));
        std::env::set_var("CC_SWITCH_TEST_HOME", temp.path());

        append_frontend_error("[frontend] window.error\nError: boom").unwrap();

        let log_path = temp
            .path()
            .join(".cc-switch")
            .join("logs")
            .join(FRONTEND_LOG_FILE_NAME);
        let contents = fs::read_to_string(&log_path).expect("frontend.log must exist on disk");
        assert!(contents.contains("[frontend] window.error"), "{contents}");
        assert!(contents.contains("Error: boom"), "{contents}");

        // 追加第二条：文件按追加写入，不覆盖。
        append_frontend_error("second entry").unwrap();
        let contents = fs::read_to_string(&log_path).unwrap();
        assert!(contents.contains("Error: boom"), "{contents}");
        assert!(contents.contains("second entry"), "{contents}");

        // Unix 上新建文件必须是私有权限。
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&log_path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "frontend.log must be private, got {mode:o}");
        }
    }

    #[test]
    fn log_url_redaction_strips_credentials_and_query_keeps_path() {
        // userinfo 与整个 query 剥离，path 保留用于诊断 base_url 配错。
        assert_eq!(
            redact_url_for_log(
                "https://user:secret@example.com:8443/v1/models?key=top-secret&alt=sse"
            ),
            "https://example.com:8443/v1/models"
        );
        // scheme-relative 保持形态，userinfo 去掉。
        assert_eq!(
            redact_url_for_log("//user:sk-secret@gw.example.com/v1"),
            "//gw.example.com/v1"
        );
        // 无 scheme 的裸 userinfo。
        assert_eq!(
            redact_url_for_log("user:sk-secret@gw.example.com/v1"),
            "gw.example.com/v1"
        );
        // 无法解析为绝对 URL 时：丢 query，其余原样保留。
        assert_eq!(redact_url_for_log("not-a-url?token=secret"), "not-a-url");
        // 不再对 path 段做“看起来像密钥”的形状猜测，正常路径完整保留。
        assert_eq!(
            redact_url_for_log("https://host.example/v1/models/gemini-2.5-pro"),
            "https://host.example/v1/models/gemini-2.5-pro"
        );
    }

    #[test]
    fn log_url_redaction_replaces_known_secret_values() {
        // 精确匹配已知密钥值：无论它出现在 path 还是别处都被抹掉。
        let secrets = vec!["k-9f3a7c2b1e".to_string()];
        assert_eq!(
            redact_url_for_log_with_secrets("https://gw.example.com/k-9f3a7c2b1e/v1", &secrets),
            "https://gw.example.com/[REDACTED]/v1"
        );
        // 过短(<8)的已知值不参与子串脱敏，避免误伤 /v1/ 之类的正常路径。
        let short_secrets = vec!["api".to_string()];
        assert_eq!(
            redact_url_for_log_with_secrets("https://api.example.com/v1", &short_secrets),
            "https://api.example.com/v1"
        );
    }

    #[test]
    fn log_url_origin_drops_path_for_credential_in_path() {
        // 没有已知密钥可脱敏时，凭据可能整个内嵌在 path，只记 origin。
        assert_eq!(
            redact_url_origin_for_log("https://gw.example.com/k-9f3a7c2b1e/v1"),
            "https://gw.example.com"
        );
        assert_eq!(
            redact_url_origin_for_log("https://user:pass@gw.example.com:8443/secret/v1"),
            "https://gw.example.com:8443"
        );
        assert_eq!(
            redact_url_origin_for_log("//gw.example.com/secret/v1"),
            "//gw.example.com"
        );
        assert_eq!(redact_url_origin_for_log("not a url"), "[invalid target]");
    }

    #[test]
    fn log_rotation_keeps_bounded_archives() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("frontend.log");

        fs::write(&path, b"first").unwrap();
        rotate_log_if_needed_with_limit(&path, 4, 2).unwrap();
        assert!(!path.exists());
        assert_eq!(fs::read(rotated_log_path(&path, 1)).unwrap(), b"first");

        fs::write(&path, b"second").unwrap();
        rotate_log_if_needed_with_limit(&path, 4, 2).unwrap();
        assert_eq!(fs::read(rotated_log_path(&path, 1)).unwrap(), b"second");
        assert_eq!(fs::read(rotated_log_path(&path, 2)).unwrap(), b"first");

        fs::write(&path, b"third").unwrap();
        rotate_log_if_needed_with_limit(&path, 4, 2).unwrap();
        assert_eq!(fs::read(rotated_log_path(&path, 1)).unwrap(), b"third");
        assert_eq!(fs::read(rotated_log_path(&path, 2)).unwrap(), b"second");
        assert!(!rotated_log_path(&path, 3).exists());
    }

    #[test]
    fn frontend_message_is_bounded_server_side() {
        let oversized = "x".repeat(FRONTEND_LOG_MAX_MESSAGE_CHARS + 100);
        let bounded = bound_frontend_message(&oversized);
        assert!(bounded.ends_with("[truncated by backend]"));
        assert!(bounded.chars().count() <= FRONTEND_LOG_MAX_MESSAGE_CHARS + 32);

        let small = bound_frontend_message("hello");
        assert_eq!(small, "hello");
    }
}
