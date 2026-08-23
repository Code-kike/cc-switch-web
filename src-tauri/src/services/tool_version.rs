use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const VALID_TOOLS: [&str; 5] = ["claude", "codex", "gemini", "grok", "opencode"];
const TOOL_COMMAND_OUTPUT_LIMIT: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolVersion {
    pub name: String,
    pub version: Option<String>,
    pub latest_version: Option<String>,
    pub error: Option<String>,
    pub env_type: String,
    pub wsl_distro: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WslShellPreferenceInput {
    #[serde(default)]
    pub wsl_shell: Option<String>,
    #[serde(default)]
    pub wsl_shell_flag: Option<String>,
}

pub async fn get_tool_versions(
    tools: Option<Vec<String>>,
    wsl_shell_by_tool: Option<HashMap<String, WslShellPreferenceInput>>,
) -> Vec<ToolVersion> {
    // Web-first architecture decision (fork creation commit 64a34eb3): the Web
    // server runs on Linux where Windows CLI detection is meaningless, so this
    // `get_tool_versions` entry point returns an empty list on Windows. This
    // makes `get_single_tool_version_impl` and the `scan_cli_version` fallback
    // unreachable on Windows **for this entry point only**.
    //
    // IMPORTANT — the helpers added by upstream `de9af49a` (registry-PATH merge
    // `effective_path_*`, standalone installer dirs, `windows_path_lookup_command`,
    // App Execution Alias filtering, `windows_shell_compatible_path`) are NOT dead
    // code: the second entry point `run_detected_tool_command_with_timeout` below
    // is not gated by this early-return and runs `locate_default_tool` →
    // `resolve_path_default` + `build_tool_search_paths` on the Windows desktop
    // binary. It is wired to the OpenCode model-discovery Web route
    // (`GET /api/config/get-opencode-models` via `model_fetch::get_opencode_models`),
    // so those helpers are live on Windows. Do not delete or stub them.
    #[cfg(target_os = "windows")]
    {
        let _ = (tools, wsl_shell_by_tool);
        return Vec::new();
    }

    #[cfg(not(target_os = "windows"))]
    {
        let requested: Vec<&str> = if let Some(tools) = tools.as_ref() {
            let set: std::collections::HashSet<&str> = tools.iter().map(|s| s.as_str()).collect();
            VALID_TOOLS
                .iter()
                .copied()
                .filter(|tool| set.contains(tool))
                .collect()
        } else {
            VALID_TOOLS.to_vec()
        };

        let mut results = Vec::new();
        for tool in requested {
            let pref = wsl_shell_by_tool.as_ref().and_then(|map| map.get(tool));
            let tool_wsl_shell = pref.and_then(|p| p.wsl_shell.as_deref());
            let tool_wsl_shell_flag = pref.and_then(|p| p.wsl_shell_flag.as_deref());
            results.push(
                get_single_tool_version_impl(tool, tool_wsl_shell, tool_wsl_shell_flag).await,
            );
        }
        results
    }
}

#[cfg(target_os = "windows")]
fn tool_env_type_and_wsl_distro(tool: &str) -> (String, Option<String>) {
    if let Some(distro) = wsl_distro_for_tool(tool) {
        ("wsl".to_string(), Some(distro))
    } else {
        ("windows".to_string(), None)
    }
}

#[cfg(target_os = "macos")]
fn tool_env_type_and_wsl_distro(_tool: &str) -> (String, Option<String>) {
    ("macos".to_string(), None)
}

#[cfg(target_os = "linux")]
fn tool_env_type_and_wsl_distro(_tool: &str) -> (String, Option<String>) {
    ("linux".to_string(), None)
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn tool_env_type_and_wsl_distro(_tool: &str) -> (String, Option<String>) {
    ("unknown".to_string(), None)
}

async fn get_single_tool_version_impl(
    tool: &str,
    wsl_shell: Option<&str>,
    wsl_shell_flag: Option<&str>,
) -> ToolVersion {
    debug_assert!(
        VALID_TOOLS.contains(&tool),
        "unexpected tool name in get_single_tool_version_impl: {tool}"
    );

    let (env_type, wsl_distro) = tool_env_type_and_wsl_distro(tool);
    let client = crate::proxy::http_client::get();

    let (local_version, local_error) = if let Some(distro) = wsl_distro.as_deref() {
        try_get_version_wsl(tool, distro, wsl_shell, wsl_shell_flag)
    } else {
        let direct = try_get_version(tool);
        if direct.0.is_some() {
            direct
        } else {
            scan_cli_version(tool)
        }
    };

    let latest_version = {
        let local = local_version.as_deref();
        match tool {
            "claude" => {
                fetch_npm_latest_for_tool(&client, "@anthropic-ai/claude-code", tool, local).await
            }
            "codex" => fetch_npm_latest_for_tool(&client, "@openai/codex", tool, local).await,
            "gemini" => fetch_npm_latest_for_tool(&client, "@google/gemini-cli", tool, local).await,
            "grok" => fetch_npm_latest_for_tool(&client, "@xai-official/grok", tool, local).await,
            "opencode" => fetch_github_latest_version(&client, "anomalyco/opencode").await,
            _ => None,
        }
    };

    ToolVersion {
        name: tool.to_string(),
        version: local_version,
        latest_version,
        error: local_error,
        env_type,
        wsl_distro,
    }
}

/// 该工具在 npm 上的预发布通道 tag（靠前者优先）。仅当本地版本已**严格领先**
/// `latest` 时才会被补查 —— 让主动在抢先通道的用户（如走 Claude Code 的 `next`）
/// 看到与所在通道对齐的"最新版本"，同时绝不把稳定通道用户暴露给预发布版。
/// 返回空切片表示该工具只看 `latest`、不补查。
///
/// 为何不通用覆盖所有工具：各家预发布 tag 命名互不统一（codex=alpha/beta/native、
/// gemini=nightly/preview），且 codex 的 beta/native 是 `0.1.x` 时间戳式版本、
/// gemini 有误发的 `false` tag —— 这些脏值虽会被 `pick_latest_version` 的版本
/// 比较挡掉，但维护成本与误报风险不值当，故暂只为 Claude Code 启用。
fn npm_prerelease_tags(tool: &str) -> &'static [&'static str] {
    match tool {
        "claude" => &["next"],
        _ => &[],
    }
}

/// 解析 "2.1.156" / "2.1.156-beta.1" → (主版本三段, 预发布段)。无法解析返回 None。
/// 与前端 `src/lib/version.ts` 的 parseVersion 语义对称（跨语言各实现一份）。
/// patch 用 u64 以容纳 codex 的 `0.1.2505172116` 时间戳式版本而不溢出。
fn parse_semver(v: &str) -> Option<([u64; 3], Vec<String>)> {
    // 忽略 `+build` 元数据，再以首个 `-` 切出预发布段。
    let core_and_pre = v.trim().split('+').next().unwrap_or("");
    let (core, pre) = match core_and_pre.split_once('-') {
        Some((c, p)) => (c, Some(p)),
        None => (core_and_pre, None),
    };
    let mut parts = core.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    let patch = parts.next()?.parse::<u64>().ok()?;
    if parts.next().is_some() {
        return None; // 多于三段，非法
    }
    let pre_segments = pre
        .map(|p| p.split('.').map(|s| s.to_string()).collect())
        .unwrap_or_default();
    Some(([major, minor, patch], pre_segments))
}

/// 比较两个版本号（遵循 semver：主版本三段优先；core 相等时有预发布 < 无预发布；
/// 预发布段逐段比 —— 数字段按数值、数字段 < 非数字段、非数字段按 ASCII、前缀相同
/// 则段更多者更大）。任一无法解析返回 None，调用方据此保守处理。
fn compare_semver(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    use std::cmp::Ordering;
    let (ac, ap) = parse_semver(a)?;
    let (bc, bp) = parse_semver(b)?;
    for i in 0..3 {
        match ac[i].cmp(&bc[i]) {
            Ordering::Equal => continue,
            other => return Some(other),
        }
    }
    match (ap.is_empty(), bp.is_empty()) {
        (true, true) => return Some(Ordering::Equal),
        (true, false) => return Some(Ordering::Greater),
        (false, true) => return Some(Ordering::Less),
        (false, false) => {}
    }
    for (x, y) in ap.iter().zip(bp.iter()) {
        let ord = match (x.parse::<u64>(), y.parse::<u64>()) {
            (Ok(xv), Ok(yv)) => xv.cmp(&yv),
            (Ok(_), Err(_)) => Ordering::Less, // 数字段 < 非数字段
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(_), Err(_)) => x.as_str().cmp(y.as_str()),
        };
        if ord != Ordering::Equal {
            return Some(ord);
        }
    }
    Some(ap.len().cmp(&bp.len()))
}

