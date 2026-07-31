# S4 report — Project Profiles + health-check refactor (upstream v3.17.0)

Branch: `sync/upstream-v3.18.0`. This batch ports the v3.17.0 Project Profiles
feature into the web-first fork and adapts the trailing health-check refactor. The
implementation remains in the working tree for the team lead to commit; this agent
did **not** commit or push.

Checkpoint log: `S4-progress.md`. The log is authoritative for implementation and
gate results.

## Porting strategy: adapted final state, not sequential cherry-picks

The 13 profile commits share heavy intermediate churn. Reconciliation established
that the profile feature files at upstream `v3.18.0` are byte-identical to their
post-`44279987` state; only `tray.rs` and `App.tsx` receive later, unrelated S5+
changes. S4 therefore reconstructed the **adapted final profile state directly**
instead of leaving a chain of intermediate `git cherry-pick -n` merges. Per-commit
attribution below comes from the authoritative S4 order and the inspected upstream
diffs.

This avoided repeatedly introducing and then deleting the same global-current,
manual-resnapshot, and Claude Desktop forms. The two high-conflict surfaces were
handled as targeted ports:

- `4f45601f` / `3ec83578` on the fork's S2-expanded proxy/takeover service;
- `06039540` on the fork's still-active streaming check, first-run confirmation,
  Web adapter, SSRF guard, and official-OAuth skip behavior.

No cherry-pick, merge, revert, or sequencer state remains.

## Per-commit outcome — authoritative S4 order

| # | Upstream | Status | Port/adaptation rationale |
|---|---|---|---|
| 1 | `8f018a2d` | **ported (adapted)** | Added the Project Profiles foundation: additive `profiles` table and DAO, snapshot/apply service, six desktop commands, desktop tray UI, shared frontend switch/manage UI, API/query layer, locales, and roundtrip coverage. The fork uses schema **v14** rather than upstream v12, limits payload slots to Claude/Codex, routes frontend invokes through the Web adapter, adds canonical SQL-restore support, and supplies full Axum parity. |
| 2 | `6179c188` | **ported (adapted)** | The switcher is rendered only on profile-supported application tabs and is placed in the header beside the route controls without shifting the remaining toolbar. In this fork the supported mapping is exactly Claude and Codex; Gemini/OpenCode/OpenClaw/Hermes render no switcher. |
| 3 | `65a5464f` | **Claude Desktop substance excluded; structural intent adapted** | The fork has no `AppType::ClaudeDesktop`, live-file writer, tab, preset file, or locale surface for Claude Desktop. Its provider payload slot, cache invalidation, tray entry, locale copy, and roundtrip assertions were not added. Generic iteration and optional-slot semantics needed by the later scope redesign were retained in the final two-scope implementation. |
| 4 | `dbb5999d` | **ported (adapted)** | Projects are shared entities with independent per-scope current pointers and payload slots. The `profiles` table has no scope column; `current_profile_id_<scope>` lives in settings. `Option<Vec<_>>` preserves the critical distinction between “never captured” (`null`, leave untouched) and “captured empty” (`[]`, clear enabled state). Scope-local merge/apply behavior, nested tray menus, and the frontend `scope.ts` mirror were ported for Claude/Codex. |
| 5 | `4cf6f175` | **ported** | Applying a different project first auto-snapshots the project being left, only for the active scope. Autosave failures become warnings and do not block the target apply. Bidirectional autosave behavior is covered by the roundtrip suite. |
| 6 | `4f45601f` | **ported (S2-aware adaptation)** | Profile apply always attempts to disable takeover for each application before writing the target provider/configuration. The new synchronous bridge is layered on the fork's existing backup → SSOT → placeholder-cleanup recovery and managed atomic writers; it clears the per-app takeover flag and health state without regressing S2's Codex OAuth/takeover behavior. Failures are reported as best-effort warnings. |
| 7 | `f05ed3db` | **ported (dual-runtime adaptation)** | A `profile-applied` listener invalidates profile, provider, prompt, MCP, skills, proxy-takeover, and proxy-status caches. It uses the fork's runtime-neutral event adapter so desktop tray events and Web SSE events converge on the same refresh behavior. |
| 8 | `3ec83578` | **ported (dual-runtime adaptation)** | `ProfileService::apply` reports whether any takeover remains. Desktop command/tray paths and the Web handler stop the proxy asynchronously when the last takeover is gone, then emit refresh events. The synchronous DB check is shared; the Web path performs the stop directly before emitting SSE events. |
| 9 | `754af2cc` | **Claude Desktop scope excluded; scope-generalization ported** | The independent-scope machinery, `ProfileScope::ALL`, per-scope current keys, payload-slot lookup, and frontend scope mirror were retained, but the final fork scope set is only `[Claude, Codex]`. No `claude-desktop` command parameter, DTO field, payload slot, tray submenu, UI mapping, or test path exists. |
| 10 | `22159430` | **ported as an adapted serialization contract** | Backend DTOs use `camelCase` response serialization and the frontend consumes `currentIds` through a typed `CurrentProfileIds` map. The upstream multiword `claudeDesktop` field is absent because that scope is excluded; the surviving `claude`/`codex` keys are case-invariant. Web adapter tests pin the final JSON shape. |
| 11 | `9f7642e2` | **ported** | Removed the manual “update from current” control and confirmation from project management; switching is now the sole snapshot-maintenance path. The manage dialog retains rename/delete and gains an explicit Close button. The backend `resnapshot` capability remains internal for pre-switch autosave, warning propagation is preserved, and the uncaptured-scope message explains the next-switch autosave. |
| 12 | `afabe801` | **excluded as not applicable** | This commit only gates a Claude Desktop assertion by platform. Because S4 removes the Claude Desktop scope and its test altogether, there is no platform-specific assertion to gate. The retained six-test Claude/Codex integration suite is platform-neutral and uses an isolated temporary HOME. |
| 13 | `44279987` | **ported (settings-store/Web adaptation)** | Added `showProfileSwitcher`, defaulting to `true`, across Rust settings, TypeScript settings/schema/form state, the Homepage Display toggle, and `App.tsx`. The existing generic settings Web API persists it without a new special route. Legacy settings files keep the switcher visible. |
| 14 | `06039540` | **ported (adapted)** | Removed per-provider `meta.testConfig` from Rust/TS types, service merge logic, provider editor state/UI, and locales. Renamed `model-test.ts` → `connectivity-check.ts`, `ModelTestConfigPanel` → `ConnectivityCheckConfigPanel`, and the Settings section/tests. Unlike upstream's reachability-only final context, the fork deliberately retains its global streaming-check model names and prompt, active `streamCheckConfirmed` first-run flow, guarded Web dial path, and S2 official-OAuth batch skip. No command name changed, so existing stream-check Web routes remain valid. |

