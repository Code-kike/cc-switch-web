# Upstream v3.16.0 sync audit

Date: 2026-05-30

## Range

```bash
# Previous sync point (last audited in 05-21-sync-upstream-v315):
3c3d4174  Enable Codex goals in provider templates (#3089)   # already ported (fork commit e6991fd5)

# Upstream now:
8f83fa20  docs: add Codex DeepSeek routing guides            # = upstream/main HEAD
v3.16.0   tag = 47232cb0 (chore(release): bump version to 3.16.0)
```

Delta: `3c3d4174..8f83fa20` = **24 commits**, and upstream cut a real release **v3.16.0**.

Fetched fresh from `https://github.com/farion1231/cc-switch.git main --tags` on 2026-05-30
(FETCH_HEAD = 8f83fa20; new tag v3.16.0 appeared).

## Methodology (unchanged from v3.15 task)

- Do NOT blind-merge upstream; fork-only Web files (`web_api/**`, `examples/server.rs`,
  `src/lib/api/web-commands.ts`, deploy scripts) read as deletions in a raw diff.
- Port focused slices as granular `sync:` commits, adapting to dual-runtime.
- Bump version metadata to 3.16.0 ONLY after the code it implies is actually synced.
- Any new `#[tauri::command]` must get a matching `web_api/handlers/*` route + `web-commands.ts` entry.

## Fork-specific impact scan (the part that differs from a desktop sync)

- **0 new `#[tauri::command]`** in the delta → command-level parity surface (261) unchanged;
  no new `web_api` handler / `web-commands.ts` entries required.
- **Event parity hotspot:** `bc1467db` adds a real-time usage event channel — new backend
  module `src-tauri/src/usage_events.rs`, new frontend hook `src/hooks/useUsageEventBridge.ts`,
  and `lib.rs` wiring. Upstream emits via the Tauri event bus; the **fork must route this through
  its `UiEventSink` abstraction** (desktop `TauriEventSink::emit` vs web `ChannelEventSink` -> SSE
  `/api/events`) or the web build gets no live usage refresh. This is the only genuinely
  fork-divergent port this round.
- **Background-timer safety:** `bc1467db` also fixes a panic on non-ASCII model names in
  `services/session_usage{,_codex,_gemini}.rs` — exactly the files the fork's `lib.rs` runs in
  60s background sync timers. High-priority must-port (a crash there kills the sync loop).
- `codex_config.rs` touched by 3 interrelated Codex routing commits (toml_edit-sensitive).
- `src-tauri/src/deeplink/` touched (dual-runtime: command + web_api handler both exist).

## Candidate matrix

### Must consider now (Web/shared-backend, low–medium scope)

