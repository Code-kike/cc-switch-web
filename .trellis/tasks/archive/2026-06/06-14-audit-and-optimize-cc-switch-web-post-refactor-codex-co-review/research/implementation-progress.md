# Implementation progress

## Phase 1 — SECURITY — DONE + GATED (2026-06-14)
Files: web_api/{routes.rs, middleware/auth.rs, handlers/{common,config,system,subscription,webdav,s3}.rs}, examples/server.rs, deploy/systemd/cc-switch-web.service, scripts/install-cc-switch-web-service.sh

- C1 path traversal: `is_safe_relative_asset` guard in try_serve_dist_web_asset (routes.rs); rejects `..`/root/prefix components before join. Tests: is_safe_relative_asset_rejects_traversal, path_traversal_does_not_read_outside_dist_root.
- C2 auth: require_auth = HTTP Basic vs CC_SWITCH_WEB_AUTH_PASSWORD/USER (auth.rs), layered in build_router (routes.rs), server.rs refuses non-loopback bind unless is_configured(). systemd: ALLOW_HTTP_BASIC_OVER_HTTP removed, auth drop-in via install script (0600, generated once, restart not enable--now). Tests: ct_eq_basic, credentials_match_*.
- F3 webdav/s3 SSRF: guard_webdav_url / guard_s3_endpoint (skips empty=AWS-default) on all 4+4 network handlers.
- F4 zenmux/balance SSRF: validate_outbound_url on subscription get_balance + get_coding_plan_quota.
- F11: validate_outbound_url now async (tokio::net::lookup_host, non-blocking DNS); test_api_endpoints caps urls at 50.

Gates: web cargo check OK; clippy clean (touched files); 11 web tests pass; cargo fmt clean; git diff --check clean; check:web-routes missing:0. All edits web-only (web_api + examples) → desktop gates unaffected.

NOT yet deployed (user controls live-service restart). Deploy = re-run scripts/install-cc-switch-web-service.sh (builds+installs+generates password drop-in+restarts).

## Phase 2 — NEXT (not started)
- F7 breaker bypass: plumb `failover_enabled` through forward_with_retry/_inner + handler call sites; bypass only when failover OFF (not len==1). NOTE: forwarder is SHARED (desktop+web) → needs desktop gates too.
- F5 migration: web server.rs must run config.json→SQLite migrate_from_json (desktop lib.rs:363-434 does; Database::init does not).
- F6 bootstrap parity: extract lib.rs:446-709 post-DB bootstrap (skill repos, default/official providers, MCP, prompts, OMO, opencode/openclaw/hermes import) into a tauri-free shared fn called by both lib.rs (desktop) + server.rs (web). Biggest/riskiest; needs desktop+web gates + dual_runtime_parity tests.

## Phase 2 — F5 + F6 — DONE + GATED (2026-06-14)
Files: src/bootstrap.rs (shared fns), src/lib.rs (desktop call sites), examples/server.rs (web call sites), src/services/provider/mod.rs (un-gate one re-export).

- Shared module: `src/bootstrap.rs` (tauri-free; `pub mod bootstrap` desktop + `#[path]`-included by web example). Two new pub fns:
  - `apply_legacy_json_migration(db, config, json_path)` — F5 migrate core: migrate_from_json + set_migration_success + archive config.json→config.json.migrated. Caller owns the load step (desktop wraps it in the dialog/retry/exit loop; web logs+continues on load failure, no dialog/no exit — headless must come up).
  - `run_post_db_bootstrap(app_state: &AppState)` — F6: verbatim extraction of lib.rs steps 1/1.1/1.5/1.6/2/2.3/3/4 (skill repos, SSOT migration, live import + official seed, opencode/openclaw/hermes, OMO+OMO-Slim, MCP, prompts). All idempotent/table-empty-gated → safe to re-run every systemd boot.
