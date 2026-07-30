# S5 report — Grok OAuth + Grok Build (upstream v3.18.0)

Branch: `sync/upstream-v3.18.0`. This batch ports the v3.18.0 Grok feature
cluster into the web-first fork as one reviewed unit. The implementation is left
staged for the team lead; this agent did **not** commit or push.

Checkpoint log: `S5-progress.md`. The log records the incremental adaptations and
focused gates; this report records the final per-commit accounting and full batch
validation.

## Porting strategy

The 14 commits were applied in dependency order from `S5-brief.md`, then reconciled
against the fork's existing Web runtime, S2 Codex OAuth preservation, S3 managed
configuration writers, and S4 profile/takeover lifecycle. Upstream desktop-only
pieces were not allowed to define the fork's architecture.

The final result has these deliberate boundaries:

- `AppType::GrokBuild` is a first-class shared application, not a Codex UI alias.
- xAI OAuth is a managed credential source for Claude and Codex providers and is
  injected only for the pinned `https://api.x.ai/v1` origin.
- Grok Official represents the CLI's own native OAuth state. CC Switch neither
  reads nor persists those native credentials and cannot proxy-take over that row.
- managed OAuth providers require local routing; editable API-key providers do not
  inherit that requirement accidentally.
- Web remains unauthenticated by design and keeps the existing same-origin mutation
  guard. Secrets remain in JSON request bodies and private local storage, never in
  query strings.
- Claude Desktop, `zh-TW`, updater/release supply-chain behavior, and post-v3.18
  Grok usage/quota commits remain excluded.

## Per-commit outcome — authoritative S5 order

| # | Upstream | Status | Port/adaptation rationale |
|---|---|---|---|
| 1 | `1c0ee0c5` | **ported (adapted)** | Added Grok Build across application parsing, database rows, providers, proxy takeover, MCP, skills, prompts, sessions, settings, startup restoration, tool discovery, frontend navigation/forms, and retained locales. The fork adds Web config-directory/route parity and normalizes legacy `grokbuild` flags at frontend boundaries. |
| 2 | `17b053ed` | **excluded** | The commit resolves Node for an anchored desktop npm command runner. This fork has no managed CLI install/update execution surface, and adding one would expose unauthenticated machine-level command execution. Its user-facing intent is represented later by copyable native-install guidance with an npm fallback. |
| 3 | `a35209a6` | **ported (adapted and hardened)** | Added xAI device flow, account storage, managed provider auth, Claude Responses transforms, and pinned xAI routing. The manager is injected through runtime-neutral proxy context and Web `ApiState`; storage uses the private restricted atomic writer, rejects a final symlink, and preserves `requires_reauth`. Claude Desktop hunks are excluded. |
| 4 | `615c99c6` | **ported (adapted)** | Added the managed xAI OAuth Claude Code preset and shared `XAI_OAUTH` provider type while preserving the fork-only LemonData preset. The Claude Desktop preset/test/locale surface is excluded. |
| 5 | `e9317f47` | **ported (adapted and hardened)** | Added xAI account management to the shared auth hook, Claude form, model fetcher, and Auth Center. Saves require a usable linked account, persist the binding, force Responses format, and disallow a full-URL override. Auth status refreshes periodically so token rejection and `requires_reauth` become visible. |
| 6 | `cdf0ee34` | **ported (targeted)** | Seeded `grok-4.5` with the upstream xAI input/output/cache rates and added prefixed lookup coverage. Unrelated parent-context pricing churn was not imported. |
| 7 | `db444847` | **ported (targeted)** | Added the Codex xAI API-key preset as native Responses with the pinned xAI base URL and Grok 4.5 metadata. The fork's shorter curated catalog and direct TOML assertion are retained. |
| 8 | `8dcedbc0` | **ported as guidance, desktop runner excluded** | Kept the fork free of a remote CLI lifecycle command runner. Existing copyable install help now prefers xAI's native POSIX installer using a temporary download/cleanup flow and falls back to `npm i -g @xai-official/grok@latest`. Tool-version coverage includes Grok. |
| 9 | `dbb5bd15` | **ported (adapted and hardened)** | Added the Codex managed `xAI (Grok) OAuth` preset, account-driven form flow, native Responses namespace flatten/restore, and xAI request sanitization. Desktop and Web proxy roots mirror the new transforms. Local token `AuthError` is non-retryable and does not poison provider health; upstream 401/403 remains retryable. The fork's richer Codex diagnostics and bounded persisted errors remain intact. |
| 10 | `6428e993` | **ported (cross-app adaptation)** | Added the shared `providerNeedsRouting`/managed-OAuth registry used by Claude, Codex, and Grok Build cards/actions. Readiness requires takeover of the current application, not merely any running proxy. Claude Desktop and `zh-TW` changes are excluded. |
| 11 | `a5aa1fd8` | **ported** | Live-import failures now surface structured Tauri/Web error details and invalidate the current application's provider query even on failure, so durable side effects such as a recreated official seed appear immediately. |
| 12 | `f733def4` | **ported (adapted and hardened)** | Added the canonical `grokbuild-official` seed, syntax-only live snapshots, native-login detection, and explicit-import recovery. Grok Official keeps an empty custom-model config, is excluded from takeover, never reads or rewrites native credentials, and is respected as deleted across ordinary startups. The manual import wrapper and exact Web ensure-seed route are shared across runtimes. |
| 13 | `a8daf7da` | **ported (targeted)** | Added the missing AiHubMix icon metadata (`aihubmix`, `#006FFB`) without importing unrelated upstream preset churn. |
| 14 | `325ba484` | **ported (adapted)** | Replaced filtered Codex-preset reuse with an independent curated Grok Build catalog. Grok Official is catalog-owned; managed OAuth, cn-official, and open-source-only entries are excluded; retained entries use Grok 4.5 Responses configurations. The fork does not add upstream's unsupported Anthropic wire-format branch. |

