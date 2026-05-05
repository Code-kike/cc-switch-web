//! Tower middleware bundle for the Web API.
//!
//! Layer 2 / Task 4 scaffolding. Each module exposes a `layer()` function
//! suitable for `Router::layer(...)` or `ServiceBuilder::layer(...)`.

pub mod auth;
pub mod cors;
pub mod csrf;
pub mod rate_limit;
pub mod security_headers;