All 14 commits are therefore accounted for: 10 ported/adapted profile changes,
2 Claude Desktop commits whose executable substance is excluded while their generic
scope machinery is represented, 1 platform-only desktop test commit that is fully
moot, and the adapted health-check refactor.

## Conflict resolution and fork behavior re-applied

### Database and migration

- Upstream allocated profiles to v12, but this fork already used v12 for
  `pricing_missing` and v13 for `input_token_semantics`. Profiles therefore use an
  additive, idempotent **v13 → v14** migration.
- Fresh-schema creation and migration use the same `profiles` shape. A development
  marker repair migrates the earlier global `current_profile_id` to
  `current_profile_id_claude` idempotently.
- `profiles` was added to the fork's constrained canonical SQL restore allowlist;
  imported DDL is still never promoted directly to the live database.
- No destructive migration is introduced in S4.

### Atomic/managed live configuration writes

Profile apply inherently changes real external-application configuration at runtime:
Claude under `~/.claude` and Codex under `~/.codex`. It does so **only through the
existing service primitives and managed atomic writers**:

- provider: `ProviderService::switch`;
- MCP: `McpService::toggle_app`;
- skills: `SkillService::toggle_app`;
- prompt: `PromptService::enable_prompt`;
- takeover restoration: the existing backup/SSOT/placeholder-cleanup path used by
  `ProxyService`.

S4 adds no direct `std::fs::write`-style profile writer and no non-atomic replacement
path. Managed symlinks, private temporary-file permissions, rollback behavior, and
Codex OAuth-preservation policy remain owned by the existing audited writers. Tests
use the repository's temp-HOME support and never target the user's real config files.

### DB-authoritative MCP projection

