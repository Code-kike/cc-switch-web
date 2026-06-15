//! 全局 HTTP 客户端模块
//!
//! 提供支持全局代理配置的 HTTP 客户端。
//! 所有需要发送 HTTP 请求的模块都应使用此模块提供的客户端。

use once_cell::sync::OnceCell;
use reqwest::Client;
use std::env;
use std::net::IpAddr;
use std::sync::RwLock;
use std::time::Duration;

/// 全局 HTTP 客户端实例
static GLOBAL_CLIENT: OnceCell<RwLock<Client>> = OnceCell::new();

/// SSRF-guarded outbound client (audit P4-A2).
///
/// Mirrors the global client's proxy configuration but installs a redirect
/// policy that re-runs the internal-IP block-check on every redirect hop, so a
/// public host that 30x-redirects to `http://127.0.0.1/` / `169.254.169.254` /
/// `100.64.x.x` cannot bypass the initial-URL-only `validate_outbound_url`
/// guard. Used by the WEB outbound handlers' service layer (balance /
/// coding_plan / model_fetch); the proxy hot path keeps the unguarded
/// `GLOBAL_CLIENT` so its upstream-3xx pass-through behavior is unchanged.
static GUARDED_CLIENT: OnceCell<RwLock<Client>> = OnceCell::new();

/// 当前代理 URL（用于日志和状态查询）
static CURRENT_PROXY_URL: OnceCell<RwLock<Option<String>>> = OnceCell::new();

/// CC Switch 代理服务器当前监听的端口
static CC_SWITCH_PROXY_PORT: OnceCell<RwLock<u16>> = OnceCell::new();

/// 设置 CC Switch 代理服务器的监听端口
///
/// 应在代理服务器启动时调用，以便系统代理检测能正确识别自己的端口
pub fn set_proxy_port(port: u16) {
    if let Some(lock) = CC_SWITCH_PROXY_PORT.get() {
        if let Ok(mut current_port) = lock.write() {
            *current_port = port;
            log::debug!("[GlobalProxy] Updated CC Switch proxy port to {port}");
        }
    } else {
        let _ = CC_SWITCH_PROXY_PORT.set(RwLock::new(port));
        log::debug!("[GlobalProxy] Initialized CC Switch proxy port to {port}");
    }
}

/// 获取 CC Switch 代理服务器的监听端口
fn get_proxy_port() -> u16 {
    CC_SWITCH_PROXY_PORT
        .get()
        .and_then(|lock| lock.read().ok())
        .map(|port| *port)
        .unwrap_or(15721) // 默认端口作为回退
}

/// 初始化全局 HTTP 客户端
///
/// 应在应用启动时调用一次。
///
/// # Arguments
/// * `proxy_url` - 代理 URL，如 `http://127.0.0.1:7890` 或 `socks5://127.0.0.1:1080`
///   传入 None 或空字符串表示直连
pub fn init(proxy_url: Option<&str>) -> Result<(), String> {
    let effective_url = proxy_url.filter(|s| !s.trim().is_empty());
    let client = build_client(effective_url)?;

    // 尝试初始化全局客户端，如果已存在则记录警告并使用 apply_proxy 更新
    if GLOBAL_CLIENT.set(RwLock::new(client.clone())).is_err() {
        log::warn!(
            "[GlobalProxy] [GP-003] Already initialized, updating instead: {}",
            effective_url
                .map(mask_url)
                .unwrap_or_else(|| "direct connection".to_string())
        );
        // 已初始化，改用 apply_proxy 更新
        return apply_proxy(proxy_url);
    }

    // 同步初始化 SSRF-guarded 客户端（与主客户端共用代理配置）
    let _ = GUARDED_CLIENT.set(RwLock::new(build_guarded_client(effective_url)?));

    // 初始化代理 URL 记录
    let _ = CURRENT_PROXY_URL.set(RwLock::new(effective_url.map(|s| s.to_string())));

    log::info!(
        "[GlobalProxy] Initialized: {}",
        effective_url
            .map(mask_url)
            .unwrap_or_else(|| "direct connection".to_string())
    );

    Ok(())
}

