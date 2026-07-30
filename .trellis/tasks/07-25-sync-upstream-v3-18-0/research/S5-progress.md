# S5 progress log — Grok OAuth + Grok Build

Append-only checkpoint log. Format:
`<hash-or-area> DONE|ADAPTED|EXCLUDED|GATE — <note>`

- START — loaded `trellis-continue`, Phase 2.1 detail, `trellis-before-dev`, the
  frontend quality contracts, and both shared thinking guides. Confirmed S4 is
  fully checked/spec-updated/committed and S5 is the next unfinished batch.
- RECON DONE — resolved the authoritative topological order for all 14 S5
  commits. Recorded Web parity, Claude Desktop exclusion, retained-locale,
  managed-write, no-auth, S2/S3/S4 preservation, and deferred post-v3.18.0
  usage/quota constraints in `S5-brief.md`.
- 1c0ee0c5 ADAPTED — ported Grok Build as a first-class application across
  live config, providers, proxy, MCP, prompts, skills, sessions, settings,
  startup import, tool discovery, frontend forms/tabs, and retained locales.
  Added Web route/config-directory parity, normalized legacy `grokbuild`
  flags at UI boundaries, and retained the fork's runtime/security hardening.
- 1c0ee0c5 GATE — `pnpm typecheck`, focused Vitest (9 tests), desktop and Web
  cargo checks, two Grok proxy hot-switch rollback tests, `cargo fmt --check`,
  `pnpm check:web-routes`, `pnpm check:locales`, and diff checks passed.
- 17b053ed EXCLUDED — the fork has no executable managed CLI install/update
  command runner; importing its anchored npm prerequisite would add a new
  unauthenticated machine-level execution surface. Preserve the user-facing
  installer intent later in 8dcedbc0 via copyable native-install guidance with
  npm fallback.
- a35209a6 ADAPTED — ported xAI device-flow/account storage, managed provider
  auth, Claude Responses transforms, and pinned `https://api.x.ai/v1` routing.
  Injected the xAI manager through runtime-neutral proxy context and Web
  `ApiState`; added shared desktop/Web model fetching; retained the fork's
  unauthenticated same-origin Web posture; and excluded Claude Desktop changes.
- a35209a6 HARDENED — xAI storage uses the private atomic writer and rejects a
  final symlink; managed placeholders are rejected for the exact xAI v1 origin
  while unrelated relays remain allowed; Web auth projection preserves
  `requires_reauth`; the standalone Web service shim now exposes the shared
  xAI model-fetch module.
- a35209a6 GATE — desktop library and Web example cargo checks, TypeScript,
  Web route coverage, 13 xAI manager tests, two managed-placeholder tests, and
  two Web auth dispatch/status tests passed.
- 615c99c6 ADAPTED — added the managed xAI OAuth preset for Claude Code and the
  shared `XAI_OAUTH` provider-type constant. Preserved the fork-only LemonData
  preset during the insertion conflict; kept the Claude Desktop preset source
  deleted and removed the desktop-only test case.
- 615c99c6 GATE — TypeScript and the focused Claude xAI preset test passed;
  staged diff and conflict-marker checks were clean.
- e9317f47 ADAPTED — added xAI account management to the shared OAuth hook,
  Claude provider form, model fetcher, and Auth Center. Preserved the Web API
  adapter import and remote-server credential warning, extended that warning
  to xAI, retained the fork's smaller Claude form contract, and kept Claude
  Desktop plus `zh-TW` deleted.
- e9317f47 HARDENED — provider saves reject missing or reauth-required linked
  accounts, persist xAI managed-account bindings, force Responses format, and
  prevent full-URL override for xAI. The Auth Center refreshes xAI status every
  15 seconds so proxy-detected refresh-token rejection becomes visible.
- e9317f47 GATE — TypeScript, seven focused xAI preset/hook/component/locale
  tests, retained-locale parity, Web route coverage, formatting, and diff
  checks passed.
- cdf0ee34 ADAPTED — seeded `grok-4.5` at the upstream xAI rates without
  importing the unrelated parent-context `grok-4.3` row. Added a prefixed
  `xai/grok-4.5` lookup regression that pins input/output/cache rates and
  prevents silent zero-cost usage.
- cdf0ee34 GATE — focused model-pricing lookup test and Rust formatting passed.
- db444847 ADAPTED — added the Codex xAI API-key preset as native Responses,
  with pinned xAI base URL, Grok 4.5 catalog metadata, and no managed-account
  provider type. Reapplied the single preset onto the fork's shorter curated
  preset list rather than importing unrelated parent-context providers.
- db444847 GATE — focused two-preset Vitest, TypeScript, formatting, and diff
  checks passed. The upstream-only wire-format extractor assertion was adapted
  to inspect the generated TOML directly.
- 8dcedbc0 ADAPTED — did not import the desktop lifecycle command runner.
  Updated the existing copyable install guidance to prefer xAI's native POSIX
  installer with a tempfile/download/cleanup flow and fall back to
  `npm i -g @xai-official/grok@latest`. Extended retained-locale copy and the
  Web tool-version integration fixture to include Grok Build.
- 8dcedbc0 GATE — eight About-section tests, TypeScript, retained-locale parity,
  formatting, and installer-command assertions passed. Real Web-server fixture
  coverage was updated for the final S5 integration run.
