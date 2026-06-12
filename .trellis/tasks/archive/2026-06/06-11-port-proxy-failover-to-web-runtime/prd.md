# Port proxy + auto-failover to web runtime (random selection strategy)

## Goal

Make the local routing proxy (路由/接管) and auto-failover work in the fork's web-server runtime,
so the maintainer's headless Linux deployment (web UI via Tailscale, CLIs running on the same
server) gets automatic provider failover. Strategy requirement from the maintainer: NOT upstream's
sequential-queue failover, but claude-code-hub-style **random selection** — pick a random available
account; on failure, randomly pick another available account.

## What I already know

- **Current state (confirmed this session)**: the proxy stack is desktop-only. `services/proxy_web.rs`
  is a stub (`start()` → "proxy service is unavailable in web-server mode", proxy_web.rs:53); the
  proxy hot path (`proxy/forwarder.rs`, `proxy/handlers.rs`, `proxy/server.rs`) is NOT compiled into
  the web binary (`examples/web_proxy.rs` includes only ~11 modules: providers tree, types,
  switch_lock, usage parser, etc.). Failover engine (circuit_breaker, provider_router,
  failover_switch) executes inside the proxy request path → configuring failover in web UI persists
  DB flags (`in_failover_queue`) that nothing consumes.
- **Upstream failover semantics** (fork code, `proxy/provider_router.rs`): failover ON = iterate
  providers strictly in queue order (with circuit-breaker permits); OFF = current provider only,
  bypassing the breaker. The maintainer explicitly does NOT want this order-based behavior.
- **Desired semantics (claude-code-hub)**: random pick among available accounts; on request failure,
  random pick among remaining available; (exact failure classification / cooldown / stickiness to be
  established by research — the tool is installed locally somewhere on this machine and is OSS).
- **Existing web infra to build on**: `UiEventSink`/`ChannelEventSink` → SSE `/api/events` (batch
  v3.16.2 sync extended `proxy_web.rs` with a real `SwitchLockManager` + `lock_switch_for_app`);
  `web_api/handlers/proxy.rs` routes already exist and call public `ProxyService` methods (today they
  hit the stub); FE ProxyPanel/FailoverToggle exist (App.tsx hides some toggles in webMode).
- **Takeover on the server**: CLIs run on the same server; takeover = backup live configs + write
  proxy URL (default `127.0.0.1:15721`) + `PROXY_MANAGED` placeholder into live configs — the
  crash-recovery invariant (`live_takeover_active` persisted before rewrite) currently runs in
  desktop `lib.rs` setup(); the web runtime would need an equivalent in `examples/server.rs` startup.
- **Tauri deps to sever**: `services/proxy.rs` (3504 lines) holds `tauri::AppHandle` directly
  (events, tray refresh); tray refresh is desktop-only; events must route through `UiEventSink`.
- **D8 interaction**: the fork's forwarder works WITHOUT the deferred Codex Chat routing stack
  (desktop compiles it today); porting the hot path to web does not depend on the chat stack.
- Local install lead: `/home/orion/cliproxyapi/` contains `cli-proxy-api` + `config.yaml` (may be a
  different tool than claude-code-hub — research must locate the actual claude-code-hub install).

## Assumptions (temporary)

- Random-selection mode should live in shared code (provider_router) so desktop gets it too as an
  optional strategy; web is the primary target.
- The proxy listens loopback-only on the server (CLIs are local); no need to expose 15721 beyond
  127.0.0.1 in MVP.

## Open Questions

- (all resolved 2026-06-12; see D1-D4 and Research References)

## Decisions (ADR-lite)

**D1 — Random selection is a NEW selectable strategy, not a replacement (confirmed 2026-06-11).**
Context: maintainer wants claude-code-hub-style random failover; upstream semantics are strict queue order.
Decision: introduce a failover strategy setting (e.g. `FailoverStrategy::{Sequential, Random}`) in shared code (`proxy/provider_router.rs`); sequential keeps upstream semantics verbatim; random is the maintainer's default. Desktop gets the option automatically (shared code).
Consequences: minimal future upstream-sync conflict surface (sequential path untouched); needs a settings field + FE selector + i18n ×3; strategy must compose with the existing circuit breaker availability model. Caveat from R1: `rand` is currently feature-gated behind `web-server` — promote to a regular dependency (provider_router compiles in both runtimes).

