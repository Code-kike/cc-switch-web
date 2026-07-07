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
//!   HOST            (default: 127.0.0.1) — a non-loopback bind REQUIRES
//!                   CC_SWITCH_WEB_AUTH_PASSWORD (audit C2)
//!   PORT            (default: 3010 — matches the systemd unit + install script)
//!   CC_SWITCH_DATA_DIR (default: ~/.cc-switch) — used by bootstrap::data_dir
//!   CC_SWITCH_WEB_AUTH_PASSWORD — enables HTTP Basic Auth on /api/* (REQUIRED for
//!                   any non-loopback bind, e.g. the Tailscale deployment)
//!   CC_SWITCH_WEB_AUTH_USER (default: cc-switch) — Basic Auth username
//!   CORS_ALLOW_ORIGINS (comma-separated, optional)
//!   ENABLE_HSTS     (default: true; set "false" for plain-HTTP local use)
//!   WEB_COOKIE_SECURE (auto|true|false; default auto, follows HTTPS)
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

    // Audit fix C2: never expose the unauthenticated API on a non-loopback address.
    // The Web API can read provider secrets, import/export the SQLite config, and
    // toggle proxy takeover, so a non-loopback bind REQUIRES Basic Auth credentials
    // (CC_SWITCH_WEB_AUTH_PASSWORD). Loopback may run open for local/dev use.
    if !addr.ip().is_loopback() && !web_api::middleware::auth::is_configured() {
        log::error!(
            "Refusing to listen on non-loopback {addr} without web auth. Set \
             CC_SWITCH_WEB_AUTH_PASSWORD (and optionally CC_SWITCH_WEB_AUTH_USER) to \
             enable HTTP Basic Auth, or bind a loopback HOST."
        );
        return Err("non-loopback bind requires CC_SWITCH_WEB_AUTH_PASSWORD".into());
    }
    if web_api::middleware::auth::is_configured() {
        log::info!("Web API Basic Auth enabled (CC_SWITCH_WEB_AUTH_PASSWORD set)");
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

    // ── Legacy config.json → SQLite migration (audit F5) ────────────────
    // Mirror the desktop `lib.rs` migration, MINUS the desktop-only dialog /
    // retry / process::exit UX: a headless server must always come up. So when
    // a legacy `config.json` exists and no DB is present yet, load + migrate +
    // archive it BEFORE creating the DB row that would otherwise suppress a
    // future desktop migration. On load failure we log and continue with the
    // fresh empty DB rather than hang or exit.
    let app_config_dir = config::get_app_config_dir();
    let json_path = app_config_dir.join("config.json");
    let db_path = app_config_dir.join("cc-switch.db");
    let migration_config = if !db_path.exists() && json_path.exists() {
        log::info!("检测到旧版配置文件，验证配置文件...");
        match app_config::MultiAppConfig::load() {
            Ok(config) => {
                log::info!("✓ 配置文件加载成功");
                Some(config)
            }
            Err(e) => {
                // No dialog in headless mode: log and continue with an empty DB.
                log::error!("加载旧配置文件失败，将以空数据库继续启动: {e}");
                None
            }
        }
    } else {
        None
    };

    // Refuse too-new databases before Database::init() can create tables or run
    // migrations. The headless Web server cannot build the full API state
    // without a compatible DB, so startup blocks with explicit recovery
    // guidance rather than writing to a schema it does not understand.
    if let Some(version) = database::Database::stored_user_version_exceeds_supported(&db_path)? {
        let message = format!(
            "database version is too new (stored user_version={version}, supported={}); \
             upgrade cc-switch-web before starting this data directory; database left untouched: {}",
            database::SCHEMA_VERSION,
            db_path.display()
        );
        init_status::set_init_error(init_status::InitErrorPayload {
            path: db_path.display().to_string(),
            error: message.clone(),
            kind: Some("db_version_too_new".to_string()),
            db_version: Some(version),
            supported_version: Some(database::SCHEMA_VERSION),
        });
        return Err(message.into());
    }

    let db = Arc::new(database::Database::init()?);

    if let Some(config) = migration_config {
        bootstrap::apply_legacy_json_migration(&db, &config, &json_path);
    }

    let app_state = Arc::new(store::AppState::new(db));

    // ── Post-DB bootstrap (audit F6) ────────────────────────────────────
    // Seed default Skills repos + official providers and auto-import live CLI
    // config / OMO / MCP / prompts, in parity with desktop `lib.rs`. Every step
    // is idempotent (table-empty gated), so re-running on each systemd boot is a
    // no-op. Runs after AppState::new and BEFORE set_runtime_ctx (matches
    // desktop, which bootstraps before the proxy lifecycle).
    bootstrap::run_post_db_bootstrap(app_state.as_ref());

    // ── Background workers (audit FIX 6) ─────────────────────────────────
    // The long-running headless server must not rely on lazy on-request work
    // for DB durability + usage freshness. Mirror the desktop `lib.rs` startup
    // workers that are tauri-free: periodic DB backup (initial + daily) and
    // the session-usage sync loop (initial + 60s). The WebDAV/S3 auto-sync
    // workers are intentionally NOT started here — they require a `tauri::
    // AppHandle` to emit their `*-sync-status-updated` events (the web build
    // only has the no-op `*_auto_sync_web` stubs), and the web frontend already
    // gates the auto-sync toggles as desktop-only (audit F10). Manual
    // upload/download/test sync stays available via the web API.
    spawn_background_workers(app_state.as_ref());

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

    // ── Proxy lifecycle, step 1/5: runtime ctx injection ─────────────────
    // 注入代理运行时上下文（S3 契约：必须在任何 recovery/restore 之前）。
    // 与 ApiState 共享同一组 Arc：事件经 ChannelEventSink → broadcast →
    // `GET /api/events` SSE；OAuth 管理器与 web_api 处理器共享 token 缓存。
    // 热切换句柄由 set_runtime_ctx 内部自动填入。
    app_state
        .proxy_service
        .set_runtime_ctx(sink, copilot_auth, codex_oauth);

    // ── Proxy lifecycle, step 2/5: global outbound proxy client init ─────
    // 镜像桌面 lib.rs setup 的「初始化全局出站代理 HTTP 客户端」块：从数据库
    // 读取已保存的全局代理 URL 并初始化 http_client。必须在接管恢复
    //（takeover restore）之前——恢复出来的代理要立即按用户的出站代理转发；
    // 缺了这一步 forwarder 走 GP-004 直连回退，依赖本地出站代理（如 Clash）
    // 的中转商全部不可达，每个故障转移候选都会失败。
    init_global_proxy_http_client(app_state.as_ref());

    // ── Proxy lifecycle, step 3/5: crash recovery ────────────────────────
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

    // ── Proxy lifecycle, step 4/5: takeover restore ──────────────────────
    // 镜像桌面 lib.rs::restore_proxy_state_on_startup：proxy_config.enabled
    // 为 true 的应用重新接管（set_takeover_for_app 内部会自动启动代理）。
    // 与桌面的 spawn 异步不同，这里在监听前内联执行：headless 服务器上
    // 本机 CLI 依赖代理端口，先恢复再 serve 才是确定性顺序。
    restore_proxy_state_on_startup(app_state.as_ref()).await;

    // Build router and bind.
    let app = build_router(state);
    let listener = TcpListener::bind(addr).await?;
    log::info!("cc-switch-web listening on http://{addr}");

    // Graceful shutdown with a BOUNDED connection-drain window.
    //
    // `axum::serve(..).with_graceful_shutdown(..)` waits for ALL in-flight
    // connections to finish — but `GET /api/events` is an infinite SSE stream
    // (its broadcast senders live for the whole process), so any open web-UI
    // tab keeps `serve` alive forever after SIGTERM. Verified empirically:
    // with one `curl -N /api/events` attached, SIGTERM left the old
    // `serve(...).await?` hanging until the client disconnected — under
    // systemd that means TimeoutStopSec → SIGKILL and `cleanup_proxy_before_exit`
    // NEVER runs, leaving PROXY_MANAGED placeholders in the live CLI configs
    // (the exact failure S3 exists to prevent). So: after the shutdown signal
    // fires, give in-flight requests SHUTDOWN_CONNECTION_GRACE to finish, then
    // proceed to cleanup regardless. Lingering connection tasks die with the
    // process; browsers' EventSource auto-reconnects on the next boot.
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    tokio::spawn(async move {
        shutdown_signal().await;
        let _ = shutdown_tx.send(true);
    });
    let serve_future = axum::serve(listener, app).with_graceful_shutdown({
        let mut rx = shutdown_rx.clone();
        async move {
            let _ = rx.changed().await;
        }
    });
    tokio::select! {
        result = serve_future => result?,
        _ = async {
            let mut rx = shutdown_rx.clone();
            let _ = rx.changed().await;
            tokio::time::sleep(SHUTDOWN_CONNECTION_GRACE).await;
        } => {
            log::warn!(
                "graceful shutdown grace ({SHUTDOWN_CONNECTION_GRACE:?}) elapsed with connections \
                 still open (long-lived SSE clients); proceeding to proxy cleanup"
            );
        }
    }

    // ── Proxy lifecycle, step 5/5: graceful shutdown ─────────────────────
    // 镜像桌面 lib.rs::cleanup_before_exit：接管残留 → stop_with_restore_keep_state
    //（恢复 Live、保留 enabled 供下次启动自动恢复）；仅运行 → stop()。
    // 必须在 axum::serve 返回之后（或宽限期到期后）、释放数据目录 flock 之前
    // 执行，确保 systemd stop/restart 不会把 PROXY_MANAGED 占位符留在 Live
    // CLI 配置里。
    cleanup_proxy_before_exit(app_state.as_ref()).await;

    log::info!("cc-switch-web stopped cleanly");
    drop(_data_lock); // explicit for clarity
    Ok(())
}

