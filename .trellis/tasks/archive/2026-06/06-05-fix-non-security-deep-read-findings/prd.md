# Fix non-security deep-read findings

## Goal

Fix the **non-security** issues surfaced by the three-round architecture deep-read (Part I risk register + Part II delta). The project is **personal-use only**, so ALL security findings are explicitly out of scope. This task targets correctness/data-loss bugs, dead code, maintainability, CI/tooling robustness, and docs accuracy.

## What I already know (sources)

- Part I full report: `/tmp/claude-1000/-home-orion-Workspace-github-cc-switch-web/411ca623-0f05-4a61-9d77-dc2b1e6f746d/tasks/w6n9e17lq.output` (risk register H1-H4 / M1-M22 / L1-L17)
- Part II full report: `/tmp/claude-1000/.../tasks/wacx18tk7.output` (delta H5-H8 / M23-M42 / L18-L32 + corrections C1-C6)
- Dual runtime: `desktop` (Tauri, default; whole `lib.rs` `#![cfg(feature="desktop")]`) vs `web-server` (Axum `web_api/`, `examples/server.rs`). `smol-toml` is ALREADY a FE dependency; `src/utils/uuid.ts` already exists (relevant to M35).

## Out of Scope (explicit) — ALL security findings

Excluded per "personal use, no security work": H1 (web unauth), H3 (SECURITY.md), H4-security framing, M4 (TLS/MITM), H5 (MCP RCE), H6 (usage-script SSRF), H7 (deeplink import), H8 (OAuth/plaintext token), M23 (file/SQLite read), M24 (rquickjs mem limit — DoS), M25 (desktop SSRF), M26 (terminal injection), M27 (deeplink JS implant), M28 (client impersonation), M29 (JWT no verify), L21 (.replace secret subst), L23 (validate_server_spec), L24 (base64 DoS), L25 (config_url SSRF), L16 (flatpak sandbox). NOTE: H4's CI-coverage *robustness* aspect is IN scope (see Tier 5), only its security framing is excluded.

---

## Triaged non-security findings (the work list)

### Tier 1 — Correctness / data-loss / functional bugs (HIGH value)
- **M21** silent cost=0: DB pricing seed has unseeded preset models + lossy `.`/case/date normalization in `find_model_pricing_row` (`database/schema.rs` seed, `services/usage_stats.rs`). [also in memory]
- **M32** Claude live `settings.json` overwritten WHOLESALE → silent loss of user's manual edits (`services/provider/live.rs:701-705`). Make it merge like Gemini/Codex.
- **M33** OpenCode `opencode.json` comments/key-order destroyed on first write (read as JSON5, written strict) (`opencode_config.rs:64-77`).
- **L29** `INSERT OR REPLACE` on `dedup_request_id` = last-writer-wins; proxy row + session-sync row sharing `session:{message_id}` silently overwrite (`proxy/usage/logger.rs:76-115`).
- **L30** common-config legacy subset auto-detect can strip fields a provider coincidentally shares (`services/provider/live.rs:354-369`).
- **M22** `switch_lock` not held by enable-time takeover writes (only hot-switch/restore) — concurrency seam (`services/proxy.rs:1124-2077`, `switch_lock.rs`).
- **L4** `streaming_idle_timeout` ignored in forwarder; failover OFF forces ALL timeouts to 0 → stalled SSE can hang indefinitely (`forwarder.rs:120`).
- **L1** `ActiveConnectionGuard`/`SseUsageFinishGuard` decrement via `Handle::try_current().spawn()`, skipped if no runtime → counter drift / lost trailing usage (`forwarder.rs:61-71`).
- **M30** `clean_schema` shallow (ignores `$defs`/`anyOf`/`oneOf`/`allOf`/`additionalProperties`) → strict OpenAI schema 400s (`transform.rs:500-519`).
- **M31** GeminiCli OAuth token never refreshed → 401 after ~1h (`gemini.rs:195-211`, `claude.rs:649-677`). [functional gap, not a security hole]
- **M40** ProviderForm validation bifurcation: for codex/gemini/OMO the zod-validated textarea is discarded; submitted config rebuilt from hook state ≠ validated field (`ProviderForm.tsx:1013-1077`, `provider.ts:38`).
- **M42** `UniversalProviderPanel` uses local `useState` + manual fetch, bypasses React Query → stale UI after external sync (`universal/UniversalProviderPanel.tsx:13-51`).
- **L32** `currentView`/`activeApp` persist independently → restored view under mismatched app renders wrong panel (`App.tsx:163-169`).
- **L27** `ProviderAdapter::transform_response` dialect heuristic can mis-route an error body lacking marker keys (`claude.rs:850-863`).
- **L28** `ModelMapping.map_model` substring `contains()` fixed haiku→opus→sonnet order can mis-map; `[1m]` strip lossy (`model_mapper.rs:56-83`). [also in memory]
- **L7** `useFormField` guard is dead code (`getFieldState` called before null check) (`components/ui/form.tsx:40-49`).
- **M35** `crypto.randomUUID()` throws on insecure/non-localhost context (OMO/OpenClaw/Hermes row keys, `ProviderForm.tsx:1098`) → route through existing `src/utils/uuid.ts` fallback.
- **L18** `navigator.clipboard.writeText` no fallback → unhandled rejection on insecure context (`proxy/ProxyPanel.tsx:345` + SessionManager/AboutSection).

