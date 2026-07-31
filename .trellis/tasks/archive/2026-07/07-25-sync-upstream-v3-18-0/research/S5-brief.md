# S5 brief — Grok OAuth + Grok Build (upstream v3.18.0)

Branch: `sync/upstream-v3.18.0`

## Goal

Port the v3.18.0 Grok feature cluster into the web-first fork as one reviewed
batch while preserving the fork's dual-runtime, security, and live-config
contracts.

## Authoritative upstream order

1. `1c0ee0c5` — first-class Grok Build backend/UI/CLI integration
2. `17b053ed` — resolve Node for anchored npm CLI commands
3. `a35209a6` — xAI OAuth device-flow backend and proxy routing
4. `615c99c6` — xAI OAuth Claude presets (Claude Desktop portion excluded)
5. `e9317f47` — xAI account management UI
6. `cdf0ee34` — `grok-4.5` pricing seed
7. `db444847` — Codex native Responses API-key preset
8. `8dcedbc0` — native Grok installer with npm fallback
9. `dbb5bd15` — Codex xAI OAuth provider and native Responses compatibility
10. `6428e993` — managed-OAuth routing-required SSOT
11. `a5aa1fd8` — live-import error surfacing/query refresh
12. `f733def4` — Grok Official provider and official-state import
13. `a8daf7da` — Codex AiHubMix preset icon
14. `325ba484` — standalone curated Grok Build presets

The PRD lists the same commit set grouped by theme; implementation uses the
topological order above so follow-up commits apply against their dependencies.

## Fork adaptations

- Keep `AppType::GrokBuild` as a first-class shared application across database,
  providers, proxy, MCP, skills, prompts, sessions, settings, and frontend tabs.
- Preserve Web-only runtime files and add Web API/adapter parity for every new
  shared command. Mutations inherit the existing unauthenticated + same-origin
  intent posture; do not add login/session authorization around xAI management.
- Keep OAuth secrets out of query strings and logs. Credential/token persistence
  must use the existing private managed atomic-write path and redaction registry.
- Inject xAI credentials only for the pinned `api.x.ai` origin. Managed OAuth
  providers always require local proxy routing.
- Preserve the fork's S2 Codex OAuth/auth-preservation behavior and S3/S4 managed
  config projection/locking; layer xAI support alongside it rather than replacing
  those implementations with older upstream code.
- Claude Desktop has no fork runtime surface. Exclude its preset, form, tray,
  locale, and test additions while retaining generic managed-OAuth helpers used by
  Claude/Codex/Grok Build.
- Keep `zh-TW.json` deleted; add new copy only to retained `en`, `ja`, and `zh`
  locale files and preserve locale parity.
- The desktop app updater remains disabled. Grok/Codex/Claude CLI lifecycle code
  is in scope only where it manages installed developer tools, not application
  updater/release supply-chain behavior.
- Grok Official represents the CLI's native OAuth state and must not cause CC
  Switch to read, persist, overwrite, or proxy-take over native Grok credentials.
- Do not import v3.18.0..main Grok Build usage/quota commits; they remain the
  explicitly deferred next-sync cluster.

## Validation

- Focused Rust tests for AppType/schema/config/proxy/xAI OAuth/Responses transforms.
- Focused frontend tests for Grok Build forms/presets, xAI account management,
  routing-required behavior, imports, and icons.
- Desktop and Web clippy/check, unfiltered Rust library tests, TypeScript,
  Prettier, Web route coverage, retained-locale parity, and full Vitest.
- Final S5 report must account for all 14 upstream commits and list all fork-authored
  Web/runtime adaptations.