fn init_logging() {
    let env = env_logger::Env::default().filter_or("RUST_LOG", "info,cc_switch=debug");
    let _ = env_logger::Builder::from_env(env).try_init();
}

/// Period between automatic DB backup checks (mirrors desktop `lib.rs`
/// `PERIODIC_MAINTENANCE_INTERVAL_SECS`).
const PERIODIC_MAINTENANCE_INTERVAL_SECS: u64 = 24 * 60 * 60;
/// Period between session-usage syncs (mirrors desktop `lib.rs`
/// `SESSION_SYNC_INTERVAL_SECS`).
const SESSION_SYNC_INTERVAL_SECS: u64 = 60;

/// Spawn the tauri-free desktop-parity background workers on the headless web
/// server (audit FIX 6): periodic DB backup (initial check + daily timer) and
/// the session-usage sync loop (initial + every 60s). These mirror the desktop
/// `lib.rs` startup workers; only the tauri-free ones are ported here (the
/// WebDAV/S3 auto-sync workers need an `AppHandle` and are intentionally
/// skipped — see the call site).
fn spawn_background_workers(state: &store::AppState) {
    // Periodic DB backup: run once now, then once per day.
    let db_for_backup = state.db.clone();
    tokio::spawn(async move {
        if let Err(e) = db_for_backup.periodic_backup_if_needed() {
            log::warn!("Periodic backup failed on startup: {e}");
        }
        let mut interval =
            tokio::time::interval(Duration::from_secs(PERIODIC_MAINTENANCE_INTERVAL_SECS));
        interval.tick().await; // skip the immediate first tick (already checked)
        loop {
            interval.tick().await;
            if let Err(e) = db_for_backup.periodic_backup_if_needed() {
                log::warn!("Periodic maintenance timer failed: {e}");
            }
        }
    });

    // Session-usage sync: run once now, then every 60s. Without this the web
    // server only synced usage lazily on `GET /api/usage`.
    let db_for_session_sync = state.db.clone();
    tokio::spawn(async move {
        run_session_usage_sync(&db_for_session_sync, "initial");
        let mut interval = tokio::time::interval(Duration::from_secs(SESSION_SYNC_INTERVAL_SECS));
        interval.tick().await; // skip the immediate first tick
        loop {
            interval.tick().await;
            run_session_usage_sync(&db_for_session_sync, "periodic");
        }
    });
}

