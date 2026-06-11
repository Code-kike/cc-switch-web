# Sync upstream cc-switch v3.16.1 + v3.16.2

## Goal

Port upstream `farion1231/cc-switch` v3.16.1 and v3.16.2 changes into this Web-first fork,
continuing the granular `sync:` commit methodology from `05-21-sync-upstream-v315` and
`05-30-sync-upstream-v316`. Adapt event-based and command-surface changes to the fork's
dual-runtime (Tauri desktop + web-server) transport. Bump version metadata to 3.16.2 only after
the code lands and gates pass.

## Range & baseline

- Previous sync point: upstream main `8f83fa20` (just past v3.16.0 tag `47232cb0`) — fork is at v3.16.0.
- Target: upstream release tag **v3.16.2** = `955ea26d` (published 2026-06-08). Upstream main (`edc597ab`) is already past the tag; we anchor on the tag for a stable cut (decision pending confirmation).
- Delta `8f83fa20..v3.16.2` = **69 commits** (~178 files, +19k/−3.1k incl. docs).
- Tags fetched locally 2026-06-10: `v3.16.1`, `v3.16.2` (local `v3.16.0` == upstream `v3.16.0`).
- **No blind merge** (spec `quality-guidelines.md` §"Upstream Desktop Sync Into Web Fork"): port targeted slices; preserve Web-only files (`web_api/**`, `examples/server.rs`, `src/lib/api/web-commands.ts`, `runtime/**`, `bootstrap.rs`, `deploy/`, `scripts/`).

## What I already know

- **v3.16.1** (23 commits): Codex stability patch — optional official-auth preservation toggle (default off), Codex `modelCatalog` DB-as-SSOT fixes (no more silent wipes on live backfill/hot-switch/takeover-restore/edit), Chat Completions tool/plugin → Responses event restoration, per-app serialized switch+takeover with placeholder-based ownership detection, diagnostics & Windows tool-detection fixes.
- **v3.16.2** (41 commits): S3-compatible cloud sync (second backend besides WebDAV; AWS SigV4 self-impl; presets for S3/MinIO/R2/OSS/COS/OBS; mutually exclusive with WebDAV auto-sync), OpenCode session usage sync (reads OpenCode local SQLite; new app filter tab + source tag), official-subscription quota templates (explicit opt-in, replaces implicit official queries), text-model image-fallback rectifier (+settings toggles), Codex `/v1/models` probe endpoint with stale-catalog guard, Codex Chat file/audio attachments, proxy robustness fixes (port-0 parsing, takeover placeholder restore loop, Anthropic `system` normalization, 413 copy, Claude Desktop `[1m]` routing), Windows/macOS platform fixes, CherryIN + ZenMux providers, usage dashboard hero redesign, trilingual user-manual refresh.
- **New `#[tauri::command]`s in delta (6)**: `s3_test_connection`, `s3_sync_upload`, `s3_sync_download`, `s3_sync_save_settings`, `s3_sync_fetch_remote_info` (S3) + `ensure_claude_desktop_official_provider` (ClaudeDesktop). Any ported command needs: web_api handler route + `web-commands.ts` entry + msw handler, gated by `pnpm check:web-routes`.
- **ClaudeDesktop is upstream's 7th managed app since v3.16.0; the fork explicitly dropped it** (0 references at HEAD). All delta hunks touching ClaudeDesktop must be adapted/excluded (spec sanctions documented deferral).
- Fork-specific divergences that complicate verbatim ports: `UiEventSink` event abstraction (web SSE), WebDAV auto-sync has a `_web` stub twin (S3 auto-sync presumably needs the same), usage stats `KNOWN_APP_TYPES` currently claude/codex/gemini only (OpenCode usage sync extends this), pricing FE-presets ↔ `seed_model_pricing` manual parity.
- Precedent exclusions (v3.16.0 sync): partner/referral presets excluded ("fork stays referral-neutral"), docs/user-manual/release-marketing excluded, version bump last.

## Assumptions (temporary)

- Anchor on tag v3.16.2, not upstream main HEAD.
- ClaudeDesktop stays deferred (port only shared bits needed to compile).
- Web runtime keeps proxy stubbed; proxy-area fixes are still ported for shared-code parity + desktop use.

