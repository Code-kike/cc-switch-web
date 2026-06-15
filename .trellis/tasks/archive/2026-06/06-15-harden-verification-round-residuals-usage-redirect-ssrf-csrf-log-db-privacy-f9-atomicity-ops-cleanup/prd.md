# PRD — Harden verification-round residuals (scope C)

## Origin
Third independent re-audit of branch `fix/web-audit-phase1-2` (merge-base `eb8ce994`): a 6-dimension Workflow review + a codex gpt-5.5 blind review + main-agent firsthand code verification, cross-validated to convergence. This task fixes the **converged** residual/new findings (scope C = security + defense-in-depth + ops/cleanup). Full converged evidence in `research/converged-findings.md`.

## Threat model (severity calibration — do NOT regress)
Single operator; Linux host; web UI over Tailscale `http://100.75.197.120:3010/` from a Win10 browser; desktop Tauri app NOT used. Production web binary = `src-tauri/examples/server.rs` (`#[path]`-includes ~30 `src` modules). Features `desktop`(default) vs `web-server` mutually exclusive; **any web-reachable module MUST stay tauri-free** or the desktop clippy/test gate breaks. `/api/*` is HTTP-Basic-gated (mandatory on non-loopback). No finding here is Critical; all are operator-password-gated. SSRF guards are **web-runtime-only** by design — the desktop runtime must keep dialing local endpoints unrestricted (do not enforce SSRF blocks on the desktop path).

## Non-negotiable constraints
- Do NOT push/merge/deploy. Local commits on `fix/web-audit-phase1-2` only.
- The live systemd service holds proxy port 15721 — do NOT stop it; tests use ephemeral ports.
- Keep all web-reachable modules tauri-free. SSRF enforcement stays web-only (thread a flag / use a web-only call site; never block the desktop path).
- All existing gates must stay green (see "Verification").

---

## FIX 1 — usage custom-template SSRF (MEDIUM-HIGH, top priority)
**Problem (firsthand-verified).** In custom-template mode the SSRF guard the audit added (P4-A1) is fully bypassed:
- `usage_script.rs:614` `should_validate_base_url = !base_url.is_empty() && !is_custom_template` → custom mode skips base_url validation.
- `usage_script.rs:644` `if !is_custom_template && scheme != "https" && !loopback` → custom mode skips the HTTPS requirement.
- `usage_script.rs:654` `if !base_url.is_empty() && !is_custom_template` → custom mode skips the same-origin check, so `request.url` may target ANY host.
- `usage_script.rs:366,379` `send_http_request` dials the script-controlled `config.url` via the **unguarded** `http_client::get()` and reads the body back.
- Web entry points: `web_api/handlers/usage.rs:317 test_usage_script` validates only `request.base_url` (usage.rs:330), NOT the script's `request.url`; `web_api/handlers/providers.rs:517 query_provider_usage` → `ProviderService::query_usage_with_templates` has **no** outbound validation at all.
- `is_loopback_host` (usage_script.rs:712) only checks `.is_loopback()` (misses 169.254/10.x/CGNAT) — irrelevant once the custom path is fixed but note it.

Result: an authenticated request with `templateType=custom` + a script whose `request.url=http://169.254.169.254/latest/meta-data/` (or `http://127.0.0.1:<port>/`, Tailscale `100.64.x`, RFC1918) is dialed and the response exfiltrated.

**Fix.** Validate the **actually-dialed** `request.url` against the SSRF guard in the **web runtime** before dialing, and dial via the redirect-hardened client:
1. Thread an `enforce_outbound_guard: bool` (or a `Runtime`/`OutboundGuard` enum) from the two web handlers (`test_usage_script`, and `query_usage_with_templates` used by `query_provider_usage`) down into `usage_script`'s `send_http_request`. Desktop callers pass `false` (behavior unchanged).
2. When `true`, before `client.request(...)`: resolve `config.url`'s host and reject if any resolved IP is blocked — reuse the **tauri-free** `crate::proxy::ip_guard::{is_blocked_ip,is_blocked_ipv4}` + async `tokio::net::lookup_host` (mirror `web_api/handlers/common.rs::validate_outbound_url`; reject non-http(s) schemes too). Honor `CC_SWITCH_WEB_SSRF_ALLOW` for parity.
3. When `true`, dial via `http_client::get_guarded()` (not `get()`) so redirect hops are also re-checked.
- `usage_script.rs` is shared + tauri-free — keep it so (ip_guard is tauri-free; do NOT import `web_api`). If a shared async guard helper is cleaner than re-implementing, place it tauri-free (e.g. a new fn in `proxy/ip_guard.rs` or a small shared module) and have `web_api/handlers/common.rs::validate_outbound_url` delegate to it to avoid divergence.

