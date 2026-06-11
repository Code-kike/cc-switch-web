# Upstream v3.16.1 + v3.16.2 sync audit

- **Query**: commit-by-commit audit of upstream farion1231/cc-switch `8f83fa20..v3.16.2` (69 commits) for the Web-first fork
- **Scope**: internal (git history + fork tree inspection)
- **Date**: 2026-06-10

## Range

```bash
# Previous sync point (05-30-sync-upstream-v316 task):
8f83fa20  docs: add Codex DeepSeek routing guides          # past v3.16.0 tag

# This round:
v3.16.1 = 25951d81 (2026-06-01)
v3.16.2 = 955ea26d (2026-06-08)   # tag points at last commit in range
git rev-list 8f83fa20..v3.16.2 → 69 commits
```

## Methodology (same as the v3.16.0 audit)

- Never blind-merge: fork-only files (`web_api/**`, `examples/server.rs`, `examples/web_proxy.rs`,
  `src/lib/api/web-commands.ts`, `bootstrap.rs`, `runtime/**`, `deploy/`, `scripts/`) read as
  deletions in a raw upstream diff. **Checked: no upstream commit in this range creates, renames,
  or moves anything into those paths — no collisions.**
- Port focused slices as granular `sync:` commits; run the full gate suite per batch
  (`cargo fmt --check`, `pnpm format:check`, `pnpm typecheck`, `pnpm check:web-routes`,
  `pnpm check:locales`, `pnpm test:unit`, `pnpm test:integration`, web cargo check
  `--no-default-features --features web-server --example server`, desktop `cargo clippy`/`cargo test`).
- Every new `#[tauri::command]` needs: `web_api/handlers/*` route + `src/lib/api/web-commands.ts`
  entry (SSOT for `check:web-routes`) + `tests/msw/handlers.ts` mock if FE tests exercise it.
- Fork dual-runtime contract (lib.rs line 1-5): **the whole crate body is
  `#![cfg(feature = "desktop")]`**; the web-server example re-includes ~30 modules via `#[path]`.
  Any `#[path]`-included module must not reference `tauri`. Twins: `services/proxy.rs` ↔
  `services/proxy_web.rs`, `services/webdav_auto_sync.rs` ↔ `services/webdav_auto_sync_web.rs`
  (selected in `services/mod.rs` via `#[cfg(feature = "desktop")]` + `pub use ..._web as ...`).

---

## Headline fork-impact scan

| Surface | Finding |
| --- | --- |
| New `#[tauri::command]` | **6**: `s3_test_connection`, `s3_sync_upload`, `s3_sync_download`, `s3_sync_save_settings`, `s3_sync_fetch_remote_info` (all in new `commands/s3_sync.rs`, commit `2a24da51`) + `ensure_claude_desktop_official_provider` (`commands/provider.rs`, commit `0960fd71`, **ClaudeDesktop-only → exclude**). Parity surface: 261 → 266 if S3 ports (CD command excluded). |
| Changed existing command signatures | **None.** Full-delta scan of `src-tauri/src/commands/` shows zero removed/modified `pub fn` signatures; all changes are internal or additive. Payload shapes: only *additive optional fields* (`AppSettings.s3_sync`, `ProxyConfig.request_media_fallback/request_media_heuristic` with serde defaults, `src/types/subscription.ts` +3 optional fields, `types/usage.ts` adds `opencode` data-source variant). msw mirrors keep working; only new mocks needed for S3. |
| New FE↔BE events | **1**: `s3-sync-status-updated` (payload `{status, error?, source}`, mirrors `webdav-sync-status-updated`). Emitted from new `services/s3_auto_sync.rs` via raw `app.emit` (`AppHandle` + `Emitter`) — desktop twin pattern; web stub twin emits nothing (same as WebDAV today). FE subscribes in `App.tsx` — fork must use `listen` from `@/lib/api/event-adapter` (SSE-backed), not upstream's `useTauriEvent` hook (fork doesn't have it). |
| Schema migrations | **No conflict.** Upstream `SCHEMA_VERSION` at v3.16.2 is still **10** (fork is also 10). No `set_user_version` / version-bump anywhere in the delta. Only change: one new `seed_model_pricing` row `("minimax-m3", "MiniMax M3", "0.60", "2.40", "0.12", "0")` in `database/schema.rs` (`43ae1e5f`). Seeding is `INSERT OR IGNORE` and runs from `create_tables` (schema.rs:645) — lands on existing DBs. Without it, MiniMax M3 usage cost is silently 0 (known fork pricing-lockstep trap). |
| ClaudeDesktop | Fork's `AppType` = {Claude, Codex, Gemini, OpenCode, OpenClaw, Hermes} — **no ClaudeDesktop**; `src-tauri/src/claude_desktop_config.rs` and `src/config/claudeDesktopProviderPresets.ts` don't exist. 8 commits touch CD files (strip list below). |
| Dual-runtime compile hazards | (a) `2a131a55` makes dual-compiled `services/provider/mod.rs` call **new `ProxyService` methods** (`lock_switch_for_app`, `hot_switch_provider_inner`) — the 141-line `proxy_web.rs` stub must grow matching methods or the web build breaks. (b) `database/mod.rs` update-hook gains `s3_auto_sync::notify_db_changed(table)` — needs the `s3_auto_sync_web` stub + alias so the web build resolves it. (c) `0527002c` adds OpenCode arm to `commands/usage.rs::sync_session_usage` — fork's `web_api/handlers/usage.rs::sync_session_usage` (lines 169-199) **manually mirrors** the claude+codex+gemini arms and must gain the OpenCode arm too. |
| Locale parity gate | `fa17194d` (CCSub) adds i18n keys to **en/ja/zh only — no zh-TW** → would fail `pnpm check:locales` if ported as-is. All other i18n commits touch all four locales. |
| New Cargo deps | `hmac = "0.12"` (`2a24da51`, S3 SigV4 — `sha2` already present; S3 client is hand-rolled reqwest, no AWS SDK, tauri-free → web-compatible). `windows-sys 0.61` w/ `Win32_Globalization` (`ee69c836`) + `Win32_UI_Shell` (`8e7d167a`) — Windows desktop only. |

