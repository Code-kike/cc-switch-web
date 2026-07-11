//! Web API surface — Axum router + middleware + 25 handler modules.
//!
//! Layer 2 / Task 4. This module is consumed by `examples/server.rs` via a
//! `#[path]` module reference; full integration into `lib.rs` is deferred to
//! the Layer 1 / Task 2 wrap-up work (lib.rs setup() refactor).
//!
//! Architecture:
//!   - `handlers/`   — one file per Web sub-resource (25 modules). Each
//!                     exports `pub fn router(state: ApiState) -> Router`.
//!   - `middleware/` — same-origin intent guard, CORS (same-origin default), and
//!                     security headers (CSP/HSTS/Permissions-Policy). Body
//!                     limit is applied in `routes.rs` via `DefaultBodyLimit`.
//!   - `state.rs`    — shared `ApiState` (DB pool + sink + cancel token).
//!   - `routes.rs`   — root router assembly + SPA fallback.

pub mod handlers;
pub mod middleware;
pub mod routes;
pub mod state;

pub use routes::{build_degraded_router, build_router};
pub use state::ApiState;