/// Run one round of the four session-usage syncs (Claude / Codex / Gemini /
/// OpenCode), logging each failure without aborting the others.
fn run_session_usage_sync(db: &database::Database, phase: &str) {
    if let Err(e) = crate::services::session_usage::sync_claude_session_logs(db) {
        log::warn!("Session usage {phase} sync failed: {e}");
    }
    if let Err(e) = crate::services::session_usage_codex::sync_codex_usage(db) {
        log::warn!("Codex usage {phase} sync failed: {e}");
    }
    if let Err(e) = crate::services::session_usage_gemini::sync_gemini_usage(db) {
        log::warn!("Gemini usage {phase} sync failed: {e}");
    }
    if let Err(e) = crate::services::session_usage_opencode::sync_opencode_usage(db) {
        log::warn!("OpenCode usage {phase} sync failed: {e}");
    }
}

// ============================================================
// Headless proxy lifecycle (S3) — mirrors desktop `lib.rs`
// ============================================================
//
// The four helpers below are deliberate line-for-line mirrors of the
// desktop lifecycle in `src/lib.rs` (global-proxy init block and
// crash-recovery block in `setup()`, `restore_proxy_state_on_startup`,
// `cleanup_before_exit`). They are duplicated here rather than shared
// because the desktop originals live in the fully `desktop`-gated `lib.rs`
// (and `cleanup_before_exit` is `AppHandle`-coupled); moving them would
// create upstream-sync diff noise. Keep semantics in lockstep when syncing
// upstream changes to `lib.rs`.

