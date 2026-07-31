# S6 report — Codex usage rebuild and session correctness

Batch S6 ports the upstream v3.17/v3.18 Codex session-usage correction cluster
(`e606adfa eb105eae a10b569a df3e07ed c9ac6efd eff1e0cc`; `7a7d41c8` excluded as
already present) into the web-first fork. Implemented by `trellis-implement`,
verified by `trellis-check` with zero findings.

## Outcome per upstream commit

- `e606adfa` — renamed Codex session titles: new shared `codex_state_db.rs`
  (state_5.sqlite resolver, fork lacks `codex_history_migration.rs`), wired in
  `lib.rs` and via `#[path]` shim in `examples/server.rs`.
- `eb105eae` — session sync serialized under one process-wide mutex with
  blocking-worker execution and missed-tick skipping across desktop
  startup/timer, web startup/timer, and manual commands in both runtimes; all
  per-insert refresh notifications removed so each sync pass emits exactly one.
  Upstream's startup cost backfill dropped (fork uses lazy query-time backfill).
- `a10b569a` — `has_suspected_codex_session_duplicate` probe added verbatim.
- `df3e07ed` — parent-rollout token-prefix alignment: explicit parent
  identification, strict prefix alignment, missing/ambiguous-parent deferral
  without cursor advance, archived-file cursor inheritance; fork deltas
  preserved (`pricing_missing`, exact+LIKE `find_codex_pricing`, non-ASCII
  normalization, shared nanosecond `metadata_modified_nanos`).
- `c9ac6efd` — stable proxy usage keys: Claude keeps bare
  `session:{message_id}` convergence; others scoped `session:{app}:{provider}:{id}`;
  idempotent logging under a single DB guard, session_log-only in-place upgrade,
  deterministic SHA-256 collision fallback.
- `eff1e0cc` — schema v15→v16 resetting only Codex detail rows, rollups, and
  rollout cursors; `rebuild_codex_usage` maintenance action shared between the
  Tauri command and fork-only `POST /api/usage/rebuild-codex-usage` (backup
  hard-fail → savepoint reset → reimport → unconditional refresh notify),
  single-flight under the session-sync mutex; UsageDashboard maintenance
  accordion with destructive confirm; locales en/ja/zh only.

## Gates (all green)

`cargo fmt --check`; desktop clippy `-D warnings`; web check/clippy
(`--example server`); `cargo test --lib` 1718 passed; web example
`web_api:: dual_runtime_parity:: web_proxy_lifecycle::` 36 passed;
`tsc --noEmit`; `pnpm format:check`; full vitest 137 files / 724 tests;
`check:web-routes` 276 commands / 0 missing / manifest up to date;
`check:locales` parity.
