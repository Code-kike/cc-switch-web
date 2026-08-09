use indexmap::IndexMap;
use std::collections::HashMap;

use crate::app_config::{AppType, McpServer};
use crate::error::AppError;
use crate::mcp;
use crate::store::AppState;

/// 单个应用的 MCP 导入函数指针
type AppImporter = fn(&AppState) -> Result<usize, AppError>;

/// MCP 相关业务逻辑（v3.7.0 统一结构）
pub struct McpService;

impl McpService {
    fn rollback_server_row(
        state: &AppState,
        id: &str,
        previous: Option<&McpServer>,
    ) -> Result<(), AppError> {
        match previous {
            Some(server) => state.db.save_mcp_server(server),
            None => state.db.delete_mcp_server(id).map(|_| ()),
        }
    }

    fn rollback_and_compensate(
        state: &AppState,
        id: &str,
        previous: Option<&McpServer>,
        affected_apps: &[AppType],
        projection_error: AppError,
    ) -> AppError {
        let mut recovery_errors = Vec::new();

        if let Err(error) = Self::rollback_server_row(state, id, previous) {
            recovery_errors.push(format!("database rollback failed: {error}"));
        }

        // Compensate every app involved in the attempted mutation. Codex is a
        // whole-table projection; other apps restore/remove this exact id from
        // the previous row. The directed remove is essential for failed creates:
        // after rolling the new row out of DB, iterating an empty DB cannot see
        // and remove the Claude entry that was already written.
        for app in affected_apps {
            let compensation = if matches!(app, AppType::Codex) {
                Self::sync_enabled_for_app(state, app)
            } else if previous.is_some_and(|server| server.apps.is_enabled_for(app)) {
                Self::sync_server_to_app(
                    state,
                    previous.expect("previous server checked above"),
                    app,
                )
            } else {
                Self::remove_server_from_app(state, id, app)
            };

            if let Err(error) = compensation {
                log::warn!("MCP compensation projection to {app:?} failed: {error}");
                recovery_errors.push(format!("{} compensation failed: {error}", app.as_str()));
            }
        }

        if recovery_errors.is_empty() {
            projection_error
        } else {
            AppError::Message(format!(
                "MCP live projection failed: {projection_error}; recovery incomplete: {}",
                recovery_errors.join("; ")
            ))
        }
    }

    fn affected_apps(previous: Option<&McpServer>, next: &McpServer) -> Vec<AppType> {
        let mut apps = previous
            .map(|server| server.apps.enabled_apps())
            .unwrap_or_default();
        for app in next.apps.enabled_apps() {
            if !apps.contains(&app) {
                apps.push(app);
            }
        }
        apps
    }

    fn rollback_toggle_and_compensate(
        state: &AppState,
        id: &str,
        app: &AppType,
        previous_enabled: bool,
        projection_error: AppError,
    ) -> AppError {
        let mut recovery_errors = Vec::new();

        let restored = match state
            .db
            .update_mcp_server_app_enabled(id, app, previous_enabled)
        {
            Ok(Some((_, server))) => Some(server),
            Ok(None) => {
                recovery_errors
                    .push("database rollback failed: MCP server no longer exists".into());
                None
            }
            Err(error) => {
                recovery_errors.push(format!("database rollback failed: {error}"));
                None
            }
        };

        if let Some(server) = restored {
            let compensation = if matches!(app, AppType::Codex) {
                Self::sync_enabled_for_app(state, app)
            } else if previous_enabled {
                Self::sync_server_to_app(state, &server, app)
            } else {
                Self::remove_server_from_app(state, id, app)
            };

            if let Err(error) = compensation {
                log::warn!("MCP compensation projection to {app:?} failed: {error}");
                recovery_errors.push(format!("{} compensation failed: {error}", app.as_str()));
            }
        }

        if recovery_errors.is_empty() {
            projection_error
        } else {
            AppError::Message(format!(
                "MCP live projection failed: {projection_error}; recovery incomplete: {}",
                recovery_errors.join("; ")
            ))
        }
    }

