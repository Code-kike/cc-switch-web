# Workspace Index - orion

> Journal tracking for AI development sessions.

---

## Current Status

<!-- @@@auto:current-status -->
- **Active File**: `journal-1.md`
- **Total Sessions**: 28
- **Last Active**: 2026-08-09
<!-- @@@/auto:current-status -->

---

## Active Documents

<!-- @@@auto:active-documents -->
| File | Lines | Status |
|------|-------|--------|
| `journal-1.md` | ~999 | Active |
<!-- @@@/auto:active-documents -->

---

## Session History

<!-- @@@auto:session-history -->
| # | Date | Title | Commits | Branch |
|---|------|-------|---------|--------|
| 28 | 2026-08-09 | Complete Product upstream v3.19.2 sync | `f2d951d9` | `sync/upstream-v3.19.2` |
| 27 | 2026-07-31 | Sync upstream v3.18.0: batches S6-S8, version bump, full gate | `0bea3f54`, `9026406b`, `f5b453c6`, `c5651b4e`, `bdadebf4`, `73bfa123` | `sync/upstream-v3.18.0` |
| 26 | 2026-07-25 | Close out web-bug audit fixes (batches A'-E) | `0a82bfb1`, `6bb1f70b`, `06e84520`, `9714c727`, `17c0bf80`, `9e483c97` | `fix/web-audit-phase1-2` |
| 25 | 2026-07-10 | Redeploy Web server as persistent service | `f3631c18` | `fix/web-audit-phase1-2` |
| 24 | 2026-07-10 | Audit and fix upstream Web integrity bugs | `6acd6097`, `a3a1ab14`, `2a84acd0`, `0370b89a`, `f35a7309`, `08a04bb5` | `fix/web-audit-phase1-2` |
| 23 | 2026-07-10 | Deploy updated unauthenticated Web service | `76aa2b83` | `fix/web-audit-phase1-2` |
| 22 | 2026-07-09 | Remove Web Basic Auth | `8fea1699` | `fix/web-audit-phase1-2` |
| 21 | 2026-07-09 | Replace persistent cc-switch web service | - | `fix/web-audit-phase1-2` |
| 20 | 2026-07-09 | Audit and fix upstream inherited bugs | `5b37ea2e`, `716fdb9a` | `fix/web-audit-phase1-2` |
| 19 | 2026-07-07 | Sync cc-switch v3.16.5 into web fork | `52197b9c`, `34602326` | `fix/web-audit-phase1-2` |
| 18 | 2026-06-15 | Round-2 verification of audit fixes (Workflow + codex) — closed 4 converged residuals | `ed07d9ba`, `d5210074` | `fix/web-audit-phase1-2` |
| 17 | 2026-06-15 | Harden residual authenticated web API vectors (R1 session-path guard + R2 log privacy) | `6abaccbf` | `fix/web-audit-phase1-2` |
| 16 | 2026-06-15 | Audit + 2-round verification co-review of cc-switch-web (Phase 1-4) | `e65b8a07`, `2a0b3305`, `a20ea3e1` | `fix/web-audit-phase1-2` |
| 15 | 2026-06-12 | Fix web runtime global outbound proxy init (failover unreachable relays) | `74635c8f`, `a4b9bcee` | `main` |
| 14 | 2026-06-12 | Port routing proxy + random auto-failover to web runtime | `260b5153`, `6dba3361`, `37fd9598`, `e44abf2d`, `ba53338b`, `cc4e24ef`, `abb71c40` | `main` |
| 13 | 2026-06-11 | Sync upstream cc-switch v3.16.1+v3.16.2 into web fork | `f48138c0`, `d5cc0039`, `b8f6dbe2`, `50dc1e9d`, `34cf2330`, `886186d3`, `e6f42a01`, `a82a0367`, `b55e2f18`, `ca797628`, `91c1b55c`, `ddfa61cb`, `6f472ace` | `main` |
| 12 | 2026-06-10 | Round 2: fix cross-validated non-security findings (Claude x Codex) | `51261807` | `fix/non-security-findings-round-2` |
| 11 | 2026-06-06 | Deep-read cc-switch + fix all non-security findings | `ebfb0835`, `a9a36d59`, `068736c2`, `f96afa2f`, `19eac480`, `6dda2d78`, `64166ce7`, `f0006877`, `334af2b5`, `f9b2abbb`, `466bbefd`, `d0be273d`, `c8ea038c`, `27ccf227`, `065dd870`, `e8e4dee1`, `93cd3ff0`, `64dc3fdc`, `ae38bb7b`, `86b1d606`, `a125cc34`, `cb7c6870`, `229e795e`, `f96df43c`, `c1faedfe`, `fb2e7377`, `d241c5e6` | `fix/non-security-deep-read-findings` |
| 10 | 2026-06-04 | Update GitHub Actions runtime compatibility | `6e54bb63`, `649bbeb3` | `chore/repo-hygiene-gitignore-task-dedup` |
| 9 | 2026-06-04 | Run web-server smoke test | `f959cc0a` | `chore/repo-hygiene-gitignore-task-dedup` |
| 8 | 2026-06-04 | Track Trellis scaffold files | `ff7262bc` | `chore/repo-hygiene-gitignore-task-dedup` |
| 7 | 2026-06-04 | Fix non-security review findings | `e0b85277` | `chore/repo-hygiene-gitignore-task-dedup` |
| 6 | 2026-06-04 | Repo hygiene: untrack .github + dedup archived task dirs | `1161253b` | `chore/repo-hygiene-gitignore-task-dedup` |
| 5 | 2026-06-04 | Add CI web-server example compile check | `112cc83b` | `ci/web-server-example-compile-check` |
| 4 | 2026-06-04 | Fix web-server example build: model_mapper regression + autoexamples | `fdf41ca8` | `fix/cargo-autoexamples-web-server-includes` |
| 3 | 2026-06-04 | Fix Frontend Checks CI: split web-server integration suites | `fea1901f` | `fix/ci-split-web-server-integration-suites` |
| 2 | 2026-06-03 | Cross-validation + cost-accuracy follow-up | `080619d6`, `f7725735`, `18a9a9d2` | `fix/audit-remediation-pricing-ci-ssrf-parity` |
| 1 | 2026-06-03 | Audit remediation (#2-#6): pricing, CI gates, web SSRF, parity, hygiene | `7114e460`, `eba33735`, `88d842f1` | `fix/audit-remediation-pricing-ci-ssrf-parity` |
<!-- @@@/auto:session-history -->

---

## Notes

- Sessions are appended to journal files
- New journal file created when current exceeds 2000 lines
- Use `add_session.py` to record sessions