/// 从一次 registry 请求得到的完整 dist-tags 出发，挑选要展示的"最新版本"。
///
/// 规则：默认就是 `latest`；仅当本地版本已**严格领先** `latest`（说明用户主动在
/// 抢先通道）时，才把 `prerelease_tags` 指向的版本纳入比较，取其中能被解析、且
/// 高于 `latest` 的最高者。无法解析或不高于 latest 的脏 tag 一律落选。
fn pick_latest_version(
    dist_tags: &serde_json::Map<String, serde_json::Value>,
    prerelease_tags: &[&str],
    local_version: Option<&str>,
) -> Option<String> {
    use std::cmp::Ordering;
    let latest = dist_tags.get("latest").and_then(|v| v.as_str())?;

    // 本地是否严格领先 latest；任一无法解析则按"未领先"保守处理（只看 latest）。
    let local_ahead = local_version
        .and_then(|local| compare_semver(local, latest))
        .map(|ord| ord == Ordering::Greater)
        .unwrap_or(false);
    if prerelease_tags.is_empty() || !local_ahead {
        return Some(latest.to_string());
    }

    let mut best = latest.to_string();
    for tag in prerelease_tags {
        if let Some(candidate) = dist_tags.get(*tag).and_then(|v| v.as_str()) {
            if compare_semver(candidate, &best) == Some(Ordering::Greater) {
                best = candidate.to_string();
            }
        }
    }
    Some(best)
}

/// 拉取 npm 包的完整 dist-tags（单次请求即含 latest/next/beta/...）。
async fn fetch_npm_dist_tags(
    client: &reqwest::Client,
    package: &str,
) -> Option<serde_json::Map<String, serde_json::Value>> {
    let url = format!(
        "{}/{}",
        read_base_url_env(
            "CC_SWITCH_NPM_REGISTRY_BASE_URL",
            "https://registry.npmjs.org"
        ),
        package.trim_start_matches('/')
    );
    let resp = client.get(&url).send().await.ok()?;
    let json = resp.json::<serde_json::Value>().await.ok()?;
    json.get("dist-tags")?.as_object().cloned()
}

/// 查询某 npm 工具要展示的"最新版本"：取 `latest`，并在本地版本领先时按工具的
/// 预发布通道（见 `npm_prerelease_tags`）补查 —— 复用同一次 registry 响应，无额外请求。
async fn fetch_npm_latest_for_tool(
    client: &reqwest::Client,
    package: &str,
    tool: &str,
    local_version: Option<&str>,
) -> Option<String> {
    let dist_tags = fetch_npm_dist_tags(client, package).await?;
    pick_latest_version(&dist_tags, npm_prerelease_tags(tool), local_version)
}

async fn fetch_github_latest_version(client: &reqwest::Client, repo: &str) -> Option<String> {
    let url = format!(
        "{}/repos/{repo}/releases/latest",
        read_base_url_env("CC_SWITCH_GITHUB_API_BASE_URL", "https://api.github.com")
    );
    match client
        .get(&url)
        .header("User-Agent", "cc-switch")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
    {
        Ok(resp) => match resp.json::<serde_json::Value>().await {
            Ok(json) => json
                .get("tag_name")
                .and_then(|v| v.as_str())
                .map(|s| s.strip_prefix('v').unwrap_or(s).to_string()),
            Err(_) => None,
        },
        Err(_) => None,
    }
}

fn read_base_url_env(var_name: &str, default: &str) -> String {
    std::env::var(var_name)
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_string())
}

static VERSION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\d+\.\d+\.\d+(-[\w.]+)?").expect("Invalid version regex"));

fn extract_version(raw: &str) -> String {
    VERSION_RE
        .find(raw)
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| raw.to_string())
}

/// 解码子进程输出：Windows 上 cmd 的报错走 OEM/ANSI 代码页（如 zh-CN 的 GBK），
/// 直接按 UTF-8 lossy 解码会变成乱码；其余平台保持 UTF-8 lossy。
pub(crate) fn decode_command_output(bytes: &[u8]) -> String {
    #[cfg(target_os = "windows")]
    {
        decode_windows_command_output(bytes)
    }

    #[cfg(not(target_os = "windows"))]
    {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

#[cfg(target_os = "windows")]
fn decode_windows_command_output(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }

    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_string();
    }

    use windows_sys::Win32::Globalization::{GetACP, GetOEMCP, MultiByteToWideChar};

    fn decode_codepage(bytes: &[u8], codepage: u32) -> Option<String> {
        if codepage == 0 {
            return None;
        }

        let input_len = i32::try_from(bytes.len()).ok()?;
        unsafe {
            let wide_len = MultiByteToWideChar(
                codepage,
                0,
                bytes.as_ptr(),
                input_len,
                std::ptr::null_mut(),
                0,
            );
            if wide_len <= 0 {
                return None;
            }

            let mut wide = vec![0u16; wide_len as usize];
            let written = MultiByteToWideChar(
                codepage,
                0,
                bytes.as_ptr(),
                input_len,
                wide.as_mut_ptr(),
                wide_len,
            );
            if written <= 0 {
                return None;
            }

            Some(String::from_utf16_lossy(&wide[..written as usize]))
        }
    }

    let oem_cp = unsafe { GetOEMCP() };
    if let Some(decoded) = decode_codepage(bytes, oem_cp) {
        return decoded;
    }

    let ansi_cp = unsafe { GetACP() };
    if ansi_cp != oem_cp {
        if let Some(decoded) = decode_codepage(bytes, ansi_cp) {
            return decoded;
        }
    }

    String::from_utf8_lossy(bytes).into_owned()
}

fn try_get_version(tool: &str) -> (Option<String>, Option<String>) {
    use std::process::Command;

    #[cfg(target_os = "windows")]
    let output = Command::new("cmd")
        .args(["/C", &format!("{tool} --version")])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    #[cfg(not(target_os = "windows"))]
    let output = {
        let shell = std::env::var("SHELL")
            .ok()
            .filter(|s| is_valid_shell(s))
            .unwrap_or_else(|| "sh".to_string());
        let flag = default_flag_for_shell(&shell);
        Command::new(shell)
            .arg(flag)
            .arg(format!("{tool} --version"))
            .output()
    };

    match output {
        Ok(out) => {
            let stdout = decode_command_output(&out.stdout).trim().to_string();
            let stderr = decode_command_output(&out.stderr).trim().to_string();
            if out.status.success() {
                let raw = if stdout.is_empty() { &stderr } else { &stdout };
                if raw.is_empty() {
                    (None, Some("not installed or not executable".to_string()))
                } else {
                    (Some(extract_version(raw)), None)
                }
            } else {
                let err = if stderr.is_empty() { stdout } else { stderr };
                (
                    None,
                    Some(if err.is_empty() {
                        "not installed or not executable".to_string()
                    } else {
                        err
                    }),
                )
            }
        }
        Err(err) => (None, Some(err.to_string())),
    }
}

#[cfg(target_os = "windows")]
fn is_valid_wsl_distro_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

fn is_valid_shell(shell: &str) -> bool {
    matches!(
        shell.rsplit('/').next().unwrap_or(shell),
        "sh" | "bash" | "zsh" | "fish" | "dash"
    )
}

#[cfg(target_os = "windows")]
fn is_valid_shell_flag(flag: &str) -> bool {
    matches!(flag, "-c" | "-lc" | "-lic")
}

fn default_flag_for_shell(shell: &str) -> &'static str {
    match shell.rsplit('/').next().unwrap_or(shell) {
        "dash" | "sh" => "-c",
        "fish" => "-lc",
        _ => "-lic",
    }
}

#[cfg(target_os = "windows")]
fn try_get_version_wsl(
    tool: &str,
    distro: &str,
    force_shell: Option<&str>,
    force_shell_flag: Option<&str>,
) -> (Option<String>, Option<String>) {
    use std::process::Command;

    debug_assert!(
        ["claude", "codex", "gemini", "opencode"].contains(&tool),
        "unexpected tool name: {tool}"
    );

    if !is_valid_wsl_distro_name(distro) {
        return (None, Some(format!("[WSL:{distro}] invalid distro name")));
    }

    let (shell, flag, cmd) = if let Some(shell) = force_shell {
        if !is_valid_shell(shell) {
            return (None, Some(format!("[WSL:{distro}] invalid shell: {shell}")));
        }
        let shell = shell.rsplit('/').next().unwrap_or(shell);
        let flag = if let Some(flag) = force_shell_flag {
            if !is_valid_shell_flag(flag) {
                return (
                    None,
                    Some(format!("[WSL:{distro}] invalid shell flag: {flag}")),
                );
            }
            flag
        } else {
            default_flag_for_shell(shell)
        };

        (shell.to_string(), flag, format!("{tool} --version"))
    } else {
        let cmd = if let Some(flag) = force_shell_flag {
            if !is_valid_shell_flag(flag) {
                return (
                    None,
                    Some(format!("[WSL:{distro}] invalid shell flag: {flag}")),
                );
            }
            format!("\"${{SHELL:-sh}}\" {flag} '{tool} --version'")
        } else {
            format!(
                "\"${{SHELL:-sh}}\" -lic '{tool} --version' 2>/dev/null || \"${{SHELL:-sh}}\" -lc '{tool} --version' 2>/dev/null || \"${{SHELL:-sh}}\" -c '{tool} --version'"
            )
        };

        ("sh".to_string(), "-c", cmd)
    };

    let output = Command::new("wsl.exe")
        .args(["-d", distro, "--", &shell, flag, &cmd])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    match output {
        Ok(out) => {
            let stdout = decode_command_output(&out.stdout).trim().to_string();
            let stderr = decode_command_output(&out.stderr).trim().to_string();
            if out.status.success() {
                let raw = if stdout.is_empty() { &stderr } else { &stdout };
                if raw.is_empty() {
                    (
                        None,
                        Some(format!("[WSL:{distro}] not installed or not executable")),
                    )
                } else {
                    (Some(extract_version(raw)), None)
                }
            } else {
                let err = if stderr.is_empty() { stdout } else { stderr };
                (
                    None,
                    Some(format!(
                        "[WSL:{distro}] {}",
                        if err.is_empty() {
                            "not installed or not executable".to_string()
                        } else {
                            err
                        }
                    )),
                )
            }
        }
        Err(err) => (None, Some(format!("[WSL:{distro}] exec failed: {err}"))),
    }
}

