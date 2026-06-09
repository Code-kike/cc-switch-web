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
