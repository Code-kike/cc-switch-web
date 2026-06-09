# PRD — Fix cross-validated non-security findings (round 2)

## Provenance
Follows a fresh full review of `main @ 786a09e0` (a 17-agent Claude Workflow review + an independent Codex gpt-5.5 review) and a **2-round Claude×Codex debate that CONVERGED** on a unified fix plan. See `research/fix-plan.md` (authoritative per-item spec) and `research/cross-review.md` (cross-comparison).

## ADR — Scope is NON-SECURITY ONLY (explicit user decision, 2026-06-08)
The project is personal-use / loopback-bound. Standing user directive: **do NOT do security work.** Re-confirmed this round via an explicit scope question — answer: *"维持原则：只修非安全"*.

**IN SCOPE (this task): items 7, 10, 11, 12, 13, 14, 15, 16.**

**OUT OF SCOPE — recommendations only, do NOT touch:** item 1 (auth wiring), 2 (SPA path traversal), 3 (systemd/README 0.0.0.0 defaults), 4 (session `sourcePath` read), 5 (resume command injection), 6 (debug-log prompts/responses), 8 (usage-script byte caps / DoS), 9 (SSRF). These stay documented in `research/fix-plan.md` and project memory; no code changes.

> Guard rail for implementers: if a fix appears to require touching auth/CSRF/rate-limit, the SPA static handler, systemd unit, SSRF blocklist, redirect policy, or log levels — STOP; that is out of scope.

## In-scope items, exact change, acceptance criteria

### Item 10 — Gemini cache tokens double-billed (cost correctness)
- **File:** `src-tauri/src/proxy/providers/transform_gemini.rs` (`build_anthropic_usage`, ≈1092–1123).
- **Current:** `input_tokens = promptTokenCount`; separately emits `cache_read_input_tokens = cachedContentTokenCount` with NO subtraction. Siblings `transform.rs:749` (`prompt_tokens - cached`) and `transform_responses.rs:~328` DO subtract. `gemini_native` served via the Claude route logs `app_type="claude"`; `calculator.rs:62` subtracts cache only for `codex|gemini`, NOT `claude` → cached portion billed as full input + as cache_read (double).
- **Change:** `input_tokens = promptTokenCount.saturating_sub(cachedContentTokenCount)`; keep emitting `cache_read_input_tokens = cached`.
- **AC:** new unit test: prompt=N, cached=C ⇒ `input_tokens == N-C` and `cache_read_input_tokens == C`. Reconcile the existing test at ~:1425–1436 (it asserts `cache_read==3`; add/adjust the `input_tokens` expectation). `cargo test` green. Scope note: only affects gemini_native-via-Claude; native Gemini app traffic unchanged.

### Item 11 — Pricing `[1M]` not composed with normalization (latent silent cost=0)
- **File:** `src-tauri/src/services/usage_stats.rs` (`pricing_lookup_candidates`, ≈1701–1748).
- **Current:** `one_m_stripped` is derived from `cleaned` only — not from `lower`/`dot_dash`. A `[1M]`-marked id that ALSO needs case/dot/date normalization misses every candidate ⇒ cost 0. (Latent: no current seed id triggers it; `[1M]` is only written onto already dash-lowercase Claude ids.)
- **Change:** also derive a candidate that strips `[1M]` from the lowercased/dot-dashed form (compose `[1M]`-strip with normalization). Preserve the ordering invariant (base candidates first; fallbacks must never override a more precise hit); keep dedup-stable order.
- **AC:** new unit test: a synthetic combined id (e.g. `Claude-Sonnet-4.6[1M]`) resolves to seed `claude-sonnet-4-6`. All existing pricing/normalization tests still pass.

### Item 7 — `useSettings` stale react-query cache in sibling side effects (frontend correctness)
- **File:** `src/hooks/useSettings.ts` (≈217 plugin path is correct; `launchOnStartup` ≈227 and `skipClaudeOnboarding` ≈382 compare against closure-captured `data`).
- **Change:** before mutation, read previous `Settings` via `queryClient.getQueryData(["settings"])` and use THAT for the `launchOnStartup` / `skipClaudeOnboarding` comparisons — mirror the already-correct plugin pattern.
- **AC:** new rapid-toggle test(s) mirroring `tests/hooks/useSettings.test.tsx:388` covering both side effects. `pnpm test:unit` green.

