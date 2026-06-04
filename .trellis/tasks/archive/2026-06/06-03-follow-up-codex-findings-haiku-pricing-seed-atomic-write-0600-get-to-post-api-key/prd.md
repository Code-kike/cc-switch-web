# Follow-up Codex findings: haiku pricing seed, atomic_write 0600, GET→POST api_key

## Goal
Address the 3 validated findings from the independent Codex review that the user opted to fix (options 1+2). Gates are already green. Commits extend the existing branch `fix/audit-remediation-pricing-ci-ssrf-parity` (PR #11).

**Decision (user, personal-use):** SKIP the SSRF hardening (#1/#2: redirect-follow bypass, 0.0.0.0, other outbound vectors) and C1 (web API auth/CSRF wiring) — "安全门禁" deemed unnecessary for personal use. Also skip Bedrock dotted-namespace pricing (LOW).

## Requirements

### #4 — Pricing: bare `claude-haiku-4-5` resolves (cost ≠ 0)
- `database/schema.rs::seed_model_pricing`: add a seed row for bare `claude-haiku-4-5` mirroring the existing dated `claude-haiku-4-5-20251001` (~line 1269) prices. Several presets default to bare `claude-haiku-4-5` (e.g. `openclawProviderPresets.ts:967,1015` — claudecn/runapi), which cleans to `claude-haiku-4-5` and currently misses (only the dated row is seeded) → cost 0. This is the sibling of the bare `claude-sonnet-4-6` row already added.
- Add a regression test (usage_stats.rs tests) asserting `claude-haiku-4-5` and a prefixed form (e.g. `claudecn/claude-haiku-4-5`) resolve to Some.

### #5 — `atomic_write` creates new credential files 0600 atomically (no race)
- `config.rs::atomic_write` (~204-258): for a NEW target (no prior file), create the temp file with `0o600` from the start under `#[cfg(unix)]` (e.g. `OpenOptions::new().write(true).create(true).truncate(true).mode(0o600)`), eliminating the create(0644)→write→chmod(0600) window Codex flagged. Keep the existing perm-preservation branch for pre-existing files; don't regress Windows.

### #3 — Move api_key out of URL: GET→POST for 3 endpoints
- Endpoints: `fetch_models_for_config` (config.rs), `get_balance` + `get_coding_plan_quota` (subscription.rs). All currently `method:"GET"` with `Query<T>`, so the web adapter serializes `api_key` into the URL query string (→ browser history / proxy & access logs).
- Change in `src/lib/api/web-commands.ts`: `method:"GET"` → `method:"POST"` for the 3 (paths unchanged). The adapter already sends `JSON.stringify(remaining)` as the body for non-GET, so args (incl. api_key) move to the JSON body.
- Change Rust routes `get(handler)` → `post(handler)` (config.rs ~194 route, subscription.rs:25,26) and handler extractors `Query(q): Query<T>` → `Json(q): Json<T>` (import `axum::Json`, `axum::routing::post`). Structs unchanged.
- Verify no other caller hardcodes GET for these paths (grep web-server tests / smoke script).

## Acceptance Criteria
- [ ] `cargo test` (incl. new haiku test) / `cargo clippy -- -D warnings` / `cargo fmt --check` green
- [ ] `pnpm typecheck` / `pnpm build:web` / `pnpm format:check` green
- [ ] `pnpm check:web-routes` green (parity is path-only, so method change is neutral — confirm `missing:0`)
- [ ] `claude-haiku-4-5` (and `claudecn/claude-haiku-4-5`) resolve to a real price
- [ ] The 3 endpoints work over POST with a JSON body; `api_key` no longer appears in the request URL; desktop (Tauri invoke) path unaffected

## Out of Scope
- #1/#2 SSRF hardening (redirect/rebinding/0.0.0.0/other outbound vectors); C1 web auth/CSRF wiring — user-deferred (personal use).
- Bedrock `global.anthropic.*` dotted-namespace pricing (LOW).

## Technical Notes
- Parity script `check-web-route-coverage.mjs` matches by PATH only (ignores method) → GET→POST safe.
- Adapter (`src/lib/api/adapter.ts`): POST is non-SAFE → adds X-CSRF-Token (verify_csrf is a no-op stub, so it passes) + an initial csrf-token fetch; functionally transparent.
- Branch: stay on `fix/audit-remediation-pricing-ci-ssrf-parity`; commits extend PR #11.

## Round 2 — cost-accuracy fixes from the final cross-validation (workflow + Codex, 2026-06-03)
The final validation confirmed the Round-1 fixes sound + all gates green, but surfaced that the "pricing id → cost 0 / wrong" trap recurs in code paths NOT touched by Round 1 (all PRE-EXISTING, same class as #2/#4). User decision: fix ALL cost items; SKIP the SSRF 0.0.0.0/:: gap (still SSRF, personal-use).

- **M2 — `[1m]`/`[1M]` suffix not stripped → cost 0.** `usage_stats.rs::find_model_pricing_row`: add a candidate that strips a trailing 1M bracket marker (reuse `proxy::model_mapper::strip_one_m_suffix_for_upstream`) so `claude-opus-4-8[1M]` → `claude-opus-4-8`. Must leave dot-lowercase ids (gpt-5.5 etc.) untouched. + test.
- **session_usage divergent matcher → cost 0.** `session_usage.rs::find_model_pricing_for_session` (~479) uses its own exact/date-strip/LIKE matching, NOT `find_model_pricing_row`, and does no `.`→`-`/lowercase — so dotted/mixed session-log ids (`claude-sonnet-4.6`, `claude-haiku-4.5`) miss → 0. ROOT-CAUSE fix: extract a shared `pub(crate) fn pricing_lookup_candidates(model_id) -> Vec<String>` (exact, lowercase, dot→dash, 1M-strip) used by BOTH lookups, so they can't drift again. + test.
- **M1 — OpenAI→Anthropic transform double-bills cached tokens (cost OVER-count).** `proxy/providers/transform.rs:666-684` + `streaming.rs:102-114` copy prompt_tokens into Anthropic input_tokens while separately emitting cache_read_input_tokens; calculator treats "claude" as cache-EXCLUSIVE (`calculator.rs:62`) so cached is billed twice. Fix: set Anthropic input_tokens = prompt_tokens.saturating_sub(cached_tokens) on the transform path (non-streaming + streaming), matching native-Anthropic semantics. HIGH-CARE: must NOT change native-Anthropic passthrough. + regression test asserting input cost excludes the cached portion.
- **Niche unseeded ids → cost 0.** Seed rows for unseeded preset-referenced ids (ark-code-latest, qianfan-code-latest, KAT-Coder-Pro/-Air, LongCat-Flash-Chat, Ling-2.5-1T, Hermes-4-405B, kimi-for-coding, opencode gemini-claude-opus/sonnet-4-5-thinking). ONLY seed prices that are AUTHORITATIVELY sourceable (preset-embedded price, alias of an already-seeded row, or well-known public price); for any id whose price cannot be sourced, DO NOT invent — list it for the user to provide.

### Round 2 — explicitly OUT
- SSRF `0.0.0.0`/`::` classifier gap (`is_unspecified()`) — user skipped (personal-use). 1-line fix if reconsidered.
- The deferred C1 / H3 / broader SSRF / Bedrock items (unchanged).
