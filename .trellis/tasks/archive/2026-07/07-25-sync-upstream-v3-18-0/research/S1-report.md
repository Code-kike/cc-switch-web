# S1 report — usage/pricing core correctness (upstream v3.16.5..v3.18.0)

Branch `sync/upstream-v3.18.0`. All work is **staged, not committed** (per brief).
Checkpoint log: `S1-progress.md`. End state: zero conflict markers, `git ls-files -u`
empty, no cherry-pick/merge state, 47 files staged.

## Per-commit outcome

| Hash | Status | Notes |
|---|---|---|
| f991726f | **ported (adapted)** | cache-write tokens across schema versions; 6 conflicts resolved |
| 13e7c1fc | **excluded** | net-zero pair with 6eb217b2 (see below) |
| b9263a80 | **ported (net)** | prompt cache breakpoint injection |
| 0e563b50 | **ported (net)** | unsupported breakpoint counts |
| 6eb217b2 | **ported (partial) / excluded (partial)** | usage-half net-zero; cache-TTL-removal half ported |
| f39d463c | **ported** | Codex subagent usage; clean auto-merge |
| 98ccde00 | **ported (adapted)** | dashboard refresh interval persistence |
| 2df2212c | **ported (adapted)** | convergent with the fork's own 716fdb9a |
| 31ee4285 | **ported (partial)** | gpt-5.6 aliases + 1.25x cache-write |
| a7b4dd94 | **ported** | GPT-5.6 Sol/Terra/Luna seed |
| 62e44c48 | **ported** | Hunyuan Hy3 seed |
| 99573d22 | **ported** | context windows pinned as literal env |
| 940ddd33 | **ported** | Kimi For Coding 256K |
| 5c39dfbf | **ported (adapted)** | gpt-5.6 context window for Claude Code takeover |

Applied in **topological order**, not the brief's listed order, where the brief's
order inverted a dependency:

- `a7b4dd94 → 62e44c48 → 31ee4285` (31ee4285 rewrites rows a7b4dd94 seeds)
- `5c39dfbf → 940ddd33 → 99573d22` (99573d22 removes the template fields the
  other two add)

## 13e7c1fc + 6eb217b2: net-zero pair

`6eb217b2` is an explicit revert of `13e7c1fc`'s 1-hour cache-TTL feature. I
cherry-picked 13e7c1fc, then verified `git diff 13e7c1fc^ 6eb217b2` is **empty**
for `response_processor.rs`, `calculator.rs`, `logger.rs`, `parser.rs`,
`session_usage{,_codex,_gemini,_opencode}.rs` and `usage_stats.rs`, and that no
`cache_creation_1h_tokens` / `cache_creation_5m_tokens` symbol survives at
6eb217b2. I then unwound 13e7c1fc rather than applying two commits' churn for
zero net effect. `SCHEMA_VERSION` stays **13**; no v13→v14 migration exists.

`6eb217b2` is **not** purely a revert: it also removes the pre-existing
`cache_ttl` selector (present since v3.16.5). That half **was** ported — see
below.

## Conflict resolutions

### f991726f (6 files)

- **`services/sql_helpers.rs` restored as its own module.** The fork had inlined
  `fresh_input_sql` into `usage_stats.rs` during its v3.16.5 port; upstream's
  new code imports `crate::services::sql_helpers::{fresh_input_sql,
  INPUT_TOKEN_SEMANTICS_*}` from `usage_rollup.rs` and `logger.rs`. I took
  upstream's module verbatim, wired `pub mod sql_helpers;`, and deleted the
  inlined copy. Verified the generated SQL was byte-identical beforehand.
- **`usage_rollup.rs`** — upstream's fresh-input normalization + FRESH-semantics
  write. Local `pricing_missing = 0` guard on both aggregation and prune, and the
  L18 orphan-twin test, preserved. Upstream's new normalization test ported.
- **`usage_stats.rs`** — adopted upstream's `row_to_request_log_detail` (the
  fork's two inline mappings were byte-identical, so this is a pure dedup) and
  upstream's semantics-aware `billable_input_tokens`. Local qualified-column
  SELECT in `get_request_detail` kept (guards the ambiguous-column bug the fork
  has a regression test for).
- **`logger.rs`** — upstream's `input_token_semantics` write kept; the fork's
  panic-safe `SystemTime` `created_at` kept over upstream's `chrono` (upstream
  did not touch that line — pure context collision).
