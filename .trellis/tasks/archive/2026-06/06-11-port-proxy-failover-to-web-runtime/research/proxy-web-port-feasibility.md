# Research: Running the routing proxy + auto-failover inside the web-server runtime

- **Query**: What exactly does it take to run cc-switch-web's routing proxy + auto-failover inside the web-server runtime (`examples/server.rs`)?
- **Scope**: internal (repo code, v3.16.2, branch sync/upstream-v3.16.2 @ ddfa61cb)
- **Date**: 2026-06-11

All paths relative to repo root `/home/orion/Workspace/github/cc-switch-web` unless absolute.

---

## 1. Tauri-coupling inventory

Grep `tauri::|AppHandle|Emitter|app_handle` over `src-tauri/src/proxy/**` + `src-tauri/src/services/proxy.rs`. Every hit, what it does, and its web equivalent:

### 1.1 `src/proxy/forwarder.rs` (3412 lines — the hot path)

| Lines | What it does | Web equivalent |
|---|---|---|
| 24 | `use crate::commands::{CodexOAuthState, CopilotAuthState}` — newtype wrappers over `Arc<RwLock<CopilotAuthManager>>` / `Arc<RwLock<CodexOAuthManager>>` (defined `src/commands/copilot.rs:14`, `src/commands/codex_oauth.rs:16`). **`commands` module is desktop-only and absent from the web build → hard compile blocker.** | Inject the two `Arc<RwLock<Manager>>` directly (web `ApiState` already holds exactly these types, `src/web_api/state.rs:10-13`). |
| 32 | `use tauri::Manager` (for `.state::<T>()` service-locator) | Removed once managers are injected. |
| 95, 173, 191 | `app_handle: Option<tauri::AppHandle>` field + ctor param | Replace with a runtime-neutral context (see §3.3). |
| 281-287 | `handle_successful_response`: clones `app_handle`, passes to `failover_manager.try_switch(ah.as_ref(), …)` after a failover success | Pass `Arc<dyn UiEventSink>` (+ hot-switch callback) instead. |
| 1145-1170 | Copilot dynamic API endpoint: `app_handle.state::<CopilotAuthState>()` → `get_api_endpoint` | Injected `copilot_auth` Arc. |
| 1264-1311 | Copilot token fetch: `state::<CopilotAuthState>()` → `get_valid_token[_for_account]`; **errors out with "no AppHandle" if absent** → Copilot providers would hard-fail in web proxy if not injected | Injected `copilot_auth` Arc. |
| 1315-1363 | Codex OAuth token fetch: `state::<CodexOAuthState>()`; same hard-fail without handle | Injected `codex_oauth` Arc. |
| 1369-1392 | Gemini OAuth — explicitly process-global singleton, "no AppHandle needed" | Compiles as-is. |
| 1891-1922 | `apply_copilot_live_model_resolution`: `state::<CopilotAuthState>()`, soft-degrades (skip) without handle | Injected Arc; degraded behavior already defined. |
| 1925-1961 | `is_copilot_openai_vendor_model`: same pattern, soft fallback to chat/completions | Injected Arc. |
| 2464 | `app_handle: None` in `#[cfg(test)]` fixture | Update with struct change. |

### 1.2 `src/proxy/failover_switch.rs` (135 lines — entirely Tauri-shaped)