#[cfg(not(target_os = "windows"))]
fn try_get_version_wsl(
    _tool: &str,
    _distro: &str,
    _force_shell: Option<&str>,
    _force_shell_flag: Option<&str>,
) -> (Option<String>, Option<String>) {
    (
        None,
        Some("WSL check not supported on this platform".to_string()),
    )
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !path.as_os_str().is_empty() && !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

fn push_env_single_dir(paths: &mut Vec<PathBuf>, value: Option<std::ffi::OsString>) {
    if let Some(raw) = value {
        push_unique_path(paths, PathBuf::from(raw));
    }
}

fn extend_from_path_list(
    paths: &mut Vec<PathBuf>,
    value: Option<std::ffi::OsString>,
    suffix: Option<&str>,
) {
    if let Some(raw) = value {
        for path in std::env::split_paths(&raw) {
            let dir = match suffix {
                Some(suffix) => path.join(suffix),
                None => path,
            };
            push_unique_path(paths, dir);
        }
    }
}

fn extend_mise_node_search_paths(paths: &mut Vec<PathBuf>, home: &Path) {
    if home.as_os_str().is_empty() {
        return;
    }

    let mise_base = home.join(".local/share/mise");
    push_unique_path(paths, mise_base.join("shims"));

    let node_installs = mise_base.join("installs").join("node");
    if node_installs.exists() {
        if let Ok(entries) = std::fs::read_dir(&node_installs) {
            for entry in entries.flatten() {
                let bin_path = entry.path().join("bin");
                if bin_path.exists() {
                    push_unique_path(paths, bin_path);
                }
            }
        }
    }
}

/// The "effective PATH" used during detection. On Windows the inherited
/// process PATH can be incomplete — most notably after an in-app self-update,
/// where the MSI/WiX-auto-launched process inherits only the machine-level PATH
/// and drops the user-level PATH (see upstream #6061). Any CLI installed in a
/// user-PATH location (winget Claude `%LOCALAPPDATA%\Programs\claude`, the
/// standalone Codex installer `%LOCALAPPDATA%\Programs\OpenAI\Codex\bin`, a
/// custom npm prefix `D:\npm-global`, …) then reads as "not installed".
///
/// Reconstruct the effective PATH by merging the process PATH with the machine
/// and user registry PATH values (`REG_EXPAND_SZ` expanded), so detection sees
/// the same installations a freshly logged-in shell would — regardless of how
/// the current process was launched. Process entries are kept first (a runtime
/// override wins); registry entries fill whatever the process is missing;
/// duplicates are removed.
///
/// See `env_checker::check_system_env` for the same set of registry keys; here
/// we read only the `Path` value. Port of upstream `de9af49a`.
#[cfg(target_os = "windows")]
fn effective_path_string() -> String {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    let process = std::env::var("PATH").unwrap_or_default();
    let user = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("Environment")
        .and_then(|k| k.get_value::<String, &str>("Path"))
        .map(|raw| expand_env_chars(&raw))
        .unwrap_or_default();
    let machine = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey("SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment")
        .and_then(|k| k.get_value::<String, &str>("Path"))
        .map(|raw| expand_env_chars(&raw))
        .unwrap_or_default();
    merge_path_segments_win(&[&process, &user, &machine])
}

/// On Windows the effective PATH comes from `effective_path_string`; on other
/// platforms the raw process value is returned unchanged (zero behaviour change).
#[cfg(target_os = "windows")]
fn effective_path_os() -> Option<std::ffi::OsString> {
    Some(std::ffi::OsString::from(effective_path_string()))
}

#[cfg(not(target_os = "windows"))]
fn effective_path_os() -> Option<std::ffi::OsString> {
    std::env::var_os("PATH")
}

/// Prepend a candidate directory without converting the existing PATH to
/// UTF-8. Unix permits arbitrary non-NUL bytes in environment values; keeping
/// this as an `OsString` ensures one non-Unicode segment cannot discard or
/// corrupt every other interpreter directory needed by an npm/python shim.
#[cfg(not(target_os = "windows"))]
fn prepend_search_dir_to_path(dir: &Path, current_path: &std::ffi::OsStr) -> std::ffi::OsString {
    let mut path = dir.as_os_str().to_os_string();
    if !current_path.is_empty() {
        path.push(":");
        path.push(current_path);
    }
    path
}

/// Expand `%VAR%` environment-variable references. The registry `Path` value is
/// `REG_EXPAND_SZ`, and the `String` returned by `winreg` is not auto-expanded.
/// Variables such as `%LOCALAPPDATA%` / `%USERPROFILE%` / `%SystemRoot%` are
/// still defined in a process that lost its user PATH (Winlogon injects them
/// from the user profile), so expanding each via `std::env::var` is safe.
/// Undefined variables are preserved verbatim (no characters dropped).
#[cfg(target_os = "windows")]
fn expand_env_chars(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(open) = rest.find('%') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        match after.find('%') {
            None => {
                out.push('%');
                out.push_str(after);
                rest = "";
                break;
            }
            Some(close) => {
                let name = &after[..close];
                let is_ident =
                    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
                if is_ident {
                    match std::env::var(name) {
                        Ok(val) => out.push_str(&val),
                        Err(_) => {
                            out.push('%');
                            out.push_str(name);
                            out.push('%');
                        }
                    }
                } else {
                    out.push('%');
                    out.push_str(name);
                    out.push('%');
                }
                rest = &after[close + 1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Merge several Windows PATH strings (`;`-separated) in order, keeping only
/// the first occurrence of each segment (case-insensitive). Process segments
/// come first to respect runtime overrides, followed by user- and
/// machine-level registry segments.
#[cfg(target_os = "windows")]
fn merge_path_segments_win(parts: &[&str]) -> String {
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut merged: Vec<&str> = Vec::new();
    for part in parts {
        for seg in part.split(';') {
            let s = seg.trim();
            if s.is_empty() || !seen.insert(s.to_ascii_lowercase()) {
                continue;
            }
            merged.push(s);
        }
    }
    merged.join(";")
}

/// Convert a canonicalized Windows path back to the form accepted by shell
/// commands. `std::fs::canonicalize` prefixes local paths with `\\?\` (and UNC
/// paths with `\\?\UNC\`), but `cmd.exe` cannot `call` a batch file through
/// those verbatim paths and reports "The system cannot find the path
/// specified." Direct Win32 executable launches accept the prefix; batch
/// scripts do not.
#[cfg(target_os = "windows")]
pub(crate) fn windows_shell_compatible_path(path: &Path) -> std::path::PathBuf {
    let raw = path.to_string_lossy();
    if let Some(unc) = raw.strip_prefix(r"\\?\UNC\") {
        std::path::PathBuf::from(format!(r"\\{unc}"))
    } else if let Some(local) = raw.strip_prefix(r"\\?\") {
        std::path::PathBuf::from(local)
    } else {
        path.to_path_buf()
    }
}

/// Whether a path lives under the Windows App Execution Alias reparse-point
/// directory (`%LOCALAPPDATA%\Microsoft\WindowsApps`). Such entries launch the
/// Store / a protocol handler and must not be treated as a PATH default CLI.
#[cfg(target_os = "windows")]
fn is_windows_app_execution_alias_dir(parent: &Path) -> bool {
    parent
        .to_string_lossy()
        .to_ascii_lowercase()
        .contains(r"microsoft\windowsapps")
}

fn opencode_extra_search_paths(
    home: &Path,
    opencode_install_dir: Option<std::ffi::OsString>,
    xdg_bin_dir: Option<std::ffi::OsString>,
    gopath: Option<std::ffi::OsString>,
) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    push_env_single_dir(&mut paths, opencode_install_dir);
    push_env_single_dir(&mut paths, xdg_bin_dir);

    if !home.as_os_str().is_empty() {
        push_unique_path(&mut paths, home.join("bin"));
        push_unique_path(&mut paths, home.join(".opencode").join("bin"));
        push_unique_path(&mut paths, home.join(".bun").join("bin"));
        push_unique_path(&mut paths, home.join("go").join("bin"));
    }

    extend_from_path_list(&mut paths, gopath, Some("bin"));
    paths
}

fn tool_executable_candidates(tool: &str, dir: &Path) -> Vec<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        vec![
            dir.join(format!("{tool}.cmd")),
            dir.join(format!("{tool}.exe")),
            dir.join(tool),
        ]
    }

    #[cfg(not(target_os = "windows"))]
    {
        vec![dir.join(tool)]
    }
}

fn build_tool_search_paths(tool: &str) -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_default();
    let mut search_paths = Vec::new();

    if !home.as_os_str().is_empty() {
        push_unique_path(&mut search_paths, home.join(".local/bin"));
        push_unique_path(&mut search_paths, home.join(".npm-global/bin"));
        push_unique_path(&mut search_paths, home.join("n/bin"));
        push_unique_path(&mut search_paths, home.join(".volta/bin"));
        extend_mise_node_search_paths(&mut search_paths, &home);
    }

    #[cfg(target_os = "macos")]
    {
        push_unique_path(&mut search_paths, PathBuf::from("/opt/homebrew/bin"));
        push_unique_path(&mut search_paths, PathBuf::from("/usr/local/bin"));
    }

    #[cfg(target_os = "linux")]
    {
        push_unique_path(&mut search_paths, PathBuf::from("/usr/local/bin"));
        push_unique_path(&mut search_paths, PathBuf::from("/usr/bin"));
    }

    #[cfg(target_os = "windows")]
    {
        // Official standalone (non-npm) installer locations — belt-and-suspenders
        // alongside the registry-PATH merge in `effective_path_os`. These
        // installers normally register themselves on the user PATH, but some
        // per-user MSI/MSIX installs do not, and an in-app-update relaunch can
        // drop the user PATH (#6061), so add them explicitly here. Placed ahead
        // of the npm directory so a native install wins over a stale npm shim
        // (#4701). Port of upstream `de9af49a`.
        if let Some(local_data) = dirs::data_local_dir() {
            if tool == "codex" {
                // OpenAI Codex Installer.exe / .msi standalone install location.
                push_unique_path(
                    &mut search_paths,
                    local_data
                        .join("Programs")
                        .join("OpenAI")
                        .join("Codex")
                        .join("bin"),
                );
            }
            if tool == "claude" {
                // `winget install Anthropic.ClaudeCode` / official native
                // installer location.
                push_unique_path(
                    &mut search_paths,
                    local_data.join("Programs").join("claude"),
                );
            }
        }
        if let Some(appdata) = dirs::data_dir() {
            push_unique_path(&mut search_paths, appdata.join("npm"));
        }
        push_unique_path(
            &mut search_paths,
            PathBuf::from("C:\\Program Files\\nodejs"),
        );
    }

    let fnm_base = home.join(".local/state/fnm_multishells");
    if fnm_base.exists() {
        if let Ok(entries) = std::fs::read_dir(&fnm_base) {
            for entry in entries.flatten() {
                let bin_path = entry.path().join("bin");
                if bin_path.exists() {
                    push_unique_path(&mut search_paths, bin_path);
                }
            }
        }
    }

    let nvm_base = home.join(".nvm/versions/node");
    if nvm_base.exists() {
        if let Ok(entries) = std::fs::read_dir(&nvm_base) {
            for entry in entries.flatten() {
                let bin_path = entry.path().join("bin");
                if bin_path.exists() {
                    push_unique_path(&mut search_paths, bin_path);
                }
            }
        }
    }

    if tool == "opencode" {
        for path in opencode_extra_search_paths(
            &home,
            std::env::var_os("OPENCODE_INSTALL_DIR"),
            std::env::var_os("XDG_BIN_DIR"),
            std::env::var_os("GOPATH"),
        ) {
            push_unique_path(&mut search_paths, path);
        }
    }

    if let Some(path_env) = effective_path_os() {
        for path in std::env::split_paths(&path_env) {
            push_unique_path(&mut search_paths, path);
        }
    }

    search_paths
}

/// Windows 双引号包裹基础原语：无条件加引号 + 内部 `"` 转义为 `\"`。
#[cfg(target_os = "windows")]
fn win_double_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\\\""))
}