**Acceptance:** new test — web-mode custom-template script with `request.url` = `http://169.254.169.254/`, `http://127.0.0.1:9999/`, `http://10.0.0.1/` each rejected before dial; a public `https://api.example.com` allowed; desktop path (`enforce=false`) unchanged; existing usage_script tests pass.

## FIX 2 — redirect-hardening incomplete (MEDIUM)
**Problem (3-way verified).** `get_guarded()` (per-hop `is_blocked_ip` redirect policy) is used only by `services/{balance,coding_plan,model_fetch}.rs`. These user/redirect-influenced outbound paths still dial via the unguarded `http_client::get()` (reqwest default policy follows ≤10 redirects with **no** per-hop IP recheck), so a validated public URL can 30x-redirect to an internal IP that is then followed:
- `services/stream_check.rs:256,710`
- `services/webdav.rs:134,173,231,258,302`
- `services/s3.rs:335,377,422,488`
- `services/speedtest.rs:122` (`build_client` returns `http_client::get()`; reached by `system.rs::test_api_endpoints`)

**Fix.** Switch these service dials from `http_client::get()` to `http_client::get_guarded()`. These are all web-reachable handlers whose initial URL is already `validate_outbound_url`-checked at the handler; `get_guarded` adds the redirect-hop re-check. The shared proxy hot-path (`forwarder`) keeps `get()` (unchanged). Confirm desktop paths that reach these services still behave (get_guarded has identical proxy/timeout config, only adds a redirect policy — safe for both runtimes).
**Acceptance:** grep shows the listed sites now call `get_guarded()`; existing webdav/s3/stream_check/speedtest tests pass; document remaining domain-redirect/DNS-rebind residual is unchanged (acknowledged at `http_client.rs:241-244`).

## FIX 3 — upstream error body persisted untruncated to request-log DB (LOW, R2 gap)
**Problem (firsthand-verified).** `proxy/handlers.rs:1041 log_forward_error` → `get_error_message(error)` (`:1051`) — for `ProxyError::UpstreamError` this is `format!("上游错误 ({status}): {body}")` with the **full** upstream body — → `logger.log_error_with_context(..., error_message, ...)` (`:1054`) → `usage/logger.rs:194 error_message: Some(error_message)` → `log_request` INSERT into the request-log DB, untruncated. R2 truncated only `handlers.rs:332` (journal line) and `:723`; this DB-persistence path was missed. Upstream error bodies can carry prompts/request fragments/tokens or large HTML.
**Fix.** Truncate the `error_message` before persistence: wrap with the existing `compact_error_message(&msg, N)` (e.g. N=400) at `handlers.rs:1051` before passing to `log_error_with_context`. (Optionally also bound `body` at `UpstreamError` construction in `forwarder.rs:1791` / `extract_error_message`, but the DB-write truncation is the required fix.) Keep client-facing error responses (`codex_proxy_error_json` etc.) as-is — only the persisted log field must be bounded.
**Acceptance:** new/updated test — a simulated UpstreamError with a >1KB body persists a `error_message` ≤ the bound; client response path unaffected.

## FIX 4 — F9 enable non-atomic on switch failure (LOW)
**Problem.** `services/proxy.rs:2394` persists `config.auto_failover_enabled = enabled` (DB write at 2395-2398) BEFORE `self.switch_proxy_target(...).await?` (2402), which can return Err (e.g. "Cannot switch to official provider during proxy takeover"). On switch failure the API returns Err but the flag is already persisted true → state divergence. (Routing stays consistent — provider_router selects via the failover queue regardless — and it matches the old desktop ordering, hence LOW; but the enable is not transactional.)
**Fix.** Reorder so the flag is committed only after a successful switch, OR roll back the flag on switch failure before returning Err. Preserve the empty-queue auto-add of current provider, the `provider-switched` emit, and the unconditional `refresh_tray`. Keep desktop+web parity (shared SSOT).
**Acceptance:** new test — enabling failover when `switch_proxy_target` errors leaves `auto_failover_enabled=false` (or rolled back) and returns Err; the happy path still persists true + emits + refreshes tray.

