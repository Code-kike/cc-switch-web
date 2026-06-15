**Part A — Reading Strategy**
1. Start at the real web runtime: `src-tauri/examples/server.rs`, not desktop `lib.rs`.
2. Compare web startup to desktop startup only for shared state: DB migration, provider import, proxy restore, event sinks.
3. Audit `src-tauri/src/web_api/routes.rs` next: router-wide auth, CSRF, rate limits, body limits, SPA fallback.
4. Prioritize mutating handlers: config import/export, providers, proxy, failover, settings, WebDAV/S3, subscription.
5. Follow each handler into service code; do not assume desktop command behavior carries over.
6. For proxy correctness, read `services/proxy.rs`, `proxy/forwarder.rs`, `provider_router.rs`, `circuit_breaker.rs`, `server.rs`, `http_client.rs`.
7. For refactor drift, inspect `examples/web_services.rs` and every `_web` shim.
8. For security, treat Tailscale as reachable network surface: no auth means every Tailnet client is an operator.
9. For performance/races, look for unbounded request fan-out, blocking DNS/filesystem on async paths, and shared `RwLock` check-then-act.
10. I avoided reporting the known loopback proxy listener, random failover semantics, graceful shutdown race, and global outbound proxy startup init as new bugs.

**Part B — Prioritized Findings**

[FINDING 1]
TITLE: SPA fallback can read arbitrary files outside `dist-web`
SEVERITY: Critical
CATEGORY: security
EVIDENCE: `src-tauri/src/web_api/routes.rs:91`, `src-tauri/src/web_api/routes.rs:99`, `src-tauri/src/web_api/routes.rs:109`, `src-tauri/src/web_api/routes.rs:110`, `src-tauri/src/web_api/routes.rs:173`, `src-tauri/src/web_api/routes.rs:192`, `src-tauri/src/web_api/routes.rs:193`, `deploy/systemd/cc-switch-web.service:16`
CLAIM: `uri.path()` is trimmed and joined to `dist_root`, then read before any canonicalization or containment check, so paths containing `../` can escape the static asset directory.
WHY-IT-MATTERS (web deployment): Any Tailscale-reachable client can request files readable by the service user, including `~/.cc-switch/cc-switch.db`, settings, shell files, or private material.
FIX: Before reading, reject `..`/absolute components or canonicalize both `dist_root` and candidate and require `candidate.starts_with(dist_root)`.
CONFIDENCE: High

[FINDING 2]
TITLE: Known no-auth API means Tailnet clients can exfiltrate and mutate all app state
SEVERITY: Critical
CATEGORY: security
EVIDENCE: `src-tauri/src/web_api/routes.rs:35`, `src-tauri/src/web_api/routes.rs:67`, `src-tauri/src/web_api/middleware/auth.rs:15`, `src-tauri/src/web_api/middleware/auth.rs:19`, `src-tauri/src/web_api/middleware/csrf.rs:15`, `src-tauri/src/web_api/middleware/csrf.rs:24`, `src-tauri/src/web_api/handlers/providers.rs:209`, `src-tauri/src/provider.rs:10`, `src-tauri/src/provider.rs:14`, `src-tauri/src/provider.rs:424`, `src-tauri/src/provider.rs:441`, `src-tauri/src/web_api/handlers/config.rs:683`, `src-tauri/src/web_api/handlers/config.rs:728`, `src-tauri/src/web_api/handlers/proxy.rs:149`, `src-tauri/src/web_api/handlers/proxy.rs:171`, `deploy/systemd/cc-switch-web.service:16`
CLAIM: This is the known limitation, but concretely the router exposes all handlers without auth/CSRF; provider responses serialize `settings_config` and universal `api_key`, SQL import/export is open, and proxy takeover/start/stop is open.
WHY-IT-MATTERS (web deployment): A Tailscale peer can steal provider keys, export/import the SQLite config, rewrite live CLI proxy settings, start/stop takeover, and disrupt local CLI usage.
FIX: Minimal fix is to wire `require_auth`, CSRF on unsafe methods, and rate/concurrency limits around `/api`, exempting only health/login/CSRF/SSE as needed.
CONFIDENCE: High

