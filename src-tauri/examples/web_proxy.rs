//! Web-runtime `#[path]` shim for the proxy module tree.
//!
//! Mirrors `src/proxy/mod.rs` 1:1 (module list, visibility, re-exports) so the
//! standalone web server compiles the SAME proxy hot path + failover engine as
//! the desktop runtime — see the spec scenario "Web Server Proxy Module
//! Wiring": any module declared in `src/proxy/mod.rs` that web-compiled code
//! reaches must also be `#[path]`-included here.

#[path = "../src/proxy/body_filter.rs"]
pub mod body_filter;

#[path = "../src/proxy/cache_injector.rs"]
pub mod cache_injector;

#[path = "../src/proxy/circuit_breaker.rs"]
pub mod circuit_breaker;

#[path = "../src/proxy/content_encoding.rs"]
pub(crate) mod content_encoding;

#[path = "../src/proxy/copilot_optimizer.rs"]
pub mod copilot_optimizer;

#[path = "../src/proxy/error.rs"]
pub mod error;

#[path = "../src/proxy/error_mapper.rs"]
pub mod error_mapper;

#[path = "../src/proxy/failover_switch.rs"]
pub(crate) mod failover_switch;

#[path = "../src/proxy/forwarder.rs"]
mod forwarder;

#[path = "../src/proxy/gemini_url.rs"]
pub mod gemini_url;

#[path = "../src/proxy/handler_config.rs"]
pub mod handler_config;

#[path = "../src/proxy/handler_context.rs"]
pub mod handler_context;

#[path = "../src/proxy/handlers.rs"]
mod handlers;

#[path = "../src/proxy/health.rs"]
mod health;

#[path = "../src/proxy/http_client.rs"]
pub mod http_client;

#[path = "../src/proxy/hyper_client.rs"]
pub mod hyper_client;

#[path = "../src/proxy/ip_guard.rs"]
pub mod ip_guard;

#[path = "../src/proxy/json_canonical.rs"]
pub(crate) mod json_canonical;

#[path = "../src/proxy/log_codes.rs"]
pub mod log_codes;

#[path = "../src/proxy/media_sanitizer.rs"]
pub mod media_sanitizer;

#[path = "../src/proxy/model_mapper.rs"]
pub mod model_mapper;

#[path = "../src/proxy/provider_router.rs"]
pub mod provider_router;

#[path = "../src/proxy/providers/mod.rs"]
pub mod providers;

#[path = "../src/proxy/response_processor.rs"]
pub mod response_processor;

#[path = "../src/proxy/runtime_ctx.rs"]
pub mod runtime_ctx;

#[path = "../src/proxy/server.rs"]
pub(crate) mod server;

#[path = "../src/proxy/session.rs"]
pub mod session;

#[path = "../src/proxy/sse.rs"]
pub(crate) mod sse;

#[path = "../src/proxy/switch_lock.rs"]
pub(crate) mod switch_lock;

#[path = "../src/proxy/thinking_budget_rectifier.rs"]
pub mod thinking_budget_rectifier;

#[path = "../src/proxy/thinking_optimizer.rs"]
pub mod thinking_optimizer;

#[path = "../src/proxy/thinking_rectifier.rs"]
pub mod thinking_rectifier;

#[path = "../src/proxy/types.rs"]
pub(crate) mod types;

#[path = "../src/proxy/usage/mod.rs"]
pub mod usage;

// 公开导出（与 src/proxy/mod.rs 保持一致）
#[allow(unused_imports)]
pub use circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerStats, CircuitState,
};
#[allow(unused_imports)]
pub use error::ProxyError;
#[allow(unused_imports)]
pub use provider_router::ProviderRouter;
#[allow(unused_imports)]
pub use session::{
    extract_session_id, ClientFormat, ProxySession, SessionIdResult, SessionIdSource,
};
#[allow(unused_imports)]
pub use types::{ProxyConfig, ProxyServerInfo, ProxyStatus};

// 内部模块间共享（供子模块使用）
#[allow(unused_imports)]
pub(crate) use types::*;
