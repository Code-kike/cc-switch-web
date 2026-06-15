# Quality Guidelines

> Code quality standards for frontend development.

---

## Overview

<!--
Document your project's quality standards here.

Questions to answer:
- What patterns are forbidden?
- What linting rules do you enforce?
- What are your testing requirements?
- What code review standards apply?
-->

(To be filled by the team)

---

## Forbidden Patterns

<!-- Patterns that should never be used and why -->

(To be filled by the team)

---

## Required Patterns

### Scenario: Secret-Bearing Web API Commands

#### 1. Scope / Trigger

- Trigger: a Web API command carries provider credentials, API keys, tokens, or
  generated credential file content.
- Applies when changing `src/lib/api/web-commands.ts`, `src/lib/api/adapter.ts`,
  `src-tauri/src/web_api/handlers/*`, or credential-writing helpers such as
  `config::atomic_write`.

#### 2. Signatures

- Frontend registry:
  - `fetch_models_for_config: { method: "POST", path: "/api/config/fetch-models-for-config" }`
  - `get_balance: { method: "POST", path: "/api/usage/get-balance" }`
  - `get_coding_plan_quota: { method: "POST", path: "/api/usage/get-coding-plan-quota" }`
- Web handlers:
  - `POST /api/config/fetch-models-for-config` with `Json<FetchModelsForConfigQuery>`
  - `POST /api/usage/get-balance` with `Json<BalanceQuery>`
  - `POST /api/usage/get-coding-plan-quota` with `Json<BalanceQuery>`
- Storage:
  - `config::atomic_write(path, data)` writes credential/config files via a
    temporary file and rename.

#### 3. Contracts

- Secret-bearing web commands must use a JSON body, not query strings.
- Registry method, Rust route method, and extractor shape must change together:
  `POST` in `web-commands.ts`, `post(handler)` in Axum, and `Json<T>` in the
  handler signature.
- Desktop Tauri commands stay unrestricted unless the change explicitly targets
  desktop; web-only SSRF/CSRF behavior belongs in `web_api` handlers/adapter.
- On Unix, newly created credential/config temp files must be opened with
  `0o600` before bytes are written. Existing destinations may mirror their
  existing permission bits before rename.

#### 4. Validation & Error Matrix

- `api_key` or token appears in a GET query string -> reject; it can leak through
  browser history, proxy logs, or access logs.
- Frontend says `POST` but Axum route still uses `get(...)` -> reject; web mode
  receives 405/404 while desktop invoke still appears fine.
- Axum route is `post(...)` but handler still extracts `Query<T>` -> reject; the
  body is ignored and credential fields deserialize as missing.
- New Unix credential file is created with default permissions then chmodded ->
  reject; there is a readable race window before chmod.

#### 5. Good/Base/Bad Cases

- Good: `fetch_models_for_config({ baseUrl, apiKey, ... })` reaches
  `Json<FetchModelsForConfigQuery>` and the browser URL remains
  `/api/config/fetch-models-for-config`.
- Base: non-secret GET commands may continue to use query strings when their
  arguments are identifiers, filters, or pagination fields.
- Bad: adding `api_key`, `access_token`, or `Authorization` data to a command
  whose web registry method is `GET`.

#### 6. Tests Required

- `pnpm check:web-routes` must report `missing: 0` after method/route edits.
- `pnpm typecheck` must pass after command registry method literal changes.
- Rust route/handler changes must pass `cargo clippy -- -D warnings` and
  `cargo test`.
- Smoke or integration probes for these endpoints must send `POST` with
  `Content-Type: application/json`, not query strings.

#### 7. Wrong vs Correct

##### Wrong

```typescript
get_balance: { method: "GET", path: "/api/usage/get-balance" };
```

```rust
.route("/usage/get-balance", get(get_balance))
async fn get_balance(Query(query): Query<BalanceQuery>) -> ApiResult<_> { ... }
```

##### Correct

```typescript
get_balance: { method: "POST", path: "/api/usage/get-balance" };
```

```rust
.route("/usage/get-balance", post(get_balance))
async fn get_balance(Json(query): Json<BalanceQuery>) -> ApiResult<_> { ... }
```

---

### Scenario: Built-In Model Pricing Lookup

#### 1. Scope / Trigger

- Trigger: provider presets, default models, or pricing seeds change.
- Applies when editing provider preset model IDs,
  `schema.rs::seed_model_pricing`, or
  `usage_stats.rs::find_model_pricing_row`.

#### 2. Signatures

- `find_model_pricing_row(conn: &Connection, model_id: &str) -> Result<Option<(String, String, String, String)>, AppError>`
- `seed_model_pricing(conn)` owns built-in `model_pricing.model_id` rows.

#### 3. Contracts

- Lookup cleans provider prefixes, colon suffixes, and `@` variants before
  querying.
- Candidate order is exact cleaned ID, lowercase cleaned ID, then lowercase
  with dots converted to dashes.
- Dot-to-dash fallback must stay last so dotted lowercase rows such as
  `gpt-5.5`, `minimax-m2.7`, and `glm-5.1` still match their exact seed rows.
- Bare default IDs used by presets, such as `claude-sonnet-4-6` and
  `claude-haiku-4-5`, must have seed rows when they are not only aliases for a
  dated model ID.

#### 4. Validation & Error Matrix

- Preset default resolves to no pricing row -> reject; usage cost silently
  records as `0`.
- Dot-to-dash runs before exact/lowercase lookup -> reject; dotted seed rows can
  be bypassed.
- Adding a preset default without searching `model_pricing` seeds -> reject;
  this creates a cost-reporting regression.

#### 5. Good/Base/Bad Cases

- Good: `claudecn/claude-haiku-4-5` cleans to `claude-haiku-4-5` and resolves
  to a seeded price.
- Good: `MiniMaxAI/MiniMax-M2.7` lowercases and resolves to `minimax-m2.7`.
- Base: `anthropic/claude-opus-4.8` falls through to dot-to-dash and resolves to
  `claude-opus-4-8`.
- Bad: only seeding `claude-haiku-4-5-20251001` while presets default to bare
  `claude-haiku-4-5`.

#### 6. Tests Required

- Rust regression tests for bare default IDs and prefixed aggregator forms.
- Guard tests for dotted lowercase IDs to prove dot-to-dash fallback does not
  preempt exact or lowercase matches.
- Unknown model IDs must still return `None`.

#### 7. Wrong vs Correct

##### Wrong

```rust
let key = cleaned.to_lowercase().replace('.', "-");
query_model_pricing(key)
```

##### Correct

```rust
for key in [cleaned.clone(), cleaned.to_lowercase(), cleaned.to_lowercase().replace('.', "-")] {
    if let Some(row) = query_model_pricing(&key)? {
        return Ok(Some(row));
    }
}
```

---

### Scenario: Provider Usage Query Templates

#### 1. Scope / Trigger

- Trigger: provider usage query spans UI, frontend API adapter, Web API handlers, Tauri commands, and provider services.
- Applies when changing `usageApi.query`, `usageApi.testScript`, `UsageScriptModal`, or backend provider usage handlers.

#### 2. Signatures

- Frontend:
  - `usageApi.query(providerId: string, appId: AppId): Promise<UsageResult>`
  - `usageApi.testScript(providerId, appId, scriptCode, timeout?, apiKey?, baseUrl?, accessToken?, userId?, templateType?): Promise<UsageResult>`
- Web routes:
  - `POST /api/providers/queryproviderusage`
  - `POST /api/usage/testusagescript`
- Backend service:
  - `ProviderService::query_usage_with_templates(state, app_type, provider_id, copilot_auth)`
  - `ProviderService::test_usage_script(..., template_type, copilot_auth)`

#### 3. Contracts

- All usage-template paths return `UsageResult`:
  - `success: boolean`
  - `data?: UsageData[]`
  - `error?: string`
- `UsageData` fields are optional. Display code must handle rows that include
  only `used`/`total`, only `remaining`, only `extra`, or invalid-state fields
  without rendering `undefined`/`NaN`.
- Built-in templates are `github_copilot`, `token_plan`, `balance`, and
  `official_subscription` (v3.16.2 sync; explicit opt-in, default off, with a
  configurable refresh interval).
- Built-in template tests must go through `usageApi.testScript`, not lower-level subscription or Copilot APIs — EXCEPT `official_subscription`, whose test button calls `subscriptionApi.getQuota(appId)` directly (upstream-verbatim): it queries via CLI/OAuth credentials with no provider secret, and `getQuota` IS the production path the saved card uses (web route `GET /api/subscription/get-subscription-quota`; both paths converge on `get_subscription_quota`).
- `token_plan` providers with paired script credentials (zenmux): credentials
  resolve as a pair (script apiKey+baseUrl together → native pair fallback →
  partial script); non-zenmux keeps per-field script-over-provider resolution
  (`resolve_coding_plan_credentials` in `services/provider/usage.rs`).
- Saved provider-card refresh must go through `usageApi.query`, which reaches `query_usage_with_templates`.
- Frontend API wrappers should normalize transport/API errors into `UsageResult { success: false, error }` so the provider UI can show actionable failures.
- `UsageScriptModal` success toasts should format rows through
  `formatUsageDataSummary` so saved-query and test-query displays stay
  consistent across sparse custom-script payloads.

#### 4. Validation & Error Matrix

