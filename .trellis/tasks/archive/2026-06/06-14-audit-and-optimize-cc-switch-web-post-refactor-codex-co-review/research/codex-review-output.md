[FIX C1]
VERDICT: correct
EVIDENCE: `src-tauri/src/web_api/routes.rs:102-131`, `src-tauri/src/web_api/routes.rs:139-145`
ISSUE: none
SEVERITY: n/a
CONFIDENCE: High

[FIX C2]
VERDICT: incomplete
EVIDENCE: `src-tauri/src/web_api/routes.rs:23-35`, `src-tauri/src/web_api/middleware/auth.rs:36-55`, `src-tauri/src/web_api/middleware/auth.rs:110-127`, `src-tauri/examples/server.rs:233-244`, `src-tauri/src/web_api/middleware/csrf.rs:15-24`, `src-tauri/src/web_api/middleware/rate_limit.rs:9-13`
ISSUE: Basic Auth and non-loopback refusal are correctly implemented, with exact `/api/health` exemption. CSRF remains a permissive stub and rate limiting remains unused, so the original “auth/CSRF/rate-limit” finding is only partially fixed.
SEVERITY: Medium
CONFIDENCE: High

[FIX F3]
VERDICT: defective
EVIDENCE: `src-tauri/src/web_api/handlers/webdav.rs:125-127`, `src-tauri/src/web_api/handlers/webdav.rs:153-200`, `src-tauri/src/web_api/handlers/s3.rs:121-126`, `src-tauri/src/web_api/handlers/s3.rs:147-193`, `src-tauri/src/services/s3.rs:50-57`, `src-tauri/src/services/s3.rs:72-86`
ISSUE: WebDAV is guarded correctly, and S3 handlers call the guard. But S3 now validates `settings.endpoint` as a full URL, while the S3 service still supports bare endpoints by implicitly treating `minio:9000` / `storage.example.com` as `https://...`. Those valid existing endpoints now fail before dialing.
SEVERITY: Medium
CONFIDENCE: High

[FIX F4]
VERDICT: correct
EVIDENCE: `src-tauri/src/web_api/handlers/subscription.rs:42-57`, `src-tauri/src/web_api/handlers/common.rs:200-253`
ISSUE: none
SEVERITY: n/a
CONFIDENCE: High

[FIX F5]
VERDICT: incomplete
EVIDENCE: `src-tauri/examples/server.rs:260-290`, `src-tauri/src/lib.rs:361-416`, `src-tauri/src/bootstrap.rs:155-180`
ISSUE: Valid legacy JSON is now preloaded before DB init and migrated afterward, which fixes the normal path. But web mode logs a legacy `config.json` load failure and still creates the SQLite DB at `server.rs:287`; later starts skip migration because the DB now exists. That preserves the “future automatic migration suppressed” failure mode for malformed or temporarily unreadable legacy configs.
SEVERITY: Medium
CONFIDENCE: High

[FIX F6]
VERDICT: correct
EVIDENCE: `src-tauri/examples/server.rs:293-301`, `src-tauri/src/lib.rs:418-432`, `src-tauri/src/bootstrap.rs:190-459`, `src-tauri/src/services/provider/mod.rs:23-30`
ISSUE: none
SEVERITY: n/a
CONFIDENCE: High

[FIX F7]
VERDICT: correct
EVIDENCE: `src-tauri/src/proxy/handler_context.rs:127-138`, `src-tauri/src/proxy/handler_context.rs:250-257`, `src-tauri/src/proxy/handlers.rs:122-134`, `src-tauri/src/proxy/handlers.rs:443-453`, `src-tauri/src/proxy/handlers.rs:508-518`, `src-tauri/src/proxy/handlers.rs:573-583`, `src-tauri/src/proxy/handlers.rs:912-922`, `src-tauri/src/proxy/forwarder.rs:387-415`, `src-tauri/src/proxy/forwarder.rs:2026-2028`
ISSUE: none
SEVERITY: n/a
CONFIDENCE: High

[FIX F8]
VERDICT: correct
EVIDENCE: `src-tauri/src/services/proxy.rs:64-70`, `src-tauri/src/services/proxy.rs:79-86`, `src-tauri/src/services/proxy.rs:419-474`, `src-tauri/src/services/proxy.rs:497-508`, `src-tauri/src/services/proxy.rs:568-584`, `src-tauri/src/services/proxy.rs:1077-1086`
ISSUE: none
SEVERITY: n/a
CONFIDENCE: High

[FIX F9]
VERDICT: correct
EVIDENCE: `src-tauri/src/commands/failover.rs:73-90`, `src-tauri/src/web_api/handlers/failover.rs:119-134`, `src-tauri/src/services/proxy.rs:2340-2422`, `src-tauri/src/proxy/runtime_ctx.rs:44-53`, `src-tauri/src/runtime/runtime_events.rs:21-24`, `src-tauri/src/runtime/runtime_events.rs:93-118`
ISSUE: none
SEVERITY: n/a
CONFIDENCE: High

[FIX F10]
VERDICT: incomplete
EVIDENCE: `src/components/settings/WebdavSyncSection.tsx:243-247`, `src/components/settings/WebdavSyncSection.tsx:462-474`, `src/components/settings/WebdavSyncSection.tsx:671-675`, `src/components/settings/WebdavSyncSection.tsx:1092-1103`, `src/components/settings/WebdavSyncSection.tsx:1382-1395`, `src/i18n/locales/en.json:444-447`, `src/i18n/locales/en.json:555-557`, `src/i18n/locales/zh.json:444-447`, `src/i18n/locales/zh.json:555-557`, `src/i18n/locales/ja.json:444-447`, `src/i18n/locales/ja.json:555-557`
ISSUE: The switches are visually disabled/off in web mode and localized. But saving settings in web mode still serializes the underlying `form.autoSync` / `s3AutoSync` state, so an existing persisted `autoSync=true` can remain true while the UI shows it disabled/off. Normalize saves to `autoSync: webMode ? false : ...`.
SEVERITY: Low
CONFIDENCE: High

[FIX F11]
VERDICT: correct
EVIDENCE: `src-tauri/src/web_api/handlers/common.rs:198-200`, `src-tauri/src/web_api/handlers/common.rs:238-249`, `src-tauri/src/web_api/handlers/system.rs:193-233`, `src-tauri/src/services/speedtest.rs:112`
ISSUE: none
SEVERITY: n/a
CONFIDENCE: High

[FIX M1]
VERDICT: incomplete
EVIDENCE: `src-tauri/src/proxy/handler_context.rs:93-98`, `src-tauri/src/proxy/handler_context.rs:127-138`, `src-tauri/src/proxy/provider_router.rs:67-87`, `src-tauri/src/proxy/provider_router.rs:193-227`
ISSUE: The redundant config read inside provider selection is removed. The original hot-path DB concern is not fully fixed: failover selection still reads all providers and queue per request, and `record_result()` still re-reads `proxy_config` for the threshold per provider result.
SEVERITY: Low
CONFIDENCE: High

[FIX L2]
VERDICT: correct
EVIDENCE: `src-tauri/src/web_api/handlers/mod.rs:1-3`, `src-tauri/src/web_api/handlers/mod.rs:5-33`, `src-tauri/src/web_api/routes.rs:39-69`
ISSUE: none
SEVERITY: n/a
CONFIDENCE: High

## NEW ISSUES

None separate from the defective/incomplete fix items above. I did not run build/tests because this review environment is read-only and those commands would write build artifacts.