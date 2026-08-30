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


## Session 22: Remove Web Basic Auth

**Date**: 2026-07-09
**Task**: Remove Web Basic Auth
**Branch**: `fix/web-audit-phase1-2`

### Summary

Removed Web Basic Auth from the standalone Web API, kept the same-origin intent guard for mutating browser requests, updated installer/systemd/docs/specs, and verified Rust plus web route/type checks.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `8fea1699` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 23: Deploy updated unauthenticated Web service

**Date**: 2026-07-10
**Task**: Deploy updated unauthenticated Web service
**Branch**: `fix/web-audit-phase1-2`

### Summary

Built and deployed the current Web server and assets, removed the legacy Basic Auth systemd drop-in, restarted the persistent service on 0.0.0.0:3010, verified unauthenticated API access and cross-site intent rejection, and corrected the spec verification route.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `76aa2b83` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 24: Audit and fix upstream Web integrity bugs

**Date**: 2026-07-10
**Task**: Audit and fix upstream Web integrity bugs
**Branch**: `fix/web-audit-phase1-2`

### Summary

Fixed stale file identity races, constrained SQL restore, transactional endpoint reconciliation, managed symlink writes, and Codex common-config and MCP atomicity; added regression coverage, ADRs, and executable specs.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `6acd6097` | (see git log) |
| `a3a1ab14` | (see git log) |
| `2a84acd0` | (see git log) |
| `0370b89a` | (see git log) |
| `f35a7309` | (see git log) |
| `08a04bb5` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 25: Redeploy Web server as persistent service

**Date**: 2026-07-10
**Task**: Redeploy Web server as persistent service
**Branch**: `fix/web-audit-phase1-2`

### Summary

