# Batch S4 brief — Project Profiles (upstream v3.17.0)

Repo: /home/orion/Workspace/github/cc-switch-web, branch `sync/upstream-v3.18.0`.
Web-first fork of `farion1231/cc-switch` (remote `product-upstream`, tags fetched).
S1 (`e6390144`), S2 (`92b55fa9`), S3 (`e946a6fc`) are committed. **Read `S2-report.md` and
`S3-report.md` first** — S2's takeover/proxy work and deferred clusters directly collide with
this batch; S3 set the conflict-resolution precedent. S1-report optional.

This batch ports a NEW feature (snapshot-based project profiles) that does not exist in the
fork, plus the health-check removal refactor. ~4k upstream lines.

## Port these upstream commits, IN THIS ORDER

Topological order from `upstream-commit-inventory.md` (11 share one commit timestamp — a
rebase — so `%ci` sorting is useless; this list is authoritative; PRD's `8f018a2c` was a typo
for `8f018a2d`):

1. `8f018a2d` feat: add project profiles for snapshot-based config switching — foundation:
   DB schema + DAO, `services/profile.rs`, `commands/profile.rs`, tray submenu, FE
   `components/profiles/*`, `lib/api/profiles.ts`, `lib/query/profiles.ts`,
   `tests/profile_roundtrip.rs`
2. `6179c188` fix(profiles): scope switcher to supported app tabs and relocate it
3. `65a5464f` feat(profiles): include Claude Desktop provider in project profiles —
   **EXCLUDE the Claude-Desktop substance** (see scope decision); take only structural hunks
   later commits build on, adapted
4. `dbb5999d` refactor(profiles): shared project entity with per-scope switching
5. `4cf6f175` feat(profiles): autosave previous profile state on switch
6. `4f45601f` feat(profiles): unconditionally disable proxy takeover before applying profile
7. `f05ed3db` fix(ui): invalidate proxy takeover status after profile switch
8. `3ec83578` fix(profiles): stop proxy server when profile switch leaves no takeovers active
9. `754af2cc` feat(profiles): split Claude Desktop into independent profile scope — scope
   itself EXCLUDED; port the scope-generalization machinery (e.g. `scope.ts` mirror,
   per-scope slot keys) that items 10–11 assume
10. `22159430` fix(profiles): use camelCase keys for current profile ids in frontend
11. `9f7642e2` refactor(profiles): drop manual snapshot update now that switching autosaves
12. `afabe801` test(profiles): gate desktop-scope assertion by platform — mostly moot once
    the desktop scope is gone; adopt whatever residue still applies
13. `44279987` feat(profiles): add setting to toggle project switcher on main page
14. `06039540` refactor(health-check): remove per-provider test config

Read each with `git show <hash>` first. `git cherry-pick -n <hash>` (stages, never commits)
where clean; manual adaptation where the fork diverged.

## Binding scope decision — Claude Desktop

The fork has NO Claude Desktop: `AppType` = {Claude, Codex, Gemini, OpenCode, OpenClaw,
Hermes} (`app_config.rs:321`), zero `ClaudeDesktop`/`claude-desktop` references repo-wide,
`claudeDesktopProviderPresets.ts` deleted, zh-TW locale deleted. Therefore:

- Fork `ProfileScope::ALL = [Claude, Codex]` (upstream final state is
  `[Claude, ClaudeDesktop, Codex]`). Drop the `claude_desktop` payload slot, `for_app` arm,
  and `scope.ts` mirror entry. Adapt `profile_roundtrip.rs` accordingly.
- Do NOT invent new scopes for Gemini/OpenCode/OpenClaw/Hermes in this batch. Upstream's own
  pattern: apps missing from `APP_PROFILE_SCOPE` simply don't render the switcher. Record
  "extend profiles to fork-only apps" as a follow-up in the report.
- zh-TW.json modify/delete conflicts → `git rm` (stays deleted; S3 item-12 precedent).

## Web parity work (fork-only, REQUIRED — upstream has none of this)

This is non-port code by definition; flag it as such in the report, but it is mandatory:

- Every Tauri command in `commands/profile.rs` needs a web handler (new
  `src-tauri/src/web_api/handlers/profiles.rs`, wired in `handlers/mod.rs` + router) and a
  route entry in `src/lib/api/web-commands.ts` (the SSOT).
  `node scripts/check-web-route-coverage.mjs` must end green (currently 267/0).
