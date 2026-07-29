# S3 report — MCP / config-sync hardening (upstream v3.17.0)

Branch `sync/upstream-v3.18.0`. All work is **staged, not committed** (per brief).
Checkpoint log: `S3-progress.md`. End state: zero conflict markers, `git ls-files -u`
empty, no cherry-pick/merge state, `git diff --check` clean, **20 files staged**.

Run context: this batch was executed across two agent runs (the first was killed after
item 10). Items 1–10 are the first run's work as recorded in `S3-progress.md`; items
11–12 plus the gate run and fmt repair are the resume run.

## Per-commit outcome

| # | Hash | Status | Notes |
|---|---|---|---|
| 1 | ffc22ea7 | **ported (adapted)** | universal-provider save-then-sync + toast split; addSuccess key dropped |
| 2 | e191af4a | **ported (adapted)** | OpenCode live import updates existing providers |
| 3 | e78aa8a7 | **ported (adapted)** | OpenClaw + Hermes live import updates; fork strip keys preserved |
| 4 | 8b1ce764 | **ported (convergent)** | fork already failed closed; upstream rationale + richer assertion added |
| 5 | 93f56198 | **ported (convergent)** | fork already stripped `[mcp_servers]` on backfill |
| 6 | 473c2aaa | **ported (convergent)** | fork strip machinery stronger; richer assertions ported |
| 7 | 6d2ee247 | **excluded (trigger) / adapted (mechanism)** | unified-session toggle unported; mechanism already present |
| 8 | 1f36f0cf | **ported (adapted)** | switch-time common-config autosync, Codex-only by design |
| 9 | 11c173c7 | **ported (convergent)** | best-effort aggregate projection already present; 1 missing regression ported |
| 10 | 94fc1cc0 | **ported (adapted)** | per-app import failures surface aggregate error; partials persist |
| 11 | 88d5ffba | **ported (convergent)** | backend toml_edit merge fully pre-existing; richer test assertions ported |
| 12 | 6245caa6 | **ported** | OpenCode limit/headers editors; near-clean cherry-pick |

**12 of 12 addressed** (1 mechanism-only exclusion recorded under item 7).

## Item detail — first run (1–10, from the checkpoint log)

1. **ffc22ea7** — `AddProviderDialog` resolved semantically: kept the fork's
   non-Claude-Desktop app surface and failure handling, added upstream's save-then-sync
   flow with success/warning toasts (`added`/`addedAndSynced`/`addedButSyncFailed`),
   removed the obsolete `addSuccess` key from en/ja/zh. zh-TW stays excluded (fork
   deleted Claude Desktop scope).
2. **e191af4a** — OpenCode live import now updates existing provider settings while
   preserving the stored display name when live omits one. Upstream's startup
   log/comment changes were moved from the obsolete desktop-only `lib.rs` block into
   the shared tauri-free `bootstrap.rs` so desktop/web stay in parity. Focused test 1/1.
3. **e78aa8a7** — OpenClaw and Hermes live imports update existing DB providers;
   shared bootstrap logs sync outcomes. Fork Hermes protections preserved by stripping
   `_cc_source`/`provider_key` before both comparison and persistence; reused the
   fork's existing Hermes test helper. Focused tests 3/3; rustfmt PASS.
4. **8b1ce764** (convergent) — the fork's MCP audit had already made Codex MCP sync
   fail closed on unparseable `config.toml` with managed atomic writes. Kept the
   stronger fork implementation and its remove-path regression; added upstream's
   rationale and a richer single-sync assertion that the error names `config.toml`
   and that bytes are left untouched. Focused test 1/1.
5. **93f56198** (convergent) — the fork's authoritative-projection audit already
   stripped `[mcp_servers]` plus legacy `[mcp.servers]` during Codex backfill with
   byte-identical no-op behavior and integration coverage. Added upstream's
   orphan-resurrection rationale; excluded unrelated unified-session/model-catalog
   conflict context. Focused tests 3/3.