#[cfg(target_os = "windows")]
fn windows_cmd_double_quote_arg(value: &str) -> String {
    win_double_quote(value)
}

/// 给 batch/`call` 用的路径引用：`%` 经历 batch parser + `call` 两轮 expansion，
/// 要让 call 最终看到字面 `%` 需要 4 个 → `%%%%`。`needs_quote` 基于原路径判断。
#[cfg(target_os = "windows")]
fn win_quote_path_for_batch(p: &str) -> String {
    let escaped = if p.contains('%') {
        p.replace('%', "%%%%")
    } else {
        p.to_string()
    };
    let needs_quote = p
        .chars()
        .any(|c| matches!(c, ' ' | '&' | '(' | ')' | '^' | ';' | '<' | '>' | '|' | ','));
    if needs_quote {
        win_double_quote(&escaped)
    } else {
        escaped
    }
}

#[cfg(target_os = "windows")]
fn is_windows_command_script(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("cmd") || ext.eq_ignore_ascii_case("bat"))
        .unwrap_or(false)
}

/// Windows 版本探测：.exe 直接执行（绕开 cmd 的嵌套引号误解析）；.cmd/.bat 走
/// `cmd /D /S /C call <quoted> --version`，用 raw_arg 绕过 Rust 的参数引用，
/// 保证 cmd 看到确定的引号形态。
#[cfg(target_os = "windows")]
fn run_windows_tool_command(
    tool_path: &Path,
    args: &[&str],
    new_path: &str,
) -> std::io::Result<std::process::Output> {
    use std::process::Command;

    if is_windows_command_script(tool_path) {
        // `resolve_path_default` returns a canonical path so callers can
        // compare installation identities. Canonical Windows paths carry a
        // `\\?\` prefix, which `cmd /C call` rejects for batch files. Normalize
        // only at this shell boundary and keep the canonical identity intact
        // everywhere else. Port of upstream `de9af49a`.
        let shell_path = windows_shell_compatible_path(tool_path);
        let path = shell_path.to_string_lossy();
        let args = args
            .iter()
            .map(|arg| windows_cmd_double_quote_arg(arg))
            .collect::<Vec<_>>()
            .join(" ");
        let command = format!(
            "call {}{}",
            win_quote_path_for_batch(&path),
            if args.is_empty() {
                String::new()
            } else {
                format!(" {args}")
            }
        );
        let mut cmd = Command::new("cmd");
        return cmd
            .args(["/D", "/S", "/C"])
            .raw_arg(&command)
            .env("PATH", new_path)
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    }

    Command::new(tool_path)
        .args(args)
        .env("PATH", new_path)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
}

#[cfg(target_os = "windows")]
fn run_windows_tool_version_command(
    tool_path: &Path,
    new_path: &str,
) -> std::io::Result<std::process::Output> {
    run_windows_tool_command(tool_path, &["--version"], new_path)
}

fn scan_cli_version(tool: &str) -> (Option<String>, Option<String>) {
    #[cfg(not(target_os = "windows"))]
    use std::process::Command;

    let search_paths = build_tool_search_paths(tool);

    #[cfg(target_os = "windows")]
    let current_path = effective_path_string();
    #[cfg(not(target_os = "windows"))]
    let current_path = effective_path_os().unwrap_or_default();

    for path in &search_paths {
        #[cfg(target_os = "windows")]
        let new_path = format!("{};{}", path.display(), current_path);

        #[cfg(not(target_os = "windows"))]
        let new_path = prepend_search_dir_to_path(path, &current_path);

        for tool_path in tool_executable_candidates(tool, path) {
            if !tool_path.exists() {
                continue;
            }

            #[cfg(target_os = "windows")]
            let output = run_windows_tool_version_command(&tool_path, &new_path);

            #[cfg(not(target_os = "windows"))]
            let output = Command::new(&tool_path)
                .arg("--version")
                .env("PATH", &new_path)
                .output();

            if let Ok(out) = output {
                let stdout = decode_command_output(&out.stdout).trim().to_string();
                let stderr = decode_command_output(&out.stderr).trim().to_string();
                if out.status.success() {
                    let raw = if stdout.is_empty() { &stderr } else { &stdout };
                    if !raw.is_empty() {
                        return (Some(extract_version(raw)), None);
                    }
                }
            }
        }
    }

    (None, Some("not installed or not executable".to_string()))
}

#[derive(Clone, Copy)]
struct CommandDeadline {
    expires_at: std::time::Instant,
    limit: std::time::Duration,
}

impl CommandDeadline {
    fn new(limit: std::time::Duration) -> Self {
        Self {
            expires_at: std::time::Instant::now() + limit,
            limit,
        }
    }

    fn remaining(self) -> Result<std::time::Duration, String> {
        self.expires_at
            .checked_duration_since(std::time::Instant::now())
            .filter(|remaining| !remaining.is_zero())
            .ok_or_else(|| self.timeout_error())
    }

    fn timeout_error(self) -> String {
        format!("Command timed out after {}s", self.limit.as_secs())
    }
}