/// 验证代理配置（不应用）
///
/// 只验证代理 URL 是否有效，不实际更新全局客户端。
/// 用于在持久化之前验证配置的有效性。
///
/// # Arguments
/// * `proxy_url` - 代理 URL，None 或空字符串表示直连
///
/// # Returns
/// 验证成功返回 Ok(())，失败返回错误信息
pub fn validate_proxy(proxy_url: Option<&str>) -> Result<(), String> {
    let effective_url = proxy_url.filter(|s| !s.trim().is_empty());
    // 只调用 build_client 来验证，但不应用
    build_client(effective_url)?;
    Ok(())
}

/// 应用代理配置（假设已验证）
///
/// 直接应用代理配置到全局客户端，不做额外验证。
/// 应在 validate_proxy 成功后调用。
///
/// # Arguments
/// * `proxy_url` - 代理 URL，None 或空字符串表示直连
pub fn apply_proxy(proxy_url: Option<&str>) -> Result<(), String> {
    let effective_url = proxy_url.filter(|s| !s.trim().is_empty());
    let new_client = build_client(effective_url)?;
    let new_guarded = build_guarded_client(effective_url)?;

    // 更新客户端
    if let Some(lock) = GLOBAL_CLIENT.get() {
        let mut client = lock.write().map_err(|e| {
            log::error!("[GlobalProxy] [GP-001] Failed to acquire write lock: {e}");
            "Failed to update proxy: lock poisoned".to_string()
        })?;
        *client = new_client;
    } else {
        // 如果还没初始化，则初始化
        return init(proxy_url);
    }

    // 同步更新 SSRF-guarded 客户端
    if let Some(lock) = GUARDED_CLIENT.get() {
        if let Ok(mut guarded) = lock.write() {
            *guarded = new_guarded;
        }
    } else {
        let _ = GUARDED_CLIENT.set(RwLock::new(new_guarded));
    }

    // 更新代理 URL 记录
    if let Some(lock) = CURRENT_PROXY_URL.get() {
        let mut url = lock.write().map_err(|e| {
            log::error!("[GlobalProxy] [GP-002] Failed to acquire URL write lock: {e}");
            "Failed to update proxy URL record: lock poisoned".to_string()
        })?;
        *url = effective_url.map(|s| s.to_string());
    }

    log::info!(
        "[GlobalProxy] Applied: {}",
        effective_url
            .map(mask_url)
            .unwrap_or_else(|| "direct connection".to_string())
    );

    Ok(())
}

/// 更新代理配置（热更新）
///
/// 可在运行时调用以更改代理设置，无需重启应用。
/// 注意：此函数同时验证和应用，如果需要先验证后持久化再应用，
/// 请使用 validate_proxy + apply_proxy 组合。
///
/// # Arguments
/// * `proxy_url` - 新的代理 URL，None 或空字符串表示直连
#[allow(dead_code)]
pub fn update_proxy(proxy_url: Option<&str>) -> Result<(), String> {
    let effective_url = proxy_url.filter(|s| !s.trim().is_empty());
    let new_client = build_client(effective_url)?;
    let new_guarded = build_guarded_client(effective_url)?;

    // 更新客户端
    if let Some(lock) = GLOBAL_CLIENT.get() {
        let mut client = lock.write().map_err(|e| {
            log::error!("[GlobalProxy] [GP-001] Failed to acquire write lock: {e}");
            "Failed to update proxy: lock poisoned".to_string()
        })?;
        *client = new_client;
    } else {
        // 如果还没初始化，则初始化
        return init(proxy_url);
    }

    // 同步更新 SSRF-guarded 客户端
    if let Some(lock) = GUARDED_CLIENT.get() {
        if let Ok(mut guarded) = lock.write() {
            *guarded = new_guarded;
        }
    } else {
        let _ = GUARDED_CLIENT.set(RwLock::new(new_guarded));
    }

    // 更新代理 URL 记录
    if let Some(lock) = CURRENT_PROXY_URL.get() {
        let mut url = lock.write().map_err(|e| {
            log::error!("[GlobalProxy] [GP-002] Failed to acquire URL write lock: {e}");
            "Failed to update proxy URL record: lock poisoned".to_string()
        })?;
        *url = effective_url.map(|s| s.to_string());
    }

    log::info!(
        "[GlobalProxy] Updated: {}",
        effective_url
            .map(mask_url)
            .unwrap_or_else(|| "direct connection".to_string())
    );

    Ok(())
}

