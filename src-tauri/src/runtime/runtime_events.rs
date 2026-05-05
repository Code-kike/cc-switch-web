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
pub struct NoopEventSink;

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

    fn open_url(&self, url: &str) -> Result<(), String> {
        use tauri_plugin_opener::OpenerExt;
        self.handle
            .opener()
            .open_url(url, None::<&str>)
            .map_err(|e| e.to_string())
    }
}
