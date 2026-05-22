# Upstream v3.15.0 Diff Research

## Sources

- GitHub release: https://github.com/farion1231/cc-switch/releases/tag/v3.15.0
- Local tag fetched from upstream: `upstream-v3.15.0`
- Local upstream main ref: `upstream/main`

## Release Identity

- Upstream tag: `v3.15.0`
- Commit: `9e3f1689038febb36da08993cd47281426b5dd7c`
- Author/commit date: 2026-05-16 11:21:18 +0800
- Current Web fork version before sync: `3.14.1`

## Upstream Release Summary

GitHub release notes describe `v3.15.0` as a major release after `v3.14.x`, centered on:

- Claude Desktop first-class management.
- Third-party provider switching through the in-app proxy gateway.
- Role-based model mapping with `sonnet` / `opus` / `haiku` and `supports1m`.
- Copilot/Codex OAuth provider reuse.
- 44 Claude Desktop provider presets translated from Claude Code.
- Major proxy reliability hardening.
- Codex OAuth live model discovery.
- Usage dashboard filter-driven Hero card.
- Provider ecosystem expansion.

Release stats from upstream `CHANGELOG.md`:

- 127 commits
- 211 files changed
- +17,980 insertions
- -2,748 deletions

Local git diff from `v3.14.1..upstream-v3.15.0` reports:

- 216 files changed
- 19,612 insertions
- 2,756 deletions

The count differs slightly from release notes because local diff includes generated/binary accounting differences.

## Important Upstream Version Files

Upstream `v3.15.0` sets:

- `package.json`: `"version": "3.15.0"`
- `src-tauri/Cargo.toml`: `version = "3.15.0"`
- `src-tauri/tauri.conf.json`: `"version": "3.15.0"`

The Web fork also has these three fields at `3.14.1`. Do not update them alone unless we intentionally accept a version-only placeholder.

## Upstream Change Areas

### Claude Desktop Surface

Major new files:

- `src-tauri/src/claude_desktop_config.rs`
- `src/components/providers/forms/ClaudeDesktopProviderForm.tsx`
- `src/config/claudeDesktopProviderPresets.ts`
- `src/components/proxy/ClaudeDesktopRouteToggle.tsx`
- tests for Claude Desktop provider form.

Likely impact:

- `AppType` and visible app handling.
- App switcher labels and settings visibility.
- Provider form routing and preset selection.
- Proxy takeover/routing model.

### Proxy Hardening

High-change files:

- `src-tauri/src/proxy/forwarder.rs`
- `src-tauri/src/proxy/handlers.rs`
- `src-tauri/src/proxy/providers/claude.rs`
- `src-tauri/src/proxy/providers/streaming.rs`
- `src-tauri/src/proxy/providers/transform.rs`
- `src-tauri/src/proxy/response_processor.rs`
- `src-tauri/src/services/proxy.rs`

New files:

- `src-tauri/src/proxy/json_canonical.rs`
- `src-tauri/src/proxy/providers/copilot_model_map.rs`

Likely impact:

- Web server proxy behavior should receive these reliability fixes.
- Need reconcile with current fork modifications in proxy and web runtime.

### Usage Dashboard / Accounting

High-change files:

- `src-tauri/src/services/usage_stats.rs`
- `src-tauri/src/database/dao/usage_rollup.rs`
- `src-tauri/src/proxy/usage/calculator.rs`
- `src-tauri/src/proxy/usage/logger.rs`
- `src/components/usage/UsageDashboard.tsx`
- `src/components/usage/RequestLogTable.tsx`
- `src/types/usage.ts`

New frontend:

- `src/components/usage/UsageHero.tsx`

Removed upstream frontend:

- `src/components/usage/UsageSummaryCards.tsx`

Likely impact:

- This interacts with the active usage-query work in the fork.
- Port carefully; do not regress Web usage query template behavior.

### Provider Presets / Icons

Changed:

- `src/config/claudeProviderPresets.ts`
- `src/config/codexProviderPresets.ts`
- `src/config/hermesProviderPresets.ts`
- `src/config/openclawProviderPresets.ts`
- `src/config/opencodeProviderPresets.ts`
- `src/components/providers/forms/ProviderPresetSelector.tsx`

Added icons/assets:

- BytePlus
- ClaudeAPI
- ClaudeCN
- Huoshan
- Pateway
- RunAPI
- RelaxyCode

Likely impact:

- Usually portable to Web fork with moderate conflict risk.
- Asset additions are low-risk but binary.

### Codex OAuth / Model Fetch

Changed/new:

- `src-tauri/src/services/codex_oauth_models.rs`
- `src-tauri/src/services/model_fetch.rs`
- `src/lib/api/model-fetch.ts`
- `src-tauri/src/commands/codex_oauth.rs`

Likely impact:

- Web route coverage may need new handler(s) or command mappings.

## Web-Fork Preservation Risk

Comparing `HEAD..upstream-v3.15.0` shows upstream would remove these fork-only files because upstream desktop does not have them:

- `deploy/systemd/cc-switch-web.service`
- `scripts/check-web-route-coverage.mjs`
- `scripts/install-cc-switch-web-service.sh`
- `scripts/smoke-web-server.mjs`
- `src-tauri/examples/server.rs`
- `src-tauri/examples/web_proxy.rs`
- `src-tauri/examples/web_services.rs`
- `src-tauri/src/bin/gen-command-manifest.rs`
- `src-tauri/src/bootstrap.rs`
- `src-tauri/src/runtime/**`
- `src-tauri/src/web_api/**`
- `src/lib/api/web-commands.ts`

These are core Web-fork infrastructure and must be preserved.

## Recommended Sync Strategy

Do not run a blind `git merge upstream-v3.15.0` in the dirty working tree.

Recommended phases:

1. Create a clean branch/worktree or first commit/stash unrelated WIP.
2. Port upstream version/dependency changes after code sync, not before.
3. Port low-risk preset/icon/README/release-note changes.
4. Port backend schema/service/proxy changes in clusters.
5. Adapt Web API handlers and `web-commands.ts` for changed command surfaces.
6. Port frontend UI changes behind Web-safe routes.
7. Run:
   - `pnpm typecheck`
   - focused Vitest suites
   - Web route parity
   - Web-server cargo check

## Open Decision

The largest scope decision is whether to make the Web fork fully expose Claude Desktop management now, or defer Claude Desktop UI parity and first port the backend/proxy/provider/usage fixes that benefit current Web-managed apps.
