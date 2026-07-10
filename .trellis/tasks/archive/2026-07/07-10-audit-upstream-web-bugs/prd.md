# Audit and Fix Inherited Web Defects

## Goal

Identify, prioritize, and fix high-confidence product-upstream inherited bugs and Web-first adaptation defects that materially affect cc-switch-web reliability, correctness, or remote-management usability, while avoiding duplicate work from the previous audit.

## What I Already Know

- The repository is a Web-first fork that follows `farion1231/cc-switch` for product capabilities and uses `Laliet/CC-Switch-Web` as a Web-direction reference.
- The current product baseline is version 3.16.5.
- The current branch is `fix/web-audit-phase1-2` and the worktree was clean before this task was created.
- A prior task named `07-08-upstream-bug-audit-fix` produced commits `5b37ea2e` (`fix(sync): port product upstream bug fixes`) and `716fdb9a` (`fix(usage): preserve last good data on transient failures`).
- The application has two execution surfaces: Tauri desktop and a standalone Axum Web server. Defects may be inherited directly or introduced when desktop assumptions cross the Web boundary.
- The Web server intentionally has no application-layer authentication; that deployment decision is already documented and is not itself a bug for this audit.

## Assumptions

- Only reproducible or code-proven defects should enter implementation scope; speculative cleanup and style refactors should not.
- Product-upstream issue reports, upstream fixes, local code review, and existing regression coverage are all valid evidence sources.
- For all remaining non-blocking scope and design decisions, use the recommended Web-first, data-preserving option without asking again. Escalate only a true blocker involving unknowable intent, unavoidable compatibility loss, or a new safety boundary.

## Open Questions

- None. Remaining non-blocking decisions are delegated to the recommended option.

## Requirements (Evolving)

- De-duplicate findings against prior upstream bug-audit work.
- Trace each selected defect to evidence: an upstream report/fix, a reproducible local behavior, or a high-confidence code-path proof.
- Prefer surgical fixes with regression tests.
- Preserve the documented Web-first deployment model and existing data compatibility.
- Verify both shared logic and Web-specific behavior whenever a fix crosses the desktop/Web boundary.
- Execute the work as severity-first, evidence-gated repair tranches rather than a bulk upstream merge or an audit-only report.
- The first tranche spans subsystems and includes every currently confirmed data-corruption defect, implemented as independent repair units:
  - Prevent Codex common-config extraction from carrying provider credentials, routing fields, model-catalog pointers, or MCP projections across providers.
  - Move Codex common-config merge/remove to backend `toml_edit` semantics so comments and ordering survive, with latest-operation and stale-baseline guards in the frontend.
  - Strip projected MCP sections from Codex provider snapshots during backfill and re-project enabled Codex MCP servers after live rewrites that can remove them.
  - Fail closed when single-server Codex MCP synchronization encounters unparseable TOML.
  - Reconcile `provider_endpoints` transactionally when an existing provider is updated.
  - Prevent stale workspace-file reads from overwriting or being saved into a newly selected file.
  - Prevent stale Daily Memory reads from overwriting, closing, or enabling save for a newly selected memory file.
  - Preserve supported symbolic-link-backed application configurations through explicit managed-target writes while rejecting dangling, non-file, or containment-escaping targets.
  - Constrain SQL backup restore to the declared CC Switch schema/data capability; a valid-looking header must not authorize arbitrary SQLite programs or external filesystem effects.
- Keep the existing SQL backup and WebDAV/S3 artifact format for this tranche. Enforce the restore contract with SQLite authorization, object/schema allowlists, integrity checks, and product-state validation.
- Exclude desktop-only packaging/updater/window/tray defects unless the same shared path can affect the standalone Web runtime.

## Acceptance Criteria (Evolving)

- [x] Every implemented fix has a documented trigger, root cause, and affected runtime surface.
- [x] Every implemented fix has regression coverage appropriate to the layer.
- [x] Previously fixed audit items are not reimplemented or regressed.
- [x] Frontend type-check, relevant Vitest suites, Rust checks/tests, and Web route/parity checks pass as applicable.
- [x] Research findings and explicit exclusions are persisted under this task.
- [x] First-tranche repairs are independently testable and can be reverted without reverting unrelated repair units.
- [x] Valid existing CC Switch SQL exports still round-trip, while valid-header payloads containing `ATTACH`, triggers/views/virtual tables, or unexpected objects are rejected without external side effects.
- [x] Stale workspace and Daily Memory read completions cannot alter, close, unlock, or save over a newer file selection.
- [x] Codex common-config operations preserve unrelated comments/order, reject stale asynchronous results, exclude provider/MCP artifacts, and leave invalid live TOML unchanged.
- [x] Existing-provider endpoint edits are visible after a fresh database read and do not retain removed URLs.
- [x] Supported managed-config symlinks remain symlinks after writes; restricted workspace writes cannot escape their allowed root through a symlink.