    /// 获取所有 MCP 服务器（统一结构）
    pub fn get_all_servers(state: &AppState) -> Result<IndexMap<String, McpServer>, AppError> {
        state.db.get_all_mcp_servers()
    }

    /// 添加或更新 MCP 服务器
    pub fn upsert_server(state: &AppState, server: McpServer) -> Result<(), AppError> {
        // 读取旧状态：用于处理“编辑时取消勾选某个应用”的场景（需要从对应 live 配置中移除）
        let previous = state.db.get_all_mcp_servers()?.get(&server.id).cloned();
        let prev_apps = previous
            .as_ref()
            .map(|server| server.apps.clone())
            .unwrap_or_default();
        let affected_apps = Self::affected_apps(previous.as_ref(), &server);

        state.db.save_mcp_server(&server)?;

        let projection = (|| {
            // 处理禁用：若旧版本启用但新版本取消，则需要从该应用的 live 配置移除
            if prev_apps.claude && !server.apps.claude {
                Self::remove_server_from_app(state, &server.id, &AppType::Claude)?;
            }
            if prev_apps.codex && !server.apps.codex {
                Self::remove_server_from_app(state, &server.id, &AppType::Codex)?;
            }
            if prev_apps.gemini && !server.apps.gemini {
                Self::remove_server_from_app(state, &server.id, &AppType::Gemini)?;
            }
            if prev_apps.grokbuild && !server.apps.grokbuild {
                Self::remove_server_from_app(state, &server.id, &AppType::GrokBuild)?;
            }
            if prev_apps.opencode && !server.apps.opencode {
                Self::remove_server_from_app(state, &server.id, &AppType::OpenCode)?;
            }
            if prev_apps.hermes && !server.apps.hermes {
                Self::remove_server_from_app(state, &server.id, &AppType::Hermes)?;
            }
            Self::sync_server_to_apps(state, &server)
        })();

        if let Err(error) = projection {
            return Err(Self::rollback_and_compensate(
                state,
                &server.id,
                previous.as_ref(),
                &affected_apps,
                error,
            ));
        }

        Ok(())
    }

