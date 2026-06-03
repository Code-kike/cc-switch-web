# Remediate audit findings (pricing, CI gates, web hardening, parity, hygiene)

## Goal

Fix recommendation groups **#2–#6** from the 2026-06-03 comprehensive review. **#1 (wiring real auth/CSRF onto the web API) is explicitly OUT OF SCOPE** per the user. The aim: get the backend CI gates green, close the silent cost=0 pricing trap, harden the web SSRF surface, restore dual-runtime parity gaps, and tidy repo hygiene — without regressing desktop behavior.

## What I already know (from the review + direct verification)

- `HEAD` is **82 commits ahead of `origin/main`** (unpushed). So CI hasn't seen these commits; the two backend gates fail **only when pushed** — fix before any PR.
- **Pricing seed forms (verified):** Anthropic seeds are **dash** form (`claude-opus-4-8`, `claude-sonnet-4-6-20260217`); MiniMax/GLM/GPT seeds are **dot, lowercase** (`minimax-m2.7`, `glm-5.1`, `gpt-5.5`, `gpt-5.2-codex-low`). ⇒ a blanket `.`→`-` in the cleaning chain is **WRONG** (would break every gpt-5.x / minimax / glm row). Correct fix is a **fallback chain** (see Technical Approach).
- **SSRF constraint (verified):** `services/model_fetch.rs::fetch_models` and `services/speedtest.rs::test_endpoints` are shared by **desktop commands** (`commands/model_fetch.rs:18`, `commands/provider.rs:238`) **and** web handlers (`web_api/handlers/config.rs:457`, `web_api/handlers/system.rs`). A guard in the shared service would block legitimate desktop use of local endpoints (Ollama/LAN). ⇒ guard must live in the **web handler layer only**.
- **cfg-gate targets (verified):** `services/web_update.rs` is consumed only by `web_api/handlers/system.rs` (web-only) ⇒ safe to `#[cfg(feature="web-server")]`. `proxy/provider_router.rs:205 update_app_configs` has **no callers** ⇒ dead.
- **M3/M5 templates (verified):** web route to mirror at `web_api/handlers/auth.rs:90,349` (`get_codex_oauth_models`); FE submit guards to mirror at `ProviderForm.tsx:802-846` (`codexAuthError` already in scope at :403).

## Requirements

### #2 — Pricing: kill the silent cost=0 trap (H1/H5, H2, M1)
- Rewrite `services/usage_stats.rs::find_model_pricing_row` to try candidate keys in order, returning the first hit:
  1. `cleaned` (existing: strip `provider/` prefix, strip `:suffix`, `trim`, `@`→`-`)
  2. `cleaned.to_lowercase()`  ← fixes MiniMax/GLM mixed-case (M1)
  3. `cleaned.to_lowercase().replace('.', "-")`  ← fixes Anthropic dotted `claude-opus-4.8` → `claude-opus-4-8` (H1) WITHOUT breaking dot-lowercase ids (they hit at step 2 first)
  - keep the warn-and-return-None tail.
- Add the bare `claude-sonnet-4-6` seed row in `database/schema.rs::seed_model_pricing` (mirror `claude-sonnet-4-6-20260217`: `3 / 15 / 0.30 / 3.75`) — date-suffix mismatch, not fixable by normalization (H2).
- Add regression tests in `usage_stats.rs`: `MiniMaxAI/MiniMax-M2.7`→Some, `ZhipuAI/GLM-5.1`→Some, bare `claude-sonnet-4-6`→Some, `anthropic/claude-opus-4.8`→Some (the existing :2763 test), and a **guard test** that `gpt-5.5` / `minimax-m2.7` STILL resolve (no fallback regression).

### #3 — Backend CI gates green (clippy -D warnings + cargo test)
- `services/mod.rs:26,53`: gate `pub mod web_update;` and `pub use web_update::WebUpdateInfo;` with `#[cfg(feature="web-server")]`.
- `Cargo.toml`: add `[[example]]` blocks for `web_proxy` and `web_services` with `required-features = ["web-server"]` (mirror `server`).
- `proxy/provider_router.rs:205 update_app_configs`: remove (no callers) or `#[allow(dead_code)]` if intended public API.
- `forwarder.rs:890` too_many_arguments, `mcp.rs:435` type_complexity, `usage_script.rs:16` doc-overindent: fix (type alias / doc reflow) or `#[allow]` with rationale.
- Iterate `cargo clippy ... -- -D warnings` to **0 warnings** (more may cascade once the above clear).

