# Fix-verification review — cross-validated consensus (Workflow 6 reviewers + codex), 2026-06-15

## Meta-verdict
The 13 committed fixes (e65b8a07 + 2a0b3305) are CORRECT IN SCOPE and REGRESSION-FREE: the
full-diff reviewer independently re-ran all gates green, codex marked C1/F4/F6/F7/F8/F9/F11/L2
"correct", and no auth-bypass / deadlock / tauri-leak / Phase-1-2-3 contract break was found.
The real problem is the SSRF hardening (F3/F4/F11) is INCOMPLETE — not a hard boundary — plus a
functional S3 regression and several cleanup items.

Cross-validation judgment: Claude's fan-out was MORE thorough on SSRF completeness (caught the
missed call sites + redirect bypass + IP-range gaps codex missed); codex + Claude independently
agreed on the S3 regression, F5 migration-skip, M1 incompleteness, F10 save, and CSRF
incompleteness; codex framed the C2 CSRF gap crisply.

## CONFIRMED issues (all firsthand-verified) — Phase 4 candidates

### SSRF cluster (F3/F4/F11 incomplete) — the dominant theme
- [HIGH] testusagescript unguarded — usage.rs:317 test_usage_script dials request.base_url, no validate_outbound_url. (Claude; codex missed.) VERIFIED.
- [HIGH] stream_check unguarded — providers.rs:553/591 stream_check_provider/all → check_with_retry dials provider base_url, no guard. (Claude project-health; codex missed.) VERIFIED.
- [HIGH] redirect bypass — http_client build_client sets no redirect policy; reqwest follows 10 redirects → 302→127.0.0.1/169.254.169.254 bypasses the initial-URL-only guard for balance/coding_plan/model_fetch/subscription. (Claude; codex missed.) VERIFIED. FIX must NOT blanket-Policy::none the shared forwarder client — use a separate guarded client or per-hop re-validation for the web handlers.
- [HIGH/regression] schemeless S3 — guard_s3_endpoint validates settings.endpoint as full URL, but split_scheme_host defaults bare `minio.example.com:9000` to https; Url::parse treats host as scheme → bad_request. Breaks documented MinIO sync. (BOTH.) VERIFIED. FIX: normalize scheme before validate.
- [MED] is_blocked_ipv4/v6 miss 0.0.0.0 / :: (unspecified → localhost on Linux) + CGNAT 100.64/10. (Claude.) VERIFIED. FIX: add is_unspecified() (+ optional 100.64/10).
- [MED, deferred-ok] DNS-rebinding TOCTOU between validate and dial (auth-gated residual). Document in guard.

### Auth / CSRF
- [MED] CSRF theater — system.rs:101 csrf_token returns static "stub-csrf-token"; verify_csrf never wired; FE adapter sends X-CSRF-Token; mod.rs advertises CSRF. (BOTH.) VERIFIED. Practical risk LOW (Basic-auth not cookies). FIX: either wire it OR delete the stub+FE plumbing+doc.
- [LOW] CORS preflight OPTIONS → 401 before CORS layer (only affects opt-in cross-origin). (Claude.) FIX: exempt OPTIONS in require_auth.

### Migration / bootstrap
- [LOW-MED] F5 corrupt config.json: headless web logs load-failure + creates empty DB → future migration permanently skipped. (BOTH.) Edge case. Optional: archive to .loadfailed or exit non-zero.

### Frontend (F10)
- [LOW] save re-persists stale autoSync:true in web mode (cosmetic; backend inert). (BOTH.) FIX: autoSync: webMode ? false : ...
- [LOW] auto-sync "last error" panels not web-gated → can contradict the desktop-only hint. (Claude.)

### Cleanup / docs
- [LOW] L2 incomplete — 4 more empty stub routers (copilot/vscode/model_test/universal) still merged. (Claude.) VERIFIED. FIX: delete like model_fetch.
- [MED] README deploy drift — README.md:69 "bind 0.0.0.0" with no mention of mandatory CC_SWITCH_WEB_AUTH_PASSWORD (manual 0.0.0.0 bind now hard-fails). (Claude.) VERIFIED.
- [LOW] web_api/mod.rs doc advertises CSRF/rate-limit/cookie-session that don't exist. (Claude.)
- [LOW, deferred] F8 update_config rebind not under start_stop_lock (fragile, not exploitable). (Claude proxy.)
- [LOW, deferred] M1 record_result still re-reads proxy_config threshold per result. (BOTH.)
- [LOW] smoke-web-server.mjs prettier drift (outside enforced glob).

## Recommended Phase 4 priority
P4-A (security, do first): guard testusagescript + stream_check; fix redirect bypass (web handlers); add is_unspecified() (+CGNAT). 
P4-B (regression, do first): fix schemeless-S3 normalization (restores MinIO).
P4-C (cleanup): resolve CSRF theater (recommend delete); README auth note; delete 4 dead stubs; F10 save-normalization + last-error gating; mod.rs doc.
Deferred-OK: DNS-rebinding doc, F8 update_config lock, M1 threshold, F5 .loadfailed.