6. **473c2aaa** (convergent) — fork already stripped Codex routing/MCP/credential
   artifacts from common-config extraction. Preserved its stronger rule that only
   cc-switch's generated `model_catalog_json` path is removed (user custom catalog
   paths remain shareable); retained the S2 strip keys
   (`strip_injected_codex_oauth_context_defaults`, fable/subagent env keys); ported
   richer rationale/assertions for MCP, `wire_api`, bearer token, catalog sentinel,
   and web_search sentinel. Focused tests 3/3. Claude-side generic credential
   scrubbing stays deferred per the brief (see Known gap below).
7. **6d2ee247** — upstream's hook is exclusively the unported unified Codex-session
   toggle, so `reapply_current_codex_official_live`, its re-export, and its
   official-routing tests were NOT added (trigger excluded). The fork already has the
   required targeted `sync_enabled_for_app` mechanism plus stronger complete
   Codex-table replacement, empty-set orphan clearing, fail-closed TOML, and
   cross-app aggregate sync; a generic contract comment was added. Focused tests 3/3.
   The spec already records: "6d2ee247 is not directly applicable because this fork
   lacks unified Codex session history."
8. **1f36f0cf** — ported switch-away live common-config re-extraction before
   value-matched backfill plus both Codex end-to-end tests (new shared keys /
   secret-artifact isolation, deletion propagation). Deliberately **Codex-only**:
   upstream's pre-existing Claude autosync prerequisite is absent in this fork, and
   enabling it against the fork's known-unscrubbed OPENROUTER/GOOGLE/OPENAI/GEMINI/
   AWS credentials would create automatic cross-provider secret propagation.
   Focused tests 2/2.
9. **11c173c7** (convergent) — the fork's MCP atomicity audit already had best-effort
   aggregate all-app projection, target-only save/switch projection, target-only
   sync-current, deferred aggregate error after Skill sync, complete Codex
   replacement, and non-fatal post-write projection. Ported fuller contracts and the
   missing switch-vs-broken-Claude regression; reused the existing aggregate
   regression. Focused tests 2/2.
10. **94fc1cc0** — all five app importers still run best-effort, but any per-app
    failure now returns an aggregate error containing the already-persisted count and
    the failed app(s); partial successes remain in DB. Kept the fork's
    importer-function table, ported the upstream partial-failure regression, updated
    prior all/partial-failure tests, and changed React Query invalidation to
    `onSettled` so error responses still refresh imported rows. Focused tests 3/3;
    command/web handler already delegated to the shared service.

## Item detail — resume run (11–12)

### 11. 88d5ffba — backend toml_edit common-config merge (CONVERGENT)