/// 初始化全局出站代理 HTTP 客户端
/// （镜像 `lib.rs` setup 内的同名块，GP-00x 日志码一致）。
///
/// 从数据库读取已保存的全局代理 URL 并初始化 `http_client`；保存的配置
/// 无效时（GP-005）清除数据库中的无效配置（GP-006/GP-007）并以直连模式
/// 重新初始化（GP-008）。
fn init_global_proxy_http_client(state: &store::AppState) {
    let proxy_url = state.db.get_global_proxy_url().ok().flatten();

    if let Err(e) = crate::proxy::http_client::init(proxy_url.as_deref()) {
        log::error!("[GlobalProxy] [GP-005] Failed to initialize with saved config: {e}");

        // 清除无效的代理配置
        if proxy_url.is_some() {
            log::warn!("[GlobalProxy] [GP-006] Clearing invalid proxy config from database");
            if let Err(clear_err) = state.db.set_global_proxy_url(None) {
                log::error!("[GlobalProxy] [GP-007] Failed to clear invalid config: {clear_err}");
            }
        }

        // 使用直连模式重新初始化
        if let Err(fallback_err) = crate::proxy::http_client::init(None) {
            log::error!(
                "[GlobalProxy] [GP-008] Failed to initialize direct connection: {fallback_err}"
            );
        }
    }
}

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

