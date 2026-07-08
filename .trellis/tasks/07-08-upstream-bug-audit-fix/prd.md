# Audit and fix upstream inherited bugs

## Goal

Identify, prioritize, and fix bugs that this Web-first fork inherited from product-upstream synchronization or from fork adaptation of upstream behavior, while preserving the Web deployment model and existing security hardening.

## What I Already Know

- The user reports many bugs in the current Web project and believes many are inherited from upstream projects.
- The project glossary distinguishes `Product upstream` (`farion1231/cc-switch`) from `Web prototype upstream` (`Laliet/CC-Switch-Web`); the generic word "upstream" is ambiguous in this repo.
- Current package version is `3.16.5`, and the latest archived task synced selected `farion1231/cc-switch v3.16.2..v3.16.5` changes into this Web-first fork.
- The current branch is `fix/web-audit-phase1-2`; the working tree was clean before this task was created.
- Recent archived work already addressed several Web security and parity findings, including API auth, SSRF guard coverage, DB migration safety, request-log privacy, failover atomicity, web background workers, and product-upstream v3.16.5 synchronization.
- The Web-first runtime uses React/Vite for the frontend and Rust/Tauri shared backend code plus a standalone Web server.
- Existing verification scripts include `pnpm typecheck`, `pnpm test:unit`, `pnpm check:web-routes`, `pnpm check:locales`, `pnpm build:web`, Rust checks/tests, and `pnpm smoke:web-server`.
- The user selected the `Product upstream` source for the first audit slice and delegated all later non-blocking preference decisions to the recommended option.

## Assumptions

- The audit should preserve Web-first deployment behavior, non-loopback auth requirements, SSRF protections, log privacy defaults, and prior proxy/failover hardening.
- Product-upstream behavior is not automatically correct for this fork when it conflicts with remote Web-server safety or headless deployment constraints.
- Previously closed audit findings should not be reopened unless new evidence shows a regression or incomplete fix.
- Cosmetic upstream divergence is lower priority than correctness, data safety, security, and workflows that block normal usage.
- For future non-blocking scope or design choices, use the recommended Web-first, security-preserving option without asking again; ask only for true blockers involving safety, data-loss risk, or unknowable user intent.

## Open Questions

- None. Preference questions are delegated to the recommended option unless a true blocker appears.

## Requirements (evolving)

- Research likely upstream-reported bugs and local regression candidates before changing code.
- First audit slice: `Product upstream` (`farion1231/cc-switch`) reported/fixed bugs, synchronization misses, and Web adaptation drift.
- Cross-check any upstream bug report against this fork's current implementation before accepting it as actionable.
- Prioritize fixes by user impact and Web-first fork risk, not by upstream chronology alone.
- For each accepted bug, capture evidence, affected files, expected behavior, verification plan, and whether the fix is a direct upstream port or a Web-specific adaptation.
- First implementation batch ports small, high-confidence product-upstream post-`v3.16.5` fixes:
  - Volcano GLM 5.2 image fallback detection (`52534618`, issue `#5025`).
  - Codex free-plan 30-day quota tier rendering (`7a7d41c8`, issue `#3651` / PR `#4886`).
  - OpenCode session resume command update (`0cda8d46`, PR `#2359`).
- Defer larger product-upstream changes, including usage transient-failure keep-last-good, Codex renamed session title lookup, and project profiles, to later batches unless the small batch reveals a dependency.

## Acceptance Criteria (evolving)

- [x] The first audit slice is explicitly scoped and documented.
- [x] Candidate bugs are backed by upstream issue/release evidence or firsthand code/test evidence in this repo.
- [x] Accepted fixes include focused tests or a documented reason tests are not practical.
- [x] Prior Web-first security hardening remains intact.
- [x] Project lint/typecheck/test gates relevant to touched code pass, or blockers are documented.

## Definition of Done

- Tests added or updated where behavior changes.
- Lint, typecheck, and relevant test gates pass.
- Research notes are persisted under `research/` when external upstream evidence is used.
- PRD decisions are current.
- Specs/domain docs are reviewed for updates after implementation.
- Work is committed in coherent batches before task wrap-up.

## Out of Scope (temporary)

- Blindly merging either upstream project.
- Replacing Web-first server behavior with desktop-only behavior.
- Sponsor/referral/marketing/catalog-display-only churn unless it causes functional breakage.
- Re-litigating previously fixed audit findings without new evidence.

## Technical Notes