### Item 12 — Multipart inherits axum 2MB default → opaque 500 (reliability/UX)
- **Files:** `src-tauri/src/web_api/handlers/config.rs` (`import_config_upload` ≈683), `handlers/skills.rs` (≈380), `handlers/prompts.rs` (≈104); `web_api/routes.rs` sets no `DefaultBodyLimit`.
- **Change:** set an explicit, GENEROUS `DefaultBodyLimit` for these multipart routes (sized for realistic SQL/skill/prompt uploads), and return a clear **413** on over-limit instead of an opaque 500. NOTE: this is a reliability fix (legit uploads currently fail), NOT a security tightening — the limit must be larger than the 2MB default, not smaller.
- **AC:** test that an over-limit upload yields 413 with a clear message (not 500); a realistic (>2MB) export now succeeds. Existing tests green.

### Item 13 — web_api Rust `#[cfg(test)]` tests never run in CI (testing/CI)
- **File:** `.github/workflows/ci.yml` (web-server job).
- **Current:** `lib.rs` is `#![cfg(feature="desktop")]`, so default `cargo test` skips web_api; the web job only `cargo build`s the example + runs JS integration. `routes.rs:200–241` (and other web_api) unit tests never compile/run.
- **Change:** add a CI step: `cargo test --no-default-features --features web-server --example server --locked --manifest-path src-tauri/Cargo.toml` (the `--example server` target is required — plain `--features web-server` won't compile the example-only modules).
- **AC:** step runs green locally; yaml valid; the web_api unit tests actually execute.

### Item 14 — No React error boundary anywhere (frontend robustness)
- **File:** `src/main.tsx` (≈90 mounts `App` with no boundary); add a new `ErrorBoundary` component.
- **Change:** add a top-level class `ErrorBoundary` (`componentDidCatch` + `getDerivedStateFromError`) wrapping `App`, with a fallback UI + reload action. i18n the fallback strings across en/ja/zh.
- **AC:** test that a throwing child renders the fallback (not a blank app). `pnpm check:locales` green (all keys in en/ja/zh). `test:unit` green.

### Item 15 — Dual-runtime module lists hand-maintained, no parity check (architecture/maintenance)
- **Files:** `src-tauri/examples/{server,web_proxy,web_services}.rs` `#[path]` module lists vs `src/{proxy,services}/mod.rs` + `src/lib.rs` module set.
- **Change:** add a **test** (preferred over a build-script) that parses the example module declarations and asserts parity with the real module set (or fails on drift). The `#[path]` dual-runtime design itself is intentional — only the missing drift-guard is fixed. Decide the exact module sets to cover (proxy, services, top-level) and document it in the test.
- **AC:** parity test passes now; would fail if a module is added to `src` without the example shim. Runs in the web-server test target (so item 13's CI step covers it).

### Item 16 — `rust-embed` compiled into web-server but unused (build cleanup)
- **File:** `src-tauri/Cargo.toml` (≈36 feature list, ≈130 optional dep).
- **Current:** `web-server` enables `rust-embed`; no `RustEmbed` usage anywhere; assets served from disk (`routes.rs:162`).
- **Change:** remove `rust-embed` from the dep table + the `web-server` feature list. Verify no source references it. Do NOT switch to embedding (that's a behavior change) — just delete the dead dep. Update `Cargo.lock`.
- **AC:** `cargo check --no-default-features --features web-server --example server --locked` compiles; `cargo tree` no longer lists `rust-embed`; desktop build unaffected.

## Verification — full CI-equivalent gate suite (run per batch; BOTH formatters + web tests)
- **Rust** (`src-tauri`): `cargo fmt --check` · `cargo clippy --all-targets --locked -- -D warnings` · `cargo test --locked` · `cargo check --no-default-features --features web-server --example server --locked` · the NEW item-13 web-server test command · `bash scripts/gen-command-manifest.sh && … --check`.
- **Frontend:** `pnpm typecheck` · `pnpm test:unit` · `pnpm format:check` · `pnpm check:web-routes` · `pnpm check:locales` · `pnpm test:integration`.

## Process
1. Implement via `trellis-implement` sub-agents (dispatch prompt starts with `Active task: <path>`); keep fan-out ≤3 (local gateway limit).
2. **Self-review**: `trellis-check` + the full gate suite above.
3. **Codex review** of the diff; iterate until Claude's review and Codex's review CONVERGE.
4. Checkpoint commit per coherent batch.

## Non-goals
- Any security/privacy item (1/2/3/4/5/6/9) or the DoS item (8).
- Opportunistic refactors or behavior changes beyond each fix's stated change.
