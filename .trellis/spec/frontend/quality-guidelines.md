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
- Built-in templates are `github_copilot`, `token_plan`, and `balance`.
- Built-in template tests must go through `usageApi.testScript`, not lower-level subscription or Copilot APIs.
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

- Good: Balance, Token Plan, and GitHub Copilot template test buttons call `usageApi.testScript(..., templateType)`, then write the returned `UsageResult` into `["usage", provider.id, appId]`.
- Base: custom/general/newapi scripts call `usageApi.testScript` with script code and explicit credential overrides.
- Bad: testing Balance via `subscriptionApi.getBalance`, Token Plan via `subscriptionApi.getCodingPlanQuota`, or Copilot via `copilot_get_usage*` from `UsageScriptModal`.
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

## Testing Requirements

<!-- What level of testing is expected -->

(To be filled by the team)

---

## Code Review Checklist

<!-- What reviewers should check -->

(To be filled by the team)
