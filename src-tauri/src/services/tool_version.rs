use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const VALID_TOOLS: [&str; 4] = ["claude", "codex", "gemini", "opencode"];

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

    let latest_version = match tool {
        "claude" => fetch_npm_latest_version(&client, "@anthropic-ai/claude-code").await,
        "codex" => fetch_npm_latest_version(&client, "@openai/codex").await,
        "gemini" => fetch_npm_latest_version(&client, "@google/gemini-cli").await,
        "opencode" => fetch_github_latest_version(&client, "anomalyco/opencode").await,
        _ => None,
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

async fn fetch_npm_latest_version(client: &reqwest::Client, package: &str) -> Option<String> {
    let url = format!(
        "{}/{}",
        read_base_url_env("CC_SWITCH_NPM_REGISTRY_BASE_URL", "https://registry.npmjs.org"),
        package.trim_start_matches('/')
    );
    match client.get(&url).send().await {
        Ok(resp) => match resp.json::<serde_json::Value>().await {
            Ok(json) => json
                .get("dist-tags")
                .and_then(|tags| tags.get("latest"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            Err(_) => None,
        },
        Err(_) => None,
    }
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

fn try_get_version(tool: &str) -> (Option<String>, Option<String>) {
    use std::process::Command;

    #[cfg(target_os = "windows")]
    let output = Command::new("cmd")
        .args(["/C", &format!("{tool} --version")])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    #[cfg(not(target_os = "windows"))]
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("{tool} --version"))
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
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

#[cfg(target_os = "windows")]
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

#[cfg(target_os = "windows")]
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
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
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

fn scan_cli_version(tool: &str) -> (Option<String>, Option<String>) {
    use std::process::Command;

    let home = dirs::home_dir().unwrap_or_default();
    let mut search_paths: Vec<PathBuf> = Vec::new();
    if !home.as_os_str().is_empty() {
        push_unique_path(&mut search_paths, home.join(".local/bin"));
        push_unique_path(&mut search_paths, home.join(".npm-global/bin"));
        push_unique_path(&mut search_paths, home.join("n/bin"));
        push_unique_path(&mut search_paths, home.join(".volta/bin"));
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

    let current_path = std::env::var("PATH").unwrap_or_default();

    for path in &search_paths {
        #[cfg(target_os = "windows")]
        let new_path = format!("{};{}", path.display(), current_path);

        #[cfg(not(target_os = "windows"))]
        let new_path = format!("{}:{}", path.display(), current_path);

        for tool_path in tool_executable_candidates(tool, path) {
            if !tool_path.exists() {
                continue;
            }

            #[cfg(target_os = "windows")]
            let output = Command::new("cmd")
                .args(["/C", &format!("\"{}\" --version", tool_path.display())])
                .env("PATH", &new_path)
                .creation_flags(CREATE_NO_WINDOW)
                .output();

            #[cfg(not(target_os = "windows"))]
            let output = Command::new(&tool_path)
                .arg("--version")
                .env("PATH", &new_path)
                .output();

            if let Ok(out) = output {
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
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

#[cfg(target_os = "windows")]
fn wsl_distro_for_tool(tool: &str) -> Option<String> {
    let override_dir = match tool {
        "claude" => crate::settings::get_claude_override_dir(),
        "codex" => crate::settings::get_codex_override_dir(),
        "gemini" => crate::settings::get_gemini_override_dir(),
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
