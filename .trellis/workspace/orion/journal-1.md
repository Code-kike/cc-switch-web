# Journal - orion (Part 1)

> AI development session journal
> Started: 2026-05-12

---



## Session 1: Audit remediation (#2-#6): pricing, CI gates, web SSRF, parity, hygiene

**Date**: 2026-06-03
**Task**: Audit remediation (#2-#6): pricing, CI gates, web SSRF, parity, hygiene
**Branch**: `fix/audit-remediation-pricing-ci-ssrf-parity`

### Summary

Comprehensive review workflow surfaced C1 (web API auth unwired, deferred) + pricing/CI/SSRF/parity findings. Fixed #2-#6: pricing fallback chain + bare claude-sonnet-4-6 seed + tests; CI gates (web_update cfg-gate, example required-features, clippy); web-layer SSRF guard (desktop unaffected); M2-M6 parity/robustness; docs hygiene (146 deletions, README links, CHANGELOG 3.16.0). All gates green. #1/C1 auth + H3 redaction intentionally deferred.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `7114e460` | (see git log) |
| `eba33735` | (see git log) |
| `88d842f1` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 2: Cross-validation + cost-accuracy follow-up

**Date**: 2026-06-03
**Task**: Cross-validation + cost-accuracy follow-up
**Branch**: `fix/audit-remediation-pricing-ci-ssrf-parity`

### Summary

Round 1 fixed Codex follow-up findings (bare claude-haiku-4-5 seed; atomic_write 0600-at-create; GET->POST for api_key-bearing endpoints). A final workflow+Codex cross-validation confirmed the PR clean + 8 gates green and surfaced more PRE-EXISTING cost-accuracy traps, fixed in Round 2: [1m] suffix stripping in pricing lookup; a shared pricing_lookup_candidates() unifying the proxy + session-log matchers; cache double-billing across all 3 OpenAI->Anthropic transform paths (incl. transform_responses.rs caught by trellis-check); 6 niche-id seeds (lowercase). 4 niche ids (ark-code-latest, qianfan-code-latest, KAT-Coder-Air, Hermes-4-405B) left unseeded pending user prices. SSRF 0.0.0.0 + C1 auth intentionally deferred (personal use).

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `080619d6` | (see git log) |
| `f7725735` | (see git log) |
| `18a9a9d2` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 3: Fix Frontend Checks CI: split web-server integration suites

**Date**: 2026-06-04
**Task**: Fix Frontend Checks CI: split web-server integration suites
**Branch**: `fix/ci-split-web-server-integration-suites`

### Summary

CI Frontend Checks failed (pre-existing on main+all branches) because pnpm test:unit ran the 20 *.web-server.test.tsx E2E suites that boot a real server (dist-web + cargo --example server), unavailable in the Node-only FE job. Option A: vitest.config.ts excludes **/*.web-server.test.tsx; new standalone vitest.integration.config.ts includes only them; package.json adds test:integration. test:unit now 104 files/521 tests/0 failed Node-only. PR #13. Out of scope (surfaced): web_proxy/web_services examples are pre-existing broken stubs under --features web-server (unused; CI skips via required-features; integration uses --example server).

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `fea1901f` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 4: Fix web-server example build: model_mapper regression + autoexamples

**Date**: 2026-06-04
**Task**: Fix web-server example build: model_mapper regression + autoexamples
**Branch**: `fix/cargo-autoexamples-web-server-includes`

### Summary

Surfaced + fixed a regression: the server web-server example (deployable web binary) failed to compile under --features web-server (E0433 cannot find model_mapper in proxy) since PR #11, where Round-2 added crate::proxy::model_mapper to usage_stats.rs but examples/web_proxy.rs (server's mod proxy) never declared it. Undetected: cargo test (desktop) skips the example via required-features + a prior --example server gate gave a FALSE PASS. Fix: web_proxy.rs declares the model_mapper path module. Also set autoexamples=false + explicit-only server example, so cargo no longer treats web_proxy/web_services (no fn main; server.rs #[path] includes) as standalone example targets. PR #14. Verified: --example server compiles; cargo test/clippy/fmt green.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `fdf41ca8` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 5: Add CI web-server example compile check

**Date**: 2026-06-04
**Task**: Add CI web-server example compile check
**Branch**: `ci/web-server-example-compile-check`

### Summary

Closed the CI coverage gap that hid the PR #14 regression: backend job's cargo test/clippy run desktop features and skip the server example (required-features), so the web-server binary was never compiled in CI. Added a 'cargo check --no-default-features --features web-server --example server' step to the backend job (cargo check, not clippy -D warnings, due to ~189 benign example dead-code warnings). PR #15, all CI green. Future 'proxy dep added to a shared service but not wired into examples/web_proxy.rs' regressions now fail CI directly. Noted: .gitignore:31 has a .github rule (ci.yml is tracked so unaffected; new workflow files would need git add -f).

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `112cc83b` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 6: Repo hygiene: untrack .github + dedup archived task dirs

**Date**: 2026-06-04
**Task**: Repo hygiene: untrack .github + dedup archived task dirs
**Branch**: `chore/repo-hygiene-gitignore-task-dedup`

### Summary

Removed the .github line from .gitignore (dormant trap: new .github files would not auto-add; the 12 tracked files unaffected). Deduped 4 already-archived task dirs that were tracked in BOTH .trellis/tasks/ and tasks/archive/2026-06/ (squash-merge artifact) — removed the active-location copies (17 files), kept archive/. get_context no longer lists them as active. PR #16. No source/build behavior changes.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `1161253b` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 7: Fix non-security review findings

**Date**: 2026-06-04
**Task**: Fix non-security review findings
**Branch**: `chore/repo-hygiene-gitignore-task-dedup`

### Summary

Fixed non-security review findings across web-server behavior, WebDAV confirmation, usage scripts, restore/live-sync consistency, web file picker cancellation, and SSE reconnect handling; validation passed for frontend and Rust test suites.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `e0b85277` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 8: Track Trellis scaffold files

**Date**: 2026-06-04
**Task**: Track Trellis scaffold files
**Branch**: `chore/repo-hygiene-gitignore-task-dedup`

### Summary

Committed reusable Trellis/Codex scaffold files, added local ignore coverage for Trellis template hashes, and kept runtime/developer state out of git.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `ff7262bc` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 9: Run web-server smoke test

**Date**: 2026-06-04
**Task**: Run web-server smoke test
**Branch**: `chore/repo-hygiene-gitignore-task-dedup`

### Summary

Ran standalone web-server smoke validation successfully, confirmed prior GitHub CI success, ran local route coverage, typecheck, and unit tests, and recorded the smoke validation contract in frontend quality guidelines.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `f959cc0a` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 10: Update GitHub Actions runtime compatibility

**Date**: 2026-06-04
**Task**: Update GitHub Actions runtime compatibility
**Branch**: `chore/repo-hygiene-gitignore-task-dedup`

### Summary

Upgraded outdated GitHub Actions workflow action majors to avoid Node.js 20 action runtime deprecation, verified workflow YAML parsing, typecheck, format check, unit tests, and recorded the reusable CI runtime guidance in Trellis specs.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `6e54bb63` | (see git log) |
| `649bbeb3` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 11: Deep-read cc-switch + fix all non-security findings

**Date**: 2026-06-06
**Task**: Deep-read cc-switch + fix all non-security findings
**Branch**: `fix/non-security-deep-read-findings`

### Summary

3-round architecture deep-read (18 subsystems) then fixed all ~45 NON-security findings across batched commits B1-B8 (security excluded per personal-use scope): data-loss/correctness (Claude settings merge, OpenCode comments, cost=0 pricing, Gemini OAuth refresh, SSE hang, switch_lock, schema clean, model-mapping), proxy behavior (rectifier/optimizer M8-M11/L5/L6), refactors (M1 forwarder dedup, M2 CB sliding window, L26 conservative God-component extraction, serde_yaml->serde_yaml_ng), FE react-query/types/M40 form-validation, CI hardening (web build/parity/locale/smoke/integration + clippy --all-targets/fmt gates), docs (web-first README_ZH/JA). Several 'looked-like-bugs' descoped-with-evidence (M32/L29/L27/M10/L22/M3/M6). Caught+fixed a real Claude-rename revert regression via newly-wired integration tests. Full CI gate suite green: 1272 Rust + 575 FE unit + 50 integration tests, clippy/fmt/typecheck/prettier/route-parity/locale-parity all clean.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `ebfb0835` | (see git log) |
| `a9a36d59` | (see git log) |
| `068736c2` | (see git log) |
| `f96afa2f` | (see git log) |
| `19eac480` | (see git log) |
| `6dda2d78` | (see git log) |
| `64166ce7` | (see git log) |
| `f0006877` | (see git log) |
| `334af2b5` | (see git log) |
| `f9b2abbb` | (see git log) |
| `466bbefd` | (see git log) |
| `d0be273d` | (see git log) |
| `c8ea038c` | (see git log) |
| `27ccf227` | (see git log) |
| `065dd870` | (see git log) |
| `e8e4dee1` | (see git log) |
| `93cd3ff0` | (see git log) |
| `64dc3fdc` | (see git log) |
| `ae38bb7b` | (see git log) |
| `86b1d606` | (see git log) |
| `a125cc34` | (see git log) |
| `cb7c6870` | (see git log) |
| `229e795e` | (see git log) |
| `f96df43c` | (see git log) |
| `c1faedfe` | (see git log) |
| `fb2e7377` | (see git log) |
| `d241c5e6` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 12: Round 2: fix cross-validated non-security findings (Claude x Codex)

**Date**: 2026-06-10
**Task**: Round 2: fix cross-validated non-security findings (Claude x Codex)
**Branch**: `fix/non-security-findings-round-2`

### Summary

Fresh full review (17-agent Workflow + independent Codex gpt-5.5) + 2-round Claude-Codex debate converged on a fix plan; implemented the 8 NON-SECURITY items (7,10,11,12,13,14,15,16) inline (gateway down for sub-agents). Codex review approved 7/8 + scope; item 12 (multipart oversize -> opaque 500) fixed per Codex's exact prescription (ApiError::from_multipart preserving status()->413). All gates green (cargo fmt/clippy --all-targets/test 1272, web example 9, typecheck, test:unit 578, route+locale parity, test:integration 50/50). Security items left untouched per personal-use scope. Committed 51261807.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `51261807` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 13: Sync upstream cc-switch v3.16.1+v3.16.2 into web fork

**Date**: 2026-06-11
**Task**: Sync upstream cc-switch v3.16.1+v3.16.2 into web fork
**Branch**: `main`

### Summary

Ported the shared-code surface of upstream v3.16.1+v3.16.2 in 12 batched sync commits adapted to the dual-runtime fork (S3 cloud sync, OpenCode usage sync, official-subscription quota templates, media image-fallback rectifier, Codex auth-preservation toggle + takeover hardening, /v1/models probe, ZenMux+CherryIN providers, MiniMax pricing). Version bumped to 3.16.2. Deferred per decision: Codex Chat routing stack + model-catalog (follow-up task created), ClaudeDesktop app, Windows desktop polish. trellis-check SHIP, all 14 gates green. Merged to main (6f472ace), pushed to origin, redeployed the systemd web service (now serving 3.16.2 on 0.0.0.0:3010).

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `f48138c0` | (see git log) |
| `d5cc0039` | (see git log) |
| `b8f6dbe2` | (see git log) |
| `50dc1e9d` | (see git log) |
| `34cf2330` | (see git log) |
| `886186d3` | (see git log) |
| `e6f42a01` | (see git log) |
| `a82a0367` | (see git log) |
| `b55e2f18` | (see git log) |
| `ca797628` | (see git log) |
| `91c1b55c` | (see git log) |
| `ddfa61cb` | (see git log) |
| `6f472ace` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 14: Port routing proxy + random auto-failover to web runtime

**Date**: 2026-06-12
**Task**: Port routing proxy + random auto-failover to web runtime
**Branch**: `main`

### Summary

Ported the desktop-only routing proxy to the web-server runtime in 5 slices: ProxyRuntimeCtx de-tauri (18 sites, desktop byte-identical), real ProxyService compiled in both runtimes (proxy_web.rs stub deleted, web_proxy.rs is a test-enforced 1:1 mirror), headless lifecycle in examples/server.rs (crash recovery, takeover restore, bounded graceful shutdown — trellis-check found and fixed a Critical: open SSE connections blocked shutdown causing SIGKILL with stale PROXY_MANAGED placeholders), web loopback-only listener (D4), FE proxy controls un-hidden in web mode, and FailoverStrategy {sequential|random} per app — random is claude-code-hub-style sticky-until-failure + uniform re-roll among circuit-closed providers. All gates green (FE 601, Rust 1427 desktop + 1372 web, smoke 123, integration 50). Merged to main (abb71c40), pushed, redeployed; live-verified failoverStrategy field and D4 400-rejection on the running service.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `260b5153` | (see git log) |
| `6dba3361` | (see git log) |
| `37fd9598` | (see git log) |
| `e44abf2d` | (see git log) |
| `ba53338b` | (see git log) |
| `cc4e24ef` | (see git log) |
| `abb71c40` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 15: Fix web runtime global outbound proxy init (failover unreachable relays)

**Date**: 2026-06-12
**Task**: Fix web runtime global outbound proxy init (failover unreachable relays)
**Branch**: `main`

### Summary

User-reported bug: codex auto-failover failed while single-account direct calls worked. Root cause: examples/server.rs never called http_client::init, so the forwarder hit the GP-004 fallback and made direct outbound connections, ignoring any saved global proxy URL — relay providers requiring the user's outbound proxy were unreachable from the local routing proxy. Fixed by mirroring desktop lib.rs:895-922 as lifecycle step 2/5 (before takeover restore) with identical GP-005..GP-008 semantics; lifecycle pin test extended + functional test added (web example suite 1373/0). Merged a4b9bcee, deployed, boot log confirms init runs. Second finding: the user had no global proxy URL saved in cc-switch at all (GET returns null) — instructed to set it in the web UI (hot-applies via the existing handler and now survives restarts).

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `74635c8f` | (see git log) |
| `a4b9bcee` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 16: Audit + 2-round verification co-review of cc-switch-web (Phase 1-4)

**Date**: 2026-06-15
**Task**: Audit + 2-round verification co-review of cc-switch-web (Phase 1-4)
**Branch**: `fix/web-audit-phase1-2`

### Summary

codex+Claude audit (13 consensus findings, multi-round debate) fixed across Phase 1-3 (C1 traversal, C2 Basic auth, F3/F4/F11 SSRF, F5/F6 bootstrap+migration parity, F7 breaker, F8 start-lock, F9 failover SSOT, F10 web-gate, M1, L2). A verification review (6-reviewer Workflow + codex, cross-validated) then found real SSRF gaps + an S3 regression in those fixes; Phase 4 closed them: get_guarded redirect-revalidating client + tauri-free ip_guard SSOT (0.0.0.0/CGNAT), testusagescript+stream_check guards, schemeless-S3 normalization, CSRF-theater removal, 4 dead-stub routers deleted. 3 commits on branch fix/web-audit-phase1-2 (NOT merged/deployed). Residual authenticated-only items (sourcePath read, MCP write-exec, debug-log prompts, OAuth plaintext) deferred to a follow-up task.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `e65b8a07` | (see git log) |
| `2a0b3305` | (see git log) |
| `a20ea3e1` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 17: Harden residual authenticated web API vectors (R1 session-path guard + R2 log privacy)

**Date**: 2026-06-15
**Task**: Harden residual authenticated web API vectors (R1 session-path guard + R2 log privacy)
**Branch**: `fix/web-audit-phase1-2`

### Summary

Follow-up to the archived audit task: R1 — session_manager::load_messages now root-guards source_path (faithful mirror of delete_session_with_roots) + opencode/hermes sqlite db-path validation, closing an authenticated arbitrary file/SQLite read (+9 tests); R2 — shipped systemd RUST_LOG=info (prompt/response bodies are debug-only → privacy-by-default) + truncated the one remaining >=info body log (handlers.rs Claude parse-error). R3 (MCP-upsert write→exec) + R4 (OAuth 0o600 at rest) accepted as operator-only/standard-practice residuals. Commit 6abaccbf on branch fix/web-audit-phase1-2 (not merged/deployed).

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `6abaccbf` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 18: Round-2 verification of audit fixes (Workflow + codex) — closed 4 converged residuals

**Date**: 2026-06-15
**Task**: Round-2 verification of audit fixes (Workflow + codex) — closed 4 converged residuals
**Branch**: `fix/web-audit-phase1-2`

### Summary

Re-audited the audit-fix branch twice with the user-requested loop (Workflow 6-dim + codex blind → cross-validate findings to convergence → co-deliberate fix plan with codex ('converged') → implement → cross-validate fixes with codex ('verified')). Scope-C round (ed07d9ba): fixed 9 findings — usage custom-template SSRF, redirect-hardening (get_guarded), request-log truncation, F9 enable atomicity, CSRF cross-site intent check, OPTIONS preflight exemption, ip_guard exotic ranges, web background workers, dead-code cleanup. Round-2 (d5210074): closed 4 residuals the re-review found — native-template (token_plan/balance) base_url SSRF (now ALL web outbound usage paths guarded), provider_health.last_error truncation, auth IPv6 Host bracket-parse, set_auto_failover_enabled full queue+flag atomicity; accepted desktop redirect-policy parity. All via trellis-implement→trellis-check; gates green (desktop clippy -D +1457 tests, web 1422 tests, fmt/format/routes/locales/tsc). Branch fix/web-audit-phase1-2 NOT pushed/merged/deployed.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `ed07d9ba` | (see git log) |
| `d5210074` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 19: Sync cc-switch v3.16.5 into web fork

**Date**: 2026-07-07
**Task**: Sync cc-switch v3.16.5 into web fork
**Branch**: `fix/web-audit-phase1-2`

### Summary

Synced selected product-upstream v3.16.5 runtime changes into the Web-first fork, preserved Web security hardening, added release notes, verified Rust/frontend/Web smoke gates, and archived the Trellis task.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `52197b9c` | (see git log) |
| `34602326` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 20: Audit and fix upstream inherited bugs

**Date**: 2026-07-09
**Task**: Audit and fix upstream inherited bugs
**Branch**: `fix/web-audit-phase1-2`

### Summary

Ported product-upstream bug fixes for GLM 5.2 media fallback, Codex 30-day quota display, OpenCode resume commands, and usage/quota transient-failure last-good behavior with Web cache-bridge hardening.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `5b37ea2e` | (see git log) |
| `716fdb9a` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 21: Replace persistent cc-switch web service

**Date**: 2026-07-09
**Task**: Replace persistent cc-switch web service
**Branch**: `fix/web-audit-phase1-2`

### Summary

Built the current Web frontend and standalone web-server binary, installed them through scripts/install-cc-switch-web-service.sh, restarted cc-switch-web.service, generated/preserved systemd Web auth configuration, and verified service status plus /api/health.

### Main Changes

(Add details)

### Git Commits

(No commits - planning session)

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete
