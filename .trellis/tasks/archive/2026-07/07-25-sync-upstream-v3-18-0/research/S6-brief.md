# S6 brief — Codex usage rebuild and session correctness

Branch: `sync/upstream-v3.18.0`

## Goal

Port the v3.17/v3.18 Codex session-usage correction cluster into the web-first
fork as one reviewed batch. The batch must eliminate future proxy/session
double-counting, rebuild affected Codex session rows safely on schema upgrade,
and preserve the fork's cache-normalized totals, `pricing_missing`, Web runtime,
and database-recovery contracts.

## Authoritative dependency order

1. `7a7d41c8` — Codex free-plan 30-day quota display
2. `e606adfa` — renamed Codex session titles from Codex state metadata
3. `eb105eae` — serialize session sync and coalesce refresh notifications
4. `a10b569a` — suspected Codex session-duplicate probe
5. `df3e07ed` — parent-rollout token-prefix fork alignment
6. `c9ac6efd` — stable proxy usage keys and idempotent raw-response logging
7. `eff1e0cc` — schema-v16 Codex usage reset plus maintenance action

## Recon outcome

- `7a7d41c8` is already represented in the fork: `TIER_THIRTY_DAY`, the Codex
  window mapper, tray month grouping, frontend tier whitelist, retained locale
  strings, and Rust/frontend regressions are present. Do not duplicate it.
- `e606adfa` must be adapted because this fork does not carry upstream's
  `codex_history_migration.rs`. Add the shared state-DB resolver for session
  titles only, and wire it into both desktop and standalone Web compilation.
- `eb105eae` must cover all three entry paths in both runtimes: desktop startup/
  timer, Web startup/timer, and manual Tauri/Web commands. Use one process-wide
  mutex, blocking-worker execution, missed-tick skipping, and one notification
  per completed sync pass.
- `df3e07ed` replaces the heuristic takeover-boundary importer with explicit
  parent identification plus strict token-signature prefix alignment. Missing or
  ambiguous parents defer without advancing cursors. Preserve nanosecond mtime
  cursors, archived-file cursor inheritance, non-ASCII model safety, and
  `pricing_missing` on imported rows.
- `c9ac6efd` keeps Claude's bare `session:{message_id}` convergence but scopes
  non-Claude envelope IDs by app and provider. Writes must be idempotent under a
  single DB guard; only `session_log` primary rows may be upgraded in place, and
  semantic collisions use deterministic SHA-256 fallback IDs.
- `eff1e0cc` advances schema v15→v16 and resets only `codex_session` detail rows,
  `_codex_session` rollups, and Codex rollout cursors. Database backup must
  hard-fail before a manual reset. Reset/import is single-flight and emits a
  refresh even when re-import is empty or fails after the reset.

## Web-first adaptations

- Add an exact Web route and `web-commands.ts` entry for
  `rebuild_codex_usage`; the destructive mutation remains unauthenticated by
  product design and inherits the same-origin intent guard.
- Share the rebuild implementation between Tauri and Axum so backup, reset,
  locking, error behavior, and notification semantics cannot drift.
- Keep the standalone server's startup/timer session sync in parity with desktop
  and move its blocking filesystem/SQLite work off the async executor.
- Keep `zh-TW.json` deleted; add copy only to retained `en`, `ja`, and `zh`.
- Do not import unrelated upstream changelog/release prose verbatim. Record the
  fork-specific S6 outcome in this task report and add only appropriate
  unreleased notes to the fork changelog.

## Validation

- Focused importer tests for parent/fork resolution, missing-parent deferral,
  replay prefix alignment, archived cursors, reset scope, suspected duplicates,
  stable IDs, collision fallback, notification coalescing, and renamed titles.
- Desktop and Web cargo checks/clippy, `web_api::`, `dual_runtime_parity::`, and
  `web_proxy_lifecycle::` namespaces.
- TypeScript, focused usage/session/quota tests, Web route coverage, retained
  locale parity, full Rust library tests, full Vitest, and Web build/smoke where
  session totals or maintenance routing changed.
