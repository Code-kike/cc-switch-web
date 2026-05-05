//! Shared state passed to every handler.

use std::sync::Arc;

use tokio::sync::broadcast;

#[derive(Clone)]
pub struct ApiState {
    pub app_state: Arc<crate::store::AppState>,
    pub copilot_auth:
        Arc<tokio::sync::RwLock<crate::proxy::providers::copilot_auth::CopilotAuthManager>>,
    pub codex_oauth:
        Arc<tokio::sync::RwLock<crate::proxy::providers::codex_oauth_auth::CodexOAuthManager>>,
    /// Shared event sink. Web mode uses `ChannelEventSink`; tests can use
    /// `NoopEventSink`. Used by handlers that need to emit events to SSE.
    pub sink: Arc<dyn super::super::runtime::UiEventSink>,
    /// Broadcast stream backing `/api/events`.
    pub events: broadcast::Sender<crate::runtime::EventEnvelope>,
}

impl ApiState {
    pub fn new(
        app_state: Arc<crate::store::AppState>,
        copilot_auth: Arc<
            tokio::sync::RwLock<crate::proxy::providers::copilot_auth::CopilotAuthManager>,
        >,
        codex_oauth: Arc<
            tokio::sync::RwLock<crate::proxy::providers::codex_oauth_auth::CodexOAuthManager>,
        >,
        sink: Arc<dyn super::super::runtime::UiEventSink>,
        events: broadcast::Sender<crate::runtime::EventEnvelope>,
    ) -> Self {
        Self {
            app_state,
            copilot_auth,
            codex_oauth,
            sink,
            events,
        }
    }
}
