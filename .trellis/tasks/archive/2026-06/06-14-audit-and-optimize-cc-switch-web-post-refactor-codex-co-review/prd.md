# PRD — Audit & Optimize cc-switch-web (post-refactor, codex co-review)

## Background

`cc-switch-web` is a web-first hard fork of the desktop Tauri app `farion1231/cc-switch`. The
owner refactored desktop→web; the production binary is `src-tauri/examples/server.rs`, accessed
from a second machine over **Tailscale at `http://100.75.197.120:3010/`**. The shipped systemd
unit binds `HOST=0.0.0.0:3010`. The desktop app is not used. This audit looks for problems and
optimization opportunities introduced (or left unaddressed) by the refactor.

## Methodology (why these findings are trustworthy)

- **Two independent auditors**: codex (`gpt-5.5` @ xhigh, read-only sandbox) ran a full independent
  audit; Claude audited independently via direct reads + **live probes against the running service**.
- **Adversarial debate, 2 rounds**: Claude cross-examined every codex finding against the code;
  codex replied point-by-point; the two open items were verified to closure.
- **Every finding was verified firsthand in code** (file:line). Three false positives were caught
  and dropped. Severities are the agreed consensus.

## Consensus findings (locked)

> Severity reflects THIS deployment: a single-user Tailscale service holding live provider secrets.

### 🔴 C1 — SPA-fallback path traversal → unauthenticated arbitrary file read — **Critical**
- **Evidence**: `web_api/routes.rs:91` `serve_spa_fallback`→`try_serve_dist_web_asset(uri.path())`;
  `:100` `rel = path.trim_start_matches('/')` (raw, undecoded); `:109-112` `dist_root.join(rel)`
  read **before** the `is_static_asset_path` gate, no canonicalization / `..` rejection (hand-rolled,
  not `tower_http::ServeDir`).
- **Proven live**: `curl --path-as-is http://127.0.0.1:3010/../../../../../../etc/hostname` returned
  the real `/etc/hostname`.
- **Impact**: any Tailscale/LAN peer reads any file the service user can —
  `~/.cc-switch/cc-switch.db` (all provider keys), `~/.claude/settings.json`, `~/.codex/auth.json`,
  `~/.ssh/*`. Total secret compromise, unauthenticated.
- **Fix**: reject `rel` containing `..`/absolute components, OR canonicalize and require
  `candidate.starts_with(canonical dist_root)`; best: serve the asset branch via
  `tower_http::services::ServeDir` (traversal-safe) and keep the index fallback for extensionless routes.

### 🔴 C2 — No auth/CSRF/rate-limit on `/api` + shipped `HOST=0.0.0.0` — **Critical**
- **Evidence**: `routes.rs:28-72` layers only TraceLayer/security_headers/cors/body-limit — no auth.
  `middleware/{auth,csrf,rate_limit}.rs` exist but are never `.layer()`-ed. `deploy/systemd/
  cc-switch-web.service:16,19` ship `HOST=0.0.0.0` + `ALLOW_HTTP_BASIC_OVER_HTTP=1` (a misnomer —
  no basic auth exists). Secret-returning/mutating endpoints (`providers.rs:209`, config SQL
  import/export `config.rs:683/728`, proxy takeover `proxy.rs:149/171`) are fully open.
- **Impact**: any tailnet/LAN peer steals provider keys, exports/imports the SQLite config, rewrites
  live CLI proxy settings, toggles takeover. (Amplifies C1, F3, F4, F11.)
- **Fix**: wire a real auth layer (bearer/shared-secret from env or settings) over `/api`, exempting
  `/api/health`; add CSRF on unsafe methods + a basic rate/concurrency limit; rename the env flag to
  reflect real protection. Interim posture: bind loopback + front with `tailscale serve`.