| Lines | What it does | Web equivalent |
|---|---|---|
| 12 | `use tauri::{Emitter, Manager}` | Remove. |
| 43, 76 | `app_handle: Option<&tauri::AppHandle>` params on `try_switch`/`do_switch` | Runtime context param. |
| 100-111 | `app.try_state::<crate::store::AppState>()` → `app_state.proxy_service.hot_switch_provider(...)` — **callback into ProxyService through the Tauri service locator** (this is how the proxy↔service cycle is broken on desktop) | Needs new design: inject a hot-switch handle (cloned `ProxyService` — it is `Clone`, all-Arc fields — or a small trait object). This is the one *real* design decision in the port. |
| 113-119 | `crate::tray::create_tray_menu(app, …)` + `tray_by_id(...).set_menu(...)` — tray refresh after failover switch | `UiEventSink::refresh_tray()` (default no-op already exists, `src/runtime/runtime_events.rs:23`); desktop `TauriEventSink` gains an override that rebuilds the tray. |
| 123-130 | `app.emit("provider-switched", {appType, providerId, source:"failover"})` | `sink.emit_json("provider-switched", …)` → broadcast → `GET /api/events` SSE. FE already listens (`src/lib/api/providers.ts:96`). The web switch handler already emits the same event name via sink (`src/web_api/handlers/providers.rs:307-313`), so payloads must stay shape-compatible (web one lacks `source`). |

### 1.3 `src/proxy/server.rs`

| Lines | What it does | Web equivalent |
|---|---|---|
| 41-42 | `ProxyState.app_handle: Option<tauri::AppHandle>` ("for events + tray") | Replace field with runtime context. |
| 60, 75 | `ProxyServer::new(..., app_handle)` ctor param | Same. |

Everything else in server.rs (hyper accept loop, header-case peek, router for `/v1/messages`, `/responses`, `/v1beta/*`, etc., lines 87-380) is tauri-free.

### 1.4 `src/proxy/handler_context.rs`

| Lines | What it does | Web equivalent |
|---|---|---|
| 225 | `create_forwarder` passes `state.app_handle.clone()` into `RequestForwarder::new` | Mechanical follow-on of the struct change. |

### 1.5 `src/proxy/response_processor.rs`

| Lines | What it does | Web equivalent |
|---|---|---|
| 954 | `app_handle: None` — **test-only** fixture (`build_state` inside `#[cfg(test)]`, imports at 827-833) | Update fixture. Non-test code is tauri-free. |

### 1.6 `src/services/proxy.rs` (desktop ProxyService, 5040 lines)

| Lines | What it does | Web equivalent |
|---|---|---|
| 18 | `use tauri::Emitter` | Remove. |
| 57-58, 72 | `app_handle: Arc<RwLock<Option<tauri::AppHandle>>>` field | Replace with `Arc<RwLock<Option<Arc<dyn UiEventSink>>>>` + auth-manager context. |
| 340-345 | `set_app_handle` (called from `lib.rs:439`) | `set_runtime_ctx(sink, copilot, codex, …)`; desktop calls it with `TauriEventSink`. |
| 390-391 | `start()`: `ProxyServer::new(config, db, app_handle)` | Pass context. |
| 662-670 | `set_takeover_for_app`: `handle.emit("proxy-official-warning", {appType, providerName})` when takeover targets an official provider | `sink.emit_json(...)`. FE already listens via adapter (`src/App.tsx:491`) and the adapter maps to SSE in web mode. |
| 2500-2501 | `update_config` restart path: `ProxyServer::new(..., app_handle)` | Pass context. |

**Total: ~17 coupled sites across 6 files.** None of them is load-bearing desktop UI logic except the tray refresh; all have a direct `UiEventSink`/dependency-injection equivalent. The OnceLock pattern in `src/usage_events.rs:34-50` (sink injected once at startup; call sites need no signature change) and `ChannelEventSink → /api/events` SSE (`src/web_api/handlers/system.rs:278-299`) are already proven in production for `usage-log-recorded`.

---

## 2. Module compile map

### 2.1 What `examples/web_proxy.rs` includes today (12 entries)

`types`, `error`, `gemini_url`, `http_client`, `json_canonical`, `model_mapper`, `providers` (whole subtree incl. `copilot_auth`, `codex_oauth_auth`, transforms, streaming), `session`, `sse`, `switch_lock`, `usage` (parser/calculator/logger) — plus an **inline-duplicated `circuit_breaker` module** (lines 34-58) carrying only `CircuitBreakerConfig` + `Default`.

