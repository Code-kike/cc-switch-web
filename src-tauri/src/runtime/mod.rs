//! Runtime adapters — bridge between core services and the host (Tauri or HTTP).
//!
//! Layer 1 / Task 2 (partial scaffolding).
//!
//! - `UiEventSink` decouples proxy/failover/webdav from `tauri::AppHandle`.
//! - `ChannelEventSink` is the Web mode counterpart, fanning events to SSE
//!   subscribers via a tokio broadcast channel.

pub mod runtime_events;

#[cfg(feature = "desktop")]
pub use runtime_events::NoopEventSink;
pub use runtime_events::{ChannelEventSink, EventEnvelope, UiEventSink};

#[cfg(feature = "desktop")]
pub use runtime_events::TauriEventSink;