---

## New command details (web mirroring contract)

### S3 commands (`2a24da51`, `src-tauri/src/commands/s3_sync.rs`)

Upstream signatures (all return `Result<serde_json::Value, String>`):

```rust
#[tauri::command] pub async fn s3_test_connection(settings: S3SyncSettings,
    #[allow(non_snake_case)] preserveEmptyPassword: Option<bool>) -> Result<Value, String>
#[tauri::command] pub async fn s3_sync_upload(state: State<'_, AppState>) -> Result<Value, String>
#[tauri::command] pub async fn s3_sync_download(state: State<'_, AppState>) -> Result<Value, String>
#[tauri::command] pub async fn s3_sync_save_settings(settings: S3SyncSettings,
    #[allow(non_snake_case)] passwordTouched: Option<bool>) -> Result<Value, String>
#[tauri::command] pub async fn s3_sync_fetch_remote_info() -> Result<Value, String>
```

`S3SyncSettings` (settings.rs, `#[serde(rename_all = "camelCase")]`): `enabled: bool`,
`autoSync: bool`, `region`, `bucket`, `accessKeyId`, `secretAccessKey`, `endpoint`,
`remoteRoot` (default), `profile` (default), `status: WebDavSyncStatus` (reused type).
FE return types reuse WebDAV shapes: `WebDavTestResult`, `WebDavSyncResult`,
`RemoteSnapshotInfo | { empty: true }` (`src/lib/api/settings.ts` additions:
`s3TestConnection`, `s3SyncUpload`, `s3SyncDownload`, `s3SyncSaveSettings`, `s3SyncFetchRemoteInfo`).

Fork needs (modeled on the existing WebDAV mirror — `web_api/handlers/webdav.rs` routes
`/api/webdav/webdav-sync-upload` etc., `web-commands.ts` lines 861-880):

| Command | web_api route (proposed, new `web_api/handlers/s3.rs`) | web-commands.ts | msw |
| --- | --- | --- | --- |
| `s3_test_connection` | `POST /api/s3/s3-test-connection` | entry keyed `s3_test_connection` | mock `{success: true}` |
| `s3_sync_upload` | `POST /api/s3/s3-sync-upload` | `s3_sync_upload` | mock sync result |
| `s3_sync_download` | `POST /api/s3/s3-sync-download` | `s3_sync_download` | mock sync result |
| `s3_sync_save_settings` | `POST /api/settings/s3-sync-save-settings` (webdav's save lives under `/api/settings/`) | `s3_sync_save_settings` | mock `{success: true}` |
| `s3_sync_fetch_remote_info` | `POST /api/s3/s3-sync-fetch-remote-info` | `s3_sync_fetch_remote_info` | mock `{empty: true}` |

Handler bodies can mirror `webdav.rs` 1:1, including the
`crate::services::s3_auto_sync::AutoSyncSuppressionGuard::new()` usage in download
(webdav.rs:170 precedent — guard resolves to the web stub via the mod alias).
Upstream also adds 16 lines to `tests/integration/App.test.tsx` — porting them requires the msw
mocks above.

### `ensure_claude_desktop_official_provider` (`0960fd71`, `commands/provider.rs`)

```rust
#[tauri::command]
pub fn ensure_claude_desktop_official_provider(state: State<'_, AppState>) -> Result<bool, String>
// body: state.db.ensure_official_seed_by_id(CLAUDE_DESKTOP_OFFICIAL_PROVIDER_ID, AppType::ClaudeDesktop)
```

References `AppType::ClaudeDesktop` + `CLAUDE_DESKTOP_OFFICIAL_PROVIDER_ID` — **does not compile in
the fork** (no such variant/constant). FE side: `src/lib/api/providers.ts`
`invoke("ensure_claude_desktop_official_provider")`, called from `AddProviderDialog.tsx` /
`mutations.ts` only for the claude-desktop app id. **EXCLUDE whole commit.** (If it were ever
ported it would need `POST /api/providers/ensure-claude-desktop-official-provider` + web-commands
entry + msw mock; not applicable.)

---

## Event audit

- **New**: `s3-sync-status-updated` — emitted by `s3_auto_sync.rs::emit_auto_sync_status_updated`
  (desktop only, raw `app.emit`). Fork adaptation: keep raw emit in the desktop twin (exactly how
  `webdav_auto_sync.rs:88-102` already does it) and a silent web stub; OR thread `UiEventSink`
  through if web-mode auto-sync is wanted (see Decision 4). FE listener goes through
  `@/lib/api/event-adapter`'s `listen` in `App.tsx` (fork pattern at App.tsx:406-441 for
  `webdav-sync-status-updated`). Upstream also renames the FE payload interface
  `WebDavSyncStatusUpdatedPayload` → `SyncStatusUpdatedPayload` (shared by both listeners).
