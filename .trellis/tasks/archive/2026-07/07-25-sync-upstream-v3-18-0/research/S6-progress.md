# S6 progress log — Codex usage rebuild and session correctness

Append-only checkpoint log. Format:
`<hash-or-area> DONE|ADAPTED|EXCLUDED|GATE — <note>`

- START — resumed after the checked/spec-updated S5 batch and committed S5 as
  `8e28f560` plus `d2875f5f`. Loaded Phase 2.1, `trellis-before-dev`, the usage,
  SQL restore, upstream-sync, Web route, startup, smoke, and shared thinking
  contracts.
- RECON DONE — mapped all seven S6 commits in dependency order. Confirmed the
  30-day quota fix is already fully present; recorded dual-runtime sync,
  schema-v16 reset, Web maintenance route, retained-locale, `pricing_missing`,
  stable-ID, and parent-rollout alignment adaptations in `S6-brief.md`.
- eb105eae ADAPTED — desktop half was already in the working tree from the prior
  session (verified against the upstream diff); completed the web-runtime half:
  `examples/server.rs` startup/timer worker and the Web manual sync handler now
  take `session_sync_mutex`, run `sync_all_unlocked` on a blocking worker, and
  skip missed ticks. Web cargo check PASS.
- e606adfa ADAPTED — added `codex_state_db.rs` (verbatim upstream shared resolver;
  the fork lacks `codex_history_migration.rs`, so title lookup is the only
  consumer), 3-way-applied the session-provider title lookup (session_index.jsonl
  + state_5.sqlite with busy timeout and SQL-side first-message filter), wired
  `mod codex_state_db` in lib.rs plus the `#[path]` shim in examples/server.rs.
  Desktop + web check PASS; focused provider/state-db tests PASS.
- a10b569a DONE — added `has_suspected_codex_session_duplicate` to usage_stats.rs verbatim (predicates reuse data_source_expr/COALESCE shape); wired by df3e07ed next.
- df3e07ed ADAPTED — took the upstream post-state Codex importer wholesale
  (parent-rollout token-prefix alignment, pending/deferral cache, archived-file
  cursor inheritance, replay caches, reset machinery), then re-applied the four
  fork deltas: pricing_model + pricing_missing insert columns, exact+LIKE
  `find_codex_pricing` (kept over upstream's `find_model_pricing` call), the
  non-ASCII normalize comment + regression test, and the fork's shared-helper
  layout — `get_sync_state`/`update_sync_state` made pub(crate) in
  session_usage.rs plus a shared `metadata_modified_nanos`; opencode's local
  triplet deduped to the shared ones and its per-insert notify removed to
  complete eb105eae coalescing. 29 codex importer tests + full session_usage
  namespace PASS.
- c9ac6efd ADAPTED — 3-way-applied parser.rs (envelope-id extraction via
  `response_id`, scoped `dedup_request_id(scope)`) and logger.rs (single-guard
  load-existing-semantic → idempotent replay return → session_log upgrade via
  INSERT OR REPLACE vs INSERT OR IGNORE → SHA-256 collision fallback). Resolved
  two conflicts: kept fork imports (no `is_placeholder_pricing_model` — fork
  uses its pricing_missing path; kept SystemTime) + upstream's new
  OptionalExtension/Sha256; merged the fork's L29 cross-source dedup rationale
  into the new conditional-verb comment. Updated all three fork call sites of
  `dedup_request_id` (handlers.rs, response_processor.rs, transform.rs test)
  with the claude-unscoped/others-scoped rule. proxy::usage tests 38 PASS.
- eff1e0cc ADAPTED — schema v15→v16 (fork chain: profiles v14, grokbuild v15,
  Codex usage reset v16) calling `reset_codex_usage_on_conn` inside the existing
  migration savepoint + upstream migration regression test; SCHEMA_VERSION 16;
  desktop `rebuild_codex_usage` command (backup hard-fail → reset → reimport
  under the session-sync mutex) + registration + notify tests;
  `finish_codex_rebuild` moved into the shared tauri-free
  `services/session_usage.rs` (commands/ is desktop-only in the web build) so
  the fork-only Axum handler `POST /api/usage/rebuild-codex-usage` shares the
  exact sequence; `web-commands.ts` entry, manifest regenerated (276 commands,
  check green), route coverage missing 0; FE `usageApi.rebuildCodexUsage`,
  UsageDashboard maintenance accordion + destructive confirm + result toast,
  locales en/ja/zh only (zh-TW stays deleted). Changelog left to S8 like S1–S5.
- 7a7d41c8 EXCLUDED — already fully present in the fork (recon-confirmed); no
  duplication.
- GATE — `cargo fmt --check` PASS.
- GATE — desktop `cargo clippy --all-targets -- -D warnings` PASS (48.94s).
- GATE — web clippy (`-D warnings -A dead_code -A clippy::duplicate_mod`) PASS
  (31.90s); web `cargo check --example server` PASS.
- GATE — unfiltered `cargo test --lib` PASS: 1718 passed / 0 failed / 2 ignored
  (includes new importer, suspected-duplicate, v15→v16 migration, rebuild
  notify, coalescing, mutex, state-db title tests).
- GATE — web example `cargo test -- web_api:: dual_runtime_parity::
  web_proxy_lifecycle::` PASS: 36 passed / 0 failed.
- GATE — `npx tsc --noEmit` PASS; `npm run format:check` PASS; full
  `npx vitest run` PASS: 137 files / 724 tests.
- GATE — `npm run check:web-routes` PASS: commands 276, missing 0,
  methodMismatch 0, parityFallback 0; `gen-command-manifest.sh --check` PASS
  (276 commands); `npm run check:locales` PASS (en/ja/zh parity).
- GATE — final hygiene: `git diff --check` clean; zero conflict markers;
  `git ls-files -u` empty after re-adding the two `git apply -3` conflict files
  (logger.rs, UsageDashboard.tsx); no commit made.