## FIX 5 — Basic-auth does not prevent CSRF on no-body POSTs (LOW)
**Problem (verified).** `require_auth` checks only `Authorization`; cached Basic creds are auto-attached cross-site like cookies. CORS (`allow_credentials(true)`, default no `allow_origin`) hides cross-origin *responses* but does NOT block simple cross-origin requests from executing. The no-body POSTs `/api/proxy/start-proxy-server` and `/api/proxy/stop-proxy-with-restore` (+ other no-body system POSTs) are simple requests (no preflight) → a malicious page (knowing the Tailscale IP, with cached creds) can toggle the proxy. JSON-body endpoints are protected (preflight → uncredentialed OPTIONS is 401'd by auth — see FIX 7) and `auth-remove-account` takes a JSON body (protected).
**Fix.** Add a lightweight request-intent check in `require_auth` for **state-changing** methods on `/api/*` (POST/PUT/PATCH/DELETE): require that EITHER `Sec-Fetch-Site` ∈ {`same-origin`,`none`} OR (if `Origin` present) `Origin`'s host matches the request `Host`. Reject mismatches with 403. No token plumbing (do NOT re-add the CSRF stub). Must not break the same-origin SPA (its fetches are same-origin → `Sec-Fetch-Site: same-origin`) nor the browser Basic-auth replay. GET/HEAD and the public paths are exempt.
**Acceptance:** new test — a cross-site `Origin` (host ≠ Host) POST to `/api/proxy/start-proxy-server` → 403; a same-origin POST (matching Origin/Host, or `Sec-Fetch-Site: same-origin`, or no Origin+`Sec-Fetch-Site: none`) → passes to handler; GET unaffected.

## FIX 6 — web server lacks periodic DB backup + background sync workers (LOW, parity/ops)
**Problem (firsthand-verified).** `examples/server.rs` never starts the desktop startup workers: `db.periodic_backup_if_needed()` + the 24h maintenance timer (desktop `lib.rs:679,686`), the session-usage sync (claude/codex/gemini/opencode), and `webdav_auto_sync::start_worker`/`s3_auto_sync::start_worker` (desktop `lib.rs:551,555`). On the long-running headless server the SQLite DB is never auto-backed-up and usage only syncs lazily on `GET /api/usage`.
**Fix.** In `server.rs main()`, after `run_post_db_bootstrap`, spawn the equivalents (tauri-free): at minimum `periodic_backup_if_needed()` initial + a daily `tokio::time::interval` timer; and the session-usage background sync + webdav/s3 auto-sync workers (the `*_auto_sync_web` / `session_usage*` modules are already compiled into the web build per `web_services.rs`). Match desktop cadence. Ensure no tauri handle is required (use the web sink / db clone). If a worker genuinely needs tauri, gate it out and document.
**Acceptance:** `server.rs` spawns periodic backup (verified by grep + a smoke assertion if feasible); web + desktop builds compile; no tauri leak; document any intentionally-skipped worker in the server.rs header.

## FIX 7 — CORS preflight 401'd by auth → CORS_ALLOW_ORIGINS dead (LOW)
**Problem.** `require_auth` (outer layer) 401s the uncredentialed `OPTIONS` preflight before the inner `CorsLayer` can answer, so `CORS_ALLOW_ORIGINS` cannot enable a working cross-origin SPA (fails closed — not a security hole, a dead knob).
**Fix.** Exempt the CORS preflight from auth: in `require_auth`, short-circuit `if req.method() == OPTIONS && req.headers().contains_key(ORIGIN) { return next.run(req).await; }` (let the CorsLayer answer it). Real cross-origin GET/POST still carry credentials and remain auth-checked. (This composes with FIX 5: the preflight pass-through only enables CORS negotiation; the actual state-changing request is still subject to the FIX-5 origin check + auth.) Alternatively, if cross-origin is never wanted, drop the `CORS_ALLOW_ORIGINS` knob + doc — but the preflight-exemption is preferred (keeps the documented escape hatch functional).
**Acceptance:** new test — `OPTIONS /api/providers` with `Origin` + `Access-Control-Request-Method` and no auth → not 401 (passes to CORS); a real `GET /api/providers` with no creds → still 401.

## FIX 8 — ip_guard exotic-range gaps (INFO, defense-in-depth)
**Problem (verified).** `proxy/ip_guard.rs` misses: IPv4 `0.0.0.0/8` (non-zero, e.g. `0.1.2.3`), IPv4 multicast/reserved (`224.0.0.0/4`, `240.0.0.0/4`), IPv4-compatible IPv6 `::a.b.c.d` (`to_ipv4_mapped` returns None for the deprecated form), 6to4 `2002::/16`, NAT64 `64:ff9b::/96`, Teredo `2001::/32`, IPv6 multicast `ff00::/8`. Core ranges (loopback/RFC1918/link-local/ULA/CGNAT/unspecified/`::ffff:` mapped/metadata `169.254.169.254`) ARE covered.
**Fix.** Extend `is_blocked_ipv4` (`octets[0]==0` for 0.0.0.0/8; `is_multicast()`; `240.0.0.0/4`) and `is_blocked_ipv6` (unwrap via `to_ipv4()` to also catch `::a.b.c.d`; block `2002::/16`, `64:ff9b::/96`, `2001::/32`, `ff00::/8`). Keep tauri-free + sync. Add unit tests for each new range; keep the existing public-IP allow tests green.
**Acceptance:** new tests cover each added range blocked + public addresses still allowed; both runtimes compile.

## FIX 9 — dead code / cleanup (INFO)
- `bootstrap.rs:14-44` — delete the never-used `RuntimeMode` enum (+ `is_web`/`is_desktop`) and `migration_marker_path()` (compiler-confirmed dead); refresh the `bootstrap.rs:1-10` header to drop the stale "partial scaffolding / deferred follow-up" language (F5/F6 are now fully integrated).
- `src/lib/api/adapter.ts:257` + `errors.ts` — either wire a top-level handler that surfaces `isWebAuthError` (re-enter-credentials toast) or drop the unused `isWebAuthError` export + the `WebAuthError(401)` throw (browser handles Basic 401 natively). Prefer dropping the dead path unless a toast is trivially addable.
- `tests/components/WebdavSyncSection.test.tsx` — `npx prettier --write` (cosmetic; out of CI scope but tidy).

**Acceptance:** `cargo check` (both features) has no new dead-code warnings from bootstrap.rs; no dangling `RuntimeMode`/`migration_marker_path` refs; frontend typechecks; prettier clean on the touched test.

---

## Out of scope (accepted residuals — do NOT change)
- R3 MCP-upsert write→exec (intended operator MCP management, auth-gated).
- deeplink import-without-FE-confirm (operator-gated; `config_url` stays unimplemented = no SSRF).
- OAuth tokens at-rest 0o600 (encryption-at-rest out of scope).
- opencode `msg_id` join + read/delete containment asymmetry (INFO, requires pre-existing in-root write; not web-reachable) — leave, or only add the `is_safe_relative_asset`-style msg_id sanitization if trivial and zero-risk (optional, lowest priority).
- Domain-redirect / DNS-rebinding residual on get_guarded (acknowledged, auth-gated).
- CORS `allow_credentials`+wildcard foot-gun (operator-only; optionally reject literal `*` when credentials=true, lowest priority).

## Verification (all must pass — the enforced gate suite)
- `source "$HOME/.cargo/env"` then: desktop `cargo clippy --all-targets --features desktop -- -D warnings` + `cargo test --features desktop`; web `cargo check --no-default-features --features web-server --example server` + the web test set (`cargo test ... web_api:: dual_runtime_parity:: web_proxy_lifecycle::` under web-server feature) ; `cargo fmt --check`.
- FE: `pnpm format:check`, `pnpm check:web-routes` (missing:0), `pnpm check:locales` (parity), typecheck, relevant unit tests.
- Do NOT run the env/bootstrap-flaky `test:integration` web-server suites as a gate (known-flaky per project memory).
- Update `.trellis/spec/frontend/quality-guidelines.md` with the new contracts (web-only request.url SSRF validation; get_guarded on all user-URL service dials; request-log error_message truncation; CSRF origin check + CORS preflight exemption; F9 atomic-enable; web background workers; ip_guard exotic ranges).
