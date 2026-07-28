# S2 report — Codex routing / protocol bridges (upstream v3.17.0)

Branch `sync/upstream-v3.18.0`. All work is **staged, not committed** (per brief).
Checkpoint log: `S2-progress.md`. End state: zero conflict markers, `git ls-files -u`
empty, no cherry-pick/merge state, **46 files staged**.

## Per-commit outcome

| # | Hash | Status | Notes |
|---|---|---|---|
| 1 | b3e5e32c | **ported (adapted)** | Claude subagent model; 12 conflicts; fable tier deferred |
| 2 | 3538b392 | **excluded** | 1M checkbox — fork FE has zero 1M-marker support |
| 3 | 95c917b3 | **ported (adapted)** | Zhipu team quota; Volcengine half excluded |
| 4 | 99e11e08 | **BLOCKED** | needs the un-ported Codex Chat bridge |
| 5 | 50270d5e | **ported (adapted)** | Fable env strip — **closes S1's deferral**; +fable mapper tier |
| 6 | ded0b63a | **ported (adapted)** | tool schema root type layered on the fork's M30 traversal |
| 7 | c6197ae3 | **ported (adapted)** | single auth placeholder; also back-ported #3784's `keep_auth_token` |
| 8 | 7479d10d | **ported (partial)** | convergent; took the TOML-escaping + staleness-guard halves |
| 9 | 27ce0a51 | **ported (adapted)** | Responses reasoning/tool-call hardening; +1064 lines auto-merged |
| 10 | a078b4b2 | **BLOCKED** | prompt_cache_key routing *for* the Chat bridge |
| 11 | 650905af | **ported (partial)** | Responses half ported; Anthropic-bridge half excluded |
| 12 | 51d6c458 | **BLOCKED** | official-routing takeover; 3 independent blockers |
| 13 | f2c6d48e | **ported (partial)** | batch-check skip ported; `resolve_base_url` doesn't exist in fork |
| 14 | f15184ed | **BLOCKED** | UI counterpart of #12 |
| 15 | af58740b | **ported** | Codex OAuth client identity; applied clean |
| 16 | ac52c851 | **ported (adapted)** | image-capability inference; new module + web shim |

**10 of 16 landed** (7 fully adapted, 3 partial), 4 blocked, 2 excluded.

## The blocker: the fork never ported the Codex Chat bridge

This is the single most important finding, and it is a **pre-existing gap, not a
conflict**. Upstream **v3.16.5 — the fork's own sync base** — already shipped
`codex_chat_common.rs`, `codex_chat_history.rs`, `streaming_codex_chat.rs` and
`transform_codex_chat.rs`. The fork has none of them, and its `codex.rs` is a
345-line passthrough adapter where upstream's is ~1000 lines with a full routing
layer. The fork already documents this at `src-tauri/src/proxy/handlers.rs:811-814`
and `:1347` ("Codex Chat 路由栈，fork 暂未移植").

Concretely, `git cherry-pick 99e11e08` produces one conflict region in `codex.rs`
spanning lines 23–479 with an **empty HEAD side**, plus two modify/delete conflicts.
Landing it means first back-porting the whole v3.16.5 chat bridge, then layering the
Anthropic bridge (`transform_codex_anthropic` 2443 lines + `streaming_codex_anthropic`
1166 + `codex_responses_sse` 399) on top — a batch of its own, in the request path.
I reverted rather than half-landing it, and verified the revert byte-for-byte.

