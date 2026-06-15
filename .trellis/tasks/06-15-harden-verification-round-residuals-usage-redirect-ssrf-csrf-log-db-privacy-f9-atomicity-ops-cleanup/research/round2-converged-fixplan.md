# Round-2 converged fix plan (re-review of commit ed07d9ba)

Origin: a SECOND three-way re-review (Workflow 6-dim + codex gpt-5.5 blind + main-agent firsthand) of the 9 hardening fixes in `ed07d9ba`, cross-validated to convergence. The 9 fixes are SOUND; this plan fixes the converged residual/new findings the re-review surfaced. Threat model unchanged (single-operator, Tailscale IPv4, desktop NOT used, /api/* Basic-auth-gated, web-reachable modules tauri-free, SSRF web-only).

Convergence: Workflow + codex + firsthand agree on every finding and every fix approach (codex round-2 recommendations CDX-1/2/3/4/5 + a follow-up fix-plan deliberation that confirmed finding C and steered C toward reusing the existing `summarize_proxy_error` helper). Severity adjudication: native-template SSRF = MEDIUM (codex) over LOW (workflow) — it's a guard-bypass of FIX1's intent reaching internal on the first hop.

## FIX A [MEDIUM] — native-template (TOKEN_PLAN/BALANCE) base_url SSRF
**Problem (3-way):** `query_provider_usage`/`test_usage_script` pass `enforce_outbound_guard=true`, but the `TEMPLATE_TYPE_TOKEN_PLAN` and `TEMPLATE_TYPE_BALANCE` arms of `query_usage_with_templates` (and the test_usage_script duplicate) call `crate::services::coding_plan::get_coding_plan_quota(&credentials.base_url, ..)` / `crate::services::balance::get_balance(&credentials.base_url, ..)` WITHOUT consulting the flag. Those services dial the user-controlled `base_url` (e.g. ZenMux `client.get(base_url)`); `get_guarded()` only re-checks REDIRECT hops, so the INITIAL dial to an internal base_url is unguarded. (codex CDX-1; workflow usage-thread NEW.)
**Fix:** in `services/provider/usage.rs`, in the TOKEN_PLAN and BALANCE arms (both in `query_usage_with_templates` ~623-642 AND the duplicate in the `test_usage_script` service fn ~772-784), when `enforce_outbound_guard == true` AND `credentials.base_url` is non-empty, call `crate::proxy::ip_guard::guard_outbound_url(&credentials.base_url).await` and map the error (reuse the same mapping as usage_script's `map_outbound_guard_error`, or a local AppError map) BEFORE invoking get_coding_plan_quota/get_balance. Guard at the arm — do NOT thread the flag into the service signatures (keeps balance.rs/coding_plan.rs desktop-callable unchanged). OFFICIAL_SUBSCRIPTION + GITHUB_COPILOT arms unchanged (hardcoded vendor hosts).
**Acceptance:** new test — web path (enforce=true) with a saved provider base_url = `http://169.254.169.254/` (or 127.0.0.1/10.x) on a token_plan/balance template is rejected before dial; a public base_url passes; desktop path (enforce=false) unchanged.

## FIX B [LOW] — FIX5 IPv6 Host port-strip bug
**Problem:** `auth.rs:163` strips the Host port via `h.split(':').next().unwrap_or(h)`, which corrupts a bracketed IPv6 Host (`[::1]:3010` → `[`), while the Origin side uses `url::Url::host_str()` (→ `::1`), so a legitimate same-origin IPv6 request lacking `Sec-Fetch-Site` is falsely 403'd. Security-safe (over-blocks, never over-allows); inert on the IPv4 deployment. (codex CDX-3; workflow auth NEW.)
**Fix:** parse the Host bracket-aware, symmetric with the Origin side: `url::Url::parse(&format!("http://{h}")).ok().and_then(|u| u.host_str().map(|s| s.to_ascii_lowercase()))`. (http::uri::Authority is an acceptable alternative.)
**Acceptance:** new test — Host `[::1]:3010` + Origin `http://[::1]:3010` (no Sec-Fetch-Site) → Ok; cross-site IPv6 Origin still 403.

## FIX C [LOW] — FIX3 incomplete: provider_health.last_error untruncated
**Problem:** the forwarder failure path `self.router.record_result(provider_id, app_type, .., Some(e.to_string()))` (forwarder.rs ~547,680,829,906) persists the FULL UpstreamError body (Display = `上游错误 (状态码 {status}): {body:?}`) into `provider_health.last_error` (provider_router::record_result → dao/proxy.rs:573 `update_provider_health_with_threshold` → UPSERT `?7` last_error). FIX3 only truncated the request_log path. (workflow proxy fix-incomplete; codex confirmed in the fix-plan deliberation.)
**Fix (codex-steered — keep the DB layer free of proxy utils):** reuse the existing `forwarder.rs:2101 summarize_proxy_error(&ProxyError) -> String` (already bounds the body to 180 chars via `summarize_upstream_body`/`summarize_text_for_log`, extracts the JSON error message, char-safe). At the failure `record_result` call sites in forwarder.rs, pass `Some(summarize_proxy_error(&e))` instead of `Some(e.to_string())` (where `e`/`error` is the `ProxyError` in scope). Do NOT relocate compact_error_message or touch the DAO.
**Acceptance:** new/updated test — a record_result with an UpstreamError carrying a >1KB body persists a `last_error` ≤ ~200 chars; success path (None) unaffected.

## FIX D [LOW] — FIX4 failed-enable leaves current provider in the failover queue
**Problem:** in `set_auto_failover_enabled`, the empty-queue auto-add `add_to_failover_queue(app_type, &current_id)` (proxy.rs:2370) runs BEFORE the switch; a failed `switch_proxy_target` (e.g. official provider during takeover) leaves current stuck in the queue (flag correctly stays false), so subsequent enables deterministically fail. (codex CDX-4; workflow proxy NEW; main firsthand.)
**Fix:** defer the queue write until AFTER a successful switch. When the queue is empty, compute `p1_provider_id = current_id` directly (do NOT add to the queue yet); run `switch_proxy_target(app_type, &p1)`; only on success do `add_to_failover_queue(app_type, &current_id)` (for the was-empty case) and then `update_proxy_config_for_app` (persist flag). `switch_proxy_target → hot_switch_provider(provider_id)` takes an explicit id and does NOT read the queue (confirmed firsthand at proxy.rs:2310-2323), so this ordering is safe.
**Acceptance:** update the existing `enable_auto_failover_does_not_persist_flag_when_switch_fails` test to ALSO assert the failover queue is unmodified (empty) after a failed enable from an empty queue; the happy path still adds current + persists + emits + refreshes tray.

## FIX E [doc] — stale comment + spec gaps
- `src-tauri/src/lib.rs:2` — drop `RuntimeMode + ` from the comment (now `Web-mode bootstrap (UiEventSink + sessions schema) lives in ...`).
- `.trellis/spec/frontend/quality-guidelines.md` — add a short FIX9 note (FE WebAuthError/isWebAuthError removed because Basic-401 is browser-native; Rust bootstrap RuntimeMode/migration_marker_path removed as unused), and rename the in-scenario `F9 atomicity (FIX 4)` → `FIX 4 (failover-enable atomicity)` to kill the F9(feature)/FIX9(round) collision. Also add the round-2 contracts (A native-template guard, B IPv6 host parse, C health-error truncation, D failover-enable queue atomicity, F desktop redirect-policy accepted).

## FIX F [ACCEPT] — FIX2 desktop redirect-policy parity
**Decision: ACCEPT, do NOT code-gate.** get_guarded() is not feature-gated, so desktop callers of stream_check/speedtest/webdav/s3 now abort internal-IP-literal redirect HOPS (initial dials unaffected). Desktop runtime is NOT used in this deployment and the redirect-block is desirable defense-in-depth. (codex CDX-2 + workflow ssrf — both said accept is reasonable.) Document the behavior change in the spec scenario (FIX F note). No code change.

## Constraints / invariants (must hold)
- tauri-free for all web-reachable modules; SSRF enforcement web-only (FIX A guards only when enforce=true; desktop path unchanged).
- No git commit by the implementer; no push/merge/deploy; do not stop the live proxy (port 15721); tests use ephemeral ports.
- `source "$HOME/.cargo/env"`. Sanity: `cargo check` (web-server example + desktop) + `cargo fmt` + FE typecheck/format. Full gate suite is trellis-check's job.

## Out of scope (unchanged)
R3 MCP-upsert, deeplink no-confirm, OAuth at-rest, opencode msg_id/containment, domain-redirect DNS-rebind residual, CORS wildcard foot-gun, native-template arms beyond TOKEN_PLAN/BALANCE (OFFICIAL/COPILOT are hardcoded-host safe).