**Inline-dup reconciliation (known silent-divergence point):** the inline defaults `{failure_threshold: 4, success_threshold: 2, timeout_seconds: 60, error_rate_threshold: 0.6, min_requests: 10}` (web_proxy.rs:47-57) currently **match** `src/proxy/circuit_breaker.rs:69-79` exactly — no drift today, but nothing enforces it. The port must delete the inline module and `#[path]`-include the real `circuit_breaker.rs` (it is tauri-free, zero crate-internal imports).

### 2.2 Full proxy needs (from `src/proxy/mod.rs:5-35`) — per-module status

| Module | In web shim? | Tauri-free as-is? | Notes / transitive deps |
|---|---|---|---|
| `body_filter` | no | yes | no crate imports |
| `cache_injector` | no | yes | used by forwarder:423 |
| `circuit_breaker` | inline dup | yes | swap dup → real file |
| `copilot_optimizer` | no | yes | used by forwarder:1047+ |
| `error` | yes | yes | |
| `error_mapper` | no | yes | no crate imports |
| `failover_switch` | no | **NO** | tauri Emitter/Manager, tray, AppState locator (§1.2) |
| `forwarder` | no | **NO** | `crate::commands::*State` + tauri::Manager + AppHandle (§1.1) |
| `gemini_url` | yes | yes | |
| `handler_config` | no | yes | deps: app_config, usage::parser (both in web build) |
| `handler_context` | no | yes* | *one line passes `state.app_handle` (225) |
| `handlers` | no | yes | deps all in-proxy + app_config; tauri-free directly |
| `health` | no | yes | placeholder struct only (7 lines) |
| `http_client` | yes | yes | |
| `hyper_client` | no | yes | no crate imports |
| `json_canonical` | yes | yes | |
| `log_codes` | no | yes | |
| `media_sanitizer` | no | yes | crate::provider only |
| `model_mapper` | yes | yes | |
| `provider_router` | no | yes | deps: database, error, provider, circuit_breaker — all web-safe |
| `providers/*` | yes | yes | already compiling in web build |
| `response_processor` | no | yes* | *test fixture only (954) |
| `server` | no | **NO** | one `Option<tauri::AppHandle>` field (§1.3) |
| `session` | yes | yes | |
| `sse` | yes | yes | |
| `switch_lock` | yes | yes | |
| `thinking_budget_rectifier` | no | yes | |
| `thinking_optimizer` | no | yes | |
| `thinking_rectifier` | no | yes | |
| `types` | yes | yes | |
| `usage/*` | yes | yes | logger→`services::usage_stats` (in web_services shim) + `usage_events` (in server.rs shim) |

**Compile blockers: exactly 3 modules** (`forwarder`, `failover_switch`, `server`) **+ 1 line in `handler_context` + the `services/proxy.rs` service layer.** The other ~19 missing modules `#[path]`-include cleanly today.

Other facts that matter for compilation:
- `Cargo.toml`: tauri/`tauri-plugin-*` are `optional = true`, gated by the `desktop` feature (lines 22-33); web-server feature already pulls `rand` (line 132/40) — relevant for the future random strategy slice.
- Crates the missing modules use (`hyper`, `flate2`, `toml_edit`, `futures`, `bytes`) are unconditional deps — no feature work needed.
- The dual-runtime drift guards in `examples/server.rs:363-476` only verify the **services** shim against cfg tags and that proxy-shim `#[path]`s resolve; the proxy shim list is "an intentional curated subset" — extending it is manual and gains no new automated protection beyond dangling-path checks.
- Per project memory: the desktop clippy/test CI gates do NOT compile the web example; `cargo check --no-default-features --features web-server --example server` (+ `cargo test ... --example server` for the parity tests) must be run per slice.

---

## 3. ProxyService surface

### 3.1 Desktop `src/services/proxy.rs` public methods (line numbers) vs web stub `src/services/proxy_web.rs`