### Tier 2 — Dead code / hygiene / trivial-safe
- **M20** dangling `lib.rs:4` comment referencing a missing `.claude/team-plan/...v3.14.1...` doc.
- **L13** `NoopEventSink` desktop-gated but `web_api/state.rs:15` doc claims web-test usability (code↔doc contradiction).
- **L31** `get_circuit_breaker_stats` permanent `Ok(None)` stub → FE circuit-breaker-stats view is dead (`commands/proxy.rs:412-422`).
- **M37** dead/duplicate proxy query layer `src/lib/query/proxy.ts` (same keys as `hooks/useProxyStatus.ts`, 6+ hooks with 0 importers).
- **M7** `CoreRuntime` graceful-shutdown helper desktop-gated yet zero call sites (`runtime/runtime_handle.rs:14`).
- **L8** `decodeBase64Utf8` fallback uses deprecated `escape()` (`lib/utils/base64.ts:37`).
- **L20** four imperative panel refs typed `useRef<any>` (`App.tsx:260-263`) — type them.
- **L19** header `ProxyToggle`/`FailoverToggle` render against no-op `proxy_web` stub in web mode (gate on `webMode`) (`App.tsx:1315-1330`).
- **H2-nonsec** `bootstrap.rs` dead scaffolding cleanup: drifted `SCHEMA_VERSION=3_140_110`, unused WAL/`busy_timeout` pragmas, `WEB_SESSIONS_SCHEMA`/`AUDIT_LOG_SCHEMA` consts (`bootstrap.rs:56-96`). Delete dead consts OR decide if WAL/busy_timeout SHOULD be applied to the real DB (open question). [the auth-table-creation aspect is security → excluded]
- **L12** manifest-only deps `syn`/`walkdir`/`proc-macro2` are non-optional → feature-gate behind the manifest bin (`Cargo.toml:128-130`).

### Tier 3 — Maintainability refactors (LARGE / higher regression risk)
- **M1** `forward_with_retry_inner` (~660 LOC) triplicates the success-handling block (`forwarder.rs:332,458,655`).
- **M13** `providerConfigUtils.ts` (1337 LOC) hand-rolled line-based TOML editing — consider `smol-toml` (already a dep).
- **L26** god-components `App.tsx` (1762) / `ProviderForm.tsx` (2244) split.
- **M3** native-Claude raw hyper write + `WriteFilter`/dummy-request hack (`hyper_client.rs`).
- **M6** web bootstrap couples to `src` via ~30 manual `#[path]` includes + inline re-implemented `app_store` (`examples/server.rs:25-182`).
- **M14** loose typing `settings_config: Record<string,any>` / `[key:string]:any` (`src/types.ts:14,370,396`).
- **M34** Hermes YAML section replacement heuristic + unmaintained `serde_yaml` (`hermes_config.rs:135-174`).
- **M2** circuit-breaker uses lifetime counters, no sliding window (`circuit_breaker.rs:268-272`).
- **L2** `select_providers` mutates breaker state during a "pure" selection pass (`provider_router.rs:73`).
- **L3** whole request body buffered in memory (200MB) + failover priming buffers full upstream body (`server.rs:333`).
- **L14** `update_settings` blocking disk writes under global write lock (`settings.rs:518-543`).
- **L26b** `commands/misc.rs` ~800-line `open_provider_terminal` concentration.

