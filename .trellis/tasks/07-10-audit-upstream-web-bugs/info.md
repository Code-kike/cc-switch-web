# First Integrity Tranche — Technical Design

## Invariants

1. A stale asynchronous result must never mutate state owned by a newer file, login, search, or settings operation.
2. A restore artifact is data for known CC Switch state, not an arbitrary SQLite program.
3. Provider-scoped credentials, routing fields, catalogs, and MCP projections must not become shared Codex configuration.
4. A failed write or parse must leave the previous valid configuration intact.
5. Provider metadata and normalized endpoint rows must commit together.
6. Atomic replacement must not silently change the filesystem ownership model of a supported managed configuration target.

## Repair Unit A: Frontend File Identity

- Add monotonically increasing request identities to workspace and Daily Memory file loads.
- Clear/lock editor state when the selected file changes.
- Apply success, failure, and finalization only when the request is still current.
- Track the filename whose contents completed loading; allow save only when it equals the current selection.
- Test inverse-order completion and obsolete failure.

## Repair Unit B: Constrained SQL Restore

- Install a rusqlite authorizer on the temporary import connection before `execute_batch`.
- Allow only known CC Switch tables/indexes, inserts into known tables, transaction operations, and narrowly approved pragmas.
- Deny attachment, temp schema, triggers, views, virtual tables, unexpected functions/actions, and unexpected objects.
- After execution and migrations, require no unknown schema objects, run `integrity_check`, foreign-key checks, and existing product-state validation.
- Apply the same path to manual, WebDAV, and S3 restores.
- Test forged-header `ATTACH`, trigger/view/virtual-table payloads, unknown table/index, and a valid legacy/current round-trip.

## Repair Unit C: Codex Common Config and MCP Integrity

- Port/adapt upstream commits `473c2aaa`, `93f56198`, `8b1ce764`, and `88d5ffba` as a coherent behavioral change.
- Do not introduce the unified Codex session-history feature solely to apply `6d2ee247`; this fork lacks that prerequisite feature. Reuse only its applicable self-healing principle through the existing targeted Codex MCP projection path.
- Exclude MCP tables, top-level bearer token, catalog pointer, injected web-search sentinel, and routing fields from common extraction.
- Strip projected MCP sections from provider snapshots on live backfill.
- Fail closed on invalid TOML during single-server MCP sync.
- Re-project Codex MCP after live rewrites that can remove the projection.
- Use backend `toml_edit` merge/remove APIs from both desktop and Web adapters.
- Guard frontend async merge/remove with operation generation and config-baseline checks.

## Repair Unit D: Endpoint Reconciliation

- Within `save_provider`, replace endpoint rows from the incoming provider metadata in the same transaction for both insert and update.
- Preserve `added_at`; do not leave removed URLs behind.
- Add focused DAO tests covering edit, removal, and rollback on invalid endpoint persistence.
- Keep the separate `last_used` schema defect for a later migration unless it becomes a prerequisite.

## Repair Unit E: Managed Symlink Writes

- Keep restricted/default atomic writes from following final symlinks implicitly.
- Add an explicit managed-config write mode that resolves an existing non-dangling regular-file target and creates the temporary file in the resolved parent.
- Preserve permissions and atomically replace the resolved target.
- For workspace paths, reject final symlinks or verify the fully resolved target remains inside the allowed root.
- Test relative/absolute links, dangling links, directory targets, containment escape, and regular-file behavior.

## Verification Order

1. Focused unit/component tests for each repair.
2. `pnpm typecheck` and targeted Vitest integration tests.
3. Rust targeted tests, default `cargo check`, and Web-server feature check.
4. Full Rust library tests and relevant frontend suite.
5. `pnpm check:web-routes`, locale parity, format checks, and `git diff --check`.
