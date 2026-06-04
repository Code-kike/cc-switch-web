# Sync upstream cc-switch v3.16.0

## Goal

Port upstream `farion1231/cc-switch` **v3.16.0** changes into this Web-first fork, continuing the
granular `sync:` commit methodology established in `05-21-sync-upstream-v315`. Bring forward the
full v3.16.0 code surface relevant to the fork (shared backend, proxy, Codex routing, usage,
presets) **except** the partner/referral preset content, and adapt anything event-based to the
fork's dual-runtime (Tauri + web-server) transport. Bump version metadata to 3.16.0 only after the
code lands.

## Range & baseline

- Previous sync point: upstream `3c3d4174` (Enable Codex goals #3089) — already ported (fork `e6991fd5`).
- Upstream now: `8f83fa20` (main HEAD), release tag **v3.16.0** = `47232cb0`.
- Delta `3c3d4174..8f83fa20` = 24 commits. Audit: [`research/upstream-v316-audit.md`](research/upstream-v316-audit.md).
- Sync via local refs `upstream/main` / tag (fetched 2026-05-30); **no blind merge** — fork-only Web
  files (`web_api/**`, `examples/server.rs`, `web-commands.ts`, deploy scripts) must be preserved.

## Scope (CONFIRMED — all 4 batches; partner presets excluded)

### Batch 1 — Shared fixes (low risk, do first)
- `bc1467db` **panic-fix subset**: non-ASCII model name crash in `services/session_usage{,_codex,_gemini}.rs` (protects the fork's `lib.rs` 60s background sync timers).
- `554e3b48` fix(proxy): normalize DeepSeek Anthropic tool thinking history (#3203).
- `e605eba2` fix(deeplink): preserve custom env fields when importing Claude providers (#2928).
- `d7a34f42` fix(about): handle prerelease tools in version check (adds `src/lib/version.ts` + test).

### Batch 2 — Model / pricing / preset refresh
- `3a154207` default models + pricing across presets; `0877b9e3` Opus→4.8; `4bb4e994` Shengsuanyun IDs + GPT 5.5; `3d6fb894` Shengsuanyun prefixed IDs; `058c9fb8` OpenCode Go preset rename; `6b0dd3c4` OMO recommended models + Fill-Recommended feedback.
- Keep Rust `database/schema.rs::seed_model_pricing` consistent with FE `src/config/*Presets`.

### Batch 3 — Codex routing refactor (toml_edit-sensitive, one reviewed batch)
- `a2ac21d0` stop force-rewriting user's model_provider in live config; `fc0433f2` unify custom model_provider routing key to "custom"; `af60c7ed` remote compaction toggle for third-party providers; `2b6ede14` Codex provider migration test expectations.
- Reconcile with fork's `codex_config.rs::normalize_codex_settings_config_model_provider`.

### Batch 4 — Real-time usage event (FORK-DIVERGENT — largest effort)
- `bc1467db` **event subset**: new `src-tauri/src/usage_events.rs` + `src/hooks/useUsageEventBridge.ts` + `lib.rs` wiring + `UsageDashboard.tsx`.
- **Adaptation required**: upstream emits via the Tauri event bus; the fork must route emission through its `UiEventSink` abstraction so web-server mode delivers it over `/api/events` SSE (mirror the existing `useUsageCacheBridge` / `event-adapter.ts` pattern). Without this, web mode gets no live refresh.

### Version bump (after batches 1–4 verify green)
- `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` → `3.16.0`; add release-notes if desired.

## Out of scope
- Partner/referral presets: `e71b9091` SudoCode, `32b30e43` AtlasCloud, `9ef14190` APINebula, `8302f1e3` APIKEY.FUN, `85552cf4` ShengSuanYun referral param, `d905ed16` Atlas UTM params — **excluded** (fork stays referral-neutral).
- Docs-only / upstream-desktop release material (`8f83fa20`, changelog, `docs/guides/codex-deepseek-routing-*`, `docs/user-manual/**`, `docs/images/**`).
- Claude Desktop as a first-class Web surface (port only shared mapping bits from `94cc3d10` if needed to compile).
- Pushing to remote; resolving unrelated pre-existing dirty files.

## Acceptance Criteria
- [ ] Batch 1–4 ported as focused `sync:` commits, adapted to the fork (not verbatim where dual-runtime differs).
- [ ] Real-time usage events work in BOTH runtimes (Tauri emit on desktop, SSE on web).
- [ ] No partner/referral preset content introduced.
- [ ] Web-only files preserved (`examples/server.rs`, `web_api/**`, `web-commands.ts`, deploy/route-parity scripts).
- [ ] `pnpm typecheck` passes.
- [ ] `pnpm check:web-routes` → still 0 missing (no new commands this round, so registry unchanged).
- [ ] `pnpm test:unit` passes (incl. ported codex migration + version + providerConfigUtils tests).
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server` passes.
- [ ] Version metadata = 3.16.0 once code is in.

## Definition of Done
- Tests added/updated for ported behavior (version.ts, providerConfigUtils, codex migration, usage).
- typecheck / web-routes / unit tests / web-server cargo check all green.
- Version strings consistent at 3.16.0.

## Technical Notes (fork-specific landmarks for implementation)
- **No new `#[tauri::command]`** in the delta → command parity surface (261) unchanged.
- Event parity pattern to mirror: `web_api/handlers/system.rs` (`GET /api/events` SSE), `runtime/` `UiEventSink`/`ChannelEventSink`/`TauriEventSink`, frontend `src/lib/api/event-adapter.ts` + existing `src/hooks/useUsageCacheBridge.ts` (analogous bridge).
- Background timers consuming the panic-fix files: `src-tauri/src/lib.rs` (session-usage sync, 60s interval).
- Codex toml_edit logic: `src-tauri/src/codex_config.rs`.
- Deeplink dual-runtime: `commands/deeplink.rs` + `web_api/handlers/deeplink.rs` share `deeplink/` core.
- Pricing dual-source: Rust `database/schema.rs::seed_model_pricing` + FE `src/config/*Presets`.

## Decision (ADR-lite) — Codex model_provider anchor removal (Batch 3)

**Context**: The fork had a fork-specific anchor mechanism that force-rewrote Codex live
`config.toml` `model_provider` to a stable `"ccswitch"` bucket (no history migration). Upstream
`a2ac21d0` removed force-rewriting, pairing it with a one-time `codex_history_migration.rs` the
fork lacks.
**Decision** (confirmed by maintainer 2026-05-30): Accept the anchor removal (align with upstream,
preserve the user's real `model_provider` id). DEFER the history-continuity migration as a separate
follow-up task; note the behavior change in the v3.16.0 release notes.
**Consequences**: New writes use each provider's real `model_provider` id. Existing fork users'
Codex resume-history bucket key changes (desktop-only, narrow impact). Follow-up task: port/adapt
`codex_history_migration.rs` if continuity is later required.

## Methodology notes
- Upstream fetched locally: `git fetch https://github.com/farion1231/cc-switch.git main --tags` (FETCH_HEAD=8f83fa20, tag v3.16.0).
- Port with `git show <sha> -- <paths>` / targeted cherry-pick, NOT `git merge upstream`.
- Dependency order: backend/types/services → proxy/provider → presets → usage event/UI → version bump.
