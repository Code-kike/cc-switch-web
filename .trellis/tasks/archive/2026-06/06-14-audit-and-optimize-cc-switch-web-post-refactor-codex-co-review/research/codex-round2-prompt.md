ROUND 2 — closing the debate. I verified the two open items against the code:

- L1: CONCEDED. forwarder.rs:1721-1727 overrides streaming requests to 24h and non-streaming to `self.non_streaming_timeout`; the hyper raw path (1757-1765) passes its own `timeout`. So http_client.rs:218's 600s default never caps the proxy hot path, and I have no streamed `http_client` callsite without an override. L1 is dropped as a false positive.

- F11: CONFIRMED with your relocation. common.rs:236 `(domain, port).to_socket_addrs()` is a BLOCKING std resolver inside `validate_outbound_url`, executed on the async handler path (system.rs:201-203), and it actually affects EVERY guarded outbound path (config fetch-models too), not just the speed test — and it is worse under the speed-test fan-out. Combined with no `urls` cap (system.rs:45-47) + `join_all` over all (speedtest.rs:112). Fix = async/`spawn_blocking` DNS in the guard + cap `urls` + bound concurrency. Agreed?

No other open items remain. LOCKED CONSENSUS (severity = agreed). Confirm sign-off, or flag any final dissent:

| # | ID | Severity | Title |
|---|----|----------|-------|
| 1 | C1 | Critical | SPA-fallback path traversal → unauth arbitrary file read (PROVEN live) |
| 2 | C2 | Critical | No auth/CSRF/rate-limit on /api + shipped HOST=0.0.0.0 |
| 3 | F3 | High | WebDAV/S3 outbound lacks validate_outbound_url (guard exists, unapplied) |
| 4 | F5 | High (general) / Medium (this install) | Web startup skips config.json→SQLite migration |
| 5 | F6 | Medium | Web startup parity gaps: provider/official/MCP/prompts/skills auto-import omitted |
| 6 | F4 | Medium | ZenMux base_url SSRF (blind, needs attacker key) |
| 7 | F7 | Medium | Single available provider bypasses circuit-breaker half-open |
| 8 | F10 | Medium | WebDAV/S3 auto-sync toggle shown in web but backend is no-op stub |
| 9 | F11 | Medium | SSRF guard blocking DNS + unbounded /system/test_api_endpoints fan-out |
| 10 | F9 | Low-Medium | Web failover-enable semantics drift from desktop (no auto-add/switch/emit) |
| 11 | F8 | Low | Proxy start() check-then-start TOCTOU race |
| 12 | M1 | Low | select_providers issues 3-4 DB reads per forwarded request |
| 13 | L2 | Low | Dead empty model_fetch router still merged |

Dropped (agreed non-findings): L1 (streaming override), config.rs model-fetch SSRF (guarded), deplink.html key (upstream-only).

Final question: given this is a PERSONAL single-user Tailscale deployment (not multi-tenant), which 3-4 of these would YOU fix first, and why? Give your independent ranking — I'll compare it with mine for the user. Keep it short.
