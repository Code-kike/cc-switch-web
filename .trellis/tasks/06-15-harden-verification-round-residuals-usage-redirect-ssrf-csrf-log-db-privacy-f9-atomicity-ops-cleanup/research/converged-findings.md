# Converged findings — three-way re-audit of `fix/web-audit-phase1-2`

Provenance: **Workflow** 6-dimension review (`workflow-6dim-review.json`) + **codex** gpt-5.5 blind review (`codex-blind-review.md`) + main-agent **firsthand** code verification. Cross-validated to convergence in one round — all factual claims agreed across sources; the only deltas were severity calibrations (adjudicated below with code evidence). This file is the evidence record; `prd.md` holds the actionable fix spec.

## Severity-delta adjudications (the only cross-source disagreements)
1. **Usage SSRF #1** — Workflow=high, codex=medium → **MEDIUM-HIGH** (operator-gated lowers from high, but it's a clean bypass of the audit's own P4-A1 guard reaching cloud-metadata/internal with read-back; cheapest+highest-value fix).
2. **ip_guard exotic ranges #8** — codex="sound", Workflow+main="info gaps". No real contradiction: codex spoke to the COMMON ranges (correct — RFC1918/loopback/metadata/CGNAT all covered); Workflow+main caught exotic ranges (6to4/NAT64/`::a.b.c.d`/multicast/`0.0.0.0/8`). → **INFO** (core sound, exotic optional hardening).
3. **F9 enable atomicity #4** — codex=fix-regression, Workflow=benign+not-a-regression. Both true: real non-atomic enable on the switch-error path (codex), but routing stays consistent + matches old desktop ordering (Workflow). → **LOW** (real, recoverable, optional fix).

## Firsthand-verified evidence (main agent read these lines directly)

### #1 usage custom-template SSRF (MEDIUM-HIGH) — `usage_script.rs`, `web_api/handlers/{usage,providers}.rs`
- `usage_script.rs:614` `fn should_validate_base_url(base_url,is_custom_template){ !base_url.is_empty() && !is_custom_template }` → custom ⇒ false (skip base_url validate).
- `usage_script.rs:644` `if !is_custom_template && parsed_request.scheme()!="https" && !is_request_loopback {..err..}` → custom skips HTTPS.
- `usage_script.rs:654` `if !base_url.is_empty() && !is_custom_template {..same-origin check..}` → custom skips same-origin ⇒ `request.url` = any host.
- `usage_script.rs:366` `let client = crate::proxy::http_client::get();` then `:379 client.request(method,&config.url)` — unguarded dial of script-controlled url, body read back at `:403 resp.text()`.
- `usage.rs:330-334` test_usage_script validates only `request.base_url` (and only when non-empty), never the script's `request.url`.
- `providers.rs:517-529` query_provider_usage → `query_usage_with_templates` with NO validate (built-in templates dial guarded balance/coding_plan; the custom path hits the same unguarded `send_http_request`).
- `is_loopback_host` (usage_script.rs:712) only `.is_loopback()` — misses 169.254/10/CGNAT.

### #2 redirect-hardening incomplete (MEDIUM) — services use unguarded `get()`
get_guarded() callers (CORRECT): `services/{coding_plan.rs:92,259,340,419 ; balance.rs:69,141,193,258,318 ; model_fetch.rs:64}`.
Unguarded `http_client::get()` on user/redirect-influenced URLs (TO FIX): `services/stream_check.rs:256,710 ; webdav.rs:134,173,231,258,302 ; s3.rs:335,377,422,488 ; speedtest.rs:122` (build_client). `get()` uses reqwest default redirect (≤10 hops, no IP recheck) → validated-public-URL → 302 → internal followed.
SOUND (Workflow-verified, leave): `subscription.rs:324,659,957,1038` dials only hardcoded vendor hosts (api.anthropic.com / chatgpt.com / oauth2.googleapis.com / cloudcode-pa.googleapis.com) — not user-influenced.

### #3 upstream error body → request-log DB untruncated (LOW) — R2 gap
`proxy/handlers.rs:1041 log_forward_error` → `:1051 let error_message = get_error_message(error)` (UpstreamError ⇒ `error_mapper.rs:69 format!("上游错误 ({status}): {body}")`, full body) → `:1054 logger.log_error_with_context(...,error_message,...)` → `usage/logger.rs:194 error_message: Some(error_message)` → `:45/:92/:113 log_request` INSERT to DB. R2 only truncated `handlers.rs:332` (`compact_error_message(&body_str,180)`, journal line) + `:723` (1800). `compact_error_message` defined `handlers.rs:861` (char-count-safe).