/// Maximum time to wait for in-flight HTTP connections after the shutdown
/// signal before proceeding to proxy cleanup anyway. Long-lived SSE clients
/// (`GET /api/events`) never finish on their own, so an unbounded graceful
/// wait would block `cleanup_proxy_before_exit` until systemd SIGKILLs us.
/// Must stay comfortably below the systemd unit's `TimeoutStopSec` (30s)
/// minus the proxy stop timeout (5s) and live-config restore writes.
const SHUTDOWN_CONNECTION_GRACE: Duration = Duration::from_secs(5);

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
///   * every `#[path]` the shims reference MUST resolve to an existing source file;
///   * since the 06-11 web proxy port, `examples/web_proxy.rs` is a 1:1 mirror
///     of `src/proxy/mod.rs` (the web runtime compiles the SAME proxy hot path
///     as desktop), so its module list is pinned to set-equality — the inline
///     `CircuitBreakerConfig` duplicate this replaced was a silent-divergence
///     hazard, and a partial shim would be the same hazard again.
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

    /// 06-11 web proxy port: `examples/web_proxy.rs` mirrors `src/proxy/mod.rs`
    /// 1:1 (the web runtime compiles the SAME proxy hot path + failover engine
    /// as desktop). Pin the module lists to set-equality so a module added to
    /// `src/proxy/mod.rs` can never silently go missing from the web build
    /// (and a future desktop-only proxy module must be cfg-gated, which this
    /// test then excludes from the mirror requirement).
    #[test]
    fn web_proxy_shim_mirrors_proxy_mod_modules() {
        let included = shim_stems(&read("examples/web_proxy.rs"), "proxy");
        let mut expected = BTreeSet::new();
        for (name, cfg) in mod_decls(&read("src/proxy/mod.rs")) {
            match cfg {
                Some("desktop") => assert!(
                    !included.contains(&name),
                    "examples/web_proxy.rs includes desktop-only proxy module `{name}` — \
                     must not be in the web shim (item 15)"
                ),
                _ => {
                    expected.insert(name);
                }
            }
        }
        assert_eq!(
            included, expected,
            "examples/web_proxy.rs must #[path]-include exactly the modules declared in \
             src/proxy/mod.rs (web runtime compiles the full proxy tree since 06-11) — \
             dual-runtime drift (item 15)"
        );
    }
}