Profile payloads select the desired MCP IDs, but they do not write MCP live files
from snapshot text. Apply computes the minimal enable/disable diff and delegates to
`McpService::toggle_app`; the MCP database remains authoritative and the audited
DB→live complete projection, empty-set orphan removal, fail-closed TOML handling,
and compensation behavior from S3 remain intact.

### S2 proxy/takeover behavior

- Takeover is disabled before provider apply through the fork's S2-aware restore
  machinery and managed atomic writers.
- `ProviderService::switch` remains the provider SSOT, including the fork's current
  official-provider restrictions and Codex auth-preservation gate.
- `sync_codex_live_from_provider_while_proxy_active` and the surrounding S2 behavior
  are not replaced by upstream's older proxy implementation.
- The deferred official-routing cluster `51d6c458` / `f15184ed` is **not** silently
  introduced. Profiles do not advertise official ChatGPT routing that the fork's
  backend still cannot provide.

### Web security posture

The six profile routes are mounted under the existing Web API router. They introduce
no Basic Auth, token, cookie session, login flow, or replacement authorization layer;
the Web API remains unauthenticated by design. State-changing profile requests still
inherit the existing same-origin intent middleware. No updater/release supply-chain
surface was touched.

### Health-check fork behavior retained

`06039540` was resolved against the fork rather than copied wholesale:

- `stream_check_confirmed` / `streamCheckConfirmed` and its first-run flow remain;
- global streaming models and the global test prompt remain configurable;
- the Web adapter remains the invoke boundary (no direct Tauri-only API import);
- the existing SSRF/guarded outbound path remains;
- `commands/stream_check.rs` still skips batch probes for `category == "official"`,
  preserving S2's official-OAuth protection.

Only per-provider test overrides were removed. The result is a global connectivity/
stream-check configuration, not upstream's narrower reachability-only implementation.

## Claude Desktop exclusion and final scope

The final scope contract is intentionally:

```text
ProfileScope::ALL = [Claude, Codex]
APP_PROFILE_SCOPE = { claude: "claude", codex: "codex" }
```

The backend `PerApp<T>` and frontend mirror contain only `claude` and `codex` slots.
Gemini, OpenCode, OpenClaw, and Hermes continue to work as ordinary fork applications,
but do not display a profile switcher. Claude Desktop code, payload data, tests,
locales, tray entries, and `claudeDesktopProviderPresets.ts` were not resurrected.
`zh-TW.json` also remains deleted; profile and connectivity strings were added only to
the fork's retained en/ja/zh locales.

## Mandatory Web parity additions

Upstream supplies only Tauri profile commands. The fork adds exact Axum/Web parity:

| Command | Web method/path |
|---|---|
| `list_profiles` | `GET /api/profiles/list-profiles` |
| `create_profile` | `POST /api/profiles/create-profile` |
| `update_profile` | `PUT /api/profiles/update-profile` |
| `delete_profile` | `DELETE /api/profiles/delete-profile?id=...` |
| `clear_current_profile` | `DELETE /api/profiles/clear-current-profile?scope=...` |
| `apply_profile` | `POST /api/profiles/apply-profile` |

Parity work includes:

- new `src-tauri/src/web_api/handlers/profiles.rs` plus handler-module and router
  wiring;
- six SSOT entries in `src/lib/api/web-commands.ts`;
- profile ownership in `gen-command-manifest.rs`;
- the `services/profile.rs` `#[path]` shim and `SkillService` re-export in
  `examples/web_services.rs`;
- adapter-based `profilesApi` and Web-safe query mutations (desktop tray refresh is
  skipped in Web mode);
- blocking profile apply moved to `tokio::task::spawn_blocking`, because the shared
  provider switch path uses a blocking bridge internally;
- matching Web SSE `provider-switched` and `profile-applied` events, proxy shutdown,
  and frontend cache invalidation.

`database::dao::profiles` is already reached through the Web example's complete
`database` module, while `commands/profile.rs` is desktop-only; neither requires an
extra `web_services.rs` path entry. Route coverage rose from 267 to 273 commands and
remains exact with zero missing or mismatched routes.

## Non-port code (complete inventory)

The following code is fork-authored or materially fork-adapted rather than a literal
upstream hunk:

1. **Web API parity**
   - all of `src-tauri/src/web_api/handlers/profiles.rs`;
   - its `handlers/mod.rs` and `routes.rs` wiring;
   - the six `web-commands.ts` registrations;
   - profile owner mapping in `gen-command-manifest.rs`;
   - the Web service shim and `SkillService` re-export.