/// 获取全局 HTTP 客户端
///
/// 返回配置了代理的客户端（如果已配置代理），否则返回跟随系统代理的客户端。
pub fn get() -> Client {
    GLOBAL_CLIENT
        .get()
        .and_then(|lock| lock.read().ok())
        .map(|c| c.clone())
        .unwrap_or_else(|| {
            log::warn!("[GlobalProxy] [GP-004] Client not initialized, using fallback");
            build_client(None).unwrap_or_default()
        })
}

/// 获取 SSRF-guarded outbound 客户端（audit P4-A2）。
///
/// 与 `get()` 共用代理配置，但额外安装了重定向策略：每一跳重定向目标若解析为
/// 内网/环回/链路本地/未指定/CGNAT 的 **IP 字面量**，即中止重定向，防止公网主机
/// 通过 `302 Location: http://127.0.0.1/` 绕过仅校验初始 URL 的
/// `validate_outbound_url`。Web outbound 服务层（balance / coding_plan /
/// model_fetch）使用此客户端；代理热路径仍用未加固的 `get()`，保持其对上游 3xx
/// 的透传行为不变。
///
/// 残留：重定向目标若是 **域名**（而非 IP 字面量），同步的重定向回调无法做 DNS
/// 解析，因此无法在此拦截 → 这是已知的 DNS-rebinding/域名重定向残留，与初始 URL
/// 校验同样依赖 `validate_outbound_url` 的解析时刻（auth-gated 部署下可接受）。
pub fn get_guarded() -> Client {
    GUARDED_CLIENT
        .get()
        .and_then(|lock| lock.read().ok())
        .map(|c| c.clone())
        .unwrap_or_else(|| {
            log::warn!("[GlobalProxy] [GP-004] Guarded client not initialized, using fallback");
            build_guarded_client(None).unwrap_or_default()
        })
}

/// 获取当前代理 URL
///
/// 返回当前配置的代理 URL，None 表示直连。
pub fn get_current_proxy_url() -> Option<String> {
    CURRENT_PROXY_URL
        .get()
        .and_then(|lock| lock.read().ok())
        .and_then(|url| url.clone())
}

/// 检查是否正在使用代理
#[allow(dead_code)]
pub fn is_proxy_enabled() -> bool {
    get_current_proxy_url().is_some()
}

/// 构建 HTTP 客户端
fn build_client(proxy_url: Option<&str>) -> Result<Client, String> {
    configure_builder(base_builder(), proxy_url)?
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))
}

/// 构建 SSRF-guarded outbound 客户端（audit P4-A2）。
///
/// 与 `build_client` 同样的超时/代理/压缩配置，但加装 `redirect::Policy::custom`：
/// 每一跳重定向目标的 host 若是被 `ip_guard::is_blocked_ip` 判定为内网/环回/链路
/// 本地/未指定/CGNAT 的 IP 字面量，则中止（`stop`）；公网→公网重定向仍在默认 10
/// 跳预算内放行。回调是同步的、不做 IO（不解析 DNS），因此可安全用于重定向策略。
fn build_guarded_client(proxy_url: Option<&str>) -> Result<Client, String> {
    let builder = base_builder().redirect(guarded_redirect_policy());
    configure_builder(builder, proxy_url)?
        .build()
        .map_err(|e| format!("Failed to build guarded HTTP client: {e}"))
}

/// 重定向策略：逐跳重新执行内网 IP 拦截，公网→公网放行，命中内网 IP 字面量即中止。
fn guarded_redirect_policy() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(|attempt| {
        if attempt.previous().len() >= 10 {
            return attempt.error("too many redirects");
        }
        match attempt.url().host() {
            // IP 字面量：直接分类，命中内网即中止。
            Some(url::Host::Ipv4(ip)) if crate::proxy::ip_guard::is_blocked_ip(IpAddr::V4(ip)) => {
                return attempt.stop();
            }
            Some(url::Host::Ipv6(ip)) if crate::proxy::ip_guard::is_blocked_ip(IpAddr::V6(ip)) => {
                return attempt.stop();
            }
            // 公网 IP 字面量 / 域名：放行（域名见 get_guarded 的 DNS 残留说明）。
            _ => {}
        }
        attempt.follow()
    })
}