/// Headless proxy lifecycle tests (S3).
///
/// Covers the parts of the S3 contract that are unit-testable inside the
/// example test binary:
///   * the `main()` startup/shutdown ordering pin (ctx injection BEFORE
///     global-proxy init, init BEFORE recovery, recovery BEFORE snippets
///     BEFORE restore, cleanup AFTER `axum::serve` returns) — textual, since
///     `main()` cannot run in tests;
///   * startup global-proxy init loading the saved DB proxy URL into
///     `http_client` (and the invalid-config clear + direct fallback);
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
    /// `dual_runtime_parity`): ctx injection → global-proxy init → crash
    /// recovery → common-config snippets → takeover restore → serve →
    /// shutdown cleanup. Global-proxy init must precede takeover restore:
    /// a restored proxy must forward through the user's saved outbound
    /// proxy immediately (the missing init was the GP-004-at-forward-time
    /// regression that made every relay failover candidate fail).
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
            "init_global_proxy_http_client(",
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

    /// FIX 6 pin: `main()` must spawn the desktop-parity background workers
    /// (periodic DB backup + session-usage sync) after the post-DB bootstrap,
    /// so the headless server keeps the SQLite DB backed up and usage fresh
    /// without relying on lazy on-request work.
    #[test]
    fn main_spawns_background_workers_after_bootstrap() {
        let source = read_server_source();
        let main_start = source
            .find("async fn main")
            .expect("examples/server.rs must define async fn main");
        let main_end = source
            .find("\nfn init_logging")
            .expect("examples/server.rs must define fn init_logging after main");
        let main_body = &source[main_start..main_end];

        let bootstrap_idx = main_body
            .find("run_post_db_bootstrap(")
            .expect("main() must run the post-DB bootstrap");
        let workers_idx = main_body
            .find("spawn_background_workers(")
            .expect("main() must spawn background workers (FIX 6)");
        assert!(
            workers_idx > bootstrap_idx,
            "spawn_background_workers must run AFTER run_post_db_bootstrap"
        );
        assert!(
            source.contains("periodic_backup_if_needed()"),
            "background workers must include the periodic DB backup"
        );
        assert!(
            source.contains("run_session_usage_sync("),
            "background workers must include the session-usage sync loop"
        );
    }

    /// S3 contract pin: the graceful-shutdown wait must be BOUNDED.
    ///
    /// `GET /api/events` SSE streams never finish (their broadcast senders are
    /// process-lifetime), so an unbounded `with_graceful_shutdown(..).await`
    /// hangs forever while a web-UI tab is open — empirically verified: SIGTERM
    /// with one attached SSE client never reached `cleanup_proxy_before_exit`,
    /// which under systemd means TimeoutStopSec → SIGKILL with PROXY_MANAGED
    /// placeholders left in the live CLI configs. Pin that main() races serve
    /// against the shutdown signal + SHUTDOWN_CONNECTION_GRACE timer.
    #[test]
    fn main_bounds_graceful_shutdown_with_connection_grace() {
        let source = read_server_source();
        let main_start = source
            .find("async fn main")
            .expect("examples/server.rs must define async fn main");
        let main_end = source
            .find("\nfn init_logging")
            .expect("examples/server.rs must define fn init_logging after main");
        let main_body = &source[main_start..main_end];

        let serve_idx = main_body
            .find("axum::serve(")
            .expect("main() must call axum::serve");
        let tail = &main_body[serve_idx..];
        assert!(
            tail.contains("tokio::select!"),
            "main() must race axum::serve against a bounded shutdown branch \
             (SSE connections never finish; cleanup must still run)"
        );
        assert!(
            tail.contains("SHUTDOWN_CONNECTION_GRACE"),
            "the shutdown branch must bound the connection drain with \
             SHUTDOWN_CONNECTION_GRACE before running cleanup_proxy_before_exit"
        );
        assert!(
            crate::SHUTDOWN_CONNECTION_GRACE < std::time::Duration::from_secs(20),
            "SHUTDOWN_CONNECTION_GRACE must leave headroom below the systemd \
             TimeoutStopSec=30 window (proxy stop timeout 5s + restore writes)"
        );
    }

    /// Startup global-proxy init: a saved DB proxy URL must be loaded into
    /// the process-wide `http_client` (this is exactly the missing-init gap
    /// behind the GP-004-at-forward-time regression: the web runtime never
    /// initialized the client, so forwarding silently ignored the user's
    /// saved outbound proxy), and an invalid saved config must be cleared
    /// from the DB with a direct-connection re-init (lib.rs GP-005..GP-008
    /// mirror).
    ///
    /// `http_client` state is process-global (`OnceCell`), so this test is
    /// `#[serial]` like the other lifecycle tests and converges the global
    /// state back to direct connection before returning (the invalid-config
    /// leg ends on the GP-008 direct fallback).
    #[tokio::test]
    #[serial]
    async fn global_proxy_init_loads_saved_url_and_clears_invalid_config() {
        let _home = TempHome::new();
        crate::settings::reload_settings().expect("reload settings");
        let (state, db) = fresh_state().await;

        // Valid saved config: init must surface it process-wide so the
        // forwarder's `get_current_proxy_url()` sees it on the first request.
        db.set_global_proxy_url(Some("http://127.0.0.1:7890"))
            .expect("save global proxy url");
        super::init_global_proxy_http_client(state.as_ref());
        assert_eq!(
            crate::proxy::http_client::get_current_proxy_url().as_deref(),
            Some("http://127.0.0.1:7890"),
            "startup init must load the saved global proxy URL into http_client"
        );

        // Invalid saved config: GP-005 path must clear the DB row (GP-006)
        // and re-init direct (GP-008) instead of leaving a stale proxy.
        db.set_global_proxy_url(Some("invalid-scheme://127.0.0.1:7890"))
            .expect("save invalid global proxy url");
        super::init_global_proxy_http_client(state.as_ref());
        assert_eq!(
            db.get_global_proxy_url().expect("read global proxy url"),
            None,
            "invalid saved proxy config must be cleared from the database"
        );
        assert_eq!(
            crate::proxy::http_client::get_current_proxy_url(),
            None,
            "after clearing an invalid config the client must fall back to direct connection"
        );
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