- Missing provider -> failed `UsageResult` or API error with provider-not-found detail.
- Disabled usage script -> failed `UsageResult` with "usage disabled" detail.
- Unsupported or malformed JS template -> failed `UsageResult`; do not leave stale success data visible.
- Built-in templates ignore the JS body; save-time JS `request.url` validation must not block them.
- Custom scripts may use a full explicit quota URL that does not match the
  provider `baseUrl`. Runtime request validation may validate that request URL,
  but must not reject the unused provider `baseUrl` fallback for custom
  templates.
- Web mode must not call commands marked `unsupported` in `src/lib/api/web-commands.ts` from usage-template testing.

#### 5. Good/Base/Bad Cases

- Good: Balance, Token Plan, and GitHub Copilot template test buttons call `usageApi.testScript(..., templateType)`, then write the returned `UsageResult` into `["usage", provider.id, appId]`; the Official Subscription test button calls `subscriptionApi.getQuota(appId)`.
- Base: custom/general/newapi scripts call `usageApi.testScript` with script code and explicit credential overrides.
- Bad: testing Balance via `subscriptionApi.getBalance`, Token Plan via `subscriptionApi.getCodingPlanQuota`, or Copilot via `copilot_get_usage*` from `UsageScriptModal` (the `official_subscription`→`getQuota` exception above is the only sanctioned direct subscription call).
- Bad: formatting custom-script test output by directly interpolating
  `plan.remaining` and `plan.unit`; sparse rows can omit both fields.

#### 6. Tests Required

- Unit test `usageApi.query` and `usageApi.testScript` error normalization for Web API failures.
- Unit test `formatUsageDataSummary` for sparse rows where `remaining` or
  `unit` is absent.
- Component tests asserting each built-in template calls `usageApi.testScript` with the expected template type.
- Rust test custom-script URL validation when an explicit HTTP/LAN quota URL is
  used with a different provider `baseUrl`.
- Backend compile check for Web server mode after changing provider usage signatures:
  - `cargo check --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server`

#### 7. Wrong vs Correct

##### Wrong

```typescript
await subscriptionApi.getBalance(baseUrl, apiKey);
await copilotGetUsage();
```

```typescript
`${plan.remaining} ${plan.unit}`;
```

##### Correct

```typescript
await usageApi.testScript(
  provider.id,
  appId,
  "",
  script.timeout,
  undefined,
  undefined,
  undefined,
  undefined,
  "balance",
);
```

```typescript
formatUsageDataSummary(plan, labels);
```

The same pattern applies to `token_plan` and `github_copilot`.

---

### Scenario: Codex Provider OAuth Preservation

#### 1. Scope / Trigger

- Trigger: changing Codex provider switching, provider import/export, proxy
  takeover, or frontend Codex API-key editing.
- Applies when modifying `src-tauri/src/codex_config.rs`,
  `src-tauri/src/services/provider/live.rs`,
  `src-tauri/src/services/proxy.rs`, `src/utils/providerConfigUtils.ts`,
  `ProviderCard`, or Codex config editor hooks.

#### 2. Signatures

- Backend Codex helpers:
  - `extract_codex_api_key(auth: Option<&Value>, config_text: Option<&str>)`
  - `extract_codex_experimental_bearer_token(config_text: &str)`
  - `prepare_codex_provider_live_config(auth: &Value, config_text: &str)`
  - `restore_codex_provider_token_for_backfill(settings, template_settings)`
  - `write_codex_provider_live_with_catalog(settings, category, auth, config_text)`
- Frontend TOML helpers:
  - `extractCodexExperimentalBearerToken(configText?: string | null)`
  - `updateCodexExperimentalBearerToken(configText: string, token: string)`

#### 3. Contracts

- **Preservation is gated (v3.16.1 sync)**: the contracts below apply only when
  the device setting `preserve_codex_official_auth_on_switch` (settings.json,
  default **false**) is enabled — FE toggle in Settings → "Codex App
  Enhancements" (`CodexAuthSettings.tsx`). With the gate OFF (default,
  upstream parity), third-party Codex switches overwrite `auth.json` via the
  legacy write path (`should_write_auth` in `codex_config.rs` decides).
- Stored third-party Codex providers keep their canonical token in
  `settings_config.auth.OPENAI_API_KEY`.
- Live third-party Codex switches (gate ON) write that token into `config.toml` as
  `experimental_bearer_token`, preferably under the active
  `[model_providers.<id>]` table. They must not overwrite a user's OAuth
  `auth.json` login cache.
- Official/OAuth-only providers may keep real login material in `auth.json`;
  backfill must not convert OAuth-only live credentials into a stored
  third-party API key.
- Config-only live installs are valid import sources: the UI and backend must
  read `experimental_bearer_token` as the Codex API key fallback.
- Proxy takeover and cleanup must check both `auth.OPENAI_API_KEY` and
  `experimental_bearer_token` for the proxy placeholder token. In
  preserve-mode takeover, OAuth `auth.json` stays untouched and cleanup
  removes the placeholder via `remove_codex_experimental_bearer_token_if`
  without touching OAuth material.

#### 4. Validation & Error Matrix

- Gate ON + third-party switch rewrites OAuth `auth.json` -> reject; user is
  logged out of ChatGPT-backed Codex. (Gate OFF: overwriting `auth.json` is
  the intended upstream-parity behavior, not a defect.)
- API key exists only in `experimental_bearer_token` but UI shows blank ->
  reject; config-only installs become uneditable.
- Backfill copies an OAuth-only access/refresh/id token into
  `auth.OPENAI_API_KEY` -> reject; provider category/auth shape is corrupted.
- Proxy cleanup only removes `auth.OPENAI_API_KEY` placeholder -> reject;
  takeover state can remain stuck in `config.toml`.

#### 5. Good/Base/Bad Cases

- Good: with the gate enabled, a third-party provider with `auth.OPENAI_API_KEY = "sk-live"` and an
  existing OAuth `auth.json` switches live by preserving `auth.json` and adding
  `experimental_bearer_token = "sk-live"` to `config.toml`.
- Base: a config-only Codex install imports with empty `auth` and
  `experimental_bearer_token = "sk-live"`, then stores the provider token back
  as `auth.OPENAI_API_KEY`.
- Bad: using provider switching to write `OPENAI_API_KEY` into live `auth.json`
  whenever the provider has a third-party token.

#### 6. Tests Required

- Rust regression tests for third-party switch preserving OAuth `auth.json` and
  writing the token to `config.toml`.
- Rust import/export or sync tests for config-only Codex providers and
  OAuth-only providers that must not be backfilled.
- Rust proxy tests for takeover detection and cleanup when the placeholder
  lives in `experimental_bearer_token`.
- Vitest coverage for extracting, displaying, and updating Codex API keys from
  `experimental_bearer_token`.
- Run `pnpm typecheck`, focused Codex Vitest suites, focused Rust Codex tests,
  Web-server cargo check, and `git diff --check`.

#### 7. Wrong vs Correct

##### Wrong

```rust
write_json_file(auth_path, json!({ "OPENAI_API_KEY": provider_key }))?;
```

##### Correct

```rust
let config = prepare_codex_provider_live_config(&provider_auth, config_text)?;
// Preserve the existing OAuth auth.json login cache.
std::fs::write(get_codex_config_path(), config)?;
```

---

### Scenario: Codex Provider Goal Mode

#### 1. Scope / Trigger

- Trigger: changes to Codex provider templates, `config.toml` editing helpers,
  or `CodexConfigSection` provider-editor controls.
- Applies when modifying `src/config/codexTemplates.ts`,
  `src/config/codexProviderPresets.ts`, `src/utils/providerConfigUtils.ts`, or
  `src/components/providers/forms/CodexConfigSections.tsx`.

#### 2. Signatures

- Frontend TOML helpers:
  - `isCodexGoalModeEnabled(configText?: string | null): boolean`
  - `setCodexGoalMode(configText: string, enabled: boolean): string`
- Codex config field:
  - `[features]`
  - `goals = true`

#### 3. Contracts

- Codex provider presets and the custom Codex template must not force Goal mode
  by default.
- Goal mode is opt-in from the provider editor and is represented only as
  `[features].goals = true` in `config.toml`.
- Disabling Goal mode removes the `goals` line. If `[features]` becomes empty,
  remove that section; if other feature flags or comments remain, keep the
  section intact.
- Goal mode parsing must tolerate temporarily invalid TOML while the user edits
  the textarea by falling back to line scanning.

#### 4. Validation & Error Matrix

- Template contains `goals = true` by default -> reject; users must opt in.
- Disabling Goal mode deletes unrelated `[features]` keys or comments -> reject.
- Invalid TOML in the editor crashes the checkbox render path -> reject; return
  disabled state until parseable or line-scannable.

#### 5. Good/Base/Bad Cases

- Good: user checks Goal mode, `setCodexGoalMode` inserts top-level
  `[features]\ngoals = true` before provider tables.
- Base: existing `[features]` with `experimental_resume = true` keeps that flag
  when Goal mode is disabled.
- Bad: hardcoding `goals = true` in `generateThirdPartyConfig` or
  `getCodexCustomTemplate`.

#### 6. Tests Required

- Unit test `setCodexGoalMode` add/remove behavior, including preserving other
  feature flags/comments.
- Component test that `CodexConfigEditor` toggles Goal mode through the checkbox.
- Preset/template tests asserting Codex templates do not contain forced
  `goals = true`.