/// 客户端通用基础配置（超时/连接池/禁用自动解压），代理与重定向策略由调用方补充。
fn base_builder() -> reqwest::ClientBuilder {
    Client::builder()
        .timeout(Duration::from_secs(600))
        .connect_timeout(Duration::from_secs(30))
        .pool_max_idle_per_host(10)
        .tcp_keepalive(Duration::from_secs(60))
        // 禁用 reqwest 自动解压：防止 reqwest 覆盖客户端原始 accept-encoding header。
        // 响应解压由 response_processor 根据 content-encoding 手动处理。
        .no_gzip()
        .no_brotli()
        .no_deflate()
}

/// 将代理配置应用到 builder（与历史 `build_client` 的代理选择逻辑一致）。
fn configure_builder(
    mut builder: reqwest::ClientBuilder,
    proxy_url: Option<&str>,
) -> Result<reqwest::ClientBuilder, String> {
    // 有代理地址则使用代理，否则跟随系统代理
    if let Some(url) = proxy_url {
        // 先验证 URL 格式和 scheme
        let parsed = url::Url::parse(url)
            .map_err(|e| format!("Invalid proxy URL '{}': {}", mask_url(url), e))?;

        let scheme = parsed.scheme();
        if !["http", "https", "socks5", "socks5h"].contains(&scheme) {
            return Err(format!(
                "Invalid proxy scheme '{}' in URL '{}'. Supported: http, https, socks5, socks5h",
                scheme,
                mask_url(url)
            ));
        }

        let proxy = reqwest::Proxy::all(url)
            .map_err(|e| format!("Invalid proxy URL '{}': {}", mask_url(url), e))?;
        builder = builder.proxy(proxy);
        log::debug!("[GlobalProxy] Proxy configured: {}", mask_url(url));
    } else {
        // 未设置全局代理时，让 reqwest 自动检测系统代理（环境变量）
        // 若系统代理指向本机，禁用系统代理避免自环
        if system_proxy_points_to_loopback() {
            builder = builder.no_proxy();
            log::warn!(
                "[GlobalProxy] System proxy points to localhost, bypassing to avoid recursion"
            );
        } else {
            log::debug!("[GlobalProxy] Following system proxy (no explicit proxy configured)");
        }
    }

    Ok(builder)
}

