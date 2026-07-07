# sync upstream cc-switch v3.16.5 into web fork

## Goal

Synchronize this Web-first fork from product upstream `farion1231/cc-switch` version `v3.16.2` to `v3.16.5`, preserving the Web deployment model and the recent local hardening work.

## What I Already Know

- The user confirmed the sync target is `farion1231/cc-switch v3.16.2..v3.16.5`.
- Current project version is `3.16.2` in `package.json` and `src-tauri/Cargo.toml`.
- The product upstream has tags `v3.16.3`, `v3.16.4`, and `v3.16.5`.
- `origin/main` is not ahead of the current branch after fetch; the current branch is ahead by local hardening commits.
- `origin` has new Dependabot branches, but those are not part of the confirmed product-upstream sync scope.
- The repository names two upstream projects in README; terminology now distinguishes product upstream from web prototype upstream in `CONTEXT.md`.
- There is an existing active planning task for a deferred `v3.16.2` Codex routing/model catalog item.
- The user chose Web security hardening as the conflict-resolution priority and delegated any remaining preference questions to the recommended option.

## Assumptions

- The sync should land on top of the current local hardening state unless the user chooses a different integration base.
- Web-first behavior, non-loopback auth requirements, SSRF/CSRF/log privacy hardening, and proxy/failover fixes are not optional regressions.
- Upstream desktop-only features should be adapted, gated, or deferred when they conflict with Web-server runtime constraints.

## Open Questions

- None. If implementation uncovers a non-blocking preference decision, choose the recommended Web-first, security-preserving option and document it. If implementation uncovers a true blocker that cannot be inferred safely, ask.

## Requirements

- Sync selected product-upstream release deltas from `v3.16.2..v3.16.5`.
- Use the current `fix/web-audit-phase1-2` branch as the integration base.
- MVP scope is core product deltas plus Web adaptation: Codex native Responses/model catalog, unified session history, usage/pricing schema changes, and key proxy/provider correctness fixes.
- Include upstream schema v11 usage/pricing storage changes and adapt the DB-too-new recovery flow for the Web-first fork.
- Sync functional provider/preset catalog deltas that affect model availability, defaults, routing behavior, or usage/pricing correctness; exclude sponsor/referral/marketing copy and pure catalog-display polish unless needed by core behavior.
- Include upstream session category/group management as a complete feature, including UI, grouping operations, and relevant fixes, adapted for the Web-first fork.
- Preserve the Web-first fork's deployment model and recent security hardening.
- Use a heavy verification gate: frontend typecheck/build, Rust check/tests, DB migration tests, session organization smoke coverage, proxy/provider key-path checks, full existing test suite where practical, and a started Web service with end-to-end manual verification.
- Update product version metadata to `3.16.5`, and make changelog/release notes identify this as a Web-first fork sync to product upstream `v3.16.5`.
- Exclude desktop updater, release packaging, Windows ARM64, Linux GDK/Wayland, and similar release-operation deltas by default; only sync parts needed for Web service startup, version display, DB recovery, or security.
- Before schema v11 migration, create a local DB backup; on migration failure, preserve the original DB, block continued startup, and show or return clear recovery guidance without automatic downgrade writes.
- Resolve product-upstream conflicts in favor of local Web security hardening: preserve non-loopback auth, SSRF/CSRF protections, log privacy, proxy/failover atomicity, and related Web-first safety constraints; defer upstream items that cannot be made compatible.
- For any remaining implementation preference decisions, use the recommended Web-first, security-preserving option and document the choice rather than pausing, unless the issue is a true blocker.
- Exclude `origin` Dependabot branches and `Laliet/CC-Switch-Web` by default.
- The deferred Codex routing/model catalog task is superseded by this sync and has been archived; its scope is included here.

## Acceptance Criteria

