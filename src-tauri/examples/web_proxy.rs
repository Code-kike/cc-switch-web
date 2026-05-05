#[path = "../src/proxy/types.rs"]
pub mod types;

#[path = "../src/proxy/error.rs"]
pub mod error;

#[path = "../src/proxy/gemini_url.rs"]
pub mod gemini_url;

#[path = "../src/proxy/http_client.rs"]
pub mod http_client;

#[path = "../src/proxy/providers/mod.rs"]
pub mod providers;

#[path = "../src/proxy/session.rs"]
pub mod session;

#[path = "../src/proxy/sse.rs"]
pub mod sse;

#[path = "../src/proxy/usage/mod.rs"]
pub mod usage;

pub mod circuit_breaker {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CircuitBreakerConfig {
        pub failure_threshold: u32,
        pub success_threshold: u32,
        pub timeout_seconds: u64,
        pub error_rate_threshold: f64,
        pub min_requests: u32,
    }

    impl Default for CircuitBreakerConfig {
        fn default() -> Self {
            Self {
                failure_threshold: 4,
                success_threshold: 2,
                timeout_seconds: 60,
                error_rate_threshold: 0.6,
                min_requests: 10,
            }
        }
    }
}
