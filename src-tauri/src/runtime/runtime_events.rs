//! Event sinks — abstract over `tauri::AppHandle::emit` vs an in-process
//! broadcast channel for Web mode.

use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;
use serde_json::Value;
use tokio::sync::broadcast;

/// Event payload carried over the Web SSE stream and Tauri event bus.
#[derive(Clone, Debug, Serialize)]
pub struct EventEnvelope {
    pub event: String,
    pub payload: Value,
    pub ts: i64,
    pub seq: u64,
}

/// Abstract sink. Allows proxy/failover/webdav code to emit events without
/// depending on Tauri or HTTP transport directly.
pub trait UiEventSink: Send + Sync {
    fn emit_json(&self, event: &str, payload: Value);
    fn refresh_tray(&self) {}
    fn open_url(&self, url: &str) -> Result<(), String> {
        Err(format!(
            "open_url not supported in this runtime (target: {url})"
        ))
    }
}

/// Drops every event. Used in tests or as a placeholder during migration.
///
/// The body is empty and pulls in nothing from `tauri`, so the gate also admits
/// `test`: the provider service's test module is compiled into the **web**
/// example's test build as well (through the `#[path]` shim in
/// `examples/web_services.rs`) and needs a sink there. Staying gated in the
/// non-test web build keeps it from becoming dead code.
#[cfg(any(feature = "desktop", test))]
pub struct NoopEventSink;

#[cfg(any(feature = "desktop", test))]
impl UiEventSink for NoopEventSink {
    fn emit_json(&self, _event: &str, _payload: Value) {}
}

/// Web-mode sink. Fans events to all subscribers (e.g. SSE handler) via
/// `tokio::sync::broadcast`. Receivers that fall behind get `Lagged`,
/// which the client handles by invalidating its cached state.
pub struct ChannelEventSink {
    tx: broadcast::Sender<EventEnvelope>,
    seq: AtomicU64,
}

impl ChannelEventSink {
    pub fn new(buffer: usize) -> (Self, broadcast::Receiver<EventEnvelope>) {
        let (tx, rx) = broadcast::channel(buffer.max(1));
        let sink = Self {
            tx,
            seq: AtomicU64::new(0),
        };
        (sink, rx)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<EventEnvelope> {
        self.tx.subscribe()
    }

    pub fn sender(&self) -> broadcast::Sender<EventEnvelope> {
        self.tx.clone()
    }
}

impl UiEventSink for ChannelEventSink {
    fn emit_json(&self, event: &str, payload: Value) {
        let env = EventEnvelope {
            event: event.to_string(),
            payload,
            ts: chrono::Utc::now().timestamp_millis(),
            seq: self.seq.fetch_add(1, Ordering::Relaxed),
        };
        // Ignore send errors: no subscribers is a normal case.
        let _ = self.tx.send(env);
    }
}

/// Tauri-backed sink (desktop mode).
#[cfg(feature = "desktop")]
pub struct TauriEventSink {
    handle: tauri::AppHandle,
}

#[cfg(feature = "desktop")]
impl TauriEventSink {
    pub fn new(handle: tauri::AppHandle) -> Self {
        Self { handle }
    }
}

#[cfg(feature = "desktop")]
impl UiEventSink for TauriEventSink {
    fn emit_json(&self, event: &str, payload: Value) {
        use tauri::Emitter;
        if let Err(err) = self.handle.emit(event, payload) {
            log::warn!("TauriEventSink emit({event}) failed: {err}");
        }
    }

    /// 重建托盘菜单（故障转移切换后让托盘立即反映新的当前供应商）。
    ///
    /// 行为与旧 `failover_switch.rs` 内联实现一致：AppState 不可用或菜单
    /// 创建失败时静默跳过，仅 `set_menu` 失败时记录错误。
    fn refresh_tray(&self) {
        use tauri::Manager;
        let Some(app_state) = self.handle.try_state::<crate::store::AppState>() else {
            return;
        };
        if let Ok(new_menu) = crate::tray::create_tray_menu(&self.handle, app_state.inner()) {
            if let Some(tray) = self.handle.tray_by_id(crate::tray::TRAY_ID) {
                if let Err(e) = tray.set_menu(Some(new_menu)) {
                    log::error!("[Failover] 更新托盘菜单失败: {e}");
                }
            }
        }
    }

    fn open_url(&self, url: &str) -> Result<(), String> {
        use tauri_plugin_opener::OpenerExt;
        self.handle
            .opener()
            .open_url(url, None::<&str>)
            .map_err(|e| e.to_string())
    }
}