- **Unchanged**: `webdav-sync-status-updated`, `universal-provider-synced`,
  `proxy-official-warning`, usage events — no other `emit` additions in the delta
  (verified by grep over the full diff).

---

## ClaudeDesktop strip list (fork dropped the app)

Commits touching `claude_desktop_config.rs` and/or `claudeDesktopProviderPresets.ts`:

| Commit | CD content | Action |
| --- | --- | --- |
| `0960fd71` | Entire commit is the CD official-provider fix (config + command + FE dialog + 136-line test) | **EXCLUDE whole commit** |
| `084857ce` | Entire commit inside `claude_desktop_config.rs` (strip `[1m]` suffix before proxy route lookup) | **EXCLUDE whole commit** |
| `2985ad2c` | `claude_desktop_config.rs` +24 (rewrite CD proxy URL on resolved port); rest (`proxy/server.rs` +12, `services/proxy.rs` +168) is the portable ephemeral-port fix | **PORT-ADAPT: drop the claude_desktop_config.rs hunk** |
| `e458e77e` (CherryIN) | `claudeDesktopProviderPresets.ts` +16 | strip that file's hunk |
| `e96eab52` (SSSAiCode) | `claudeDesktopProviderPresets.ts` 12 lines | strip |
| `fa17194d` (CCSub) | `claudeDesktopProviderPresets.ts` +13 | excluded anyway (referral) |
| `5beb63e6` (CCSub align) | `claudeDesktopProviderPresets.ts` 26 lines | excluded anyway |
| `955ea26d` (Kimi affiliate) | `claudeDesktopProviderPresets.ts` 4 lines | strip if ported |

Also note `473f2197` and `c1dff066` touch `src-tauri/src/tray.rs` — fork *has* tray.rs
(desktop-gated whole-crate), so those hunks port fine; not a CD issue.

---

## Partner / referral / aggregator presets

