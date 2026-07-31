# S1 handoff state — 2026-07-25 evening (infrastructure outage)

## Where the work stands

Branch `sync/upstream-v3.18.0` (task 07-25-sync-upstream-v3-18-0, PRD in this dir).
A cherry-pick of upstream `f991726f` (fix(usage): account for cache-write tokens across
schema versions) is **mid-conflict and intentionally left in place**:

- RESOLVED + staged (by first agent run): delete/modify conflicts on files our fork
  restructured — transform_codex_anthropic.rs, transform_codex_chat.rs, sql_helpers.rs,
  plus clean auto-merges staged for database/mod.rs, database/schema.rs,
  proxy/usage/calculator.rs, proxy/usage/parser.rs.
- STILL UNMERGED (conflict markers present), 6 files:
  - src-tauri/src/database/dao/usage_rollup.rs
  - src-tauri/src/proxy/providers/streaming.rs
  - src-tauri/src/proxy/providers/transform.rs
  - src-tauri/src/proxy/providers/transform_responses.rs
  - src-tauri/src/proxy/usage/logger.rs
  - src-tauri/src/services/usage_stats.rs

Resume procedure: resolve the 6 files upstream-first (re-apply local `pricing_missing`,
client-tz bucketing, web feature-gating on top), `git add`, `git cherry-pick --quit`,
then continue the S1 list via `git cherry-pick -n` per prd.md S1 table, checkpointing
each hash to S1-progress.md. A ready-made self-contained brief exists at
`/tmp/s1-codex-brief.md` (also usable verbatim for any agent).

## Why work stopped (2026-07-25 16:11–21:31)

Sub-agent inference via the local gateway (127.0.0.1:23000) was hard-down all evening:
six consecutive agent attempts failed with `429 Service Unavailable` / request timeouts
BEFORE completing a single inference request (zero durable edits):
1. trellis-implement (inherit model) — timeout ×2 (one run did land the delete/modify
   resolutions above before dying)
2. trellis-implement fresh takeover — 429
3. codex-rescue wrapper — 429 before launching codex
4. `codex exec` CLI directly — blocked by relay policy: muyuan.do returns
   403 "channel does not allow the current client (codex_exec)". Interactive-only
   channel; do not spoof the client.
5. trellis-implement surgical single-file — 429
6. general-purpose + sonnet surgical single-file — 429

Main-session requests were unaffected throughout (orchestration turns all succeeded),
so the outage is specific to sub-agent sessions at the gateway.

## Options when resuming

- Gateway recovered → re-dispatch a trellis-implement agent with the /tmp brief
  (or per-file surgical agents; both patterns are prepared).
- User grants inline override ("你直接改" / "do it inline") → main session resolves
  the 6 files directly per the same policy.
- Codex relay policy changed / interactive codex available → drive codex interactively.

## Completed before the outage

- 07-10-web-bug archived; audit branch ff-merged into main (4ad02386) and pushed.
- sync/upstream-v3.18.0 branch + task created; PRD (all grilling decisions) and
  upstream-commit-inventory.md written. S2–S8 batch plan in prd.md.