The approved Phase 3.4 logical commit split preserves independent rollback
boundaries for the five repair units.

## Definition of Done

- Tests are added or updated for each selected defect.
- Lint/format checks, TypeScript type-check, relevant frontend tests, and relevant Rust tests are green.
- Web-server parity and integration checks are run for affected routes.
- Domain vocabulary or durable project guidance is updated when the audit reveals a reusable distinction or prevention rule.
- Rollback risk is assessed for any persistence, provider-routing, import/export, or configuration migration change.

## Out of Scope (Evolving)

- Fixing all open issues in either upstream repository in one task.
- Desktop packaging, updater, tray, or window-management defects that cannot affect shared logic or the Web runtime.
- Pure style cleanup, broad rewrites, and unproven defensive changes.
- Reversing the accepted unauthenticated Web API deployment posture.
- Replacing SQL backups with a new structured snapshot format in this tranche.
- Project Profiles and other broad product-upstream feature additions.
- Fixing every medium/low audit candidate before the first integrity tranche is verified.

## Technical Notes

- Frontend: React 18, TypeScript, Vite, TanStack Query, Vitest/MSW.
- Backend: Rust/Tauri shared library plus Axum standalone Web server under `src-tauri/src/web_api/`.
- Existing route/parity tooling: `scripts/check-web-route-coverage.mjs`.
- Existing smoke tooling: `scripts/smoke-web-server.mjs`.
- Domain glossary: `CONTEXT.md`.
- Prior audit commits and archived task materials must be inspected before scope is finalized.

## Technical Approach

Implement the first tranche as focused repair units with separate regression gates:

1. **Frontend file identity**: generation-scope workspace and Daily Memory reads; bind saving to the file whose load completed.
2. **Constrained restore**: authorize only the SQL actions/objects emitted by CC Switch exports; validate schema, integrity, foreign keys, and product state before live replacement.
3. **Codex configuration integrity**: adapt the coherent upstream common-config/MCP fix series rather than cherry-picking; preserve Web API parity and local tests.
4. **Endpoint reconciliation**: update provider rows and endpoint rows in the same SQLite transaction.
5. **Managed symlink writes**: introduce explicit write policies; follow only supported managed-config links, while restricted/default paths reject or contain links.

Run targeted tests after each unit, then the complete TypeScript, Rust, Web-route/parity, locale, formatting, and relevant integration gates.

## Research References

- [`research/audit-candidate-matrix.md`](research/audit-candidate-matrix.md) — de-duplicated upstream and local-code evidence, risk classification, and recommended repair tranches.
- [`research/frontend-race-and-web-adaptation-audit.md`](research/frontend-race-and-web-adaptation-audit.md) — frontend race conditions, stale state, and Web-specific behavior mismatches found by code review.
- [`research/backend-state-integrity-audit.md`](research/backend-state-integrity-audit.md) — unsafe SQL restore, cross-store atomicity failures, runtime directory split, proxy rollback, sync locking, and persistence defects.
- [`research/symlink-write-policy.md`](research/symlink-write-policy.md) — comparison of symlink-following, rejection, and in-place write semantics with a context-aware recommendation.
- [`research/sql-import-safety-options.md`](research/sql-import-safety-options.md) — compatibility and security trade-offs for constrained SQL, disabled restore, and structured snapshot formats.

## Decision (ADR-lite)

### Severity-First Repair Campaign

**Context**: The audit already has multiple high-confidence defects, while blindly porting every upstream change would mix unrelated behavior and an audit-only phase would delay fixes for proven data-integrity risks.

**Decision**: Use severity-first, evidence-gated repair tranches. Each tranche must contain reproducible or code-proven defects, focused regression tests, and runtime-surface verification.

**Consequences**: Critical data-loss and configuration-integrity issues are addressed first. Lower-impact parity and UX defects remain documented for later tranches instead of being silently dropped.

### Cross-Module First Tranche

**Context**: Local code review found file-identity races capable of writing one file's content into another, alongside the already confirmed Codex/configuration-integrity defects.