2. **Web/runtime-neutral frontend integration**
   - `src/lib/api/profiles.ts` uses `./adapter` instead of
     `@tauri-apps/api/core`;
   - profile query mutations avoid the unsupported desktop tray command in Web mode;
   - `App.tsx` subscribes through the runtime-neutral event adapter and invalidates
     takeover/proxy caches for both Web SSE and desktop tray applies;
   - because prompts use local hook state rather than React Query, the same listener
     reloads an open `PromptPanel` directly when the applied scope matches the active
     application.
3. **Claude/Codex-only scope adaptation**
   - removal of the backend/frontend Claude Desktop enum, payload slot, current ID,
     app mapping, tray/UI branches, locale text, and tests;
   - explicit unsupported-tab behavior for Gemini/OpenCode/OpenClaw/Hermes;
   - fork-specific scope/null-vs-empty regression coverage.
4. **Fork database safety adaptation**
   - v14 allocation and v13→v14 migration instead of upstream v11→v12;
   - the canonical SQL restore allowlist entry;
   - the idempotent development global-marker repair regression.
5. **S2 takeover adaptation**
   - the synchronous profile bridge is reconciled with the fork's enhanced restore,
     managed-write, placeholder-cleanup, OAuth-preservation, and health cleanup paths;
   - cross-runtime “last takeover” handling and the Web proxy-stop/event path.
6. **Settings and `06039540` fork adaptation**
   - Zod/useSettingsForm/Web persistence coverage for `showProfileSwitcher`;
   - preservation of global streaming model/prompt fields and
     `streamCheckConfirmed`;
   - connectivity API import through the Web adapter and retained structured error
     extraction;
   - Web-server integration test rename/coverage rather than a Tauri-only panel.
7. **Fork-authored/adapted tests**
   - Web method/body/query encoding for all profile API forms;
   - Claude/Codex current-ID and unsupported-tab switcher tests;
   - App `profile-applied` takeover/proxy cache invalidation;
   - settings toggle/default persistence;
   - connectivity-panel load/save/default/error coverage;
   - Claude/Codex-only temp-HOME profile roundtrip adaptation.

No non-port code adds a new direct writer for real `~/.claude`, `~/.codex`, or
`~/.config` files.

## Tests added/adapted

Coverage includes:

- six profile integration tests: snapshot/apply roundtrip, shared-scope isolation,
  dangling-reference best effort, scoped clear, switch autosave, and takeover disable;
- profile payload serde, missing-field compatibility, scope merge/capture, app mapping,
  parse/serde, and minimal-toggle unit tests;
- profile DAO CRUD/current-pointer tests and v13→v14/dev-marker migration tests;
- legacy-settings default for `showProfileSwitcher`;
- Web adapter method/body/query encoding;
- ProfileSwitcher current-ID/scope behavior and unsupported fork tabs;
- null-vs-empty scope semantics;
- settings visibility toggle persistence;
- runtime `profile-applied` cache invalidation;
- runtime `profile-applied` reload of an open prompt panel for the matching scope;
- connectivity panel rename plus load/save/default/error behavior and real Web-server
  settings persistence.

Before the full gate, focused checks passed as recorded in the checkpoint log:

- Rust: **6 integration + 17 filtered lib tests**;
- frontend: **7 files / 34 tests**.

## Exact S4 light-gate matrix

Cargo commands were run from `src-tauri/`; frontend commands were run from the
repository root.

| Command | Recorded result |
|---|---|
| `cargo fmt --check` | **PASS** — no output |
| `cargo clippy --all-targets -- -D warnings` | **PASS** — final cached verification completed in **0.69s** after the full compile pass |
| `cargo clippy --no-default-features --features web-server --example server -- -D warnings -A dead_code -A clippy::duplicate_mod` | **PASS** — finished dev profile in **15.09s** |
| `cargo check --no-default-features --features web-server --example server` | **PASS** — 64 expected dead-code warnings from the standalone shim; finished dev profile in **6.02s** |
| `cargo test --lib` | **PASS** — **1614 passed / 0 failed / 2 ignored / 0 filtered**; finished in **9.81s** |
| `cargo test --test profile_roundtrip` | **PASS** — **6 passed / 0 failed / 0 ignored**; finished in **2.99s** |
| `npx tsc --noEmit` | **PASS** — no output |
| `npm run format:check` | **PASS** — all matched `src/**/*.{js,jsx,ts,tsx,css,json}` files use Prettier style |
| `node scripts/check-web-route-coverage.mjs` | **PASS** — commands **273**, routes **261**, wildcardRoutes **20**, unsupported **29**, webReplacements **7**, webFetchLiteralPaths **4**, missing **0**, methodMismatch **0**, danglingReplacementPaths **0**, parityExact **5**, parityFallback **0** |
| `npm run check:locales` | **PASS** — en/ja/zh each **2395** keys, totalUniqueKeys **2395**, `inParity: true` |
| `npx vitest run` | **PASS** — **127 test files / 678 tests**, duration **61.29s**; only expected negative-path stderr and existing MSW/browser-data/React/CodeMirror warnings |