### #4 — Web SSRF guard (H4)  (H3 read-endpoint redaction DEFERRED — out of scope)
- Add an outbound-URL validator (shared helper, e.g. `web_api/middleware` or a small util): reject non-`http(s)` schemes; resolve host and refuse loopback / link-local `169.254/16` / RFC1918 / ULA unless allow-listed.
- Enforce it in `web_api/handlers/config.rs::fetch_models_for_config` and `web_api/handlers/system.rs::test_api_endpoints` **before** delegating to the shared service (desktop path untouched).
- Stop echoing upstream response bodies for non-allowlisted hosts in the web path.

### #5 — Parity / robustness mediums (M2–M6)
- **M2:** extract a shared `ProviderService` method that does import + snippet auto-extraction + legacy migration; call it from both `commands/provider.rs:105` and `web_api/handlers/providers.rs:283`. Wire `initialize_common_config_snippets` into `examples/server.rs` startup (mirror `lib.rs:1586`).
- **M3:** add `web_api/handlers/auth.rs` route `/auth/get-codex-oauth-quota` mirroring `get_codex_oauth_models`; remove `unsupported` flag at `web-commands.ts:81`. (implement full parity)
- **M4:** mount a web-mode bridge that `listen("__lagged", () => queryClient.invalidateQueries())` (at least usage/providers/proxyStatus/subscription).
- **M5:** in `ProviderForm.tsx` (~:1016), block submit when `codexAuthError` is set (mirror the opencode/openclaw/hermes guards at :802-846); on the catch branch surface an error toast + abort instead of silently falling back to the stale `settingsConfig`.
- **M6:** `config.rs::atomic_write` (~:204-258): create the temp file `0o600` (unix) and `set_permissions` after rename when no prior file existed; keep the existing perm-preservation branch for existing files.

### #6 — Repo hygiene
- README_ZH.md:151,244 / README_JA.md:154,245 dead links to deleted docs → fix/remove. [depends on docs decision]
- CHANGELOG.md: add `[3.16.0]` entry (or a pointer to `docs/release-notes/`).
- `pnpm format` the 24 flagged files (and `cargo fmt` if anything drifts).
- Stage + commit the 146 deleted docs as one reviewable commit (decision #1).

## Decisions (resolved 2026-06-03)
1. **146 deleted docs → commit the deletions.** Deliberate web-first cleanup; stage + commit as one reviewable commit; fix README_ZH/JA dead links; CHANGELOG points to `docs/release-notes/`.
2. **H3 secret-redaction → DEFERRED (out of scope).** Coupled to the skipped #1 auth and risks breaking the web UI key-edit flows. Only H4 SSRF in #4.
3. **M3 codex-oauth-quota → implement the web handler (full parity)**; remove the `unsupported` flag.

## Decision (ADR-lite)
**Context:** the synthesis recommended adding `.`→`-` to the pricing cleaning chain and adding SSRF guards to the shared service.
**Decision:** (a) pricing uses a **fallback chain** (exact → lowercase → lowercase+`.`→`-`) rather than mutating the single cleaned key, because seeds split between dash-form (Anthropic) and dot-lowercase (gpt/minimax/glm) and a blanket replace would break the latter; (b) SSRF validation lives in the **web handler layer**, not the shared `model_fetch`/`speedtest` service, so desktop local-endpoint use is preserved.
**Consequences:** pricing resolution is order-sensitive (documented + guard-tested); the SSRF check is applied at the 2 web call sites via a shared helper.

## Acceptance Criteria
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` → 0 warnings
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` → compiles (examples gated) and all tests pass, incl. new pricing tests
- [ ] `cargo check --no-default-features --features web-server --example server` → still passes
- [ ] `pnpm typecheck` / `pnpm build:web` / `pnpm format:check` → pass
- [ ] `pnpm test:unit` non-web-server suites pass (web-server suites remain cargo/env-dependent)
- [ ] Pricing: dotted/mixed-case/bare default ids resolve to a real price (cost ≠ 0); gpt-5.x/minimax/glm unaffected
- [ ] SSRF: web `fetch-models`/`test-endpoints` reject private/loopback/link-local targets; **desktop local-endpoint use still works**
- [ ] M2–M6 behaviors verified

## Definition of Done
- Tests added/updated (pricing regression + guards); CI gates green; docs/CHANGELOG updated; desktop + web both build; no new clippy warnings.

## Out of Scope (explicit)
- **#1**: wiring real authN/authZ/CSRF/rate-limit onto the web API (user-deferred). C1 stays open.
- **H3**: secret-redaction on get-providers/export (deferred — coupled to #1 auth).
- Dependency CVE scan; broad refactors beyond what the lints require.

## Technical Notes
- Branch off `main` before implementing (currently 82 commits ahead, unpushed).
- Dual-runtime gate rule (memory): web-only code must be `#[cfg(feature="web-server")]`-gated or it breaks desktop clippy/test.
- Cargo env: `source ~/.cargo/env` (cargo 1.95). Run gates with `mkdir -p dist` first; `src-tauri/target` is ~17G.
