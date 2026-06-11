//! Standalone Web server entry point.
//!
//! Layer 2 / Task 4. Bootstraps the shared core (`bootstrap::init_core_state`),
//! mounts the 28 web_api handlers, and listens on `HOST:PORT` (defaults
//! `127.0.0.1:3010`). Graceful shutdown on SIGINT/SIGTERM.
//!
//! Run with:
//!   cargo run --no-default-features --features web-server --example server
//!
//! Environment variables:
//!   HOST            (default: 127.0.0.1) — refuse non-loopback unless
//!                   ALLOW_HTTP_BASIC_OVER_HTTP=1
//!   PORT            (default: 3010 — matches the systemd unit + install script)
//!   CC_SWITCH_DATA_DIR (default: ~/.cc-switch) — used by bootstrap::data_dir
//!   CORS_ALLOW_ORIGINS (comma-separated, optional)
//!   ENABLE_HSTS     (default: true; set "false" for plain-HTTP local use)
//!   WEB_COOKIE_SECURE (auto|true|false; default auto, follows HTTPS)
//!   ALLOW_HTTP_BASIC_OVER_HTTP=1 — required for non-loopback HTTP listen
//!
//! ## Dual-build `#[path]` contract (M6)
//!
//! This example re-includes ~30 modules from `src/` via `#[path = "../src/..."]`
//! rather than going through `lib.rs`, because `lib.rs` is entirely
//! `#![cfg(feature = "desktop")]`-gated and so exposes nothing to a
//! `--no-default-features --features web-server` build. Each `#[path]` line
//! below compiles that `src` module *directly into this example crate*.
//!
//! **Invariant — keep reachable `src` modules `tauri`-free.** Any `src` module
//! reachable from this file (directly, or transitively through another
//! `#[path]`-included module) MUST NOT reference `tauri`/`tauri_plugin_*`, or
//! the web build breaks. There is currently NO default-CI coverage that would
//! catch such a regression — the desktop `cargo clippy`/`cargo test` gates only
//! build the `desktop` feature. The real safety net is wiring this web build
//! (`cargo check --no-default-features --features web-server --example server`)
//! into CI (deep-read finding H4, Batch 7); until then, run it manually after
//! touching any backend module.
//!
//! **Why `app_store` is reimplemented inline below instead of `#[path]`-included:**
//! the desktop `src/app_store.rs` is Tauri-coupled — it persists the
//! `app_config_dir` override through `tauri_plugin_store::StoreExt`, which does
//! not exist in the web build. The inline module persists the *same*
//! `app_paths.json` (so desktop and web read each other's setting) but via plain
//! `std::fs` + `serde_json`. The two impls share the in-memory override cache
//! and `~` path resolution by copy. A proper de-duplication (a shared,
//! `tauri`-free `app_store` core that both runtimes consume) is a follow-up; it
//! is deliberately NOT done here because the persistence backends genuinely
//! differ and restructuring the dual-runtime module mechanism carries more
//! regression risk than the ~25 duplicated lines justify.

#[path = "../src/runtime/mod.rs"]
mod runtime;

#[path = "../src/bootstrap.rs"]
mod bootstrap;

/// Web-runtime reimplementation of `src/app_store.rs`. See the dual-build
/// `#[path]` contract at the top of this file for why this is inline rather
/// than `#[path]`-included: the desktop original is Tauri-`Store`-coupled, so
/// the web build persists the same `app_paths.json` via `std::fs` instead.
mod app_store {
    use std::path::PathBuf;
    use std::sync::{OnceLock, RwLock};

    use crate::error::AppError;

    const STORE_FILE_NAME: &str = "app_paths.json";
    const STORE_KEY_APP_CONFIG_DIR: &str = "app_config_dir_override";

    static APP_CONFIG_DIR_OVERRIDE: OnceLock<RwLock<Option<PathBuf>>> = OnceLock::new();