The fork already ported this commit's entire substance during its own C5/atomicity
audit ("fix(codex): make common config and MCP projection atomic"; the contract is
codified in `.trellis/spec/frontend/quality-guidelines.md` under "Codex Common
Configuration and MCP Derived-State Atomicity"). Verified present, piece by piece:

- Backend `update_toml_common_config_snippet` in `services/provider/live.rs`
  (byte-equivalent logic to upstream's, English doc comment), backed by the same
  `merge_toml_table_like` / `remove_toml_table_like` used by live writes.
- Tauri command in `commands/config.rs`, registered in `lib.rs`, re-exported in
  `services/provider/mod.rs`.
- **Fork-only extra upstream lacks**: web handler
  (`web_api/handlers/config.rs`, `POST /api/config/update-toml-common-config-snippet`)
  + `web-commands.ts` registry entry — route coverage green.
- FE: the smol-toml `updateTomlCommonConfigSnippet` helper is deleted from
  `providerConfigUtils.ts` (with the do-not-reintroduce comment), `configApi.
  updateTomlCommonConfigSnippet` invokes the backend, and `CodexCommonConfigModal` /
  `CodexConfigEditor` carry the async `boolean | Promise<boolean>` signatures.
- Hook `useCodexCommonConfig.ts` is a strict **superset** of upstream's guards: it has
  upstream's per-hook operation sequence + config-baseline staleness check, PLUS the
  fork's serialized durable-save queue, preset-identity invalidation, unmount
  invalidation, and structured error formatting via `extractErrorMessage`.
- FE regressions: upstream's two new race tests exist under fork titles in
  `tests/hooks/useCommonConfigSave.test.tsx` ("keeps the latest Codex toggle when
  backend TOML operations resolve out of order", "does not overwrite a manual Codex
  config edit with an in-flight merge") plus three fork-only ones (serialized saves,
  preset-switch invalidation, unmount invalidation).

**Only delta ported** (the S3 convergent-commit precedent: take upstream's richer
rationale/assertions): in `live.rs` tests —

- `update_toml_common_config_snippet_preserves_comments_and_key_order` gains the
  no-synthesized-empty-parent-header C5 lock
  (`!merged.contains("[model_providers]\n")`), the merged-value assertion
  (`notifications = true`), the survivor-key assertion after removal
  (`disable_response_storage = true`), and upstream's assertion messages.
- `update_toml_common_config_snippet_overrides_and_removes_by_value` (fork's name for
  upstream's `..._scalar_override_and_value_matched_removal`) gains upstream's
  rationale doc comment and assertion messages.

Focused tests PASS 2/2. No conflicts (no cherry-pick attempted — pure delta edit).

### 12. 6245caa6 — OpenCode known field editors

`git cherry-pick -n 6245caa6` auto-merged everything except one conflict:

- **Conflict**: `src/i18n/locales/zh-TW.json` modify/delete — resolved with `git rm`;
  zh-TW **stays deleted** (established S1/S2/S3 fork scope).
- The fork's `OpenCodeFormFields.tsx` diverges from upstream's parent only by keeping
  a **local inline `ModelDropdown`** (upstream extracted it to `./shared`; the fork
  never did). The auto-merge preserved it untouched (definition + usage verified).

What the commit adds (taken verbatim):

- **Model limit editor**: per-model `limit.context` / `limit.output` numeric fields in
  the expanded model details; empty-clears delete the key, and an empty `limit` object
  is removed from the model.
- **Headers editor**: structured `options.headers` editing (add/rename/remove/value),
  case-insensitive duplicate-name rejection with input restore, hydrate-from-config.
- **Draft-key prefixes**: `OPENCODE_HEADER_DRAFT_PREFIX = "draft-header:"` and
  `OPENCODE_EXTRA_OPTION_DRAFT_PREFIX = "draft-option:"` (`:` is invalid in an HTTP
  field name so it cannot collide) replace the old `option-` placeholder convention —
  legitimate user keys named `option-*` / `header-*` are no longer silently dropped
  on save (`!k.startsWith(prefix)` on the raw key, not the trimmed one).
- **Layout**: extra options become a `Collapsible` (auto-opens when options exist);
  headers/extra-options gain hint copy; model-details toggle gains an aria-label.
- `useOpencodeFormState` gains `opencodeHeaders` + `handleOpencodeHeadersChange`
  (writes trimmed non-draft keys; removes `options.headers` when empty) and resets
  headers in `resetOpencodeState`; `ProviderForm` threads the two new props (single
  call site in the fork).
- Locales: 13 new `opencode.*` keys each in en/ja/zh (zh-TW dropped with the file).
- New test files taken verbatim: `tests/components/OpenCodeFormFields.test.tsx`
  (9 tests) and `tests/hooks/useOpencodeFormState.test.tsx` (5 tests). Compatibility
  verified before running: `OpenCodeModel.limit` and `OpenCodeProviderOptions.headers`
  already exist in `src/types.ts`, `@/components/ui/collapsible` exists, and the
  fork's props interface matches the test fixtures. All 14 PASS.

No backend changes; no new Tauri command (web route SSOT untouched by this item).

## Fork behavior re-applied / preserved across the batch

- Atomic/managed writers and fail-closed invalid-TOML handling (ADR-0003/0004) kept
  wherever upstream hunks landed on audited code; upstream non-atomic writes were not
  reintroduced.
- Authoritative DB→live MCP projection (complete-table replacement, empty-set orphan
  clearing, rollback/compensation) preserved; no regression to live-derived state.
- Strip machinery fully retained: `provider_common_config_strip_opt_in`,
  `strip_common_config_for_backfill`, S2's
  `strip_injected_codex_oauth_context_defaults` + fable/subagent keys, and the
  user-custom-catalog-path shareability rule.
- Hermes `_cc_source`/`provider_key` stripping before comparison and persistence.
- zh-TW.json and `claudeDesktopProviderPresets.ts` stay deleted.
- Web constraints intact: no new command (item 11's command already had its route);
  route coverage `missing: 0`; no auth/updater surface touched.

## Deferrals / known gaps (for the team lead — none landed here)

1. **Claude-side generic credential scrubbing** (carried from S2, re-confirmed by
   item 6): `extract_claude_common_config` still lets OPENROUTER/GOOGLE/OPENAI/
   GEMINI/AWS_* and top-level `apiKey`/`api_key` survive into the shared snippet.
   Item 8's Claude autosync half was deliberately NOT enabled for exactly this
   reason — schedule the scrub before extending switch-time autosync to Claude.
2. **Unified Codex session toggle** (item 7 trigger) — remains unported with the
   Codex Chat bridge cluster (S2 blockers 99e11e08/a078b4b2/51d6c458/f15184ed).
3. Pre-existing failure not chased (per brief):
   `tests/provider_commands.rs::switch_provider_updates_codex_live_and_state` fails
   on clean HEAD too (verified twice in S1/S2).

## Non-port code written (flagged per brief)

- **None in the resume run beyond test-assertion strengthening**: item 11's edit is
  confined to two existing `#[cfg(test)]` functions in `live.rs` (assertions +
  comments only); item 12 is upstream-verbatim plus the zh-TW deletion. Nothing new
  writes `~/.codex` or `~/.claude`.
- First run's flagged non-port code (from the log): bootstrap.rs log/comment
  relocation into the shared tauri-free bootstrap (items 2–3), a generic contract
  comment (item 7), and the `onSettled` React Query invalidation change (item 10,
  required so error responses still refresh imported rows).

## Gate results (resume run, full light gate per brief)

| Gate | Result |
|---|---|
| `cargo fmt --check` | PASS — after `cargo fmt` repaired drift from the first run in 2 files (`services/provider/mod.rs` assertion wrap, `tests/import_export_sync.rs` stray blank line); fmt-only diffs staged |
| `cargo clippy --all-targets -- -D warnings` (desktop) | PASS |
| web `cargo clippy --no-default-features --features web-server --example server -- -D warnings -A dead_code -A clippy::duplicate_mod` | PASS |
| web `cargo check --no-default-features --features web-server --example server` | PASS |
| `cargo test --lib` (UNFILTERED) | **1599 passed / 0 failed / 2 ignored** |
| `npx tsc --noEmit` | PASS |
| `npm run format:check` | PASS |
| `node scripts/check-web-route-coverage.mjs` | commands 267 / missing 0 / methodMismatch 0 / parityFallback 0 |
| locale parity check | en/ja/zh in parity |
| `npx vitest run` (full FE suite) | **124 files / 669 tests PASS** (+2 files/+14 tests = the two new OpenCode suites) |
| integration suite | not run (final S8 gate, per brief) |

## Files changed (20 staged, excluding task records)

New: `tests/components/OpenCodeFormFields.test.tsx`,
`tests/hooks/useOpencodeFormState.test.tsx`

Rust: `src-tauri/src/bootstrap.rs`, `src-tauri/src/codex_config.rs`,
`src-tauri/src/mcp/codex.rs`, `src-tauri/src/services/mcp.rs`,
`src-tauri/src/services/provider/{live,mod}.rs`,
`src-tauri/tests/{import_export_sync,mcp_commands,provider_service}.rs`

Frontend: `src/components/providers/AddProviderDialog.tsx`,
`src/components/providers/forms/{OpenCodeFormFields,ProviderForm}.tsx`,
`src/components/providers/forms/helpers/opencodeFormUtils.ts`,
`src/components/providers/forms/hooks/useOpencodeFormState.ts`,
`src/hooks/useMcp.ts`, `src/i18n/locales/{en,ja,zh}.json`

## End state

- `git ls-files -u` empty; no `CHERRY_PICK_HEAD`/`MERGE_HEAD`/sequencer state
- Zero conflict markers in staged content; `git diff --check` clean
- All work staged, **no commit made** (team lead commits)
