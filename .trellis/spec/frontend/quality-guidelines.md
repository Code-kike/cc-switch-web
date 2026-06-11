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
- `examples/web_proxy.rs` manually re-exports only the proxy modules needed in Web-server mode.
- If a provider module imports `crate::proxy::<module_name>`, the module must be available in both:
  - `src-tauri/src/proxy/mod.rs`
  - `src-tauri/examples/web_proxy.rs`

#### 4. Validation & Error Matrix

- Missing desktop module declaration -> normal Rust builds fail with unresolved import.
- Missing `examples/web_proxy.rs` path module -> Web server compile fails even when desktop module wiring is correct.
- Helper module added but no Web compile run -> risk of shipping a Web-only compile break.

#### 5. Good/Base/Bad Cases

- Good: add `src-tauri/src/proxy/json_canonical.rs`, add `pub(crate) mod json_canonical;` in `src-tauri/src/proxy/mod.rs`, add the matching `#[path = "../src/proxy/json_canonical.rs"] pub(crate) mod json_canonical;` in `examples/web_proxy.rs`, then run Web server cargo check.
- Base: modules used only by desktop-only proxy code do not need to be exposed through Web if no Web-compiled module imports them.
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
  at a developer's real CLI configuration.
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

## Testing Requirements

<!-- What level of testing is expected -->

(To be filled by the team)

---

## Code Review Checklist

<!-- What reviewers should check -->

(To be filled by the team)
