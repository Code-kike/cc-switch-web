# Independent verification review — cc-switch-web branch `fix/web-audit-phase1-2`

You are doing an INDEPENDENT, firsthand code review. The cwd is the repo root; the branch `fix/web-audit-phase1-2` is checked out. This branch is the OUTPUT of a completed security+correctness audit you previously co-reviewed. It already FIXED these findings:

- **C1** SPA static path-traversal (`is_safe_relative_asset` guard in `routes.rs`)
- **C2** HTTP Basic auth wiring (`middleware/auth.rs::require_auth` over `/api/*`) + non-loopback bind refusal in `examples/server.rs`
- **F3** webdav/s3 SSRF guards; **F4** subscription SSRF guards; **F11** speed-test fan-out bound
- **F5** legacy-json migration ordering; **F6** post-DB bootstrap extraction (`bootstrap.rs`)
- **F7** circuit-breaker bypass policy (`!failover_enabled`); **F8** proxy start/stop lock; **F9** failover-enable parity; **M1** provider-config reuse
- **F10** frontend web-mode gating (`WebdavSyncSection.tsx`); **L2** dead stub-router removal
- **P4-A/B/C** SSRF redirect re-check (`http_client::get_guarded`) + ip_guard SSOT (`proxy/ip_guard.rs`) + all outbound call sites + schemeless-S3 + CSRF plumbing removal (`adapter.ts`)
- **R1** session-path root guard (`session_manager/mod.rs::load_messages`); **R2** log privacy (`RUST_LOG=info` + body truncation)

Merge-base with `main` = `eb8ce994`. Diff any file with `git diff main...HEAD -- <path>`.

## THREAT MODEL (calibrate severity to THIS)
Single operator; Linux host; web UI reached from a Win10 browser over Tailscale at `http://100.75.197.120:3010/`; the desktop Tauri app is NOT used. Production web binary = `src-tauri/examples/server.rs`, which `#[path]`-includes ~30 `src` modules; cargo features `desktop`(default) vs `web-server` are mutually exclusive; any web-reachable module MUST stay tauri-free. `/api/*` is gated by HTTP Basic auth (mandatory when bind is non-loopback). So "unauthenticated" attacks generally require the operator password — rate those LOWER (operator-gated), but STILL report multi-user / defense-in-depth gaps as low/info.

## YOUR JOB (two-fold)
1. **VERIFY** each fix is correct, complete, and regression-free. Actively try to BYPASS each guard; check edge cases, layer/init ordering, races, and web-vs-desktop parity.
2. **FIND NEW** issues the audit missed — including pre-existing bugs in adjacent code you read.