## Decisions (ADR-lite, confirmed by maintainer 2026-06-10)

**D1 — Scope: full alignment to v3.16.2.**
Context: 69-commit delta with several large features; trimmed alternatives would leave version metadata in a dishonest in-between state and grow drift.
Decision: port the full shared-code surface of v3.16.1 + v3.16.2 (Codex stability, S3 sync, OpenCode usage sync, subscription quota templates, image-fallback rectifier, `/v1/models`, attachments, proxy/platform fixes). Fixed exclusions stand: upstream docs/user-manual/release material, ClaudeDesktop as a managed app.
Consequences: largest batch of the three options; S3's 5 new commands require web routes + `web-commands.ts` + msw mirrors.

**D2 — CherryIN and ZenMux: both included.**
Context: precedent excludes referral content, but both commits (`e458e77e`, `c1dff066`) are referral-param-free; the fork already ships AiHubMix (same aggregator category); ZenMux is functional coding-plan/quota code, not just a preset.
Decision: port both. Referral params (`ref=`, `aff=cc-switch`, etc.) found in OTHER preset-refresh commits are stripped per precedent.
Consequences: preset catalogs gain two entries; referral-stripping must be done hunk-by-hunk in the preset batch.

**D3 — S3 sync on web follows the WebDAV pattern.**
Context: desktop `webdav_auto_sync.rs` depends on `tauri::{AppHandle, Emitter}`, hence its web no-op twin; manual WebDAV commands have full web routes (`web_api/handlers/webdav.rs`).
Decision: the 5 manual S3 commands get full web routes (usable from the web UI); the S3 auto-sync background loop stays desktop-only with a `_web` stub twin mirroring `webdav_auto_sync_web.rs`.
Consequences: web server instances need manual S3 upload after config changes. Follow-up task (not this one): make WebDAV+S3 auto-sync tauri-free via `UiEventSink` so server deployments auto-backup.

## Open Questions

- (resolved 2026-06-10) Audit complete — no schema-migration conflicts (upstream still SCHEMA_VERSION 10); no breaking command-signature changes (additive-optional only); one new event `s3-sync-status-updated`. Remaining minor defaults recorded in D4-D6 pending final confirmation.

## Decisions (continued)

**D4 — Windows desktop polish (Batch 11: `8e7d167a`, `ab6266f7`): SKIPPED** (confirmed 2026-06-10). Maintainer's deployment is Linux server + browser access from Win10 — no Windows desktop app in use. Recorded as documented deferral; revisit only if Windows desktop builds are ever distributed.
**D5 — `preserve_codex_official_auth_on_switch` default**: accept upstream default (off / opt-in) — align-with-upstream principle.
**D6 — Version bump timing**: bump to 3.16.2 after Batch 10 regardless of Batch 11; release-notes/CHANGELOG stay excluded per fork docs policy.
**D7 — CCSub exclusion side-effect**: keep fork's `tests/config/providerPresetOrder.test.ts` (upstream deleted it alongside the excluded CCSub preset); Kimi `?aff=` change ports as a no-op (stripped).

**D8 — Codex Chat Completions routing stack: deferral CONTINUES (confirmed 2026-06-10).**
Context: the audit's batch-3 premise was wrong — the fork never ported the Codex Chat routing stack (`transform_codex_chat.rs`, `streaming_codex_chat.rs`, `codex_chat_history.rs`, `codex_chat_common.rs`, ~5.7k lines; 14 pre-range commits, deferred since the 05-21 v3.15 sync) nor the Codex model-catalog feature (`791ced00`/`ad8bdf16`, also pre-range). 8 of 11 batch-3 commits and most of batch 4 patch those absent bases. The stack only runs inside the desktop-only local proxy — inert in the maintainer's web deployment.
Decision: keep the deferral; created follow-up task for the full stack port. This round ports only what stands without the base (batch 3 delivered f4e2c28a f5acef32 27c41f74 adapted; batch 4/5 to be trimmed the same way).
Consequences: v3.16.2 version bump ships without Codex Chat tool/plugin restoration, stream-truncation fix, attachments, reasoning-token accounting, and catalog-restore behaviors (all recorded below in Out of Scope). `chat_error_to_response_error` lives inlined in `handlers.rs` and `handle_models` returns the guard-declined fallback until the stack lands.