- New Rust modules (`services/profile.rs`, `commands/profile.rs`, `database/dao/profiles.rs`)
  need `#[path]` shims in `src-tauri/examples/web_services.rs` — only the web clippy gate
  catches a missing one (bit S1).
- `tray.rs` additions are desktop-only; must not leak into the web build nor break desktop
  clippy. FE profile UI must work over the web adapter (no tauri-only API calls).
- `06039540` renames `src/lib/api/model-test.ts` → `connectivity-check.ts` and
  `StreamCheckConfigPanel` → `ConnectivityCheckConfigPanel`; if any command name changes,
  update `web-commands.ts` + coverage accordingly.

## Fork-local behavior in this area — preserve on top of upstream

- **Profile apply writes live configs → fork's atomic/managed writers only** (ADR-0003/0004).
  Snapshot restore must not introduce direct non-atomic writes, must not bypass the strip
  machinery, and MCP re-projection stays DB-authoritative — never live-config-derived.
- Gemini config protections (non-strict-JSON `settings.json` never replaced with `{}`;
  `.env` line-level edits) are out of profile scope by the decision above — but if any
  shared helper you touch feeds gemini paths, keep the protections intact.
- Items 6–8 land on `services/proxy.rs` that S2 grew by ~311 lines (takeover hardening,
  `sync_codex_live_from_provider_while_proxy_active`, atomic live writers). Upstream-first,
  then re-apply S2 behavior. The **official-routing takeover cluster (`51d6c458`/`f15184ed`)
  is still unported** — any profile-switch hunk referencing official-routing/built-in
  provider state must be adapted to the fork's takeover model; record each in the report.
- `06039540`: fork's `stream_check` kept the portable half of `f2c6d48e` (skip reachability
  probes for official OAuth entries) — keep that behavior. S2 also touched
  `commands/stream_check.rs`; reconcile, don't revert.
- DB schema additions follow the fork's existing migration pattern: additive tables,
  idempotent, on failure keep the original DB usable (recovery precedent from v3.16.5 sync).
  No destructive migration here — that's S6.
- Settings persistence (`44279987`) goes through the fork's settings store
  (`src-tauri/src/settings.rs` + `web_api/handlers/settings.rs` parity).

Excluded regardless: sponsor/referral/marketing copy, desktop updater / release packaging,
anything reintroducing authentication (ADR-0001), Codex Chat bridge area
(`*_codex_chat*`/`*_codex_anthropic*` hunks → skip, record).

## Checkpoint discipline (mandatory — the gateway kills agents mid-run)

Create `S4-progress.md` in this directory as your FIRST action, then append a line after
EVERY unit of work: `<hash-or-file> DONE|ADAPTED|EXCLUDED — <note>`. If resumed, re-read it
plus `git status` to find your place.

## Light gate (run at the end; fix what fails)

`source "$HOME/.cargo/env"` first — cargo is not on the default PATH.
- in `src-tauri/`: `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`
- web build: `cargo clippy --no-default-features --features web-server --example server -- -D warnings -A dead_code -A clippy::duplicate_mod`
  and `cargo check --no-default-features --features web-server --example server`
- `cargo test --lib` UNFILTERED, plus focused `cargo test --test profile_roundtrip`
- repo root: `npx tsc --noEmit`; `npm run format:check`; `node scripts/check-web-route-coverage.mjs`;
  `npm run check:locales`; `npx vitest run`
- Known pre-existing failure, do not chase: `provider_commands::switch_provider_updates_codex_live_and_state`
  (verified on clean HEAD). Cross-test `$HOME` interference in default parallel runs is also
  pre-existing.
- Do NOT run the integration suite (`test:integration`) — that is the final S8 gate.

## Hard rules

- NO `git commit`, NO `git push`. Leave everything staged/working-tree; the team lead commits.
- End state: zero conflict markers, `git ls-files -u` empty, no cherry-pick/merge state left.
- Tests must never touch the real `~/.claude`/`~/.codex`/`~/.config` — use the fork's
  temp-HOME test helpers. Flag prominently any runtime code path that writes real user
  configs (profile apply inherently does — via the atomic writers only).
- Anything you write that is NOT a port of an upstream hunk (web handlers, shims, scope
  adaptation) → list it explicitly in the report's "non-port code" section.

## Deliverable

Write `S4-report.md` here: per-commit status (ported / adapted-how / excluded-why), conflicts
and resolutions, fork behavior re-applied, the Claude-Desktop scope adaptation summary, web
parity additions, deferrals/follow-ups, gate output summary.