The second blocker cluster (#12/#14) is independent and equally real: the fork has
no `sync_codex_live_from_provider_while_proxy_active`, its event plumbing is
`runtime_ctx`/`UiEventSink` where upstream uses `app_handle`/`emit`, and 51d6c458
rewrites the Codex live-config writer (`update_toml_base_url` →
`apply_codex_takeover_fields_for_provider`) on top of the fork's own atomic-write
fixes. That code writes the user's real `~/.codex/config.toml` and `auth.json`.

## MUST-FIX carried over from S1 — CLOSED

All three keys upstream strips are now stripped by
`ProviderService::extract_claude_common_config`:
`ANTHROPIC_DEFAULT_FABLE_MODEL`, `ANTHROPIC_DEFAULT_FABLE_MODEL_NAME` (50270d5e)
and `CLAUDE_CODE_SUBAGENT_MODEL` (b3e5e32c). `CLAUDE_MODEL_OVERRIDE_ENV_KEYS` is
now `[&str; 12]`. Upstream's `extract_claude_common_config_strips_fable_model_env_keys`
regression test ported verbatim and passes.

## Notable conflict resolutions

**b3e5e32c** — upstream replaced the per-role model blocks with a `modelRoleRows`
table (fable + display names + 1M checkboxes). That whole UI belongs to unported
commits, so it was dropped; a plain 子代理模型 field was added to the fork's existing
layout instead, with `subagentModel` threaded through
`ClaudeFormFields → ProviderForm → useModelState`.

**ded0b63a** — upstream rewrote `clean_schema` as `clean_schema_inner(schema, is_root)`
on top of the *old* narrow recursion (`properties` + `items` only). The fork has the
much broader M30 version. Kept the fork's full traversal and layered upstream's root
`type: "object"` injection on top; `clean_schema_child` recurses with
`is_root = false` so the root-only injection cannot leak into sub-schemas. All 8 fork
M30 tests plus upstream's 5 new ones pass.

**c6197ae3** — required back-porting a prerequisite: the fork's
`ClaudeTakeoverAuthPolicy::ManagedAccount` was a unit variant that unconditionally
inserted only `ANTHROPIC_API_KEY`; it never had upstream's #3784 `keep_auth_token`
fix. Added `ManagedAccount { keep_auth_token: bool }` with
`keep_auth_token: !provider.is_github_copilot()`, which is what c6197ae3's
`if keep_auth_token` needs to compile. Net: Codex-managed → AUTH_TOKEN only,
Copilot → API_KEY only, never both.

**7479d10d** — convergent. The fork had already implemented the Codex default-model
field independently under different names. Kept the fork's naming and took the parts
it lacked: **TOML basic-string escaping** for model names and base_url (a `/models`
id containing `"` + newline could previously inject `config.toml` lines, e.g. a forged
`[mcp_servers.*] command = …`), control-character stripping, and the `/models`
request-identity staleness guard. `providerConfigUtils.ts` was resolved by hand after
`git checkout --ours` — the 3-way merge mis-aligned so badly that one conflict had an
empty HEAD side spanning 396 lines re-inserting functions the fork already has.

**650905af** — split cleanly along the Responses/Anthropic seam. Ported
`validate_responses_success_response` + `validate_responses_stream_start` and their
helpers, adapted to the fork's 2-tuple return (upstream's is a 3-tuple carrying the
un-ported `outbound_model`). Real gain: a Claude→Responses gateway returning a
semantic failure inside an HTTP 2xx body — or `response.failed` before any output on
SSE — is now detected *inside* the retry loop, so it fails over instead of surfacing
a dead response. Also ported `ProxyResponse::is_json()`, which the fork lacked.

## Fork behavior re-applied on top of upstream

- `http_client::get_guarded()` (SSRF guard) used in the new `query_zhipu_team_at`
  where upstream uses `get()`.
- The fork's `TOML_MODEL_REPLACE_PATTERN` inline-comment preservation kept in
  `setCodexModelName`, now substituting the escaped value.
- The fork's M30 full-schema traversal kept under upstream's root-type injection.
- `runtime_ctx`/`UiEventSink` web abstraction untouched.
- `src/config/claudeDesktopProviderPresets.ts` and `src/i18n/locales/zh-TW.json`
  stay deleted; upstream's zh-TW hunks dropped in every commit.
- The fork's `usageApi.testScript` path kept for every coding-plan provider except
  `zhipu_team`, which needs in-modal (unsaved) org/project IDs.

## Two things I wrote rather than ported

Both flagged for review:

1. **`ModelMapping.fable_model` + the fable tier in `model_mapper.rs`.** 50270d5e
   makes takeover write `ANTHROPIC_DEFAULT_FABLE_MODEL{,_NAME}` into live config,
   but the fork's mapper had no fable tier — requests would have fallen through to
   `default_model` while the /model menu showed the provider's fable name. Used the
   fork's word-boundary `contains_model_tier` (its L28 fix) rather than upstream's
   bare `contains`, plus upstream's fable→opus downgrade. Upstream's 4 fable tests
   ported and pass.
2. **`sync_codex_live_from_provider_while_proxy_active`** (~40 lines). ac52c851's
   auto-merged resync branch calls it and the fork had no Codex twin of the Claude
   function. Written with the fork's existing helpers
   (`build_effective_settings_with_common_config` →
   `preserve_codex_mcp_servers_from_existing_config` → placeholder auth +
   `update_toml_base_url` → `write_codex_takeover_live_for_provider`) so it does
   **not** drag in blocked 51d6c458. It writes the user's real `~/.codex/config.toml`.

## Bug I introduced and fixed

The hot-switch assertion I ported from upstream in b3e5e32c
(`subagent model should follow the target provider`) is **wrong for this fork**: the
fork's hot switch calls `cleanup_claude_model_overrides_in_live`, which removes every
`CLAUDE_MODEL_OVERRIDE_ENV_KEYS` entry after the takeover sync, so subagent/fable pins
are cleared like `ANTHROPIC_MODEL`. Inverted to `is_none()` with a comment recording
the divergence. It was caught by an unfiltered `cargo test --lib` during 50270d5e,
not by the filtered run I did for b3e5e32c — hence I ran the full lib suite after
every subsequent commit.

