# Claude independent findings — empirically verified (do not lose)

## CRITICAL-1: Unauthenticated arbitrary file read via SPA-fallback path traversal

**Status: VERIFIED LIVE on the running service (2026-06-14).**

### Proof
```
$ ss -tlnp | grep 3010
LISTEN 0 4096 0.0.0.0:3010 ... users:(("cc-switch-web",pid=1040,fd=11))   # bound to 0.0.0.0, reachable over Tailscale

$ curl -s --path-as-is "http://127.0.0.1:3010/../../../../../../etc/hostname"
orion-NH5x-7xRCx-RDx          # <-- real /etc/hostname content returned
```

### Root cause
- `src-tauri/src/web_api/routes.rs:91` `serve_spa_fallback` → `try_serve_dist_web_asset(uri.path())`
- `routes.rs:100` `let rel = path.trim_start_matches('/');`  (raw URI path, NOT decoded/normalized; `http::Uri` does not collapse `..`)
- `routes.rs:109-112` `let candidate = dist_root.join(rel); if let Some(resp) = read_dist_web_file(&candidate).await { return resp; }`
  - reads the joined path **before** the `is_static_asset_path` gate (line 114) and with **no canonicalization / `..` rejection**.
- `Path::join` with a `rel` containing `../` escapes `dist_root`. Hand-rolled static serving — does NOT use `tower_http::services::ServeDir` (which has built-in traversal protection).

### Impact (Tailscale web deployment)
- Service binds `0.0.0.0:3010` (systemd unit) + NO auth middleware (see CRITICAL-2). Any tailnet device → unauthenticated read of any file readable by the service user:
  - `~/.cc-switch/cc-switch.db` (ALL provider API keys / tokens, schema v10)
  - `~/.claude/settings.json`, `~/.codex/auth.json` (OAuth), `~/.gemini/*`
  - `~/.ssh/id_*`, `/etc/passwd`, arbitrary source.

### Fix (minimal)
In `try_serve_dist_web_asset`, before/after the join: reject any `rel` whose split path contains a `..` component, OR canonicalize `candidate` and verify `candidate.starts_with(dist_root.canonicalize())`. Best: replace the hand-rolled asset path with `tower_http::services::ServeDir` (traversal-safe) for the asset branch, keeping the SPA index fallback for extensionless routes.

---

## CRITICAL-2: Web HTTP API has zero authentication, wired exposure on 0.0.0.0

- `src-tauri/src/web_api/routes.rs:23-72` — `build_router`/`api_router` layer ONLY `TraceLayer`, `security_headers`, `cors`, `DefaultBodyLimit`. No auth/CSRF/rate-limit layer.
- `src-tauri/src/web_api/middleware/{auth.rs (769B), csrf.rs (871B), rate_limit.rs (506B)}` exist but are never `.layer()`-ed → dead stubs.
- `examples/server.rs:18,231-238` env flag is named `ALLOW_HTTP_BASIC_OVER_HTTP=1` to permit non-loopback bind — the name implies HTTP Basic protects it, but **no basic-auth (or any auth) exists**. Misleading + dangerous: operator sets it for tailscale and believes a credential gate exists.
- Every `/api/*` mutating + secret-returning endpoint (provider CRUD, get-global-proxy-url, config read/write, takeover) is reachable with no credential.

### Fix
Wire a real auth layer (bearer token / shared secret from env or settings) applied to `/api` (allowlist `/api/health`). Rename/repurpose the env flag to reflect actual protection. Until then, document loopback-only + `tailscale serve` reverse proxy as the supported posture.

---

## Hot-path perf note (Medium/Low at personal scale)
- `proxy/provider_router.rs::select_providers` issues several DB reads per forwarded request: `get_proxy_config_for_app` (47-54), `get_all_providers` (58), `get_failover_queue` (61-66), then `record_result` re-reads `get_proxy_config_for_app` (182). Fine at personal scale; would matter under load.

## http_client note (Low)
- `proxy/http_client.rs:218` fixed `.timeout(600s)` total-request cap can truncate very long streaming generations. Consider exempting streaming or raising for SSE.

## Dead code note (Low)
- `web_api/handlers/model_fetch.rs` is an empty Layer-2 stub router (merged at routes.rs:49 but returns `Router::new()`); real model fetch lives in `config.rs` (`/api/config/fetch-models-for-config`). Dead merge.

---
# CROSS-VERIFIED MATRIX (Claude verified every codex finding firsthand) — pre-Round-1-reply

| ID | codex sev | Claude verdict | Claude sev | evidence verified |
|----|-----------|----------------|-----------|-------------------|
| F1 traversal | Critical | AGREE (PROVEN LIVE) | Critical | curl read /etc/hostname; routes.rs:100,109-112 |
| F2 no-auth | Critical | AGREE | Critical | routes.rs:28-72 no auth layer; systemd HOST=0.0.0.0 |
| F3 webdav/s3 SSRF | High | AGREE | High | handlers/{webdav,s3} + services/{webdav,s3} = 0 validate_outbound_url; config.rs:459/system.rs:202 DO guard |
| F4 zenmux SSRF | High | AGREE validity, sev↓ | Medium | coding_plan.rs query_zenmux(base_url) unguarded; but needs attacker key + blind |
| F5 migration gap | High | AGREE validity, sev split | Med(this user)/High(general) | server.rs:251 init has no migrate_from_json; lib.rs:363-434 does |
| F6 bootstrap imports | Medium | AGREE, BROADER | Med-High | lib.rs:446-700 seeds providers/official/MCP/prompts/skills/OMO; server.rs does none |
| F7 breaker bypass len==1 | Medium | AGREE | Medium | forwarder.rs:377 bypass=len==1 |
| F8 start race | Medium | AGREE validity, sev↓ | Low | proxy.rs read→bind→write no mutex; rare trigger |
| F9 failover-enable drift | Medium | AGREE (intentional-ish) | Low-Med | failover.rs set_auto_failover only flips flag+rejects empty; no auto-add/switch/emit |
| F10 auto-sync UI no-op | Medium | AGREE | Medium | WebdavSyncSection.tsx:1092,1372 toggle ungated by webMode; _web notify_db_changed no-op |
| F11 speedtest | Medium | PARTIAL | Medium | fan-out real (no urls cap, join_all all); SSRF guarded (system.rs:202); BLOCKING-DNS sub-claim UNVERIFIED (no to_socket_addrs in speedtest.rs) |

## Claude additions codex missed
- M1 (Med-Low perf): provider_router select_providers 3-4 DB reads/forwarded-req (47/58/61) + record_result re-read (182).
- L1 (Low): http_client.rs:218 fixed 600s total timeout truncates long streams.
- L2 (Low dead-code): model_fetch.rs empty stub still merged routes.rs:49.

## False positives caught (not findings)
- config.rs fetch-models SSRF: RETRACTED — guarded at config.rs:459 (Claude's own initial error).
- deplink.html "leaked key": upstream-only (8f7423f0 on upstream/main, not ancestor of fork main); fork = placeholder only.

## Verified-correct (reject if flagged): Random failover (provider_router:138-156 tested) · bounded shutdown (server.rs:339-362 test-pinned) · 0o600 cred files (config.rs:238) · CORS same-origin default · CSP/headers · forwarder retry bounded (max_attempts) · circuit breaker double-check-lock (provider_router:268-304).
