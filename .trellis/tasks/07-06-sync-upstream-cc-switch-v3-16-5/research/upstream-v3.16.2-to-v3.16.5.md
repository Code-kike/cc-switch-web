# Upstream v3.16.2 to v3.16.5 Snapshot

Source: `farion1231/cc-switch` tags fetched via `git ls-remote` and a temporary partial bare fetch.

## Confirmed Scope

- Current project version: `3.16.2` in `package.json` and `src-tauri/Cargo.toml`.
- Product upstream latest observed tag: `v3.16.5`.
- Confirmed by user: this sync targets `farion1231/cc-switch v3.16.2..v3.16.5`.
- Explicitly not included by default: `origin` Dependabot branches and `Laliet/CC-Switch-Web` changes.

## Notable Upstream Themes

- Codex native Responses direct-connect and generated model catalog.
- Codex model-mapping decoupled from Local Routing toggles.
- Codex unified session-history toggle, migration, and ledger-based restore.
- Usage accounting changes, including pricing basis/takeover dimensions in storage schema v11, live end time for custom date ranges, imported pricing from `models.dev`, and refreshed pricing seeds.
- Provider and preset catalog changes: Claude Sonnet 5 default/pricing, Kimi K2.7, Doubao Seed 2.1, GLM 5.1/5.2, Qiniu, FennoAI, ZetaAPI, TeamoRouter, Amux, Code0.ai, NekoCode, SubRouter, OpenCode Go, Unity2.ai, and related referral/link changes.
- Proxy changes: custom User-Agent override, local proxy request override, zstd/body decompression before forwarding, takeover auth preservation, takeover-residue recovery, DeepSeek effort stripping, image rectifier extension, cache token fixes, and route-takeover billing model fixes.
- UI/UX changes: session category/group management, provider preset search/sorting, Skills import green dot for unmanaged skills, dark-mode JSON editors, usage dashboard filter refinements, narrow popover/date-range fixes, long dropdown scroll bounds, fullscreen popover z-index fixes, About progressive tool version checks, Linux GDK backend override.
- Operational changes: DB too-new recovery screen, backend-driven updater restart/download flow, Windows ARM64 release support, Linux Wayland escape hatch docs.

## Local Constraints

- This fork is Web-first and has recent hardening on `fix/web-audit-phase1-2`.
- Current branch is 13 commits ahead of `origin/main` and 0 behind after fetch.
- Existing active planning task `06-10-port-codex-chat-routing-model-catalog-stack` is a deferred v3.16.2 sync item and should be reconciled or superseded deliberately, not silently ignored.
- No root `CONTEXT.md` existed before this session; terminology now distinguishes product upstream from web prototype upstream.

## Research Gaps

- A full file-level diff stat failed twice because the temporary partial fetch hit network/promisor fetch errors. Commit themes are reliable, but implementation planning should still inspect upstream files commit-by-commit or via a more robust local clone before coding.
