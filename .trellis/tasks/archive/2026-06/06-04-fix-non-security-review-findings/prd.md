# Fix Non-Security Review Findings

## Goal

Fix the non-security issues identified during the project review so the web-server runtime is clearer, less noisy to validate, and safer against ordinary personal-use mistakes.

## What I Already Know

* The user explicitly excluded security hardening from this task.
* `pnpm check:web-routes` currently passes with `missing: 0`.
* `cargo check --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server` passes but emits many warnings.
* Web mode intentionally keeps local proxy runtime control unavailable; the frontend already displays a web-mode configuration-only notice.
* WebDAV download applies a remote snapshot to the local database and skills.
* Custom usage scripts execute in QuickJS and currently have HTTP request timeouts but no obvious JS evaluation interruption.

## Assumptions

* Scope is the five non-security items from the prior review:
  * reduce actionable web-server compile warnings where practical;
  * clarify README Web-mode capability boundaries;
  * make SPA static asset missing behavior return 404 instead of HTML for asset-like requests;
  * improve WebDAV download confirmation/preview UX;
  * prevent usage-script eval/extractor mistakes from hanging indefinitely.
* Security-only changes such as auth, CSRF, SSRF allow-lists, and network exposure policy are out of scope.
* Keep changes narrow and consistent with existing code patterns.

## Requirements

* Static asset handling:
  * Missing asset-like requests such as `.js`, `.css`, `.map`, images, and fonts return 404.
  * Client-side route paths without asset extensions still fall back to `index.html`.
* README:
  * Document that web-server mode is primarily remote configuration/management.
  * Document that local proxy runtime start/takeover remains desktop-only or configuration-only in Web mode.
* WebDAV:
  * Manual download flow must show remote snapshot metadata when available before applying remote state.
  * Confirmation copy must make clear that local DB and skills will be replaced from the selected remote snapshot.
* Usage scripts:
  * Custom script config/extractor execution must have a bounded evaluation path or equivalent interruption.
  * Existing usage-script behavior and result format must remain compatible.
* Warnings:
  * Remove straightforward warnings introduced by the web-server example/module wiring without large refactors.
  * Avoid broad `allow` blankets unless the item is intentionally unused in a conditional build.

## Acceptance Criteria

* [x] `pnpm check:web-routes` passes.
* [x] `pnpm typecheck` passes or any pre-existing failure is clearly reported.
* [x] `cargo check --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server` passes.
* [x] Targeted Rust tests for changed static fallback and usage-script timeout behavior pass.
* [x] WebDAV UI displays a confirmation that names remote snapshot metadata before download when metadata is available.
* [x] README documents Web-mode proxy runtime limitations.

## Out of Scope

* Authentication, CSRF, CORS, SSRF, and deployment exposure hardening.
* Implementing the proxy runtime in web-server mode.
* Large frontend redesign.
* Full cleanup of every warning in the inherited web-server build if it requires broad conditional-build refactors.

## Technical Notes

* Static fallback: `src-tauri/src/web_api/routes.rs`.
* README: `README.md`.
* WebDAV UI: `src/components/settings/WebdavSyncSection.tsx`, `src/lib/api/settings.ts`, `src/types.ts`.
* Usage scripts: `src-tauri/src/usage_script.rs`, `src-tauri/src/services/provider/usage.rs`.
* Web-server warning source includes `examples/server.rs`, `examples/web_services.rs`, and conditional exports under `src-tauri/src/runtime/mod.rs`.
* Relevant specs:
  * `.trellis/spec/frontend/quality-guidelines.md`
  * `.trellis/spec/guides/cross-layer-thinking-guide.md`
  * `.trellis/spec/guides/code-reuse-thinking-guide.md`