## Implementation Plan (12 batches, from audit)

| # | Batch | Commits | Risk |
|---|-------|---------|------|
| 1 | Small shared fixes (FE + trivial BE) | `41433cfa dadefdee aa09c9cb ce538265 73073454 2626eeeb ae90b534 5c36ae06 03a9296c ee69c836` | low |
| 2 | Shared proxy-core correctness (dual-compiled) | `e02a2763 33eafbad 8e0e9ac3 3cd9a0de 4f5250fc` | low-med |
| 3 | Codex chat-proxy transform series (port in upstream order) | `59683363 d66030be b4f262c7 c2337d68 ea95f39a 6940a4b2 f59fab6c ea6123ad` + desktop-only `f4e2c28a f5acef32 27c41f74` | med |
| 4 | Codex config/catalog/credentials | `5ef72a20 7811383b afa09e12 8bf16602 0fbba426 d5328e52` | med |
| 5 | Codex auth preservation + takeover hardening | `2683af57 3f59ab37 60a9b330 c9cadd6e ce993bae a04e72a2 aeaa016c 2a131a55 b7499fc8 8047f954 2985ad2c` (strip CD hunk) | **HIGH** — `2a131a55` forces `proxy_web.rs` stub extension (`lock_switch_for_app`, `hot_switch_provider_inner`) |
| 6 | Model/pricing/preset/coding-plan refresh (incl. ZenMux, CherryIN; strip CD + referral hunks) | `43ae1e5f ad030da3 e891f5c8 c1dff066 473f2197 e458e77e e96eab52` | med — keep `seed_model_pricing` minimax-m3 row with coding-plan code |
| 7 | S3 sync (big feature) | `2a24da51` | **HIGH** — `s3_auto_sync_web.rs` stub + services/mod.rs cfg gates (db update-hook calls `s3_auto_sync::notify_db_changed`); 5 web routes + web-commands.ts + msw; `s3-sync-status-updated` via event-adapter; `sync_protocol.rs` extraction rewrites `webdav_sync.rs`; 101×4 i18n keys; hmac dep |
| 8 | OpenCode usage sync | `0527002c` | med — hand-mirror OpenCode arm into `web_api/handlers/usage.rs::sync_session_usage` |
| 9 | Media fallback rectifier | `6692343d` | med — desktop-only wiring; dual-compiled `ProxyConfig` field additions |
| 10 | Sessions UI | `6716a4c4` | low |
| 11 | Windows desktop polish (D4) | `8e7d167a ab6266f7` | low, desktop-only |
| 12 | Version metadata → 3.16.2 | fields from `25951d81 f1118d37` | after 1-10 green |

Excluded outright: `0e6f2b39 256b0499 c67494ba 693c3872 1392ef62` (docs), `0960fd71 084857ce` (ClaudeDesktop), `bda625a4` (no-op), `fa17194d 5beb63e6 955ea26d` (partner/referral).

Implementer caveats (from audit "Caveats"): read `2a131a55`'s `services/proxy.rs` hunk for `lock_switch_for_app`'s guard type before extending `proxy_web.rs`; verify whether fork's web_api subscription/usage-script handlers need `afa09e12`'s `resolve_native_credentials` mirrored; msw S3 mocks only needed if ported `App.test.tsx` additions exercise S3 flows.

## Requirements (evolving)

- Port upstream delta `8f83fa20..v3.16.2` as focused, batched `sync:` commits adapted to dual-runtime.
- S3 manual commands (`s3_test_connection`, `s3_sync_upload`, `s3_sync_download`, `s3_sync_save_settings`, `s3_sync_fetch_remote_info`) exposed via new web_api routes + `web-commands.ts` entries + msw mocks; S3 auto-sync gets a web stub twin.
- CherryIN preset + ZenMux coding-plan provider ported; referral params in other preset commits stripped.
- OpenCode usage sync ported for both runtimes (service must stay tauri-free; new dashboard filter tab included).
- Subscription quota templates + image-fallback rectifier + Codex `/v1/models` + attachments + proxy/platform fixes ported.
- Preserve all Web-only runtime files; never regress route coverage (0 missing).
- Version metadata bump to 3.16.2 across `package.json`, `src-tauri/Cargo.toml`, `Cargo.lock`, `tauri.conf.json` — together, last.

