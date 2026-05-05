//! Standalone Web server entry point.
//!
//! Layer 2 / Task 4. Bootstraps the shared core (`bootstrap::init_core_state`),
//! mounts the 28 web_api handlers, and listens on `HOST:PORT` (defaults
//! `127.0.0.1:3000`). Graceful shutdown on SIGINT/SIGTERM.
//!
//! Run with:
//!   cargo run --no-default-features --features web-server --example server
//!
//! Environment variables:
//!   HOST            (default: 127.0.0.1) — refuse non-loopback unless
//!                   ALLOW_HTTP_BASIC_OVER_HTTP=1
//!   PORT            (default: 3000)
//!   CC_SWITCH_DATA_DIR (default: ~/.cc-switch) — used by bootstrap::data_dir
//!   CORS_ALLOW_ORIGINS (comma-separated, optional)
//!   ENABLE_HSTS     (default: true; set "false" for plain-HTTP local use)
//!   WEB_COOKIE_SECURE (auto|true|false; default auto, follows HTTPS)
//!   ALLOW_HTTP_BASIC_OVER_HTTP=1 — required for non-loopback HTTP listen
//!
//! NOTE: This example uses `#[path]` to consume `runtime`, `bootstrap`, and
//! `web_api` modules from `src/`. They are not exposed via `lib.rs` because
//! that file is currently desktop-gated; the integration is deferred to the
//! Layer 1 / Task 2 wrap-up patch.

#[path = "../src/runtime/mod.rs"]
mod runtime;

#[path = "../src/bootstrap.rs"]
mod bootstrap;

mod app_store {
    use std::path::{Path, PathBuf};
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

    fn load_from_disk() -> Option<PathBuf> {
        let path = store_path();
        let content = std::fs::read_to_string(&path).ok()?;
        let value: serde_json::Value = serde_json::from_str(&content).ok()?;
        let raw = value.get(STORE_KEY_APP_CONFIG_DIR)?.as_str()?.trim();
        if raw.is_empty() {
            return None;
        }
        let resolved = resolve_path(raw);
        if !resolved.exists() {
            log::warn!(
                "Stored app_config_dir override no longer exists: {}",
                resolved.display()
            );
            return None;
        }
        Some(resolved)
    }

    pub fn get_app_config_dir_override() -> Option<PathBuf> {
        if let Ok(guard) = override_cache().read() {
            if let Some(value) = guard.clone() {
                return Some(value);
            }
        }

        let loaded = load_from_disk();
        update_cached_override(loaded.clone());
        loaded
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
    let port: u16 = std::env::var("PORT")
        .ok()
        .as_deref()
        .unwrap_or("3000")
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
    let state = ApiState::new(app_state, copilot_auth, codex_oauth, sink, events);

    // Build router and bind.
    let app = build_router(state);
    let listener = TcpListener::bind(addr).await?;
    log::info!("cc-switch-web listening on http://{addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    log::info!("cc-switch-web stopped cleanly");
    drop(_data_lock); // explicit for clarity
    Ok(())
}

fn init_logging() {
    let env = env_logger::Env::default().filter_or("RUST_LOG", "info,cc_switch=debug");
    let _ = env_logger::Builder::from_env(env).try_init();
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