### Tier 4 — Behavior changes (MAY BE INTENTIONAL — need confirmation)
- **M8** rectifier trigger `"invalid request"` substring fires destructive thinking/signature strip + retry (`thinking_rectifier.rs:100-106`).
- **M9** `strip_thinking_blocks` can leave `content:[]` → rejected upstream → loops with M8 (`copilot_optimizer.rs:460-481`).
- **M10** destructive top-level `thinking` removal on tool-use continuations (`thinking_rectifier.rs:174-237`).
- **M11** warmup heuristic silently swaps requested model for `gpt-5-mini` (`copilot_optimizer.rs:121-129`).
- **L5** budget rectifier hardcodes 32000/64000 with no per-model awareness (`thinking_budget_rectifier.rs:10-16`).
- **L6** `deterministic_request_id` depends on implicit JSON key ordering (`copilot_optimizer.rs:504-538`).
- **M36** FE refresh amplification (`staleTime:0` + focus refetch + dense intervals) (`queryClient.ts:5-9`).
- **M38** over-broad usage invalidation: `usageKeys.all` refetch on each 200ms event (`useUsageEventBridge.ts:31`).

### Tier 5 — CI / tooling / build robustness
- **H4-robustness** web-server Rust only `cargo check` (no clippy/test); parity gate + web smoke + `*.web-server` integration + `gen-command-manifest --check` NOT in CI → wire into CI (`.github/workflows/`).
- **M12** `copilot-optimizer-config` has no web route (parity hole) — add route OR confirm `unsupported` marking is intended (`config.rs`, `web-commands.ts:279,703`).
- **M16** `rust-toolchain.toml` pins 1.95 but CI uses `@stable`; MSRV 1.85 never verified.
- **M17** `commands.manifest.json` never validated in CI (stale).
- **M18** `test:unit` excludes `*.web-server.test.tsx`; integration vitest config never invoked.
- **M41** two SSOT for parity (heuristic manifest vs authoritative `web-commands.ts`).
- **M39** query-key drift: per-provider usage keys hand-rebuilt in ≥4 sites instead of `usageKeys.script()`.
- **L9** locale key parity (~2607 keys) not enforced by CI.
- **L10** parity gate naive `.route()` regex can't see macro/variable routes.
- **L11** no `--locked` on any CI cargo invocation.
- **L22** usage-script source eval'd twice in two runtimes (nondeterminism) (`usage_script.rs:151-216`).