#[cfg(not(target_os = "windows"))]
fn first_abs_path_line(raw: &str) -> Option<&str> {
    raw.lines()
        .map(str::trim)
        .find(|line| line.starts_with('/'))
}

#[cfg(not(target_os = "windows"))]
fn resolve_path_default(tool: &str, deadline: CommandDeadline) -> Result<Option<PathBuf>, String> {
    use std::process::{Command, Stdio};

    let shell = std::env::var("SHELL")
        .ok()
        .filter(|shell| is_valid_shell(shell))
        .unwrap_or_else(|| "sh".to_string());
    let flag = default_flag_for_shell(&shell);
    let mut cmd = Command::new(shell);
    cmd.arg(flag)
        .arg(format!("command -v {tool}"))
        // spawn 后 stdin 不再像 output() 那样默认置 null，须显式关闭：
        // 继承来的 stdin 可能是终端/管道，交互式 rc 里的读操作会永久阻塞。
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    isolate_child_process_group(&mut cmd);
    let child = cmd
        .spawn()
        .map_err(|error| format!("Failed to locate {tool}: {error}"))?;
    let output = wait_child_output(child, deadline)?;
    if !output.status.success() {
        return Ok(None);
    }

    let raw = decode_command_output(&output.stdout);
    let Some(first) = first_abs_path_line(&raw) else {
        return Ok(None);
    };
    Ok(std::fs::canonicalize(first).ok())
}

#[cfg(target_os = "windows")]
fn windows_runnable_sibling_for_extensionless_tool(path: &Path) -> Option<PathBuf> {
    if path.extension().is_some() {
        return None;
    }

    ["cmd", "exe"]
        .iter()
        .map(|extension| path.with_extension(extension))
        .find(|candidate| candidate.is_file())
}

#[cfg(target_os = "windows")]
fn windows_path_lookup_command(
    tool: &str,
    effective_path: &std::ffi::OsStr,
) -> std::process::Command {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    // Use the system copy explicitly so a project-local `where.exe` cannot
    // hijack the passive lookup before the PATH-only pattern is evaluated.
    let where_exe = PathBuf::from(
        std::env::var_os("SystemRoot").unwrap_or_else(|| std::ffi::OsString::from(r"C:\\Windows")),
    )
    .join("System32")
    .join("where.exe");
    let mut command = Command::new(where_exe);
    command
        // `$PATH:pattern` is where.exe's documented environment-variable
        // search form. Unlike a bare pattern, it does not search the current
        // directory before PATH.
        .arg(format!("$PATH:{tool}"))
        .env("PATH", effective_path)
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

#[cfg(target_os = "windows")]
fn resolve_path_default(tool: &str, deadline: CommandDeadline) -> Result<Option<PathBuf>, String> {
    use std::process::Stdio;

    // Restrict `where` to the merged effective PATH. A bare `where {tool}` also
    // searches the current directory first, which would let a project-local
    // `codex.cmd` be executed by a passive version check. The `$PATH:pattern`
    // form searches only the supplied environment variable while still seeing
    // registry PATH entries lost by an in-app-update relaunch (#6061).
    let current_path = effective_path_os().unwrap_or_default();
    let child = windows_path_lookup_command(tool, &current_path)
        .spawn()
        .map_err(|error| format!("Failed to locate {tool}: {error}"))?;
    let output = wait_child_output(child, deadline)?;
    if !output.status.success() {
        return Ok(None);
    }

    let raw = decode_command_output(&output.stdout);
    // `where` lists every match on PATH in order; the first is what the user
    // actually runs. Skip App Execution Aliases (reparse points under
    // `Microsoft\WindowsApps`) — they launch the Store / a protocol handler,
    // are not CLIs we can `--version`-probe, and must not be treated as the
    // PATH default. Take the first remaining real entry.
    let resolved = raw.lines().map(str::trim).find(|line| {
        !line.is_empty()
            && !is_windows_app_execution_alias_dir(
                Path::new(line).parent().unwrap_or(Path::new("")),
            )
    });
    let Some(first) = resolved else {
        return Ok(None);
    };
    let path = Path::new(first);
    let preferred =
        windows_runnable_sibling_for_extensionless_tool(path).unwrap_or_else(|| path.to_path_buf());
    Ok(std::fs::canonicalize(preferred).ok())
}

fn locate_default_tool(tool: &str, deadline: CommandDeadline) -> Result<PathBuf, String> {
    let path_default = resolve_path_default(tool, deadline)?;
    let mut seen = std::collections::HashSet::new();
    let mut candidates = Vec::new();

    for dir in build_tool_search_paths(tool) {
        for candidate in tool_executable_candidates(tool, &dir) {
            if !candidate.is_file() {
                continue;
            }
            let real = std::fs::canonicalize(&candidate).unwrap_or_else(|_| candidate.clone());
            if path_default.as_ref() == Some(&real) {
                return Ok(candidate);
            }
            if seen.insert(real) {
                candidates.push(candidate);
            }
        }
    }

    if let Some(path) = path_default {
        return Ok(path);
    }

    match candidates.as_slice() {
        [only] => Ok(only.clone()),
        [] => Err(format!("{tool} is not installed")),
        _ => Err(format!(
            "{tool} is installed but its default installation is ambiguous"
        )),
    }
}

#[cfg(target_os = "windows")]
fn terminate_child_tree(child: &mut std::process::Child) -> bool {
    use std::process::{Command, Stdio};

    let status = Command::new("taskkill")
        .args(["/PID", &child.id().to_string(), "/T", "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    matches!(status, Ok(status) if status.success()) || child.kill().is_ok()
}

#[cfg(not(target_os = "windows"))]
fn terminate_child_tree(child: &mut std::process::Child) -> bool {
    let process_group = -(child.id() as libc::pid_t);
    // SAFETY: bounded runtime commands are placed in a dedicated process group before spawn.
    (unsafe { libc::kill(process_group, libc::SIGKILL) == 0 }) || child.kill().is_ok()
}

#[cfg(not(target_os = "windows"))]
fn isolate_child_process_group(command: &mut std::process::Command) {
    use std::os::unix::process::CommandExt;

    // setsid 而非 process_group(0)：新会话自带新进程组（组长=自身，
    // terminate_child_tree 的 kill(-pid) 整组击杀语义不变），并额外**脱离控制终端**。
    // 只隔离进程组时，探测用的交互式 shell（zsh -lic）若还持有控制终端（如 dev 模式
    // 从终端启动），其作业控制会因处于背景进程组被 SIGTTIN/SIGTTOU 停住，`wait()`
    // 永远等不到退出；脱离终端后 shell 拿不到 /dev/tty，作业控制自动关闭。
    // SAFETY: setsid 是 async-signal-safe；fork 出的子进程继承父进程组、必不是组长，
    // 调用不会因 EPERM 失败。
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

fn read_bounded_pipe<R: std::io::Read>(
    mut pipe: R,
    overflow: &std::sync::atomic::AtomicBool,
) -> Vec<u8> {
    use std::sync::atomic::Ordering;

    let mut output = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let count = match pipe.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(count) => count,
        };
        let remaining = TOOL_COMMAND_OUTPUT_LIMIT.saturating_sub(output.len());
        output.extend_from_slice(&chunk[..count.min(remaining)]);
        if count > remaining {
            overflow.store(true, Ordering::Release);
            break;
        }
    }
    output
}

fn spawn_bounded_reader<R: std::io::Read + Send + 'static>(
    pipe: Option<R>,
    overflow: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Option<std::thread::JoinHandle<Vec<u8>>> {
    pipe.map(|pipe| std::thread::spawn(move || read_bounded_pipe(pipe, &overflow)))
}

fn wait_child_output(
    mut child: std::process::Child,
    deadline: CommandDeadline,
) -> Result<std::process::Output, String> {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let overflow = Arc::new(AtomicBool::new(false));
    let stdout_handle = spawn_bounded_reader(child.stdout.take(), Arc::clone(&overflow));
    let stderr_handle = spawn_bounded_reader(child.stderr.take(), Arc::clone(&overflow));

    let status = loop {
        if overflow.load(Ordering::Acquire) {
            if terminate_child_tree(&mut child) {
                let _ = child.wait();
            }
            drop(stdout_handle);
            drop(stderr_handle);
            return Err(format!(
                "Command output exceeded {} MiB",
                TOOL_COMMAND_OUTPUT_LIMIT / (1024 * 1024)
            ));
        }

        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                let remaining = match deadline.remaining() {
                    Ok(remaining) => remaining,
                    Err(error) => {
                        if terminate_child_tree(&mut child) {
                            let _ = child.wait();
                        }
                        drop(stdout_handle);
                        drop(stderr_handle);
                        return Err(error);
                    }
                };
                std::thread::sleep(std::cmp::min(
                    std::time::Duration::from_millis(50),
                    remaining,
                ));
            }
            Err(error) => {
                if terminate_child_tree(&mut child) {
                    let _ = child.wait();
                }
                return Err(format!("Failed to wait for command: {error}"));
            }
        }
    };

    while stdout_handle
        .as_ref()
        .is_some_and(|handle| !handle.is_finished())
        || stderr_handle
            .as_ref()
            .is_some_and(|handle| !handle.is_finished())
    {
        if overflow.load(Ordering::Acquire) {
            let _ = terminate_child_tree(&mut child);
            drop(stdout_handle);
            drop(stderr_handle);
            return Err(format!(
                "Command output exceeded {} MiB",
                TOOL_COMMAND_OUTPUT_LIMIT / (1024 * 1024)
            ));
        }
        let remaining = match deadline.remaining() {
            Ok(remaining) => remaining,
            Err(error) => {
                let _ = terminate_child_tree(&mut child);
                drop(stdout_handle);
                drop(stderr_handle);
                return Err(error);
            }
        };
        std::thread::sleep(std::cmp::min(
            std::time::Duration::from_millis(50),
            remaining,
        ));
    }

    if overflow.load(Ordering::Acquire) {
        return Err(format!(
            "Command output exceeded {} MiB",
            TOOL_COMMAND_OUTPUT_LIMIT / (1024 * 1024)
        ));
    }

    let stdout = stdout_handle
        .map(|handle| handle.join().unwrap_or_default())
        .unwrap_or_default();
    let stderr = stderr_handle
        .map(|handle| handle.join().unwrap_or_default())
        .unwrap_or_default();

    Ok(std::process::Output {
        status,
        stdout,
        stderr,
    })
}