The integration suite was intentionally **not run** in S4; it remains part of the
final S8 gate per the brief.

## Deferrals and follow-ups

1. **Extend profiles to fork-only applications** — Gemini, OpenCode, OpenClaw, and
   Hermes are intentionally not invented as new scopes in this upstream-sync batch.
   A follow-up should define snapshot dimensions and live-write semantics per app
   before extending both backend `ProfileScope` and frontend `APP_PROFILE_SCOPE`.
2. **Claude Desktop remains excluded** — only revisit this if the fork first adds a
   real Claude Desktop application/runtime surface.
3. **S2 official-routing takeover cluster remains deferred** — `51d6c458` and
   `f15184ed` still need their own reviewed cross-runtime batch. S4 neither unblocks
   nor advertises that capability.
4. **Other pre-existing S2/S3 blockers remain unchanged** — the Codex Chat/
   Anthropic bridge and unified-session trigger are still absent; S4 does not pull
   them into profile switching.
5. **Release metadata/changelog and full integration/E2E** remain S8 work. The
   upstream `06039540` changelog line was not copied because this fork writes its own
   release notes.

## Phase 2.2 independent review addendum

The independent backend/Web check found one concurrency defect after the original
light gate: `disable_takeover_for_app_sync` restored live configuration and cleared
backup/flags without taking the per-application switch lock used by takeover and hot
switch operations. A concurrent takeover could therefore interleave with profile
apply and overwrite or consume the wrong live/backup state.

The synchronous profile path now acquires the existing per-app switch lock across
restore → backup deletion → proxy flag/health cleanup, while calling the non-locking
`restore_live_config_for_app_with_fallback_inner` helper to avoid recursive locking.
The new `profile_takeover_disable_serializes_on_switch_lock` regression holds the
same lock, proves profile disable waits, then verifies the original live config is
restored, the consumed backup is deleted, and the takeover flag is cleared.

Post-fix checks passed: the new regression (1 test), profile integration suite
(6 tests), filtered profile/migration library set (18 tests), Web route coverage
(273 commands; zero missing/method mismatch), `cargo fmt --check`, and
`git diff --check`.

The frontend/health review then found a cross-layer refresh defect: both the profile
mutation and `App.tsx` invalidated `['prompts', scope]`, but `PromptPanel` does not use
React Query and therefore ignored that invalidation. An open prompt view could remain
stale after another Web client or the desktop tray applied a profile. The panel handle
now exposes `reload`; the runtime-neutral `profile-applied` listener calls it only for
the matching active scope, and the fake prompt-query invalidations were removed. An
App integration regression covers both matching and non-matching scopes. Retained
connectivity-check locale text was also updated so errors and model-retirement hints no
longer point users to the removed “Model Test Config” surface.

The final full affected gate is green with the matrix above.

## Final hygiene and state

The final post-review hygiene pass found:

- `git diff --check` clean;
- zero conflict markers and `git ls-files -u` empty;
- no `CHERRY_PICK_HEAD`, `MERGE_HEAD`, `REVERT_HEAD`, or sequencer state;
- no staged files;
- no changed `zh-TW` or Claude Desktop scope path and no executable Claude Desktop
  profile scope;
- no direct Tauri API/global use in the profile frontend; `profilesApi` imports
  `invoke` only from `./adapter`.

The working tree intentionally remains dirty with S4 implementation and task records.
Unrelated untracked `.pi/` and `.pi-subagents/` artifacts are outside this batch and
were ignored. **No commit, push, reset, merge, or discard operation was performed.**