### 🟠 F3 — WebDAV/S3 outbound lacks the SSRF guard that other handlers use — **High**
- **Evidence**: `handlers/{webdav,s3}.rs` + `services/{webdav,s3}.rs` make **zero** `validate_outbound_url`
  calls (`webdav.rs:146`, `s3.rs:137` dial straight into sync services), while `config.rs:459-461` and
  `system.rs:202` DO guard. Inconsistent application of an existing guard.
- **Impact**: unauth tailnet peer makes the server probe/dial internal/loopback addresses (signed S3
  requests to arbitrary hosts).
- **Fix**: call `validate_outbound_url` on WebDAV base URLs and constructed S3 endpoints, with the
  existing `CC_SWITCH_WEB_SSRF_ALLOW` allow-list escape hatch.

### 🟠 F5 — Web startup skips legacy `config.json`→SQLite migration — **High (general) / Medium (this install)**
- **Evidence**: `server.rs:251` calls only `Database::init()`, which (`database/mod.rs:95-159`) does
  schema migration but never `migrate_from_json`; desktop `lib.rs:363-434` does `!has_db && has_json`
  → `MultiAppConfig::load()` → `db.migrate_from_json()`.
- **Impact**: a user upgrading from the pre-SQLite (v3.8-era) `config.json` who first launches the WEB
  binary gets an empty DB and the config is ignored; the now-created DB then suppresses desktop
  migration too. (This owner is SQLite-native — no `config.json` — so low personal impact.)
- **Fix**: port the pre-DB JSON migration into `server.rs`, or make `Database::init` perform an
  idempotent migration when the DB is empty and legacy JSON exists.

### 🟡 F6 — Web startup parity gaps: provider/official/MCP/prompts/skills auto-import omitted — **Medium**
- **Evidence**: desktop `lib.rs:446-709` runs `init_default_skill_repos`, skills-SSOT migration,
  `import_default_config` + `init_default_official_providers` per app, OpenCode/OpenClaw/Hermes/OMO
  import, MCP import, prompts import. Web `server.rs:251-318` does none of it.
- **Impact**: a fresh web install starts empty (no official presets, no auto-imported live CLI config /
  MCP / prompts / skills) — degraded first-run; everything must be added manually.
- **Fix**: extract the post-DB bootstrap into a tauri-free function called by both desktop and web,
  with web-safe (SSE) event reporting.

### 🟡 F4 — ZenMux `base_url` SSRF (blind, needs attacker key) — **Medium**
- **Evidence**: `subscription.rs:49-54` passes `base_url` through to `coding_plan.rs` →
  `query_zenmux(base_url,…)` (`:418-427`) dials it with no `validate_outbound_url`.
- **Fix**: guard `base_url` with `validate_outbound_url` in the web handler, or restrict ZenMux to
  trusted domains.

### 🟡 F7 — Single available provider bypasses circuit-breaker half-open — **Medium**
- **Evidence**: `forwarder.rs:377` `bypass_circuit_breaker = providers.len() == 1`; the comment assumes
  len==1 means "failover off", but with failover ON and only one *available* provider, an
  Open-past-timeout provider skips half-open probe limiting and receives full traffic.
- **Fix**: bypass only when failover is disabled by policy (pass an explicit flag), not when the
  selected list merely happens to have one element.

### 🟡 F10 — WebDAV/S3 auto-sync toggle shown in web but backend is a no-op stub — **Medium**
- **Evidence**: `SettingsPage.tsx:423-429` renders `WebdavSyncSection` with no web-mode flag; switches
  at `WebdavSyncSection.tsx:1083-1096,1370-1386`. Web `services/{webdav,s3}_auto_sync_web.rs`
  `notify_db_changed` is a no-op (intentional stub).
- **Impact**: users believe automatic backup/sync is active when only manual sync works.
- **Fix**: hide/disable the auto-sync controls in web mode (explicit "manual only" wording), or
  implement a tauri-free web background sync worker.