**D2 — Stickiness: random + sticky-until-failure (confirmed 2026-06-12).**
Context: CCH does per-request weighted random + 300s Redis session affinity; the maintainer's literal requirement ("随机调用某一个账号，失败后随机调其他可用账号") is sticky-until-failure.
Decision: requests stay on the current provider; on failure the forwarder re-rolls uniformly among the remaining circuit-closed providers in the failover pool, and `failover_switch` persists the new pick as current — the fork's "current provider" IS the stickiness mechanism; zero new session machinery.
Consequences: best prompt-cache locality / lowest cost; no instantaneous load-balancing across accounts (accepted). Concretely: Random strategy = current-provider-first candidate order, then Fisher-Yates shuffle of the remaining healthy pool (vs sequential queue order).

**D3 — Architecture: de-tauri in place, ONE real ProxyService for both runtimes (from R2, 2026-06-12).**
Context: only ~17 tauri-coupled sites across 6 files block web compilation (forwarder.rs: AppHandle + desktop-only `commands::{Copilot,CodexOAuth}State`; failover_switch.rs: Emitter/tray/AppState locator; server.rs: one AppHandle field; handler_context.rs:225); batch-5 takeover code is fresh and must not be forked; separate-process proxy is ruled out by the exclusive data-dir flock.
Decision: introduce `ProxyRuntimeCtx` (UiEventSink handle + injected Copilot/CodexOAuth manager Arcs + hot-switch handle) replacing direct `AppHandle`; delete the `proxy_web.rs` stub; compile the real `services/proxy.rs` + full proxy module tree into both runtimes; reconcile the inline-duplicated `CircuitBreakerConfig` defaults in `web_proxy.rs`; rewire the 2 broken web handlers (`get_circuit_breaker_stats` hardcoded null, `update_proxy_config` bypasses ProxyService).
Consequences: single code path, no stub drift; desktop behavior must be regression-guarded (3.4k-line freshly-synced hot path); web runtime gains proxy lifecycle/recovery in `examples/server.rs`. No new commands or SSE event names (`provider-switched`/`proxy-official-warning` already plumbed).

**D4 — Web proxy listener is loopback-only in MVP (engineering default).**
Context: the web API itself has no auth; the proxy would be a second unauthenticated listener.
Decision: in web runtime, force/reject non-loopback proxy `listen_address` (CLIs run on the same server; loopback suffices). Desktop behavior unchanged.

## Requirements (evolving)

- Web runtime can start/stop the routing proxy and toggle takeover from the web UI.
- Auto-failover executes on the server with random selection among available providers.
- Proxy status/failover events reach the web UI via the existing SSE channel.

## Acceptance Criteria (final — verified 2026-06-12, trellis-check verdict: SHIP after 1 critical fix)

- [x] Routing master switch turns ON in web UI; proxy listens on the server (integration test starts/stops the real proxy through the rendered switch).
- [x] Random selection + auto re-selection: unit-tested (seeded-RNG determinism, current-first stickiness, circuit-open exclusion, distribution over 100 draws) + strategy persistence integration-tested. NOTE: request-level re-roll with a real dying upstream + uninterrupted CLI streams is only provable in a live multi-provider run — recommend manual validation after deploy.
- [x] Status/SSE events in web UI (event-adapter test); takeover/restore safe across restarts (crash-recovery idempotency + cleanup tests + empirical SIGTERM verification; SSE-blocked-shutdown Critical found by check and fixed: 5s bounded grace).
- [x] Full fork gate suite green: FE 601 unit / locales / routes 0 missing / manifest 266 unchanged; Rust desktop 1427+ / clippy -D warnings; web example suite 1372; build:web + smoke 123; integration 50.

## Final commit list (feat/web-proxy-failover)