fn apply_extra_env(command: &mut std::process::Command, extra_env: &[(&str, String)]) {
    for (key, value) in extra_env {
        command.env(key, value);
    }
}

pub(crate) fn run_detected_tool_command_with_timeout(
    tool: &str,
    args: &[&str],
    timeout: std::time::Duration,
    extra_env: &[(&str, String)],
    working_dir: &Path,
) -> Result<std::process::Output, String> {
    if !VALID_TOOLS.contains(&tool) {
        return Err(format!("Unsupported tool: {tool}"));
    }
    if args.iter().any(|arg| {
        arg.is_empty()
            || !arg.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
            })
    }) {
        return Err("Invalid tool command arguments".to_string());
    }
    if !working_dir.is_dir() {
        return Err(format!(
            "Tool working directory does not exist: {}",
            working_dir.display()
        ));
    }

    let deadline = CommandDeadline::new(timeout);

    #[cfg(target_os = "windows")]
    if let Some(distro) = wsl_distro_for_tool(tool) {
        return run_wsl_tool_command(tool, args, &distro, deadline, extra_env, working_dir);
    }

    let tool_path = locate_default_tool(tool, deadline)?;
    let directory = tool_path
        .parent()
        .ok_or_else(|| format!("Invalid {tool} executable path"))?;
    let current_path = std::env::var_os("PATH")
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();

    #[cfg(target_os = "windows")]
    {
        run_windows_tool_command_capture(
            &tool_path,
            args,
            &format!("{};{current_path}", directory.display()),
            deadline,
            extra_env,
            working_dir,
        )
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::process::{Command, Stdio};

        let mut command = Command::new(&tool_path);
        command
            .args(args)
            .env("PATH", format!("{}:{current_path}", directory.display()))
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        apply_extra_env(&mut command, extra_env);
        isolate_child_process_group(&mut command);
        let child = command
            .spawn()
            .map_err(|error| format!("Failed to run {tool}: {error}"))?;
        wait_child_output(child, deadline)
    }
}