[FINDING 3]
TITLE: WebDAV and S3 sync endpoints are SSRF primitives
SEVERITY: High
CATEGORY: security
EVIDENCE: `src-tauri/src/web_api/handlers/webdav.rs:122`, `src-tauri/src/web_api/handlers/webdav.rs:137`, `src-tauri/src/web_api/handlers/webdav.rs:146`, `src-tauri/src/settings.rs:132`, `src-tauri/src/settings.rs:148`, `src-tauri/src/services/webdav.rs:32`, `src-tauri/src/services/webdav.rs:49`, `src-tauri/src/services/webdav.rs:131`, `src-tauri/src/services/webdav.rs:143`, `src-tauri/src/web_api/handlers/s3.rs:118`, `src-tauri/src/web_api/handlers/s3.rs:130`, `src-tauri/src/services/s3.rs:50`, `src-tauri/src/services/s3.rs:72`, `src-tauri/src/services/s3.rs:347`, `src-tauri/src/services/s3.rs:390`, `src-tauri/src/services/s3.rs:434`
CLAIM: The web handlers accept user-controlled WebDAV/S3 targets and dial them; WebDAV only checks `http/https`, and S3 custom endpoints preserve arbitrary hosts and `http://`.
WHY-IT-MATTERS (web deployment): A Tailnet client can make the server probe or send signed requests to loopback/private addresses reachable only from the server.
FIX: Apply the same outbound URL guard to WebDAV base URLs and constructed S3 bucket/object URLs, with an explicit allowlist for legitimate private storage endpoints.
CONFIDENCE: High

[FINDING 4]
TITLE: Coding-plan quota endpoint can SSRF arbitrary `zenmux` URLs
SEVERITY: High
CATEGORY: security
EVIDENCE: `src-tauri/src/web_api/handlers/subscription.rs:29`, `src-tauri/src/web_api/handlers/subscription.rs:49`, `src-tauri/src/web_api/handlers/subscription.rs:53`, `src-tauri/src/services/coding_plan.rs:22`, `src-tauri/src/services/coding_plan.rs:34`, `src-tauri/src/services/coding_plan.rs:616`, `src-tauri/src/services/coding_plan.rs:633`, `src-tauri/src/services/coding_plan.rs:656`, `src-tauri/src/services/coding_plan.rs:418`, `src-tauri/src/services/coding_plan.rs:422`
CLAIM: `base_url` is passed directly to `get_coding_plan_quota`; any URL containing `zenmux` is classified as ZenMux and fetched with `client.get(base_url)` without the web SSRF guard.
WHY-IT-MATTERS (web deployment): A caller can use a non-empty API key and `http://127.0.0.1:PORT/zenmux` or private host URLs to make the server dial internal services.
FIX: Validate `base_url` with `validate_outbound_url` in the web handler before calling the service, or restrict ZenMux to configured trusted domains.
CONFIDENCE: High

[FINDING 5]
TITLE: Web startup skips legacy `config.json` to SQLite migration
SEVERITY: High
CATEGORY: dual-runtime-drift
EVIDENCE: `src-tauri/examples/server.rs:247`, `src-tauri/examples/server.rs:251`, `src-tauri/src/database/mod.rs:95`, `src-tauri/src/database/mod.rs:104`, `src-tauri/src/database/mod.rs:120`, `src-tauri/src/lib.rs:357`, `src-tauri/src/lib.rs:363`, `src-tauri/src/lib.rs:416`, `src-tauri/src/database/migration.rs:49`, `src-tauri/src/database/migration.rs:62`
CLAIM: Desktop checks `!has_db && has_json`, loads `config.json`, and migrates providers/MCP/prompts/skills/common config; web creates/opens the DB directly, so an old `config.json` is ignored and the new empty DB suppresses future desktop migration.
WHY-IT-MATTERS (web deployment): Existing users starting the web binary after the refactor can appear to lose providers and related config.
FIX: Port the pre-DB JSON migration path into `examples/server.rs`, or make `Database::init` perform an idempotent migration when DB is empty and legacy JSON exists.
CONFIDENCE: High

