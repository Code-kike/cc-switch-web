//! Bootstrap — common initialization for desktop and web-server runtimes.
//!
//! Tauri-free startup core shared by both front ends (this module is
//! `#[path]`-included into the web example, where `tauri` does not exist):
//!   - `data_dir` / `db_lock_path` path helpers
//!   - `acquire_data_dir_lock` cross-process advisory lock (web-server only)
//!   - `check_filesystem_local` non-local-FS guard
//!   - `apply_legacy_json_migration` (F5) + `run_post_db_bootstrap` (F6),
//!     the fully-integrated shared startup path called by both
//!     `src/lib.rs::setup()` and `examples/server.rs::main()`.

use std::path::{Path, PathBuf};

/// Default data directory, `~/.cc-switch`. Override via `CC_SWITCH_DATA_DIR`
/// environment variable (used by Docker `/data` volume).
pub fn data_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("CC_SWITCH_DATA_DIR") {
        return PathBuf::from(custom);
    }
    dirs::home_dir()
        .map(|h| h.join(".cc-switch"))
        .unwrap_or_else(|| PathBuf::from(".cc-switch"))
}

/// Path of the cross-process DB lock, `<data_dir>/cc-switch.db.lock`.
pub fn db_lock_path(data_dir: &Path) -> PathBuf {
    data_dir.join("cc-switch.db.lock")
}

/// Cross-process advisory lock for the data directory (web-server only).
///
/// Returns the locked file handle; dropping the handle releases the lock.
/// On NFS / 9p / sshfs the call may fail or be silently lost; callers should
/// also gate on `check_filesystem_local` to refuse non-local volumes.
#[cfg(feature = "fs2")]
pub fn acquire_data_dir_lock(data_dir: &Path) -> Result<std::fs::File, String> {
    use fs2::FileExt;
    std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
    let lock_path = db_lock_path(data_dir);
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .read(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|e| format!("open {}: {}", lock_path.display(), e))?;
    file.try_lock_exclusive()
        .map_err(|e| format!("data dir already locked: {e}"))?;
    Ok(file)
}