- Relevant archived tasks:
  - `.trellis/tasks/archive/2026-07/07-06-sync-upstream-cc-switch-v3-16-5`
  - `.trellis/tasks/archive/2026-06/06-14-audit-and-optimize-cc-switch-web-post-refactor-codex-co-review`
  - `.trellis/tasks/archive/2026-06/06-15-harden-verification-round-residuals-usage-redirect-ssrf-csrf-log-db-privacy-f9-atomicity-ops-cleanup`
- Release note baseline: `docs/release-notes/v3.16.5-en.md`.
- The repo does not currently include `docs/release-notes/v3.16.5-zh.md`; do not treat that as a bug without user-facing documentation scope confirmation.

## Research References

- [`research/product-upstream-post-3.16.5-bugs.md`](research/product-upstream-post-3.16.5-bugs.md) - product-upstream post-`v3.16.5` bug-fix candidates and first-batch selection.

## Technical Approach

First batch: apply three focused upstream fixes manually against the Web-first fork, preserving local structure and tests rather than blind cherry-picking. Add or update regression tests for each behavior:

- `src-tauri/src/proxy/media_sanitizer.rs` tests for GLM 5.2 text-only model classification and text-only upstream errors without image wording.
- `src-tauri/src/services/subscription.rs`, `src/components/SubscriptionQuotaFooter.tsx`, and locale files for `"30_day"` quota display.
- `src-tauri/src/session_manager/providers/opencode.rs` tests for `opencode -s <session_id>` resume commands.

## Implementation Update (2026-07-08)

- Ported Volcano GLM 5.2 media fallback behavior into `src-tauri/src/proxy/media_sanitizer.rs`: exact `glm-5.2` text-only classification, no prefix match for `glm-5.2v`, and reactive handling for text-only upstream errors that do not mention image/media.
- Ported Codex free-plan `30_day` quota support across backend tier mapping, tray summary grouping, frontend tier display, and en/zh/ja locale keys.
- Ported the OpenCode resume command change from `opencode session resume <id>` to `opencode -s <id>` for both sqlite and file-backed session metadata.
- Updated `.trellis/spec/frontend/quality-guidelines.md` with the cross-layer subscription quota tier contract.

## Verification Log (2026-07-08)

- `cargo fmt --manifest-path src-tauri/Cargo.toml`
- `pnpm exec prettier --write src/components/SubscriptionQuotaFooter.tsx tests/components/SubscriptionQuotaFooter.test.tsx src/i18n/locales/en.json src/i18n/locales/zh.json src/i18n/locales/ja.json`
- `cargo test --manifest-path src-tauri/Cargo.toml --lib media_sanitizer` — 19 passed.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib window_seconds_map_to_expected_tier_names` — 1 passed.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib codex_summary_thirty_day_only_still_renders` — 1 passed.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib scan_sessions_sqlite_reads_temp_database` — 1 passed.
- `pnpm exec vitest run tests/components/SubscriptionQuotaFooter.test.tsx` — 1 file and 4 tests passed.
- `pnpm test:unit -- tests/components/SubscriptionQuotaFooter.test.tsx` — not used as the targeted gate because the package script expands into the current 117-file suite; it hit the existing `tests/integration/App.test.tsx > covers basic provider flows via real hooks` 10s timeout after 602/603 tests passed.
- `pnpm typecheck`
- `pnpm check:locales` — en/ja/zh in parity, 2357 keys each.
- `git diff --check`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo check --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server` — passed with existing dead-code warnings in the web example.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- `pnpm format:check`
- `pnpm check:web-routes` — `missing: 0`.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib` — 1476 passed, 2 ignored.
- `git diff --check`

## Decision (ADR-lite)

### First Audit Slice

**Context**: The term "upstream bug" is ambiguous because this project has both a product upstream and a Web prototype upstream. Starting with all possible sources would blur the audit and slow down fixes.

**Decision**: Start with `Product upstream` (`farion1231/cc-switch`) bugs and fixes. Treat candidate bugs as actionable only after confirming they are present in this Web-first fork or that a Web adaptation missed the upstream fix.

**Consequences**: The first pass focuses on the most recent version-sync risk. `Web prototype upstream` and a broad local-only audit remain available as later slices, but they are not part of the first implementation batch unless product-upstream research points there.

### Preference Delegation

**Context**: The user asked that all later questions use the assistant's recommended answer.

**Decision**: For non-blocking decisions, choose the recommended Web-first, security-preserving option and document the choice. Ask only if the issue is a true blocker involving safety, data loss, or unknowable user intent.

**Consequences**: Planning and implementation can continue without pausing for low-risk preference choices, while still preserving escalation for decisions that should not be guessed.
