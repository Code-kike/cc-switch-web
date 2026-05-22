# Sync upstream cc-switch v3.15.0

## Goal

Update this Web-first fork to track upstream `farion1231/cc-switch` `v3.15.0` while preserving the fork-specific browser/server runtime. The sync should bring forward upstream user-facing features and fixes that matter for the Web deployment, without deleting Web-only infrastructure or overwriting unrelated local work.

## What I Already Know

- The current fork identifies as `3.14.1` in:
  - `package.json`
  - `src-tauri/Cargo.toml`
  - `src-tauri/tauri.conf.json`
- Upstream `farion1231/cc-switch` has tag `v3.15.0` at commit `9e3f1689038febb36da08993cd47281426b5dd7c`.
- Upstream `v3.15.0` is a major release, not just a version bump.
- Upstream release notes say the release centers on:
  - Claude Desktop as a first-class managed surface.
  - Third-party provider switching through proxy gateway.
  - Major reverse-proxy reliability and lifecycle hardening.
  - Provider preset expansion.
  - Codex OAuth live model discovery.
  - Usage dashboard Hero and usage accounting changes.
- Directly comparing current fork to upstream `v3.15.0` shows many Web-fork files as deletions because upstream desktop does not contain them. Those files must be preserved or ported, not removed.
- Current working tree already contains many uncommitted changes unrelated to this sync, including docs deletions and active usage-query work. The sync should avoid mixing those changes into a blind merge.

## Assumptions

- The intended target version is upstream `v3.15.0`, not just `3.14.1`.
- The Web fork should remain Web-first and keep:
  - standalone Axum web server mode
  - `/api/**` Web command parity layer
  - service install scripts
  - Web build scripts
  - Web update behavior
- Version strings should only be bumped once the codebase actually carries the upstream functionality expected for that version.
- A safe sync will likely require multiple focused patches rather than one large automatic merge.
- The first execution pass should implement the safe MVP sync: upstream backend/proxy/provider/usage fixes that benefit current Web-managed apps, plus low-risk preset/icon/docs changes. Full Claude Desktop Web UI parity is deferred unless needed by those fixes.

## Open Questions

- Resolved: proceed with a safe MVP sync first. Keep full Claude Desktop Web UI parity out of the first pass.

## Requirements

- Preserve Web-first fork behavior and deployment files.
- Preserve existing local uncommitted changes unless explicitly included in this sync.
- Bring upstream `v3.15.0` version metadata into the fork only after the relevant code sync is complete.
- First pass scope:
  - low-risk upstream release notes / changelog material that does not conflict with Web positioning
  - provider preset/icon updates that are already consumed by existing Web UI
  - proxy, provider, usage, and Codex OAuth backend changes needed by current apps
  - Web API/web-command adaptations for newly ported command surfaces
- First pass explicitly defers full Claude Desktop screen parity unless required for shared backend/types to compile.
- Port upstream changes in dependency-aware order:
  - schema/types/service changes before UI that depends on them
  - proxy/provider changes before provider card badges and controls
  - usage stats/backend semantics before usage dashboard UI
  - command additions before Web command map/routes
- Regenerate or update Web command mapping when Tauri commands change.
- Add/update Web API handlers for any newly surfaced command needed by Web mode.
- Add tests for newly ported Web-visible behavior.

## Acceptance Criteria

- [ ] The fork still builds and runs in Web mode.
- [ ] Web-only files are preserved:
  - `src-tauri/examples/server.rs`
  - `src-tauri/src/web_api/**`
  - `src/lib/api/web-commands.ts`
  - Web route parity scripts and service deployment scripts
- [ ] Version metadata is consistently updated to `3.15.0` when the sync is complete.
- [ ] Provider presets and icons relevant to Web UI are updated.
- [ ] Proxy and usage fixes that affect Web/server operation are ported.
- [ ] Web command route coverage remains valid.
- [ ] `pnpm typecheck` passes.
- [ ] Web-server Rust compile check passes:
  - `cargo check --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server`

## Out Of Scope

- Blindly replacing the Web fork with upstream desktop source.
- Removing Web server, Web API, route parity, or service deployment infrastructure.
- Pushing to remote.
- Resolving unrelated pre-existing docs deletions or usage-query WIP unless they directly block the sync.

## Technical Notes

- Upstream tag fetched locally as `upstream-v3.15.0`.
- Upstream main fetched locally as `upstream/main`.
- Research artifact: `research/upstream-v315-diff.md`.
- A direct `git diff HEAD..upstream-v3.15.0` includes false deletion signals for fork-only Web files. Use `v3.14.1..upstream-v3.15.0` to understand upstream release content, and use targeted patching/cherry-picking to port into the Web fork.
