# Batch S3 brief — MCP / config-sync hardening (upstream v3.17.0)

Repo: /home/orion/Workspace/github/cc-switch-web, branch `sync/upstream-v3.18.0`.
Web-first fork of `farion1231/cc-switch` (remote `product-upstream`, tags fetched).
Batches S1 (`e6390144`) and S2 (`92b55fa9`) are already committed. **Read
`S1-report.md`, `S2-report.md` and `S2-progress.md` first** — their resolutions set the
precedent for this batch, and S2 touched the same provider/common-config code you will.

## Port these upstream commits, IN THIS ORDER (verified chronological)

1. `ffc22ea7` feat(universal-provider): auto-sync after adding; drop unused addSuccess i18n key (#2811)
2. `e191af4a` fix: OpenCode live provider import updates (#4712)
3. `e78aa8a7` fix: sync openclaw and hermes live provider updates (#5098)
4. `8b1ce764` fix(mcp): fail closed when Codex config.toml is unparseable during MCP sync
5. `93f56198` fix(codex): strip synced `[mcp_servers]` from provider snapshots on backfill
6. `473c2aaa` fix(provider): exclude injected artifacts and routing fields from Codex common-config extraction
7. `6d2ee247` fix(provider): re-project Codex MCP after unified-session toggle rewrites live config
8. `1f36f0cf` feat(provider): extend switch-time common-config autosync to Codex
9. `11c173c7` fix(mcp): stop cross-app failures from blocking MCP re-projection
10. `94fc1cc0` fix(mcp): surface per-app failures when importing MCP servers from apps
11. `88d5ffba` fix(codex): move common-config TOML merge off smol-toml to backend toml_edit
12. `6245caa6` Fix/opencode known field editors (#2907)

Use `git cherry-pick -n <hash>` (stages, never commits). Read each with `git show <hash>` first.

## Fork-local behavior in this exact area — preserve it on top of upstream

This batch lands squarely on code the fork's own audits rewrote. Expect conflicts and
re-apply these on top of upstream's version:
- **Atomic / transactional writes**: `fix(codex): make common config and MCP projection atomic`,
  `fix(config): preserve managed symlinks on atomic writes`,
  `fix(provider): reconcile custom endpoints transactionally` (see `docs/adr/0003`, `0004`).
  Upstream hunks that write config non-atomically must be adapted to the fork's atomic writers,
  not the other way round.
- **MCP projection from authoritative state** (ADR-0004) — do not regress to live-config-derived
  projection.
- `provider_common_config_strip_opt_in` / `strip_common_config_for_backfill` and, from S2,
  `strip_injected_codex_oauth_context_defaults` plus the Fable/subagent strip keys. Item 6
  (`473c2aaa`) extends exactly this strip machinery — reconcile carefully, keep all fork keys.
- Gemini config protections from the audit: non-strict-JSON `settings.json` must not be replaced
  with `{}`; MCP `timeout` must not be unconditionally overwritten; `.env` edits stay line-level
  (comments/`export` preserved).
- `pricing_missing` marking; client-tz usage bucketing; SSRF/IP-guard and log-privacy hardening.
- `src/config/claudeDesktopProviderPresets.ts` stays DELETED (fork dropped Claude Desktop scope).
- Web feature gating: desktop-only code behind `#[cfg]`; the web build has its OWN module map in
  `src-tauri/examples/web_services.rs` (+ sibling shims) — any NEW module needs a `#[path]` shim
  there, and only the web clippy gate catches a missing one (this bit S1).

Excluded regardless: sponsor/referral/marketing copy, desktop updater / release packaging,
anything reintroducing authentication (fork is unauthenticated by design, ADR-0001).

## Known blocked area (do not try to unblock)

The fork never ported upstream's Codex Chat bridge (4 modules, 5732 LOC); by user decision it
stays unported. If a hunk here targets `*_codex_chat*` or `*_codex_anthropic*`, skip that hunk,
keep the rest, and record it in the report's blocked section. Same for the official-routing
takeover cluster (`51d6c458`/`f15184ed`) deferred in S2.

## Known gap — record, do NOT fix here

S2 found `extract_claude_common_config` has no generic credential scrubbing (OPENROUTER/GOOGLE/
OPENAI/GEMINI/AWS_* and top-level `apiKey`/`api_key` survive into the shared snippet). Item 6
is the Codex-side analogue. If porting it makes the Claude-side fix trivial, say so in the
report with the concrete diff you would write — but do not land it; the team lead schedules it.

## Checkpoint discipline (mandatory — the gateway kills agents mid-run)

Create `S3-progress.md` in this directory as your FIRST action, then append a line after EVERY
unit of work: `<hash-or-file> DONE|ADAPTED|EXCLUDED — <note>`. If resumed, re-read it plus
`git status` to find your place.

## Light gate (run at the end; fix what fails)

`source "$HOME/.cargo/env"` first — cargo is not on the default PATH.
- in `src-tauri/`: `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`
- web build: `cargo clippy --no-default-features --features web-server --example server -- -D warnings -A dead_code -A clippy::duplicate_mod`
  and `cargo check --no-default-features --features web-server --example server`
- `cargo test --lib` (run it UNFILTERED — a filtered run hid a broken assertion in S2)
- repo root: `npx tsc --noEmit`; `npm run format:check`; `node scripts/check-web-route-coverage.mjs`;
  the locales parity check; `npx vitest run`
- Known pre-existing failure, do not chase: `provider_commands::switch_provider_updates_codex_live_and_state`
  fails on clean HEAD too (verified twice). Extra failures in a default parallel run are cross-test
  `$HOME` interference, also pre-existing.
- Do NOT run the integration suite — that is the final S8 gate.

## Hard rules

- NO `git commit`, NO `git push`. Leave everything staged/working-tree; the team lead commits.
- End state: zero conflict markers, `git ls-files -u` empty, no cherry-pick/merge state left.
- Any NEW Tauri command needs its web route: `src/lib/api/web-commands.ts` is the SSOT and
  `check-web-route-coverage.mjs` must stay green.
- If you write code that is NOT a port of an upstream hunk, flag it prominently in your final
  summary — especially anything that writes the user's real `~/.codex` or `~/.claude` files.

## Deliverable

Write `S3-report.md` here: per-commit status (ported / adapted-how / excluded-why), conflicts and
resolutions, fork behavior re-applied, deferrals, gate output summary.
