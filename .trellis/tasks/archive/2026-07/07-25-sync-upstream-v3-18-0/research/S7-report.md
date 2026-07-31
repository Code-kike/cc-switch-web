# S7 report — logging overhaul + misc fixes (upstream v3.17/v3.18)

Branch `sync/upstream-v3.18.0`. All work is in the working tree, **not committed**
(per brief; the two old-ErrorBoundary deletions are staged as a side effect of
`git rm`). Checkpoint log: `S7-progress.md`. End state: zero conflict markers,
`git ls-files -u` empty, `git diff --check` clean.

## Per-commit outcome (topological order)

| # | Hash | Status | Notes |
|---|---|---|---|
| 1 | 7e73a1ff | **ported** | `proxyReasonAnthropicMessages` added to en/ja/zh; the key was already referenced by `useProviderActions.ts` with only a Chinese defaultValue. zh-TW stays deleted. |
| 2 | 9ca1a41f | **excluded (blocked)** | Confined to `transform_codex_chat.rs`, which the fork does not have (Codex Chat bridge = S2 blocker cluster 99e11e08/a078b4b2). |
| 3 | 6d316c0b | **excluded (blocked)** | `streaming_codex_chat.rs` absent (same cluster). |
| 4 | f6e37ed9 | **ported (trimmed)** | Ported: `codex_state_db.rs` TOML literal-string path fix, `settings.rs` `io::Write` moved into the cfg(unix) block, and the `get_app_skills_dir_honors_test_home_override` regression (the fork had already routed all five skill.rs home lookups through `get_home_dir` — convergent). Excluded/N-A: CI Windows/macOS matrix (Linux-first web fork, PRD trims CI churn), `codex_history_migration.rs` (absent), the six anchored-upgrade misc tests and POSIX-helper `dead_code` attrs (fork's trimmed misc.rs has no `anchored_command_from_paths`), `install_update_and_restart` (absent). |
| 5 | edea624a | **ported** | `init_default_skill_repos` is now one-shot via the `default_skill_repos_initialized` flag; deleted default repos stay deleted; pre-existing selections are grandfathered. Both upstream DB regressions added. |
| 6 | 613fef70 | **excluded (blocked)** | `transform_codex_chat.rs` absent (same cluster). |
| 7 | 08710d51 | **excluded (blocked)** | Both `transform_codex_chat.rs` and `transform_codex_anthropic.rs` absent. The fork's Claude-side `clean_schema` already carries the root-object injection from the S2 `ded0b63a` port; the Codex-bridge tool paths this commit hardens do not exist here. |
| 8 | 22d2872c | **ported (adapted)** | Backend logging overhaul; see below. |
| 9 | 62747058 | **ported (adapted)** | Frontend error capture with dual-runtime disk persistence; see below. |
| 10 | 2bfecead | **ported** | `kimi-k3` seed row (3.00/15.00/0.30); seed-only, no schema bump. |
| 11 | f2045822 | **ported (adapted)** | `kimi-k3` appended to the Hermes/OpenClaw/OpenCode Kimi presets. Fork base is `kimi-k2.6` (upstream had already moved to `kimi-k2.7-code` before v3.16.5; the fork's presets deliberately diverged). The fork's Codex presets have no Kimi entry (chat-bridge-dependent) → that hunk N/A; `tests/config/codexChatProviderPresets.test.ts` absent → skipped. |
| 12 | e356fc6e | **ported (adapted)** | OpenClaw list-price repricing applied to every fork-present entry: deepseek-v4-pro (+cacheRead), glm-5.1 ×2, qwen3.5-plus, kimi-for-coding → 0.95/4/0.19, MiniMax-M2.7 ×2 + ModelScope ZhipuAI/GLM-5.1 + SiliconFlow MiniMax ×2, KAT-Coder-Pro, Ling-2.5-1T, moonshotai/kimi-k2.5 → 0.6/3/0.1; fork's kimi-k2.6 entry gains seed-aligned cacheRead 0.16. `LongCat-Flash-Chat` left untouched — upstream repriced `LongCat-2.0`, a model the fork never adopted (`7a8b9562` was not in any S-batch). |
| 13 | 6fddcaa9 | **ported** | Bare `k3` alias seed row for the Kimi For Coding plan. |
| 14 | c4795e98 | **ported (adapted)** | `fill_template_fields_from_static` + `CODEX_CATALOG_PARSER_REQUIRED_FIELDS` whitelist, adapted onto the fork's un-split `load_codex_model_catalog_template` (no `_uncached` layer here). Both upstream tests ported; the fork's `gpt5_5_template.json` already carries `supports_reasoning_summaries: true`. |

## 22d2872c — backend logging adaptation detail

Upstream put the new redaction helpers at the crate root of a desktop-only lib.
This fork's shared code (proxy, webdav, model_fetch, mcp, deeplink) is compiled
into the standalone web binary through `#[path]` shims, so the helpers moved into
a **new shared tauri-free module `src-tauri/src/logging.rs`** (declared in
`lib.rs` and shimmed in `examples/server.rs`):

- `RedactedUrl` lazy wrapper, `url_for_log`, `url_for_log_with_secrets`,
  `redact_url_for_log(_with_secrets)`, `redact_url_origin_for_log`,
  known-secret exact-match redaction (min 8 chars), bare-userinfo stripping;
- a generic size-based rotation helper shared by crash.log and frontend.log;
- the frontend error-log writer (62747058's backend half).

Applied per upstream, adapted to fork call sites:

- **lib.rs (desktop)**: old `?[keys:...]` redactor removed; deeplink raw-URL
  debug log dropped; plugin-log now KeepSome(4) × 20 MiB, **no startup delete**
  (prior-run logs survive restarts); dispatch-layer `filter` +
  `runtime_log_level_allows` (+test); early conservative `Info` + startup
  banner + config-override replay; updater plugin registration moved after the
  logger; the persisted DB log level is applied **immediately after DB open**
  with an explicit fail-closed `Info` fallback (old late block removed).
- **panic_hook.rs**: crash.log bounded to 5 MiB × 2 archives; size-check +
  rotate + append run inside one mutex critical section.
- **forwarder.rs**: exact auth material (api_key/access_token, incl. resolved
  Copilot/Codex-OAuth/xAI tokens) collected as log-only secrets; the request
  info log now prints an origin-only target when no known secrets exist
  (credentials embedded in a base_url path can no longer reach the journal at
  Info) or the secret-redacted full URL when they do; the debug request-body
  log is metadata-only (`bytes` + `short_value_hash`); reqwest send errors are
  logged `without_url()`; the CacheTrace endpoint strips the Gemini `?key=`
  query. The fork's `summarize_upstream_body` is byte-identical to upstream's
  post-commit state; the fork's extra regression test was kept.
- **hyper_client.rs**: `send_request` takes a caller-sanitized `log_display`
  and masks the proxy URL; never derives log output from the raw URI.
- **handlers.rs**: the Claude upstream-parse-failure log is now
  metadata-only (`body_bytes=N`). The fork previously truncated to 180 chars
  (its own FIX-3 hardening); upstream's stricter no-content form was adopted
  per the 上游优先 rule. Upstream's other handler hunks (Codex chat/anthropic
  paths, `body_snippet`/`classify_body_for_diagnostics` C7 machinery) are N/A —
  that bridge-era code does not exist in the fork.
- **response_processor.rs**: non-streaming body log is bytes-only; passthrough
  SSE data moved to `trace!` with content omitted; `format_headers` is now an
  allowlist (content-type/encoding/length, retry-after, cf-ray, request-id
  variants, ratelimit families) with 160-char bounded values (+test).
- **mcp/codex.rs**: `headers` joined `http_headers` as an http/sse core field
  (auth values no longer flow through the generic field logger and are no
  longer emitted twice), extended/custom field logs print names only (+test).
- **webdav.rs / model_fetch.rs / deeplink**: delegate to the shared redaction
  helpers; model fetch redacts with the API key as a known secret.
- `docs/user-manual` FAQ hunks: N/A (directory absent in the fork).
- Web runtime: keeps its existing env_logger + M8 dynamic-level machinery;
  gains the same redaction via the shared module (journald output is now
  origin-only/secret-redacted on the same code paths).

## 62747058 — frontend error capture, dual-runtime adaptation

The PRD requires frontend error persistence to work in Web mode by writing to
server-side disk. Upstream writes through `@tauri-apps/plugin-log`, which does
not exist in a browser. Adaptation:

- **One shared backend sink**: `logging::append_frontend_error` bounds input
  (20k chars server-side; the FE already caps at 12k), appends timestamped
  entries to `<app_config_dir>/logs/frontend.log` (created 0600 on Unix,
  5 MiB × 2 rotation, single-mutex critical section), and mirrors to
  `log::error!(target: "frontend")` (desktop → cc-switch.log/stdout, web →
  stdout/journald). File persistence is intentionally independent of the
  dynamic log level so a white-screen crash always leaves a trace.
- **Desktop**: new Tauri command `log_frontend_error` (commands/misc.rs),
  registered in `generate_handler`.
- **Web**: `POST /api/system/log_frontend_error` in
  `web_api/handlers/system.rs` (Json body), inheriting the same-origin intent
  guard; `web-commands.ts` SSOT entry; `commands.manifest.json` regenerated
  (277 commands — note the manifest file is git-ignored, so it does not appear
  in the diff). Route coverage: missing 0 / methodMismatch 0 / parityFallback 0.
- **`src/lib/frontendLogger.ts`**: upstream-verbatim redaction pipeline (two
  layers: structured property-name serializer + universal text-layer regex
  pass; V8/WebKit stack rendering; oversized-JSON drop; bounded inputs). Only
  the writer differs: `invoke("log_frontend_error", { message })` through
  `@/lib/api/adapter`, all failures swallowed so logging can never cascade.
  The `@tauri-apps/plugin-log` npm dep and the `log:default` capability were
  **not** introduced.
- **`FrontendErrorBoundary`** (upstream component) replaces the fork's
  console-only `ErrorBoundary` (item-14): component + test deleted,
  `renderCrash*` locale keys replaced by `frontendCrash*`/`reloadInterface`.
  `main.tsx` installs the global error/unhandledrejection handlers at module
  load and reports the two previously console-only startup failures
  (`config_load_error_listener`, `get_init_error`).
- **i18n**: upstream's logConfig copy ("Application Diagnostic Logs") and the
  proxy `enableLogging` → "Record Request Usage" relabel applied to en/ja/zh;
  the fork-only `loadFailed`/`saveFailed` keys kept; zh-TW stays deleted.
- **Tests**: upstream's 327-line `frontendLogger.test.ts` ported with the mock
  retargeted from plugin-log to the adapter (18 tests), plus
  `FrontendErrorBoundary.test.tsx`; a new backend regression proves
  `append_frontend_error` persists to disk under an isolated test HOME with
  0600 permissions and passes in **both** the desktop lib suite and the web
  example suite.

## Fork-seed reconciliation forced by e356fc6e (non-port code)

The fork has an L19 invariant test (`openclawPresetPricing.test.ts`): any
OpenClaw preset model whose id also exists in the Rust pricing seed must carry
the seed's input/output price. Two fork-local seed rows had been derived from
the old preset values, so repricing the presets required repricing the seed in
the same batch:

- `kimi-for-coding` 0.002/0.006 → **0.95/4.00/0.19** (K2.7 Code list price,
  upstream's rationale);
- `ling-2.5-1t` 0.001/0.004 → **0.56/2.24** (official CNY 4/16 at ~7.14);
- `kimi-k2.5` output 2.50 → **3.00**, matching upstream v3.18.0's own seed and
  repair entry ("Kimi K2.5 官方 output 3.00" — that upstream seed correction had
  not been picked up by any earlier batch).

All three also received `repair_current_model_pricing` entries so existing
databases converge only when the row still equals the old built-in values
(user-modified prices untouched). `minimax-m2.7` seed already matched the new
preset values.

## Non-port code inventory

1. `src-tauri/src/logging.rs` as the shared home for upstream's lib.rs-root
   helpers + the frontend.log writer/rotation; `#[path]` shim in
   `examples/server.rs`.
2. The dual-runtime frontend-error sink: Tauri command, Axum route,
   `web-commands.ts` entry, manifest regen (the FE↔backend transport replaces
   upstream's plugin-log dependency).
3. Seed/repair reconciliation described above (kimi-for-coding, ling-2.5-1t,
   kimi-k2.5) plus the seed-aligned `cacheRead: 0.16` on the fork's kimi-k2.6
   OpenClaw preset entry.
4. The `append_frontend_error` disk-persistence regression test.
5. Test-mock adaptation in `frontendLogger.test.ts` (adapter instead of
   plugin-log; one test retitled and its options assertion replaced by a
   command-shape assertion).

No non-port code writes real `~/.claude`/`~/.codex`/`~/.config`; the new
frontend.log lives under `<app_config_dir>/logs/` (CC_SWITCH_DATA_DIR /
CC_SWITCH_TEST_HOME-aware), and tests use isolated temp HOMEs.

## Gate results

| Gate | Result |
|---|---|
| `cargo fmt --check` | PASS |
| desktop `cargo clippy --all-targets -- -D warnings` | PASS |
| web `cargo clippy --no-default-features --features web-server --example server -- -D warnings -A dead_code -A clippy::duplicate_mod` | PASS |
| web `cargo check --no-default-features --features web-server --example server` | PASS |
| `cargo test --lib` (UNFILTERED) | **1732 passed / 0 failed / 2 ignored** |
| web example `-- web_api:: dual_runtime_parity:: web_proxy_lifecycle::` | **36 passed / 0 failed** |
| `npx tsc --noEmit` | PASS |
| `npm run format:check` | PASS (new tests/ files also Prettier-clean) |
| `npm run check:web-routes` | commands **277**, missing **0**, methodMismatch **0**, parityFallback **0** |
| `npm run check:locales` | en/ja/zh in parity (**2442** unique keys) |
| `npx vitest run` (full FE suite) | **138 files / 741 tests PASS** |
| integration suite / smoke | not run (final S8 gate, per brief) |

## Deferrals / notes for the team lead

1. The four bridge commits (9ca1a41f, 6d316c0b, 613fef70, 08710d51) join the
   existing Codex Chat bridge back-port backlog; porting the bridge should
   include them.
2. The fork's presets remain on `kimi-k2.6` and `LongCat-Flash-Chat`; the
   pre-v3.16.5/unbatched upstream model-generation bumps (`kimi-k2.7-code`,
   `7a8b9562` LongCat-2.0) were intentionally not smuggled in via S7 — decide
   separately whether to adopt them.
3. Desktop behavior change worth calling out in S8 release notes: cc-switch.log
   is no longer wiped on startup (4 × 20 MiB rotation), crash.log is bounded,
   and proxy request/response bodies no longer appear in logs at any level;
   frontend errors now persist to `logs/frontend.log` in both runtimes.
4. `commands.manifest.json` is git-ignored; it was regenerated locally and will
   regenerate identically from source.

## Phase 2.2 independent review addendum

The check pass verified every ported hunk byte-for-byte against its upstream
commit (22d2872c shared-file hunks, 62747058 frontend files, edea624a DAO and
tests, c4795e98 core function, f2045822/e356fc6e preset values) and re-verified
all exclusions. One finding required fixes:

**e356fc6e seed reconciliation was incomplete.** The fork-local seed row
`kat-coder-pro` still carried the old preset-derived `0.002/0.006` while this
batch repriced the OpenClaw preset to `0.3/1.2` (cacheRead 0.06) — a 150×
runtime cost understatement for KAT-Coder traffic, exactly the class the batch
repriced `kimi-for-coding`/`ling-2.5-1t` to prevent. It escaped the L19
invariant test for two compounding reasons: the test's cost regex could not
parse cost objects carrying `cacheRead`/`cacheWrite` (which this batch's
repricing added to most shared entries, silently shrinking the compared set),
and it compared preset ids case-sensitively (`KAT-Coder-Pro` never matched seed
`kat-coder-pro`) while the runtime lookup lowercases.

Fixes applied:

- `kat-coder-pro` and the peg-documented `kat-coder-air` (“同族 Pro 套餐价兜底”)
  seeds → `0.30/1.20/0.06` plus `repair_current_model_pricing` entries that
  update only rows still equal to the old built-in `0.002/0.006` values;
- `qwen3.5-plus` seed cacheRead `0` → `0.052` plus a repair entry, matching the
  upstream seed/repair values verbatim (upstream has carried 0.052 since
  v3.16.5; this batch's preset port introduced the 0.052 preset side);
- `tests/config/openclawPresetPricing.test.ts` now tolerates extra cost fields,
  lowercases preset ids to mirror runtime lookup, and pins
  `kat-coder-pro`/`minimax-m2.7`/`kimi-k3` as sentinel ids that must be present
  on both sides, so parser drift can no longer make the invariant vacuous.

Post-fix, the complete S7 gate matrix above was re-run and is green
(`cargo test --lib` 1732 passed; web example suites 36 passed plus `logging::`
6 passed in the web build; route coverage 277/0/0; locales in parity; full
Vitest 138 files / 741 tests). Hygiene remains clean and no exclusion was
violated.
