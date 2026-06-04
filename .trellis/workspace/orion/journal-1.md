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