| Method (desktop line) | In stub? | Stub behavior | Portability class |
|---|---|---|---|
| `new` (68) | yes | real | portable |
| `cleanup_claude_model_overrides_in_live` (81) | yes | no-op Ok | **pure logic** (file I/O via `crate::config`) |
| `sync_claude_live_from_provider_while_proxy_active` (323) | yes | no-op Ok | pure logic |
| `set_app_handle` (341) | no (n/a) | — | **desktop-only → becomes `set_runtime_ctx`/`set_event_sink`** |
| `lock_switch_for_app` (347) | yes | **real lock** (same `SwitchLockManager` contract) | portable, already shared |
| `start` (355) | yes | Err("unavailable") | needs ctx (passes handle to `ProxyServer::new`), otherwise portable |
| `start_with_takeover` (443) | yes | Err | pure logic (backup/sync/takeover via db + live files) |
| `get_takeover_status` (523) | yes | default | pure logic |
| `set_takeover_for_app` (562) | yes | Err | portable **except** the official-provider warning emit (662-670) → sink |
| `stop` (1008) | yes | no-op Ok | pure logic |
| `stop_with_restore` (1039) | yes | no-op Ok | pure logic |
| `stop_with_restore_keep_state` (1086) | yes | no-op Ok | pure logic |
| `detect_takeover_in_live_config_for_app` (1606) | yes | false | pure logic |
| `is_takeover_active` (1864) | yes | false | pure logic |
| `recover_from_crash` (1873) | yes | no-op Ok | pure logic |
| `detect_takeover_in_live_configs` (1897) | yes | false | pure logic |
| `update_live_backup_from_provider` (1989) | yes | no-op Ok | pure logic |
| `hot_switch_provider` (2061) | yes | silent no-op | pure logic (db + settings + live backup + `server.set_active_target`) |
| `hot_switch_provider_inner` (2070) | yes | silent no-op | pure logic; caller holds per-app lock |
| `switch_proxy_target` (2237) | yes | Err | pure (delegates to hot_switch) |
| `get_status` (2444) | yes | default | pure (reads ProxyServer state) |
| `get_config` (2457) | yes | default | pure (db) |
| `update_config` (2465) | yes | no-op Ok | needs ctx only for server restart path (2500-2501) |
| `is_running` (2553) | yes | false | pure |
| `update_circuit_breaker_configs` (2560) | yes | no-op Ok | pure |
| `reset_provider_circuit_breaker` (2576) | yes | no-op Ok (param order deliberately pinned, proxy_web.rs:159-169) | pure |
| `get_circuit_breaker_stats` (2593) | **MISSING from stub** | web route hardcodes `null` in the handler instead (`web_api/handlers/system.rs:250-258`) | pure |

**Classification summary:** only `set_app_handle` is desktop-semantics-only (and even it just stashes the event/auth context). Zero methods are tray-only — the tray refresh lives in `failover_switch.rs`, not ProxyService. Everything else is portable file/db/server-lifecycle logic.

### 3.2 Web routes that call the stub (`src/web_api/handlers/proxy.rs:63-119`)

- `GET  /proxy/get-proxy-status` → `proxy_service.get_status()`
- `GET  /proxy/get-proxy-takeover-status` → `get_takeover_status()`
- `GET  /proxy/is-proxy-running` → `is_running()`
- `POST /proxy/start-proxy-server` → `start()` (currently always Err in web)
- `POST /proxy/stop-proxy-with-restore` → `stop_with_restore()`
- `PUT  /proxy/set-proxy-takeover-for-app` → `set_takeover_for_app()`
- `POST /providers/switch-proxy-provider` → `switch_proxy_target()`
- `GET/PUT /config/get|update-proxy-config`, `get|update-global-proxy-config`, `get|update-proxy-config-for-app` → **db direct, bypass ProxyService** — note: desktop `update_proxy_config` command goes through `ProxyService::update_config` which hot-applies/restarts the running server; the web `update_proxy_config` handler (lines 209-220) writes db only. Once the proxy runs in web, this route must be re-pointed at `proxy_service.update_config` or a running server never sees config changes.
- `POST /system/get|set_default_cost_multiplier`, `get|set_pricing_model_source` → db direct.

