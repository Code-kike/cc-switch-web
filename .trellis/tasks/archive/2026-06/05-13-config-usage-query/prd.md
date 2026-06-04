# Fix configuration usage query

## Goal

Make provider usage query configuration actually usable in this fork, especially in the Web-first runtime. Users should be able to configure usage querying for a provider, test the configuration, save it, and then see query results refresh on provider cards without switching to the desktop-only path.

## What I Already Know

- The current project is based on `farion1231/cc-switch` and `Laliet/CC-Switch-Web`.
- The local code already has partial usage query infrastructure:
  - Frontend provider-card display and refresh live in `src/components/UsageFooter.tsx` and `src/components/providers/ProviderCard.tsx`.
  - Usage configuration UI lives in `src/components/UsageScriptModal.tsx`.
  - Frontend calls go through `src/lib/api/usage.ts`, `src/lib/query/queries.ts`, and `src/lib/query/usage.ts`.
  - Desktop command handlers exist in `src-tauri/src/commands/provider.rs`.
  - Web handlers exist in `src-tauri/src/web_api/handlers/providers.rs` and `src-tauri/src/web_api/handlers/usage.rs`.
  - Backend logic exists in `src-tauri/src/services/provider/usage.rs`, `src-tauri/src/services/balance.rs`, and `src-tauri/src/services/coding_plan.rs`.
- Web command registration already maps `queryProviderUsage` to `/api/providers/queryproviderusage` and `testUsageScript` to `/api/usage/testusagescript`.
- A likely current gap is not a totally missing route; it is consistency across configure/test/save/query paths, especially for built-in templates and Web mode.

## Assumptions

- MVP should preserve existing UI shape and not redesign the provider list.
- The task should fix the current fork, not rewrite usage accounting or proxy request statistics.
- Query support should cover the existing templates already present in code: custom/general/newapi JS scripts, balance, token_plan, and GitHub Copilot where supported by backend state.

## Requirements

- Provider usage configuration can be opened from provider cards for supported provider types.
- Saving a usage query configuration persists `provider.meta.usage_script`.
- Saved usage configuration invalidates the correct React Query cache so provider cards refresh from saved configuration.
- Testing and saved querying use the same backend behavior as much as possible, including built-in templates.
- Web mode must not route supported usage-template tests to commands currently marked as unsupported.
- Errors should remain visible and actionable instead of silently returning stale data.
- Add or update focused tests for the fixed behavior.

## Acceptance Criteria

- [ ] A provider with saved usage configuration can query usage via `usageApi.query()` in Web mode.
- [ ] Built-in usage templates can be tested/configured without relying on desktop-only commands.
- [ ] Saving usage config refreshes provider list data and usage result cache.
- [ ] Existing query key conventions are used consistently.
- [ ] Relevant unit or integration tests pass.
- [ ] `pnpm typecheck` passes, or any failure is documented if unrelated.

## Out Of Scope

- Replacing the entire usage dashboard.
- Adding new billing providers beyond the templates already present.
- Changing proxy request accounting or model pricing logic except where directly needed for configuration usage query.
- Implementing full desktop-only UI features in Web mode when the backend cannot support them safely.

## Technical Notes

- `src/components/UsageScriptModal.tsx` currently has special test branches for balance, token_plan, and GitHub Copilot.
- `src/lib/api/web-commands.ts` marks low-level Copilot commands as unsupported, but saved provider usage can already go through `queryProviderUsage`.
- `src/hooks/useProviderActions.ts` already invalidates `["providers", activeApp]` and `["usage", provider.id, activeApp]` after saving usage configuration.
- `src/hooks/useUsageCacheBridge.ts` mirrors backend usage-cache events into the same `usageKeys.script(providerId, appType)` cache.
- The implementation should prefer reusing the central `usageApi.query` / `usageApi.testScript` path or adding a narrow backend test endpoint over duplicating client-only logic.
