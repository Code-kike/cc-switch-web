### [CDX-1] Saved Token Plan ZenMux usage bypasses the new web outbound guard
- kind: fix-incomplete
- severity: medium
- confidence: high
- file: src-tauri/src/services/provider/usage.rs:623-633; src-tauri/src/services/coding_plan.rs:418-450
- evidence: `query_provider_usage` passes `true`, but the `TEMPLATE_TYPE_TOKEN_PLAN` branch ignores `enforce_outbound_guard` and calls `get_coding_plan_quota(&credentials.base_url, ...)`. ZenMux then does `client.get(base_url)` and on non-2xx returns `API error (HTTP {status}): {body}`.
- impact: authenticated web API usage can still make the server GET a saved `usage_script.base_url` such as an internal URL containing `zenmux`; `get_guarded()` only hardens redirects, not the first dial.
- recommendation: in web mode, run `guard_outbound_url(&credentials.base_url)` before built-in Token Plan ZenMux dials, or thread an enforced guard into `get_coding_plan_quota`.

### [CDX-2] FIX2 changes desktop redirect behavior for shared services
- kind: fix-regression
- severity: low
- confidence: high
- file: src-tauri/src/services/webdav.rs:132-137; src-tauri/src/proxy/http_client.rs:293-309
- evidence: WebDAV/S3/stream_check/speedtest now use `get_guarded()`, whose redirect policy `attempt.stop()`s on internal IP literals. These services are also reached by desktop commands such as `webdav_test_connection` and `s3_test_connection`.
- impact: desktop local/LAN setups still dial initial local URLs, but a legitimate local redirect to `192.168.x.x`, `127.x`, etc. will no longer be followed. Desktop is out of the stated deployment, so impact is low here.
- recommendation: if desktop parity matters, choose guarded clients at web handler boundaries instead of inside shared services, or add a runtime flag for redirect guarding.

### [CDX-3] Origin fallback mis-parses bracketed IPv6 Host
- kind: new-issue
- severity: low
- confidence: high
- file: src-tauri/src/web_api/middleware/auth.rs:156-167
- evidence: `Origin` is parsed with `Url::host_str()`, but request `Host` is parsed with `h.split(':').next()`. For `Host: [::1]:3010`, that yields `"["`, not `"::1"`.
- impact: same-origin IPv6 requests without `Sec-Fetch-Site` can be rejected with 403. Modern same-origin SPA fetches use `Sec-Fetch-Site: same-origin`, and the target deployment is IPv4 Tailscale, so this is not merge-blocking for that threat model.
- recommendation: parse `Host` with `http::uri::Authority`, `url::Host`, or bracket-aware host/port parsing.

### [CDX-4] Failed F9 enable can still mutate an empty failover queue
- kind: fix-incomplete
- severity: low
- confidence: high
- file: src-tauri/src/services/proxy.rs:2356-2403
- evidence: when the queue is empty, `add_to_failover_queue(app_type, &current_id)` runs before `switch_proxy_target(...)`. If the switch fails, the flag is not persisted, but the queue insert remains.
- impact: a failed enable leaves `auto_failover_enabled=false` as intended, but it can silently seed P1 for a later enable attempt.
- recommendation: roll back the auto-added queue item on switch failure, or defer the queue write until after validating the switch target.

### [CDX-5] RuntimeMode cleanup leaves a stale reference
- kind: fix-incomplete
- severity: info
- confidence: high
- file: src-tauri/src/lib.rs:1-4
- evidence: the deleted type is still named in a top comment: `Web-mode bootstrap (RuntimeMode + UiEventSink + sessions schema)`.
- impact: no runtime effect; it is a dangling cleanup reference after FIX9.
- recommendation: remove `RuntimeMode` from the comment.

### [CDX-6] Custom-template JS SSRF threading is sound
- kind: confirmation-ok
- severity: info
- confidence: high
- file: src-tauri/src/usage_script.rs:388-399
- evidence: web handlers pass `true`, desktop commands pass `false`, and `send_http_request` runs `guard_outbound_url(&config.url).await` before choosing `get_guarded()` for web mode.
- impact: the claimed custom-template `request.url` SSRF path is closed before first dial, with redirect IP-literal recheck.
- recommendation: none — sound.

### [CDX-7] Request-log DB error truncation is sound
- kind: confirmation-ok
- severity: info
- confidence: high
- file: src-tauri/src/proxy/handlers.rs:1051-1057
- evidence: `let error_message = compact_error_message(&get_error_message(error), 400);` is the value passed to `log_error_with_context`; `compact_error_message` truncates by `.chars()`.
- impact: persisted upstream error bodies are bounded without changing client-facing error generation.
- recommendation: none — sound.

### [CDX-8] FIX5/FIX7 core interaction is sound
- kind: confirmation-ok
- severity: info
- confidence: high
- file: src-tauri/src/web_api/middleware/auth.rs:188-200
- evidence: only `OPTIONS + Origin` bypasses auth for CORS; mutating methods are `POST | PUT | PATCH | DELETE`, so real state-changing requests still run the intent check and then Basic auth.
- impact: I did not find an OPTIONS smuggling path. `Sec-Fetch-Site: same-site` is intentionally rejected; no-Origin API clients are allowed and still require Basic auth.
- recommendation: none, aside from the IPv6 parser fix above.

### [CDX-9] Web background workers are scoped correctly
- kind: confirmation-ok
- severity: info
- confidence: high
- file: src-tauri/examples/server.rs:301-313; src-tauri/examples/server.rs:456-503
- evidence: `spawn_background_workers` runs after `run_post_db_bootstrap`; it spawns periodic backup and the four session-usage syncs. WebDAV/S3 auto-sync is explicitly skipped because it needs `AppHandle`.
- impact: startup is non-blocking and tauri-free; worker failures are logged, not startup-aborting.
- recommendation: none — sound.

### [CDX-10] Exotic ip_guard ranges are sound
- kind: confirmation-ok
- severity: info
- confidence: high
- file: src-tauri/src/proxy/ip_guard.rs:31-102
- evidence: IPv4 now blocks `0/8`, multicast, and `240/4`; IPv6 now checks `to_ipv4()`, 6to4, Teredo, NAT64 well-known prefix, and multicast. Tests keep public `2606:4700:4700::1111` allowed.
- impact: no false-positive on normal public IPv6 found.
- recommendation: none — sound.

## SUMMARY

The 9-fix round mostly holds, but I found one substantive residual: saved Token Plan/ZenMux usage can still dial an internal first-hop URL in web mode. I also found low-impact regressions/cleanup gaps around shared-service desktop redirects, IPv6 Origin fallback parsing, F9 queue mutation on failed enable, and a stale `RuntimeMode` comment.

I did not run the full test suite in this read-only review. `git diff --check 072d3d68 HEAD` is clean. Overall verdict: close CDX-1 before merge; the remaining items are low/info under the stated Tailscale single-operator web deployment.