Adjacent surfaces already present: failover queue CRUD (`handlers/failover.rs:31-55`, all db-direct — work unchanged), circuit-breaker config get/put (`handlers/config.rs:222-226` — note `update_circuit_breaker_config` at config.rs:664-678 ALREADY calls `proxy_service.update_circuit_breaker_configs`), reset/stats (`handlers/system.rs:82-85`), SSE `GET /events` (`system.rs:93`, 278-299).

### 3.3 Realistic split: shared core vs cfg-gating vs real proxy_web.rs

Three options evaluated against the constraint that **batch-5-fresh code must not fork** (commit 34cf2330 "codex auth preservation + takeover hardening" touched: takeover ownership detection `live_takeover_matches_current_proxy` proxy.rs:1738+, its use in `set_takeover_for_app` 580-609, `lock_switch_for_app` 347, `hot_switch_provider`/`_inner` outer/inner split 2061-2134, `preserve_codex_oauth_auth_in_backup` 2202):

1. **Duplicate a real impl into `proxy_web.rs`** — REJECTED. It would copy ~2.5k lines of takeover logic including all batch-5 hardening; every future upstream sync would need dual application. This is precisely the fork the task forbids.
2. **Extract a `proxy_core`** (new module both runtimes consume) — overkill. The coupling inventory (§1) shows the desktop file is already 99% runtime-neutral; moving 5k lines creates massive diff noise against upstream syncs for no structural gain.
3. **De-tauri `services/proxy.rs` + `src/proxy/{server,forwarder,failover_switch}.rs` in place, delete the stub** — RECOMMENDED.
   - Replace `Option<tauri::AppHandle>` with a small runtime context struct (suggested name `ProxyRuntimeCtx`), e.g. `{ sink: Option<Arc<dyn UiEventSink>>, copilot_auth: Arc<RwLock<CopilotAuthManager>>, codex_oauth: Arc<RwLock<CodexOAuthManager>>, hot_switch: <see below> }`, defined in a tauri-free location (`src/proxy/types.rs` or `src/runtime/`).
   - Desktop: `lib.rs:439` builds the ctx from `TauriEventSink::new(app.handle())` + the same Arcs it `app.manage()`s into `CopilotAuthState`/`CodexOAuthState` (lib.rs:854-866). `TauriEventSink` gains a `refresh_tray()` override (it already holds the handle; tray rebuild = `crate::tray::create_tray_menu` via `handle.try_state`, all under `#[cfg(feature="desktop")]` in runtime_events.rs — already a desktop-gated impl block).
   - Web: `examples/server.rs` already constructs both auth managers (lines 271-276) and the `ChannelEventSink` (279-287) — exactly the needed ingredients, in the right order.
   - The **hot-switch cycle** (failover_switch → ProxyService.hot_switch_provider): desktop breaks it via `app.try_state`. Replacement options: (a) pass a cloned `ProxyService` into the ctx at `start()` time — it's `Clone` with all-Arc fields; the resulting Arc cycle (service→server→failover_manager→service) is an app-lifetime singleton, leak-irrelevant; or (b) a 1-method trait `HotSwitchHandler` implemented by ProxyService to keep the dependency direction clean. Either preserves the single implementation of `hot_switch_provider_inner`.
   - `services/proxy.rs` then compiles in both runtimes; `web_services.rs:28-29` flips from `proxy_web.rs` to `proxy.rs`; `src/services/mod.rs` cfg-gates accordingly; `proxy_web.rs` is deleted (the parity test at server.rs:440-457 enforces the shim follows the cfg tags).
   - Residual cfg-gating inside proxy.rs: expected ZERO or near-zero lines (the `tauri::Emitter` import and emit site are fully replaced by the sink).

---

## 4. Lifecycle on a headless server

### 4.1 What desktop does (must be replicated)