#### 7. Wrong vs Correct

##### Wrong

```typescript
const config = `[features]\ngoals = true\n...`;
```

##### Correct

```typescript
const nextConfig = setCodexGoalMode(currentConfig, checked);
```

---

### Scenario: Cache-Normalized Usage API Totals

#### 1. Scope / Trigger

- Trigger: changes to usage summary/trend/detail APIs, session usage sync, pricing recalculation, or Web smoke assertions around cached tokens.
- Applies to Web and desktop callers that read usage totals from `proxy_request_logs` through `src-tauri/src/services/usage_stats.rs`.

#### 2. Signatures

- Summary route:
  - `GET /api/usage/get-usage-summary?appType=<app>`
- Trends route:
  - `GET /api/usage/get-usage-trends?appType=<app>&startDate=<unix>&endDate=<unix>`
- Detail route:
  - `POST /api/system/get_request_detail`
- Smoke verifier:
  - `pnpm smoke:web-server`

#### 3. Contracts

- `proxy_request_logs.input_tokens` stores the raw provider/session payload.
- For Codex and Gemini aggregate totals, `totalInputTokens` is fresh input:
  - `freshInputTokens = max(inputTokens - cacheReadTokens, 0)`
- For Claude/Anthropic aggregate totals, `totalInputTokens` is the raw input because Claude already reports fresh input.
- Summary responses expose cache-normalized derived fields:
  - `realTotalTokens = freshInputTokens + outputTokens + cacheCreationTokens + cacheReadTokens`
  - `cacheHitRate = cacheReadTokens / (freshInputTokens + cacheCreationTokens + cacheReadTokens)`
- Trend responses may expose only the existing daily bucket fields; do not assert `realTotalTokens` or `cacheHitRate` unless the response type explicitly adds them.
- Request detail responses preserve raw log fields. For Codex session sync with `input_tokens=1200` and `cached_input_tokens=300`, detail should still show `inputTokens=1200` while summary/trends show `totalInputTokens=900`.

#### 4. Validation & Error Matrix

- Summary reports Codex/Gemini raw input instead of fresh input -> reject; dashboards overstate paid input.
- Request detail rewrites raw `inputTokens` to fresh input -> reject; drill-down no longer matches the persisted request log.
- Smoke script asserts old raw aggregate input for Codex/Gemini -> update the smoke assertion, not the production aggregate code.
- Smoke script requires fields not present in a route response -> reject the test; route contracts must stay explicit.

#### 5. Good/Base/Bad Cases

- Good: Web smoke seeds Codex session usage `input=1200`, `cacheRead=300`, then asserts summary `totalInputTokens=900`, `realTotalTokens=1650`, `cacheHitRate=0.25`, and detail `inputTokens=1200`.
- Base: daily trends assert `totalInputTokens=900`, `totalOutputTokens=450`, `totalCacheReadTokens=300`, and positive cost.
- Bad: using the same `inputTokens` expectation for summary, trends, and detail.

#### 6. Tests Required

- Run `pnpm smoke:web-server` after changing session usage sync, summary/trends/detail response fields, or cache token aggregation.
- Keep Rust regression coverage for effective usage filters and legacy NULL `data_source` rows:
  - `test_effective_filter_keeps_legacy_null_data_source_proxy_rows`
  - `test_matching_proxy_log_treats_legacy_null_data_source_as_proxy`
- Run `cargo test --manifest-path src-tauri/Cargo.toml --lib` after changing `usage_stats.rs`.

#### 7. Wrong vs Correct

##### Wrong

```javascript
payload.totalInputTokens === smokeUsage.inputTokens;
detail.inputTokens === smokeUsage.freshInputTokens;
```

##### Correct

```javascript
payload.totalInputTokens === smokeUsage.freshInputTokens;
detail.inputTokens === smokeUsage.inputTokens;
```

---

### Scenario: Web Command Route Coverage

#### 1. Scope / Trigger

- Trigger: changes to `src/lib/api/web-commands.ts`, `commands.manifest.json`, Tauri command registration, Web API handlers, or `scripts/check-web-route-coverage.mjs`.
- Applies when adding, removing, or changing a command-to-route contract.

#### 2. Signatures

- Frontend command registry:
  - `defineCommands({ [commandName]: { method, path, unsupported?, webReplacement? } })`
- Route check:
  - `pnpm check:web-routes`
- Web handlers:
  - `.route("/<handler-path>", get|post|put|delete(...))`, mounted under `/api`.

#### 3. Contracts

- Every supported command in `src/lib/api/web-commands.ts` must resolve to:
  - an exact Web route in `src-tauri/src/web_api/handlers/*.rs`, or
  - a non-parity wildcard route only when intentionally covered.
- `unsupported: true` commands are intentionally excluded from route coverage.
- `webReplacement: true` commands are implemented by frontend Web replacement code and are intentionally excluded from route coverage.
- `check-web-route-coverage.mjs` must parse `web-commands.ts` as TypeScript AST, not with line-oriented regex. Generated command entries may be compact or Prettier-expanded.

#### 4. Validation & Error Matrix

- Missing route for supported command -> `pnpm check:web-routes` must fail and print command, method, and path.
- Parser cannot find `defineCommands({...})` -> route check must fail with exit code 2.
- Command missing literal `method` or `path` -> route check must fail with exit code 2.
- Command covered only by parity wildcard -> `--fail-on-parity-fallback` must fail.

#### 5. Good/Base/Bad Cases

- Good: add a Tauri command, add Web handler route, add one `web-commands.ts` entry, then run `pnpm check:web-routes` and verify `missing: 0`.
- Base: mark desktop-only command with `unsupported: true`.
- Bad: relying on a one-line regex over generated TypeScript; Prettier can expand objects and make the check silently undercount commands.

#### 6. Tests Required

- Run `pnpm check:web-routes` after changing command mappings or Web handlers.
- Verify the reported `commands` count is plausible for the current manifest; a sudden drop indicates parser drift even when `missing: 0`.
- For Web-server command additions, also run:
  - `cargo check --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server`

#### 7. Wrong vs Correct

##### Wrong

```javascript
const commandRe =
  /^\\s*([A-Za-z0-9_]+): \\{ method: "([A-Z]+)", path: "([^"]+)"/gm;
```

##### Correct

```javascript
const sourceFile = ts.createSourceFile(
  path,
  source,
  ts.ScriptTarget.Latest,
  true,
);
// Locate defineCommands({...}) and read object literal properties from the AST.
```

---

### Scenario: Web Server Proxy Module Wiring

#### 1. Scope / Trigger

- Trigger: adding a new Rust helper module under `src-tauri/src/proxy/**` that is imported by proxy provider modules.
- Applies when changing `src-tauri/src/proxy/mod.rs`, `src-tauri/src/proxy/providers/mod.rs`, `src-tauri/src/proxy/providers/*.rs`, or `src-tauri/examples/web_proxy.rs`.

#### 2. Signatures

- Desktop/module root:
  - `src-tauri/src/proxy/mod.rs`
- Standalone Web server proxy root:
  - `src-tauri/examples/web_proxy.rs`
- Web compile check:
  - `cargo check --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server`

#### 3. Contracts