[FINDING 6]
TITLE: Web startup omits desktop post-DB bootstrap imports
SEVERITY: Medium
CATEGORY: dual-runtime-drift
EVIDENCE: `src-tauri/examples/server.rs:251`, `src-tauri/examples/server.rs:293`, `src-tauri/examples/server.rs:300`, `src-tauri/examples/server.rs:311`, `src-tauri/examples/server.rs:318`, `src-tauri/src/lib.rs:446`, `src-tauri/src/lib.rs:457`, `src-tauri/src/lib.rs:526`, `src-tauri/src/lib.rs:545`, `src-tauri/src/lib.rs:568`, `src-tauri/src/lib.rs:641`, `src-tauri/src/lib.rs:686`, `src-tauri/src/database/schema.rs:933`
CLAIM: Web startup initializes DB/AppState and proxy lifecycle, but desktop also seeds default skill repos, consumes `skills_ssot_migration_pending`, imports live/default providers, seeds official providers, and imports MCP/prompts.
WHY-IT-MATTERS (web deployment): Fresh or upgraded web-only installs can miss expected providers, MCP servers, prompts, and skill migration that desktop users receive automatically.
FIX: Extract shared post-DB bootstrap into a tauri-free function and call it from both desktop and web, with web-safe event reporting.
CONFIDENCE: High

[FINDING 7]
TITLE: Single selected provider bypasses circuit-breaker half-open logic
SEVERITY: Medium
CATEGORY: correctness-bug
EVIDENCE: `src-tauri/src/proxy/provider_router.rs:56`, `src-tauri/src/proxy/provider_router.rs:78`, `src-tauri/src/proxy/circuit_breaker.rs:130`, `src-tauri/src/proxy/circuit_breaker.rs:137`, `src-tauri/src/proxy/circuit_breaker.rs:156`, `src-tauri/src/proxy/circuit_breaker.rs:175`, `src-tauri/src/proxy/forwarder.rs:376`, `src-tauri/src/proxy/forwarder.rs:399`, `src-tauri/src/proxy/circuit_breaker.rs:216`
CLAIM: `select_providers` can return a single failover candidate based on read-only `is_available`, but `forwarder` skips `allow_request` whenever `providers.len() == 1`, so Open→HalfOpen transition and probe limiting are bypassed.
WHY-IT-MATTERS (web deployment): A timed-out Open provider can receive normal traffic without the intended single half-open probe and may remain in inconsistent Open state after success.
FIX: Bypass the breaker only when failover is disabled/current-provider-only by policy, not merely when the selected list length is one.
CONFIDENCE: High

[FINDING 8]
TITLE: Proxy start has a check-then-start race
SEVERITY: Medium
CATEGORY: concurrency
EVIDENCE: `src-tauri/src/services/proxy.rs:57`, `src-tauri/src/services/proxy.rs:59`, `src-tauri/src/services/proxy.rs:411`, `src-tauri/src/services/proxy.rs:436`, `src-tauri/src/services/proxy.rs:449`, `src-tauri/src/services/proxy.rs:462`, `src-tauri/src/services/proxy.rs:619`, `src-tauri/src/services/proxy.rs:622`, `src-tauri/src/services/proxy.rs:626`, `src-tauri/src/web_api/handlers/proxy.rs:71`, `src-tauri/src/web_api/handlers/proxy.rs:149`
CLAIM: `start()` reads `self.server`, awaits bind/start, then writes `self.server`; concurrent web calls can both observe `None` and race to start.
WHY-IT-MATTERS (web deployment): Concurrent UI/API calls can produce spurious bind failures, or with port `0`, start two listeners and only track the last one.
FIX: Add a service-wide async start/stop mutex, or guard the state machine so only one start operation can be in flight.
CONFIDENCE: High

