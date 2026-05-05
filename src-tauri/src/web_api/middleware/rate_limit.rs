//! Rate limit — global request cap + tighter cap on `/api/auth/login` and
//! `/api/system/web-credentials` (Round 5 P1-1 brute-force defence).
//!
//! Layer 2 stub. Production wires `tower::limit::ConcurrencyLimitLayer` on
//! the global router and a per-IP token bucket on auth endpoints.

use tower::limit::ConcurrencyLimitLayer;

#[allow(dead_code)]
pub fn global_layer() -> ConcurrencyLimitLayer {
    // 100 concurrent in-flight requests; tune via env later.
    ConcurrencyLimitLayer::new(100)
}