#[cfg(target_os = "windows")]
fn run_windows_tool_command_capture(
    tool_path: &Path,
    args: &[&str],
    new_path: &str,
    deadline: CommandDeadline,
    extra_env: &[(&str, String)],
    working_dir: &Path,
) -> Result<std::process::Output, String> {
    use std::process::{Command, Stdio};

    let mut command = if is_windows_command_script(tool_path) {
        let path = tool_path.to_string_lossy();
        let args = args
            .iter()
            .map(|arg| windows_cmd_double_quote_arg(arg))
            .collect::<Vec<_>>()
            .join(" ");
        let command_line = format!(
            "call {}{}",
            win_quote_path_for_batch(&path),
            if args.is_empty() {
                String::new()
            } else {
                format!(" {args}")
            }
        );
        let mut command = Command::new("cmd");
        command
            .args(["/D", "/S", "/C"])
            .raw_arg(&command_line)
            .env("PATH", new_path)
            .creation_flags(CREATE_NO_WINDOW);
        command
    } else {
        let mut command = Command::new(tool_path);
        command
            .args(args)
            .env("PATH", new_path)
            .creation_flags(CREATE_NO_WINDOW);
        command
    };

    apply_extra_env(&mut command, extra_env);
    command
        .current_dir(working_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = command
        .spawn()
        .map_err(|error| format!("Failed to run tool: {error}"))?;
    wait_child_output(child, deadline)
}

#[cfg(target_os = "windows")]
fn wsl_unc_path_to_linux(path: &Path) -> Option<String> {
    use std::path::{Component, Prefix};

    let mut components = path.components();
    let Component::Prefix(prefix) = components.next()? else {
        return None;
    };
    match prefix.kind() {
        Prefix::UNC(server, _share) | Prefix::VerbatimUNC(server, _share) => {
            let server_name = server.to_string_lossy();
            if !(server_name.eq_ignore_ascii_case("wsl$")
                || server_name.eq_ignore_ascii_case("wsl.localhost"))
            {
                return None;
            }
        }
        _ => return None,
    }

    let mut linux = String::new();
    for component in components {
        match component {
            Component::RootDir => {}
            Component::Normal(part) => {
                linux.push('/');
                linux.push_str(&part.to_string_lossy());
            }
            Component::CurDir | Component::ParentDir | Component::Prefix(_) => return None,
        }
    }
    (!linux.is_empty()).then_some(linux)
}

#[cfg(target_os = "windows")]
fn build_wsl_env_argv(extra_env: &[(&str, String)]) -> Result<Vec<String>, String> {
    let mut env_argv = Vec::new();
    for (key, value) in extra_env {
        if key.is_empty()
            || key.contains('=')
            || key
                .chars()
                .any(|character| character.is_whitespace() || character.is_control())
        {
            return Err(format!("invalid env for {key}"));
        }

        let linux_value = if *key == "OPENCODE_CONFIG_DIR" {
            let Some(value) = wsl_unc_path_to_linux(Path::new(value)) else {
                continue;
            };
            value
        } else {
            value.clone()
        };
        if linux_value.chars().any(char::is_control) {
            return Err(format!("invalid env for {key}"));
        }
        env_argv.push(format!("{key}={linux_value}"));
    }
    Ok(env_argv)
}

#[cfg(target_os = "windows")]
fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[cfg(target_os = "windows")]
fn build_wsl_tool_command(
    tool: &str,
    args: &[&str],
    deadline: CommandDeadline,
) -> Result<String, String> {
    let invocation = std::iter::once(tool)
        .chain(args.iter().copied())
        .collect::<Vec<_>>()
        .join(" ");
    let command = format!(
        "for flag in -lic -lc -c; do if \"${{SHELL:-sh}}\" \"$flag\" 'command -v {tool}' >/dev/null 2>&1; then exec \"${{SHELL:-sh}}\" \"$flag\" '{invocation}'; fi; done; exit 127"
    );
    let remaining = deadline.remaining()?;
    let timeout_arg = format!("{:.3}s", remaining.as_secs_f64());
    Ok(format!(
        "command -v timeout >/dev/null 2>&1 || {{ echo 'timeout is required for bounded CLI execution' >&2; exit 127; }}; exec timeout --signal=TERM --kill-after=1s {timeout_arg} sh -c {}",
        shell_single_quote(&command)
    ))
}

#[cfg(target_os = "windows")]
fn run_wsl_tool_command(
    tool: &str,
    args: &[&str],
    distro: &str,
    deadline: CommandDeadline,
    extra_env: &[(&str, String)],
    working_dir: &Path,
) -> Result<std::process::Output, String> {
    use std::process::{Command, Stdio};

    if !is_valid_wsl_distro_name(distro) {
        return Err(format!("[WSL:{distro}] invalid distro name"));
    }

    let command_line = build_wsl_tool_command(tool, args, deadline)?;
    let linux_working_dir = wsl_unc_path_to_linux(working_dir)
        .ok_or_else(|| format!("[WSL:{distro}] invalid working directory"))?;
    let env_argv =
        build_wsl_env_argv(extra_env).map_err(|error| format!("[WSL:{distro}] {error}"))?;

    let mut command = Command::new("wsl.exe");
    command
        .arg("-d")
        .arg(distro)
        .arg("--cd")
        .arg(linux_working_dir)
        .arg("--");
    if !env_argv.is_empty() {
        command.arg("env");
        for item in &env_argv {
            command.arg(item);
        }
    }
    command
        .args(["sh", "-c", &command_line])
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = command
        .spawn()
        .map_err(|error| format!("[WSL:{distro}] failed to run {tool}: {error}"))?;
    let output = wait_child_output(child, deadline).map_err(|error| {
        if error.starts_with("Command timed out") {
            format!("[WSL:{distro}] {error}")
        } else {
            error
        }
    })?;
    if output.status.code() == Some(124) {
        return Err(format!("[WSL:{distro}] {}", deadline.timeout_error()));
    }
    Ok(output)
}

#[cfg(target_os = "windows")]
fn wsl_distro_for_tool(tool: &str) -> Option<String> {
    let override_dir = match tool {
        "claude" => crate::settings::get_claude_override_dir(),
        "codex" => crate::settings::get_codex_override_dir(),
        "gemini" => crate::settings::get_gemini_override_dir(),
        "grok" => crate::settings::get_grok_override_dir(),
        "opencode" => crate::settings::get_opencode_override_dir(),
        _ => None,
    }?;

    wsl_distro_from_path(&override_dir)
}

#[cfg(target_os = "windows")]
fn wsl_distro_from_path(path: &Path) -> Option<String> {
    use std::path::{Component, Prefix};

    let Some(Component::Prefix(prefix)) = path.components().next() else {
        return None;
    };

    match prefix.kind() {
        Prefix::UNC(server, share) | Prefix::VerbatimUNC(server, share) => {
            let server_name = server.to_string_lossy();
            if server_name.eq_ignore_ascii_case("wsl$")
                || server_name.eq_ignore_ascii_case("wsl.localhost")
            {
                let distro = share.to_string_lossy().to_string();
                if !distro.is_empty() {
                    return Some(distro);
                }
            }
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_version() {
        assert_eq!(extract_version("claude 1.0.20"), "1.0.20");
        assert_eq!(extract_version("v2.3.4-beta.1"), "2.3.4-beta.1");
        assert_eq!(extract_version("no version here"), "no version here");
    }

    #[test]
    fn grok_is_registered_for_version_discovery() {
        assert!(VALID_TOOLS.contains(&"grok"));
        assert!(npm_prerelease_tags("grok").is_empty());
    }

    #[test]
    fn runtime_command_rejects_shell_metacharacters_before_tool_lookup() {
        let dir = tempfile::tempdir().unwrap();
        let error = run_detected_tool_command_with_timeout(
            "opencode",
            &["models;echo"],
            std::time::Duration::from_secs(1),
            &[],
            dir.path(),
        )
        .unwrap_err();

        assert_eq!(error, "Invalid tool command arguments");
    }

    #[test]
    fn runtime_command_pipe_reader_caps_captured_output() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let overflow = AtomicBool::new(false);
        let input = vec![b'x'; TOOL_COMMAND_OUTPUT_LIMIT + 1];
        let output = read_bounded_pipe(std::io::Cursor::new(input), &overflow);

        assert_eq!(output.len(), TOOL_COMMAND_OUTPUT_LIMIT);
        assert!(overflow.load(Ordering::Acquire));
    }

    #[test]
    fn test_compare_semver() {
        use std::cmp::Ordering;
        assert_eq!(
            compare_semver("2.1.156", "2.1.154"),
            Some(Ordering::Greater)
        );
        assert_eq!(compare_semver("2.1.154", "2.1.156"), Some(Ordering::Less));
        assert_eq!(compare_semver("2.1.156", "2.1.156"), Some(Ordering::Equal));
        // 预发布 < 同核心正式版
        assert_eq!(
            compare_semver("2.1.156-beta.1", "2.1.156"),
            Some(Ordering::Less)
        );
        // core 更高的预发布仍高于较低的正式版（gemini nightly 场景）
        assert_eq!(
            compare_semver("0.45.0-nightly.1", "0.44.1"),
            Some(Ordering::Greater)
        );
        // 大 patch（codex 时间戳式）不溢出
        assert_eq!(
            compare_semver("0.1.2505172116", "0.135.0"),
            Some(Ordering::Less)
        );
        // 无法解析返回 None（gemini 的 `false` 脏 tag）
        assert_eq!(compare_semver("false", "1.0.0"), None);
    }

    #[test]
    fn test_pick_latest_version() {
        use serde_json::json;
        let tags = json!({
            "latest": "2.1.154",
            "next": "2.1.156",
            "stable": "2.1.145"
        });
        let map = tags.as_object().unwrap();

        // 本地领先 latest（在 next 通道）→ 补查到 next，数字对齐
        assert_eq!(
            pick_latest_version(map, &["next"], Some("2.1.156")),
            Some("2.1.156".to_string())
        );
        // 本地等于 latest → 不补查，仍显示 latest
        assert_eq!(
            pick_latest_version(map, &["next"], Some("2.1.154")),
            Some("2.1.154".to_string())
        );
        // 本地落后 latest（稳定通道用户）→ 不补查，不被推向预发布版
        assert_eq!(
            pick_latest_version(map, &["next"], Some("2.1.145")),
            Some("2.1.154".to_string())
        );
        // 无预发布白名单 → 永远只看 latest（不解析 local，避免脏 local 触发）
        assert_eq!(
            pick_latest_version(map, &[], Some("2.1.156")),
            Some("2.1.154".to_string())
        );
        // 本地版本未知 → 保守只看 latest
        assert_eq!(
            pick_latest_version(map, &["next"], None),
            Some("2.1.154".to_string())
        );
    }

    #[test]
    fn test_pick_latest_version_filters_dirty_prerelease() {
        use serde_json::json;
        // 模拟 codex：beta 是低于 latest 的时间戳式脏版本
        let tags = json!({
            "latest": "0.135.0",
            "beta": "0.1.2505172116"
        });
        let map = tags.as_object().unwrap();
        // 即便本地领先 latest，低于 latest 的脏 beta 也不会被选
        assert_eq!(
            pick_latest_version(map, &["beta"], Some("0.200.0")),
            Some("0.135.0".to_string())
        );
    }

    #[cfg(target_os = "windows")]
    mod wsl_helpers {
        use super::super::*;

        #[test]
        fn test_is_valid_shell() {
            assert!(is_valid_shell("bash"));
            assert!(is_valid_shell("zsh"));
            assert!(is_valid_shell("sh"));
            assert!(is_valid_shell("fish"));
            assert!(is_valid_shell("dash"));
            assert!(is_valid_shell("/usr/bin/bash"));
            assert!(is_valid_shell("/bin/zsh"));
            assert!(!is_valid_shell("powershell"));
            assert!(!is_valid_shell("cmd"));
            assert!(!is_valid_shell(""));
        }

        #[test]
        fn test_is_valid_shell_flag() {
            assert!(is_valid_shell_flag("-c"));
            assert!(is_valid_shell_flag("-lc"));
            assert!(is_valid_shell_flag("-lic"));
            assert!(!is_valid_shell_flag("-x"));
            assert!(!is_valid_shell_flag(""));
            assert!(!is_valid_shell_flag("--login"));
        }

        #[test]
        fn test_default_flag_for_shell() {
            assert_eq!(default_flag_for_shell("sh"), "-c");
            assert_eq!(default_flag_for_shell("dash"), "-c");
            assert_eq!(default_flag_for_shell("/bin/dash"), "-c");
            assert_eq!(default_flag_for_shell("fish"), "-lc");
            assert_eq!(default_flag_for_shell("bash"), "-lic");
            assert_eq!(default_flag_for_shell("zsh"), "-lic");
            assert_eq!(default_flag_for_shell("/usr/bin/zsh"), "-lic");
        }

        #[test]
        fn test_is_valid_wsl_distro_name() {
            assert!(is_valid_wsl_distro_name("Ubuntu"));
            assert!(is_valid_wsl_distro_name("Ubuntu-22.04"));
            assert!(is_valid_wsl_distro_name("my_distro"));
            assert!(!is_valid_wsl_distro_name(""));
            assert!(!is_valid_wsl_distro_name("distro with spaces"));
            assert!(!is_valid_wsl_distro_name(&"a".repeat(65)));
        }
    }

    #[test]
    fn opencode_extra_search_paths_includes_install_and_fallback_dirs() {
        let home = PathBuf::from("/home/tester");
        let install_dir = Some(std::ffi::OsString::from("/custom/opencode/bin"));
        let xdg_bin_dir = Some(std::ffi::OsString::from("/xdg/bin"));
        let gopath =
            std::env::join_paths([PathBuf::from("/go/path1"), PathBuf::from("/go/path2")]).ok();

        let paths = opencode_extra_search_paths(&home, install_dir, xdg_bin_dir, gopath);

        assert_eq!(paths[0], PathBuf::from("/custom/opencode/bin"));
        assert_eq!(paths[1], PathBuf::from("/xdg/bin"));
        assert!(paths.contains(&PathBuf::from("/home/tester/bin")));
        assert!(paths.contains(&PathBuf::from("/home/tester/.opencode/bin")));
        assert!(paths.contains(&PathBuf::from("/home/tester/.bun/bin")));
        assert!(paths.contains(&PathBuf::from("/home/tester/go/bin")));
        assert!(paths.contains(&PathBuf::from("/go/path1/bin")));
        assert!(paths.contains(&PathBuf::from("/go/path2/bin")));
    }

    #[test]
    fn opencode_extra_search_paths_deduplicates_repeated_entries() {
        let home = PathBuf::from("/home/tester");
        let same_dir = Some(std::ffi::OsString::from("/same/path"));

        let paths = opencode_extra_search_paths(&home, same_dir.clone(), same_dir, None);

        let count = paths
            .iter()
            .filter(|path| path.as_path() == std::path::Path::new("/same/path"))
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn opencode_extra_search_paths_deduplicates_bun_default_dir() {
        let home = PathBuf::from("/home/tester");
        let paths = opencode_extra_search_paths(&home, None, None, None);

        let count = paths
            .iter()
            .filter(|path| path.as_path() == std::path::Path::new("/home/tester/.bun/bin"))
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn mise_node_search_paths_include_shims_and_installed_node_bins() {
        let temp = tempfile::tempdir().expect("temp dir should be created");
        let home = temp.path();
        let node_bin = home
            .join(".local/share/mise/installs/node/25.8.0")
            .join("bin");
        std::fs::create_dir_all(&node_bin).expect("node bin should be created");

        let mut paths = Vec::new();
        extend_mise_node_search_paths(&mut paths, home);

        assert!(paths.contains(&home.join(".local/share/mise/shims")));
        assert!(paths.contains(&node_bin));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn tool_executable_candidates_non_windows_uses_plain_binary_name() {
        let dir = PathBuf::from("/usr/local/bin");
        let candidates = tool_executable_candidates("opencode", &dir);

        assert_eq!(candidates, vec![PathBuf::from("/usr/local/bin/opencode")]);
    }

    /// setsid 改造后的语义偶检：spawn（含 pre_exec setsid）能启动、输出能捕获。
    /// `/bin/echo --version` 在 macOS/Linux 均即刻成功退出。等价于上游
    /// `probe_version_command_captures_healthy_tool_output` 的回归点，但 fork
    /// 的探测函数是 `scan_cli_version`（未走 spawn 超时路径），故直接偶检
    /// `isolate_child_process_group` + `wait_child_output` 的健康路径。
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn isolated_child_process_group_captures_healthy_tool_output() {
        use std::process::{Command, Stdio};

        let mut cmd = Command::new("/bin/echo");
        cmd.arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        isolate_child_process_group(&mut cmd);
        let child = cmd.spawn().expect("spawn /bin/echo");
        let output = wait_child_output(
            child,
            CommandDeadline::new(std::time::Duration::from_secs(10)),
        )
        .expect("wait_child_output should succeed for /bin/echo");
        assert!(output.status.success());
    }

    /// 超时击杀路径：挂死的子进程到点被整组击杀、wait 返回超时错误而非永等。
    /// 同时锚定 setsid 改造后的语义——child 是新会话/新进程组组长，
    /// terminate_child_tree 的 kill(-pid) 仍能命中（回归红线：改回 process_group
    /// 或去掉隔离都会让本测试的击杀路径失效）。
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn isolated_hung_child_is_killed_on_deadline() {
        use std::process::{Command, Stdio};

        let mut cmd = Command::new("/bin/sh");
        cmd.args(["-c", "sleep 30"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        isolate_child_process_group(&mut cmd);
        let child = cmd.spawn().expect("spawn sleep");
        let started = std::time::Instant::now();
        let result = wait_child_output(
            child,
            CommandDeadline::new(std::time::Duration::from_millis(200)),
        );
        assert!(result.is_err(), "expected timeout error, got {result:?}");
        assert!(
            started.elapsed() < std::time::Duration::from_secs(5),
            "kill should return promptly instead of waiting out the sleep"
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn tool_executable_candidates_windows_includes_cmd_exe_and_plain_name() {
        let dir = PathBuf::from("C:\\tools");
        let candidates = tool_executable_candidates("opencode", &dir);

        assert_eq!(
            candidates,
            vec![
                PathBuf::from("C:\\tools\\opencode.cmd"),
                PathBuf::from("C:\\tools\\opencode.exe"),
                PathBuf::from("C:\\tools\\opencode"),
            ]
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn merge_path_segments_win_preserves_order_and_dedupes_case_insensitively() {
        let merged = merge_path_segments_win(&[
            r"C:\a;C:\B;%SystemRoot%\system32",
            r"C:\b;C:\a", // dup of C:\a (case-insensitive) and C:\B
            "",
        ]);
        assert_eq!(merged, r"C:\a;C:\B;%SystemRoot%\system32");
    }

    #[cfg(unix)]
    #[test]
    fn prepend_search_dir_to_path_preserves_non_utf8_bytes() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};

        let current = std::ffi::OsString::from_vec(b"/usr/bin:/tmp/\xff/bin".to_vec());
        let combined = prepend_search_dir_to_path(Path::new("/candidate/bin"), &current);

        assert_eq!(
            combined.as_os_str().as_bytes(),
            b"/candidate/bin:/usr/bin:/tmp/\xff/bin"
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn expand_env_chars_preserves_unknown_vars_and_plain_text() {
        // No percent signs -> verbatim.
        assert_eq!(
            expand_env_chars(r"C:\Program Files\nodejs"),
            r"C:\Program Files\nodejs"
        );
        // Undefined variable is preserved verbatim (nothing dropped).
        assert_eq!(
            expand_env_chars(r"D:\npm-global\%DEFINITELY_NOT_A_REAL_VAR_xyz%\bin"),
            r"D:\npm-global\%DEFINITELY_NOT_A_REAL_VAR_xyz%\bin"
        );
        // Empty percent pair is not treated as a variable name.
        assert_eq!(expand_env_chars(r"C:\path\%%\tail"), r"C:\path\%%\tail");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn build_tool_search_paths_includes_standalone_installer_dirs() {
        // Non-npm installer locations must be scanned even when the process PATH
        // dropped them (regression guard for #6061 / #6278 / #6047).
        let local_data = dirs::data_local_dir().expect("LOCALAPPDATA should resolve");

        let codex_paths = build_tool_search_paths("codex");
        assert!(codex_paths.contains(
            &local_data
                .join("Programs")
                .join("OpenAI")
                .join("Codex")
                .join("bin")
        ));

        let claude_paths = build_tool_search_paths("claude");
        assert!(claude_paths.contains(&local_data.join("Programs").join("claude")));

        // The standalone Codex dir is codex-specific; it must not pollute other tools.
        assert!(!build_tool_search_paths("gemini").contains(
            &local_data
                .join("Programs")
                .join("OpenAI")
                .join("Codex")
                .join("bin")
        ));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_path_lookup_ignores_same_named_file_in_current_directory() {
        let current_dir = tempfile::tempdir().expect("current directory should be created");
        let path_dir = tempfile::tempdir().expect("PATH directory should be created");
        std::fs::write(current_dir.path().join("codex.cmd"), "@echo current\r\n")
            .expect("current-directory shim should be created");
        let expected = path_dir.path().join("codex.cmd");
        std::fs::write(&expected, "@echo path\r\n").expect("PATH shim should be created");

        let effective_path =
            std::env::join_paths([path_dir.path()]).expect("test PATH should join");
        let output = windows_path_lookup_command("codex", &effective_path)
            .current_dir(current_dir.path())
            .output()
            .expect("where.exe should execute");
        let stderr = decode_command_output(&output.stderr);
        let matches = decode_command_output(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(PathBuf::from)
            .collect::<Vec<_>>();

        assert!(output.status.success(), "where.exe failed: {stderr}");
        assert_eq!(matches.len(), 1);
        assert_eq!(
            std::fs::canonicalize(&matches[0]).expect("where.exe match should canonicalize"),
            std::fs::canonicalize(&expected).expect("expected PATH shim should canonicalize")
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_shell_compatible_path_strips_verbatim_prefixes() {
        assert_eq!(
            windows_shell_compatible_path(Path::new(r"\\?\C:\tools\codex.cmd")),
            PathBuf::from(r"C:\tools\codex.cmd")
        );
        assert_eq!(
            windows_shell_compatible_path(Path::new(r"\\?\UNC\server\share\tools\codex.cmd")),
            PathBuf::from(r"\\server\share\tools\codex.cmd")
        );
        assert_eq!(
            windows_shell_compatible_path(Path::new(r"C:\tools\codex.cmd")),
            PathBuf::from(r"C:\tools\codex.cmd")
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn run_windows_tool_version_command_accepts_canonicalized_cmd_path() {
        let dir = tempfile::tempdir().expect("temp dir should be created");
        let cmd = dir.path().join("codex.cmd");
        std::fs::write(&cmd, "@echo off\r\necho codex-cli 0.144.3\r\n")
            .expect("cmd shim should be created");
        let canonical = std::fs::canonicalize(&cmd).expect("cmd shim should canonicalize");
        assert!(
            canonical.to_string_lossy().starts_with(r"\\?\"),
            "Windows canonical paths should use the verbatim prefix: {}",
            canonical.display()
        );

        let current_path = effective_path_string();
        let output = run_windows_tool_version_command(&canonical, &current_path)
            .expect("canonicalized cmd shim should execute");
        let stderr = decode_command_output(&output.stderr);

        assert!(output.status.success(), "cmd shim failed: {stderr}");
        assert_eq!(
            decode_command_output(&output.stdout).trim(),
            "codex-cli 0.144.3"
        );
    }
}