- `examples/server.rs` mounts proxy code through `#[path = "web_proxy.rs"] mod proxy;`.
- `examples/web_proxy.rs` is a **1:1 mirror** of `src/proxy/mod.rs` (module set, visibility,
  re-exports) since the web-proxy port — enforced by the example test
  `web_proxy_shim_mirrors_proxy_mod_modules` (desktop-cfg'd modules excluded). Adding a module to
  `src/proxy/mod.rs` without the matching `#[path]` entry fails that test, not just the compile.
- The proxy tree and `services/proxy.rs` must stay tauri-free: runtime needs (event emission, tray
  refresh, OAuth manager access, hot-switch handle) go through `proxy/runtime_ctx.rs::ProxyRuntimeCtx`,
  injected via `ProxyService::set_runtime_ctx` (desktop: `lib.rs` setup after auth managers are
  managed; web: `examples/server.rs` before crash recovery).
- Web runtime proxy listener is loopback-only: `ensure_loopback_listen_address_for_web` rejects
  non-loopback `listen_address` at `ProxyService::start()` AND `update_config()` (cfg!-gated,
  desktop unaffected). Known residual: `update_global_proxy_config` is db-direct and can persist a
  bad address, but every bind funnels through the two enforced entry points.
- Headless lifecycle order in `examples/server.rs` (mirrors desktop `lib.rs`): set_runtime_ctx →
  global-proxy outbound client init (`db.get_global_proxy_url()` → `http_client::init`; invalid
  config cleared + direct fallback, GP-005..GP-008 — must precede takeover restore or forwarding
  GP-004-falls-back to direct and relay providers behind the user's proxy are unreachable) →
  crash recovery → common-config snippets → takeover restore → serve → cleanup. Graceful shutdown
  must stay BOUNDED: infinite SSE connections (`GET /api/events`) block axum's
  `with_graceful_shutdown` forever — the watch-channel + 5s connection-grace race in `main()` is
  load-bearing; removing it re-introduces SIGKILL-with-PROXY_MANAGED-placeholders under systemd.
- Failover strategy is a cross-layer contract: `FailoverStrategy { Sequential (default), Random }`
  in `proxy/types.rs` (serde lowercase, `#[serde(default)]` on `AppProxyConfig.failoverStrategy`),
  DB column `proxy_config.failover_strategy TEXT NOT NULL DEFAULT 'sequential'` (unknown values →
  Sequential via `from_db_str`), FE mirror in `src/types/proxy.ts`. Sequential must stay
  byte-identical to upstream queue order (sync-conflict surface); Random = current-provider-first
  (sticky until failure) + Fisher-Yates shuffle of the remaining circuit-closed pool, implemented
  ONLY in `provider_router.rs::select_providers()` — forwarder/breaker/failover_switch stay
  strategy-agnostic.
- If a provider module imports `crate::proxy::<module_name>`, the module must be available in both:
  - `src-tauri/src/proxy/mod.rs`
  - `src-tauri/examples/web_proxy.rs`

#### 4. Validation & Error Matrix

- Missing desktop module declaration -> normal Rust builds fail with unresolved import.
- Missing `examples/web_proxy.rs` path module -> Web server compile fails even when desktop module wiring is correct.
- Helper module added but no Web compile run -> risk of shipping a Web-only compile break.

#### 5. Good/Base/Bad Cases

- Good: add `src-tauri/src/proxy/json_canonical.rs`, add `pub(crate) mod json_canonical;` in `src-tauri/src/proxy/mod.rs`, add the matching `#[path = "../src/proxy/json_canonical.rs"] pub(crate) mod json_canonical;` in `examples/web_proxy.rs`, then run Web server cargo check.
- Base: modules gated `#[cfg(feature = "desktop")]` in `src/proxy/mod.rs` are the ONLY ones omitted from the shim (the mirror test excludes them); an ungated module must always appear in both.
- Bad: only updating `src-tauri/src/proxy/mod.rs` and assuming `examples/server.rs` sees the same module tree.

#### 6. Tests Required

- Run targeted rustfmt on every touched Rust file.
- Run:
  - `cargo check --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server`

#### 7. Wrong vs Correct

##### Wrong

```rust
// src-tauri/src/proxy/mod.rs only
pub(crate) mod json_canonical;
```

##### Correct

```rust
// src-tauri/src/proxy/mod.rs
pub(crate) mod json_canonical;

// src-tauri/examples/web_proxy.rs
#[path = "../src/proxy/json_canonical.rs"]
pub(crate) mod json_canonical;
```

---

### Scenario: Desktop-Only Service Worker Twin Stubs

#### 1. Scope / Trigger

- Trigger: adding a `services/` background worker that depends on `tauri::AppHandle`/`Emitter` (auto-sync loops, tray refreshers) while its API is called from dual-compiled code (e.g. the database update-hook).
- Applies when changing `src-tauri/src/services/mod.rs`, `src-tauri/examples/web_services.rs`, or adding `services/*_web.rs` stubs. Established by `webdav_auto_sync`/`webdav_auto_sync_web`; repeated by `s3_auto_sync`/`s3_auto_sync_web` (v3.16.2 sync).

#### 2. Signatures

- Desktop worker: `services/<name>.rs` (may use `tauri::{AppHandle, Emitter}`, `app.emit("<event>", …)`).
- Web stub twin: `services/<name>_web.rs` — mirrors the *called-from-shared-code* surface only: `AutoSyncSuppressionGuard::new()`, `is_auto_sync_suppressed()`, no-op `notify_db_changed(_table: &str)`.

#### 3. Contracts

- `services/mod.rs` cfg-pair:
  `#[cfg(feature = "desktop")] pub mod <name>; #[cfg(not(feature = "desktop"))] pub mod <name>_web; #[cfg(not(feature = "desktop"))] pub use <name>_web as <name>;`
- `examples/web_services.rs` includes the stub via `#[path = "../src/services/<name>_web.rs"]` so the web example resolves `services::<name>::…` call sites unchanged.
- The stub MUST stay tauri-free; the desktop worker is the only emitter of its status event (web mode simply never fires it — FE listens runtime-neutrally via `event-adapter`).
- **FE control gating (audit F10)**: if the desktop-only worker exposes a user-facing
  control (e.g. an auto-sync toggle), the web frontend MUST gate it in web mode
  (`isWebMode()` from `src/lib/api/adapter.ts`): disable the control and show a hint
  that the feature is desktop-only / manual-only, because the web `_web` stub is a
  no-op. Leaving it active misleads users into believing the background work runs.
  Manual equivalents (explicit upload/download/test) stay ungated; desktop is
  unchanged. Established by `WebdavSyncSection.tsx` gating the WebDAV/S3 auto-sync
  switches with `settings.{webdavSync,s3Sync}.autoSyncWebDisabledHint` (i18n in
  en/ja/zh).

#### 4. Validation & Error Matrix

- Shared call site (e.g. `database/mod.rs` update-hook) references `services::<name>` but no stub/cfg-pair -> web example compile fails.
- Stub added but `web_services.rs` include missing -> web example compile fails; `dual_runtime_parity::web_services_shim_covers_web_cfg_gated_service_modules` also fails.
- Stub grows real behavior that emits Tauri events -> reject (web build must not depend on tauri).

#### 5. Good/Base/Bad Cases

- Good: `s3_auto_sync.rs` (desktop worker, emits `s3-sync-status-updated`) + `s3_auto_sync_web.rs` no-op twin + cfg-pair + shim include.
- Base: a worker with no shared-code callers needs no twin — gate the whole module `#[cfg(feature = "desktop")]`.
- Bad: making the shared caller itself cfg-gated to dodge the stub (forks the call-site logic between runtimes).

#### 6. Tests Required

- `cargo check --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server`
- `cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server -- dual_runtime_parity::`
- **Desktop-intent FE component tests must pin desktop mode (audit P4-C)**: a component
  whose web-mode gating depends on `isWebMode()` (e.g. `WebdavSyncSection`'s auto-sync
  toggle/error panels) needs `window.__TAURI_INTERNALS__` + `window.__TAURI__` set in
  `beforeEach`. Plain jsdom has NO Tauri globals, so `isWebMode()` defaults to `true` and the
  F10 gating hides those controls — a desktop-behavior assertion would fail without the pin.

#### 7. Wrong vs Correct

##### Wrong

```rust
// database/mod.rs — runtime fork at the call site
#[cfg(feature = "desktop")]
crate::services::s3_auto_sync::notify_db_changed(table);
```

##### Correct

```rust
// services/mod.rs cfg-pair + no-op stub twin; call site stays unconditional:
crate::services::s3_auto_sync::notify_db_changed(table);
```

---

### Scenario: Upstream Desktop Sync Into Web Fork

#### 1. Scope / Trigger

- Trigger: porting changes from upstream `farion1231/cc-switch` desktop releases into this Web-first fork.
- Applies when syncing version metadata, proxy/provider backend behavior, Web API command surfaces, or frontend app/provider flows from upstream.

#### 2. Signatures

- Version metadata:
  - `package.json` -> `"version"`
  - `src-tauri/Cargo.toml` -> `[package].version`
  - `src-tauri/Cargo.lock` -> `[[package]] name = "cc-switch"` version
  - `src-tauri/tauri.conf.json` -> `"version"`
- Web-only runtime boundaries:
  - `src-tauri/examples/server.rs`
  - `src-tauri/src/web_api/**`
  - `src-tauri/src/runtime/**`
  - `src-tauri/src/bootstrap.rs`
  - `src/lib/api/web-commands.ts`
  - service/deploy scripts under `deploy/` and `scripts/`

#### 3. Contracts

- Do not blind-merge or replace the Web fork with upstream desktop sources.
- Compare upstream release content against the previous upstream tag, then port targeted slices into the Web fork.
- Preserve Web-only runtime files even when direct diffs against upstream show them as deletions.
- Keep Web adapter imports such as `src/lib/api/model-fetch.ts` using the local Web adapter when they are required for browser/server mode.
- Bump version metadata only after the relevant upstream behavior for the release has been ported and verified.
- Desktop-only surfaces, such as Claude Desktop UI parity, may be explicitly deferred if the Web fork does not expose them yet; record the deferral in the task PRD/research instead of deleting Web behavior to match upstream.

#### 4. Validation & Error Matrix

- Missing Web-only runtime file after sync -> reject the sync patch before commit.
- Version mismatch across metadata files -> reject the version bump.
- New or changed Tauri command without Web route coverage -> `pnpm check:web-routes` must fail.
- Backend/proxy/provider sync compiles for desktop but not `web-server` -> reject until Web server cargo check passes.
- Upstream desktop import replaces Web adapter import -> reject unless the route has an equivalent Web implementation and tests.

#### 5. Good/Base/Bad Cases

- Good: port proxy/provider changes in focused patches, adapt Web API handlers when command surfaces change, run Web route coverage and Web-server cargo check, then bump all version files together.
- Base: preserve an upstream desktop-only feature as a documented deferral while still porting shared backend fixes that benefit Web-managed apps.
- Bad: checking out upstream `src-tauri/` wholesale, deleting `web_api` or `examples/server.rs`, then fixing compile errors reactively.

#### 6. Tests Required

- Run `pnpm check:web-routes` after command, API, or handler changes.
- Run `pnpm typecheck` after frontend or API adapter changes.
- Run focused Rust tests for changed proxy/provider/config modules.
- Make runtime mode explicit in tests:
  - Tauri/desktop tests set `window.__TAURI_INTERNALS__` and `window.__TAURI__`, then clear `window.__CC_SWITCH_API_BASE__`.
  - Real Web server tests clear the Tauri globals and set `window.__CC_SWITCH_API_BASE__` to the spawned server URL.
- When a real Web server test asserts fake CLI binaries on `PATH`, pass a controlled env such as `SHELL=sh` with the test `PATH`; otherwise login-shell startup can rewrite `PATH` before backend tool detection runs.
- After Web API mutations, wait for the rendered UI/query state to refresh before selecting DOM rows; backend state alone does not prove React Query has re-rendered.
- Run:
  - `cargo check --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server`
- Run `git diff --check` on the files included in the sync patch before final review.

#### 7. Wrong vs Correct

##### Wrong

```bash
git merge upstream-v3.15.0
```

##### Correct

```bash
git diff v3.14.1..upstream-v3.15.0 -- <focused-area>
# Port the required behavior manually, preserve Web-only files, then run Web checks.
```

---

### Scenario: Standalone Web-Server Smoke Validation

#### 1. Scope / Trigger

- Trigger: validating Web API behavior after changes to Web routes, server runtime,
  provider/config persistence, session/usage flows, or deployment-facing server
  behavior.
- Applies when using `scripts/smoke-web-server.mjs`,
  `src-tauri/examples/server.rs`, `src-tauri/src/web_api/**`, or
  `src/lib/api/web-commands.ts`.

#### 2. Signatures

- Web build command:
  - `pnpm build:web`
- Smoke command:
  - `pnpm smoke:web-server`
- Server entry:
  - `cargo run --no-default-features --features web-server --example server`
- Required smoke environment:
  - `CC_SWITCH_DATA_DIR`
  - `CC_SWITCH_TEST_HOME`
  - `CC_SWITCH_WEB_DIST_DIR`

#### 3. Contracts

- `pnpm smoke:web-server` requires `dist-web/index.html`; run
  `pnpm build:web` first when the artifact is missing or stale.
- Smoke runs must use isolated temp data/home directories through
  `CC_SWITCH_DATA_DIR` and `CC_SWITCH_TEST_HOME`; do not point the smoke server
  at a developer's real CLI configuration. The spawn env MUST also isolate
  `HOME` + `USERPROFILE` + `XDG_DATA_HOME` + `XDG_CONFIG_HOME` (not just
  `CC_SWITCH_TEST_HOME`): session scanners reached via the F6 startup bootstrap
  (e.g. `session_manager/providers/opencode.rs`) resolve through
  `dirs::home_dir()`/XDG, so without this the smoke server reads the developer's
  real `~/.local/share/opencode/opencode.db` (mirror the Rust example `TempHome`).
- `CC_SWITCH_WEB_DIST_DIR` must point at the built Web bundle that should be
  served by the standalone server.
- The smoke result should be interpreted by probe expectations, not by status
  code alone. Desktop-only endpoints returning `501` and validation probes
  returning `400` can be correct when the probe expects those statuses.
- A smoke task should leave business-code files unchanged unless it uncovered a
  real defect that is being fixed in a separate task.

#### 4. Validation & Error Matrix

- Missing `dist-web/index.html` -> build Web assets before smoke testing.
- Server exits before `/api/health` responds -> investigate server startup,
  feature flags, and runtime environment before changing product code.
- Smoke mutates files outside the isolated temp directories -> reject the run and
  fix the smoke setup.
- Probe returns an unexpected status or payload -> categorize as product defect,
  test fixture defect, or environment flake before mixing repairs into the smoke
  task.
- Web route coverage has `missing > 0` after command/route edits -> reject until
  `pnpm check:web-routes` passes.

#### 5. Good/Base/Bad Cases

- Good: build Web assets when needed, run `pnpm smoke:web-server`, confirm the
  standalone server starts on localhost, then verify the working tree has no
  business-code changes.
- Base: after frontend-only changes that do not touch routes or server runtime,
  run `pnpm typecheck` and targeted unit tests; smoke can be deferred unless the
  PRD requires standalone server validation.
- Bad: relying only on desktop Tauri checks after Web API handler changes.
- Bad: running the server against a real `$HOME` and treating mutated personal
  config files as smoke fixtures.

#### 6. Tests Required

- Run `pnpm check:web-routes` after Web command, adapter, route, or handler
  changes.
- Run `pnpm typecheck` after frontend or API adapter changes.
- Run `pnpm smoke:web-server` for standalone Web API/server validation.
- If `dist-web/index.html` is missing or stale, run `pnpm build:web` before the
  smoke command.
- After smoke, run `git status --short` and confirm only intended task or
  generated artifacts changed.

#### 7. Wrong vs Correct

##### Wrong

```bash
cargo run --no-default-features --features web-server --example server
# Then manually click around using the developer's real HOME/config files.
```

##### Correct

```bash
pnpm build:web
pnpm smoke:web-server
git status --short
```

---

### Scenario: GitHub Actions Runtime Compatibility

#### 1. Scope / Trigger

- Trigger: GitHub Actions reports that an action is running on a deprecated
  runtime such as Node.js 20, or a workflow action has an available major version
  bump already adopted upstream.
- Applies when editing `.github/workflows/*.yml` action `uses:` entries.

#### 2. Signatures

- CI workflow:
  - `.github/workflows/ci.yml`
- Release workflow:
  - `.github/workflows/release.yml`
- Stale issue workflow:
  - `.github/workflows/stale.yml`
- Current expected action major versions:
  - `actions/checkout@v6`
  - `actions/setup-node@v6`
  - `pnpm/action-setup@v6`
  - `actions/cache@v5`
  - `actions/stale@v10`
  - `softprops/action-gh-release@v3`

#### 3. Contracts

- Fix runtime deprecation by upgrading the action major version, not by setting
  environment variables that suppress or bypass GitHub runner warnings.
- Do not change the app runtime version, such as `node-version: "20"`, unless the
  task explicitly targets application runtime migration.
- Preserve workflow triggers, permissions, job names, matrix entries, shell
  commands, cache keys, release artifact names, and stale issue policy unless the
  task explicitly changes those behaviors.
- Verify target action tags exist before pinning a new major version.
- Keep the change scoped to workflow metadata when the goal is runtime
  compatibility; product code should remain untouched.

#### 4. Validation & Error Matrix

- Workflow still references a deprecated action major after the edit -> reject.
- Target action tag does not exist -> reject and choose a supported major.
- App runtime changes without an explicit runtime migration PRD -> reject.
- YAML no longer parses -> reject before pushing.
- CI succeeds but still reports the same deprecated action annotation -> inspect
  all workflows and jobs for remaining older `uses:` entries.

#### 5. Good/Base/Bad Cases

- Good: upgrade `actions/checkout@v4` to `actions/checkout@v6` in every active
  workflow that still references v4, then confirm PR CI passes.
- Base: keep `node-version: "20"` in build/test jobs when the application still
  supports Node 20 and the warning only concerns action runtime.
- Bad: adding `ACTIONS_ALLOW_USE_UNSECURE_NODE_VERSION=true` to silence an action
  runtime warning.
- Bad: upgrading release workflow commands and artifact names while only trying
  to update action runtimes.

#### 6. Tests Required

- Verify target tags with `git ls-remote --tags <action-repo> refs/tags/v<major>`.
- Parse edited workflow YAML locally with an available YAML parser.
- Run `git diff --check`.
- Run `pnpm typecheck`, `pnpm format:check`, and `pnpm test:unit` for PR CI
  parity.
- Push and watch the PR CI run; confirm frontend and backend jobs pass and the
  old action-runtime annotation is gone.

#### 7. Wrong vs Correct

##### Wrong

```yaml
env:
  ACTIONS_ALLOW_USE_UNSECURE_NODE_VERSION: true
```

##### Correct

```yaml
- name: Checkout
  uses: actions/checkout@v6
```

---

### Scenario: Web API Authentication (HTTP Basic) — audit C2

#### 1. Scope / Trigger

- Trigger: changing the web router assembly, the auth middleware, the server bind
  gate, or the systemd/install deploy of the standalone web server.
- Applies to `web_api/routes.rs`, `web_api/middleware/auth.rs`,
  `examples/server.rs`, `deploy/systemd/cc-switch-web.service`,
  `scripts/install-cc-switch-web-service.sh`.

#### 2. Signatures

- `web_api/middleware/auth.rs::require_auth(req, next) -> Response`
- `web_api/middleware/auth.rs::is_configured() -> bool`
- Env: `CC_SWITCH_WEB_AUTH_PASSWORD` (enables auth), `CC_SWITCH_WEB_AUTH_USER`
  (default `cc-switch`).

#### 3. Contracts

- `require_auth` is layered in `build_router` and enforces HTTP Basic on `/api/*`
  EXCEPT exactly `/api/health` (exact equality) and non-`/api/` static SPA assets.
- Auth is enabled iff `CC_SWITCH_WEB_AUTH_PASSWORD` is non-empty; the credential
  comparison is constant-time (`ct_eq`).
- `examples/server.rs` REFUSES a non-loopback bind unless `auth::is_configured()`.
- Static SPA assets stay public so the browser loads the app, then an `/api` 401
  with `WWW-Authenticate: Basic` triggers the native prompt — no frontend change.
- `ALLOW_HTTP_BASIC_OVER_HTTP` is REMOVED (a misnomer — no auth existed). The
  password ships in a `0600` systemd drop-in; install uses `restart`, not
  `enable --now` (the latter does not restart an already-running unit).
- **CSRF + rate-limit are intentionally ABSENT (audit P4-C)**: the prior `middleware/
  {csrf,rate_limit}.rs` stubs, the `/system/csrf-token` endpoint, and the FE adapter
  X-CSRF token plumbing were DELETED, not wired. HTTP Basic over the encrypted
  Tailscale tunnel is the control; cookie-CSRF is not the threat model (Basic creds
  aren't ambient like cookies; the default CORS is same-origin). Do NOT re-add CSRF
  token plumbing or advertise CSRF/rate-limit/cookie-session in docs. Mutating `/api`
  calls `fetch` directly with `credentials: "include"`.
- **Residual authenticated-only vectors (06-15 follow-up)**: with C2 the whole `/api`
  is auth-gated, so the remaining "write→exec"/"file-read" surfaces are OPERATOR-ONLY
  (the single authenticated user configuring their own machine):
  - `get-session-messages?sourcePath=` now root-guards reads exactly like
    `delete_session` (`session_manager::load_messages` resolves the provider's session
    roots, canonicalizes, and rejects out-of-root paths; the `sqlite:<db>` form must be
    the provider's own DB) — out-of-root reads are rejected before any bytes.
  - `POST /api/mcp/upsert-mcp-server` writes an MCP stdio command that runs on the next
    CLI launch. This is the INTENDED operator feature; ACCEPTED post-C2 (no confirmation
    gate). If the tailnet ever becomes multi-user, reconsider a confirmation gate or
    command allowlist.
  - OAuth token files are written `0o600` on Linux; encryption-at-rest is out of scope
    for a single-user host (key-management cost > benefit when 0o600 already gates other
    local users).
- **Body-logging is privacy-by-default (06-15 follow-up)**: full request/response bodies
  (prompts + model outputs) and SSE data are logged ONLY at `log::debug!`
  (`proxy/forwarder.rs`, `proxy/response_processor.rs`). The shipped systemd unit defaults
  to `RUST_LOG=info` so bodies are NOT journaled in plaintext; operators opt into
  `cc_switch=debug` temporarily. `info!`/`warn!`/`error!` may carry metadata (model,
  provider id, counts) and a ≤180-char summary of upstream HTTP ERROR responses
  (`summarize_upstream_body`) — never the prompt or normal model output. Do NOT raise a
  prompt/response BODY log above debug.

#### 4. Validation & Error Matrix

- A secret/mutating `/api` route reachable without valid creds while a password is
  set -> reject.
- The exemption widened to a prefix/`starts_with` match (e.g. `/api/healthz`) ->
  reject; it must be exact `/api/health`.
- Non-loopback bind starts with no password configured -> reject (server must
  refuse to listen).
- A change that requires the frontend to send credentials -> reject; Basic is
  browser-native.

#### 5. Good/Base/Bad Cases

- Good: with the password set, `/api/providers` without creds returns 401, with
  creds returns 200; `/api/health` and the SPA bundle stay public.
- Base: loopback dev with no password -> middleware is a no-op (open) — permissible
  ONLY because `server.rs` forbids that combination on a non-loopback bind.
- Bad: re-introducing `ALLOW_HTTP_BASIC_OVER_HTTP` as the non-loopback gate, or
  exempting `/api/*` beyond `/api/health`.

#### 6. Tests Required

- `web_api::middleware::auth::tests` (`ct_eq_basic`, `credentials_match_*`).
- Web server cargo check.

#### 7. Wrong vs Correct

##### Wrong

```rust
let is_public = !path.starts_with("/api/") || path.starts_with("/api/health");
```

##### Correct

```rust
let is_public = !path.starts_with("/api/") || path == "/api/health";
```

---

### Scenario: Web Request Hardening — Path Traversal + Outbound SSRF (audit C1/F3/F4/F11)

#### 1. Scope / Trigger

- Trigger: changing the SPA static-serve fallback, or adding/altering any web
  handler that dials a user-influenced outbound URL.
- Applies to `web_api/routes.rs` (`try_serve_dist_web_asset`),
  `web_api/handlers/common.rs` (`validate_outbound_url`), and any handler that
  reaches the network (`config`, `system`, `subscription`, `webdav`, `s3`, …).

#### 2. Signatures

- `routes.rs::is_safe_relative_asset(rel: &str) -> bool`
- `common.rs::validate_outbound_url(raw: &str) -> Result<(), ApiError>` (**async**)

#### 3. Contracts

- The hand-rolled SPA static server must gate EVERY disk read behind
  `is_safe_relative_asset(rel)` (only `Component::Normal`/`CurDir` allowed; rejects
  `..`/RootDir/Prefix) BEFORE `dist_root.join`. `uri.path()` is NOT
  dot-segment-normalized, so an unguarded `join` escapes the asset root.
- `validate_outbound_url` is `async` and resolves DNS via non-blocking
  `tokio::net::lookup_host` (never the blocking `std` resolver on an async task);
  ALL callers must `.await`.
- It must be applied on EVERY outbound-dial web handler: webdav (4 handlers), s3 (4;
  skipping an empty endpoint = AWS default), subscription `get_balance` +
  `get_coding_plan_quota` (2), config fetch-models (2). The shared sync/service
  layer stays unrestricted for desktop (the guard is web-handler-only).
- `/system/test_api_endpoints` caps `urls` at 50 (clean 400 over the cap).
- **Call-site coverage (audit P4-A1)**: the guard ALSO covers `usage.rs::test_usage_script`
  (guards `request.base_url` only when `Some(non-empty)`) and BOTH stream-check variants in
  `providers.rs` via `services/stream_check.rs::resolve_outbound_base_url` (mirrors the exact
  per-app base-url the dial uses; `_all_` records a per-provider Failed result on rejection). Any
  NEW handler dialing a user-influenced URL MUST call the guard.
- **IP classifier SSOT (audit P4-A3)**: the block-list lives in the tauri-free
  `proxy/ip_guard.rs` (`is_blocked_ip`/`_ipv4`/`_ipv6`), re-exported by `common.rs` and mirrored
  in `examples/web_proxy.rs`. Blocks loopback, link-local, private, ULA, **unspecified
  (`0.0.0.0`/`::`/`::ffff:0.0.0.0`)**, and **CGNAT `100.64.0.0/10`**. Add a range in `ip_guard.rs`
  only.
- **Redirect guard (audit P4-A2)**: user-URL web services (`services/{balance,coding_plan,
  model_fetch}.rs`) dial via `proxy/http_client.rs::get_guarded()` — a SECOND client whose
  `reqwest::redirect::Policy` re-runs `ip_guard::is_blocked_ip` on EACH redirect hop's IP-literal
  host and aborts on internal (public→public still follows ≤10 hops). The shared proxy forwarder
  keeps the UNGUARDED `get()` (upstream-3xx pass-through unchanged). `init`/`apply_proxy`/
  `update_proxy` rebuild BOTH clients in lock-step. Residual (deferred-ok): a redirect to a DOMAIN
  resolving internal is not caught (sync callback can't DNS — same rebinding class as the guard).
- **S3 schemeless normalization (audit P4-B)**: `s3.rs::guard_s3_endpoint` runs
  `normalize_s3_endpoint_for_guard` (bare `host:port` → `https://host:port`, mirroring
  `split_scheme_host`) BEFORE validating, else `Url::parse` mis-reads the bare host as the scheme
  and 400s a valid MinIO endpoint. Explicit `http(s)://` is preserved (no bypass).

#### 4. Validation & Error Matrix

- A static read reachable with a `..` component (raw or that a future change
  un-gates) -> reject.
- A new outbound handler that dials a user URL without `validate_outbound_url` ->
  reject.
- A blocking `to_socket_addrs` reintroduced on an async path -> reject.
- A test that relies on a NON-resolvable host to reach a downstream "unknown
  provider" branch -> the guard now 400s it first; use a resolvable public host.

#### 5. Good/Base/Bad Cases

- Good: `/../../../../etc/passwd` falls back to the SPA index (no host file read);
  webdav/s3/subscription dials to a private/loopback target are 400-rejected.
- Base: an empty S3 endpoint (AWS default, public) skips the guard intentionally.
- Bad: adding a `models_url`/`base_url`/`endpoint` dial without the guard, or
  serving `dist_root.join(rel)` before the traversal check.

#### 6. Tests Required

- `web_api::routes::tests::{is_safe_relative_asset_rejects_traversal,
  path_traversal_does_not_read_outside_dist_root}`.
- `web_api::handlers::common` SSRF guard tests.
- Web server cargo check + `pnpm smoke:web-server` (probes use resolvable hosts).

#### 7. Wrong vs Correct

##### Wrong

```rust
let candidate = dist_root.join(rel);
if let Some(resp) = read_dist_web_file(&candidate).await { return resp; }
```

##### Correct

```rust
if is_safe_relative_asset(rel) {
    let candidate = dist_root.join(rel);
    if let Some(resp) = read_dist_web_file(&candidate).await { return resp; }
}
```

---

### Scenario: Dual-Runtime Startup Bootstrap & Legacy Migration (audit F5/F6)

#### 1. Scope / Trigger

- Trigger: changing app startup — DB init ordering, the `config.json`→SQLite
  migration, or the post-DB import/seed of providers/MCP/prompts/skills/OMO.
- Applies to `src/bootstrap.rs`, `src/lib.rs` (desktop setup), and
  `examples/server.rs` (web main).

#### 2. Signatures

- `bootstrap::apply_legacy_json_migration(db, config, json_path)` (tauri-free)
- `bootstrap::run_post_db_bootstrap(app_state: &AppState)` (tauri-free)

#### 3. Contracts

- Both runtimes call BOTH functions: desktop `lib.rs` and web `examples/server.rs`.
  `bootstrap.rs` MUST stay tauri-free (it is `#[path]`-included by the web example).
- The JSON LOAD step stays per-caller: desktop wraps it in the dialog/retry/
  `process::exit(1)` loop; web logs-and-continues on load failure (the headless
  server must still come up). Do not move the dialog/exit into `bootstrap.rs`.
- Web migration MUST run BEFORE `Database::init()` (otherwise `!db_path.exists()` is
  already false and migration is skipped — the F5 bug).
- `run_post_db_bootstrap` runs after `AppState::new` and BEFORE `set_runtime_ctx`
  (i.e. before the proxy lifecycle, matching desktop). It must not disturb the order
  pinned by `web_proxy_lifecycle::main_pins_proxy_lifecycle_ordering`.
- Every bootstrap step stays idempotent (table-empty / flag gated) so it re-runs
  safely on each systemd boot — the smoke fixture depends on this (explicit
  re-import endpoints report 0 newly-imported after startup already imported).
- A helper the shared bootstrap needs (e.g.
  `services::provider::should_import_default_config_on_startup`) must be re-exported
  in BOTH runtimes, not desktop-gated.

#### 4. Validation & Error Matrix

- Web `Database::init()` runs before the legacy-JSON check -> reject (migration
  dead).
- `bootstrap.rs` references `tauri`/`AppHandle`/`Emitter` -> reject (web build
  breaks).
- A non-idempotent step added to `run_post_db_bootstrap` -> reject (re-runs each
  boot).
- Bootstrap moved after `set_runtime_ctx` / into the proxy lifecycle -> reject.

#### 5. Good/Base/Bad Cases

- Good: a fresh web install seeds official providers + imports live config/MCP/
  prompts/skills exactly as desktop; a legacy `config.json` migrates then archives
  to `config.json.migrated`.
- Base: desktop behaviour is byte-for-byte unchanged (pure extraction + dialog stays).
- Bad: duplicating or removing `initialize_common_config_snippets` (a SEPARATE
  existing step), or making `run_post_db_bootstrap` desktop-only.

#### 6. Tests Required

- Desktop `cargo test --lib` (migration/provider/mcp areas) unchanged-green.
- Web `dual_runtime_parity::` + `web_proxy_lifecycle::` tests.
- `pnpm smoke:web-server` (startup import idempotency).

#### 7. Wrong vs Correct

##### Wrong

```rust
// examples/server.rs
let db = Arc::new(database::Database::init()?); // creates the DB first
// ... legacy config.json now ignored forever
```

##### Correct

```rust
let needs_migration = !db_path.exists() && json_path.exists();
let cfg = needs_migration.then(|| MultiAppConfig::load().ok()).flatten();
let db = Arc::new(database::Database::init()?);
if let Some(cfg) = cfg { bootstrap::apply_legacy_json_migration(&db, &cfg, &json_path); }
```

---

### Scenario: Failover Circuit-Breaker Bypass Policy (audit F7)

#### 1. Scope / Trigger

- Trigger: changing how the forwarder decides to consult vs bypass the circuit
  breaker, or the failover provider-selection plumbing.
- Applies to `proxy/forwarder.rs`, `proxy/handler_context.rs`, `proxy/handlers.rs`.

#### 2. Signatures

- `forwarder.rs::should_bypass_circuit_breaker(failover_enabled: bool) -> bool`
- `forward_with_retry[_inner](.., failover_enabled: bool, providers: Vec<Provider>)`
- `handler_context.rs::RequestContext::failover_enabled() -> bool`

#### 3. Contracts

- The breaker bypass is `!failover_enabled`, NOT `providers.len() == 1`. With
  failover OFF the current-provider-only path bypasses the breaker (don't block the
  user's sole provider); with failover ON the breaker + half-open probing applies
  even when only one provider is currently available.
- `failover_enabled` is sourced from `AppProxyConfig.auto_failover_enabled` (the
  same value `provider_router::select_providers` reads) and plumbed to ALL five
  handler call sites (messages, chat_completions, responses, responses_compact,
  gemini) — no app family may diverge.

#### 4. Validation & Error Matrix

- Bypass keyed on `providers.len() == 1` -> reject (an Open-past-timeout sole
  failover provider skips half-open limiting).
- `failover_enabled` passed at some call sites but not others -> reject.
- A second DB read added just to learn the flag -> reject; reuse the per-request
  `AppProxyConfig` on `RequestContext`.

#### 5. Good/Base/Bad Cases

- Good: failover ON + one available provider whose breaker is Open -> breaker is
  consulted (probe-limited), not bypassed.
- Base: failover OFF + single provider -> breaker bypassed (preserved behaviour).
- Bad: inferring "failover off" from the selected list length.

#### 6. Tests Required

- `forwarder.rs` regression tests both directions
  (`single_available_provider_with_failover_on_still_consults_breaker`,
  `single_provider_with_failover_off_bypasses_open_breaker`).
- Desktop proxy tests + web cargo check (forwarder is shared).

#### 7. Wrong vs Correct

##### Wrong

```rust
let bypass_circuit_breaker = providers.len() == 1;
```

##### Correct

```rust
let bypass_circuit_breaker = should_bypass_circuit_breaker(failover_enabled); // = !failover_enabled
```

---

### Scenario: Failover Enable Parity, Proxy Start/Stop Safety & Config Reuse (audit Phase 3)

#### 1. Scope / Trigger

- Trigger: changing how auto-failover is enabled/disabled, how the proxy server
  starts/stops, or the per-request provider-selection config reads.
- Applies to `services/proxy.rs`, `commands/failover.rs`,
  `web_api/handlers/failover.rs`, `proxy/provider_router.rs`,
  `proxy/handler_context.rs`.

#### 2. Signatures

- `ProxyService::set_auto_failover_enabled(app_type, enabled) -> Result<_, String>`
- `ProxyService` field `start_stop_lock: Arc<tokio::sync::Mutex<()>>`
- `ProviderRouter::select_providers_with_config(app_type, auto_failover_enabled, failover_strategy)`

#### 3. Contracts

- **F9 (failover-enable SSOT, cross-runtime)**: `set_auto_failover_enabled` is the
  single tauri-free SSOT; BOTH `commands/failover.rs` (desktop) and
  `web_api/handlers/failover.rs` (web) delegate to it.
  - `enabled=true` + EMPTY queue auto-adds the current provider as P1 (errors only
    if there is no current provider), writes the flag, `switch_proxy_target`s to P1,
    then emits `provider-switched` via the injected `UiEventSink` (desktop = Tauri
    bus, web = `ChannelEventSink` SSE).
  - The tray is refreshed on BOTH enable AND disable; moving `refresh_tray()` inside
    `if enabled` is a desktop-parity regression.
  - `enabled=false` only flips the flag and KEEPS the queue (no switch, no event).
  - The method must stay tauri-free (no `tauri::`/`Emitter`/`AppHandle`); the web
    handler maps `Result<_, String>` via `ApiError::from_service_message`.
  - A failover queue P1 must be a third-party provider — switching to an OFFICIAL
    provider is rejected by `hot_switch_provider_inner`.
- **F8 (start/stop serialization)**: `start_stop_lock` is acquired at the top of
  `start()` and `stop()` to serialize check→bind→set / check→take. It is
  NON-reentrant: no path may call `self.start()`/`self.stop()` while holding it
  (internal callers call them sequentially; inner restarts use
  `ProxyServer::{start,stop}` on the inner instance). `update_config` does NOT take
  it (serialized via the `server` RwLock). The post-acquire running-state
  double-check must be preserved.
- **M1 (per-request config reuse)**: the forward hot path calls
  `select_providers_with_config` reusing the `AppProxyConfig` already loaded in
  `RequestContext::new`, NOT `select_providers` (which re-reads `proxy_config`). Both
  must produce byte-identical candidate lists for the same config (read-dedup only).
  Known deferral: `record_result`'s `circuit_failure_threshold` re-read.

#### 4. Validation & Error Matrix

- Web and desktop failover-enable diverge (web rejects empty queue instead of
  auto-adding current) -> reject; both must go through `set_auto_failover_enabled`.
- `refresh_tray()` moved inside `if enabled` -> reject (disable path stops refreshing).
- A re-entrant `start()`/`stop()` call while holding `start_stop_lock` -> reject
  (deadlock; tokio Mutex is not reentrant).
- `select_providers_with_config` diverging from `select_providers` for equal config
  -> reject.

#### 5. Good/Base/Bad Cases

- Good: enabling failover on an empty queue (web or desktop) auto-adds the current
  third-party provider as P1, switches to it, and fires `provider-switched` (SSE in
  web); 8 concurrent `start()` calls bind exactly one listener.
- Base: disabling failover flips the flag, keeps the queue, refreshes the tray, no event.
- Bad: re-implementing enable semantics separately in the web handler, or keying the
  breaker bypass / a second config read off the request path again.

#### 6. Tests Required

- `services/proxy.rs` F8 concurrent-start test + F9 enable-on-empty-queue test.
- `provider_router.rs` `select_providers_with_config` parity test.
- Desktop `cargo test --lib` + web `dual_runtime_parity::`/`web_proxy_lifecycle::`.
- `pnpm smoke:web-server` failover probes model desktop-equivalent enable (200 on
  empty queue via auto-add; non-official P1).

#### 7. Wrong vs Correct

##### Wrong

```rust
// web handler: separate, drifting semantics
if request.enabled && queue.is_empty() { return Err(bad_request("add a provider first")); }
config.auto_failover_enabled = request.enabled; // flip only
```

##### Correct

```rust
// both runtimes delegate to the shared tauri-free SSOT
state.app_state.proxy_service
    .set_auto_failover_enabled(&request.app_type, request.enabled).await
    .map_err(ApiError::from_service_message)?;
```

---

### Scenario: Web Outbound SSRF — Usage request.url, Redirect Hop, Log Privacy, CSRF/CORS, F9 Atomicity, Background Workers (verification round, scope C)

#### 1. Scope / Trigger

- Trigger: changing usage-script outbound dialing, any user-URL service dial,
  upstream-error logging, the failover-enable path, the web auth middleware,
  the headless server startup, or the shared IP block-list.
- Applies to `usage_script.rs`, `services/provider/usage.rs`,
  `services/provider/mod.rs`, `commands/provider.rs`,
  `web_api/handlers/{usage,providers}.rs`, `services/{stream_check,webdav,s3,
  speedtest}.rs`, `proxy/handlers.rs`, `services/proxy.rs`,
  `web_api/middleware/auth.rs`, `examples/server.rs`, `proxy/ip_guard.rs`,
  `web_api/handlers/common.rs`.

#### 2. Signatures

- `proxy::ip_guard::guard_outbound_url(raw: &str) -> Result<(), OutboundUrlError>`
  (async; tauri-free SSOT) + `is_blocked_ipv4/_ipv6/_ip` + `ssrf_host_allowed`.
- `usage_script::{execute_usage_script, send_http_request}(.., enforce_outbound_guard: bool)`.
- `ProviderService::{query_usage, query_usage_with_templates, test_usage_script}(.., enforce_outbound_guard: bool)`.
- `proxy/handlers.rs::compact_error_message(message, max_chars)`.
- `web_api/middleware/auth.rs::{require_auth, check_same_origin_intent, is_state_changing}`.
- `examples/server.rs::spawn_background_workers(state)` + `run_session_usage_sync`.

#### 3. Contracts

- **Usage request.url SSRF (FIX 1)**: secret/user-influenced usage dials must
  validate the ACTUALLY-DIALED `request.url` (not just `base_url`) in the WEB
  runtime via `guard_outbound_url` BEFORE dialing, then dial through
  `http_client::get_guarded()`. Thread `enforce_outbound_guard: bool` from the
  two web handlers (`test_usage_script`, `query_provider_usage` →
  `query_usage_with_templates`) down to `send_http_request`; desktop callers
  pass `false` (unchanged). `usage_script.rs` stays tauri-free (it imports
  `proxy::ip_guard`, never `web_api`).
- **SSRF SSOT (FIX 1)**: `guard_outbound_url` is the single tauri-free guard;
  `web_api/handlers/common.rs::validate_outbound_url` DELEGATES to it (maps
  `OutboundUrlError` → `ApiError`), and the usage-script path maps it →
  `AppError`. Do not re-implement the parse/scheme/DNS/block logic in two places.
- **Redirect hardening (FIX 2)**: every user-URL web-reachable service dial uses
  `http_client::get_guarded()` (per-hop IP recheck), NOT `get()`. Covered:
  `services/{stream_check,webdav,s3,speedtest}.rs`. The proxy hot-path
  (`forwarder`) keeps the unguarded `get()` (upstream-3xx pass-through unchanged).
- **Log-DB privacy (FIX 3)**: the persisted request-log `error_message` is
  bounded — `log_forward_error` wraps `get_error_message(error)` in
  `compact_error_message(.., 400)` before `log_error_with_context`. Upstream
  error bodies (prompts/tokens/HTML) must never reach the DB untruncated. The
  client-facing error response stays as-is.
- **F9 atomicity (FIX 4)**: `set_auto_failover_enabled` switches FIRST, then
  persists `auto_failover_enabled=true` only after a successful
  `switch_proxy_target`. On switch failure it returns Err with the flag still
  false. Preserve empty-queue auto-add of the current provider, the
  `provider-switched` emit, and the unconditional `refresh_tray` (both paths).
- **CSRF intent (FIX 5)**: state-changing `/api/*` methods (POST/PUT/PATCH/
  DELETE) require a same-origin intent — `Sec-Fetch-Site ∈ {same-origin, none}`,
  or (Origin present) Origin host == Host (port-insensitive); else 403. No token
  plumbing. GET/HEAD and public paths exempt. The same-origin SPA is unaffected.
- **CORS preflight (FIX 7)**: in `require_auth`, an `OPTIONS` request carrying
  `Origin` is passed through to the inner `CorsLayer` (no 401), so
  `CORS_ALLOW_ORIGINS` can negotiate; the real cross-origin request is still
  auth-checked + FIX-5-checked.
- **Background workers (FIX 6)**: `examples/server.rs` spawns the tauri-free
  desktop-parity workers after `run_post_db_bootstrap`: periodic DB backup
  (initial + daily) and session-usage sync (initial + 60s). WebDAV/S3 auto-sync
  workers are intentionally SKIPPED (need `AppHandle`; FE gates them desktop-only
  per F10).
- **ip_guard exotic ranges (FIX 8)**: `is_blocked_ipv4` also blocks `0.0.0.0/8`
  (octet0==0), multicast (224/4), reserved (240/4); `is_blocked_ipv6` unwraps
  `to_ipv4()` (catches `::a.b.c.d`) and blocks 6to4 (2002::/16), NAT64
  (64:ff9b::/96), Teredo (2001::/32), multicast (ff00::/8). Add ranges ONLY in
  `ip_guard.rs` (tauri-free + sync).

#### 4. Validation & Error Matrix

- A custom-template usage script with `request.url` reaching loopback/metadata/
  CGNAT dialed in web mode without `guard_outbound_url` -> reject.
- Desktop usage path forced through the guard (`enforce=true`) -> reject (must
  stay unrestricted).
- A user-URL service dial still on `http_client::get()` -> reject (redirect-hop
  bypass).
- Upstream error body persisted to the request-log DB untruncated -> reject.
- `auto_failover_enabled=true` persisted before a failed switch -> reject.
- A cross-site mutating POST accepted because auth alone passed -> reject.
- An `OPTIONS`+Origin preflight 401'd before the CorsLayer -> reject.
- A new exotic range added in `common.rs` instead of `ip_guard.rs` -> reject
  (SSOT divergence).

#### 5. Good/Base/Bad Cases

- Good: web custom-script `request.url=http://169.254.169.254/` is 400-rejected
  before dial; a public `https://api.example.com` passes; the desktop command
  path is unchanged.
- Base: built-in usage templates (balance/token_plan/copilot/official) keep
  their existing guarded service dials; only the custom JS path gains the guard.
- Bad: importing `web_api` into `usage_script.rs`, or keying the breaker bypass /
  CSRF check off something other than the documented signals.

#### 6. Tests Required

- `usage_script::tests::{web_guard_rejects_internal_request_url_before_dial,
  web_guard_rejects_non_http_scheme}`.
- `proxy::ip_guard::tests::{blocks_exotic_ipv4_ranges, blocks_exotic_ipv6_ranges,
  guard_outbound_url_blocks_internal_and_allows_public}`.
- `proxy::handlers::tests::upstream_error_body_is_truncated_before_db_persistence`.
- `services::proxy` F9: `enable_auto_failover_does_not_persist_flag_when_switch_fails`.
- `web_api::middleware::auth::tests` CSRF intent cases (cross-site 403,
  same-origin/none/no-origin pass, opaque null 403).
- `examples/server.rs` `main_spawns_background_workers_after_bootstrap`.
- Gates: desktop `cargo clippy --features desktop -- -D warnings` + `cargo test`;
  web `cargo check --no-default-features --features web-server --example server` +
  the web test set; `cargo fmt --check`; FE `pnpm format:check`,
  `pnpm check:web-routes` (missing:0), `pnpm typecheck`.

#### 7. Wrong vs Correct

##### Wrong

```rust
let client = crate::proxy::http_client::get(); // user-URL dial, no redirect recheck
let error_message = get_error_message(error);   // full upstream body → DB
let is_public = path == "/api/health"; // OPTIONS preflight 401'd
```

##### Correct

```rust
if enforce_outbound_guard { crate::proxy::ip_guard::guard_outbound_url(&config.url).await?; }
let client = crate::proxy::http_client::get_guarded();
let error_message = compact_error_message(&get_error_message(error), 400);
if req.method() == Method::OPTIONS && req.headers().contains_key(header::ORIGIN) {
    return next.run(req).await; // let CorsLayer answer the preflight
}
```

---

## Testing Requirements

<!-- What level of testing is expected -->

(To be filled by the team)

---

## Code Review Checklist

<!-- What reviewers should check -->

(To be filled by the team)