Built and installed the current Web frontend and release server, preserved rollback artifacts, restarted the enabled systemd user service, verified Linger=yes, artifact checksums, listener/API/security behavior, and documented the persistent deployment rollback contract.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `f3631c18` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 26: Close out web-bug audit fixes (batches A'-E)

**Date**: 2026-07-25
**Task**: Close out web-bug audit fixes (batches A'-E)
**Branch**: `fix/web-audit-phase1-2`

### Summary

Archived 07-10-web-bug: all 5 fix batches (A' supply-chain/secrets, B upstream data-loss + TZ skew, C web dead-wiring, D gate hardening/CI, E upstream-core correctness & pricing) landed on fix/web-audit-phase1-2 and verified. Branch ready to merge into main ahead of the v3.18.0 upstream sync.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `0a82bfb1` | (see git log) |
| `6bb1f70b` | (see git log) |
| `06e84520` | (see git log) |
| `9714c727` | (see git log) |
| `17c0bf80` | (see git log) |
| `9e483c97` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 27: Sync upstream v3.18.0: batches S6-S8, version bump, full gate

**Date**: 2026-07-31
**Task**: Sync upstream v3.18.0: batches S6-S8, version bump, full gate
**Branch**: `sync/upstream-v3.18.0`

### Summary

Completed the v3.16.5->v3.18.0 upstream sync task. S6: Codex usage rebuild + session correctness (schema v16, serialized session sync with single-notification passes, stable proxy usage keys, shared rebuild_codex_usage across Tauri/Axum). S7: frontend error-log persistence to server disk via shared tauri-free logging.rs + full log-redaction hardening + pricing seeds; check caught 4 preset<->seed repricing defects incl. SSOT-test blind spots. S8: version 3.18.0 in 4 SSOT files, fork-authored changelog/release notes, integration triage (7 stale expectations traced to intentional behavior), smoke RUSTUP_HOME fix; full gate green incl. 124-probe web-server E2E with fresh-DB v16 migration. Spec: repricing-consistency, smoke toolchain pin, integration-deferral triage + 2 new contract scenarios (S6 sync/rebuild, S7 logging). Follow-ups: push branch, PR to main, then DB backup + rebuild + systemd restart per PRD.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `0bea3f54` | (see git log) |
| `9026406b` | (see git log) |
| `f5b453c6` | (see git log) |
| `c5651b4e` | (see git log) |
| `bdadebf4` | (see git log) |
| `73bfa123` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 28: Complete Product upstream v3.19.2 sync

**Date**: 2026-08-09
**Task**: Complete Product upstream v3.19.2 sync
**Branch**: `sync/upstream-v3.19.2`

### Summary

Ported and verified Product upstream v3.19.2 plus the catalog ownership fix with Web-first adaptations across pricing, usage, backup, Codex, OMO, management UI, Skills, and Hermes.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `f2d951d9` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 29: Port pi native coding agent + session usage into Web-first fork

**Date**: 2026-08-24
**Task**: Port pi native coding agent + session usage into Web-first fork
**Branch**: `sync/upstream-v3.20.0`

### Summary

Completed child task feat-pi-native-agent: ported Product upstream 84e75ad2 (pi native coding agent, 156 files) + 40d747c0 (pi session usage, 17 files) via 5-batch selective port. P1 Rust backend (pi_config/pi_state/pi_prompt_files/provider-pi/commands-pi/session_manager-pi + 9 commands with Web parity). P2 frontend provider UI (PiProviderForm 2038 lines + presets/catalog/thinking + AppId union + model_fetch request_headers/api_format). P3 frontend prompts/skills/sessions (PiPromptPanel + PiNativePromptResources + PromptLibrary, Q3 ruling kept non-pi PromptPanel path untouched with parameterized regression test). P3.5 corrected a P1 over-claim: services/skill.rs was ticked done but only 4 lines landed vs upstream +1141; ported 18 safety helpers (path alias/overlap validation, preserving uninstall, migrate alias rejection) + 19 regression tests, closing a bug where uninstalling a managed pi skill would delete a user's same-named external directory. P4 session_usage_pi importer + SCHEMA_VERSION 16->17 session_usage_dedup + docs/pi-native-contract-zh.md + i18n. Phase 2.2 check found three more P1 module-without-dispatcher gaps (session_manager never dispatched pi, PromptService lacked a Pi early-return so restore could rewrite AGENTS.md from DB flags, bootstrap skipped import_pi_providers_from_live) fixed in bdc273ff. Also found the fork's SQL restore authorizer is stricter than upstream: the new semantic index had to join SQL_RESTORE_INDEXES or WebDAV/SQL import is denied. Spec updated with an additive-provider wiring checklist, v17 restore contracts, and a new Pi Native AGENTS.md Prompt Activation scenario. Final gate: test:unit 173 files/1044 tests green, Rust parity 37, focused Rust 125, web-routes 292/0 gaps, locales 2637 parity, test:integration 50/54 (4 PRD flakes), build:web + smoke:web-server exit 0.

### Git Commits

| Hash | Message |
|------|---------|
| `18340719` | (see git log) |
| `a7fac324` | (see git log) |
| `cf465755` | (see git log) |
| `cd8b950a` | (see git log) |
| `43a72a5f` | (see git log) |
| `bdc273ff` | (see git log) |
| `ab02396c` | (see git log) |

### Status

[OK] **Completed**


## Session 30: Port Codex Alpha Search + Claude hosted WebSearch into Web-first fork

**Date**: 2026-08-25
**Task**: Port Codex Alpha Search + Claude hosted WebSearch into Web-first fork
**Branch**: `sync/upstream-v3.20.0`

### Summary

Completed child task feat-codex-alpha-websearch: ported Product upstream bdeaac75 (9 files, +10041/-1691) as 4 batches. W1 Codex Alpha Search passthrough (4 local aliases, fail-closed full-URL derivation). W2 hosted WebSearch request translation + ~30 markdown citation primitives with a 7-row fail-closed matrix. W2.5 fork-side hardening: the sub-agent proactively reported that 4 markdown shapes are O(n^2) (worst 6.7s CPU on 128 KiB of hostile model output) on an already-live path, so five caps were added at the single citation-dedup entry with deliberate fail-OPEN skip semantics, dropping the worst case to 116us while the legal caps-maxed worst case stayed flat at 57ms. W3 response/stream bridge + non-streaming aggregation + usage + three-language docs. Baseline research paid off: streaming_responses.rs and transform_responses.rs drifted only 28/78 lines from upstream pre-commit, so nearly 10k lines applied on upstream anchors; the flagged forwarder/handlers drift risk (2679/2287 lines) never materialized because every upstream anchor symbol existed. Q1 (skip transform_codex_anthropic.rs) was amended mid-task: its premise 'the fork has no Anthropic SSE aggregator' expired when W3 introduced exactly that, so one function was ported as a private fn in handlers.rs with mutation-proven necessity while all four deferred files stayed absent. Final check found two real defects: rewrite_codex_alpha_search_full_url sliced a raw URL by a length derived from Url::parse().path(), which panics on multi-byte boundaries and silently misderives on dot-segment URLs (present verbatim upstream, worth reporting there), and a 1ms Date.now() boundary flake fixed by porting upstream's own 60s margin. Spec grew two scenarios: fail-open vs fail-closed classified by what degradation costs, and the private-helper exception to a deferred-stack boundary. Final gate: cargo test --lib 2233, test:unit 173/1044 green, Rust parity 37, check:web-routes held at 292/280/0, test:integration 50/54 (4 PRD flakes), build:web + smoke exit 0.

### Git Commits

| Hash | Message |
|------|---------|
| `258245f4` | (see git log) |
| `a49e7af5` | (see git log) |
| `860b0523` | (see git log) |
| `c917d5cf` | (see git log) |
| `ff067851` | (see git log) |
| `e59d9c98` | (see git log) |

### Status

[OK] **Completed**

---

## 2026-08-26 — feat-managed-oauth-accounts 完成归档

**任务**：managed OAuth account selection（父任务 08-18-sync-upstream-v3.20.0 的第 3 个、最后一个子任务）
**分支**：sync/upstream-v3.20.0　**批次**：W0 调研 + W1–W5 实现，共 5 个实现 commit + 4 个 docs/spec commit

### 结果概要

移植 `a2e22f33`（54 文件 +11601/−1131）+ S4b 移交的 `0455a92c` managed-codex 事务到 fork。
基线不对齐（proxy.rs 撞 3661 行漂移）→ W0 逐 hunk 调研先行，实测 --3way 冲突块数后切批。

| 批次 | commit | 内容 |
|---|---|---|
| W1 | `38d04f5a` | 凭据与数据面 17 文件：id_token 进 managed bundle、reauth_required 双运行时、ADR0003 managed 原子写 |
| W2 | `6738c700` | provider 事务层净 diff（+2188）：managed helper 移入新模块 managed_codex.rs；add/update/switch 事务 |
| W3 | `89d0c7b3` | proxy 服务层 + 三处 official 阻断同 commit 放开（carve-out 比上游窄：仅 managed Official） |
| W4 | `979a32dc` | 前端最小表面（Q1 反推）：CodexFormFields 补 OAuth 块、10 个移交用例全绿 |
| W5 | `2f822677` | 剩余上游测试 + 文档同步 + 全量门禁 + PRD 验收核验 |

终态门禁：cargo test --lib **2289** passed / 5 ignored；test:unit **177 files / 1089 tests**；
integration 50/54（恰 4 白名单 flake）；web-routes **292/280/0**；locales **2664** parity；
SCHEMA_VERSION **17**；build:web / smoke exit 0。

### 关键裁定（详见 prd.md / implement.md）

- **Q3 不移植 migrate_legacy_codex_official_managed_binding**：`0455a92c` 在 v3.20.0 删除了它
  （git grep 零命中），移植反而会把用户 current provider 从固定 id 搬到新 UUID —— 上游主动放弃的破坏性改动。
- **Codex takeover carve-out 只放开 managed Official**：fork 无 inbound Authorization passthrough，
  unbound 原生登录保持 fail-closed（打开 = 把明确拒绝变成一次失败的 Codex 会话）。
- **前端谓词与 Rust 对齐**：supportsOfficialProxyTakeover 收窄为仅 managed_account +
  单一派生 isOfficialBlockedByTakeover（ProviderCard/useProviderActions 共用），全部变异验证钉住。
- **有意分歧备案**：useProviderActions.test 的 native-login takeover 用例反转上游断言；
  useProviderActions.ts / ProviderCard.tsx / providerCapabilities.ts 下次同步会冲突，须按取据判定。

### 方法论沉淀（已入 spec bdb99038）

1. 提交区间移植必须对**目标 tag** 核验每个交付物存在性（中间提交引入的符号可能被下游删除）。
2. 存在性检查用扩展通配（`find tests -name '*.test.ts*'`），单扩展名 grep 曾把既有 133 行套件误报为缺失。
3. Rust 策略的前端镜像谓词必须是单一派生 + 双侧测试钉住（doc 声称对齐 ≠ 行为对齐）。

### 教训

- 子代理 7 次被内容策略截断 → lean prompt 是唯一稳定形态；被截后主会话按 git status 接力收口。
- W4 曾把 test:integration 延到 W5，导致 AuthCenterPanel.web-server 2/2 回归晚一批才暴露 ——
  门禁延后 = 回归发现延后。

### Status

[OK] **Completed**

---

## 2026-08-26 — sync-upstream-v3.20.0 父任务完成并合入 main

**任务**：同步 Product upstream `413c09e0..v3.20.0`（71 非 merge 提交）到 Web-first fork
**结构**：父主体 S1–S8（65 提交）+ 三个子任务，各自独立规划/门禁/归档
**合并**：`65436f90`（`--no-ff`），0 冲突，main 3.18.0 → 3.20.0

### 交付

| 部分 | 结果 |
|---|---|
| 父主体 S1–S8 | 38 commit；每提交可审计处置（移植/适配/排除/延期），每个 skip 附 proven-inapplicable 取据 |
| 子任务 1 pi-native-agent | `7d839ed8` 归档；Pi 成为一等 provider，9 个新 Web 命令，schema v16→v17 |
| 子任务 2 codex-alpha-websearch | `b6f02cfc` 归档；Alpha Search 直通 + hosted WebSearch 全链转译，fail-closed URL 推导 |
| 子任务 3 managed-oauth-accounts | `e1b88f48` 归档；账号绑定只存 id、reauth 拒绝保存、carve-out 仅托管卡 |

终态门禁：`cargo test --lib` **2290** / test:unit **177 files / 1089** / parity **38** /
integration **50/54**（恰 4 白名单 flake）/ web-routes **292/280/0** / locales **2664** /
`SCHEMA_VERSION` **17** / build:web 与 smoke 均 exit 0。

### 意外发现（两个都值得记）

1. **main 实际在 3.18.0，不是 PRD 写的 3.19.2** —— v3.19.2 同步已归档但从未合入 main。
   规划期照抄了归档任务的结论，没读 `git show main:package.json`。同步链没断（本分支基于
   v3.19.2 工作切出），但合并一次交付两个周期。**教训：版本前提必须读当前 main 的实际文件。**
2. **发布文档反向失真** —— CHANGELOG 与三语 release notes 写于子任务落地前，仍称三特性
   「拆分为独立子任务，单独落地」并留「子任务范围之外」章节。**已交付能力被声明为未交付**，
   是集成 review 的唯一阻塞项（`6676dfdc` 已修）。比通常的「宣称未实现能力」反过来，但同样违反
   PRD「发布说明反映实际移植范围」。

### 集成 review 的三项契约

- **相互无冲突**：pi∩alpha=0、alpha∩oauth=0、pi∩oauth=15 文件（pi 建立的行零丢失）
- **与父主体无回退**（这项我最初派单漏了，经审阅补测）：父主体 28 个实现 commit 新增的全部
  Rust `fn` 与全部前端 `export` 在 HEAD 存活，脚本化核验零缺失；S2 never-clobber helper + 6 用例存活；
  父主体 i18n 2506 键零丢失
- **parity / 安全边界**：web-routes 逐项相同；五类上限原值；三子任务守卫互不削弱

### review 后修正（架构审阅连开两枪）

- `proxy.rs:1122` 是**第五处**未收敛的 official 判定（接管开启时的封禁警告），托管卡本该不告警却在告警。
  改用 `blocked_by_proxy_takeover` + 变异验证。为此给测试模块补了第一个录制型 event sink。
- 随后 `provider.rs` doc 仍写「四处执行点/切换必须被拒绝」，第五个消费者语义不同（警告 ≠ 拒绝）→
  改写为**分类谓词 + 五消费者表格**，并声明不得为措辞一致再分裂谓词。

### 计数被连续纠正两次（值得单独记）

同一张「内联 official 判定」表格被 grounded-reviewer 纠正两轮：
第一轮普查模式太窄（漏否定式与非 `as_deref`），第二轮改对模式后仍有两处差账
（`codex_config.rs:2647/2648` 是独立两处被并成一行；`provider.rs` 4 处被整文件 `rg -v` 排除而未说明理由）。
**教训：给下次同步用的清单，计数与排除项都必须可复现 —— 写明模式、写明减项、写明为什么减。**

### 方法论沉淀（已入 spec `bdb99038`）

1. 提交区间移植必须对**目标 tag** 核验每个交付物存在性（中间提交的符号可能被下游删除）
2. 存在性检查用扩展通配，单扩展名 grep 曾把既有 133 行套件误报为缺失
3. Rust 策略的前端镜像谓词必须单一派生 + 双侧测试钉住（doc 声称对齐 ≠ 行为对齐）

### 遗留（不阻塞）

- pi 徽章统一后 3 个无引用 locale 键（三语齐备，check:locales 绿）
- README 枚举 5 应用而 `AppId` 有 8（grokbuild/hermes 既有漂移 + pi 新增）→ 独立 README 任务
- `useProviderActions.ts` / `ProviderCard.tsx` / `providerCapabilities.ts` 的 Official-takeover 谓词
  与上游有意分歧，下次同步这三个文件会冲突，须按 in-code 注释取据判定
- **main 尚未推送 origin**（远端操作待确认）

### Status

[OK] **Completed**

---

## 2026-08-26 — v3.20.0 systemd 部署（运维动作，未建 Trellis 任务）

PRD 裁定 #11 把 systemd 部署列为合并后的独立确认步骤。本次执行。

### 现场：不是首次部署，是在跑服务的原地升级

| 项 | 升级前 | 升级后 |
|---|---|---|
| 二进制 | 8/1 构建（旧 main = v3.18.0） | 8/26 13:43，19.9 MB |
| 服务 | active，PID 1036（8/22 起） | active，PID 3591129，NRestarts=0 |
| 活库 schema | **16** | **17** |
| provider 数 | 24 | 28（+4 pi） |
| 代理接管 | 四 app 全 `enabled=0`，无接管态 | 同 |

### 关键判断：schema 升级单向

`schema.rs:449` 对 `user_version > SCHEMA_VERSION` 直接拒绝启动。所以**回滚不能只换二进制**，
必须同时恢复库。回滚三件套（已备好）：

```
~/.cc-switch/deploy-rollback/cc-switch-web.pre-3.20.0.20260826_133606       # 旧二进制
~/.cc-switch/deploy-rollback/cc-switch.pre-3.20.0.20260826_133606.db        # sqlite3 .backup 一致快照，integrity ok
systemctl --user restart cc-switch-web.service
```

库快照用 `sqlite3 .backup` 而非 `cp`（服务在跑，cp 可能取到不一致页）。
另外 `database/mod.rs:129` 自身在迁移前也会自动备份，备份失败即中止迁移 —— 双保险。

迁移 16→17 是纯新增（`CREATE TABLE IF NOT EXISTS session_usage_dedup` + `CREATE INDEX IF NOT EXISTS`），
无 ALTER / 无改写 / 无删除，所以实际风险低。

### provider 24 → 28 的解释（我自己的检查先报了"应为 24"，需澄清）

新增 4 行**全部是 `app_type=pi`**：`axonhub-claude` / `axonhub-gpt` / `axonhub-grok` / `axonhub-国产`，
与 `~/.pi/agent/models.json` 里的四个 provider 逐一对应 —— 是 pi 子任务的 `import_pi_providers_from_live`
从用户既有 Pi 原生配置导入，不是丢数据也不是种默认值。备份库里 `app_type='pi'` 计数为 **0**，
非 pi 的 24 行（claude 10 / codex 11 / gemini 2 / hermes 1）一个没少。

### 验证

- 服务 active / enabled，NRestarts=0
- `user_version` **17**，`integrity_check` **ok**
- `session_usage_dedup` 表已建
- HTTP `/` 200；`/api/providers/get-providers` 三个 app 计数与 DB 逐一相符（claude 10 / codex 11 / pi 4）
- 新特性端到端：PI-SYNC 导入 1 条（扫 81 文件）；pi 的 4 个 provider 经 API 可读

### 日志里两条非故障项

- `CODEX-SYNC deferred 114`：rollout 父链未写到 child fork 时刻，是 v3.19.2 S6b 引入的既有设计行为，非升级问题
- 一条 `501 Not Implemented`：**我自己验证时探错端点**造成（`/api/providers/claude` 不存在，
  desktop-only 命令在 web 模式返回 `WEB_NOT_SUPPORTED`）。查 `web-commands.ts` SSOT 后改用
  `/api/providers/get-providers` 即正常。**教训：验证端点应先读路由 SSOT，别凭直觉猜路径。**

### Status

[OK] **Completed** —— 部署完成，服务健康，回滚物料就位。

---

## 2026-08-27 — v3.20.0 部署后全面代码 review（goal 4a6c75e6）

触发：用户「部署使用存在问题」+「全面 review 3.20.0 trellis 建造任务所有代码」。无具体症状提供。

### 结论（每条独立证据）

1. **服务健康（事实，非判无 bug）**：active / NRestarts=0；RSS 稳态 47 MB（无泄漏）；近 2h 无 error 级日志。
2. **schema 迁移正确**：16→17 纯新增 `session_usage_dedup`，`integrity_check ok`。
3. **定价幂等**：191 行 191 唯一，重启重插不覆写用户自定义。
4. **新代码零 panic 面**：9 个新增文件 fn 体内 unwrap/expect/panic 全 0；安全常数到位
   （pi 128 MiB/500k entry/1 MiB，canonicalize 根解析，slug 跨平台校验）。
5. **`cargo test --lib` 2290 passed**；test:unit 177 文件 / 1089（合并期已证，此后无 src 改动）。
6. **pi 的 743 条「500」不是 cc-switch 代理失败**（四层证据，纠正过两次错误 grep）：
   - `proxy_config.enabled=0` 全 app（路由代理关闭）
   - 代理端口 15721 未监听（`ss` 无）
   - pi `baseUrl=127.0.0.1:8090`（`ah-` 前缀 = AxonHub 中继；8090 进程持有者在 orion 权限外）
   - pi DB 行 `data_source` 全 `pi_session`（session 文件导入），零代理日志
   → 失败是 Pi↔AxonHub(8090) 之间，cc-switch 只是把 session 里的 stopReason=error 原样
   导入用量面板展示。

### 曾被我下错、被审阅连着推翻的结论（记录教训）

- 「`Stream ended without finish_reason` 不在 cc-switch 源码」—— 搜带空格串漏了
  `streaming.rs:1278` 的 snake_case 测试名；该场景确为 cc-switch 一等处理（流无 finish_reason
  时静默丢弃终止事件，不发 message_delta/message_stop）。字符串本身非 cc-switch 产出，
  但「据此排除 cc-switch」证据不足 —— 只能靠端口/数据源/进程三重事实排除。
- 「服务健康、非我方问题」—— 在未拿到用户症状前就是越界结论。

### 审阅纠偏：成本核算这条我下错了结论

- 事实（已读 usage_stats.rs:628/633/637 与 session_usage_pi.rs:475-540 确认）：`total_cost` =
  `SUM(CAST(total_cost_usd AS REAL))` **无 status 过滤**；`total_requests` 用 `COUNT(*)`（含失败）；`success_count`
  才过滤 2xx；`success_rate = success_count / total_requests`。所以**失败(500)/中止(499)但 session 带 cost 的
  pi 请求会被计入「总花费」，成功数不算它们**（实测 42/750 失败 + 3/59 中止带非零成本，合计约 $7.92）。
- 我此前「语义一致、非 bug」是错的、且无证据：全库**没有测试/注释钉住**「失败成本是否应计入总花费」这个语义。
  两种语义都说得通（计=钱确实被上游扣了；不计=失败请求不是有效用量）。这是一个**未钉住的产品决策**，不是
  可以随口判 non-bug 的实现细节。
- `effective_usage_log_filter` 只做 session↔proxy 去重，且不涉及 `pi_session`（pi 不经代理，无需去重）。
- 待用户裁定："总花费"是否只统计成功(2xx)请求；若裁「是」，需统一 detail/rollup 两侧 SUM 加 status 过滤并补回归。
- CODEX-SYNC 启动刷数百条「找不到父 rollout」WARN：v3.19.2 S6b 既有行为（父链 dedup），非 3.20.0 回归；
  属日志噪声 + 历史 session 永久 defer，可作后续独立清理项。

### 未决

用户具体症状从未提供（两度询问未答）。「全面 review」可完成；「修用户的特定错误」需其可复现
报错/操作才能继续。若后续提供症状，从 AxonHub(8090) 中继与 pi 配置两条线入手复现。

### Status

[OK] **Review complete**（review 交付完成；用户症状待补充）

---

## 2026-08-27 — 3.20.0 文档目标「所有代码」系统性 review（goal 8f418e5e）

范围锚定：3.20.0 建造任务 = 父主体 S1–S8（`25e34700..cb6a1229`）+ 三子任务（`cb6a1229..HEAD`），
`git diff --name-only 25e34700..HEAD -- src src-tauri` = **206 文件**。

### 覆盖矩阵（与上一 goal 合起来 = 全量）

| 域 | 处置 |
|---|---|
| pi 后端 8 新文件 | 逐读/安全扫：128 MiB session cap、500k entry、1 MiB 文件、canonicalize 根、slug 可移植校验 |
| managed_codex.rs | 逐读：事务回滚 + `restore_preserving_newer_same_account_auth` 保持 never-clobber |
| session_usage_pi.rs | 全文读：去重账本/append 增量/快照边界/symlink 拒绝，13 测试 |
| pi_prompt_files.rs | 全文读：revision 冲突检测/原子写/rename 回滚 |
| alpha websearch | 5 个 MAX_CITATION_DEDUP_* 上限 + `strip_suffix` fail-closed URL 推导（非长度切片）均已落地 |
| 全 diff 缺陷扫描 | `todo!/unimplemented!/panic!/unsafe` 无问题；`unwrap/expect` 全为测试 fixture + 1 处闭集局部不变量；无非 UTF-8 边界裸切片 |
| 运行时 | 服务 health、迁移 16→17、定价幂等、成本核算、端口架构 |

### 权威门禁（review 收口复证）

fmt OK；typecheck OK；web-routes **292/280/0**；locales **2664** parity；`cargo test --lib`
**2290 passed / 0 failed / 5 ignored**（test:unit 177/1089 于合并期已证，review 期无 src 改动）。

### 结论

3.20.0 建造任务落地的 206 文件经系统 review **未发现需修复的 cc-switch 代码缺陷**；发现并记录的两个问题是
(a) 失败/中止但带 cost 的请求计入「总花费」——未钉住语义的产品决策（待裁），(b) 非 3.20.0 引入的
CODEX-SYNC 日志洪水。用户 pi 报错源头为其 AxonHub(8090) 中继，非 cc-switch 代理（代理 enabled=0 / 15721 未监听）。

### Status

[OK] **Review complete**（全量覆盖 + 门禁复证）

---

## 2026-08-27 — cost 修复补正：历史 rollup 泄漏边界（审阅指正）

审阅指出 e941a3e7 未覆盖历史数据——属实但需精确界定：

- **detail 层（<30 天，含 pi session）**：失败/中止成本 $5.67（710 条 500 + 53 条 499），
  在 `proxy_request_logs`，`usage_stats.rs` 的 2xx 门已覆盖、重部署后生效。这是用户当前可见的泄漏主体。
- **rollup 层（>30 天历史冻结）**：`usage_rollup.rs` 先前按旧逻辑聚合、detail 行已删，
  **无法按 status 精确重算**。量化上限：39 个含失败行，1617 请求中 636 失败（39.3%），
  这些行总成本 $197.10（即失败专属成本 ≤$197.10 的绝对上限，真实值通常远小于此——失败请求
  通常更早退出、成本更低）。相对 rollup 总成本 $4891，占比 ≤4%（上限口径）。

**结论**：fix-forward 正确；历史 rollup 泄漏有界、不可逆（detail 已删）、无合法回填（按均值
拆分会编造数据）。若不接受该 ≤$197 历史噪声，唯一干净修法是清空 `usage_daily_rollups` 重算——
但同样缺 detail 源，只能靠未来重新累积，会丢失 >30 天全部历史用量——不可取，故记录后保留。

### Status

[OK] **cost 修复 = 新数据生效 + 历史泄漏已量化记账**（重部署完成，服务 active，schema 17 不变）

---

## 2026-08-30 — v3.20.0 Web 回归审阅：两个用量链路缺陷（已修复并部署）

用户报「web 端很多 bug、无法使用」。以运行中的服务、真实数据库与浏览器为事实源复查，
定位并修复两个**真实且可复现**的缺陷，均属用量链路。

### F1 Codex 会话用量自 8/14 起永久停止导入（P0）

线上症状：每分钟扫 518 文件、重复 118 条 deferred WARN、导入 0；Claude/Pi 正常。
DB 佐证：Codex 最新记录停在 2026-08-14，Claude/Pi 为当日。

根因是把**永久状态误判为暂态**：`signatures_before` 用「父 rollout 的 max_timestamp
< child fork cutoff」判断父文件尚未写到 fork 时刻，但无法区分「仍在写」与「早已完结」。
`codex resume` 旧会话正是后者（实测父文件最后一行 07-29，子会话 fork 于 08-29）。
另有告警放大：重试分支先 `pending.remove()` 再让 `mark_deferred` 重新 insert，
去重基线被自己摧毁。

修法：120s 静默期视为父文件已定型（远高于同步周期，避免提前接受导致 replay prefix
变短而**重复计费**）；重试不再清除 pending。两处均经变异验证。
线上：首轮导入 271 条，Codex 用量恢复当日；告警由「每分钟」变为「首轮一次」。

### F2 回填成本后未清 `pricing_missing`（P1）

`usage_rollup` 用 `pricing_missing = 0` 同时门控聚合与清理，M4 注释承诺「补齐定价后重算」。
重算确实发生，但 UPDATE 只写成本列 → 刚被正确定价的行仍被永久排除出 rollup 且永不清理。
实测 `claude-opus-5` 232 行成本 $0 而种子早已存在。修复后生产库 still_flagged 232 → 0，
恢复计入 $1351.41。

### 方法论记录

- **"通过"不等于"有效"**：F1 的第一版去重测试只看周期末状态，变异验证显示它是**空断言**
  （remove+reinsert 后末状态相同）。改为让 `mark_deferred` 把决策随 `CodexFileSyncResult`
  返回并汇入 pass 汇总日志后，变异才真正 FAILED。凡是"防重复"类断言，必须观测事件本身。
- **架构审阅的分层纠偏成立**：为测试可观测性在生产缓存上加只写计数器是污染；
  既有逐文件结果通道就是正确出口，且顺带让运维看到「本轮新告警 N 个」。
- **不伪造修复**：`gpt-5.4-xhigh-px`（7880 万 tokens）无种子，成本确实未知，
  保持排除是对的；92 个 `MissingParent` 的父文件在磁盘上确不存在，fail-closed 保留。

### 覆盖诚实说明

深审并取证的是 session usage 全链、浏览器全部应用/工具页、Web API 契约、部署运行时。
302 文件基线的其余部分（S2 backup/sync 深审、S4/S5 表单细节、Alpha/OAuth 逐 hunk）
本轮**未做人工语义复核**，仅由既有回归覆盖。

### Status

[OK] 门禁：cargo test --lib **2296**、test:unit **177/1089**、integration **50/54**（恰 4 白名单 flake）、
web-routes **292/280/0**、locales parity、build:web + smoke exit 0、schema 17、integrity ok。
[OK] 已部署，服务 active / NRestarts=0。

## 2026-08-30 — F3 收尾

- Pi 编辑崩溃：非安全上下文下 crypto.randomUUID 缺失，三处裸调用已统一走 generateUUID
  兜底（xd 非 secure context 上的 LAN IP 验证通过）。回归测试带红绿対。
- CODEX DAO 搬迁收尾完成（reset/cursor/insert SQL 收入 dao，行为测试无一变动通过）。