fn system_proxy_points_to_loopback() -> bool {
    const KEYS: [&str; 6] = [
        "HTTP_PROXY",
        "http_proxy",
        "HTTPS_PROXY",
        "https_proxy",
        "ALL_PROXY",
        "all_proxy",
    ];

    KEYS.iter()
        .filter_map(|key| env::var(key).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .any(|value| proxy_points_to_loopback(&value))
}

fn proxy_points_to_loopback(value: &str) -> bool {
    fn host_is_loopback(host: &str) -> bool {
        if host.eq_ignore_ascii_case("localhost") {
            return true;
        }
        host.parse::<IpAddr>()
            .map(|ip| ip.is_loopback())
            .unwrap_or(false)
    }

    // 检查是否指向 CC Switch 自己的代理端口
    // 只有指向自己的代理才需要跳过，避免递归
    fn is_cc_switch_proxy_port(port: Option<u16>) -> bool {
        let cc_switch_port = get_proxy_port();
        port == Some(cc_switch_port)
    }

    if let Ok(parsed) = url::Url::parse(value) {
        if let Some(host) = parsed.host_str() {
            // 只有当主机是 loopback 且端口是 CC Switch 的端口时才返回 true
            return host_is_loopback(host) && is_cc_switch_proxy_port(parsed.port());
        }
        return false;
    }

    let with_scheme = format!("http://{value}");
    if let Ok(parsed) = url::Url::parse(&with_scheme) {
        if let Some(host) = parsed.host_str() {
            return host_is_loopback(host) && is_cc_switch_proxy_port(parsed.port());
        }
    }

    false
}

/// 隐藏 URL 中的敏感信息（用于日志）
pub fn mask_url(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        // 隐藏用户名和密码，保留 scheme、host 和端口
        let host = parsed.host_str().unwrap_or("?");
        match parsed.port() {
            Some(port) => format!("{}://{}:{}", parsed.scheme(), host, port),
            None => format!("{}://{}", parsed.scheme(), host),
        }
    } else {
        // URL 解析失败，返回部分内容
        if url.len() > 20 {
            format!("{}...", &url[..20])
        } else {
            url.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn test_mask_url() {
        assert_eq!(mask_url("http://127.0.0.1:7890"), "http://127.0.0.1:7890");
        assert_eq!(
            mask_url("http://user:pass@127.0.0.1:7890"),
            "http://127.0.0.1:7890"
        );
        assert_eq!(
            mask_url("socks5://admin:secret@proxy.example.com:1080"),
            "socks5://proxy.example.com:1080"
        );
        // 无端口的 URL 不应显示 ":?"
        assert_eq!(
            mask_url("http://proxy.example.com"),
            "http://proxy.example.com"
        );
        assert_eq!(
            mask_url("https://user:pass@proxy.example.com"),
            "https://proxy.example.com"
        );
    }

    #[test]
    fn test_build_client_direct() {
        let result = build_client(None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_guarded_client_direct() {
        // The SSRF-guarded outbound client (audit P4-A2) must build with the
        // same proxy permutations as the main client.
        assert!(build_guarded_client(None).is_ok());
        assert!(build_guarded_client(Some("http://127.0.0.1:7890")).is_ok());
        assert!(build_guarded_client(Some("socks5://127.0.0.1:1080")).is_ok());
        assert!(build_guarded_client(Some("invalid-scheme://127.0.0.1:7890")).is_err());
    }

    #[test]
    fn test_build_client_with_http_proxy() {
        let result = build_client(Some("http://127.0.0.1:7890"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_client_with_socks5_proxy() {
        let result = build_client(Some("socks5://127.0.0.1:1080"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_client_invalid_url() {
        // reqwest::Proxy::all 对某些无效 URL 不会立即报错
        // 使用明确无效的 scheme 来触发错误
        let result = build_client(Some("invalid-scheme://127.0.0.1:7890"));
        assert!(result.is_err(), "Should reject invalid proxy scheme");
    }

    #[test]
    fn test_proxy_points_to_loopback() {
        // 设置 CC Switch 代理端口为 15721（默认值）
        set_proxy_port(15721);

        // 只有指向 CC Switch 自己端口的 loopback 地址才返回 true
        assert!(proxy_points_to_loopback("http://127.0.0.1:15721"));
        assert!(proxy_points_to_loopback("socks5://localhost:15721"));
        assert!(proxy_points_to_loopback("127.0.0.1:15721"));

        // 其他 loopback 端口不应该被跳过（允许使用其他本地代理工具）
        assert!(!proxy_points_to_loopback("http://127.0.0.1:7890"));
        assert!(!proxy_points_to_loopback("socks5://localhost:1080"));

        // 非 loopback 地址不应该被跳过
        assert!(!proxy_points_to_loopback("http://192.168.1.10:7890"));
        assert!(!proxy_points_to_loopback("http://192.168.1.10:15721"));
    }

    #[test]
    fn test_system_proxy_points_to_loopback() {
        let _guard = env_lock().lock().unwrap();

        // 设置 CC Switch 代理端口
        set_proxy_port(15721);

        let keys = [
            "HTTP_PROXY",
            "http_proxy",
            "HTTPS_PROXY",
            "https_proxy",
            "ALL_PROXY",
            "all_proxy",
        ];

        for key in &keys {
            std::env::remove_var(key);
        }

        // 指向 CC Switch 端口的代理应该被跳过
        std::env::set_var("HTTP_PROXY", "http://127.0.0.1:15721");
        assert!(system_proxy_points_to_loopback());

        // 指向其他端口的本地代理不应该被跳过
        std::env::set_var("HTTP_PROXY", "http://127.0.0.1:7890");
        assert!(!system_proxy_points_to_loopback());

        // 非 loopback 地址不应该被跳过
        std::env::set_var("HTTP_PROXY", "http://10.0.0.2:7890");
        assert!(!system_proxy_points_to_loopback());

        for key in &keys {
            std::env::remove_var(key);
        }
    }
}