    fn override_cache() -> &'static RwLock<Option<PathBuf>> {
        APP_CONFIG_DIR_OVERRIDE.get_or_init(|| RwLock::new(None))
    }

    fn update_cached_override(value: Option<PathBuf>) {
        if let Ok(mut guard) = override_cache().write() {
            *guard = value;
        }
    }

    fn store_path() -> PathBuf {
        crate::bootstrap::data_dir().join(STORE_FILE_NAME)
    }

    fn resolve_path(raw: &str) -> PathBuf {
        if raw == "~" {
            if let Some(home) = dirs::home_dir() {
                return home;
            }
        } else if let Some(stripped) = raw.strip_prefix("~/") {
            if let Some(home) = dirs::home_dir() {
                return home.join(stripped);
            }
        } else if let Some(stripped) = raw.strip_prefix("~\\") {
            if let Some(home) = dirs::home_dir() {
                return home.join(stripped);
            }
        }
        PathBuf::from(raw)
    }

    /// Cache-only read, mirroring the desktop `src/app_store.rs` semantics: the
    /// cache is seeded explicitly at startup (`main()` always calls
    /// `set_app_config_dir_override_web` before the router is built), never
    /// lazily from disk. A lazy disk fallback here would let the EXAMPLE TEST
    /// binary read the developer's real `~/.cc-switch/app_paths.json` the first
    /// time any test calls it under the real `$HOME`, cache that override
    /// process-wide, and silently redirect every subsequent test's
    /// `get_app_config_dir()` out of its isolated temp HOME (observed:
    /// `openclaw_config` backup-count tests failing against, and leaking backup
    /// files into, the real `~/.cc-switch`).
    pub fn get_app_config_dir_override() -> Option<PathBuf> {
        override_cache().read().ok()?.clone()
    }

    pub fn set_app_config_dir_override_web(path: Option<&str>) -> Result<(), AppError> {
        let store_path = store_path();
        if let Some(parent) = store_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
        }

        let trimmed = path.map(str::trim).filter(|s| !s.is_empty());
        let mut obj = serde_json::Map::new();
        if let Some(value) = trimmed {
            obj.insert(
                STORE_KEY_APP_CONFIG_DIR.to_string(),
                serde_json::Value::String(value.to_string()),
            );
        }
        crate::config::write_json_file(&store_path, &serde_json::Value::Object(obj))?;
        update_cached_override(trimmed.map(resolve_path));
        Ok(())
    }
}

#[path = "web_proxy.rs"]
mod proxy;

#[path = "../src/app_config.rs"]
mod app_config;
#[path = "../src/claude_mcp.rs"]
mod claude_mcp;
#[path = "../src/claude_plugin.rs"]
mod claude_plugin;
#[path = "../src/codex_config.rs"]
mod codex_config;
#[path = "../src/config.rs"]
mod config;
#[path = "../src/database/mod.rs"]
mod database;
#[cfg(test)]
pub use database::Database;
#[path = "../src/deeplink/mod.rs"]
mod deeplink;
#[path = "../src/services/env_checker.rs"]
mod env_checker;
#[path = "../src/error.rs"]
mod error;
#[path = "../src/gemini_config.rs"]
mod gemini_config;
#[path = "../src/gemini_mcp.rs"]
mod gemini_mcp;
#[path = "../src/hermes_config.rs"]
mod hermes_config;
#[path = "../src/init_status.rs"]
mod init_status;
#[path = "../src/json5_doc.rs"]
mod json5_doc;
#[path = "../src/mcp/mod.rs"]
mod mcp;
#[path = "../src/openclaw_config.rs"]
mod openclaw_config;
#[path = "../src/opencode_config.rs"]
mod opencode_config;
#[path = "../src/prompt.rs"]
mod prompt;
#[path = "../src/prompt_files.rs"]
mod prompt_files;
#[path = "../src/provider.rs"]
mod provider;
#[path = "web_services.rs"]
mod services;
#[path = "../src/session_manager/mod.rs"]
mod session_manager;
#[path = "../src/settings.rs"]
mod settings;
#[path = "../src/store.rs"]
mod store;
#[path = "../src/usage_script.rs"]
mod usage_script;

#[path = "../src/usage_events.rs"]
mod usage_events;

#[path = "../src/web_api/mod.rs"]
mod web_api;

pub use app_config::AppType;

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use tokio::net::TcpListener;
use tokio::signal;
use tokio::sync::RwLock;