- Desktop lib.rs: pure extraction. The post-load migration block (was lib.rs:412-434) → `bootstrap::apply_legacy_json_migration(&db, &config, &json_path)`. The import block (was lib.rs:424-691) → `bootstrap::run_post_db_bootstrap(&app_state)`. Dialog/retry/process::exit loop + `migrate_app_config_dir_from_settings(app.handle())` (AppHandle-coupled) stay in lib.rs. Same steps, same order, same logs.
- Web server.rs: F5 migration block added right BEFORE `Database::init()` (load+detect) and the migrate call right AFTER (mirrors desktop ordering); F6 `run_post_db_bootstrap(app_state.as_ref())` added after `AppState::new`, BEFORE `set_runtime_ctx` (matches desktop, before proxy lifecycle). `initialize_common_config_snippets` left untouched. Lifecycle ordering test `web_proxy_lifecycle::main_pins_proxy_lifecycle_ordering` still passes (new calls precede the first proxy marker).
- One cfg-gate change (NOT a desktop-only sub-step deferral): `services/provider/mod.rs:24-25` `#[cfg(feature="desktop")] pub use live::should_import_default_config_on_startup;` was desktop-only because only desktop called it; un-gated (the underlying fn in live.rs is already web-compiled) so the shared bootstrap fn resolves in BOTH runtimes. Matches the sibling ungated live re-exports.
- NO sub-step had to be cfg-gated as desktop-only: every service the bootstrap calls (skill, provider live fns, OmoService, mcp McpService, prompt PromptService, init_status) is web-compiled via the web shim. init_status confirmed tauri-free.

Gates (all verbatim PASS): cargo fmt --check OK; desktop `cargo clippy -- -D warnings` clean; desktop `cargo test --lib` 1430 passed/0 failed/2 ignored; web `cargo check --no-default-features --features web-server --example server` OK (66 pre-existing dead-code warnings only); web `cargo test ... -- dual_runtime_parity:: web_proxy_lifecycle::` 9 passed/0 failed; `pnpm check:web-routes` missing:0 (266 commands, unchanged); `git diff --check` clean.

## CHECK AGENT review — DONE (2026-06-14)

Reviewed all 17 changed files against prd.md findings + the binding scenarios in
quality-guidelines.md. C1/C2/F3/F4/F5/F6/F7/F11 implementations verified correct in code
(file:line traced). No business-code defects found. The check-agent fixes below are all in
the SMOKE TEST FIXTURE (`scripts/smoke-web-server.mjs`), surfaced by running
`pnpm build:web && pnpm smoke:web-server` — they are NOT product bugs.

### Smoke-fixture fixes applied (all caused by intended Phase-1/2 behavior, per the
### "Standalone Web-Server Smoke Validation" spec: probe categorized = test-fixture defect)

1. F6 startup-import idempotency: 7 probes assumed the web server imported nothing at
   startup and that the explicit import endpoints would report the first import. F6 now runs
   `run_post_db_bootstrap` at boot (desktop parity), so the explicit re-imports are
   idempotent and report 0 newly-imported. Relaxed the COUNT assertions (kept the
   authoritative `*-after-import` existence/merge checks):
   - `import-mcp-from-apps` (was `>= 5` → `>= 0`)
   - `import-opencode-from-live` / `import-openclaw-from-live` / `import-hermes-from-live`
     (was `=== 1` → `>= 0`)
   - `import-default-claude` / `import-default-codex` / `import-default-gemini`
     (was `=== true` → `typeof boolean`; idempotent import returns `false`).
2. F6 + harness isolation gap: `sessions-list` leaked the DEVELOPER'S REAL opencode
   session DB (`/home/orion/.local/share/opencode/opencode.db`). Root cause: the session
   scanner (`session_manager/providers/opencode.rs`) resolves via `XDG_DATA_HOME` /
   `dirs::home_dir()`, NOT `CC_SWITCH_TEST_HOME`, and the smoke spawn env never isolated
   `HOME`/`XDG_*`. Pre-existing latent gap, now exposed because the F6 path is exercised and
   the dev has a live opencode session today. Fix: the smoke spawn env now sets
   `HOME` + `USERPROFILE` + `XDG_DATA_HOME` + `XDG_CONFIG_HOME` to the temp home (mirrors the
   Rust example `TempHome`). Spec contract: "do not point the smoke server at a developer's
   real CLI configuration."
3. F4 SSRF guard ordering: `balance-unknown-provider` / `coding-plan-unknown-provider`
   probed with non-resolvable `*.example.com` hosts to exercise the "unknown provider"
   deterministic branch. F4 added `validate_outbound_url` (DNS resolution) BEFORE the
   matcher, so non-resolvable hosts now 400 before the branch runs. Fix: switched both to
   the resolvable public host `https://example.com` (passes the guard, still matches no known
   provider) so the unknown-provider branch is still exercised AND the guard is proven to
   pass legit public hosts.