### 🟡 F11 — SSRF guard does blocking DNS + `/system/test_api_endpoints` unbounded fan-out — **Medium**
- **Evidence**: `common.rs:236` `(domain,port).to_socket_addrs()` is a BLOCKING std resolver inside
  `validate_outbound_url`, run on the async path (affects every guarded outbound call, not just speed
  test). `TestApiEndpointsRequest.urls` has no length cap (`system.rs:45-47`); `speedtest.rs:112`
  `join_all`s all URLs at once.
- **Fix**: resolve DNS via async or `spawn_blocking` in the guard; cap `urls`; bound fan-out with a
  semaphore/buffered stream.

### 🟢 F9 — Web failover-enable semantics drift from desktop — **Low-Medium**
- **Evidence**: web `failover.rs::set_auto_failover_enabled` rejects an empty queue and only flips the
  flag; desktop `commands/failover.rs` auto-adds the current provider, switches to P1, and emits
  `provider-switched`. Reachable in web UI (`App.tsx:1308-1309` FailoverToggle; `ProxyTabContent.tsx:190-193`).
- **Fix**: move the enable behavior into a shared service the web handler calls; emit the equivalent
  SSE event. (Partly intentional — confirm desired web behavior.)

### 🟢 F8 — Proxy `start()` check-then-start TOCTOU race — **Low**
- **Evidence**: `services/proxy.rs` `start()` reads `self.server` (None), awaits bind, then writes —
  no mutex spanning. Concurrent `POST /proxy/start-proxy-server` (open under C2) can double-start;
  with port 0, leaks a listener.
- **Fix**: a service-wide async start/stop mutex (or a state machine guard).

### 🟢 M1 — `select_providers` issues 3-4 DB reads per forwarded request — **Low**
- **Evidence**: `provider_router.rs:47/58/61` (config + all providers + queue) per request via
  `handler_context.rs:127-132`; `record_result:182` re-reads config.
- **Fix**: cache proxy_config / queue per app with invalidation on config change.

### 🟢 L2 — Dead empty `model_fetch` router still merged — **Low**
- **Evidence**: `handlers/model_fetch.rs:13-14` returns `Router::new()`; merged at `routes.rs:49`.
  Real model fetch is `config.rs`.
- **Fix**: delete the stub module + its merge.

## Dropped (agreed non-findings)
- **L1** (http_client 600s truncates streams): streaming overrides to 24h (`forwarder.rs:1721-1727`),
  hyper path uses its own timeout — 600s never caps the proxy hot path. (Claude false positive.)
- **config.rs model-fetch SSRF**: guarded at `config.rs:459-461`. (Claude false positive.)
- **deplink.html "leaked Context7 key"**: `ctx7sk-4ddd…` is upstream-only (`8f7423f0` on `upstream/main`,
  not an ancestor of the fork's `main`); fork tree/history carry only the placeholder. Not this user's leak.

## Verified-correct (NOT defects — do not "fix")
Random failover strategy (`provider_router.rs:138-156`, well-tested) · bounded graceful shutdown
(`server.rs:339-362`, test-pinned) · credential files written `0o600` (`config.rs:238`) · CORS
same-origin default · CSP/security headers · forwarder retry bounded by `max_attempts` · proxy
listener loopback-only enforcement.

## Recommended remediation phasing (Claude)

- **Phase 1 — security, do now** (the user is exposed today): C1 (traversal guard) + C2 (auth layer
  over `/api`). These two close the actual compromise. F3 + F4 + F11 (apply `validate_outbound_url`
  + cap fan-out) ride along as the same "outbound/exposure hardening" change.
- **Phase 2 — correctness/parity**: F5 (migration) + F6 (startup parity) extracted as a shared
  tauri-free bootstrap; F7 (breaker bypass flag).
- **Phase 3 — UX/polish/perf**: F10 (hide web auto-sync) · F9 (failover semantics) · F8 (start mutex)
  · M1 (config cache) · L2 (delete dead stub).

(codex's independent Phase-1 ranking is recorded in `research/codex-round2-output.md`.)
