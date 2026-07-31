# S1 Progress Log

Append-only checkpoint log. Format: `<file-or-hash> <status> — <note>`

## Run: 2026-07-27 (resumed attempt)

- `S1-progress.md` CREATED — checkpoint file initialized before any code work
- `src-tauri/src/services/sql_helpers.rs` CREATED — restored upstream module (post-f991726f content) that the fork had inlined into usage_stats.rs; keeps upstream import paths working
- `src-tauri/src/services/mod.rs` EDITED — `pub mod sql_helpers;`
- `src-tauri/src/proxy/usage/logger.rs` RESOLVED — upstream input_token_semantics write kept; local SystemTime-safe `created_at` kept over upstream chrono; dropped upstream `is_placeholder_pricing_model` import (fn absent in fork)
- `src-tauri/src/database/dao/usage_rollup.rs` RESOLVED — upstream fresh-input normalization + INPUT_TOKEN_SEMANTICS_FRESH write; local `pricing_missing = 0` rollup/prune guard and L18 orphan-twin test preserved; upstream's new normalization test ported
- `src-tauri/src/services/usage_stats.rs` RESOLVED — adopted upstream `row_to_request_log_detail` (fork's 2 inline mappings were byte-identical), moved fresh_input_sql to sql_helpers, upstream semantics-aware billable-input in maybe_backfill_log_costs; local qualified-column SELECT in get_request_detail kept; upstream `backfill_missing_usage_costs*` block + its 8 tests EXCLUDED (fork replaced that machinery with pricing_missing marking; helpers model_pricing_candidates/log_pricing_scope_matches/is_placeholder_pricing_model absent since the v3.16.5 port)
- `src-tauri/src/proxy/providers/streaming.rs` RESOLVED — upstream extract_cache_write_tokens + PromptTokensDetails.cache_write_tokens + 3-bucket subtraction (upstream form taken at message_start; subsumes the fork's DRY variant); 2 upstream regression tests ported
- `src-tauri/src/proxy/providers/transform.rs` RESOLVED — upstream nested cache_write resolution + 3-bucket fresh input
- `src-tauri/src/proxy/providers/transform_responses.rs` RESOLVED — upstream Step1/Step2 cache resolution + tail subtraction of both cache buckets; new nested-details test ported
- f991726f DONE — all 6 conflicts resolved, `git cherry-pick --quit`, `git ls-files -u` empty, `cargo check --all-targets` PASS
- 13e7c1fc EXCLUDED (net-zero pair with 6eb217b2) — cherry-picked, then unwound: verified `git diff 13e7c1fc^ 6eb217b2` is EMPTY for response_processor/calculator/logger/parser/session_usage×4/usage_stats, and no 1h/5m token symbols survive at 6eb217b2. SCHEMA_VERSION stays 13; no v13->v14 migration. Worktree restored, `git ls-files -u` empty.
- 6eb217b2 EXCLUDED (revert half of the pair) — usage/pricing/schema half is net-zero; its cache_injector/forwarder/types/settings/i18n half is folded into the NET post-revert port of b9263a80 + 0e563b50
- b9263a80 + 0e563b50 + (cache-TTL half of) 6eb217b2 DONE as a NET port — `cache_injector.rs` taken wholesale at 6eb217b2 (4-BP budget across tools/system/latest-cacheable-msg/prior-user anchor, caller markers preserved verbatim, warn when >4, standard 5m ephemeral). Fork's local "latest cacheable message" fix is absorbed by upstream (same test name upstream). `cache_ttl` REMOVED everywhere per the revert: proxy/types.rs OptimizerConfig, commands/settings.rs validation, web_api/handlers/config.rs validation (web-only site, not upstream), thinking_optimizer.rs fixtures, RectifierConfigPanel.tsx selector, lib/api/settings.ts type, en/ja/zh cacheTtl* keys (fork has 3 locales, not 4). 12 cache_injector tests PASS; cargo check --all-targets PASS.
- f39d463c DONE — Codex subagent usage (thread-id identity, history-replay boundary, distinct request ids) auto-merged clean into session_usage_codex.rs; local `pricing_missing` write intact; 21 tests PASS
- 98ccde00 ADAPTED — persist dashboard refresh interval. Backend/schema/types/SettingsPage wiring taken verbatim (usage_dashboard_refresh_interval_ms). UsageDashboard.tsx: upstream's normalizeRefreshInterval + props + useEffect adopted, but bound to the FORK's cycling refresh button (upstream has a select) and fork's tabs/useServerTimezone (M5 client-tz fix) kept. Upstream's add/add test file REJECTED (written against upstream's select + provider/model filters the fork lacks); its 3 persistence cases re-expressed against the fork UI + a normalization case. 6 tests PASS
- 2df2212c ADAPTED (convergent fix) — the fork had ALREADY implemented this exact fix independently as 716fdb9a (task 07-08-upstream-bug-audit-fix, 2026-07-09). 12 conflicted files reconciled hunk-by-hunk:
  * upstream's richer doc comments TAKEN everywhere (balance/coding_plan/subscription module + fn docs, queries.ts keep-last-good rationale, subscription.ts, UsageFooter, commands/subscription.rs, commands/provider.rs, commands/codex_oauth.rs, provider/usage.rs)
  * KEPT fork's `read_json_response` + `JsonResponseError::{Read,Parse}` helper over upstream's inlined `bytes()`+`from_slice` at ~14 sites — identical Err/Ok channel semantics, DRY factoring
  * KEPT fork's `http_client::get_guarded()` (SSRF guard) over upstream's `get()` at 10 sites — local security fix upstream lacks
  * KEPT fork's `siliconflow_balance_labels()` helper + its 2 tests over upstream's inlined labels
  * KEPT fork's `ProviderService::query_usage_with_templates(.., ssrf_guard)` call over upstream's `query_provider_usage_inner` (fork's signature carries the guard flag)
  * EXCLUDED upstream's entire Volcengine (火山方舟) Agent/Coding Plan block + its 5 tests + the CodingPlanProvider::Volcengine branch — the feature exists at v3.16.5 upstream but was never ported into this fork (same pre-existing gap class as backfill_missing_usage_costs); 2df2212c only adds a `Transient` VolcCall variant to it
  * queries.ts hunk: upstream comment adopted, fork's `error` variable name kept (upstream renamed to `e`)
  * keepLastGoodUsage.test.ts: upstream's 3 richer `it` blocks TAKEN; fork's now-redundant network-transient test dropped. 15 tests PASS
  * cargo check --all-targets PASS
- a7b4dd94 DONE — GPT-5.6 Sol/Terra/Luna seed rows (auto-merged clean)
- 62e44c48 DONE — Hunyuan Hy3 seed rows (hunyuan-hy3 + hy3). Conflict context also carried upstream's `kimi-k2.7-code` seed row, which the fork lacked since the v3.16.5 port — INCLUDED (pure pricing seed; a missing row means silent $0 that `pricing_missing` would flag)
- 31ee4285 ADAPTED — seed half applied verbatim (bare `gpt-5.6` alias + 5 effort suffixes at Sol rates; Sol/Terra/Luna cache-write 6.25/3.125/1.25). Repair half: kept ONLY 31ee4285's own 3 gpt-5.6 repair rows; EXCLUDED the ~20-row "2026-06-10 全量核价" repair block that came along as conflict context — the fork never had it (another v3.16.5-port gap: fork's repair table starts at deepseek-v4-flash and its seed still carries the pre-核价 values for glm-4.6/4.7, kimi-k2.5, minimax-m2.5, devstral-2, doubao-seed-2.0*, mimo*, qwen3*, grok-4.20*). DEFERRED — flag for the team lead, out of S1 scope.
- 5c39dfbf ADAPTED — gpt-5.6 context window for Claude Code takeover. Applied in topo order 5c39dfbf -> 940ddd33 -> 99573d22 (brief listed them reversed). Codex OAuth preset -> gpt-5.6 family, codexTemplates default model gpt-5.6, live.rs 372K injection + mirror-inverse backfill strip, mod.rs strip-list gains the 2 context keys. Insertion collision in live.rs resolved by KEEPING BOTH the fork's L30 `provider_common_config_strip_opt_in`/`strip_common_config_for_backfill` and upstream's `strip_injected_codex_oauth_context_defaults`. `src/config/claudeDesktopProviderPresets.ts` stays DELETED (fork dropped Claude Desktop scope). EXCLUDED from mod.rs strip list: `ANTHROPIC_DEFAULT_FABLE_MODEL{,_NAME}` + `CLAUDE_CODE_SUBAGENT_MODEL` (earlier-upstream keys the fork never ported — DEFERRED, real cross-provider leak, issue #4272) and 2 pre-existing upstream tests in mod.rs the fork lacks
- 940ddd33 DONE — Kimi For Coding routes the `kimi-for-coding` alias on every tier + 256K context/auto-compact
- 99573d22 DONE — both presets' context knobs pinned as literal env values, template form fields dropped. 18 FE preset tests PASS; cargo check --all-targets PASS
- `src-tauri/examples/web_services.rs` EDITED — added the `sql_helpers` `#[path]` shim; the web build has its own services module map and broke without it (caught by the web-server clippy gate, not the desktop one)

## Light gate (all after the final edits)

- `cargo fmt --check` PASS
- `cargo clippy --all-targets -- -D warnings` (desktop) PASS
- `cargo clippy --no-default-features --features web-server --example server -- -D warnings -A dead_code -A clippy::duplicate_mod` PASS (after adding the sql_helpers shim)
- `cargo check --no-default-features --features web-server --example server` PASS
- `cargo test` (desktop) — lib 1544 passed / 0 failed. ONE PRE-EXISTING failure in the `provider_commands` integration test (`switch_provider_updates_codex_live_and_state`); verified by stashing ALL S1 changes and re-running on clean HEAD: it fails identically there. NOT caused by S1.
- `npx tsc --noEmit` PASS (required widening `handleAutoSave` to `Promise<boolean>` + rollback, and `ProxyTabContent.onAutoSave` to `Promise<boolean | void>` — 98ccde00's persistence contract needs the boolean; both taken from upstream 98ccde00)
- `prettier --check` PASS
- `check:web-routes` PASS (missing 0, methodMismatch 0, parityFallback 0)
- `check:locales` PASS (en/ja/zh at 2355 keys, in parity)
- `npx vitest run` (full FE unit suite) — 120 files / 640 tests PASS
- Integration suite NOT run (final gate, per brief)

## End state

- zero conflict markers, `git ls-files -u` empty, no cherry-pick/merge state
- 47 files staged, no commit made