/// Best-effort check that the data directory lives on a local filesystem.
/// Layer 1 / Task 2 (Round 3 P0-1 + Round 4 P1-8).
#[cfg(target_os = "linux")]
pub fn check_filesystem_local(path: &Path) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
    let target = path.canonicalize().map_err(|e| e.to_string())?;
    let mounts = match std::fs::read_to_string("/proc/self/mounts") {
        Ok(s) => s,
        Err(_) => return Ok(()),
    };

    let mut best_match_len: usize = 0;
    let mut best_fstype: Option<String> = None;
    for line in mounts.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }
        let mount_point = parts[1];
        let fstype = parts[2];
        if target.starts_with(mount_point) && mount_point.len() > best_match_len {
            best_match_len = mount_point.len();
            best_fstype = Some(fstype.to_lowercase());
        }
    }

    if let Some(fs) = best_fstype {
        let blocked = matches!(
            fs.as_str(),
            "nfs"
                | "nfs4"
                | "cifs"
                | "smbfs"
                | "smb2"
                | "smb3"
                | "9p"
                | "fuse.sshfs"
                | "fuse.gvfs"
                | "gvfs"
        );
        if blocked {
            return Err(format!(
                "cc-switch DB only supports local filesystems; detected `{fs}` for {}",
                target.display()
            ));
        }
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn check_filesystem_local(_path: &Path) -> Result<(), String> {
    // macOS / Windows: defer to higher-level checks; native APIs vary widely.
    Ok(())
}

// ============================================================
// Shared startup core (desktop + web) — audit F5 / F6
// ============================================================
//
// Both runtimes (`src/lib.rs::setup()` for desktop, `examples/server.rs::main()`
// for web) must, after the SQLite database is created:
//   1. (F5) migrate a legacy `config.json` into the fresh DB and archive it;
//   2. (F6) seed default Skills repos + official providers, auto-import live CLI
//      config / OMO / MCP / prompts.
//
// These two helpers hold that logic in ONE place so the web runtime stays in
// parity with desktop. They MUST remain tauri-free (this module is `#[path]`-
// included into the web example, where `tauri` does not exist) — desktop-only UI
// (the migration error dialog + retry/exit loop) stays in `lib.rs` and wraps the
// load step; web has no dialog and falls through to an empty DB on load failure.

/// (F5) Apply a pre-loaded legacy `config.json` into the fresh SQLite database,
/// then archive the JSON file. Tauri-free core shared by both runtimes.
///
/// The caller is responsible for the load step (`MultiAppConfig::load()`):
/// desktop wraps it in a dialog/retry/exit loop, web logs-and-continues. This
/// helper only runs once the config is already in hand and a fresh DB exists.
///
/// Mirrors the desktop `lib.rs` post-load block byte-for-byte: on success it
/// marks `init_status::set_migration_success()` (frontend Toast) and renames
/// `config.json` → `config.json.migrated`; migration failure (disk full etc.)
/// is logged but non-fatal — the run continues with whatever the DB holds.
pub fn apply_legacy_json_migration(
    db: &crate::database::Database,
    config: &crate::app_config::MultiAppConfig,
    json_path: &Path,
) {
    log::info!("开始执行数据迁移...");

    match db.migrate_from_json(config) {
        Ok(_) => {
            log::info!("✓ 配置迁移成功");
            // 标记迁移成功，供前端显示 Toast
            crate::init_status::set_migration_success();
            // 归档旧配置文件（重命名而非删除，便于用户恢复）
            let archive_path = json_path.with_extension("json.migrated");
            if let Err(e) = std::fs::rename(json_path, &archive_path) {
                log::warn!("归档旧配置文件失败: {e}");
            } else {
                log::info!("✓ 旧配置已归档为 config.json.migrated");
            }
        }
        Err(e) => {
            // 配置加载成功但迁移失败的情况极少（磁盘满等），仅记录日志
            log::error!("配置迁移失败: {e}，将从现有配置导入");
        }
    }
}

/// (F6) Post-DB bootstrap: seed defaults + auto-import live config. Tauri-free
/// core shared by both runtimes, called right after `AppState::new(db)` and
/// before the proxy runtime context is injected.
///
/// Every step is idempotent (table-empty / `should_import_default_config_on_startup`
/// gated), so this is safe to re-run on every systemd boot of the web server.
/// This is a verbatim extraction of the desktop `lib.rs::setup()` block
/// (steps 1, 1.1, 1.5, 1.6, 2, 2.3, 3, 4) — desktop behavior is unchanged.
pub fn run_post_db_bootstrap(app_state: &crate::store::AppState) {
    // ============================================================
    // 按表独立判断的导入逻辑（各类数据独立检查，互不影响）
    // ============================================================

    // 1. 初始化默认 Skills 仓库（已有内置检查：表非空则跳过）
    match app_state.db.init_default_skill_repos() {
        Ok(count) if count > 0 => {
            log::info!("✓ Initialized {count} default skill repositories");
        }
        Ok(_) => {} // 表非空，静默跳过
        Err(e) => log::warn!("✗ Failed to initialize default skill repos: {e}"),
    }

    // 1.1. Skills 统一管理迁移：当数据库迁移到 v3 结构后，自动从各应用目录导入到 SSOT
    // 触发条件由 schema 迁移设置 settings.skills_ssot_migration_pending = true 控制。
    match app_state.db.get_setting("skills_ssot_migration_pending") {
        Ok(Some(flag)) if flag == "true" || flag == "1" => {
            // 安全保护：如果用户已经有 v3 结构的 Skills 数据，就不要自动清空重建。
            let has_existing = app_state
                .db
                .get_all_installed_skills()
                .map(|skills| !skills.is_empty())
                .unwrap_or(false);

            if has_existing {
                log::info!(
                    "Detected skills_ssot_migration_pending but skills table not empty; skipping auto import."
                );
                let _ = app_state
                    .db
                    .set_setting("skills_ssot_migration_pending", "false");
            } else {
                match crate::services::skill::migrate_skills_to_ssot(&app_state.db) {
                    Ok(count) => {
                        log::info!("✓ Auto imported {count} skill(s) into SSOT");
                        if count > 0 {
                            crate::init_status::set_skills_migration_result(count);
                        }
                        let _ = app_state
                            .db
                            .set_setting("skills_ssot_migration_pending", "false");
                    }
                    Err(e) => {
                        log::warn!("✗ Failed to auto import legacy skills to SSOT: {e}");
                        crate::init_status::set_skills_migration_error(e.to_string());
                        // 保留 pending 标志，方便下次启动重试
                    }
                }
            }
        }
        Ok(_) => {} // 未开启迁移标志，静默跳过
        Err(e) => log::warn!("✗ Failed to read skills migration flag: {e}"),
    }

    // 1.5. 自动导入 live 配置 + seed 官方预设供应商（Claude / Codex / Gemini）
    //
    // 先 import 后 seed 是有意为之：先把用户手动配置的 settings.json / auth.json / .env
    // 落成 "default" provider 设为 current，再追加官方预设（is_current=false）。
    // 这样用户切到官方预设时，回填机制会保护原 live 配置不丢失。
    //
    // 捕获首次运行快照：所有全新装用户都会看到欢迎弹窗介绍 CC Switch 的工作方式。
    // 读失败时默认不弹，宁可漏弹也不要因为故障打扰用户。
    let first_run_already_confirmed = crate::settings::get_settings()
        .first_run_notice_confirmed
        .unwrap_or(false);
    let fresh_install_at_startup = app_state.db.is_providers_empty().unwrap_or(false);

    for app_type in crate::app_config::AppType::all().filter(|t| !t.is_additive_mode()) {
        if !crate::services::provider::should_import_default_config_on_startup(app_state, &app_type)
            .unwrap_or(false)
        {
            log::debug!(
                "○ {} already has providers; live import skipped",
                app_type.as_str()
            );
            continue;
        }

        match crate::services::provider::import_default_config(app_state, app_type.clone()) {
            Ok(true) => log::info!(
                "✓ Imported live config for {} as default provider",
                app_type.as_str()
            ),
            Ok(false) => log::debug!(
                "○ {} already has providers; live import skipped",
                app_type.as_str()
            ),
            Err(e) => log::debug!("○ No live config to import for {}: {e}", app_type.as_str()),
        }
    }

    match app_state.db.init_default_official_providers() {
        Ok(count) if count > 0 => {
            log::info!("✓ Seeded {count} official provider(s)");
        }
        Ok(_) => {}
        Err(e) => log::warn!("✗ Failed to seed official providers: {e}"),
    }

    // 老用户 / 已确认的路径由 `fresh_install_at_startup` 自行拦截，这里不做写入。
    // 字段只由前端在用户点击"我知道了"时 save_settings 回写，语义是"用户显式确认过"。
    if !first_run_already_confirmed && fresh_install_at_startup {
        log::info!("✓ First-run welcome notice pending");
    }

    // 1.6. 自动同步 OpenCode / OpenClaw 的 live providers 到数据库
    //
    // additive 模式（OpenCode / OpenClaw / Hermes）的 import 函数按 id 幂等——
    // 新 id 执行导入，已有 id 则更新 settings 和 display name，所以每次
    // 启动都跑是安全的：既保证新装用户开箱可见 live 中的供应商，也让外部
    // 修改的 live 文件能在重启后同步到数据库（与之前依赖前端"导入当前配置"
    // 按钮手动触发不同）。
    //
    // 底层 read_*_config 在文件不存在时返回默认空配置，因此新装且无
    // live 文件的用户走 Ok(0) 路径，不会产生错误日志噪音。
    match crate::services::provider::import_opencode_providers_from_live(app_state) {
        Ok(count) if count > 0 => {
            log::info!("✓ Synced {count} OpenCode provider(s) from live config");
        }
        Ok(_) => log::debug!("○ No OpenCode provider changes from live config"),
        Err(e) => log::warn!("✗ Failed to import OpenCode providers: {e}"),
    }
    match crate::services::provider::import_openclaw_providers_from_live(app_state) {
        Ok(count) if count > 0 => {
            log::info!("✓ Synced {count} OpenClaw provider(s) from live config");
        }
        Ok(_) => log::debug!("○ No OpenClaw provider changes from live config"),
        Err(e) => log::warn!("✗ Failed to import OpenClaw providers: {e}"),
    }
    match crate::services::provider::import_hermes_providers_from_live(app_state) {
        Ok(count) if count > 0 => {
            log::info!("✓ Synced {count} Hermes provider(s) from live config");
        }
        Ok(_) => log::debug!("○ No Hermes provider changes from live config"),
        Err(e) => log::warn!("✗ Failed to import Hermes providers: {e}"),
    }

    // 2. OMO 配置导入（当数据库中无 OMO provider 时，从本地文件导入）
    {
        let has_omo = app_state
            .db
            .get_all_providers("opencode")
            .map(|providers| {
                providers
                    .values()
                    .any(|p| p.category.as_deref() == Some("omo"))
            })
            .unwrap_or(false);
        if !has_omo {
            match crate::services::OmoService::import_from_local(
                app_state,
                &crate::services::omo::STANDARD,
            ) {
                Ok(provider) => {
                    log::info!(
                        "✓ Imported OMO config from local as provider '{}'",
                        provider.name
                    );
                }
                Err(crate::error::AppError::OmoConfigNotFound) => {
                    log::debug!("○ No OMO config to import");
                }
                Err(e) => {
                    log::warn!("✗ Failed to import OMO config from local: {e}");
                }
            }
        }
    }

    // 2.3 OMO Slim config import (when no omo-slim provider in DB, import from local)
    {
        let has_omo_slim = app_state
            .db
            .get_all_providers("opencode")
            .map(|providers| {
                providers
                    .values()
                    .any(|p| p.category.as_deref() == Some("omo-slim"))
            })
            .unwrap_or(false);
        if !has_omo_slim {
            match crate::services::OmoService::import_from_local(
                app_state,
                &crate::services::omo::SLIM,
            ) {
                Ok(provider) => {
                    log::info!(
                        "✓ Imported OMO Slim config from local as provider '{}'",
                        provider.name
                    );
                }
                Err(crate::error::AppError::OmoConfigNotFound) => {
                    log::debug!("○ No OMO Slim config to import");
                }
                Err(e) => {
                    log::warn!("✗ Failed to import OMO Slim config from local: {e}");
                }
            }
        }
    }

    // 3. 导入 MCP 服务器配置（表空时触发）
    if app_state.db.is_mcp_table_empty().unwrap_or(false) {
        log::info!("MCP table empty, importing from live configurations...");

        match crate::services::mcp::McpService::import_from_claude(app_state) {
            Ok(count) if count > 0 => {
                log::info!("✓ Imported {count} MCP server(s) from Claude");
            }
            Ok(_) => log::debug!("○ No Claude MCP servers found to import"),
            Err(e) => log::warn!("✗ Failed to import Claude MCP: {e}"),
        }

        match crate::services::mcp::McpService::import_from_codex(app_state) {
            Ok(count) if count > 0 => {
                log::info!("✓ Imported {count} MCP server(s) from Codex");
            }
            Ok(_) => log::debug!("○ No Codex MCP servers found to import"),
            Err(e) => log::warn!("✗ Failed to import Codex MCP: {e}"),
        }

        match crate::services::mcp::McpService::import_from_gemini(app_state) {
            Ok(count) if count > 0 => {
                log::info!("✓ Imported {count} MCP server(s) from Gemini");
            }
            Ok(_) => log::debug!("○ No Gemini MCP servers found to import"),
            Err(e) => log::warn!("✗ Failed to import Gemini MCP: {e}"),
        }

        match crate::services::mcp::McpService::import_from_opencode(app_state) {
            Ok(count) if count > 0 => {
                log::info!("✓ Imported {count} MCP server(s) from OpenCode");
            }
            Ok(_) => log::debug!("○ No OpenCode MCP servers found to import"),
            Err(e) => log::warn!("✗ Failed to import OpenCode MCP: {e}"),
        }

        match crate::services::mcp::McpService::import_from_hermes(app_state) {
            Ok(count) if count > 0 => {
                log::info!("✓ Imported {count} MCP server(s) from Hermes");
            }
            Ok(_) => log::debug!("○ No Hermes MCP servers found to import"),
            Err(e) => log::warn!("✗ Failed to import Hermes MCP: {e}"),
        }
    }

    // 4. 导入提示词文件（表空时触发）
    if app_state.db.is_prompts_table_empty().unwrap_or(false) {
        log::info!("Prompts table empty, importing from live configurations...");

        for app in [
            crate::app_config::AppType::Claude,
            crate::app_config::AppType::Codex,
            crate::app_config::AppType::Gemini,
            crate::app_config::AppType::OpenCode,
            crate::app_config::AppType::OpenClaw,
            crate::app_config::AppType::Hermes,
        ] {
            match crate::services::prompt::PromptService::import_from_file_on_first_launch(
                app_state,
                app.clone(),
            ) {
                Ok(count) if count > 0 => {
                    log::info!("✓ Imported {count} prompt(s) for {}", app.as_str());
                }
                Ok(_) => log::debug!("○ No prompt file found for {}", app.as_str()),
                Err(e) => log::warn!("✗ Failed to import prompt for {}: {e}", app.as_str()),
            }
        }
    }
}
