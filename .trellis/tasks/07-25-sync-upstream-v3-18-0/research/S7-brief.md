# S7 brief — Frontend error-log persistence and misc fixes

Branch: `sync/upstream-v3.18.0`

## Goal

Port the v3.17/v3.18 logging overhaul (frontend error capture to disk with
structured redaction + backend log persistence/secret redaction) and the
remaining misc fixes into the web-first fork as one reviewed batch. The
frontend error log must persist to server-side disk in Web mode through the
route SSOT, and every backend redaction path must compile in both runtimes.

## Authoritative dependency order (topological within v3.16.5..v3.18.0)

1. `7e73a1ff` — i18n `proxyReasonAnthropicMessages` key
2. `9ca1a41f` — normalize function parameters for strict OpenAI providers
3. `6d316c0b` — streamed tool call identity/order
4. `f6e37ed9` — CI matrix + platform-gated test repairs (裁剪/trim)
5. `edea624a` — preserve deleted default skill repositories
6. `613fef70` — codex-chat reasoning forward
7. `08710d51` — default Codex tool parameters to object schema
8. `22d2872c` — persist diagnostics across restarts and redact secrets
9. `62747058` — capture frontend errors to disk with structured redaction
10. `2bfecead` — Kimi K3 built-in pricing row
11. `f2045822` — Kimi K3 open-platform presets
12. `e356fc6e` — OpenClaw preset list-price repricing
13. `6fddcaa9` — bare `k3` pricing alias
14. `c4795e98` — backfill parser-required Codex catalog fields

## Recon outcome per commit

- `7e73a1ff` applies directly: `useProviderActions.ts:198` already references
  the key with only a Chinese defaultValue; add en/ja/zh copy, zh-TW stays
  deleted.
- `9ca1a41f`, `6d316c0b`, `613fef70`, `08710d51` are confined to
  `transform_codex_chat.rs`, `streaming_codex_chat.rs`, and
  `transform_codex_anthropic.rs` — none exist in this fork (S2 blocker
  cluster: the Codex Chat/Anthropic bridge was never ported). EXCLUDE as
  blocked; the fork's own Claude-side `clean_schema` already carries the
  root-object injection from the S2 `ded0b63a` port.
- `f6e37ed9` is trimmed:
  - ci.yml Windows/macOS matrix: EXCLUDE (fork CI is Linux-first; PRD trims
    release/CI churn).
  - `codex_history_migration.rs`: N/A (file absent).
  - `commands/misc.rs` anchored-upgrade tests and most POSIX-helper
    attributes: N/A (fork's misc.rs is a trimmed 1039-line version without
    `anchored_command_from_paths`).
  - `commands/settings.rs` `install_update_and_restart`: N/A (absent).
  - `codex_state_db.rs` TOML literal-string test fix: PORT.
  - `settings.rs` `use std::io::Write` into the cfg(unix) block: PORT.
  - `services/skill.rs` get_home_dir routing: CONVERGENT (fork already routes
    all five sites); PORT the regression test
    `get_app_skills_dir_honors_test_home_override` (serial_test available).
- `edea624a` applies cleanly: fork has the old supplement-missing
  `init_default_skill_repos`, `get_bool_flag`, `set_setting`, and
  `Database::memory()`; SkillStore has exactly 4 default repos matching the
  upstream test.
- `22d2872c` is the backend meat; fork call sites verified present:
  old `redact_url_for_log` in lib.rs, raw deeplink URL logs, model_fetch
  endpoint logs, forwarder request URL/body logs, hyper_client uri log,
  CacheTrace endpoint, response_processor body/SSE/format_headers, mcp/codex
  extended-field value logs, webdav redact_url. The fork's handlers.rs has
  only the single Claude parse-failure site (bridge-era C7 diagnostics
  machinery absent) and already truncates via `compact_error_message`;
  upstream's stricter metadata-only form is adopted there.
  `summarize_upstream_body` is byte-identical upstream after this commit —
  keep the fork's extra regression test.
- `62747058` needs the dual-runtime adaptation (PRD hard requirement):
  upstream writes through `@tauri-apps/plugin-log`; the fork has no plugin-log
  and Web mode has no Tauri. Use one shared backend writer instead.
