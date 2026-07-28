# Task: Port upstream batch S1 (usage/pricing core correctness) into cc-switch-web

You are working in /home/orion/Workspace/github/cc-switch-web on branch `sync/upstream-v3.18.0`.
This is a web-first fork of `farion1231/cc-switch` (git remote `product-upstream`, tags v3.17.0/v3.18.0 fetched locally).

## Current repo state
A cherry-pick of upstream `f991726f` ("fix(usage): account for cache-write tokens across schema versions") is mid-conflict.
Delete/modify conflicts were already resolved and staged by a predecessor (this fork restructured
transform_codex_anthropic.rs / transform_codex_chat.rs / sql_helpers.rs). Six files remain UNMERGED with conflict markers:

- src-tauri/src/database/dao/usage_rollup.rs
- src-tauri/src/proxy/providers/streaming.rs
- src-tauri/src/proxy/providers/transform.rs
- src-tauri/src/proxy/providers/transform_responses.rs
- src-tauri/src/proxy/usage/logger.rs
- src-tauri/src/services/usage_stats.rs

Useful: `git status`, conflict hunks in the files, `git show f991726f -- <file>`, `git log -p -5 <file>` for our side's history.
Batch context: `.trellis/tasks/07-25-sync-upstream-v3-18-0/prd.md` and
`.trellis/tasks/07-25-sync-upstream-v3-18-0/research/upstream-commit-inventory.md`.

## Steps

1. Resolve the 6 conflicts. Binding policy (user-decided): **upstream-first** — where both sides changed the same logic,
   take upstream's implementation, then re-apply on top the local-only fixes upstream lacks:
   (a) `pricing_missing` no-silent-$0 semantics (usage_rollup / schema — pricing misses must stay marked, never silently cost $0),
   (b) client-IANA-timezone usage bucketing in usage_stats.rs (usage APIs accept a client tz for day bucketing),
   (c) web/desktop feature-gating so BOTH the desktop Tauri build and the web build (examples/server.rs) compile.
   Then `git add` the resolved files and run `git cherry-pick --quit` (keeps index/worktree, clears pick state).

2. Port the remaining S1 upstream commits IN ORDER via `git cherry-pick -n <hash>` (stages without committing),
   resolving conflicts under the same policy:
   13e7c1fc  (Anthropic cache write TTLs)
   b9263a80  (strengthen prompt cache breakpoint injection)
   0e563b50  (surface unsupported breakpoint counts)
   6eb217b2  (REVERT of the 1-hour cache TTL option — final state must be the net post-revert code)
   f39d463c  (Codex subagent usage counted)
   98ccde00  (persist dashboard refresh interval)
   2df2212c  (reject transient transport failures; note fork already has tests/lib/keepLastGoodUsage.test.ts — reconcile upstream-first)
   31ee4285  (gpt-5.6 alias pricing rows, 1.25x cache-write)
   a7b4dd94  (GPT-5.6 Sol/Terra/Luna pricing)
   62e44c48  (Tencent Hunyuan Hy3 pricing)
   99573d22  (pin context window values instead of form fields)
   940ddd33  (Kimi For Coding 256K context window)
   5c39dfbf  (gpt-5.6 context window for Claude Code takeover)
   Skip/strip sponsor or referral copy if it rides along in any diff.

3. CHECKPOINT after each hash: append a line `<hash> done|adapted|excluded — <one-line note>` to
   `.trellis/tasks/07-25-sync-upstream-v3-18-0/research/S1-progress.md` (create the file first, recording f991726f).

4. Light verification gate (fix what fails):
   - `source "$HOME/.cargo/env"` (cargo is at ~/.cargo/bin, not on default PATH)
   - in src-tauri/: `cargo fmt --check`, `cargo clippy --workspace --all-targets`
   - web-side Rust check: see package.json scripts / .github/workflows/ci.yml for the exact web cargo check command and run it
   - targeted `cargo test` for the usage/pricing/proxy-cache modules you touched
   - repo root: `pnpm typecheck` (or the tsc script in package.json), `pnpm format:check`,
     targeted `pnpm vitest run <files>` for FE files you touched
   - Do NOT run the full integration suite (that is a later, final gate).

5. Write `.trellis/tasks/07-25-sync-upstream-v3-18-0/research/S1-report.md`:
   per-commit status (ported/adapted-how/excluded-why), conflict resolutions, local fixes re-applied, gate results.

## Hard constraints
- NEVER run `git commit` or `git push`. Leave all changes staged/working-tree only.
- End state: zero conflict markers (`grep -rn '<<<<<<<' src-tauri/src src/` empty), zero unmerged index entries (`git ls-files -u` empty).
- No authentication code (fork is unauthenticated by design, ADR-0001). Desktop updater stays disabled.
- If any NEW Tauri command is introduced, wire its web route: src/lib/api/web-commands.ts is the route SSOT;
  keep `node scripts/check-web-route-coverage.mjs` green.
- Keep changes upstream-shaped and minimal to ease future syncs.
