//! 代理运行时上下文
//!
//! 取代代理核心（forwarder / failover_switch / server）对 `tauri::AppHandle`
//! 的直接依赖，使热路径在桌面（Tauri）与 Web（headless server）两个运行时
//! 之间共享同一份实现：
//!
//! - 事件发射：通过 [`UiEventSink`] 抽象（桌面 = `TauriEventSink`，
//!   Web = `ChannelEventSink` → SSE `/api/events`）。
//! - 托盘刷新：`UiEventSink::refresh_tray`（默认 no-op，桌面端覆写）。
//! - Copilot / Codex / xAI OAuth 认证管理器：直接注入 `Arc<RwLock<Manager>>`，
//!   取代桌面专属的 `app_handle.state::<CopilotAuthState/CodexOAuthState>()`
//!   service-locator 查找。
//! - 故障转移热切换：持有 `ProxyService` 的 clone（全 Arc 字段，共享同一
//!   实例状态），取代 `app.try_state::<AppState>().proxy_service` 查找。
//!   由此形成的 service → server → failover_manager → service 引用环是
//!   应用生命周期单例，无泄漏影响。

use std::sync::Arc;

use tokio::sync::RwLock;

use crate::proxy::providers::codex_oauth_auth::CodexOAuthManager;
use crate::proxy::providers::copilot_auth::CopilotAuthManager;
use crate::proxy::providers::xai_oauth_auth::XaiOAuthManager;
use crate::runtime::UiEventSink;
use crate::services::ProxyService;

/// 代理运行时上下文（runtime-neutral，替代 `Option<tauri::AppHandle>`）。
///
/// 在应用初始化时构建一次（桌面：`lib.rs` setup；Web：`examples/server.rs`
/// 启动序列），通过 `ProxyService::set_runtime_ctx` 注入，再随
/// `ProxyServer::new` 传入代理热路径。
#[derive(Clone)]
pub struct ProxyRuntimeCtx {
    /// 运行时事件 sink（事件发射 + 托盘刷新）
    pub sink: Arc<dyn UiEventSink>,
    /// Copilot 认证管理器（多账号 token / API endpoint / 模型目录）
    pub copilot_auth: Arc<RwLock<CopilotAuthManager>>,
    /// Codex OAuth 认证管理器（ChatGPT Plus/Pro access_token）
    pub codex_oauth: Arc<RwLock<CodexOAuthManager>>,
    /// xAI OAuth 认证管理器（Grok API access_token）
    pub xai_oauth: Arc<RwLock<XaiOAuthManager>>,
    /// 故障转移热切换句柄（`ProxyService` clone，Arc 字段共享同一实例）
    pub hot_switch: ProxyService,
}

impl ProxyRuntimeCtx {
    /// 发射事件到前端（桌面 = Tauri 事件总线；Web = SSE 广播）。
    pub fn emit_json(&self, event: &str, payload: serde_json::Value) {
        self.sink.emit_json(event, payload);
    }

    /// 刷新托盘菜单（桌面端由 `TauriEventSink` 覆写；其他运行时为 no-op）。
    pub fn refresh_tray(&self) {
        self.sink.refresh_tray();
    }
}