- 260b5153 S1 ProxyRuntimeCtx de-tauri (18 sites, desktop byte-identical)
- 6dba3361 S2 dual-runtime ProxyService (stub deleted, shim 1:1, 2 handlers rewired)
- 37fd9598 S3 headless lifecycle + D4 loopback enforcement
- e44abf2d S4+S5 FE un-hiding + FailoverStrategy random
- ba53338b fix: bounded graceful shutdown (SSE could block cleanup → SIGKILL + stale placeholders)
- (+ spec update commit)

## Definition of Done

- Tests for the new selection strategy (unit) + web-runtime proxy lifecycle (integration).
- All CI gates green; spec updated (new scenario for web proxy runtime).

## Out of Scope (explicit, evolving)

- Codex Chat routing stack / model-catalog (separate deferred task).
- Exposing the proxy port beyond loopback; auth for the proxy port.
- Web auto-sync for S3/WebDAV (separate deferred item).

## Research References

- [`research/claude-code-hub-failover.md`](research/claude-code-hub-failover.md) — user runs CCH v0.8.4 (ding113/claude-code-hub, Docker :23000); algorithm: filter (enabled/format/circuit-closed/cost) → top priority tier → weighted random (uniform in user's config); failure: same-provider retry ×2 (100ms) → breaker (5-10 consecutive → open 60s → half-open → 2 ok closes) → re-roll among healthy, ≤20 switches, full request replay; 300s session affinity. Fork's forwarder/breaker/failover_switch already cover everything except the random pick. `/home/orion/cliproxyapi` is a different tool (CLIProxyAPI 6.10.9), not the reference.
- [`research/proxy-web-port-feasibility.md`](research/proxy-web-port-feasibility.md) — 3 blocked modules + 1 line (~17 sites / 6 files); `ProxyRuntimeCtx` de-tauri plan; ~19 other proxy modules already tauri-free; slice plan S1-S5; biggest risks: desktop hot-path regression, second unauthenticated listener (→D4).

## Implementation Plan (5 slices, from R2)

| # | Slice | Content | Risk |
|---|-------|---------|------|
| S1 | Tauri-free proxy core | `ProxyRuntimeCtx` introduced; de-tauri the ~17 sites in forwarder.rs / failover_switch.rs / proxy server.rs / handler_context.rs; desktop adapters keep behavior byte-identical | HIGH (desktop regression surface) |
| S2 | Dual-runtime ProxyService | delete `proxy_web.rs` stub; compile real `services/proxy.rs` in both runtimes; extend `web_proxy.rs` shim to the full module list; remove inline `CircuitBreakerConfig` duplicate; rewire 2 web handlers | MED |
| S3 | Headless lifecycle | proxy crash-recovery + `live_takeover_active` restore + graceful shutdown in `examples/server.rs` (mirror desktop `lib.rs` setup); D4 loopback enforcement | MED |
| S4 | FE un-hiding + events | remove webMode hiding of Proxy/Failover toggles (App.tsx/ProxyPanel); verify SSE status events end-to-end; i18n ×3 | LOW |
| S5 | Random strategy | `FailoverStrategy::{Sequential,Random}` (D1/D2 semantics) in provider_router + settings field + FE selector + unit tests; ungate `rand` | LOW-MED |

Gates per slice: full fork gate suite (typecheck/format/unit/locales/web-routes/manifest/fmt/clippy/cargo test/web example check+tests/smoke/integration), with emphasis on desktop proxy integration tests after S1/S2.

## Technical Notes

- Spec scenarios that bind this work: "Web Server Proxy Module Wiring" (dual `#[path]` wiring),
  "Desktop-Only Service Worker Twin Stubs" (cfg-pair conventions — this task INVERTS it: promoting
  a stub to a real web implementation), "Web Command Route Coverage" (route surface may grow),
  "Secret-Bearing Web API Commands".
- Prior session knowledge: fork architecture deep-read (2026-06), v3.16.2 sync (batch 5 touched
  proxy.rs heavily — takeover ownership/serialization is fresh code, build on it, don't fork it).