[FINDING 9]
TITLE: Web failover enable semantics drift from desktop
SEVERITY: Medium
CATEGORY: dual-runtime-drift
EVIDENCE: `src-tauri/src/web_api/handlers/failover.rs:119`, `src-tauri/src/web_api/handlers/failover.rs:123`, `src-tauri/src/web_api/handlers/failover.rs:129`, `src-tauri/src/web_api/handlers/failover.rs:142`, `src-tauri/src/web_api/handlers/failover.rs:146`, `src-tauri/src/commands/failover.rs:89`, `src-tauri/src/commands/failover.rs:100`, `src-tauri/src/commands/failover.rs:111`, `src-tauri/src/commands/failover.rs:147`, `src-tauri/src/commands/failover.rs:150`, `src-tauri/src/commands/failover.rs:154`, `src/lib/api/failover.ts:92`
CLAIM: Web rejects an empty queue and flips the flag only; desktop auto-adds the current provider when needed, switches immediately to P1, and emits `provider-switched`.
WHY-IT-MATTERS (web deployment): The same frontend action can leave the web proxy on a different provider than desktop semantics and can dead-end users who have not prebuilt the queue.
FIX: Move desktop failover-enable behavior into a shared service and have the web handler call it, emitting the equivalent SSE/UI event.
CONFIDENCE: High

[FINDING 10]
TITLE: Web UI exposes auto-sync toggles but web auto-sync workers are no-ops
SEVERITY: Medium
CATEGORY: dual-runtime-drift
EVIDENCE: `src-tauri/src/database/mod.rs:79`, `src-tauri/src/database/mod.rs:83`, `src-tauri/src/database/mod.rs:84`, `src-tauri/examples/web_services.rs:37`, `src-tauri/examples/web_services.rs:85`, `src-tauri/src/services/webdav_auto_sync_web.rs:27`, `src-tauri/src/services/s3_auto_sync_web.rs:1`, `src-tauri/src/services/s3_auto_sync_web.rs:34`, `src/components/settings/WebdavSyncSection.tsx:1083`, `src/components/settings/WebdavSyncSection.tsx:1091`, `src/components/settings/WebdavSyncSection.tsx:1369`, `src/components/settings/WebdavSyncSection.tsx:1378`
CLAIM: DB change hooks call web auto-sync modules, but the web modules’ `notify_db_changed` functions are empty while the UI still lets users enable WebDAV/S3 auto-sync.
WHY-IT-MATTERS (web deployment): Users can believe automatic backup/sync is active when only manual sync actually works.
FIX: Either implement a tauri-free web background worker or hide/disable auto-sync controls in web mode with explicit manual-sync wording.
CONFIDENCE: High

[FINDING 11]
TITLE: Endpoint speed test allows unbounded blocking DNS and request fan-out
SEVERITY: Medium
CATEGORY: performance
EVIDENCE: `src-tauri/src/web_api/handlers/system.rs:43`, `src-tauri/src/web_api/handlers/system.rs:46`, `src-tauri/src/web_api/handlers/system.rs:199`, `src-tauri/src/web_api/handlers/system.rs:201`, `src-tauri/src/web_api/handlers/common.rs:198`, `src-tauri/src/web_api/handlers/common.rs:236`, `src-tauri/src/services/speedtest.rs:26`, `src-tauri/src/services/speedtest.rs:70`, `src-tauri/src/services/speedtest.rs:73`, `src-tauri/src/services/speedtest.rs:82`, `src-tauri/src/services/speedtest.rs:112`, `src-tauri/src/services/speedtest.rs:128`, `src-tauri/src/web_api/routes.rs:67`
CLAIM: `/system/test_api_endpoints` accepts an unbounded URL vector, resolves domains with blocking `to_socket_addrs` in the async handler, then `join_all`s two GETs per valid URL with up to 30s timeout.
WHY-IT-MATTERS (web deployment): An unauthenticated Tailnet client can consume runtime threads and outbound sockets with one large request.
FIX: Add a per-request URL cap, use async DNS or `spawn_blocking` for resolution, and limit concurrency with a semaphore/stream buffer.
CONFIDENCE: High