## Acceptance Criteria (final — all verified 2026-06-11, trellis-check verdict: SHIP)

- [x] All in-scope batches ported as `sync:` commits; out-of-scope items documented here (11 commits f48138c0..91c1b55c; batches 3/4/5 trimmed per D8, batch 11 skipped per D4).
- [x] New/changed commands have web routes + `web-commands.ts` + msw coverage; `pnpm check:web-routes` 0 missing (command surface 261→266, all 5 S3 commands POST+Json).
- [x] `pnpm typecheck`, `pnpm format:check`, `pnpm test:unit` (587), `pnpm check:locales` green.
- [x] `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked` (1502) green.
- [x] `cargo check --no-default-features --features web-server --example server --locked` green; web tests 9/9 + dual_runtime_parity green.
- [x] `gen-command-manifest.sh --check` green (266); `pnpm smoke:web-server` (×2) + `pnpm test:integration` (50/50) green.
- [x] Version strings consistent at 3.16.2 (all four files; /api/health verified live).

## Definition of Done

- Tests added/updated for ported behavior (both runtimes where applicable).
- Full local gate suite green (see Acceptance Criteria).
- Deferred/excluded upstream items recorded in this PRD.

## Out of Scope (explicit, evolving)

- Docs-only upstream material (user-manual, guides, release marketing, images).
- Claude Desktop as a managed app (deferred again; only compile-necessary shared bits).
- **Codex Chat Completions routing stack + model-catalog feature (D8)** — base never ported (deferred since v3.15 sync); this round's dependent commits skipped: `59683363 d66030be b4f262c7 c2337d68 ea95f39a 6940a4b2 f59fab6c ea6123ad` (chat stack) + catalog-only hunks of batch 4/5. Follow-up task: `port-codex-chat-routing-model-catalog-stack`.
- Windows desktop polish `8e7d167a ab6266f7` (D4 — maintainer runs no Windows desktop builds).
- Web auto-sync for S3/WebDAV (D3 follow-up).
- Pushing to remote without maintainer say-so.

## Research References

- [`research/upstream-v3161-v3162-audit.md`](research/upstream-v3161-v3162-audit.md) — commit-by-commit audit of `8f83fa20..v3.16.2` (in progress).
- Precedent: `.trellis/tasks/archive/2026-06/05-30-sync-upstream-v316/prd.md` (+ its research) — methodology source.

## Technical Notes

- **Spec-update DONE (2026-06-11)**: both TODOs executed via trellis-update-spec — "Codex Provider OAuth Preservation" now documents the `preserve_codex_official_auth_on_switch` gate (default OFF) across Contracts/Validation/Good-case; "Provider Usage Query Templates" now lists 4 built-ins incl. `official_subscription` with its sanctioned `subscriptionApi.getQuota` test path + zenmux paired-credential rule; NEW scenario "Desktop-Only Service Worker Twin Stubs" captures the webdav/s3 auto-sync cfg-pair pattern.
- **Release-note-worthy behavior change**: third-party Codex switches now overwrite `auth.json` by default (upstream parity); preservation is opt-in via Settings → Codex App Enhancements.
- **Integration-test runner caveat**: `pnpm test:integration` spawns `cargo run` — shells need `~/.cargo/bin` on PATH; the new `codex_official_to_deepseek` test binds the real default proxy port 15721.
- Spec contract: `.trellis/spec/frontend/quality-guidelines.md:666` "Upstream Desktop Sync Into Web Fork".
- Port with `git show <sha> -- <paths>` / targeted apply, dependency order: backend/types/services → proxy/provider → presets → events/UI → version bump.
- Event parity pattern: `runtime/runtime_events.rs` (`UiEventSink`/`ChannelEventSink`), `web_api/handlers/system.rs` SSE, FE `src/lib/api/event-adapter.ts` + bridge hooks.
- cfg-pair pattern for desktop-only services: `services/proxy.rs`/`proxy_web.rs`, `webdav_auto_sync.rs`/`webdav_auto_sync_web.rs` — S3 auto-sync likely needs the same twin.
- msw third copy: `tests/msw/handlers.ts` must mirror any new command/route.
