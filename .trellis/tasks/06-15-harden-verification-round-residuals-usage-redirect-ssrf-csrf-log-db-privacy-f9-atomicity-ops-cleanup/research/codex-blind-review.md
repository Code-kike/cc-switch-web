### [CDX-1] Usage queries still have unguarded SSRF paths
- kind: fix-incomplete
- severity: medium
- confidence: high
- file: src-tauri/src/web_api/handlers/providers.rs:517-529; src-tauri/src/web_api/handlers/usage.rs:326-344; src-tauri/src/services/provider/usage.rs:614-632,672,780-789; src-tauri/src/usage_script.rs:240-244,363-394,642-654
- evidence: `query_provider_usage` calls `ProviderService::query_usage_with_templates(...)` with no `validate_outbound_url`; `test_usage_script` validates only `request.base_url`; the service then dials `get_coding_plan_quota(&credentials.base_url, ...)`, `get_balance(&credentials.base_url, ...)`, or falls through to `execute_and_format_usage_result(...)`; the JS path sends `client.request(..., &config.url)` via `http_client::get()`. Custom templates explicitly skip HTTPS/same-origin checks: `if !base_url.is_empty() && !is_custom_template`.
- impact: An authenticated web user or imported provider config can make the server dial loopback/Tailscale/internal endpoints through saved usage queries or custom usage scripts.
- recommendation: validate the resolved final URL at the web boundary after provider credential resolution and after JS `request.url` evaluation; use an outbound client/path that blocks the initial target and redirects.

### [CDX-2] Initial SSRF checks do not cover downstream redirects
- kind: fix-incomplete
- severity: medium
- confidence: high
- file: src-tauri/src/web_api/handlers/webdav.rs:122-127; src-tauri/src/services/webdav.rs:131-143,253-267; src-tauri/src/web_api/handlers/s3.rs:136-143; src-tauri/src/services/s3.rs:335-352,422-440; src-tauri/src/services/stream_check.rs:255-256,405-415; src-tauri/src/services/speedtest.rs:73-83,119-124
- evidence: WebDAV/S3 handlers validate the configured base endpoint, but services use `http_client::get()` and then `.get()/.put()/.head()/.post(...).send()`; stream-check and speed-test do the same after handler validation. The guarded redirect policy in `http_client::get_guarded()` is not used on these paths.
- impact: A validated public URL can redirect the server to `127.0.0.1`, `169.254.169.254`, Tailscale CGNAT, or other blocked ranges.
- recommendation: use `get_guarded()` or a manual no-auto-redirect loop that runs `validate_outbound_url` on every `Location` before following.

### [CDX-3] Upstream error bodies are still persisted untruncated
- kind: fix-incomplete
- severity: low
- confidence: high
- file: src-tauri/src/proxy/forwarder.rs:1791-1797; src-tauri/src/proxy/error.rs:42-43; src-tauri/src/proxy/error_mapper.rs:69-72; src-tauri/src/proxy/handlers.rs:1051-1060
- evidence: non-2xx responses store `let body_text = String::from_utf8(response.bytes().await?.to_vec()).ok();` into `ProxyError::UpstreamError { body: body_text }`; `Display` prints `{body:?}` and `get_error_message` formats `上游错误 ({status}): {body}`; `log_proxy_error` persists that message through `log_error_with_context`.
- impact: Provider error responses can include prompts, request fragments, tokens, or large HTML bodies and will remain in proxy request logs despite the R2 truncation work.
- recommendation: truncate/redact when constructing `UpstreamError`, and ensure `Display`, `get_error_message`, and usage logging only receive bounded summaries.

### [CDX-4] Failover enable is not atomic on P1 switch failure
- kind: fix-regression
- severity: low
- confidence: high
- file: src-tauri/src/services/proxy.rs:2388-2403; src-tauri/src/services/proxy.rs:2156-2161
- evidence: `config.auto_failover_enabled = enabled` is persisted before `self.switch_proxy_target(app_type, &p1_provider_id).await?`; the switch path can return `Err` for official providers: `Cannot switch to official provider during proxy takeover`.
- impact: The API can report enable failure while leaving `auto_failover_enabled=true`, causing later proxy behavior/UI state to diverge from the returned result.
- recommendation: switch first and persist after success, or rollback the flag on switch failure before returning the error.

### [CDX-5] Basic Auth leaves mutating endpoints CSRFable after browser credential caching
- kind: new-issue
- severity: low
- confidence: medium
- file: src-tauri/src/web_api/middleware/auth.rs:110-124; src-tauri/src/web_api/middleware/cors.rs:11-36; src-tauri/src/web_api/handlers/proxy.rs:63-75,149-158
- evidence: auth only checks `Authorization` and has no `Origin`/`Sec-Fetch-Site`/CSRF check; CORS defaults to same-origin, but CORS does not block simple cross-origin form POSTs; `/proxy/start-proxy-server` is a no-body `POST`.
- impact: If the operator’s browser has cached Basic credentials for the Tailscale URL, a malicious page can attempt state-changing form posts such as proxy start/stop.
- recommendation: add origin/fetch-metadata checks or restore a lightweight CSRF/request-intent token for mutating `/api/*` routes.