### #4 F9 enable non-atomic (LOW) — `services/proxy.rs`
`:2394 config.auto_failover_enabled = enabled` → `:2395-2398 update_proxy_config_for_app(config).await` (DB write) BEFORE `:2402 self.switch_proxy_target(app_type,&p1_provider_id).await?` (can Err on official-provider-during-takeover). Emit `provider-switched` (`:2406`, inside `if enabled`) + unconditional `:2419 refresh_tray()`.

### #5 Basic-auth CSRF on no-body POST (LOW) — `auth.rs`/`proxy.rs`/`cors.rs`
`auth.rs:110-128 require_auth` checks only Authorization, exempts by path not method. `cors.rs:24 allow_credentials(true)`, default no allow_origin (same-origin responses only, but simple cross-origin requests still execute). No-body POSTs (simple, no preflight): `web_api/handlers/proxy.rs:71 /proxy/start-proxy-server`, `:73 /proxy/stop-proxy-with-restore` (both `post(handler)` with NO Json extractor). `auth-remove-account`/`auth-logout` take `Json<...>` (preflight-protected). Workflow's CORS-preflight finding (#7) confirms uncredentialed OPTIONS is 401'd → JSON/PUT endpoints doubly protected; only no-preflight simple POSTs exposed.

### #6 no periodic backup / bg workers on web (LOW) — `examples/server.rs` vs `lib.rs`
server.rs grep: NONE of periodic_backup/session sync/start_worker. Desktop `lib.rs:551 webdav_auto_sync::start_worker`, `:555 s3_auto_sync::start_worker`, `:679 periodic_backup_if_needed()`, `:686 PERIODIC_MAINTENANCE_INTERVAL_SECS=24h` timer. Web usage syncs only lazily on `GET /api/usage` (usage.rs:174).

### #7 CORS preflight 401'd → CORS_ALLOW_ORIGINS dead (LOW) — Workflow-verified
`routes.rs:35` require_auth on OUTER router wraps `:66` CorsLayer (inner). Uncredentialed `OPTIONS /api/*` → 401 before CorsLayer answers (Workflow reproduced with axum 0.7.9 harness). Fails closed (no exposure), dead knob.

### #8 ip_guard exotic gaps (INFO) — `proxy/ip_guard.rs` (Workflow empirically compiled a replica)
Missed: 6to4 `2002:7f00:1::`→false ; NAT64 `64:ff9b::7f00:1`→false ; IPv4-compat `::127.0.0.1`→false (to_ipv4_mapped None for deprecated form) ; multicast `224.0.0.1`→false ; `0.1.2.3`→false (is_unspecified only exact 0.0.0.0). Covered: `::ffff:127.0.0.1`, `::ffff:169.254.169.254`, ::1, ::, fe80::/10, fc00::/7, 100.64/10, RFC1918.

### #9 dead code (INFO) — Workflow compiler-confirmed
`bootstrap.rs:15 enum RuntimeMode` + `is_web/is_desktop` + `:42 migration_marker_path()` = `warning: never used`; header `:3-10` stale "partial scaffolding/deferred". `adapter.ts:257 throw WebAuthError(401)` + `errors.ts isWebAuthError` = zero consumers (browser handles Basic 401). `tests/components/WebdavSyncSection.test.tsx` prettier nit (out of CI src/** scope).

## Triple-confirmed SOUND (no action)
C1 traversal (axum-harness-probed 13 vectors), C2 auth predicate + non-loopback refusal + ct_eq, R1 session guard (canonicalize+component starts_with), SQLite validation (canonical-equality, READ_ONLY, no SQLITE_OPEN_URI), R4 0o600, F5/F6 bootstrap (byte-faithful extraction, tauri-free, global-outbound-proxy init present), F7/F8/F9-core/M1, R2 log-line truncation, get_guarded-where-applied (per-hop incl v4-mapped, both clients lock-step), web_proxy `#[path]` single SSOT, S3 normalize (no bypass: 0:0/homoglyph/IPv6/decimal/hex/userinfo all handled), F11 bound, frontend CSRF removal + L2 deletions (routes missing:0) + F10 + i18n parity + systemd/install. Correctly-dismissed false positives: `global_proxy.rs:109 test_proxy_url` raw client (by-design proxy testing) ; subscription unguarded get() (hardcoded hosts).