use crate::runtime::{ChannelEventSink, UiEventSink};
use crate::web_api::{build_router, ApiState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_logging();

    let host: IpAddr = std::env::var("HOST")
        .ok()
        .as_deref()
        .unwrap_or("127.0.0.1")
        .parse()
        .map_err(|e| format!("invalid HOST: {e}"))?;
    // Default 3010 to stay consistent with `deploy/systemd/cc-switch-web.service`
    // and `scripts/install-cc-switch-web-service.sh` (deep-read finding L15).
    let port: u16 = std::env::var("PORT")
        .ok()
        .as_deref()
        .unwrap_or("3010")
        .parse()
        .map_err(|e| format!("invalid PORT: {e}"))?;
    let addr = SocketAddr::new(host, port);

    if !addr.ip().is_loopback() && std::env::var("ALLOW_HTTP_BASIC_OVER_HTTP").as_deref() != Ok("1")
    {
        log::error!(
            "Refusing to listen on non-loopback {} without ALLOW_HTTP_BASIC_OVER_HTTP=1",
            addr
        );
        return Err("non-loopback bind requires ALLOW_HTTP_BASIC_OVER_HTTP=1".into());
    }

    // Pre-flight: data dir + filesystem type + cross-process lock.
    let data_dir = bootstrap::data_dir();
    bootstrap::check_filesystem_local(&data_dir)
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.into() })?;
    log::info!("Using data directory: {}", data_dir.display());
    let _data_lock = bootstrap::acquire_data_dir_lock(&data_dir)
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.into() })?;
    let data_dir_override = data_dir.to_string_lossy().to_string();
    app_store::set_app_config_dir_override_web(Some(&data_dir_override))
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;

    let db = Arc::new(database::Database::init()?);
    let app_state = Arc::new(store::AppState::new(db));
    let app_config_dir = config::get_app_config_dir();
    let copilot_auth = Arc::new(RwLock::new(
        crate::proxy::providers::copilot_auth::CopilotAuthManager::new(app_config_dir.clone()),
    ));
    let codex_oauth = Arc::new(RwLock::new(
        crate::proxy::providers::codex_oauth_auth::CodexOAuthManager::new(app_config_dir),
    ));

    // Event sink (broadcast for SSE).
    let (channel_sink, _rx) = ChannelEventSink::new(64);
    let events = channel_sink.sender();
    let sink: Arc<dyn UiEventSink> = Arc::new(channel_sink);
    // Register the same sink with `usage_events` so background write paths
    // (UsageLogger, session sync, startup rollup) that don't hold a sink can
    // emit `usage-log-recorded`. In web-server mode this fans out over
    // broadcast -> `GET /api/events` SSE.
    usage_events::init(Arc::clone(&sink));
    let state = ApiState::new(
        Arc::clone(&app_state),
        Arc::clone(&copilot_auth),
        Arc::clone(&codex_oauth),
        Arc::clone(&sink),
        events,
    );

    // ── Proxy lifecycle, step 1/4: runtime ctx injection ─────────────────
    // 注入代理运行时上下文（S3 契约：必须在任何 recovery/restore 之前）。
    // 与 ApiState 共享同一组 Arc：事件经 ChannelEventSink → broadcast →
    // `GET /api/events` SSE；OAuth 管理器与 web_api 处理器共享 token 缓存。
    // 热切换句柄由 set_runtime_ctx 内部自动填入。
    app_state
        .proxy_service
        .set_runtime_ctx(sink, copilot_auth, codex_oauth);

    // ── Proxy lifecycle, step 2/4: crash recovery ────────────────────────
    // 异常退出恢复（镜像桌面 lib.rs setup 的恢复任务，顺序同桌面：
    // recovery → snippets → restore）。systemd 随时可能 SIGKILL/重启本进程，
    // 接管残留（占位符 Live + 备份）必须在对外服务前恢复；recover_from_crash
    // 幂等（无备份时 restore 为 no-op，标志/备份清理可重复执行）。
    recover_proxy_from_crash_residue(app_state.as_ref()).await;

    // Startup parity with the desktop `initialize_common_config_snippets`
    // hook: auto-extract common config snippets from clean live files, run the
    // one-shot legacy common-config migration, and strip legacy
    // `_cc_source` / `provider_key` markers that older
    // `import_hermes_providers_from_live` baked into provider records. All steps
    // are idempotent and gated by their own settings flags, so a successful run
    // never repeats. Must run AFTER crash recovery (so it reads the user's real
    // live configs, not proxy placeholders) and BEFORE takeover restore. See
    // `ProviderService::initialize_common_config_snippets` and lib.rs ordering.
    crate::services::ProviderService::initialize_common_config_snippets(state.app_state.as_ref());

    // ── Proxy lifecycle, step 3/4: takeover restore ──────────────────────
    // 镜像桌面 lib.rs::restore_proxy_state_on_startup：proxy_config.enabled
    // 为 true 的应用重新接管（set_takeover_for_app 内部会自动启动代理）。
    // 与桌面的 spawn 异步不同，这里在监听前内联执行：headless 服务器上
    // 本机 CLI 依赖代理端口，先恢复再 serve 才是确定性顺序。
    restore_proxy_state_on_startup(app_state.as_ref()).await;

    // Build router and bind.
    let app = build_router(state);
    let listener = TcpListener::bind(addr).await?;
    log::info!("cc-switch-web listening on http://{addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    // ── Proxy lifecycle, step 4/4: graceful shutdown ─────────────────────
    // 镜像桌面 lib.rs::cleanup_before_exit：接管残留 → stop_with_restore_keep_state
    //（恢复 Live、保留 enabled 供下次启动自动恢复）；仅运行 → stop()。
    // 必须在 axum::serve 返回之后、释放数据目录 flock 之前执行，确保
    // systemd stop/restart 不会把 PROXY_MANAGED 占位符留在 Live CLI 配置里。
    cleanup_proxy_before_exit(app_state.as_ref()).await;

    log::info!("cc-switch-web stopped cleanly");
    drop(_data_lock); // explicit for clarity
    Ok(())
}