- `2bfecead`/`6fddcaa9` seed rows are absent from the fork seed; the
  `ensure_model_pricing_seeded` INSERT OR IGNORE mechanism exists, so no
  schema bump.
- `f2045822` presets apply to all four fork preset files;
  `tests/config/codexChatProviderPresets.test.ts` is absent (skip that hunk).
- `e356fc6e` intersects the fork-only L19 invariant test
  `tests/config/openclawPresetPricing.test.ts` (preset cost must equal the
  Rust seed for identical ids). The fork seeded `kimi-for-coding`
  (0.002/0.006) and `ling-2.5-1t` (0.001/0.004) FROM the old preset values;
  upstream reprices those presets to official list prices, so the fork seed
  rows must be repriced in the same batch (kimi-for-coding → 0.95/4.00/0.19,
  ling-2.5-1t → 0.56/2.24) to preserve the seed-as-SSOT invariant.
- `c4795e98` applies with one adaptation: the fork's loader chain is
  `load_codex_model_catalog_template` (no `_uncached` split); the
  `CodexCatalogModelSpec` fields match the upstream tests exactly and
  `gpt5_5_template.json` already carries `supports_reasoning_summaries`.

## Web-first adaptations

- New shared tauri-free module `src-tauri/src/logging.rs` holding the URL
  redaction API (`url_for_log`, `url_for_log_with_secrets`,
  `redact_url_for_log`, `redact_url_for_log_with_secrets`,
  `redact_url_origin_for_log`), the generic size-based log rotation helper,
  and the frontend error-log disk writer. lib.rs declares `mod logging;`
  and `examples/server.rs` gains the matching `#[path]` shim so shared code
  (forwarder, webdav, model_fetch, mcp, hyper_client, deeplink) can call
  `crate::logging::…` in both runtimes.
- Frontend error persistence: one new Tauri command `log_frontend_error`
  (message: String) plus exact Web parity
  `POST /api/system/log_frontend_error` in `web_api/handlers/system.rs`, a
  `web-commands.ts` entry, and manifest regeneration. Both runtimes call the
  same `logging::append_frontend_error`, which bounds input, appends to
  `<app_config_dir>/logs/frontend.log` (0600 on Unix, 5 MiB × 2 rotation),
  and mirrors to `log::error!(target: "frontend")`. No `@tauri-apps/plugin-log`
  npm dependency and no `log:default` capability are introduced.
- `src/lib/frontendLogger.ts` is upstream-verbatim except the writer, which
  uses `invoke` from `@/lib/api/adapter` and swallows all failures (no error
  loops in web/test environments).
- Upstream's `FrontendErrorBoundary` replaces the fork's console-only
  `ErrorBoundary` (item-14 component); `renderCrash*` locale keys are replaced
  by `frontendCrash*`/`reloadInterface` keys in en/ja/zh.
- Desktop-only 22d2872c pieces (tauri-plugin-log rotation KeepSome(4)/20 MiB,
  startup no-delete, dispatch-level runtime filter, early Info level, DB log
  config applied right after DB open, updater plugin registered after the
  logger) stay in lib.rs; the Web server keeps its existing env_logger + M8
  level machinery, gaining journald-safe redaction through the shared module.
- zh-TW stays deleted; docs/user-manual FAQ hunks are N/A (directory absent).

## Validation

- Focused Rust tests: logging redaction unit tests, crash-log rotation,
  frontend-log writer, format_headers allowlist, mcp http_headers test,
  skills init-flag DB tests, skill test-home override, codex catalog backfill
  tests, codex_state_db literal string.
- Focused Vitest: frontendLogger suite (adapter mock), FrontendErrorBoundary,
  openclawPresetPricing, locale checks.
- Gates: `cargo fmt --check`; desktop clippy `-D warnings`; web
  check/clippy `--example server`; `cargo test --lib` unfiltered; web example
  tests `web_api:: dual_runtime_parity:: web_proxy_lifecycle::`;
  `pnpm format:check`; `tsc --noEmit`; `pnpm check:web-routes`;
  `check:locales`; focused vitest for touched areas.