    /// 删除 MCP 服务器
    pub fn delete_server(state: &AppState, id: &str) -> Result<bool, AppError> {
        let server = state.db.get_all_mcp_servers()?.shift_remove(id);

        if let Some(server) = server {
            state.db.delete_mcp_server(id)?;

            if let Err(error) = Self::remove_server_from_all_apps(state, id, &server) {
                return Err(Self::rollback_and_compensate(
                    state,
                    id,
                    Some(&server),
                    &server.apps.enabled_apps(),
                    error,
                ));
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 切换指定应用的启用状态
    pub fn toggle_app(
        state: &AppState,
        server_id: &str,
        app: AppType,
        enabled: bool,
    ) -> Result<(), AppError> {
        if let Some((previous_enabled, server)) = state
            .db
            .update_mcp_server_app_enabled(server_id, &app, enabled)?
        {
            let projection = if matches!(app, AppType::Codex) {
                Self::sync_enabled_for_app(state, &app)
            } else if enabled {
                Self::sync_server_to_app(state, &server, &app)
            } else {
                Self::remove_server_from_app(state, server_id, &app)
            };
            if let Err(error) = projection {
                return Err(Self::rollback_toggle_and_compensate(
                    state,
                    server_id,
                    &app,
                    previous_enabled,
                    error,
                ));
            }
        }

        Ok(())
    }

    /// 将 MCP 服务器同步到所有启用的应用
    fn sync_server_to_apps(state: &AppState, server: &McpServer) -> Result<(), AppError> {
        for app in server.apps.enabled_apps() {
            if matches!(app, AppType::Codex) {
                Self::sync_enabled_for_app(state, &app)?;
            } else {
                Self::sync_server_to_app_no_config(server, &app)?;
            }
        }

        Ok(())
    }

    /// 将 MCP 服务器同步到指定应用
    fn sync_server_to_app(
        _state: &AppState,
        server: &McpServer,
        app: &AppType,
    ) -> Result<(), AppError> {
        Self::sync_server_to_app_no_config(server, app)
    }

    fn sync_server_to_app_no_config(server: &McpServer, app: &AppType) -> Result<(), AppError> {
        match app {
            AppType::Claude => {
                mcp::sync_single_server_to_claude(&Default::default(), &server.id, &server.server)?;
            }
            AppType::Codex => {
                // Codex uses TOML format, must use the correct function
                mcp::sync_single_server_to_codex(&Default::default(), &server.id, &server.server)?;
            }
            AppType::Gemini => {
                mcp::sync_single_server_to_gemini(&Default::default(), &server.id, &server.server)?;
            }
            AppType::GrokBuild => {
                mcp::sync_single_server_to_grokbuild(
                    &Default::default(),
                    &server.id,
                    &server.server,
                )?;
            }
            AppType::OpenCode => {
                mcp::sync_single_server_to_opencode(
                    &Default::default(),
                    &server.id,
                    &server.server,
                )?;
            }
            AppType::OpenClaw => {
                // OpenClaw MCP support is still in development (Issue #4834)
                // Skip for now
                log::debug!("OpenClaw MCP support is still in development, skipping sync");
            }
            AppType::Hermes => {
                mcp::sync_single_server_to_hermes(&Default::default(), &server.id, &server.server)?;
            }
        }
        Ok(())
    }

    /// 从所有曾启用过该服务器的应用中移除
    fn remove_server_from_all_apps(
        state: &AppState,
        id: &str,
        server: &McpServer,
    ) -> Result<(), AppError> {
        // 从所有曾启用的应用中移除
        for app in server.apps.enabled_apps() {
            if matches!(app, AppType::Codex) {
                Self::sync_enabled_for_app(state, &app)?;
            } else {
                Self::remove_server_from_app(state, id, &app)?;
            }
        }
        Ok(())
    }

    fn remove_server_from_app(_state: &AppState, id: &str, app: &AppType) -> Result<(), AppError> {
        match app {
            AppType::Claude => mcp::remove_server_from_claude(id)?,
            AppType::Codex => mcp::remove_server_from_codex(id)?,
            AppType::Gemini => mcp::remove_server_from_gemini(id)?,
            AppType::GrokBuild => mcp::remove_server_from_grokbuild(id)?,
            AppType::OpenCode => {
                mcp::remove_server_from_opencode(id)?;
            }
            AppType::OpenClaw => {
                // OpenClaw MCP support is still in development
                log::debug!("OpenClaw MCP support is still in development, skipping remove");
            }
            AppType::Hermes => {
                mcp::remove_server_from_hermes(id)?;
            }
        }
        Ok(())
    }

    /// Manually project all enabled MCP servers. Best effort: a broken live
    /// file for one application must not block independent applications.
    /// After every target is attempted, failures are returned as one aggregate
    /// so callers still see incomplete projection.
    pub fn sync_all_enabled(state: &AppState) -> Result<(), AppError> {
        let servers = Self::get_all_servers(state)?;

        let mut failures: Vec<String> = Vec::new();
        for app in AppType::all() {
            if let Err(err) = Self::project_servers_to_app(state, &servers, &app) {
                log::warn!("同步 MCP 到 {app:?} 失败: {err}");
                failures.push(format!("{}: {err}", app.as_str()));
            }
        }

        if failures.is_empty() {
            Ok(())
        } else {
            Err(AppError::Message(format!(
                "部分应用 MCP 同步失败: {}",
                failures.join("; ")
            )))
        }
    }

    /// Project the authoritative database MCP set to one application after a
    /// full live rewrite, without exposing the target path to unrelated live
    /// failures. Codex receives a complete table replacement so stale orphans
    /// are removed even when the authoritative set is empty.
    pub fn sync_enabled_for_app(state: &AppState, app: &AppType) -> Result<(), AppError> {
        let servers = Self::get_all_servers(state)?;
        Self::project_servers_to_app(state, &servers, app)
    }

    fn project_servers_to_app(
        state: &AppState,
        servers: &IndexMap<String, McpServer>,
        app: &AppType,
    ) -> Result<(), AppError> {
        if matches!(app, AppType::OpenClaw) {
            return Ok(());
        }
        if matches!(app, AppType::Codex) {
            return mcp::sync_servers_to_codex(servers);
        }

        for server in servers.values() {
            if server.apps.is_enabled_for(app) {
                Self::sync_server_to_app(state, server, app)?;
            } else {
                Self::remove_server_from_app(state, &server.id, app)?;
            }
        }

        Ok(())
    }

    // ========================================================================
    // 兼容层：支持旧的 v3.6.x 命令（已废弃，将在 v4.0 移除）
    // ========================================================================

    /// [已废弃] 获取指定应用的 MCP 服务器（兼容旧 API）
    #[deprecated(since = "3.7.0", note = "Use get_all_servers instead")]
    pub fn get_servers(
        state: &AppState,
        app: AppType,
    ) -> Result<HashMap<String, serde_json::Value>, AppError> {
        let all_servers = Self::get_all_servers(state)?;
        let mut result = HashMap::new();

        for (id, server) in all_servers {
            if server.apps.is_enabled_for(&app) {
                result.insert(id, server.server);
            }
        }

        Ok(result)
    }

    /// [已废弃] 设置 MCP 服务器在指定应用的启用状态（兼容旧 API）
    #[deprecated(since = "3.7.0", note = "Use toggle_app instead")]
    pub fn set_enabled(
        state: &AppState,
        app: AppType,
        id: &str,
        enabled: bool,
    ) -> Result<bool, AppError> {
        Self::toggle_app(state, id, app, enabled)?;
        Ok(true)
    }

    /// [已废弃] 同步启用的 MCP 到指定应用（兼容旧 API）
    #[deprecated(since = "3.7.0", note = "Use sync_all_enabled instead")]
    pub fn sync_enabled(state: &AppState, app: AppType) -> Result<(), AppError> {
        let servers = Self::get_all_servers(state)?;

        for server in servers.values() {
            if server.apps.is_enabled_for(&app) {
                Self::sync_server_to_app(state, server, &app)?;
            }
        }

        Ok(())
    }

    /// 从 Claude 导入 MCP（v3.7.0 已更新为统一结构）
    pub fn import_from_claude(state: &AppState) -> Result<usize, AppError> {
        // 创建临时 MultiAppConfig 用于导入
        let mut temp_config = crate::app_config::MultiAppConfig::default();

        // 调用原有的导入逻辑（从 mcp.rs）
        let count = crate::mcp::import_from_claude(&mut temp_config)?;

        let mut new_count = 0;

        // 如果有导入的服务器，保存到数据库
        if count > 0 {
            if let Some(servers) = &temp_config.mcp.servers {
                let mut existing = state.db.get_all_mcp_servers()?;
                for server in servers.values() {
                    // 已存在：仅启用 Claude，不覆盖其他字段（与导入模块语义保持一致）
                    let to_save = if let Some(existing_server) = existing.get(&server.id) {
                        let mut merged = existing_server.clone();
                        merged.apps.claude = true;
                        merged
                    } else {
                        // 真正的新服务器
                        new_count += 1;
                        server.clone()
                    };

                    state.db.save_mcp_server(&to_save)?;
                    existing.insert(to_save.id.clone(), to_save.clone());

                    // 导入是读取已有配置，不应反向写回任何应用的 live 配置。
                    // 显式编辑、启用/禁用或手动同步时再执行写回。
                }
            }
        }

        Ok(new_count)
    }

    /// 从 Codex 导入 MCP（v3.7.0 已更新为统一结构）
    pub fn import_from_codex(state: &AppState) -> Result<usize, AppError> {
        // 创建临时 MultiAppConfig 用于导入
        let mut temp_config = crate::app_config::MultiAppConfig::default();

        // 调用原有的导入逻辑（从 mcp.rs）
        let count = crate::mcp::import_from_codex(&mut temp_config)?;

        let mut new_count = 0;

        // 如果有导入的服务器，保存到数据库
        if count > 0 {
            if let Some(servers) = &temp_config.mcp.servers {
                let mut existing = state.db.get_all_mcp_servers()?;
                for server in servers.values() {
                    // 已存在：仅启用 Codex，不覆盖其他字段（与导入模块语义保持一致）
                    let to_save = if let Some(existing_server) = existing.get(&server.id) {
                        let mut merged = existing_server.clone();
                        merged.apps.codex = true;
                        merged
                    } else {
                        // 真正的新服务器
                        new_count += 1;
                        server.clone()
                    };

                    state.db.save_mcp_server(&to_save)?;
                    existing.insert(to_save.id.clone(), to_save.clone());

                    // 导入是读取已有配置，不应反向写回任何应用的 live 配置。
                    // 显式编辑、启用/禁用或手动同步时再执行写回。
                }
            }
        }

        Ok(new_count)
    }

    /// 从 Gemini 导入 MCP（v3.7.0 已更新为统一结构）
    pub fn import_from_gemini(state: &AppState) -> Result<usize, AppError> {
        // 创建临时 MultiAppConfig 用于导入
        let mut temp_config = crate::app_config::MultiAppConfig::default();

        // 调用原有的导入逻辑（从 mcp.rs）
        let count = crate::mcp::import_from_gemini(&mut temp_config)?;

        let mut new_count = 0;

        // 如果有导入的服务器，保存到数据库
        if count > 0 {
            if let Some(servers) = &temp_config.mcp.servers {
                let mut existing = state.db.get_all_mcp_servers()?;
                for server in servers.values() {
                    // 已存在：仅启用 Gemini，不覆盖其他字段（与导入模块语义保持一致）
                    let to_save = if let Some(existing_server) = existing.get(&server.id) {
                        let mut merged = existing_server.clone();
                        merged.apps.gemini = true;
                        merged
                    } else {
                        // 真正的新服务器
                        new_count += 1;
                        server.clone()
                    };

                    state.db.save_mcp_server(&to_save)?;
                    existing.insert(to_save.id.clone(), to_save.clone());

                    // 导入是读取已有配置，不应反向写回任何应用的 live 配置。
                    // 显式编辑、启用/禁用或手动同步时再执行写回。
                }
            }
        }

        Ok(new_count)
    }

    /// 从 Grok Build 的 `[mcp_servers]` 导入 MCP。
    pub fn import_from_grokbuild(state: &AppState) -> Result<usize, AppError> {
        let mut temp_config = crate::app_config::MultiAppConfig::default();
        let count = crate::mcp::import_from_grokbuild(&mut temp_config)?;
        let mut new_count = 0;

        if count > 0 {
            if let Some(servers) = &temp_config.mcp.servers {
                let mut existing = state.db.get_all_mcp_servers()?;
                for server in servers.values() {
                    let to_save = if let Some(existing_server) = existing.get(&server.id) {
                        let mut merged = existing_server.clone();
                        merged.apps.grokbuild = true;
                        merged
                    } else {
                        new_count += 1;
                        server.clone()
                    };
                    state.db.save_mcp_server(&to_save)?;
                    existing.insert(to_save.id.clone(), to_save);
                }
            }
        }
        Ok(new_count)
    }

    /// 从 OpenCode 导入 MCP（v3.9.2+ 新增）
    pub fn import_from_opencode(state: &AppState) -> Result<usize, AppError> {
        // 创建临时 MultiAppConfig 用于导入
        let mut temp_config = crate::app_config::MultiAppConfig::default();

        // 调用原有的导入逻辑（从 mcp/opencode.rs）
        let count = crate::mcp::import_from_opencode(&mut temp_config)?;

        let mut new_count = 0;

        // 如果有导入的服务器，保存到数据库
        if count > 0 {
            if let Some(servers) = &temp_config.mcp.servers {
                let mut existing = state.db.get_all_mcp_servers()?;
                for server in servers.values() {
                    // 已存在：仅启用 OpenCode，不覆盖其他字段（与导入模块语义保持一致）
                    let to_save = if let Some(existing_server) = existing.get(&server.id) {
                        let mut merged = existing_server.clone();
                        merged.apps.opencode = true;
                        merged
                    } else {
                        // 真正的新服务器
                        new_count += 1;
                        server.clone()
                    };

                    state.db.save_mcp_server(&to_save)?;
                    existing.insert(to_save.id.clone(), to_save.clone());

                    // 导入是读取已有配置，不应反向写回任何应用的 live 配置。
                    // 显式编辑、启用/禁用或手动同步时再执行写回。
                }
            }
        }

        Ok(new_count)
    }

    /// 从 Hermes 导入 MCP
    pub fn import_from_hermes(state: &AppState) -> Result<usize, AppError> {
        // 创建临时 MultiAppConfig 用于导入
        let mut temp_config = crate::app_config::MultiAppConfig::default();

        // 调用导入逻辑（从 mcp/hermes.rs）
        let count = crate::mcp::import_from_hermes(&mut temp_config)?;

        let mut new_count = 0;

        // 如果有导入的服务器，保存到数据库
        if count > 0 {
            if let Some(servers) = &temp_config.mcp.servers {
                let mut existing = state.db.get_all_mcp_servers()?;
                for server in servers.values() {
                    // 已存在：仅启用 Hermes，不覆盖其他字段（与导入模块语义保持一致）
                    let to_save = if let Some(existing_server) = existing.get(&server.id) {
                        let mut merged = existing_server.clone();
                        merged.apps.hermes = true;
                        merged
                    } else {
                        // 真正的新服务器
                        new_count += 1;
                        server.clone()
                    };

                    state.db.save_mcp_server(&to_save)?;
                    existing.insert(to_save.id.clone(), to_save.clone());

                    // 导入是读取已有配置，不应反向写回任何应用的 live 配置。
                    // 显式编辑、启用/禁用或手动同步时再执行写回。
                }
            }
        }

        Ok(new_count)
    }

    /// Import MCP servers from every supported application.
    ///
    /// Best effort: one malformed source does not block successful imports from
    /// other applications. After all importers run, any failures are returned
    /// together with the number already persisted so the UI cannot misreport a
    /// partial result as an unqualified success.
    pub fn import_from_all_apps(state: &AppState) -> Result<usize, AppError> {
        let importers: [(&str, AppImporter); 6] = [
            ("claude", Self::import_from_claude),
            ("codex", Self::import_from_codex),
            ("gemini", Self::import_from_gemini),
            ("grokbuild", Self::import_from_grokbuild),
            ("opencode", Self::import_from_opencode),
            ("hermes", Self::import_from_hermes),
        ];

        let mut total = 0usize;
        let mut failures: Vec<String> = Vec::new();

        for (app, importer) in importers {
            match importer(state) {
                Ok(count) => total += count,
                Err(err) => {
                    log::warn!("从 {app} 导入 MCP 失败: {err}");
                    failures.push(format!("{app}: {err}"));
                }
            }
        }

        if failures.is_empty() {
            Ok(total)
        } else {
            Err(AppError::Message(format!(
                "已导入 {total} 个，部分应用导入失败: {}",
                failures.join("; ")
            )))
        }
    }
}