- **Startup** (`src/lib.rs:907-935`, async task in `setup()`):
  1. `db.has_any_live_backup()` OR `proxy_service.detect_takeover_in_live_configs()` → `recover_from_crash()` (proxy.rs:1873-1891: restore live configs, `set_live_takeover_active(false)`, delete backups).
  2. `initialize_common_config_snippets` — **must run before takeover restore** (lib.rs:1609 comment), already called in `examples/server.rs:297`.
  3. `restore_proxy_state_on_startup` (lib.rs:1564-1605): for each app with `proxy_config.enabled == true` → `set_takeover_for_app(app, true)` (which auto-starts the proxy server, proxy.rs:567-571); on failure, clears that app's flag.
- **Shutdown** (`cleanup_before_exit`, lib.rs:1519-1554): if backups or takeover placeholders exist → `stop_with_restore_keep_state()` (restores live configs but KEEPS `proxy_config.enabled` so next launch re-takes over); else if running → plain `stop()`.

### 4.2 What `examples/server.rs` must add

- Order in `main()` (current: bootstrap → db → AppState → auth managers → sink → usage_events::init → snippets → serve, lines 257-306): insert after step "snippets" (so it reads real configs, mirroring lib.rs ordering — note desktop actually runs recovery BEFORE snippets; keep desktop's order: recovery → snippets → restore): crash-recovery check + `restore_proxy_state_on_startup` equivalent, plus injecting the `ProxyRuntimeCtx` into `app_state.proxy_service` before any of it.
- Graceful shutdown: today `shutdown_signal()` (318-345) just drains 50ms after SIGINT/SIGTERM. It must call the `cleanup_before_exit` equivalent (`stop_with_restore_keep_state` / `stop`) **after** `axum::serve` returns (or in the select before returning), so systemd `stop`/`restart` never leaves placeholder tokens (`PROXY_MANAGED`, proxy.rs:22) in `~/.claude/settings.json` etc.

### 4.3 systemd implications

- `deploy/systemd/cc-switch-web.service` restarts the unit on failure. Timeline on SIGKILL/OOM with takeover active: live configs keep placeholders pointing at the dead proxy port (default `127.0.0.1:15721`, types.rs:46) → local CLIs hard-fail until the unit restarts → startup recovery restores then re-takes over (gap = restart delay). Same semantics as a desktop crash, but on a headless box nobody sees a window; CLI breakage is the only signal. `TimeoutStopSec` must exceed proxy stop timeout (5s, proxy/server.rs:228) + restore writes.
- The data-dir flock (`bootstrap::acquire_data_dir_lock`, server.rs:262) guarantees single-instance recovery races cannot happen — and **also rules out running the proxy as a separate process sharing `~/.cc-switch`** (see §6 architecture note).
- Product semantics worth stating in the PRD: takeover rewrites live config files **on the server host**. The web UI in a browser elsewhere is only a remote control; the proxied CLIs are the ones running on that host (or explicitly pointed at `server:15721`). A non-loopback `listen_address` exposes an **unauthenticated forwarding proxy carrying real provider tokens** — and per project memory the web API's auth is itself no-op stubs. Default must stay loopback.

---

## 5. FE / web-API surface

### 5.1 What FE hides/disables in webMode today

- `src/components/settings/ProxyTabContent.tsx:43-45`: `runtimeControlsUnavailable = webMode`; effects: proxy toggle Switch disabled + amber "stays desktop-only for now" card (`ProxyPanel.tsx:262, 266-280` via `disableRuntimeControls` prop, line 133); failover info card "Web mode keeps failover in configuration-only mode" (168-193); `failoverPanelsDisabled = !isRunning && !runtimeControlsUnavailable` (45) — i.e. in web mode the queue/config panels are deliberately ENABLED for remote config-editing; amber "runtime stats unavailable" card at 269-283.
- `src/App.tsx:1297-1311`: header `ProxyToggle` + `FailoverToggle` wrapped in `!webMode && currentView === "providers" && …` — fully hidden in web mode.
- `ProxyPanel` takeover switches (255-333) are not webMode-gated themselves; they're unreachable because `isRunning` is always false via the stub.
- `useProxyStatus` (`src/hooks/useProxyStatus.ts:26-28`) polls `get_proxy_status` with `refetchInterval` 2s while running — works unmodified once the route returns real data.

### 5.2 Web routes that exist (full list)

`handlers/proxy.rs`: the 16 routes listed in §3.2. `handlers/failover.rs:31-55`: get-failover-queue, get-available-providers-for-failover, add-to/remove-from-failover-queue, get/set-auto-failover-enabled. `handlers/config.rs:222-226`: get/update-circuit-breaker-config. `handlers/system.rs:82-93`: reset_circuit_breaker, get_circuit_breaker_stats (hardcoded `null`, 250-258), `GET /events` (SSE).

### 5.3 Events

- Existing SSE event names FE listens for via the adapter (`src/lib/api/event-adapter.ts`, subscribe → `GET /api/events`): `usage-log-recorded` (useUsageEventBridge.ts:37), `provider-switched` (providers.ts:96), `proxy-official-warning` (App.tsx:491), `s3-sync-status-updated`, `universal-provider-synced`, `configLoadError`, `__lagged`.
- **No proxy-status SSE event exists in either runtime** — desktop FE also polls. So the port needs zero new event names: failover emits `provider-switched` (payload `{appType, providerId, source:"failover"}` — keep `source` for parity with desktop), takeover warning emits `proxy-official-warning`. Both flow through the sink automatically once §1 is done.
- `usage_events` is already initialized in server.rs:286, so proxy request logging (`proxy/usage/logger.rs:124` → `notify_log_recorded`) lights up the usage dashboard with no extra work.

### 5.4 Command/route coverage changes

- Expected: **no new command names**. `web-commands.ts` already maps every proxy/failover/circuit-breaker command (`get_proxy_status`:403, `start_proxy_server`:781, `switch_proxy_provider`:795, `get_circuit_breaker_stats`:236, etc.). `check:web-routes` (scripts/check-web-route-coverage.mjs) validates web-commands.ts names→paths only, and `commands.manifest.json` is desktop-side; neither changes when the stub behaviors become real.
- Two handler-level rewires ARE needed: `system.rs::get_circuit_breaker_stats` (drop hardcoded null → call `proxy_service.get_circuit_breaker_stats`, which also means adding that method when the stub dies) and `proxy.rs::update_proxy_config` (db-direct → `proxy_service.update_config`, §3.2).
- Locale keys: removing/softening the three web-mode notice cards touches `proxy.runtimeUnavailable*`, `proxy.failover.webConfigOnly*`, `proxy.failover.runtimeStatsUnavailable*` → `check:locales` gate.

---

## 6. Risk register + slicing proposal

### Architecture verdict

**In-process is the right call; a separate proxy process/binary is effectively blocked** by `bootstrap::acquire_data_dir_lock` (exclusive data-dir flock) plus shared in-memory state the proxy needs (rusqlite `Database` behind one lock, circuit-breaker state in `ProviderRouter`, auth-manager token caches). Nothing found makes the in-process port infeasible: the hot path is already 90% compiled into the web binary (providers/transforms/streaming/usage), and the remaining coupling is 17 sites in 6 files with an existing, production-proven sink abstraction.

### Slices (4 code slices + 1 follow-on)

**S1 — Tauri-free proxy core (no behavior change, desktop-only effect).**
Introduce `ProxyRuntimeCtx`; rewrite `server.rs` field, `forwarder.rs` (auth-manager injection + sink), `failover_switch.rs` (sink emit + `refresh_tray` + hot-switch handle), `handler_context.rs:225`, test fixtures. Add `TauriEventSink::refresh_tray`. Desktop `lib.rs:439` wires the ctx.
*Risk: HIGH — touches the 3.4k-line forwarder hot path and batch-5-adjacent code; any auth-manager injection mistake breaks Copilot/CodexOAuth proxying on desktop.* Gates: `cargo fmt --check`, desktop `cargo clippy` + `cargo test`, web `cargo check --no-default-features --features web-server --example server` (must stay green even though proxy isn't included yet).

**S2 — Dual-runtime ProxyService; delete the stub.**
De-tauri `services/proxy.rs` (§1.6), add `get_circuit_breaker_stats` parity, extend `examples/web_proxy.rs` with the ~19 missing modules, drop the inline `CircuitBreakerConfig` dup, flip `web_services.rs` to `proxy.rs`, delete `proxy_web.rs`, rewire `system.rs::get_circuit_breaker_stats` + `proxy.rs::update_proxy_config` handlers.
*Risk: MEDIUM — biggest diff; silent-divergence dup removed here; `dual_runtime_parity` tests pin the shim.* Gates: full local suite per memory (fmt ×2, desktop clippy/test, web check, `cargo test --example server`, `test:unit`, `test:integration`, `check:web-routes`).

**S3 — Headless lifecycle in `examples/server.rs`.**
Ctx injection order, crash recovery, `restore_proxy_state_on_startup` equivalent, graceful-shutdown restore; systemd unit `TimeoutStopSec` review; doc the placeholder-while-down window.
*Risk: MEDIUM — wrong ordering vs `initialize_common_config_snippets` corrupts the common-config snippet (lib.rs:1609); shutdown path must be idempotent with the flock held.* Gates: `test:integration` web-server suites + `smoke-web-server.mjs` + manual SIGTERM test with takeover active.

**S4 — FE un-hiding + events.**
Remove/conditionalize `runtimeControlsUnavailable` (ProxyTabContent 43-45, ProxyPanel 262-280), un-hide header toggles (App.tsx:1297), align `provider-switched` payload (`source` field), locale updates.
*Risk: LOW — pure FE; main hazard is leaving a disabled-state branch that hides a real error.* Gates: `pnpm format:check`, `check:locales`, `check:web-routes`, FE unit tests, browser smoke against the real server.

**S5 — Random/weighted routing strategy** — separate research covers the algorithm; lands on top of `provider_router.rs` (web-server feature already has `rand`).

### Top risks (ordered)

1. **Desktop hot-path regression in S1/S2** — forwarder + batch-5 takeover hardening (34cf2330) are freshly synced; the design deliberately avoids forking but the in-place rewrite still touches them. Mitigation: zero-behavior-change refactor first, full desktop gate suite per slice, upstream-sync-friendly minimal diffs.
2. **Security: second unauthenticated listener.** Proxy on `0.0.0.0` would forward with real tokens and no auth (and web API auth is itself stubbed per memory). Keep default loopback; refuse non-loopback `listen_address` in web mode without an explicit env opt-in, mirroring server.rs:248-255.
3. **Headless takeover residue** — server down ⇒ placeholder live configs with no UI to warn; document + ensure systemd auto-restart, graceful-stop restore.
4. **No CI for the web build** — every slice relies on locally-run web gates (deep-read finding H4); a CI job adding the web `cargo check`/`--example server` tests would de-risk all slices.
5. **`update_proxy_config` web handler bypass** (§3.2) — easy to miss; running server would silently ignore config changes.

## Caveats / Not Found

- `src/proxy/health.rs` is a 7-line placeholder ("占位实现") — listed in the task's module list but contributes nothing; no health-check loop exists to port.
- No `proxy-status-changed`-style event exists anywhere; status is poll-only in both runtimes (2s interval while running). If the PRD wants push status, that's NEW surface, not parity.
- Did not deep-verify every transitive import of `providers/*` (already compiling in web build — empirically tauri-free).
- `GlobalProxySettings` (outbound/upstream proxy, `global_proxy.rs` handlers + `proxy/http_client.rs::init` at lib.rs:879-899) is a separate subsystem; web server currently never calls `http_client::init` with the db-stored upstream proxy — worth folding into S3 startup parity but it is outside the strict failover scope.