## DIMENSIONS TO COVER (the same scope the parallel reviewers used)
1. **Auth & access control (C2/C1)** — `web_api/middleware/auth.rs`, `web_api/routes.rs`, `examples/server.rs`, `middleware/cors.rs`. Which paths are public vs gated? Public-path predicate exploitable (prefix match, `/api/health/../`, trailing slash, `//api/`, percent-encoding)? Basic header parsing edge cases; constant-time compare; non-loopback bind refusal bypass; `is_safe_relative_asset` traversal attempts (`..`, `%2e%2e`, absolute, Windows prefix, symlink); layer ordering (auth vs body-limit vs trace).
2. **SSRF & outbound (P4/F3/F4/F11)** — `proxy/ip_guard.rs`, `proxy/http_client.rs`, `web_api/handlers/common.rs`, `handlers/{subscription,webdav,s3,usage,providers}.rs`, `services/stream_check.rs`, `examples/web_proxy.rs`. ip_guard range completeness (IPv4/IPv6/`::ffff:` mapped/`0.0.0.0`/CGNAT/ULA/link-local); `get_guarded` redirect re-check on every hop incl. final & first; is the second (forwarder) client intentionally unguarded & safe; TOCTOU/DNS-rebinding residual; EVERY user-influenced outbound call site guarded (grep `reqwest`/`.get(`/`.post(`); `normalize_s3_endpoint_for_guard` (schemeless host:port, IPv6 literal); allowlist bypass; web_proxy.rs ip_guard = single SSOT or drifting copy?
3. **Proxy/failover (F7/F8/F9/M1)** — `proxy/{forwarder,handlers,handler_context,provider_router}.rs`, `services/proxy.rs`, `commands/failover.rs`, `web_api/handlers/failover.rs`. F7 semantic correct & plumbed to all 5 sites & no regression vs `providers.len()==1`; F8 lock actually serializes & no deadlock & TimeoutStopSec interaction; F9 enable+disable parity (emit provider-switched + refresh_tray) web vs desktop after the -96-line refactor; M1 no stale config; R2 truncation correctness.
4. **Dual-runtime/bootstrap (F5/F6)** — `bootstrap.rs`, `examples/server.rs`, `lib.rs`, `examples/web_proxy.rs`, `proxy/mod.rs`. F6 faithful extraction (no dropped/dup behavior); bootstrap error abort-vs-continue; F5 idempotent & ordered; full init ordering & global-outbound-proxy init present on web path; tauri-free invariant for bootstrap.rs (grep for `tauri::`/AppHandle); desktop parity.
5. **Session path safety + residuals (R1/R3/R4/deeplink)** — `session_manager/mod.rs`, `providers/{opencode,hermes}.rs`, plus mcp-upsert + deeplink handlers. R1 canonicalize + within-roots = same allow-set as `delete_session_with_roots`; symlink/`..`/prefix-not-contained escape; `sqlite:<db>:<id>` path validation covers opencode+hermes & rejects arbitrary `.db`; OTHER unguarded session entry points; R3 mcp-upsert write→exec auth-gated (note residual); R4 OAuth 0o600; deeplink web path executes import without FE confirm (auth-gated, low).
6. **Frontend + diff hygiene + docs (P4-C/F10/L2)** — `src/lib/api/adapter.ts`, `WebdavSyncSection.tsx`, `i18n/locales/*.json`, README*, `deploy/systemd/cc-switch-web.service`, `scripts/install-cc-switch-web-service.sh`, `scripts/smoke-web-server.mjs`, `tests/*`, deleted handlers/middleware. Dangling X-CSRF refs anywhere in `src/`; all mutating calls keep `credentials:"include"`; dangling `mod`/`merge()` refs to deleted modules; FE calls to deleted endpoints (cross-check `src/lib/api/web-commands.ts`); F10 gating + desktop test pin; i18n keys referenced & present in all 3 locales; systemd/install/smoke correctness vs Basic auth; tests removed — lost coverage vs CSRF scaffolding?

## RULES
- Verify every claim FIRSTHAND by reading the actual code; cite `file:line` and quote the decisive line(s). Do NOT infer from names.
- If you cannot confirm a finding in source, drop it or mark confidence **low**.
- Prefer a few high-certainty findings over many vague ones.
- Distinguish: `fix-incomplete` / `fix-regression` (problem in the audit's own fix) vs `new-issue` (anything else) vs `confirmation-ok` (a guard you checked and found genuinely sound).

## OUTPUT FORMAT (strict — I will machine-cross-map this against the parallel review)
For EACH finding, emit a block exactly like:

```
### [CDX-N] <title>
- kind: fix-incomplete | fix-regression | new-issue | confirmation-ok
- severity: critical | high | medium | low | info
- confidence: high | medium | low
- file: <path>:<lines>
- evidence: <quoted decisive code + why>
- impact: <concrete consequence under the threat model>
- recommendation: <concrete fix, or "none — sound" for confirmation-ok>
```

End with a short `## SUMMARY` (3-6 lines): overall posture, the single most important finding, and whether you consider the audit's headline fixes (C1/C2/P4-SSRF/R1) genuinely closed.