- dbb5bd15 ADAPTED — added the Codex managed `xAI (Grok) OAuth` preset and
  account-driven form flow, pinned the adapter to `api.x.ai` plus managed-token
  injection, and ported native Responses namespace flatten/restore plus xAI
  request sanitization as mirrored desktop/Web proxy-root modules. Preserved
  the fork's rich Codex errors, debug-only response bodies, bounded persisted
  errors, and session-aware usage logging instead of replacing the handlers.
- dbb5bd15 HARDENED — local xAI token `AuthError` is non-retryable and does not
  poison provider health or silently fail over, while upstream 401/403 remains
  retryable. Managed Codex saves no longer require editable API key/base URL,
  and `providerType` now resolves from the cross-app preset SSOT.
- dbb5bd15 GATE — desktop/Web cargo checks, the Web proxy mirror test, 22
  namespace/sanitization tests, 17 xAI routing/account tests, five focused
  frontend preset/form tests, TypeScript, Rust/Prettier formatting, and staged
  plus unstaged diff checks passed.
- 6428e993 ADAPTED — introduced a cross-app `providerNeedsRouting` SSOT and
  managed-OAuth provider-type registry for Claude, Codex, and Grok Build.
  Provider cards and switch warnings now treat managed credentials as routing
  required regardless of editable or legacy-missing API-format metadata, and
  readiness requires takeover of the current app rather than merely any
  running proxy process. Claude Desktop and `zh-TW` changes were excluded.
- 6428e993 GATE — 40 focused capability/card/action tests, TypeScript,
  retained-locale parity, Prettier, and diff checks passed.
- a5aa1fd8 ADAPTED — live-provider import failures now surface serialized
  Tauri/Web error details and invalidate the current app's provider query even
  on failure, so durable side effects such as seeded Grok Official rows render
  immediately instead of waiting for a later refresh.
- a5aa1fd8 GATE — 12 ProviderList tests, TypeScript, Prettier, and diff checks
  passed.
- f733def4 ADAPTED — added the canonical `grokbuild-official` seed, syntax-only
  Grok live snapshots, native-login detection, and an explicit-import recovery
  flow that selects Grok Official without materializing a custom `default`.
  Startup still uses the strict service path, so deleting the official row is
  respected across restarts. The shared manual-import wrapper serves both
  Tauri and Web, and the new ensure-seed command has an exact Web route.
- f733def4 HARDENED — Grok Official keeps an empty custom-model config, never
  reads or rewrites sibling native credential state, and cannot be proxy-taken
  over. Global/best-effort takeover skips native official live state while the
  strict per-app path rejects it. The Grok form hides custom model/API fields,
  bypasses custom validation only for official rows, and filters the Codex-only
  managed xAI OAuth preset.
- f733def4 GATE — 40 focused Rust Grok tests, four live-import integration
  tests, 20 focused frontend tests, desktop/Web cargo checks, TypeScript, Web
  route coverage, Rust/Prettier formatting, and staged/unstaged diff checks
  passed.
- a8daf7da ADAPTED — added the missing AiHubMix icon metadata to the fork's
  shorter Codex preset catalog (`aihubmix`, `#006FFB`) without importing any
  unrelated upstream preset churn.
- a8daf7da GATE — TypeScript, the focused preset/form tests, Prettier, and
  staged/unstaged diff checks passed.
- 325ba484 ADAPTED — replaced Grok Build's filtered Codex preset reuse with an
  independently maintained curated catalog, moved Grok Official into that
  catalog, removed cn_official/managed-OAuth/open-source-only providers, and
  normalized retained entries to Grok 4.5 Responses configurations. The fork's
  `CodexApiFormat` supports only `openai_responses | openai_chat`, so the
  upstream-only Anthropic wire-format branch/helpers were not introduced.
- 325ba484 GATE — 15 focused catalog/form tests, TypeScript, Prettier, and
  staged/unstaged diff checks passed.
- FINAL FIX — aligned fresh and v14→v15 `proxy_config` schemas by placing
  `live_takeover_active` consistently, preserving `failover_strategy`, and
  accepting SQLite's quoted post-rename DDL in the legacy-export fixture. The
  focused migration, three SQL-import cases, and restore-lock regression passed.
- FINAL GATE — desktop fmt/check/clippy passed; unfiltered library tests finished
  at 1694 passed / 0 failed / 2 ignored on the green rerun. Web check/clippy and
  the required namespaces passed: `web_api::` 26, `dual_runtime_parity::` 3,
  `web_proxy_lifecycle::` 7.
- FINAL WEB FIXTURE — the real-server About fixture passed 1 file / 3 tests,
  including Grok CLI version refresh and xAI npm metadata, in 71.93s.
- FINAL TRELLIS CHECK — corrected the Grok Official frontend-SSOT reference and
  the xAI restricted-writer description. Added a focused mutation regression
  proving the official preset returns the canonical seed without falling
  through to ordinary provider creation.
- FINAL SPEC — extended the canonical SQL restore contract with v15 column/value
  preservation and quoted-DDL compatibility, and documented the executable
  Grok Official/xAI managed-OAuth cross-runtime contract.
- FINAL FRONTEND GATE — typecheck, formatting, route parity (275 commands; zero
  missing/method mismatch/fallback), retained locales (2432 keys each), full
  Vitest (137 files / 724 tests), and `build:web` passed.