All 14 upstream commits are accounted for: 13 are represented by targeted or
adapted final behavior, and `17b053ed` is explicitly excluded because its prerequisite
desktop execution surface does not exist in this fork.

## Final architecture and fork contracts

### First-class Grok Build application

Grok Build participates in the same shared contracts as the existing applications:

- backend `AppType`, config directories, provider persistence, per-app proxy row,
  takeover state, failover, health, MCP/skills enablement, prompts, sessions, tray,
  settings visibility, startup recovery, and tool-version discovery;
- frontend navigation, app metadata, provider add/edit/list flows, proxy and
  failover controls, usage/pricing views, directory settings, skills/MCP panels,
  session browsing, and retained-locale copy;
- test fixtures, API types, MSW state, and application-flag normalization.

The provider catalog is intentionally standalone. It is not computed by filtering
Codex presets at runtime, so later Codex-only provider additions cannot silently leak
into Grok Build.

### xAI OAuth ownership and persistence

`XaiOAuthManager` owns CC Switch-managed xAI accounts. It stores only the managed
device-flow state required for provider use and writes through the existing private
atomic path. The Web and desktop runtimes share the same manager semantics.

Account bindings are validated when a provider is saved:

- the selected account must exist;
- `requires_reauth` accounts are rejected;
- managed providers use the account binding instead of an editable API key;
- Responses format and the pinned xAI origin are enforced.

The Auth Center exposes xAI as the third managed-auth provider alongside Copilot and
Codex OAuth. Web status mapping preserves the default-account flag and reauth state.

### Pinned xAI routing and retry behavior

xAI credential injection is host-scoped to `api.x.ai`; unrelated relay providers do
not receive xAI tokens merely because a placeholder resembles managed auth. The
managed placeholder is rejected when it cannot be resolved for the exact xAI v1
origin.

The Codex native Responses path adds two shared transforms:

- namespace flatten/restore for xAI-compatible tool naming;
- removal of request fields xAI does not accept.

Those modules are compiled by both `src/proxy/mod.rs` and the Web example shim.
Local account/token failures are terminal for that request and do not trigger silent
provider failover or mark the remote provider unhealthy. Actual upstream 401/403
responses retain the ordinary retry/failover behavior.

### Grok Official/native credential isolation

`grokbuild-official` is a durable provider seed and a representation of native CLI
login state, not a managed credential container. Its contract is:

- no native credential file is imported into SQLite;
- no native credential is overwritten or deleted;
- no custom API-key/model editor is shown;
- no proxy takeover is permitted, including global/best-effort takeover;
- explicit import can recreate/select the canonical row, while normal startup does
  not resurrect a row the user deliberately deleted.

### Database and canonical restore consistency

S5 allocates schema **v15**. The v14→v15 migration:

- rebuilds `proxy_config` with the Grok Build check value and inserts its row;
- preserves all existing per-app fields, including `failover_strategy` and the
  legacy `live_takeover_active` value;
- adds `enabled_grokbuild` to MCP servers and skills without changing existing
  application flags.

The final Rust gate found a fresh-vs-upgraded schema drift. The canonical fresh DDL
now places `live_takeover_active` in the same position as the v15 rebuild, and the
rebuild carries `failover_strategy` instead of resetting `random` to `sequential`.
Regression coverage also accepts SQLite's quoted `CREATE TABLE "proxy_config"`
form after a rename. Current SQL exports from upgraded legacy databases now pass the
fork's constrained canonical-restore validation.

Comments and startup messages now describe `proxy_config` as one row per application
rather than the obsolete three-row shape.

### Web API parity and security posture

The fork-authored Web/runtime work includes:

1. **Runtime injection**
   - `examples/server.rs` constructs and shares `XaiOAuthManager` with `ApiState` and
     proxy runtime context;
   - startup proxy restoration uses the shared application list, including Grok
     Build.
2. **Web shims**
   - `examples/web_proxy.rs` includes both xAI Responses transforms;
   - `examples/web_services.rs` includes the shared xAI model-fetch service;
   - `server.rs` includes the shared Grok config module.
3. **Auth/model API parity**
   - generic managed-auth start/poll/list/status/remove/default/logout handlers now
     dispatch `xai_oauth`;
   - `GET /api/auth/get-xai-oauth-models` exposes account-scoped model discovery;
   - status serialization carries `requiresReauth`.
4. **Official-seed parity**
   - `POST /api/providers/ensure-grokbuild-official-provider` recreates only the
     canonical seed;
   - both commands have exact `web-commands.ts` registrations.
5. **Frontend adapter parity**
   - auth, models, providers, skills, directories, and provider imports use the
     existing runtime-neutral adapter rather than direct Tauri-only calls;
   - structured Web errors and provider-query invalidation mirror desktop behavior.

No login/session authorization was added. State-changing Web calls still inherit the
same-origin intent middleware. Account IDs used by GET model discovery are not
secrets; device codes and credential-bearing mutations remain JSON bodies. No xAI
token is placed in a query string, frontend log, or provider-editable full URL.

### Claude Desktop, locales, and supply-chain exclusions

- No Claude Desktop preset, form, tray branch, runtime module, locale text, or test
  was resurrected. Generic managed-auth helpers are shared only where needed by
  retained Claude Code/Codex/Grok Build surfaces.
- `zh-TW.json` remains deleted. New text exists only in `en`, `ja`, and `zh`, which
  remain in exact key parity.
- The desktop application updater/release channel remains disabled. S5 only changes
  copyable developer-tool installation guidance.
- Post-v3.18 commits for Grok Build usage import and Grok subscription quota are not
  present; they remain the PRD's next-sync work.

## Fork-authored or materially adapted test coverage

Coverage added or extended in S5 includes:

