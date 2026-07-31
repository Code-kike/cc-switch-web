# S8 report — version 3.18.0, changelog and full gate

Final batch of the v3.16.5→v3.18.0 sync. Implemented by `trellis-implement`,
verified by `trellis-check` with zero defects (three informational
observations left as-is).

## Delivered

- **Version bump 3.16.5 → 3.18.0** in the four SSOT files: `package.json`,
  `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` (cc-switch entry only),
  `src-tauri/tauri.conf.json`. No other version constants exist (Rust/FE derive
  from `CARGO_PKG_VERSION` / `GET /api/health`).
- **Changelog**: new `## [3.18.0] - 2026-07-31` entry in `CHANGELOG.md` plus
  fork-authored `docs/release-notes/v3.18.0-en.md` (per-theme S1–S7 summary,
  fork-side changes, Web adaptations, explicit exclusions incl. no-auth /
  no-updater by design, v13→v16 upgrade notes). No upstream marketing prose.
- **Integration triage**: `test:integration` had been deferred to S8 by every
  batch; 7 stale expectations were root-caused against server logs and fixed
  test-side only (UUID fixture stems per df3e07ed, stable-key probes per
  c9ac6efd, resolvable balance host per 2df2212c contract, baseline
  auto-backup clearing, longer v14–v16 restore waits, S5 hint copy). The check
  pass traced every fix to a named intentional behavior — zero product edits.
- **Smoke infra**: HOME-isolation kept; `RUSTUP_HOME`/`CARGO_HOME` pinned to
  the real home (prevents toolchain re-download + startup timeout); permanent
  `log-frontend-error-persists` probe added.

## Full gate matrix (final state, re-run by check)

cargo fmt/clippy desktop + web, `cargo test --lib` 1732 passed, web example
suites 36 passed, tsc, format:check, vitest 741 tests, `check:web-routes` 277
commands / 0 missing, manifest up to date, `check:locales` parity,
`pnpm build:web`, `test:integration` 47 passed + 3 documented pre-existing
env-flakes (ProviderList empty-state import, SkillsPage GitHub 404 ×2),
`pnpm smoke:web-server` EXIT 0 with 124 probes incl. fresh-DB v16 migration,
0600 `frontend.log` persistence, clean shutdown.

## Spec updates (Phase 3.3)

`.trellis/spec/frontend/quality-guidelines.md`: repricing-consistency rules
added to the pricing-lookup scenario; RUSTUP/CARGO_HOME pin added to the smoke
scenario; integration-deferral triage rule added to the upstream-sync
scenario; two new scenarios — "Serialized Session-Usage Sync and Destructive
Codex Usage Rebuild" (S6) and "Shared Logging Module and Frontend Error-Log
Persistence" (S7).

## Post-merge follow-ups (outside this task's commits)

Per PRD 决策6: after PR acceptance into `main` — back up the DB (schema v16
migration will rebuild Codex usage), rebuild, restart the local systemd web
service, and smoke it. Next-sync backlog (9 unreleased upstream commits +
Codex Chat bridge cluster) is recorded in the PRD and release notes.
