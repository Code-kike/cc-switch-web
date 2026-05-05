//! Runtime adapters — bridge between core services and the host (Tauri or HTTP).
//!
//! Layer 1 / Task 2 (partial scaffolding).
//!
//! - `UiEventSink` decouples proxy/failover/webdav from `tauri::AppHandle`.
//! - `ChannelEventSink` is the Web mode counterpart, fanning events to SSE
//!   subscribers via a tokio broadcast channel.
//! - `CoreRuntime` collects spawned worker handles for graceful shutdown.

pub mod runtime_events;
pub mod runtime_handle;

pub use runtime_events::{ChannelEventSink, EventEnvelope, NoopEventSink, UiEventSink};
pub use runtime_handle::CoreRuntime;

#[cfg(feature = "desktop")]
pub use runtime_events::TauriEventSink;