### Tier 6 — Docs accuracy
- **M19** README_ZH/JA still ship upstream desktop marketing, not the web-first narrative.
- **L15** host/port defaults inconsistent (`server.rs` `127.0.0.1:3000` vs unit `0.0.0.0:3010` vs install `:3010`) — the *consistency* fix (not the 0.0.0.0 security aspect).
- **L17** 103 tracked Trellis scaffold files + duplicate archived task dirs (likely already this branch's concern — verify/dedup).

---

## Decision (ADR-lite)

**Context**: ~45 non-security findings of widely varying risk/effort; personal-use project, security excluded.
**Decision** (user, 2026-06-05):
1. Scope = **真·全部 / everything** — all six tiers, INCLUDING Tier 3 large refactors AND Tier 4 proxy-behavior changes.
2. Tier 4 = **apply all per the deep-read recommendations** (narrow rectifier triggers, make warmup model-swap configurable, etc.).
3. (resolved by me) `bootstrap.rs`: DELETE the dead drifted consts (`SCHEMA_VERSION=3_140_110`, `WEB_SESSIONS_SCHEMA`, `AUDIT_LOG_SCHEMA`, unused pragma consts). Do NOT change the real DB's pragmas (no WAL/busy_timeout) — the single Mutex'd connection makes WAL largely moot; revisit only if web multi-process contention is observed. (Creating web_sessions/audit_log is security → excluded.)
4. (resolved by me) Branching = ONE feature branch `fix/non-security-deep-read-findings`; **checkpoint commit per batch** with clear messages so each tier is revertible. Verify gates green before each commit.

**Consequences**: long multi-batch effort; Tier 3/4 carry real regression risk → every batch must pass: `pnpm typecheck`, `pnpm test:unit`, desktop `cargo clippy`/`cargo test`, web `cargo check --features web-server --example server`. Behavior-changing batches (Tier 4, data-loss fixes) get new/updated tests.

## Implementation Plan (batches; each ends with gates + checkpoint commit)

- **B1 — FE quick-wins & dead code** (low risk, validates the gate pipeline): L7, L8, M35, L18, L20, L19, M37, L31(FE side), M42, L32.
- **B2 — Config-files data-loss/correctness** (Rust, +tests): M32 (Claude merge), M33 (OpenCode comments), L30 (legacy subset strip), M31 (Gemini OAuth refresh).
- **B3 — Proxy correctness** (Rust, +tests): M21 (cost=0 seed/normalize), M30 (clean_schema deep), L1 (guard decrement), L4 (idle timeout/0-timeout hang), L27, L28, L29, M22 (switch_lock), L2.
- **B4 — Tier 4 proxy behavior** (Rust, +tests): M8, M9, M10, M11, L5, L6.
- **B5 — Backend cleanup/refactor**: M20, L13, M7, L12, H2-nonsec (bootstrap dead consts), M1 (forwarder dedup), M2 (CB sliding window), L14, M3/M6/M34 (assess), L31(backend stat).
- **B6 — FE state/types/refactor**: M14 (types), M36/M38/M39 (query), M40 (form validation), M13 (TOML via smol-toml), L26 (god-component split — incremental).
- **B7 — CI/tooling**: H4, M12, M16, M17, M18, M41, L9, L10, L11, L22.
- **B8 — Docs**: M19, L15, L17.

Order rationale: lowest-risk first to validate gates, then data-loss/correctness (highest value), then behavior changes, then refactors, then CI/docs. Re-sequence if dependencies surface.

## Open Questions (resolved)

All four resolved above (scope, Tier 4, bootstrap, branching).

## Progress log

- **B1 ✅ committed `fb2e7377`** — L7 (dead guard removed, behavior-preserving), L8 (escape→TextDecoder), M35 (randomUUID→generateUUID, 5 files), L18 (clipboard→copyText, 3 files). typecheck + 526 unit tests green. M37 found NOT dead (still imported) → deferred to B6.
- **B2 ✅ committed `c1faedfe`** — M33 (OpenCode comment-preserving write via new json5_doc.rs + safety net), L30 (common-config strip now explicit opt-in), M31 (Gemini OAuth refresh via new gemini_oauth.rs). M32 = DESCOPED (Claude wholesale write is intentional; merge would cause cross-provider contamination on switch — evidence comment + `claude_live_snapshot_is_wholesale` test). Reviewed by trellis-check; cargo test --lib 1221 passed, clippy --lib clean, web check green.
  - **B2.1 (M32 residual)** folded into next batch: non-switch same-provider re-sync still wholesale-overwrites → preserve unmanaged user keys on same-provider re-sync only.
- **B3a (in progress)** — backend data/cost correctness: B2.1 (M32 residual), M21 (cost=0 seed+normalization), L29 (usage dedup last-writer-wins).
- Re-grouped remaining: B3b = proxy forwarding correctness (M30, L1, L2, L4, L27, L28, M22); B4 = proxy behavior (M8-M11, L5, L6); B5 = backend cleanup/refactor; B6 = FE state/types/refactor (incl. M37); B7 = CI/tooling (incl. M16/H4 pre-existing clippy); B8 = docs.

## Acceptance Criteria (evolving)

- [ ] Selected-scope findings fixed with tests where behavior changes
- [ ] `pnpm typecheck` + `pnpm test:unit` green; desktop `cargo clippy`/`cargo test` green
- [ ] Web build (`cargo check --no-default-features --features web-server --example server`) green
- [ ] No security findings touched (or only incidentally hardened with no scope creep)

## Definition of Done

- Tests added/updated for correctness fixes; lint/typecheck/CI green; docs updated where behavior changes.

## Technical Notes

- Dual-runtime trap: any `src/` Rust module referencing `tauri` breaks the web build (`examples/server.rs` re-includes via `#[path]`) and has NO default-CI coverage — verify web build after backend edits.
- `smol-toml` already a FE dep (M13); `src/utils/uuid.ts` already exists (M35).