| Commit | Content | Referral? | Recommendation |
| --- | --- | --- | --- |
| `e458e77e` CherryIN (#3643) | Plain `category: "aggregator"` preset across claude/codex/gemini/hermes/openclaw/opencode (+CD) + icon. `websiteUrl: https://open.cherryin.ai`, `apiKeyUrl: .../console/token` — **no ref/aff params, not isPartner** | No | PORT-ADAPT (strip CD file hunk) |
| `c1dff066` ZenMux (#2709) | Real feature: ZenMux Token Plan coding-plan provider (manual credentials, USD quota rich display). `coding_plan.rs` +146, `subscription.rs` +14, `commands/provider.rs` +147 (zenmux usage-script credential resolution — internal, no signature change), `tray.rs` +2, FE `codingPlanProviders.ts` adds `id: "zenmux"`, +3 optional fields in `types/subscription.ts`, icon | No referral params found | PORT-ADAPT (dual-compile check; tray hunk desktop) |
| `fa17194d` CCSub | Partner preset, `isPartner: true`, `partnerPromotionKey`, `apiKeyUrl: https://www.ccsub.net/register?ref=Y6Z8DXEA`, README sponsor rows, **i18n only en/ja/zh (breaks check:locales)** | **Yes (ref code)** | EXCLUDE per precedent (or port stripped — decision) |
| `5beb63e6` CCSub align | Reorders partner blocks; **deletes `tests/config/providerPresetOrder.test.ts` (fork has this file)** | follows CCSub | EXCLUDE; keep fork's test |
| `955ea26d` Kimi affiliate (#3809) | Adds `?aff=cc-switch` to existing Kimi/Moonshot preset `websiteUrl`/`apiKeyUrl` (fork HAS these presets, claudeProviderPresets.ts:320+) | **Yes (aff param)** | Decision: skip, or port without `?aff=` (then it's a no-op) |
| `e96eab52` SSSAiCode | Domain migration `sssaicode.com → sssaicodeapi.com` incl. endpoint nodes; the `?ref=DCP0SM` param is pre-existing in the fork's preset | Pre-existing | PORT-ADAPT (fork has 55 SSSAiCode references; strip CD hunk). Functional fix — old domain presumably dying |
| `bda625a4` APINebula opencode SDK fix | Changes preset fork **does not have** (APINebula/SudoCode/AtlasCloud/APIKEY.FUN were deferred in v3.16.0 sync; 0 matches in `src/config`) | n/a | EXCLUDE (no-op) |
| `0e6f2b39` sponsor ad swap | README*4 only | marketing | EXCLUDE (docs) |

---

## Pricing / preset lockstep

- `43ae1e5f` (MiniMax #3518): backend-only — new balance-query endpoint adaptation in
  `services/coding_plan.rs` (+280, incl. default-pricing fallback) + the `minimax-m3` seed row.
  FE `src/config` holds no pricing tables (verified: no `glm-4.7` matches in `src/`), so lockstep
  here = make sure the **Rust seed row ports together with the coding-plan code**, or MiniMax M3
  cost rows silently price at 0 (the `find_model_pricing_row` lossy-cleaning trap from the fork
  memory applies to lookups, `minimax-m3` is a clean id).
- `e891f5c8` (Zhipu coding plan): pairs `services/model_fetch.rs` +74 with FE preset edits in
  `codexProviderPresets.ts` / `hermesProviderPresets.ts` / `openclawProviderPresets.ts` /
  `opencodeProviderPresets.ts` + test update — port FE and Rust in the same commit.
- No other `seed_model_pricing` edits in the delta.

---

## S3 sync architecture (`2a24da51`, feat #1351 — the big one)

**New files** (all under `src-tauri/src/`):

| File | Lines | Tauri-coupled? | Role |
| --- | --- | --- | --- |
| `commands/s3_sync.rs` | 352 | `tauri::State` (commands layer, desktop-only in fork) | 5 commands above + secret-preserving merge helpers |
| `services/s3.rs` | 926 | **No** (reqwest + hand-rolled SigV4 via `hmac`/`sha2`, uses `crate::proxy::http_client` — already `#[path]`-included by `examples/web_proxy.rs:10`) | S3 client (virtual-hosted for AWS, path-style for custom endpoints) |
| `services/s3_sync.rs` | 319 | **No** | upload/download/check_connection/fetch_remote_info orchestration + global `sync_mutex()` |
| `services/s3_auto_sync.rs` | 270 | **Yes** — `tauri::{AppHandle, Emitter}`, `tauri::async_runtime::spawn` | debounced (1s, 10s max-wait) background upload worker; `notify_db_changed`, `AutoSyncSuppressionGuard`, `should_trigger_for_table` (providers/provider_endpoints/mcp_servers/prompts/skills/skill_repos/settings/proxy_config), emits `s3-sync-status-updated` |
| `services/sync_protocol.rs` | 648 | **No** | snapshot/archive protocol **extracted out of `webdav_sync.rs`** (-571 lines there), shared by WebDAV + S3 |

**Modified**: `database/mod.rs` (+1: update-hook now also calls
`s3_auto_sync::notify_db_changed`), `settings.rs` (+135: `AppSettings.s3_sync:
Option<S3SyncSettings>`, `get/set_s3_sync_settings`, `update_s3_sync_status`, validate/normalize),
`commands/settings.rs` (+74: secret-preserving merge in `merge_settings_for_save` + tests),
`lib.rs` (+9: `s3_auto_sync::start_worker(db, app.handle())` in setup + 5 command registrations),
`services/webdav_sync.rs` / `webdav_sync/archive.rs` (protocol extraction), `Cargo.toml` (+hmac).
FE: `WebdavSyncSection.tsx` (+~1100: `type SyncType = "webdav" | "s3"` selector with confirm
dialog on switch), `App.tsx` (S3 status listener), `useSettings.ts`, `settings.ts` API,
`types.ts` (+17), 101 i18n keys × 4 locales, `tests/integration/App.test.tsx` (+16).

**Auto-sync loop**: yes, its own loop, separate from WebDAV's. Worker started unconditionally in
upstream lib.rs setup; runs only when `s3_sync.enabled && s3_sync.auto_sync`.
**Fork adaptation (required)**: create `services/s3_auto_sync_web.rs` stub twin exporting
`AutoSyncSuppressionGuard`, `is_auto_sync_suppressed()`, `notify_db_changed(_table) {}` (exact
mirror of `webdav_auto_sync_web.rs`), and gate in `services/mod.rs`:
`#[cfg(feature = "desktop")] pub mod s3_auto_sync; #[cfg(not(feature = "desktop"))] pub mod
s3_auto_sync_web; #[cfg(not(feature = "desktop"))] pub use s3_auto_sync_web as s3_auto_sync;`.
`start_worker` call stays in desktop `lib.rs` (next to the webdav worker at lib.rs:827).

**Mutual exclusion with WebDAV**: there is **no hard backend exclusion** — `AppSettings` keeps
independent `webdav` and `s3_sync` blocks, and each auto-sync worker checks only its own
`enabled && auto_sync` flags. Exclusivity is FE-level: `WebdavSyncSection` renders a single
`SyncType` select (`webdav` | `s3`, initialized from `s3Config?.enabled`) with a pending-switch
confirm dialog. Both workers are started at boot upstream.

**Sequencing note**: the `sync_protocol.rs` extraction rewrites `webdav_sync.rs`. Fork's
`webdav_sync.rs` is only **+1 line** vs upstream base 8f83fa20 → the refactor applies nearly
clean; port S3 as one batch, don't split protocol extraction from S3.

---

## OpenCode usage sync (`0527002c`, #3215)

- New `services/session_usage_opencode.rs` (574 lines; reads OpenCode's sqlite via
  `opencode_config.rs::get_opencode_db_path()` — tauri-free, dual-compiles).
- `services/usage_stats.rs`: adds `'_opencode_session' → 'OpenCode (Session)'` provider label,
  adds `'opencode_session'` to the proxy-dedup `effective_usage_log_filter`, and `"opencode"` to
  `has_matching_proxy_usage_log`'s missing-cache-creation allowance (note: this delta does not
  literally touch a `KNOWN_APP_TYPES` const — the extension is via these three SQL/match sites).
- **No new command**: extends existing `commands/usage.rs::sync_session_usage` with an OpenCode
  arm. **Fork must mirror the arm in `web_api/handlers/usage.rs::sync_session_usage`
  (lines 169-199)**, which hand-copies the claude/codex/gemini arms today.
- `lib.rs`: adds "OpenCode usage initial sync" + "OpenCode usage periodic sync" `run_step`s to the
  desktop 60s timer block (lib.rs:964-1007 in fork). Fork's web runtime has no background usage
  timers (bootstrap.rs has none) — web mode gets OpenCode usage via the manual sync route; no web
  loop to extend.
- FE: `DataSourceBar.tsx` +1 source, `UsageHero.tsx` +4, `types/usage.ts` adds the
  `opencode` variant — fork has all these files; usage-dashboard filters pick the new source up
  from the data-source union.

---

## Codex takeover/proxy series — `proxy_web.rs` stub risk

Commits `2683af57, 3f59ab37, 60a9b330, c9cadd6e, 8bf16602, 0fbba426, d5328e52, ce993bae,
a04e72a2, 2a131a55, aeaa016c, b7499fc8, 8047f954, 2985ad2c` all center on
`services/proxy.rs` (desktop twin, 3504 lines in fork) + `codex_config.rs` (dual-compiled) +
`services/provider/{mod,live}.rs` (dual-compiled).

- **`2a131a55` is the dangerous one**: dual-compiled `services/provider/mod.rs` now calls
  `state.proxy_service.lock_switch_for_app(app_type.as_str())` (returns a lock guard) and
  `hot_switch_provider_inner(...)`. The fork's `services/proxy_web.rs` stub (141 lines) exposes
  neither → **web build breaks unless the stub gains both methods** (guard type must be
  constructible without a running proxy; check what `lock_switch_for_app` returns and stub it).
- `b7499fc8` extends hot-switch to refresh proxy-safe Live labels (provider/mod.rs comment-level
  + proxy.rs internals) — re-verify stub signature parity after.
- `2683af57`/`3f59ab37`: new settings field `preserve_codex_official_auth_on_switch`
  (serde default; first true, then flipped to off/opt-in) + FE `CodexAuthSettings.tsx` (new file),
  `SettingsPage.tsx`, `useSettingsForm.ts`, `lib/schemas/settings.ts`, `types.ts`. Additive.
- `60a9b330` is mostly `src-tauri/tests/*` — upstream test files; fork has
  `import_export_sync.rs`/`provider_commands.rs`/`provider_service.rs`/`support.rs` equivalents —
  port to keep desktop `cargo test` gate meaningful.
- `8047f954`, `8bf16602`, `ce993bae`, `d5328e52`, `c9cadd6e`: additions inside
  `services/proxy.rs`/`codex_config.rs` only — verbatim, desktop-compile surface; codex_config
  parts dual-compile (tauri-free).

## Codex chat-proxy transform series (dual-compiled `proxy/providers/*`)

`59683363 → d66030be → b4f262c7 → c2337d68 → ea95f39a → 6940a4b2 → f59fab6c → ea6123ad → 4f5250fc`
all stack changes in `transform_codex_chat.rs` / `streaming_codex_chat.rs` /
`codex_chat_history.rs` / `codex_chat_common.rs` / `transform.rs` — these modules are included in
the web build via `web_proxy.rs::providers`, so they compile in BOTH runtimes (tauri-free,
verified pattern). Port **in upstream order** as one batch; out-of-order cherry-picks will
conflict in `transform_codex_chat.rs`.

`f4e2c28a`, `f5acef32`, `27c41f74` touch `proxy/handlers.rs`/`error_mapper.rs`/`server.rs` —
**not** included by `web_proxy.rs` (desktop local-proxy server only); `27c41f74`'s new
`GET /models` + `/v1/models` routes live on the embedded proxy server, no collision with the
fork's `web_api` router.

`6692343d` (media fallback rectifier): new `proxy/media_sanitizer.rs` (703 lines) wired into
`proxy/mod.rs` (desktop) + `forwarder.rs` (desktop) + `proxy/types.rs` (+2 `ProxyConfig` fields,
**dual-compiled** — serde-defaulted, additive) + FE `RectifierConfigPanel.tsx` + 2 fields in
`lib/api/settings.ts`. Web build: `web_proxy.rs` does not include `mod.rs`/`forwarder.rs`, so
`media_sanitizer` can stay desktop-only — but confirm `types.rs` additions don't reference it
(they don't; fields only).

---

## Commit-by-commit classification (chronological)

Legend: **PV** = PORT-VERBATIM, **PA** = PORT-ADAPT, **EX** = EXCLUDE.

| # | SHA | Subject (short) | Class | Note |
| --- | --- | --- | --- | --- |
| 1 | `2683af57` | Codex auth preservation setting | PA | new settings field + FE; `services/proxy.rs` hunks desktop twin only |
| 2 | `ee69c836` | Windows version-probe garbled output | PV | `commands/misc.rs` (desktop layer) + windows-sys dep; cfg(windows) internals |
| 3 | `3f59ab37` | Preservation default off | PA | pairs with #1 |
| 4 | `41433cfa` | Codex restart hint | PV | FE + test |
| 5 | `0e6f2b39` | Swap sponsor ads | EX | README only (docs/referral) |
| 6 | `f4e2c28a` | Enrich Codex forwarding errors | PV | proxy handlers/error_mapper (desktop-only compile surface) |
| 7 | `60a9b330` | Refactor live-write routing + tests | PV | mostly `src-tauri/tests/*` |
| 8 | `c9cadd6e` | OAuth cleared in preserve-mode takeover | PA | proxy.rs (desktop) + codex_config + FE strings |
| 9 | `e02a2763` | kimi/moonshot thinking normalizer | PV | `proxy/providers/claude.rs` (dual-compiled) |
| 10 | `5ef72a20` | Codex CLI discovery + gpt-5.5 template | PV | `codex_config.rs` + new `src/resources/gpt5_5_template.json` (new dir in fork) |
| 11 | `0960fd71` | CD official provider fix | **EX** | ClaudeDesktop-only; incl. new command — see detail section |
| 12 | `afa09e12` | Per-app credentials for balance/coding-plan | PA | new `resolve_native_credentials` in root `provider.rs` (dual); verify fork's `web_api` subscription/usage-script handlers use the same resolution |
| 13 | `8bf16602` | Catalog JSON on provider switch | PV | proxy.rs additions only |
| 14 | `0fbba426` | Catalog wiped by live backfill | PV | `services/provider/live.rs` (dual) + FE + tests |
| 15 | `d5328e52` | Catalog lost on takeover-off restore | PV | proxy.rs + codex_config |
| 16 | `ce993bae` | OAuth cleared, mis-categorized provider | PV | proxy.rs |
| 17 | `a04e72a2` | Edit dialog masking live OAuth | PV | FE only + tests |
| 18 | `2a131a55` | Harden takeover ownership, serialize switch | **PA** | **proxy_web.rs stub must add `lock_switch_for_app` / `hot_switch_provider_inner`**; CHANGELOG hunk drop |
| 19 | `aeaa016c` | Takeover notice copy | PV | FE strings |
| 20 | `b7499fc8` | Refresh label on hot-switch | PA | provider/mod.rs (dual) + proxy.rs; re-check stub parity |
| 21 | `59683363` | Tool plugins over Chat Completions | PV | transform/streaming codex chat (dual) |
| 22 | `d66030be` | Stream custom tools native events | PV | same files, after #21 |
| 23 | `25951d81` | release 3.16.1 | PA | version metadata only (package.json/Cargo/tauri.conf); release notes + CHANGELOG excluded |
| 24 | `256b0499` | Codex auth guide docs | EX | docs |
| 25 | `c67494ba` | Release-note docs tweak | EX | docs |
| 26 | `693c3872` | User-manual refresh | EX | docs (fork holds docs deletions aside) |
| 27 | `b4f262c7` | reasoning_tokens always present | PV | codex chat (dual) |
| 28 | `43ae1e5f` | MiniMax balance API + default pricing | PA | coding_plan.rs + **schema seed row lockstep** |
| 29 | `c1dff066` | ZenMux Token Plan | PA | feature; tray hunk desktop; +3 optional FE subscription fields |
| 30 | `7811383b` | Relative model_catalog_json | PV | codex_config.rs |
| 31 | `73073454` | zh VS Code wording | PV | zh/zh-TW values only; keys unchanged → locales gate OK |
| 32 | `e891f5c8` | Zhipu coding-plan presets | PV | model_fetch.rs + FE presets + test, keep together |
| 33 | `ae90b534` | Isolate deeplink test home | PV | test-only |
| 34 | `e458e77e` | CherryIN preset | PA | plain aggregator, no referral; strip CD preset hunk |
| 35 | `ce538265` | Proxy panel error display | PV | FE 5 lines |
| 36 | `c2337d68` | Preserve custom tool metadata | PV | transform_codex_chat (dual) |
| 37 | `33eafbad` | Copilot whitespace threshold 20→500 | PV | `proxy/providers/streaming.rs` (dual) |
| 38 | `6692343d` | Media fallback rectifier | PA | new desktop proxy module; ProxyConfig +2 serde-default fields (dual); FE panel |
| 39 | `084857ce` | CD `[1m]` suffix strip | **EX** | entirely `claude_desktop_config.rs` |
| 40 | `f5acef32` | Codex 413 clarification | PV | proxy/handlers.rs (desktop) |
| 41 | `ea95f39a` | Drop tool_choice when tools empty | PV | transform_codex_chat (dual) |
| 42 | `2a24da51` | **S3-compatible cloud sync** | **PA** | see architecture section: 5 commands → web routes/web-commands/msw; `s3_auto_sync_web` stub; event via event-adapter; hmac dep |
| 43 | `0527002c` | **OpenCode session usage sync** | **PA** | mirror arm into `web_api/handlers/usage.rs`; lib.rs timers; FE source |
| 44 | `8047f954` | Skip backup/restore on placeholder | PV | proxy.rs |
| 45 | `ad030da3` | Zhipu quota → configured base URL | PV | coding_plan.rs (dual) |
| 46 | `dadefdee` | Input auto-capitalize off | PV | FE 4 lines |
| 47 | `8e7d167a` | Windows taskbar icon (AppUserModelID) | PV | desktop lib.rs + wix (fork has `wix/per-user-main.wxs`) + windows-sys feature |
| 48 | `03a9296c` | Usage stats UI polish | PV | FE UsageDashboard/UsageHero (fork has both); removes 1 i18n key ×4 |
| 49 | `473f2197` | Official subscription quota template | PA | commands/provider.rs + usage_cache (dual) + tray.rs (desktop) + FE; new `OFFICIAL_SUBSCRIPTION` constant |
| 50 | `bda625a4` | APINebula opencode SDK | **EX** | preset absent in fork (no-op) |
| 51 | `8e0e9ac3` | Inflated input_tokens in Claude stream | PV | `proxy/usage/parser.rs` (dual via web_proxy `usage`) |
| 52 | `3cd9a0de` | Normalize Anthropic system messages | PV | providers/claude.rs + forwarder 1-line + openclaw session provider (all present in fork) |
| 53 | `1392ef62` | README release-note links | EX | docs |
| 54 | `ab6266f7` | Windows tray residue on exit | PV | desktop lib.rs only |
| 55 | `2626eeeb` | Path separators in scan_dir_recursive | PV | services/skill.rs 1 line |
| 56 | `6716a4c4` | VS Code session previews | PV | session_manager/providers/codex.rs (dual) + FE sessions + new `sessions/utils.ts` fns + tests |
| 57 | `27c41f74` | GET /v1/models on local proxy | PV | embedded proxy server; no web_api collision |
| 58 | `aa09c9cb` | Normalize localhost listen addr | PV | FE ProxyPanel |
| 59 | `2985ad2c` | Resolve ephemeral port 0 | PA | **strip claude_desktop_config.rs hunk**; proxy/server.rs + proxy.rs port |
| 60 | `e96eab52` | SSSAiCode domain update | PA | fork has preset (55 refs); strip CD hunk; ref param pre-existing |
| 61 | `ea6123ad` | Cache reasoning across turns | PV | codex_chat_history.rs (dual) |
| 62 | `6940a4b2` | Truncated vs normal stream end | PV | streaming_codex_chat.rs (dual) |
| 63 | `f59fab6c` | input_file/input_audio mapping | PV | transform_codex_chat.rs (dual) |
| 64 | `5c36ae06` | Only block explicit official providers | PV | FE ProviderActions/ProviderCard |
| 65 | `4f5250fc` | Strip cache_control in OpenAI conversion | PV | proxy/providers/transform.rs (dual) |
| 66 | `f1118d37` | release 3.16.2 | PA | version metadata only; CHANGELOG/release notes excluded |
| 67 | `fa17194d` | CCSub partner preset | **EX** | referral `?ref=Y6Z8DXEA`; i18n missing zh-TW (breaks locales gate) |
| 68 | `5beb63e6` | CCSub partner-block align | **EX** | also deletes `providerPresetOrder.test.ts` — keep fork's copy |
| 69 | `955ea26d` | Kimi affiliate links | **EX*** | `?aff=cc-switch` on presets fork has — product decision (port stripped = no-op) |

Totals: 41 PV, 17 PA, 11 EX (5 docs-only, 2 ClaudeDesktop-only, 3 partner/referral, 1 no-op
preset). Release plumbing (`25951d81`, `f1118d37`) counted as PA (version metadata only).

---

## Proposed batch plan

1. **Batch 1 — small shared fixes (FE + trivial BE)** — `41433cfa, dadefdee, aa09c9cb, ce538265,
   73073454, 2626eeeb, ae90b534, 5c36ae06, 03a9296c, ee69c836`.
   *Risk: low.* Pure FE/test/1-liners; `ee69c836` adds windows-sys (verify desktop Linux/macOS
   builds unaffected; cfg(windows) internals).
2. **Batch 2 — shared proxy-core correctness (dual-compiled)** — `e02a2763, 33eafbad, 8e0e9ac3,
   3cd9a0de, 4f5250fc`.
   *Risk: low-medium.* Dual compile: run web example check after; these files are `#[path]`-included.
3. **Batch 3 — Codex chat-proxy transform series (ordered)** — `59683363, d66030be, b4f262c7,
   c2337d68, ea95f39a, 6940a4b2, f59fab6c, ea6123ad` + desktop-only `f4e2c28a, f5acef32, 27c41f74`.
   *Risk: medium.* Same-file stacking — port in upstream order; heavy test surface comes along.
4. **Batch 4 — Codex config/catalog/credentials** — `5ef72a20, 7811383b, afa09e12, 8bf16602,
   0fbba426, d5328e52`.
   *Risk: medium.* `afa09e12` needs a check that fork's web_api usage-script/subscription handlers
   pick up `resolve_native_credentials`; new `src/resources/` dir lands.
5. **Batch 5 — Codex auth preservation + takeover hardening** — `2683af57, 3f59ab37, 60a9b330,
   c9cadd6e, ce993bae, a04e72a2, aeaa016c, 2a131a55, b7499fc8, 8047f954, 2985ad2c` (strip CD hunk).
   *Risk: HIGH.* `2a131a55` forces `proxy_web.rs` stub extension (`lock_switch_for_app`,
   `hot_switch_provider_inner`); serialize-switch semantics must hold in web mode (stub guard).
   Largest Rust diff of the round (~1000+ lines in proxy.rs alone).
6. **Batch 6 — model/pricing/preset/coding-plan refresh** — `43ae1e5f, ad030da3, e891f5c8,
   c1dff066, 473f2197, e458e77e, e96eab52`.
   *Risk: medium.* Keep `seed_model_pricing` row with coding-plan code; strip CD preset hunks in
   `e458e77e`/`e96eab52`; `473f2197`/`c1dff066` tray hunks are desktop-only files.
7. **Batch 7 — S3 sync (big feature)** — `2a24da51` alone.
   *Risk: HIGH.* New `s3_auto_sync_web.rs` stub + services/mod.rs gates; 5 new web_api routes +
   web-commands.ts entries + msw mocks (`check:web-routes` gate); event listener via
   `event-adapter`; `sync_protocol.rs` extraction rewrites `webdav_sync.rs` (fork is +1 line vs
   base → near-clean); 101×4 i18n keys (locales gate); hmac dep; integration-test additions.
8. **Batch 8 — OpenCode usage sync** — `0527002c`.
   *Risk: medium.* Must mirror the OpenCode arm in `web_api/handlers/usage.rs` (handler
   hand-copies arms); lib.rs timer additions; usage-dedup SQL changes need `test:unit` +
   desktop `cargo test`.
9. **Batch 9 — media fallback rectifier** — `6692343d`.
   *Risk: medium.* Desktop-only module wiring; dual-compiled `ProxyConfig` field additions;
   decide whether web_proxy.rs should include `media_sanitizer` (not required).
10. **Batch 10 — sessions UI** — `6716a4c4`.
    *Risk: low.*
11. **Batch 11 — Windows desktop polish (optional)** — `8e7d167a, ab6266f7`.
    *Risk: low; desktop-only.* Skippable if fork doesn't ship Windows desktop builds.
12. **Batch 12 — version metadata → 3.16.2** — version fields from `25951d81` + `f1118d37` after
    batches 1-10 land (precedent: bump only once implied code is synced).

**Excluded outright**: `0e6f2b39, 256b0499, c67494ba, 693c3872, 1392ef62` (docs),
`0960fd71, 084857ce` (ClaudeDesktop), `bda625a4` (no-op), `fa17194d, 5beb63e6, 955ea26d`
(partner/referral — see decisions).

## Decision points (need maintainer input)

1. **CCSub partner preset** (`fa17194d`+`5beb63e6`): carries `?ref=Y6Z8DXEA` referral + partner
   promotion key + README sponsor rows, and its i18n misses zh-TW. Precedent says exclude.
   If excluded, also keep fork's `tests/config/providerPresetOrder.test.ts` (upstream deleted it).
2. **Kimi affiliate params** (`955ea26d`): fork has the Kimi presets; the only change is
   `?aff=cc-switch`. Port stripped (= no-op) or adopt upstream's affiliate tagging?
3. **CherryIN** (`e458e77e`): plain aggregator preset (no referral found) — port it, or treat all
   new third-party preset additions as product decisions?
4. **S3 auto-sync in web mode**: replicate the WebDAV precedent (silent `_web` stub → web users
   get manual S3 sync only, no background loop, no `s3-sync-status-updated` over SSE), or invest
   in a UiEventSink-based web worker now? Recommendation: stub first (parity with WebDAV),
   UiEventSink adaptation as a follow-up task.
5. **Windows desktop polish** (`8e7d167a`, `ab6266f7`, wix): port for desktop parity or skip as
   out-of-scope for the web-first fork?
6. **`preserve_codex_official_auth_on_switch`** ships off/opt-in after `3f59ab37` — accept
   upstream default (recommended) or fork-divergent default?
7. **Version bump timing**: bump to 3.16.2 after Batch 10, even if Batch 11 (Windows polish) is
   skipped? (Release notes/CHANGELOG stay excluded per fork docs policy.)

## Caveats / Not found

- `lock_switch_for_app`'s exact return type (guard) was not extracted — implementer must read
  `2a131a55`'s `services/proxy.rs` hunk when extending `proxy_web.rs`.
- Whether fork's `web_api` subscription/usage-script handlers need `afa09e12`'s
  credential-resolution mirrored was flagged but not fully traced (fork handler internals not read).
- Upstream has no `KNOWN_APP_TYPES` constant change in `usage_stats.rs`; the OpenCode extension is
  via three SQL/match sites (documented above) — if the fork added such a constant locally, extend
  it manually.
- msw `tests/msw/handlers.ts` currently has no WebDAV mocks (WebDAV FE paths aren't exercised in
  tests); S3 mocks are only needed if the ported `App.test.tsx` additions exercise S3 flows.
