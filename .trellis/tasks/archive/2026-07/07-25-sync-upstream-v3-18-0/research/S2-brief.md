# Batch S2 brief — Codex routing / protocol bridges (upstream v3.17.0)

Repo: /home/orion/Workspace/github/cc-switch-web, branch `sync/upstream-v3.18.0`.
Web-first fork of `farion1231/cc-switch` (remote `product-upstream`, tags fetched).
Batch S1 is already committed (e6390144) — read its report at `S1-report.md` and its
checkpoint log at `S1-progress.md` before starting; the resolutions there set precedent.

## Port these upstream commits, IN THIS ORDER (verified chronological)

1. `b3e5e32c` feat: add Claude subagent model config (#4830)
2. `3538b392` feat(claude): add 1M checkbox to fallback model field (#5124)
3. `95c917b3` feat(provider): add Zhipu team plan quota query support (#5128)
4. `99e11e08` feat(codex): support native Anthropic Messages protocol as upstream (#5071)
5. `50270d5e` fix: exclude Fable model env from Claude common config (#4272) (#5206)
6. `ded0b63a` fix: handle missing provider keys and tool schema types (#5069)
7. `c6197ae3` fix(proxy): inject a single auth placeholder on managed Claude takeover (#5095)
8. `7479d10d` feat(codex): add default model field to provider form
9. `27ce0a51` fix(proxy): harden Responses reasoning and tool-call conversion
10. `a078b4b2` feat(proxy): session-based prompt_cache_key routing for Codex Chat bridge
11. `650905af` fix(proxy): harden Responses and Anthropic protocol bridges
12. `51d6c458` feat(codex): route native ChatGPT sessions through proxy takeover
13. `f2c6d48e` fix(providers): skip reachability probes for official OAuth entries
14. `f15184ed` feat(codex): expose official routing and restore the built-in provider
15. `af58740b` fix(proxy): align Codex OAuth client identity
16. `ac52c851` fix(codex): infer image capability for generated catalogs and resync takeover live on save

Use `git cherry-pick -n <hash>` (stages, never commits). Read each with `git show <hash>` first.

## MUST-FIX carried over from S1 (do this with #5 `50270d5e`)

S1 found the fork's Codex common-config strip list is missing three keys that upstream
strips — a real cross-provider leak (upstream issue #4272):
`ANTHROPIC_DEFAULT_FABLE_MODEL`, `ANTHROPIC_DEFAULT_FABLE_MODEL_NAME`, `CLAUDE_CODE_SUBAGENT_MODEL`
(in `src-tauri/src/services/provider/mod.rs`'s strip list; S1 deliberately deferred them
because the fork never ported the earlier upstream commits that introduced them).
`50270d5e` is the upstream fix for exactly this — make sure all three keys end up stripped,
and note in the progress log that the S1 deferral is now closed. If upstream tests for this
exist in mod.rs and the fork lacks their fixtures, port what applies.

## Conflict policy (binding, same as S1)

**Upstream-first**: where both sides changed the same logic, take upstream's implementation,
then re-apply on top the local-only behavior upstream lacks. Known fork-local behavior to
preserve in this area:
- `pricing_missing` no-silent-$0 marking (fork replaced upstream's `backfill_missing_usage_costs`
  machinery — keep excluding that machinery, as S1 did)
- `provider_common_config_strip_opt_in` / `strip_common_config_for_backfill` (fork's L30 work) —
  S1 resolved an insertion collision here by KEEPING BOTH the fork's helpers and upstream's
  `strip_injected_codex_oauth_context_defaults`; follow that pattern
- transactional/atomic config + MCP projection writes (fork audit fixes)
- SSRF/IP-guard and log-privacy hardening in the proxy layer
- `src/config/claudeDesktopProviderPresets.ts` stays DELETED (fork dropped Claude Desktop scope) —
  adapt or skip upstream hunks that target it
- web feature gating: desktop-only code behind `#[cfg]`; the web build has its OWN module map in
  `src-tauri/examples/web_services.rs` (+ sibling example shims) — S1 had to add a `#[path]` shim
  there for a new module, and only the web clippy gate caught it. Check for that on every new module.

Excluded regardless: sponsor/referral/marketing copy, desktop updater or release-packaging
changes, anything reintroducing authentication (fork is unauthenticated by design, ADR-0001).

## Checkpoint discipline (mandatory — the gateway kills agents mid-run)

Create `S2-progress.md` in this directory as your FIRST action, then append a line after
EVERY unit of work: `<hash-or-file> DONE|ADAPTED|EXCLUDED — <note>`. If you are resumed,
re-read it plus `git status` to find your place. Record deferrals explicitly with enough
detail for the team lead to schedule them.

## Light gate (run at the end; fix what fails)

`source "$HOME/.cargo/env"` first — cargo is not on the default PATH.
- in `src-tauri/`: `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`
- web build: `cargo clippy --no-default-features --features web-server --example server -- -D warnings -A dead_code -A clippy::duplicate_mod`
  and `cargo check --no-default-features --features web-server --example server`
- `cargo test` (desktop) — NOTE: `provider_commands::switch_provider_updates_codex_live_and_state`
  fails on clean HEAD too (pre-existing, verified in S1). Don't chase it.
- repo root: `npx tsc --noEmit`; `npx prettier --check .` (or the package.json script);
  `node scripts/check-web-route-coverage.mjs`; the locales check script; `npx vitest run`
- Do NOT run the integration suite — that is the final S8 gate.

## Hard rules

- NO `git commit`, NO `git push`. Leave everything staged/working-tree; the team lead commits.
- End state: zero conflict markers, `git ls-files -u` empty, no cherry-pick/merge state left.
- Any NEW Tauri command needs its web route: `src/lib/api/web-commands.ts` is the SSOT and
  `check-web-route-coverage.mjs` must stay green.

## Deliverable

Write `S2-report.md` here: per-commit status (ported / adapted-how / excluded-why), conflicts
and how resolved, fork behavior re-applied, deferrals, gate output summary.