### [CDX-6] C1/C2 static traversal and non-loopback auth fixes are sound
- kind: confirmation-ok
- severity: info
- confidence: high
- file: src-tauri/src/web_api/middleware/auth.rs:111-124; src-tauri/examples/server.rs:237-244; src-tauri/src/web_api/routes.rs:108-116,135-140
- evidence: API auth treats only non-`/api/*` and exact `/api/health` as public; non-loopback bind refuses without `CC_SWITCH_WEB_AUTH_PASSWORD`; static serving only joins paths whose components are `Normal` or `CurDir`.
- impact: The audited unauthenticated remote API exposure and lexical `..` static traversal are closed.
- recommendation: none — sound.

### [CDX-7] IP guard range coverage and guarded redirect helper are sound where applied
- kind: confirmation-ok
- severity: info
- confidence: high
- file: src-tauri/src/proxy/ip_guard.rs:21-27,39-65; src-tauri/src/web_api/handlers/common.rs:167-220; src-tauri/src/proxy/http_client.rs:293-310
- evidence: IPv4 blocks loopback/link-local/private/unspecified/CGNAT; IPv6 blocks loopback/unspecified/link-local/ULA and unwraps IPv4-mapped addresses; `validate_outbound_url` resolves domains and rejects any blocked IP; `guarded_redirect_policy` stops blocked IP-literal redirect hops.
- impact: The shared guard correctly handles the expected internal ranges and IP-literal redirect bypasses when call sites use it correctly.
- recommendation: none — sound.

### [CDX-8] R1 session path loading is closed
- kind: confirmation-ok
- severity: info
- confidence: high
- file: src-tauri/src/session_manager/mod.rs:93-123,201-223; src-tauri/src/session_manager/providers/opencode.rs:237-261; src-tauri/src/session_manager/providers/hermes.rs:197-220
- evidence: `load_messages` routes SQLite sources through provider-specific validators; file sources canonicalize the source and each existing provider root and require `validated_source.starts_with(&validated_root)`; OpenCode/Hermes require the SQLite DB path to canonicalize equal to the expected provider DB.
- impact: Arbitrary file reads via `source_path`, `..`, symlink escape, or foreign `sqlite:<db>` are blocked.
- recommendation: none — sound.

### [CDX-9] F7 circuit-breaker bypass and F8 lifecycle locking are sound
- kind: confirmation-ok
- severity: info
- confidence: high
- file: src-tauri/src/proxy/handler_context.rs:127-137; src-tauri/src/proxy/forwarder.rs:387-420; src-tauri/src/services/proxy.rs:63-70,418-477,1076-1107
- evidence: provider selection receives the already-loaded failover config; bypass is `should_bypass_circuit_breaker(failover_enabled)`, not `providers.len()==1`; start/stop share `start_stop_lock` and re-check state under the lock.
- impact: The prior circuit-breaker and concurrent start/stop races are addressed.
- recommendation: none — sound.

### [CDX-10] F5/F6 bootstrap extraction and web init ordering are sound
- kind: confirmation-ok
- severity: info
- confidence: high
- file: src-tauri/examples/server.rs:270-301,332-342; src-tauri/src/bootstrap.rs:155-173,190-201; src-tauri/src/lib.rs:412-432; src-tauri/examples/web_proxy.rs:54-55; src-tauri/src/proxy/mod.rs:18-20
- evidence: web loads legacy JSON before `Database::init()`, applies migration after DB creation, then calls shared `bootstrap::run_post_db_bootstrap`; desktop calls the same bootstrap function; web initializes runtime context and global outbound proxy before restore; `ip_guard` is path-included from the same source module.
- impact: Web and desktop bootstrap behavior are aligned without introducing Tauri dependencies into the web path.
- recommendation: none — sound.

### [CDX-11] Frontend web-mode sync gating is sound
- kind: confirmation-ok
- severity: info
- confidence: high
- file: src/components/settings/WebdavSyncSection.tsx:473-476,1099-1110,1388-1402; src/i18n/locales/en.json:446,557; src/lib/api/adapter.ts:222-247
- evidence: WebDAV persists `autoSync: webMode ? false : form.autoSync`; WebDAV/S3 auto-sync switches render unchecked and disabled in web mode; locale keys exist; web fetch helpers consistently use `credentials: "include"` and no CSRF header plumbing.
- impact: Desktop-only auto-sync cannot be re-enabled accidentally from web UI, and the frontend no longer references removed CSRF middleware.
- recommendation: none — sound.

## SUMMARY
Overall posture is materially better: C1/C2, R1, bootstrap parity, failover bypass, and lifecycle locking are genuinely fixed.
The most important remaining issue is SSRF coverage, not the IP guard itself: usage queries and several redirected service calls still bypass full guarded dialing.
I consider C1, C2, and R1 closed. I consider P4 SSRF only partially closed until the saved usage and redirect-following gaps are fixed.
R2 log privacy is also only partially closed because full upstream error bodies still persist through `ProxyError`.