- [x] Version metadata is updated to `3.16.5`, and changelog/release notes identify the sync as Web-first fork alignment with product upstream `v3.16.5`.
- [x] Relevant upstream functionality from `v3.16.3`, `v3.16.4`, and `v3.16.5` is either ported, explicitly adapted for Web mode, or documented as out of scope/deferred.
- [x] Existing Web-server auth, SSRF, CSRF, log privacy, and proxy/failover hardening remains intact.
- [x] The previously deferred Codex routing/model catalog scope is implemented or explicitly deferred within this task's final scope.
- [x] Schema v11 migration, usage/pricing reads, and DB-too-new recovery behavior are verified for Web-first runtime constraints.
- [x] Functional provider/preset catalog deltas are ported where they affect model availability, defaults, routing, or usage/pricing; nonfunctional catalog churn is documented as deferred or out of scope.
- [x] Upstream session category/group management works as a complete Web-adapted session organization feature, not only as a storage compatibility shim.
- [x] Frontend typecheck/build and Rust Web-server checks pass for the touched areas.
- [x] Rust check/tests, DB migration tests, session organization smoke coverage, and proxy/provider key-path checks pass, or any unable-to-run gate is documented with the blocker.
- [x] The full existing test suite is run where practical, and any skipped/blocked suite is documented.
- [x] The Web service starts successfully and is exercised with an end-to-end manual verification pass covering the synced user-visible flows.
- [x] Schema v11 migration creates a local backup before changing existing DBs; migration failure preserves the original DB, blocks continued startup, and surfaces clear recovery guidance.
- [x] DB recovery avoids automatic downgrade writes that could corrupt data written by a newer product version.
- [x] Desktop updater/release packaging/platform-operation changes are excluded unless they are necessary for Web service startup, version display, DB recovery, or security, and any excluded upstream items are documented.
- [x] Any conflict between product-upstream behavior and local Web security hardening is resolved in favor of the hardening, or the incompatible upstream item is explicitly deferred.

## Definition of Done

- Tests added or updated where behavior changes.
- Lint/typecheck/build/test gates run and pass, including full-suite and Web-service end-to-end verification where practical; any unable-to-run gate is documented.
- PRD decisions and research notes are current.
- Specs are reviewed for updates after implementation.
- Work is committed in coherent batches before task wrap-up.

## Out of Scope

- Dependabot branch updates unless explicitly pulled into this task later.
- `Laliet/CC-Switch-Web` sync unless a specific Web-only fix is identified and accepted.
- Full upstream parity for sponsor copy, cosmetic release banners, icon-size-only optimizations, and release packaging/CI changes unless they become necessary dependencies of core product behavior.
- Replacing the Web-first deployment model with desktop-only behavior.

## Research References

- [`research/upstream-v3.16.2-to-v3.16.5.md`](research/upstream-v3.16.2-to-v3.16.5.md) - confirmed tags, major upstream themes, and local constraints.

## Technical Notes

- Current branch during planning: `fix/web-audit-phase1-2`.
- Integration base decision: build on the current `fix/web-audit-phase1-2` branch to preserve the recent Web audit/security hardening.
- Product upstream tag range: `v3.16.2..v3.16.5`.
- Full diff stat/name-status still needs a robust clone or commit-by-commit inspection before implementation because partial diff fetches hit network errors.
- Superseded task archived without auto-commit: `.trellis/tasks/archive/2026-07/06-10-port-codex-chat-routing-model-catalog-stack`.

## Implementation Update (2026-07-07)

- Updated version metadata to `3.16.5` in `package.json`, `src-tauri/Cargo.toml`, `Cargo.lock`, and `tauri.conf.json`.
- Added fork-specific release notes and changelog entry for the Web-first `v3.16.5` sync.
- Ported schema v11 usage/pricing dimensions, pre-migration backup abort behavior, and DB-too-new startup recovery surface.
- Ported Web-adapted session organization/grouping UI and session usage accounting paths, including smoke coverage for startup session sync idempotency.
- Ported Codex native Responses/catalog support enough for `/v1/models`, generated model metadata, and native Responses safety gates. Full upstream Codex Chat Completions conversion remains explicitly deferred.
- Ported proxy content-encoding support for gzip/deflate/br/zstd through the shared Web/desktop proxy module wiring, including compressed request/error-body decoding where needed.
- Preserved local Web security hardening. Desktop updater/release packaging/Windows ARM/Linux desktop operations and sponsor/referral/marketing changes remain excluded unless runtime-critical.

## Verification Log (2026-07-07)

- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo check --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server`
- `cargo test --manifest-path src-tauri/Cargo.toml --lib` — 1472 passed, 2 ignored.
- Focused Rust tests: content encoding, native Responses/catalog, native gateway web-search rejection, schema v10→v11 migration, pricing schema columns, model pricing matching, and model pricing seed repair/insert.
- `cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server -- dual_runtime_parity::` — 3 passed.
- `pnpm typecheck`
- `pnpm test:unit -- tests/components/assembleSettingsConfig.test.ts` — command expanded to the existing Vitest suite; 117 files and 602 tests passed.
- `pnpm check:web-routes` — `missing: 0`.
- `pnpm build:web`
- `RUSTUP_HOME=/home/orion/.rustup CARGO_HOME=/home/orion/.cargo pnpm smoke:web-server` — standalone Web server started and all probes passed.
- `pnpm format:check`
- `git diff --check`

## Verification Notes

- The first `pnpm smoke:web-server` run failed before product startup because isolated smoke `HOME` made rustup try to download toolchain `1.95` and the TLS handshake failed. Re-running with explicit `RUSTUP_HOME`/`CARGO_HOME` used the installed toolchain and passed.
- The smoke script originally expected the explicit session-usage sync probe to import exactly one row. Startup bootstrap now imports the seeded archived Codex session first, so the explicit probe is idempotent and may return `0`; downstream probes still strictly verify data source, summary, trends, request logs, and request detail.

## Decision (ADR-lite)

### Integration Base

**Context**: The current branch contains 13 local commits beyond `origin/main`, mainly Web hardening and task wrap-up. Re-basing the sync onto `origin/main` would make the upstream sync cleaner but would force the hardening work to be re-applied and re-verified.

**Decision**: Use the current `fix/web-audit-phase1-2` branch as the integration base.

**Consequences**: The sync must resolve upstream changes against the latest local Web security posture. This increases conflict-analysis work, but reduces the risk of losing hardening fixes.

### MVP Sync Depth

**Context**: Upstream `v3.16.2..v3.16.5` contains core product changes, provider/preset updates, UI polish, docs, icon optimization, release packaging, and sponsor/link churn. Full parity would take longer and can blur the Web-first fork's product boundary.

**Decision**: Sync core product deltas plus Web adaptation. Prioritize Codex native Responses/model catalog, unified session history, usage/pricing schema changes, and key proxy/provider correctness fixes. Defer purely cosmetic, sponsor, release-banner, icon-size-only, and nonessential packaging changes.

**Consequences**: The fork reaches the important product/runtime behavior without turning this task into a broad upstream mirror. Some upstream-visible polish may remain intentionally out of date until a later sweep.

### Deferred Codex Task

**Context**: The existing `06-10-port-codex-chat-routing-model-catalog-stack` task was a deferred `v3.16.2` sync item. Upstream `v3.16.3..v3.16.5` includes newer Codex native Responses/model catalog work that overlaps this task.

**Decision**: Merge that scope into this sync task and archive the old planning task as superseded.

**Consequences**: Codex routing/model catalog work is tracked in one place. The sync implementation must still decide exact port/adapt/defer boundaries for Codex changes during upstream analysis.

### DB Compatibility

**Context**: Upstream `v3.16.5` includes usage/pricing storage changes in schema v11 and a DB-too-new recovery flow. This Web-first fork already has local database-version guarding, but the recovery experience and migration behavior need to preserve server deployment constraints.

**Decision**: Include schema v11 usage/pricing storage changes and Web-adapted DB-too-new recovery in the MVP.

**Consequences**: The implementation must treat DB compatibility as part of core product sync, not as a follow-up. Migration and recovery behavior must be tested explicitly because a failed DB transition can block both local and self-hosted Web deployments.

### Provider/Preset Catalog Scope

**Context**: Upstream `v3.16.2..v3.16.5` includes many provider, preset, pricing, referral, link, search, sorting, and display changes. Some are product behavior because they determine model availability, defaults, routing, or accounting. Others are marketing or UI polish and do not change core operation.

**Decision**: Sync functional provider/preset catalog deltas that affect model availability, defaults, routing behavior, or usage/pricing correctness. Do not sync sponsor/referral/marketing copy or pure catalog-display polish unless they are required by core behavior.

**Consequences**: The implementation must inspect catalog changes by behavior, not by file name alone. This keeps model and accounting correctness current while avoiding broad nonfunctional churn.

### Session Organization Scope

**Context**: Upstream `v3.16.2..v3.16.5` includes unified session-history work and session category/group management. A minimal sync could stop at compatibility, but that would leave visible upstream session organization behavior incomplete.

**Decision**: Include upstream session category/group management as a complete feature, including UI, grouping operations, and relevant fixes, adapted for the Web-first fork.

**Consequences**: The implementation scope expands beyond storage migration into frontend behavior and user workflows. The port must preserve Web-first security and runtime constraints while keeping session history, grouping, and restore behavior coherent.

### Verification Gate

**Context**: This sync touches frontend workflows, Rust backend behavior, database schema migration, proxy/provider behavior, and Web-service runtime behavior. Code review and narrow checks would leave too much integration risk.

**Decision**: Require the heavy verification gate: frontend typecheck/build, Rust check/tests, DB migration tests, session organization smoke coverage, proxy/provider key-path checks, the full existing test suite where practical, and a started Web service with end-to-end manual verification.

**Consequences**: Completion may take longer and some gates may expose pre-existing fragility. Any gate that cannot be run must be documented with the blocker rather than silently skipped.

### Version Identity

**Context**: After the sync, the Web-first fork will contain selected product-upstream `v3.16.5` behavior while preserving fork-specific Web deployment behavior. Keeping version metadata at `3.16.2` would obscure the sync target; adding a fork suffix could affect existing version parsing and update logic.

**Decision**: Update product version metadata, including `package.json` and `src-tauri/Cargo.toml`, to `3.16.5`. Changelog/release notes must explicitly identify this as a Web-first fork sync to product upstream `v3.16.5`.

**Consequences**: The visible version aligns with the product-upstream target while the notes carry the fork-specific qualification. Implementation must update all relevant metadata consistently and avoid implying unqualified full upstream parity where behavior was intentionally adapted or deferred.

### Release-Operation Scope

**Context**: Upstream `v3.16.2..v3.16.5` includes desktop updater, release packaging, Windows ARM64, Linux GDK/Wayland, and similar platform-operation changes. These can be valuable upstream, but most are not part of the Web-first fork's runtime contract.

**Decision**: Exclude release-operation deltas by default. Only sync portions necessary for Web service startup, version display, DB recovery, or security.

**Consequences**: The sync remains focused on user-visible product behavior and Web runtime correctness. Excluded upstream release-operation items must be documented so future release engineering work can revisit them deliberately.

### Schema Migration Safety

**Context**: Schema v11 changes persisted usage/pricing shape. Existing Web-first deployments may have long-lived local databases, and a failed migration can prevent service startup or risk data loss.

**Decision**: Before schema v11 migration, create a local DB backup. If migration fails, preserve the original DB, block continued startup, and show or return clear recovery guidance. Do not attempt automatic downgrade writes.

**Consequences**: The migration path favors data preservation and operator clarity over best-effort startup. Implementation must verify both successful migration and failure handling paths.

### Security Conflict Resolution

**Context**: The integration base contains recent Web security hardening for remote/self-hosted deployments. Product-upstream changes may assume desktop-local trust boundaries and can conflict with non-loopback auth, SSRF/CSRF protections, log privacy, or proxy/failover atomicity.

**Decision**: Local Web security hardening takes priority over product-upstream behavior. Upstream functionality must be adapted to preserve Web-first safety constraints; incompatible upstream items are deferred rather than weakening the hardening.

**Consequences**: The sync may diverge from upstream implementation details where the Web runtime requires stronger constraints. Each such divergence must be intentional, testable where practical, and documented as an adaptation or deferral.

### Remaining Preference Delegation

**Context**: The major scope decisions have been resolved. Continuing to pause for low-risk preference questions would slow implementation without improving the plan.

**Decision**: For remaining non-blocking preference decisions, use the recommended Web-first, security-preserving option and document it. Ask only if implementation hits a true blocker that cannot be inferred safely.

**Consequences**: Phase 2 can proceed without further planning churn. The implementation still must surface hard blockers instead of guessing through data-loss, security, or compatibility risks.
