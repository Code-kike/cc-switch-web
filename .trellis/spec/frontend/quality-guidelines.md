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
- Built-in templates are `github_copilot`, `token_plan`, and `balance`.
- Built-in template tests must go through `usageApi.testScript`, not lower-level subscription or Copilot APIs.
- Saved provider-card refresh must go through `usageApi.query`, which reaches `query_usage_with_templates`.
- Frontend API wrappers should normalize transport/API errors into `UsageResult { success: false, error }` so the provider UI can show actionable failures.

#### 4. Validation & Error Matrix

- Missing provider -> failed `UsageResult` or API error with provider-not-found detail.
- Disabled usage script -> failed `UsageResult` with "usage disabled" detail.
- Unsupported or malformed JS template -> failed `UsageResult`; do not leave stale success data visible.
- Built-in templates ignore the JS body; save-time JS `request.url` validation must not block them.
- Web mode must not call commands marked `unsupported` in `src/lib/api/web-commands.ts` from usage-template testing.

#### 5. Good/Base/Bad Cases

- Good: Balance, Token Plan, and GitHub Copilot template test buttons call `usageApi.testScript(..., templateType)`, then write the returned `UsageResult` into `["usage", provider.id, appId]`.
- Base: custom/general/newapi scripts call `usageApi.testScript` with script code and explicit credential overrides.
- Bad: testing Balance via `subscriptionApi.getBalance`, Token Plan via `subscriptionApi.getCodingPlanQuota`, or Copilot via `copilot_get_usage*` from `UsageScriptModal`.

#### 6. Tests Required

- Unit test `usageApi.query` and `usageApi.testScript` error normalization for Web API failures.
- Component tests asserting each built-in template calls `usageApi.testScript` with the expected template type.
- Backend compile check for Web server mode after changing provider usage signatures:
  - `cargo check --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server`

#### 7. Wrong vs Correct

##### Wrong

```typescript
await subscriptionApi.getBalance(baseUrl, apiKey);
await copilotGetUsage();
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

The same pattern applies to `token_plan` and `github_copilot`.

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

## Testing Requirements

<!-- What level of testing is expected -->

(To be filled by the team)

---

## Code Review Checklist

<!-- What reviewers should check -->

(To be filled by the team)
