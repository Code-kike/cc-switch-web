# Debug Retrospective — Integrity Repair Tranche

## Bug Analysis: Constrained SQL Restore

### 1. Root Cause Category

- **Primary: E — Implicit Assumption**: a forgeable export header was treated
  as proof that the remaining SQL was trusted backup data, although SQLite
  accepts executable schema and external-file capabilities.
- **B — Cross-Layer Contract**: export, execution, migration, validation, and
  live replacement did not share an executable definition of allowed CC Switch
  state.
- **A/D — Missing Spec and Test Coverage Gap**: normal round-trip tests existed,
  but adversarial objects, DDL semantics, integrity failures, and replacement
  concurrency were not part of the restore contract.

### 2. Why Earlier Fixes Were Incomplete

1. Header validation plus a temporary database did not contain `ATTACH` side
   effects or prevent imported DDL from reaching the live schema.
2. An authorizer restricted action categories but did not prove that allowed
   table/index definitions had canonical product semantics.
3. Column, foreign-key, and index metadata checks still missed semantics such as
   `CHECK`, `AUTOINCREMENT`, expression indexes, `COLLATE`, and `ON CONFLICT`.
4. The decisive boundary was to copy accepted rows unconditionally into a
   schema created by trusted current code, so any remaining unmodeled DDL could
   not survive.
5. Separate locks for safety backup and replacement left a window where a
   concurrent write could be absent from both the backup and the final database.
6. A global test restore hook let an unrelated parallel `Database` consume it;
   binding the hook to the target instance fixed the test-only race.

### 3. Prevention Mechanisms

| Priority | Mechanism | Specific Action | Status |
|---|---|---|---|
| P0 | Architecture | Execute untrusted SQL only in a temporary DB, then copy known table rows into a trusted canonical schema | DONE |
| P0 | Runtime | Default-deny authorizer, known-object validation, `integrity_check`, and foreign-key checks | DONE |
| P0 | Concurrency | Hold one main DB lock across safety backup and replacement | DONE |
| P0 | Tests | Cover `ATTACH`, executable/unknown objects, DDL tampering, canonicalization, legacy schemas, and concurrent writes | DONE |
| P1 | Documentation | Treat restore compatibility and adversarial cases as schema-migration Definition of Done | DONE |

### 4. Systematic Expansion

- Manual, WebDAV, and S3 restore must share the same validator and replacement
  path; no transport-specific bypass is allowed.
- New tables, indexes, pragmas, or export behavior must update the restore
  allowlist, legacy compatibility, and adversarial tests together.
- Do not expand handwritten parsing to model every SQLite DDL semantic when
  canonical projection can structurally remove those semantics.

### 5. Knowledge Capture

- ADR 0002 records the restore capability boundary.
- `frontend/quality-guidelines.md` contains the executable seven-section
  scenario.
- Import/export integration runs serially because the test harness shares one
  HOME/global filesystem mutex; this is a harness constraint, not a production
  restore requirement.

## Bug Analysis: Codex Common Configuration and MCP Projection

### 1. Root Cause Category

- **Primary: B — Cross-Layer Contract**: provider snapshots, common snippets,
  the MCP database, live `config.toml`, and the React form modified overlapping
  state without explicit field ownership or commit ordering.
- **C — Change Propagation Failure**: live rewrites, backfill, toggle/delete,
  empty-database sync, and multi-application writes did not share one complete
  projection and recovery model.
- **E/D — Implicit Assumptions and Test Gap**: code assumed async completion
  order, treated parse failure as empty state, assumed DB commits implied live
  write success, and relied on current DB rows to find already-created orphans.

### 2. Why Earlier Fixes Were Incomplete

1. Frontend parse/deep-merge/stringify merged structures but destroyed TOML
   comments, whitespace, and ordering.
2. Moving mutation to backend `toml_edit` preserved syntax but did not stop an
   older async result from overwriting a newer toggle or manual edit.
3. Operation sequence alone could not distinguish preset changes with identical
   TOML; preset identity, baseline equality, and unmount invalidation were also
   required.
4. Ignoring an old response in the UI was insufficient because concurrent
   durable saves could still reach storage out of order; saves had to serialize.
5. Per-entry MCP updates could not clear live orphans when the DB set was empty.
6. Rolling back only the DB left earlier successful application writes changed.
   Re-syncing the rolled-back DB also missed failed creates whose row no longer
   existed, requiring directed removal/restoration for non-Codex apps.
7. First-error sync prevented later applications from self-healing; attempts
   must continue per application and aggregate failures.

### 3. Prevention Mechanisms

| Priority | Mechanism | Specific Action | Status |
|---|---|---|---|
| P0 | Ownership | Separate provider-scoped fields, user common configuration, MCP DB state, and live derived projections | DONE |
| P0 | Syntax preservation | Mutate Codex TOML only through backend `toml_edit` | DONE |
| P0 | Concurrency | Generation, baseline, preset identity, unmount invalidation, and serialized durable saves | DONE |
| P0 | Projection | Rebuild the complete Codex MCP table from DB state and clear empty/orphan state | DONE |
| P0 | Compensation | Capture previous row/affected apps; rollback DB and compensate every touched application | DONE |
| P0 | Error handling | Fail closed on invalid TOML and aggregate recovery failures | DONE |
| P1 | Documentation | Add ownership/projection/compensation checks to project specs and review guide | DONE |

### 4. Systematic Expansion

- Every derived live configuration needs a named source of truth and an
  explicit empty-source reconciliation case.
- Every database-plus-external-files mutation needs the model: capture previous
  state -> write DB -> project -> rollback DB -> directed compensation ->
  aggregate recovery errors.
- Provider extraction/backfill must exclude derived state at both entry points;
  filtering only the editor path allows deleted MCP or credentials to return on
  a later provider switch.
- Upstream `6d2ee247` is not directly applicable because this fork lacks unified
  Codex session history. Only its complete-projection self-healing principle was
  adapted; the bug fix must not silently expand product scope.

### 5. Knowledge Capture

- ADR 0004 records MCP authoritative-state and compensation semantics.
- `frontend/quality-guidelines.md` contains the executable seven-section
  common-config/MCP contract.
- `cross-layer-thinking-guide.md` now asks for identity, ownership, empty-source
  projection, and failed-create compensation explicitly.