fn init_logging() {
    let env = env_logger::Env::default().filter_or("RUST_LOG", "info,cc_switch=debug");
    let _ = env_logger::Builder::from_env(env).try_init();
}

// ============================================================
// Headless proxy lifecycle (S3) — mirrors desktop `lib.rs`
// ============================================================
//
// The three helpers below are deliberate line-for-line mirrors of the
// desktop lifecycle in `src/lib.rs` (crash-recovery block in `setup()`,
// `restore_proxy_state_on_startup`, `cleanup_before_exit`). They are
// duplicated here rather than shared because the desktop originals live in
// the fully `desktop`-gated `lib.rs` (and `cleanup_before_exit` is
// `AppHandle`-coupled); moving them would create upstream-sync diff noise.
// Keep semantics in lockstep when syncing upstream changes to `lib.rs`.

/// 异常退出恢复（镜像 `lib.rs` setup 内的恢复任务）。
///
/// 存在 Live 备份或 Live 配置仍含接管占位符 ⇒ 上次未走正常退出路径
/// （SIGKILL/OOM/断电），恢复 Live 配置并清理标志与备份。幂等：干净状态下
/// 两个检查均为 false，直接跳过；残留状态下 `recover_from_crash` 收敛到
/// 干净状态（恢复 → 清标志 → 删备份），重复执行无副作用。
async fn recover_proxy_from_crash_residue(state: &store::AppState) {
    let has_backups = match state.db.has_any_live_backup().await {
        Ok(v) => v,
        Err(e) => {
            log::error!("检查 Live 备份失败: {e}");
            false
        }
    };
    let live_taken_over = state.proxy_service.detect_takeover_in_live_configs();

    if has_backups || live_taken_over {
        log::warn!("检测到上次异常退出（存在接管残留），正在恢复 Live 配置...");
        if let Err(e) = state.proxy_service.recover_from_crash().await {
            log::error!("恢复 Live 配置失败: {e}");
        } else {
            log::info!("Live 配置已恢复");
        }
    }
}

/// 启动时根据 proxy_config 表中的代理状态自动恢复代理服务
/// （镜像 `lib.rs::restore_proxy_state_on_startup`）。
///
/// 检查 `proxy_config.enabled` 字段，如果有任一应用的状态为 `true`，
/// 则自动启动代理服务并接管对应应用的 Live 配置；失败时清除该应用的
/// 状态，避免下次启动反复尝试。
async fn restore_proxy_state_on_startup(state: &store::AppState) {
    let mut apps_to_restore = Vec::new();
    for app_type in ["claude", "codex", "gemini"] {
        if let Ok(config) = state.db.get_proxy_config_for_app(app_type).await {
            if config.enabled {
                apps_to_restore.push(app_type);
            }
        }
    }

    if apps_to_restore.is_empty() {
        log::debug!("启动时无需恢复代理状态");
        return;
    }

    log::info!("检测到上次代理状态需要恢复，应用列表: {apps_to_restore:?}");

    for app_type in apps_to_restore {
        match state
            .proxy_service
            .set_takeover_for_app(app_type, true)
            .await
        {
            Ok(()) => {
                log::info!("✓ 已恢复 {app_type} 的代理接管状态");
            }
            Err(e) => {
                log::error!("✗ 恢复 {app_type} 的代理接管状态失败: {e}");
                if let Err(clear_err) = state
                    .proxy_service
                    .set_takeover_for_app(app_type, false)
                    .await
                {
                    log::error!("清除 {app_type} 代理状态失败: {clear_err}");
                }
            }
        }
    }
}