### All gates re-run after fixes — verbatim PASS
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` → clean
- desktop `cargo clippy -- -D warnings` → clean (exit 0)
- desktop `cargo test --lib` → 1430 passed, 0 failed, 2 ignored
- web `cargo check --no-default-features --features web-server --example server` → OK (66
  pre-existing dead-code warnings only; none from the new bootstrap/auth/routes code)
- web `cargo test ... -- web_api:: dual_runtime_parity:: web_proxy_lifecycle::` → 20 passed,
  0 failed
- `pnpm check:web-routes` → missing:0 (266 commands, 256 routes)
- `pnpm typecheck` → clean
- `pnpm build:web && pnpm smoke:web-server` → exit 0 (all probes pass after fixture fixes)
- `git diff --check` → clean

### SPEC-UPDATE NOTES (for the spec-update step; NOT edited here)

The following are new/changed contracts that `.trellis/spec/frontend/quality-guidelines.md`
should capture. The closest existing scenarios are noted.

- NEW auth contract (C2) — add to a security scenario (near "Secret-Bearing Web API
  Commands" or a new "Web API Authentication" scenario):
  - The Web API enforces HTTP Basic Auth on `/api/*` EXCEPT exactly `/api/health` (exact
    equality, not `starts_with`) and non-`/api/` SPA assets, via
    `web_api/middleware/auth.rs::require_auth` layered in `routes.rs::build_router`.
  - Auth is enabled by `CC_SWITCH_WEB_AUTH_PASSWORD` (+ optional `CC_SWITCH_WEB_AUTH_USER`,
    default `cc-switch`); credential compare is constant-time (`ct_eq`).
  - `examples/server.rs` REFUSES a non-loopback bind unless `auth::is_configured()`.
  - The env flag `ALLOW_HTTP_BASIC_OVER_HTTP` is REMOVED from the systemd unit + install
    script (was a misnomer — no auth existed). Any spec/doc referencing it is stale.

- NEW path-traversal contract (C1) — the hand-rolled SPA static server in `routes.rs` must
  gate every disk read behind `is_safe_relative_asset(rel)` (rejects `..`/RootDir/Prefix
  components). `uri.path()` is NOT dot-segment-normalized; only `Component::Normal`/`CurDir`
  are allowed before `dist_root.join`.

- NEW SSRF-guard-coverage contract (F3/F4/F11) — extend the SSRF guidance:
  - `validate_outbound_url` is now `async` (non-blocking `tokio::net::lookup_host`); ALL
    callers must `.await`.
  - It must be applied on every outbound-dial web handler: webdav (4), s3 (4, skipping
    empty endpoint = AWS default), subscription `get_balance` + `get_coding_plan_quota` (2),
    config fetch-models (2, pre-existing). It does DNS, so an unresolvable host returns 400
    BEFORE any service-level branch — tests that relied on non-resolvable hosts to reach a
    downstream "unknown provider" branch must use a resolvable public host instead.
  - `/system/test_api_endpoints` caps `urls` at 50 (clean 400 over the cap).

- NEW shared-bootstrap contract (F5/F6) — add a scenario (or extend "Web Server Proxy
  Module Wiring" / "Upstream Desktop Sync Into Web Fork"):
  - `src/bootstrap.rs` holds the tauri-free `apply_legacy_json_migration` (F5) and
    `run_post_db_bootstrap` (F6); BOTH desktop (`lib.rs`) and web (`examples/server.rs`)
    call them. Desktop keeps the dialog/retry/process::exit loop around the JSON LOAD step;
    web logs-and-continues (headless must come up).
  - Web migration MUST run before `Database::init()` (so `!db_path.exists()` is still true).
  - `run_post_db_bootstrap` runs after `AppState::new` and BEFORE `set_runtime_ctx` /
    proxy lifecycle (pinned by `web_proxy_lifecycle::main_pins_proxy_lifecycle_ordering`).
    Every step is idempotent (table-empty / flag gated) so it re-runs safely on each boot —
    this is a CONTRACT the smoke fixture now depends on (explicit re-import endpoints return
    0 newly-imported after startup already imported).
  - `services/provider/mod.rs`: `should_import_default_config_on_startup` is now re-exported
    in BOTH runtimes (was desktop-only), required by the shared bootstrap.

- NEW circuit-breaker contract (F7) — add to a proxy scenario:
  - `bypass_circuit_breaker` is now `!failover_enabled` (sourced from
    `AppProxyConfig.auto_failover_enabled` via `RequestContext::failover_enabled()`), NOT
    `providers.len() == 1`. `failover_enabled` is plumbed through `forward_with_retry[_inner]`
    and passed at all 5 handler call sites (messages, chat_completions, responses,
    responses_compact, gemini).

- Smoke-isolation contract (harness) — the "Standalone Web-Server Smoke Validation"
  scenario should note that `scripts/smoke-web-server.mjs` must isolate HOME + XDG_DATA_HOME
  + XDG_CONFIG_HOME (not just CC_SWITCH_TEST_HOME), because session scanners reached via the
  F6 bootstrap resolve through `dirs::home_dir()`/XDG, not `CC_SWITCH_TEST_HOME`.

## Phase 3 — L2 + F8 + F10 + M1 + F9 — DONE + GATED (2026-06-14)

Files: web_api/{routes.rs, handlers/mod.rs, handlers/failover.rs}, (deleted)
web_api/handlers/model_fetch.rs, services/proxy.rs, commands/failover.rs,
proxy/provider_router.rs, proxy/handler_context.rs,
src/components/settings/WebdavSyncSection.tsx, src/i18n/locales/{en,ja,zh}.json.

- L2 (dead code): deleted `web_api/handlers/model_fetch.rs` (empty `Router::new()`
  stub), removed its `.merge(...)` in routes.rs:53 and `pub mod model_fetch;` from
  handlers/mod.rs; updated the mod.rs doc-count comment. Confirmed via grep that the
  only remaining `model_fetch` refs are `services::model_fetch` / `commands::model_fetch`
  (both real) and that `web-commands.ts` maps fetch-models to `/api/config/...`.
  `check:web-routes` still `missing:0`.

- F8 (proxy start/stop TOCTOU): added `start_stop_lock: Arc<tokio::sync::Mutex<()>>`
  to `ProxyService` (SHARED desktop+web), acquired at the top of `start()` and
  `stop()`. The check→bind→set in start() and check→take in stop() are now serialized;
  re-check of the running state inside `start()` (the existing `self.server.read()`
  guard) is preserved → single-call behaviour unchanged. `start_with_takeover` and
  `start_before_takeover_if_ephemeral_port` call `self.start()`/`self.stop()`
  SEQUENTIALLY (not nested while holding) → no re-entrant deadlock. `update_config`'s
  separate restart path was deliberately NOT touched (out of F8 scope; it already
  holds the `server` write-lock across its whole restart, and contends on the same
  RwLock as start's read, giving practical exclusion). Test:
  `concurrent_starts_bind_only_one_listener`.

- F10 (web auto-sync UX): `WebdavSyncSection.tsx` now computes `const webMode =
  isWebMode()` (`@/lib/api/adapter`). In web mode BOTH auto-sync `Switch`es (WebDAV
  ~1083, S3 ~1369) are `disabled` and rendered `checked={!webMode && ...}`, and their
  hint line shows the new desktop-only string instead of the normal hint. Manual
  sync/upload/download/test/save controls are untouched. Added i18n keys
  `settings.webdavSync.autoSyncWebDisabledHint` + `settings.s3Sync.autoSyncWebDisabledHint`
  to en/ja/zh (locale parity check green, 2356 keys each). Desktop unchanged
  (webMode=false there).

- M1 (perf, PARTIAL — documented deferral): added
  `ProviderRouter::select_providers_with_config(app_type, auto_failover_enabled,
  failover_strategy)`; `RequestContext::new` passes its already-loaded `app_config`
  fields → one fewer `get_proxy_config_for_app` read per forwarded request. Legacy
  `select_providers(app_type)` retained (reads config itself) for tests + future
  callers, marked `#[allow(dead_code)]` (production hot path uses `_with_config`).
  DEFERRED: `record_result`'s `circuit_failure_threshold` re-read — threading it
  needs plumbing through 6+ forwarder.rs retry-loop call sites (disproportionate
  churn for a Low finding); NO cache layer built (per instruction). Test:
  `select_providers_with_config_matches_db_read_path`.

- F9 (failover-enable parity, APPROACH = full shared extraction): new tauri-free
  `ProxyService::set_auto_failover_enabled(app_type, enabled)` in SHARED
  services/proxy.rs holds the strong-consistency enable semantics (auto-add current
  provider when queue empty → determine P1 → write flag → `switch_proxy_target` to P1
  → emit `provider-switched` + `refresh_tray` via the injected `UiEventSink`). Desktop
  `commands/failover.rs` now delegates to it (dropped the `AppHandle` param, inline
  `app.emit`, and inline tray rebuild — `TauriEventSink::{emit_json,refresh_tray}`
  does the identical work; removed unused `tauri::Emitter`/`FromStr` imports). Web
  `web_api/handlers/failover.rs` now calls the same method (was reject-empty + flip
  only); the web `ChannelEventSink` (already wired via `set_runtime_ctx` in
  examples/server.rs) broadcasts `provider-switched` over SSE, and the FE already
  listens runtime-neutrally (`src/lib/api/providers.ts:96`) so the web UI refreshes.
  Disable path unchanged (flip flag only, keep queue). Desktop behaviour preserved.
  Tests: `enable_auto_failover_on_empty_queue_auto_adds_current_provider`,
  `disable_auto_failover_keeps_queue_and_only_flips_flag`.

### Phase 3 gates (all verbatim PASS)
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` → clean (FMT_CHECK_OK)
- desktop `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` → clean
  (had to mark legacy `select_providers` `#[allow(dead_code)]` after M1 rewire)
- desktop `cargo test --manifest-path src-tauri/Cargo.toml --lib` → 1434 passed, 0
  failed, 2 ignored
- web `cargo check --no-default-features --features web-server --example server` → OK
  (pre-existing dead-code warnings only)
- web `cargo test ... --example server -- dual_runtime_parity:: web_proxy_lifecycle::`
  → 9 passed, 0 failed
- `pnpm typecheck` → clean
- `pnpm check:web-routes` → missing:0 (266 commands, 256 routes — unchanged by L2 removal)
- `pnpm check:locales` → in parity (2356 keys each, en/ja/zh)
- `git diff --check` → clean (DIFF_CHECK_OK)
- No WebdavSyncSection component unit tests exist in the repo → none to run.

## CHECK AGENT review — Phase 3 — DONE (2026-06-14)

Reviewed all Phase-3 working-tree changes (L2/F8/F9/M1/F10) against prd.md + the binding
scenarios in quality-guidelines.md. Implementations traced file:line and verified correct.

### Findings & fixes applied

1. F9 desktop tray-refresh equivalence (BUSINESS CODE — fixed). The original desktop
   `commands/failover.rs` refreshed the tray on BOTH enable and disable paths (the
   `create_tray_menu`+`set_menu` block sat OUTSIDE the `if enabled` block). The first Phase-3
   port placed `ctx.refresh_tray()` INSIDE `if enabled`, so the disable path no longer
   refreshed the tray — a behavioral drift from byte-for-byte desktop parity. Fixed in
   `services/proxy.rs::set_auto_failover_enabled` (~2400): moved `refresh_tray()` to an
   unconditional call AFTER the `if enabled` block (event emit stays enable-only, matching the
   original). In practice the tray content is identical on disable (no provider switch), so this
   was harmless, but it now matches desktop exactly. Desktop tests + lib suite re-green (1434).
   - NOTE: `switch_proxy_target` uses `?` in both old and new code, so on enable a failed switch
     short-circuits before the tray refresh in BOTH versions — equivalence preserved.

2. Smoke fixture: F9-driven (TEST FIXTURE, not product) — two updates to
   `scripts/smoke-web-server.mjs`, both direct consequences of F9 web ↔ desktop unification
   (per the "Standalone Web-Server Smoke Validation" spec: probe categorized = fixture defect):
   - `failover-enable-codex-without-queue-blocked` asserted the OLD web-only behavior (400 reject
     empty queue). F9 now delegates to the shared `set_auto_failover_enabled`, which on an empty
     queue auto-adds the current provider as P1 and returns 200 (desktop parity). Renamed →
     `failover-enable-codex-without-queue-auto-adds-current`; asserts 200 + queue auto-populated
     + flag set.
   - `failover-available-providers-claude` blindly picked `payload[0]` as the failover provider —
     after F6 bootstrap that is the seeded OFFICIAL provider (sortIndex 0). F9's switch-to-P1
     then hit `hot_switch_provider_inner`'s "cannot switch to official provider during proxy
     takeover" guard (identical to pre-F9 desktop; the old web path never switched, so it never
     hit it). Fixed to select the first NON-official available provider (the imported "Smoke
     Claude" third-party live provider) — a realistic failover queue. This is the same guard the
     real desktop enforces; the fixture now models a valid failover setup.

### No other defects

- F8: `tokio::sync::Mutex` (non-reentrant) is acquired ONLY at the top of `start()`/`stop()`.
  Every internal call site (`start_with_takeover`, `start_before_takeover_if_ephemeral_port`,
  `set_takeover_for_app`) calls `self.start()`/`self.stop()` SEQUENTIALLY (never while holding
  the lock); the inner restart in `start()`/`update_config` uses `ProxyServer::{start,stop}` on
  the inner instance, not `self`. No re-entrant deadlock. `update_config` deliberately does NOT
  take the lock and is serialized against `start()` via the `server` RwLock (write vs read). The
  running-state double-check after acquiring the lock is preserved. Verified correct.
- F9 shared method is tauri-free (events/tray via injected `UiEventSink` only); desktop wires
  `TauriEventSink` (emit==`handle.emit`, refresh_tray==identical menu rebuild), web wires
  `ChannelEventSink` (SSE broadcast → FE `providers.ts` listener). Disable path unchanged.
- M1: `select_providers_with_config` is the verbatim body of `select_providers` with the config
  passed in instead of re-read; reuses the per-request `app_config` loaded in
  `RequestContext::new` (same value F7's `failover_enabled()` reads). Legacy `select_providers`
  is still referenced by 9 test call sites → `#[allow(dead_code)]` justified (no prod callers).
- F10: both auto-sync switches `disabled` + `checked={!webMode && …}` in web mode; only those two
  controls are webMode-gated, manual upload/download/test/save stay live; i18n keys present in
  all 3 locales. Desktop unchanged (webMode=false).
- L2: no dangling `web_api::…::model_fetch` ref; the surviving `services::model_fetch` /
  `commands::model_fetch` are the real (unrelated) modules and must stay.
- Phase 1/2 intact: auth layer (`routes.rs:35`), traversal guard (`is_safe_relative_asset`),
  F7 `!failover_enabled` bypass, F5/F6 bootstrap — none touched by the Phase-3 diff.

### Gates re-run after fixes — verbatim PASS
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` → clean (FMT_EXIT=0)
- desktop `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` → clean (exit 0)
- desktop `cargo test --manifest-path src-tauri/Cargo.toml --lib` → 1434 passed, 0 failed,
  2 ignored
- web `cargo check --no-default-features --features web-server --example server` → OK (66
  pre-existing dead-code warnings only)
- web `cargo test … --example server -- web_api:: dual_runtime_parity:: web_proxy_lifecycle::`
  → 20 passed, 0 failed
- `pnpm typecheck` → clean
- `pnpm check:web-routes` → missing:0 (266 commands, 256 routes)
- `pnpm check:locales` → in parity (2356 keys each, en/ja/zh)
- `pnpm build:web && pnpm smoke:web-server` → exit 0 (all probes pass after the two F9 fixture
  fixes). ENV NOTE: the pinned rustup toolchain re-syncs under the smoke harness's isolated temp
  `HOME`; set `RUSTUP_HOME`/`CARGO_HOME` to the real dirs (or pre-install the toolchain) so
  `cargo run` resolves without a network round-trip — otherwise the server times out before
  `/api/health` (toolchain flake, NOT a product defect; direct binary boot returns health 200).
- `git diff --check` → clean

### SPEC-UPDATE NOTES (Phase 3 — for the spec-update step; NOT edited here)

Closest existing scenario to extend: a proxy scenario (near "Failover Circuit-Breaker Bypass
Policy" / the failover-strategy contract in "Web Server Proxy Module Wiring").

- NEW F9 cross-runtime failover-enable contract — `ProxyService::set_auto_failover_enabled(
  app_type, enabled)` is the single tauri-free SSOT for the enable/disable semantics; BOTH
  `commands/failover.rs` (desktop) and `web_api/handlers/failover.rs` (web) delegate to it.
  - On `enabled=true` with an EMPTY queue it auto-adds the current provider as P1 (errors only if
    there is no current provider at all), writes the flag, `switch_proxy_target`s to P1, then
    emits `provider-switched` (payload `{appType, providerId, source:"failoverEnabled"}`) via the
    injected `UiEventSink` (desktop=Tauri bus, web=`ChannelEventSink` SSE).
  - The tray is refreshed on BOTH enable AND disable (desktop `TauriEventSink::refresh_tray`;
    web no-op) — moving it inside `if enabled` is a desktop-parity regression → reject.
  - `enabled=false` only flips the flag and KEEPS the queue (no switch, no event).
  - The method must stay tauri-free (no `tauri::`/`Emitter`/`AppHandle`); the web handler maps
    its `Result<_, String>` via `ApiError::from_service_message`.
  - Switching to an OFFICIAL provider is rejected by `hot_switch_provider_inner` ("Cannot switch
    to official provider during proxy takeover"); a valid failover queue must hold third-party
    providers (the smoke fixture now selects a non-official P1).

- NEW F8 start/stop serialization contract — `ProxyService` holds a service-wide
  `start_stop_lock: Arc<tokio::sync::Mutex<()>>` acquired at the top of `start()` and `stop()`
  to serialize the check→bind→set / check→take sequences. It is NON-reentrant: no path may call
  `self.start()`/`self.stop()` while holding it (internal callers call them sequentially; inner
  restarts use `ProxyServer::{start,stop}` on the inner instance). `update_config` deliberately
  does not take it (serialized via the `server` RwLock). The running-state double-check after
  acquiring the lock must be preserved.

- NEW M1 config-reuse contract — the forward hot path calls
  `ProviderRouter::select_providers_with_config(app_type, auto_failover_enabled,
  failover_strategy)` reusing the per-request `AppProxyConfig` already loaded in
  `RequestContext::new`, instead of `select_providers` (which re-reads `proxy_config`). The two
  must produce byte-identical candidate lists for the same config (read-dedup only). DEFERRED:
  `record_result`'s `circuit_failure_threshold` re-read (Low; not threaded — disproportionate
  churn).

- NEW F10 web-mode auto-sync gating contract — in `WebdavSyncSection.tsx`, web mode
  (`isWebMode()`) disables BOTH auto-sync switches (`disabled`, `checked={!webMode && …}`) and
  shows `settings.{webdavSync,s3Sync}.autoSyncWebDisabledHint` (the auto-sync worker is a no-op
  stub in web). Manual upload/download/test/save must stay ungated; desktop unchanged. i18n keys
  required in en/ja/zh.

- L2 — the empty `web_api/handlers/model_fetch.rs` stub router is removed; real model fetch is
  `config.rs` at `/api/config/fetch-models-for-config`. `services::model_fetch` /
  `commands::model_fetch` are unrelated and remain.

- Smoke-fixture contract (extend "Standalone Web-Server Smoke Validation") — failover smoke
  probes must model desktop-equivalent enable semantics: enabling on an empty queue auto-adds the
  current provider (200, not 400), and the failover queue P1 must be a NON-official provider
  (official P1 is rejected by the switch guard).


---

## Phase 4 spec-update notes (check-agent verified 2026-06-15)

These reflect the new P4 contracts; fold into `.trellis/spec/frontend/quality-guidelines.md`
(do NOT auto-edit the spec — record here, owner applies).

- NEW ip_guard SSOT contract (extend "Web Request Hardening") — the IP-range block classifier is
  the SINGLE tauri-free source of truth in `proxy/ip_guard.rs`
  (`is_blocked_ipv4`/`is_blocked_ipv6`/`is_blocked_ip`), shared by BOTH
  `web_api/handlers/common.rs::validate_outbound_url` (re-export) AND the web-outbound redirect
  policy in `proxy/http_client.rs`. It must stay IO-free (no DNS, no syscalls) so it is safe to
  call from a sync redirect callback. Blocked ranges now also include the unspecified address
  (`0.0.0.0` / `::` / `::ffff:0.0.0.0`, which route to localhost on Linux) and the CGNAT/Tailscale
  shared range `100.64.0.0/10` (mask `octets[1] & 0xc0 == 0x40`; `100.63.*` and `100.128.*` are
  NOT blocked). Module must be mirrored in `proxy/mod.rs` (`pub mod ip_guard;`) AND the web shim
  `examples/web_proxy.rs` (`#[path] pub mod ip_guard;`) — enforced by
  `dual_runtime_parity::web_proxy_shim_mirrors_proxy_mod_modules`.

- NEW get_guarded redirect-policy contract (extend "Web Request Hardening") — `http_client.rs`
  exposes a SECOND client `get_guarded()` alongside `get()`. The guarded client installs a custom
  `reqwest::redirect::Policy` that re-classifies EACH redirect hop's host: an IP literal that
  `ip_guard::is_blocked_ip` rejects aborts the chain (`attempt.stop()`); public→public follows
  within the default 10-hop budget. The SHARED forwarder/proxy hot path keeps the UNGUARDED
  `get()` so upstream-3xx pass-through is unchanged. The web outbound SERVICES — `services/{balance,
  coding_plan,model_fetch}.rs` — must dial via `get_guarded()`; the user-URL subscription handlers
  (`get_balance`/`get_coding_plan_quota`) keep their initial-URL `validate_outbound_url` AND reach
  the guarded client through those services (defense in depth). `init`/`apply_proxy`/`update_proxy`
  MUST keep BOTH clients in lock-step (a proxy change rebuilds both). DOCUMENTED RESIDUAL: a
  redirect to a DOMAIN (not an IP literal) resolving internal is NOT caught — the sync redirect
  callback can't resolve DNS; same DNS-rebinding deferral as the initial guard (auth-gated).

- NEW per-handler outbound guards (extend "Web Request Hardening" call-site list) —
  `usage.rs::test_usage_script` guards `request.base_url` ONLY when `Some(non-empty)` (empty/None
  falls back to the provider's own base_url, not user-supplied; desktop path unguarded). BOTH
  stream-check variants in `providers.rs` guard the resolved base URL via the NEW
  `services/stream_check.rs::resolve_outbound_base_url(app_type, provider)`, which mirrors the
  exact per-app base-url derivation that the dial uses (Claude/Codex/Gemini via
  `get_adapter().extract_base_url`; OpenClaw/Hermes/OpenCode via their `extract_*_base_url`
  helpers). `stream_check_all_providers` records a per-provider Failed result on rejection
  (preserving result order/length); `stream_check_provider` guards only when resolution succeeds
  (a resolution failure means the dial also fails to extract → never dials).

- NEW S3 schemeless-endpoint normalization (extend "Web Request Hardening" S3 note) —
  `s3.rs::guard_s3_endpoint` runs `normalize_s3_endpoint_for_guard` before validating: a bare
  `host:port` (the documented MinIO form) becomes `https://host:port`, EXACTLY mirroring
  `services/s3.rs::split_scheme_host`'s default-to-https. This restores the P4-B regression where
  `Url::parse` mis-read the bare host as a scheme and 400'd a valid MinIO endpoint. Explicit
  `http://`/`https://` are preserved untouched (no bypass: `http://evil` stays http and is still
  DNS-resolved + IP-checked). Empty endpoint = AWS default (public) → skip guard.

- CSRF removal (REPLACE the "no-op CSRF" wording in the C2 auth scenario / mod.rs doc) — the CSRF
  theater is fully DELETED, not wired: `middleware/{csrf,rate_limit}.rs`, the `/system/csrf-token`
  route + `csrf_token` handler, the FE `adapter.ts` X-CSRF token cache/refresh/retry plumbing
  (`setCsrfToken`/`getCsrfToken`/`refreshCsrfToken`/`fetchWithCsrfRetry`), and the cors
  `x-csrf-token` allowed header are all removed. Mutating `/api` calls now `fetch` directly with
  `credentials: "include"`; protection is HTTP Basic auth only (C2, unchanged). Practical risk was
  LOW (Basic auth, not cookies). Spec must NOT advertise CSRF/rate-limit/cookie-session.

- Stub-router cleanup (extend L2) — beyond `model_fetch`, the empty Layer-2 stub routers
  `copilot`, `vscode`, `model_test`, and `universal` are deleted (files + `handlers/mod.rs` decls +
  `routes.rs` merges). Their FE commands are `unsupported: true` (copilot) or served by the real
  `providers` router (`*_universal_provider` → `/api/providers/...`); the real desktop
  `commands/copilot.rs` is untouched. `web_api/mod.rs` now documents 25 router-exporting handler
  modules (26 files incl. the non-router `common`). `pnpm check:web-routes` stays `missing: 0`.

- TEST contract (add to F10 scenario "Tests Required") — desktop-behavior component tests for
  `WebdavSyncSection` (auto-sync toggle saves true, auto-error callout renders) MUST pin desktop
  mode by setting `window.__TAURI_INTERNALS__`/`window.__TAURI__` in `beforeEach`. Plain jsdom has
  no Tauri globals, so `isWebMode()` defaults to TRUE and the F10 gating would otherwise hide those
  controls and fail the assertions. (Fixed here: `tests/components/WebdavSyncSection.test.tsx`.)