**Decision**: Include all confirmed data-corruption defects in the first tranche even when they cross frontend and backend modules. Keep each repair in a focused implementation/test unit rather than coupling their code paths.

**Consequences**: The tranche has a broader file set, but severity is not subordinated to module cohesion. Review and rollback remain manageable because each defect has its own trigger, regression test, and commit boundary.

### Constrained SQL Restore

**Context**: Existing SQL backup files are both a user-visible import/export format and the WebDAV/S3 database artifact, but the current header-only check treats the remainder as an unrestricted SQLite program.

**Decision**: Preserve the SQL format while enforcing a narrow restore grammar through SQLite authorization, known-object/schema validation, integrity checks, and identical enforcement across manual and cloud restore paths.

**Consequences**: Existing backups remain compatible. Schema changes must update the restore allowlist/tests, and malformed or hand-edited SQL outside the CC Switch export contract will be rejected.

### Context-Aware Symbolic-Link Writes

**Context**: Atomic rename over a final symlink breaks dotfile-managed configurations, while globally following symlinks would let restricted workspace paths escape their intended boundary.

**Decision**: Use explicit write policies. Managed application-configuration writes may resolve a valid file symlink and atomically replace its target; restricted/default writes must reject or containment-check final symlinks.

**Consequences**: Supported dotfiles/NixOS links survive configuration updates without sacrificing complete-file atomicity. Call sites must identify their path role, and dangling/directory/escaping links fail clearly.

### Preference Delegation

**Context**: The user selected the recommended repair strategy and requested that all later questions use the recommended option.

**Decision**: Resolve non-blocking decisions using the recommended Web-first, data-preserving option and document them. Ask only for a true blocker that cannot be safely inferred.

**Consequences**: Planning and implementation continue without repeated preference prompts while preserving escalation for material authority or compatibility changes.

## Verification Log — 2026-07-10

- Targeted frontend regressions: PASS — 37/37 tests.
- Full Vitest suite: PASS — 118 files, 618 tests.
- MCP integration: PASS — 25/25 tests.
- Import/export integration: PASS — 32/32 tests with `--test-threads=1`.
  Serial execution is required because this integration harness intentionally
  shares one test HOME/global filesystem mutex; a parallel invocation can race
  through the shared harness and does not indicate a production restore race.
- Full Rust library tests: PASS — 1507 passed, 2 ignored. The first full run
  exposed a test-only global SQL-restore hook race; binding the hook to its
  target `Database` instance made the complete parallel library suite pass.
- Default Rust `cargo check`: PASS.
- Web-server feature/example `cargo check`: PASS with only pre-existing
  dead-code warnings.
- Clippy with `-D warnings`: PASS.
- Desktop/Web dual-runtime parity regressions: PASS — 3/3 tests.
- TypeScript `pnpm typecheck`: PASS.
- Web route coverage `pnpm check:web-routes`: PASS — 267 commands, missing 0.
- Locale parity `pnpm check:locales`: PASS — 2357 keys in each locale.
- Prettier `pnpm format:check`: PASS.
- Rust `cargo fmt --check`: PASS.
- Web production build: PASS.
- Web smoke: PASS — all probes. The successful rerun used
  `RUSTUP_HOME=/home/orion/.rustup CARGO_HOME=/home/orion/.cargo pnpm smoke:web-server`.
  The first attempt was stopped because the smoke script's isolated HOME made
  rustup try to download a toolchain, not because an application probe failed.
- `git diff --check`: PASS; no smoke-generated artifacts were found.
- Independent final reviews of SQL restore, Codex/MCP integrity, and managed
  symbolic-link writes found no remaining substantive implementation defect.

## Upstream Applicability and Residuals

- Upstream `6d2ee247` / #5131 was reviewed and intentionally not ported
  directly because this fork lacks unified Codex session history. Only its MCP
  self-healing principle was adapted through the existing complete Codex MCP
  projection/reconciliation path.
- Windows delete-before-rename replacement safety remains a separate defect.
- Managed symbolic-link deletion ownership semantics remain separate from write
  semantics.
- Workspace symbolic-link policy has helper-level regressions and correct
  Desktop/Web call sites, but no direct handler-level symbolic-link integration
  test yet.
- The implemented common-configuration boundary prevents provider credential
  and routing bleed, but this task does not claim that every symptom reported in
  upstream #5174 has been reproduced through the exact original save/switch path.