/// 退出前清理（镜像 `lib.rs::cleanup_before_exit`，不含 AppHandle）。
///
/// 接管残留（备份或占位符）⇒ `stop_with_restore_keep_state`：恢复 Live、
/// 删除备份，但保留 `proxy_config.enabled`，下次启动自动重新接管；
/// 无接管但代理在运行 ⇒ 仅 `stop()`。保证 systemd stop/restart 后
/// 本机 CLI 的 Live 配置不会残留 `PROXY_MANAGED` 占位符。
async fn cleanup_proxy_before_exit(state: &store::AppState) {
    let proxy_service = &state.proxy_service;

    let has_backups = match state.db.has_any_live_backup().await {
        Ok(v) => v,
        Err(e) => {
            log::error!("退出时检查 Live 备份失败: {e}");
            false
        }
    };
    let live_taken_over = proxy_service.detect_takeover_in_live_configs();

    if has_backups || live_taken_over {
        log::info!("检测到接管残留，开始恢复 Live 配置（保留代理状态）...");
        if let Err(e) = proxy_service.stop_with_restore_keep_state().await {
            log::error!("退出时恢复 Live 配置失败: {e}");
        } else {
            log::info!("已恢复 Live 配置（代理状态已保留，下次启动将自动恢复）");
        }
        return;
    }

    if proxy_service.is_running().await {
        log::info!("检测到代理服务器正在运行，开始停止...");
        if let Err(e) = proxy_service.stop().await {
            log::error!("退出时停止代理失败: {e}");
        }
        log::info!("代理服务器清理完成");
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = signal::ctrl_c().await {
            log::warn!("ctrl_c handler failed: {err}");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(err) => log::warn!("SIGTERM handler failed: {err}"),
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => log::info!("Received Ctrl-C, shutting down…"),
        _ = terminate => log::info!("Received SIGTERM, shutting down…"),
    }

    // Allow workers a brief moment to drain (Round 2 P1-3).
    tokio::time::sleep(Duration::from_millis(50)).await;
}

/// Dual-runtime module-list drift guard (item 15).
///
/// The web runtime re-includes a HAND-MAINTAINED subset of `src/` modules via
/// `#[path]` shims (`web_proxy.rs`, `web_services.rs`, and this file). Nothing in
/// the compiler enforces that those lists stay in sync with the real `mod.rs`
/// files, so a module added to `src/` can silently go missing from the web build.
///
/// These tests pin the parts of that contract that have a machine-readable signal
/// WITHOUT false positives:
///   * every service module the real `mod.rs` gates into the web runtime
///     (`#[cfg(not(feature = "desktop"))]` / `#[cfg(feature = "web-server")]`)
///     MUST be in the services shim, and `#[cfg(feature = "desktop")]`-only ones
///     MUST NOT be;
///   * every `#[path]` the shims reference MUST resolve to an existing source file.
/// The proxy shim is an intentional curated subset with no cfg signal, so its
/// web-relevance is a judgment call and is only checked for dangling paths.
#[cfg(test)]
mod dual_runtime_parity {
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    fn manifest() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn read(rel: &str) -> String {
        let p = manifest().join(rel);
        std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
    }

    /// Top-level source stems referenced by `#[path = "../src/<subdir>/<stem>(/mod)?.rs"]`
    /// lines in an example shim file.
    fn shim_stems(shim_src: &str, subdir: &str) -> BTreeSet<String> {
        let needle = format!("../src/{subdir}/");
        let mut out = BTreeSet::new();
        for line in shim_src.lines() {
            let line = line.trim();
            if !line.starts_with("#[path") {
                continue;
            }
            let Some(start) = line.find(&needle) else {
                continue;
            };
            let rest = &line[start + needle.len()..];
            let Some(end) = rest.find(".rs\"") else {
                continue;
            };
            let stem = rest[..end].strip_suffix("/mod").unwrap_or(&rest[..end]);
            let top = stem.split('/').next().unwrap_or(stem);
            out.insert(top.to_string());
        }
        out
    }

    /// `(module_name, cfg_tag)` for each `mod`/`pub mod`/`pub(crate) mod NAME;`
    /// declaration in a real `mod.rs`; cfg_tag reflects the immediately preceding
    /// `#[cfg(...)]` line.
    fn mod_decls(mod_src: &str) -> Vec<(String, Option<&'static str>)> {
        let mut out = Vec::new();
        let mut pending: Option<&'static str> = None;
        for line in mod_src.lines() {
            let t = line.trim();
            if t.starts_with("#[cfg(") {
                pending = Some(if t.contains("not(feature = \"desktop\")") {
                    "not-desktop"
                } else if t.contains("feature = \"web-server\"") {
                    "web-server"
                } else if t.contains("feature = \"desktop\"") {
                    "desktop"
                } else {
                    "other"
                });
                continue;
            }
            let decl = t
                .strip_prefix("pub(crate) mod ")
                .or_else(|| t.strip_prefix("pub mod "))
                .or_else(|| t.strip_prefix("mod "));
            if let Some(rest) = decl {
                let taken = pending.take();
                if let Some(name) = rest.strip_suffix(';') {
                    out.push((name.trim().to_string(), taken));
                }
                continue;
            }
            if !t.is_empty() && !t.starts_with("#[") && !t.starts_with("//") {
                pending = None;
            }
        }
        out
    }

    #[test]
    fn web_services_shim_covers_web_cfg_gated_service_modules() {
        let modrs = read("src/services/mod.rs");
        let included = shim_stems(&read("examples/web_services.rs"), "services");
        for (name, cfg) in mod_decls(&modrs) {
            match cfg {
                Some("not-desktop") | Some("web-server") => assert!(
                    included.contains(&name),
                    "src/services/mod.rs gates web-runtime module `{name}` (cfg {cfg:?}) but \
                     examples/web_services.rs does not #[path]-include it — dual-runtime drift (item 15)"
                ),
                Some("desktop") => assert!(
                    !included.contains(&name),
                    "examples/web_services.rs includes desktop-only module `{name}` — must not be in the web shim (item 15)"
                ),
                _ => {} // unconditional modules: web inclusion is a curated judgment call
            }
        }
    }

    #[test]
    fn shim_path_includes_resolve_to_existing_sources() {
        for (subdir, shim) in [
            ("proxy", "examples/web_proxy.rs"),
            ("services", "examples/web_services.rs"),
        ] {
            let base = manifest().join("src").join(subdir);
            for stem in shim_stems(&read(shim), subdir) {
                assert!(
                    base.join(format!("{stem}.rs")).is_file()
                        || base.join(&stem).join("mod.rs").is_file(),
                    "{shim} includes `{stem}` but neither src/{subdir}/{stem}.rs nor \
                     src/{subdir}/{stem}/mod.rs exists — dual-runtime drift (item 15)"
                );
            }
        }
    }
}

/// Headless proxy lifecycle tests (S3).
///
/// Covers the parts of the S3 contract that are unit-testable inside the
/// example test binary:
///   * the `main()` startup/shutdown ordering pin (ctx injection BEFORE
///     recovery, recovery BEFORE snippets BEFORE restore, cleanup AFTER
///     `axum::serve` returns) — textual, since `main()` cannot run in tests;
///   * crash recovery converging (and staying converged) from takeover residue;
///   * graceful-shutdown cleanup restoring live configs while keeping
///     `proxy_config.enabled`, and the follow-up boot re-takeover;
///   * cleanup stopping a running proxy that has no takeover.
/// What only `pnpm smoke:web-server` / integration can cover: real
/// SIGINT/SIGTERM delivery through `shutdown_signal()` and the flock-held
/// process-exit path.
#[cfg(test)]
mod web_proxy_lifecycle {
    use std::env;
    use std::sync::Arc;

    use serde_json::json;
    use serial_test::serial;
    use tempfile::TempDir;

    use crate::app_config::AppType;
    use crate::database::Database;
    use crate::provider::Provider;

    /// HOME/CC_SWITCH_TEST_HOME swap, mirroring `services/proxy.rs` tests
    /// (same `#[serial]` key keeps env mutation race-free across the binary).
    struct TempHome {
        #[allow(dead_code)]
        dir: TempDir,
        original_home: Option<String>,
        original_userprofile: Option<String>,
        original_test_home: Option<String>,
    }

    impl TempHome {
        fn new() -> Self {
            let dir = TempDir::new().expect("failed to create temp home");
            let original_home = env::var("HOME").ok();
            let original_userprofile = env::var("USERPROFILE").ok();
            let original_test_home = env::var("CC_SWITCH_TEST_HOME").ok();

            env::set_var("HOME", dir.path());
            env::set_var("USERPROFILE", dir.path());
            env::set_var("CC_SWITCH_TEST_HOME", dir.path());

            Self {
                dir,
                original_home,
                original_userprofile,
                original_test_home,
            }
        }
    }

    impl Drop for TempHome {
        fn drop(&mut self) {
            match &self.original_home {
                Some(value) => env::set_var("HOME", value),
                None => env::remove_var("HOME"),
            }
            match &self.original_userprofile {
                Some(value) => env::set_var("USERPROFILE", value),
                None => env::remove_var("USERPROFILE"),
            }
            match &self.original_test_home {
                Some(value) => env::set_var("CC_SWITCH_TEST_HOME", value),
                None => env::remove_var("CC_SWITCH_TEST_HOME"),
            }
        }
    }

    fn read_server_source() -> String {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/server.rs");
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
    }

    async fn fresh_state() -> (Arc<crate::store::AppState>, Arc<Database>) {
        let db = Arc::new(Database::memory().expect("init db"));
        // Ephemeral port: lifecycle tests start a real proxy listener and must
        // not collide on the default 15721 (or with a developer's instance).
        let mut proxy_config = db.get_proxy_config().await.expect("get proxy config");
        proxy_config.listen_port = 0;
        db.update_proxy_config(proxy_config)
            .await
            .expect("set ephemeral proxy port");
        let state = Arc::new(crate::store::AppState::new(db.clone()));
        (state, db)
    }

    fn seed_claude_provider_and_live(db: &Arc<Database>) {
        let provider = Provider::with_id(
            "p1".to_string(),
            "P1".to_string(),
            json!({
                "env": {
                    "ANTHROPIC_API_KEY": "provider-key",
                    "ANTHROPIC_BASE_URL": "https://api.anthropic.com"
                }
            }),
            None,
        );
        db.save_provider("claude", &provider)
            .expect("save provider");
        db.set_current_provider("claude", "p1")
            .expect("set db current provider");
        crate::settings::set_current_provider(&AppType::Claude, Some("p1"))
            .expect("set local current provider");
        crate::config::write_json_file(
            &crate::config::get_claude_settings_path(),
            &json!({
                "env": {
                    "ANTHROPIC_API_KEY": "live-key",
                    "ANTHROPIC_BASE_URL": "https://api.anthropic.com"
                }
            }),
        )
        .expect("seed claude live config");
    }

    fn claude_live_env_value(key: &str) -> Option<String> {
        let live: serde_json::Value =
            crate::config::read_json_file(&crate::config::get_claude_settings_path())
                .expect("read claude live config");
        live.get("env")
            .and_then(|env| env.get(key))
            .and_then(|v| v.as_str())
            .map(str::to_string)
    }

    /// S3 contract pin: `main()` cannot run under the test harness, so the
    /// startup/shutdown ordering is pinned textually (same spirit as
    /// `dual_runtime_parity`): ctx injection → crash recovery → common-config
    /// snippets → takeover restore → serve → shutdown cleanup.
    #[test]
    fn main_pins_proxy_lifecycle_ordering() {
        let source = read_server_source();
        let main_start = source
            .find("async fn main")
            .expect("examples/server.rs must define async fn main");
        let main_end = source
            .find("\nfn init_logging")
            .expect("examples/server.rs must define fn init_logging after main");
        let main_body = &source[main_start..main_end];

        let markers = [
            ".set_runtime_ctx(",
            "recover_proxy_from_crash_residue(",
            "initialize_common_config_snippets(",
            "restore_proxy_state_on_startup(",
            "axum::serve(",
            "cleanup_proxy_before_exit(",
        ];
        let mut last = 0usize;
        for marker in markers {
            let idx = main_body.find(marker).unwrap_or_else(|| {
                panic!("main() must call `{marker}…` (S3 proxy lifecycle contract)")
            });
            assert!(
                idx > last,
                "S3 lifecycle ordering violated: `{marker}` must come after the previous step in main()"
            );
            last = idx;
        }
    }

    #[tokio::test]
    #[serial]
    async fn crash_recovery_restores_live_and_is_idempotent() {
        let _home = TempHome::new();
        crate::settings::reload_settings().expect("reload settings");
        let (state, db) = fresh_state().await;

        // Simulate a SIGKILL'd takeover: live config holds the proxy
        // placeholder, db still holds the pre-takeover backup.
        crate::config::write_json_file(
            &crate::config::get_claude_settings_path(),
            &json!({
                "env": {
                    "ANTHROPIC_AUTH_TOKEN": "PROXY_MANAGED",
                    "ANTHROPIC_BASE_URL": "http://127.0.0.1:15721"
                }
            }),
        )
        .expect("seed taken-over claude live config");
        let original = json!({
            "env": {
                "ANTHROPIC_AUTH_TOKEN": "real-token",
                "ANTHROPIC_BASE_URL": "https://api.anthropic.com"
            }
        });
        db.save_live_backup("claude", &original.to_string())
            .await
            .expect("seed live backup");

        super::recover_proxy_from_crash_residue(state.as_ref()).await;

        assert_eq!(
            claude_live_env_value("ANTHROPIC_AUTH_TOKEN").as_deref(),
            Some("real-token"),
            "crash recovery must restore the backed-up live config"
        );
        assert!(
            !db.has_any_live_backup().await.expect("check backups"),
            "crash recovery must delete consumed backups"
        );
        assert!(
            !state.proxy_service.detect_takeover_in_live_configs(),
            "no takeover residue may remain after recovery"
        );

        // Idempotency (systemd restarts run this on every boot): a second run
        // on the converged state must be a no-op.
        super::recover_proxy_from_crash_residue(state.as_ref()).await;
        assert_eq!(
            claude_live_env_value("ANTHROPIC_AUTH_TOKEN").as_deref(),
            Some("real-token"),
            "re-running crash recovery on a clean state must not alter live configs"
        );
    }

    #[tokio::test]
    #[serial]
    async fn cleanup_before_exit_restores_live_keeps_enabled_and_next_boot_retakes_over() {
        let _home = TempHome::new();
        crate::settings::reload_settings().expect("reload settings");
        let (state, db) = fresh_state().await;
        seed_claude_provider_and_live(&db);

        // Takeover (auto-starts the proxy server on the ephemeral port).
        state
            .proxy_service
            .set_takeover_for_app("claude", true)
            .await
            .expect("enable claude takeover");
        assert!(state.proxy_service.is_running().await);
        assert!(state.proxy_service.detect_takeover_in_live_configs());

        // Graceful shutdown path: restore live, keep enabled flag.
        super::cleanup_proxy_before_exit(state.as_ref()).await;

        assert!(
            !state.proxy_service.is_running().await,
            "cleanup must stop the proxy server"
        );
        assert!(
            !state.proxy_service.detect_takeover_in_live_configs(),
            "cleanup must not leave PROXY_MANAGED placeholders in live configs"
        );
        assert_eq!(
            claude_live_env_value("ANTHROPIC_API_KEY").as_deref(),
            Some("live-key"),
            "cleanup must restore the original live config"
        );
        assert!(
            !db.has_any_live_backup().await.expect("check backups"),
            "cleanup must delete consumed backups"
        );
        let claude_config = db
            .get_proxy_config_for_app("claude")
            .await
            .expect("get claude proxy config");
        assert!(
            claude_config.enabled,
            "cleanup must KEEP proxy_config.enabled so the next boot auto-restores"
        );

        // Next boot: restore_proxy_state_on_startup re-takes over claude.
        super::restore_proxy_state_on_startup(state.as_ref()).await;
        assert!(
            state.proxy_service.is_running().await,
            "startup restore must auto-start the proxy for enabled apps"
        );
        assert!(
            state.proxy_service.detect_takeover_in_live_configs(),
            "startup restore must re-take over the live config"
        );

        // Leave the temp home clean for the other serial tests.
        super::cleanup_proxy_before_exit(state.as_ref()).await;
        assert!(!state.proxy_service.detect_takeover_in_live_configs());
    }

    #[tokio::test]
    #[serial]
    async fn cleanup_before_exit_stops_running_proxy_without_takeover() {
        let _home = TempHome::new();
        crate::settings::reload_settings().expect("reload settings");
        let (state, _db) = fresh_state().await;

        state
            .proxy_service
            .start()
            .await
            .expect("start proxy without takeover");
        assert!(state.proxy_service.is_running().await);

        super::cleanup_proxy_before_exit(state.as_ref()).await;
        assert!(
            !state.proxy_service.is_running().await,
            "cleanup must stop a takeover-less running proxy"
        );
    }
}