| Commit | Subject | Files | Why it matters / fork note |
| --- | --- | --- | --- |
| `bc1467db` | feat(usage): real-time stats refresh + fix codex sync panic on non-ASCII model names (#3027) | `usage_events.rs`(new), `session_usage*.rs`, `proxy/usage/logger.rs`, `dao/usage_rollup.rs`, `lib.rs`, `UsageDashboard.tsx`, `useUsageEventBridge.ts`(new) | **Split in two:** (a) panic fix = must-port, low risk; (b) real-time event refresh = needs `UiEventSink`/SSE adaptation for web mode. |
| `554e3b48` | fix(proxy): normalize DeepSeek Anthropic tool thinking history (#3203) | `proxy/providers/*` | Shared proxy correctness; relevant to server-mode Claude-compatible/DeepSeek routing. |
| `e605eba2` | fix(deeplink): preserve custom env fields when importing Claude providers (#2928) | `src-tauri/src/deeplink/*` | Dual-runtime deeplink import; clean port (command + web_api handler share core). |
| `d7a34f42` | fix(about): handle prerelease tools in version check | `src/lib/version.ts`(new)+test, AboutSection | Small FE fix; adds version-compare util + tests. |
| `6b0dd3c4` | fix(omo): sync recommended models + improve Fill Recommended feedback | OMO presets / forms | FE preset/UX; safe if current OMO form matches. |

### Model / pricing / preset refresh (FE consumes `src/config/*Presets`; product-flavored)

| Commit | Subject | Note |
| --- | --- | --- |
| `3a154207` | Update default models and pricing across presets | Pricing also seeded in Rust `schema.rs::seed_model_pricing` — keep DB pricing + FE presets consistent. |
| `0877b9e3` | Upgrade default Claude Opus model to 4.8 | Default model bump. |
| `94cc3d10` | Align Claude Desktop model mapping with Claude Code three-role tiers | Claude Desktop is not a first-class Web surface yet — port only the shared mapping bits. |
| `4bb4e994` / `3d6fb894` | Fix Shengsuanyun prefixed model IDs (+ GPT 5.5) | Preset ID correctness. |
| `058c9fb8` | Rename OpenCode Go preset to drop model suffix | Preset label. |

### Codex routing/config refactors (toml_edit-sensitive; port as one coherent batch)

| Commit | Subject | Note |
| --- | --- | --- |
| `a2ac21d0` | refactor(codex): stop force-rewriting user's model_provider field in live config | Reverses prior behavior; reconcile with fork's `normalize_codex_settings_config_model_provider`. |
| `fc0433f2` | refactor(codex): unify custom model_provider routing key to "custom" | Pairs with a2ac21d0. |
| `af60c7ed` | feat(codex): add remote compaction toggle for third-party providers | New provider field + form control. |
| `2b6ede14` | Update Codex provider migration test expectations | Test alignment for the above. |

### Partner provider presets — PRODUCT DECISION (referral/UTM links)

| Commit | Subject | Note |
| --- | --- | --- |
| `e71b9091` | Add SudoCode partner provider presets | Adds partner preset + logo/icon. |
| `32b30e43` | Add AtlasCloud partner provider presets | + UTM link (`d905ed16`). |
| `9ef14190` | Add APINebula partner provider presets | |
| `8302f1e3` | Add APIKEY.FUN partner provider presets | |
| `85552cf4` | Add referral param to ShengSuanYun website links | Referral param. |
| `d905ed16` | Add UTM params to Atlas Cloud partner link across all locales | Marketing params. |

> These embed the upstream author's referral/UTM/partner links. Whether to carry them into this
> fork is a product/branding decision, not a bugfix. Default recommendation: **defer** (or port
> presets without the referral/UTM params) unless the maintainer wants them.

### Defer / docs-only

| Commit | Reason |
| --- | --- |
| `8f83fa20`, `fe3eb7e6`, `47232cb0` (release-notes/changelog), `docs/guides/codex-deepseek-routing-*`, `docs/user-manual/**`, `docs/images/**` | Docs-only / upstream-desktop release material. Fork already holds docs deletions aside — do not mix into code sync. The `47232cb0` version bump to 3.16.0 applies only after code is synced. |

## Recommended port order

1. `bc1467db` **panic-fix subset only** (session_usage non-ASCII) — isolated, protects fork timers.
2. `554e3b48`, `e605eba2`, `d7a34f42` — small isolated shared/FE fixes.
3. Model/pricing/preset refresh (`3a154207`, `0877b9e3`, `4bb4e994`, `3d6fb894`, `058c9fb8`, `6b0dd3c4`) — keep Rust `seed_model_pricing` and FE presets consistent.
4. Codex routing batch (`a2ac21d0`+`fc0433f2`+`af60c7ed`+`2b6ede14`) — one reviewed batch.
5. `bc1467db` **real-time usage event** — adapt to `UiEventSink`/SSE for web parity (largest effort).
6. Partner presets (`e71b9091`,`32b30e43`,`9ef14190`,`8302f1e3`,`85552cf4`,`d905ed16`) — only if product wants them; consider stripping referral/UTM.
7. Version metadata -> 3.16.0 (package.json / Cargo.toml / tauri.conf.json) once 1–5 land.

## Verify gates (from v3.15 task acceptance)

```bash
pnpm typecheck
pnpm check:web-routes        # must stay 0 missing
pnpm test:unit
cargo check --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server
```