- application parsing, schema v14→v15 preservation, per-app proxy rows, startup
  restoration, canonical SQL export/import, and Grok official seeding;
- Grok provider live import, native-login state, explicit recovery, takeover denial,
  hot-switch rollback, sessions, prompts, MCP, skills, and pricing lookup;
- xAI atomic storage/symlink rejection, placeholder resolution, account status,
  model fetching, routing, token error classification, Responses namespace handling,
  and request sanitization;
- exact Web auth dispatch/status mapping, Web route/shim parity, official-seed
  routing, and frontend duplicate-provider prevention for the seed-only mutation;
- standalone Grok preset catalog, legacy flag normalization, provider icons,
  managed-routing capability, forms, cards, provider actions, Auth Center, settings,
  pricing, skills/MCP, and retained locale parity;
- real Web-server About/tool-version coverage for `grok` and xAI npm metadata.

## Exact S5 gate matrix

Cargo commands were run from `src-tauri/`; frontend and integration commands were
run from the repository root.

| Command | Recorded result |
|---|---|
| `cargo fmt --all -- --check` | **PASS** |
| `cargo check --all-targets --locked` | **PASS** — finished in **13.34s** after the final schema repair |
| `cargo clippy --all-targets --locked -- -D warnings` | **PASS** — finished in **26.46s** |
| `cargo test --lib --locked` | **PASS** — final rerun **1694 passed / 0 failed / 2 ignored / 0 filtered**, **10.01s** |
| focused v14→v15 migration regression | **PASS** — preserves `random`, takeover state, retry count, MCP flags, and skills flags |
| focused SQL import regressions | **PASS** — 3 accepted legacy/current-export cases |
| focused restore-lock regression | **PASS** — 1 passed in isolation |
| `cargo check --locked --no-default-features --features web-server --example server` | **PASS** — 65 expected standalone-shim dead-code warnings, **6.74s** |
| `cargo clippy --locked --no-default-features --features web-server --example server -- -D warnings -A dead_code -A clippy::duplicate_mod` | **PASS** — **30.62s** |
| Web `web_api::` namespace | **PASS** — **26 passed / 0 failed** |
| Web `dual_runtime_parity::` namespace | **PASS** — **3 passed / 0 failed** |
| Web `web_proxy_lifecycle::` namespace | **PASS** — **7 passed / 0 failed** |
| `pnpm typecheck` | **PASS** |
| `pnpm format:check` | **PASS** |
| `pnpm check:web-routes` | **PASS** — **275 commands**, missing **0**, method mismatch **0**, parity fallback **0** |
| `pnpm check:locales` | **PASS** — en/ja/zh each **2432** keys, parity true |
| `pnpm test:unit` | **PASS** — **137 files / 724 tests** |
| `pnpm build:web` | **PASS** |
| `pnpm exec vitest run --config vitest.integration.config.ts tests/integration/AboutSection.web-server.test.tsx` | **PASS** — real Web process, **1 file / 3 tests**, duration **71.93s** |

The first post-repair parallel library run hit the already-observed five-second
scheduling timeout in `sql_import_holds_main_lock_across_safety_backup_and_replace`.
That regression passed in isolation, and the immediate unfiltered rerun completed
with zero failures. No product/schema failure remained.

Expected non-failing output was limited to standalone Web-shim dead-code warnings,
the integration runner's stale `baseline-browser-mapping` notice, and ordinary
negative-path test diagnostics.

## Final hygiene and state

- `zh-TW` and Claude Desktop runtime/preset surfaces remain absent.
- no post-v3.18 Grok usage/quota code was imported.
- updater/release supply-chain behavior remains untouched and disabled.
- no merge/cherry-pick/revert sequencer state is intended to remain.
- untracked `.pi/` and `.pi-subagents/` are unrelated and intentionally preserved.
- all S5 implementation, test, and report changes are intended to remain staged as
  one batch; no commit or push is performed here.
