REVIEW ROUND — the fixes are now IMPLEMENTED; verify them. Earlier you audited cc-switch-web and we reached consensus on 13 findings (C1, C2, F3, F4, F5, F6, F7, F8, F9, F10, F11, M1, L2). All have now been implemented across 2 commits on branch `fix/web-audit-phase1-2`. This is INDEPENDENT verification — be skeptical and evidence-based; every claim cites a concrete file:line you read.

See the full change set:
  git diff eb8ce994..HEAD        (29 files, +1865/-497)
  git log --oneline eb8ce994..HEAD

Deployment model (unchanged): web binary `examples/server.rs` on 0.0.0.0:3010 over Tailscale; Win10 browser; API holds live secrets. Dual-runtime: `src/lib.rs` desktop-only; web re-includes ~30 src modules via #[path]; reachable modules must stay tauri-free.

## How each finding was fixed — verify each is CORRECT and COMPLETE
- C1: `routes.rs::is_safe_relative_asset` gates every `dist_root` read (rejects `..`/RootDir/Prefix) before the join.
- C2: `web_api/middleware/auth.rs::require_auth` (HTTP Basic vs CC_SWITCH_WEB_AUTH_PASSWORD/USER, constant-time), layered in `routes.rs::build_router`; `examples/server.rs` refuses a non-loopback bind unless `auth::is_configured()`; systemd 0600 password drop-in; install uses `restart`. `ALLOW_HTTP_BASIC_OVER_HTTP` removed.
- F3/F4/F11: `web_api/handlers/common.rs::validate_outbound_url` is now async (`tokio::net::lookup_host`) and applied to webdav (4 handlers), s3 (4; empty endpoint = AWS default, skipped), subscription get_balance + get_coding_plan_quota (2); `system.rs::test_api_endpoints` caps urls at 50.
- F7: `forwarder.rs` bypass = `!failover_enabled` (was `providers.len()==1`), plumbed via `handler_context.rs::failover_enabled()` to 5 handler call sites.
- F5/F6: tauri-free `src/bootstrap.rs::{apply_legacy_json_migration, run_post_db_bootstrap}`; `lib.rs` (desktop) + `examples/server.rs` (web) both call. Web migration runs BEFORE `Database::init`; bootstrap runs after `AppState::new`, before `set_runtime_ctx`. `services/provider/mod.rs` un-gates one re-export.
- F8: `services/proxy.rs` `start_stop_lock: Arc<tokio::sync::Mutex<()>>` in `start()`/`stop()`.
- F9: `services/proxy.rs::set_auto_failover_enabled` SSOT (tauri-free, emits via UiEventSink; auto-add current->P1->emit provider-switched->refresh_tray on BOTH enable+disable); `commands/failover.rs` + `web_api/handlers/failover.rs` delegate.
- M1: `provider_router.rs::select_providers_with_config` reuses the per-request `AppProxyConfig` from `RequestContext`.
- F10: `WebdavSyncSection.tsx` gates both auto-sync toggles in web mode + i18n (en/ja/zh).
- L2: deleted `web_api/handlers/model_fetch.rs` + its merge.

## Output format

For EACH of the 13 findings:
```
[FIX <ID>]
VERDICT: correct | defective | incomplete
EVIDENCE: <file:line you read>
ISSUE: <if not correct: precisely what is wrong; else "none">
SEVERITY: <if issue> Critical|High|Medium|Low
CONFIDENCE: High|Medium|Low
```

Then a `## NEW ISSUES` section: any NEW bug / regression / tauri-leak / race / deadlock / dual-runtime drift the fixes INTRODUCED (not one of the 13), each with file:line + severity + minimal fix.

Scrutinize especially: F8 deadlock (tokio::sync::Mutex is NON-reentrant — any self.start()/self.stop() called while holding start_stop_lock is a deadlock); F9 desktop byte-for-byte parity + tauri-freeness + the tray-on-both-paths detail; C2 auth exemption holes (must be exact /api/health, not a prefix) + that every secret/mutating route is actually behind the layer; C1 traversal bypass (percent-encoding, absolute, ordering vs is_static_asset); any outbound-dial handler the SSRF guard missed; F5 migration ordering (must precede Database::init); F6 idempotency + lifecycle order + tauri-freeness.

Read-only. I (Claude) reviewed in parallel via a fan-out; we will cross-examine your verdicts against mine afterward, so only assert what the code supports.