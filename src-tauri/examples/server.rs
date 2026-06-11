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
    let state = ApiState::new(app_state, copilot_auth, codex_oauth, sink, events);

    // Startup parity with the desktop `initialize_common_config_snippets`
    // hook: auto-extract common config snippets from clean live files, run the
    // one-shot legacy common-config migration, and strip legacy
    // `_cc_source` / `provider_key` markers that older
    // `import_hermes_providers_from_live` baked into provider records. All steps
    // are idempotent and gated by their own settings flags, so a successful run
    // never repeats. See
    // `ProviderService::initialize_common_config_snippets`.
    crate::services::ProviderService::initialize_common_config_snippets(state.app_state.as_ref());

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