## Deferrals for the team lead

**Schedule as their own batches:**

1. **Codex Chat bridge back-port from v3.16.5** — unblocks 99e11e08 and a078b4b2.
   4-5 modules + `codex.rs` routing + forwarder/handlers/codex_config/ProviderMeta/
   `CodexApiFormat` wiring.
2. **51d6c458 + f15184ed together** — official-routing takeover. Prerequisites that
   *do* exist: `CODEX_OFFICIAL_PROVIDER_ID`, `CodexCatalogToolProfile::NativeResponses`,
   `require_current_provider_for_app`, `preserve_codex_mcp_servers_from_existing_config`,
   `write_codex_takeover_live_for_provider`, and now
   `sync_codex_live_from_provider_while_proxy_active`. Needs a `web-commands.ts` route
   for the new `ensure_codex_official_provider` command.

**Security gap found (not S2 scope, worth its own ticket):** the fork's
`extract_claude_common_config` has **no generic credential scrubbing**. Upstream's
`extract_claude_common_config_strips_all_credentials_keeps_shareable` test could not
be ported because `OPENROUTER_API_KEY`, `GOOGLE_API_KEY`, `OPENAI_API_KEY`,
`GEMINI_API_KEY`, `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN`,
`GOOGLE_APPLICATION_CREDENTIALS`, `AWS_BEARER_TOKEN_BEDROCK` and top-level
`apiKey`/`api_key` all survive extraction — so "use this provider's config as the
shared common config" can copy a credential into the snippet that then deep-merges
into every other provider. The fork's `ENV_EXCLUDES` only covers Anthropic auth,
models and base URL.

**Smaller gaps surfaced:** the fork's FE has no 1M-marker support at all (blocks
3538b392 and upstream's model-role table); no model-catalog editor UI; no
`resolve_base_url`/`probe_reachability` reachability-probe refactor in
`services/stream_check.rs`.

## Files changed (46 staged)

New: `src-tauri/src/model_capabilities.rs`,
`src-tauri/src/proxy/providers/reasoning_bridge.rs`,
`tests/hooks/useModelState.test.tsx`, `tests/hooks/useApiKeyState.test.tsx`

Rust: `examples/server.rs` (model_capabilities `#[path]` shim), `codex_config.rs`,
`commands/{coding_plan,stream_check}.rs`, `deeplink/provider.rs`, `lib.rs`,
`provider.rs`, `proxy/{forwarder,handlers,hyper_client,media_sanitizer,model_mapper,types}.rs`,
`proxy/providers/{auth,claude,mod,streaming_responses,transform,transform_responses}.rs`,
`resources/codex_native_responses_template.json`, `services/coding_plan.rs`,
`services/provider/{mod,usage}.rs`, `services/proxy.rs`,
`web_api/handlers/subscription.rs`

Frontend: `components/UsageScriptModal.tsx`,
`components/providers/forms/{ClaudeFormFields,CodexFormFields,ProviderForm}.tsx`,
`components/providers/forms/hooks/{useApiKeyState,useCodexConfigState,useModelState}.ts`,
`config/{codexProviderPresets,codingPlanProviders}.ts`, `lib/api/subscription.ts`,
`types.ts`, `utils/providerConfigUtils{,.test}.ts`, `i18n/locales/{en,ja,zh}.json`

Tests: `tests/components/ClaudeFormFields.test.tsx`

## Gate results

| Gate | Result |
|---|---|
| `cargo fmt --check` | PASS |
| `cargo clippy --all-targets -- -D warnings` | PASS |
| web `cargo clippy --no-default-features --features web-server --example server` | PASS |
| web `cargo check --no-default-features --features web-server --example server` | PASS |
| `cargo test --lib` | **1596 passed / 0 failed / 2 ignored** |
| `cargo test` (full desktop) | 1 pre-existing failure (below) |
| `npx tsc --noEmit` | PASS |
| `npm run format:check` | PASS |
| `npx vitest run` | **122 files / 655 tests PASS** |
| `check:web-routes` | missing 0, methodMismatch 0, parityFallback 0 |
| `check:locales` | in parity |
| integration suite | not run (final S8 gate, per brief) |

**One pre-existing failure**, not caused by S2:
`tests/provider_commands.rs::switch_provider_updates_codex_live_and_state`
("live file should contain provider's original config"). Verified by stashing all S2
work and re-running `cargo test --test provider_commands -- --test-threads=1` on
clean HEAD: identical single failure. The extra failures visible in the default
parallel run are cross-test `$HOME`/global-settings interference (these tests are not
`#[serial]`) and reproduce on clean HEAD too.