- **`streaming.rs` / `transform.rs` / `transform_responses.rs`** — upstream taken
  wholesale: `extract_cache_write_tokens`, `PromptTokensDetails.cache_write_tokens`,
  nested `cache_write_tokens` resolution, three-bucket `saturating_sub`. The
  fork's older two-bucket logic was superseded; its DRY `build_anthropic_usage_json`
  reuse at `message_start` was replaced by upstream's inline form (upstream had
  modified that exact line, so "upstream-first" applies).

### 2df2212c — convergent fix

The fork had **already implemented this exact fix independently** as `716fdb9a`
("fix(usage): preserve last good data on transient failures", task
`07-08-upstream-bug-audit-fix`, 2026-07-09). 12 files conflicted; reconciled
hunk-by-hunk:

- Upstream's (much richer) doc comments **taken** everywhere.
- **Kept** the fork's `read_json_response` + `JsonResponseError::{Read,Parse}`
  helper over upstream's inlined `bytes()` + `from_slice` at ~14 sites —
  identical `Err`/`Ok(success:false)` channel semantics, DRY factoring.
- **Kept** `http_client::get_guarded()` over upstream's `get()` at 10 sites
  (fork's SSRF guard — a local security fix upstream lacks).
- **Kept** the fork's `siliconflow_balance_labels()` helper + its 2 tests, and
  `ProviderService::query_usage_with_templates(.., ssrf_guard)` over upstream's
  `query_provider_usage_inner` (the fork's signature carries the guard flag).
- `keepLastGoodUsage.test.ts`: upstream's 3 richer `it` blocks taken; the fork's
  now-redundant network-transient case dropped. 15 tests pass.

### 98ccde00 — adapted to the fork's UI

Backend/schema/types/SettingsPage wiring taken verbatim. `UsageDashboard.tsx`
takes upstream's `normalizeRefreshInterval` + props + `useEffect`, but bound to
the **fork's cycling refresh button** (upstream has a `<select>`), and the fork's
tabs + `useServerTimezone()` (M5 client-tz bucketing) preserved. Upstream's
add/add test file was **rejected** — it is written against upstream's select and
provider/model filters, which the fork does not have; its 3 persistence cases
were re-expressed against the fork's UI plus a normalization case.

This commit's contract also required porting upstream's `handleAutoSave`
(`Promise<boolean>` + optimistic-update rollback on failure) and widening
`ProxyTabContent.onAutoSave` — otherwise the rollback path is unreachable and
`tsc` fails.

### 5c39dfbf — insertion collision in `live.rs`

`provider/live.rs` conflicted because the fork's L30 audit fix
(`provider_common_config_strip_opt_in` + `strip_common_config_for_backfill`) and
upstream's new `strip_injected_codex_oauth_context_defaults` were inserted at the
same point. **Both kept.** `src/config/claudeDesktopProviderPresets.ts` stays
deleted (the fork dropped Claude Desktop scope).

## Local fixes re-applied on top of upstream

- `pricing_missing` no-silent-$0 semantics (rollup aggregation + prune guards,
  session-usage writers, schema v11→v12).
- Client/server-timezone usage bucketing (`useServerTimezone`, M5).
- `http_client::get_guarded()` SSRF guard on all balance/coding-plan/subscription
  dials.
- Panic-safe `SystemTime` `created_at` in the usage logger.
- Qualified-column SELECT in `get_request_detail`.
- L18 orphan session-twin pruning.
- Web feature-gating: added the `sql_helpers` `#[path]` shim to
  `examples/web_services.rs` (the web build has its own module map and broke
  without it — caught only by the web-server clippy gate, not the desktop one).

## Deliberately excluded

| What | Why |
|---|---|
| `backfill_missing_usage_costs*` + `model_pricing_candidates` + `log_pricing_scope_matches` + `is_placeholder_pricing_model` and their 8 tests | The fork replaced this machinery with `pricing_missing` marking; the helpers have been absent since the v3.16.5 port |
| Upstream's Volcengine (火山方舟) Agent/Coding Plan block, 5 tests, `CodingPlanProvider::Volcengine` branch | Feature exists at v3.16.5 upstream but was never ported into this fork; 2df2212c only adds a `Transient` variant to it |
| "2026-06-10 全量核价" repair block (~20 models) | Came along as 31ee4285 conflict context; belongs to an earlier upstream commit the fork never ported |
| `ANTHROPIC_DEFAULT_FABLE_MODEL{,_NAME}`, `CLAUDE_CODE_SUBAGENT_MODEL` strip-list keys | Earlier-upstream keys, out of 5c39dfbf's scope |
| 2 pre-existing upstream tests in `provider/mod.rs`, 1 in `usage_stats.rs` | Depend on the excluded machinery above |

## Deferred — needs a decision from the team lead

These are **pre-existing gaps from the fork's v3.16.5 port**, surfaced (not
caused) by S1. None are S1 scope; all are real:

1. **Stale pricing for ~20 models.** The fork's seed still carries the
   pre-核价 values (glm-4.6/4.7, kimi-k2.5, minimax-m2.5, devstral-2,
   doubao-seed-2.0×5, mimo×3, qwen3×4, grok-4.20×2) and its
   `repair_current_model_pricing` table starts at `deepseek-v4-flash`. Directly
   contradicts the "no silent mispricing" goal.
2. **Cross-provider env leak (upstream issue #4272).** The fork's common-config
   strip list lacks `ANTHROPIC_DEFAULT_FABLE_MODEL{,_NAME}` and
   `CLAUDE_CODE_SUBAGENT_MODEL`, so those provider-specific model pins can enter
   the shared snippet and pollute other providers.
3. **No Volcengine coding-plan support** (upstream has had it since v3.16.5).
4. **No batch cost backfill.** Whether the fork's `pricing_missing` marking is a
   deliberate replacement for upstream's `backfill_missing_usage_costs` or an
   incomplete port is worth confirming — the two solve the same problem.

Also note for **S4 (Profiles)**: the schema versions have diverged. Upstream's
v11→v12 creates the `profiles` table; the fork's v11→v12 adds `pricing_missing`.
Both are now at v13 (input_token_semantics). Profiles will need a **v14**
migration in this fork.

## Gate results

| Gate | Result |
|---|---|
| `cargo fmt --check` | PASS |
| `cargo clippy --all-targets -- -D warnings` (desktop) | PASS |
| `cargo clippy --no-default-features --features web-server --example server -- -D warnings -A dead_code -A clippy::duplicate_mod` | PASS |
| `cargo check --no-default-features --features web-server --example server` | PASS |
| `cargo test` (desktop) | lib **1544 passed / 0 failed**; 1 pre-existing integration failure (below) |
| `tsc --noEmit` | PASS |
| `prettier --check` | PASS |
| `check:web-routes` | PASS (missing 0, methodMismatch 0, parityFallback 0) |
| `check:locales` | PASS (en/ja/zh at 2355 keys) |
| `vitest run` (full FE unit suite) | **120 files / 640 tests PASS** |
| integration suite | not run (final gate, per brief) |

**One pre-existing test failure**, not caused by S1:
`tests/provider_commands.rs::switch_provider_updates_codex_live_and_state`
("live file should contain provider's original config"). Verified by stashing
**all** S1 changes and re-running on clean `HEAD` — it fails identically there.

One fork test was updated to upstream's semantics:
`transform_responses::tests::test_responses_to_anthropic_with_direct_cache_fields`
asserted `input_tokens == 40` (subtract cache_read only); under f991726f's
three-bucket model it is `100 - 60 - 20 = 20`, matching upstream v3.18.0 verbatim.

## Files changed (47 staged)

New: `src-tauri/src/services/sql_helpers.rs`
Deleted: `src/config/claudeDesktopProviderPresets.ts` (kept deleted)

Rust: `examples/web_services.rs`, `commands/{codex_oauth,provider,settings,subscription}.rs`,
`database/{mod,schema}.rs`, `database/dao/usage_rollup.rs`,
`proxy/{cache_injector,thinking_optimizer,types}.rs`,
`proxy/providers/{streaming,transform,transform_responses}.rs`,
`proxy/usage/{calculator,logger,parser}.rs`,
`services/{balance,coding_plan,mod,subscription,session_usage_codex,usage_stats}.rs`,
`services/provider/{live,mod,usage}.rs`, `settings.rs`, `web_api/handlers/config.rs`

Frontend: `components/{UsageFooter,UsageScriptModal}.tsx`,
`components/settings/{ProxyTabContent,RectifierConfigPanel,SettingsPage}.tsx`,
`components/usage/UsageDashboard.tsx`,
`config/{claudeProviderPresets,codexTemplates}.ts`,
`lib/api/settings.ts`, `lib/query/{queries,subscription}.ts`,
`lib/schemas/settings.ts`, `types.ts`, `i18n/locales/{en,ja,zh}.json`

Tests: `tests/components/UsageDashboard.test.tsx`,
`tests/config/claudeProviderPresets.test.ts`, `tests/lib/keepLastGoodUsage.test.ts`
