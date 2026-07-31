# Upstream commit inventory: v3.16.5..v3.18.0 (farion1231/cc-switch)

Authoritative hash list. PRD batch tables reference these; on hash mismatch this file wins.

## v3.16.5..v3.17.0
```
3d176b98 docs(release): add v3.17.0 release notes
c154d30b chore(release): v3.17.0
6eb217b2 revert(proxy): drop the 1-hour cache TTL option and TTL-bucketed write accounting
ac52c851 fix(codex): infer image capability for generated catalogs and resync takeover live on save
618723b4 feat(presets): promote SudoCode to paid sponsor across six clients
af58740b fix(proxy): align Codex OAuth client identity
99573d22 refactor(presets): pin context window values instead of form fields
940ddd33 feat(kimi): declare the 256K context window for Kimi For Coding
31ee4285 feat(pricing): seed gpt-5.6 alias rows and 1.25x cache-write rates
5c39dfbf feat(codex): declare gpt-5.6 context window for Claude Code takeover
f15184ed feat(codex): expose official routing and restore the built-in provider
f2c6d48e fix(providers): skip reachability probes for official OAuth entries
51d6c458 feat(codex): route native ChatGPT sessions through proxy takeover
13e7c1fc fix(usage): account for Anthropic cache write TTLs
b9263a80 fix(cache): strengthen prompt cache breakpoint injection
650905af fix(proxy): harden Responses and Anthropic protocol bridges
a078b4b2 feat(proxy): session-based prompt_cache_key routing for Codex Chat bridge
0e563b50 fix(cache): surface unsupported breakpoint counts
27ce0a51 fix(proxy): harden Responses reasoning and tool-call conversion
f991726f fix(usage): account for cache-write tokens across schema versions
06039540 refactor(health-check): remove per-provider test config
44279987 feat(profiles): add setting to toggle project switcher on main page
7479d10d feat(codex): add default model field to provider form
62e44c48 feat(pricing): seed Tencent Hunyuan Hy3 pricing
a7b4dd94 feat(pricing): seed GPT-5.6 Sol/Terra/Luna pricing
c6197ae3 fix(proxy): inject a single auth placeholder on managed Claude takeover (#5095)
f39d463c fix(usage): 修复 Codex 子代理使用量未计入统计 (#5187)
ded0b63a fix: handle missing provider keys and tool schema types (#5069)
6245caa6 Fix/opencode known field editors (#2907)
50270d5e fix: exclude Fable model env from Claude common config (#4272) (#5206)
99e11e08 feat(codex): support native Anthropic Messages protocol as upstream (#5071)
98ccde00 fix(usage): persist dashboard refresh interval (#5057)
95c917b3 feat(provider): add Zhipu team plan quota query support (#5128)
3538b392 feat(claude): add 1M checkbox to fallback model field (#5124)
ba531ca2 docs(readme): add new-api as a sponsor
88d5ffba fix(codex): move common-config TOML merge off smol-toml to backend toml_edit
94fc1cc0 fix(mcp): surface per-app failures when importing MCP servers from apps
11c173c7 fix(mcp): stop cross-app failures from blocking MCP re-projection
1f36f0cf feat(provider): extend switch-time common-config autosync to Codex
6d2ee247 fix(provider): re-project Codex MCP after unified-session toggle rewrites live config
473c2aaa fix(provider): exclude injected artifacts and routing fields from Codex common-config extraction
93f56198 fix(codex): strip synced [mcp_servers] from provider snapshots on backfill
8b1ce764 fix(mcp): fail closed when Codex config.toml is unparseable during MCP sync
fad5b4c0 Revert "fix(presets): point Volcengine/Doubao/BytePlus website links to official sites"
bad3610d refactor(presets): drop redundant 'OpenAI Compatible' preset
e78aa8a7 fix: sync openclaw and hermes live provider updates (#5098)
e191af4a fix: OpenCode live provider import updates (#4712)
d271d60c docs: add Codex Kimi routing guides
358bf1e2 chore(pnpm): settle build-script approvals for esbuild and msw
2df2212c fix(usage): reject transient transport failures so retry and keep-last-good work
468c93d4 ci: harden release supply chain
52534618 fix(proxy): close media fallback gaps for Volcano GLM 5.2 image 400s
afabe801 test(profiles): gate desktop-scope assertion by platform in profile roundtrip
7fada72d chore(code0): update partner invite link to agent register URL
9f7642e2 refactor(profiles): drop manual snapshot update now that switching autosaves
22159430 fix(profiles): use camelCase keys for current profile ids in frontend
754af2cc feat(profiles): split Claude Desktop into independent profile scope
3ec83578 fix(profiles): stop proxy server when profile switch leaves no takeovers active
f05ed3db fix(ui): invalidate proxy takeover status after profile switch
4f45601f feat(profiles): unconditionally disable proxy takeover before applying profile
4cf6f175 feat(profiles): autosave previous profile state on switch
dbb5999d refactor(profiles): shared project entity with per-scope switching
65a5464f feat(profiles): include Claude Desktop provider in project profiles
6179c188 fix(profiles): scope switcher to supported app tabs and relocate it
8f018a2d feat: add project profiles for snapshot-based config switching
b3e5e32c feat: add Claude subagent model config (#4830)
e606adfa fix(codex): display renamed session titles (#4927)
7a7d41c8 fix(subscription): display Codex free-plan 30-day quota window (#3651) (#4886)
ffc22ea7 feat(universal-provider): Auto-sync after adding and  drop unused addSuccess i18n key (#2811)
7a8b9562 Update Longcat presets to LongCat-2.0 (#4838)
0cda8d46 fix: 更新 OpenCode 会话恢复命令 (#2359)
```

## v3.17.0..v3.18.0
```
606e7bbe docs(release): add v3.18.0 release notes
df1751a8 chore(release): v3.18.0
f3108bf7 feat(providers): group presets into sponsors and non-sponsors
b3f3aee3 docs(readme): move Unity2.ai sponsor row ahead of Shengsuanyun
2bfca548 docs(readme): reorder sponsors and refresh RunAPI benefit copy
107eade3 chore(presets): add RunAPI referral link runapi.co/register
3537076a chore(presets): update ClaudeCN referral link to claudecn.ai/register
72efd64a docs(readme): serve Kimi sponsor banners from Moonshot CDN
f0b7b81c chore(presets): update ZetaAPI referral link to go/u117
b660061b chore(presets): update APINebula referral link to VjM74M
200aa142 style(providers): fix prettier formatting in ProviderForm
bc800123 docs(readme): update pinned Kimi sponsor copy to K3
325ba484 feat(grokbuild): curate standalone provider presets
a8daf7da fix(codex): add missing AiHubMix preset icon
f733def4 feat(grokbuild): add Grok Official provider with official-state import
a5aa1fd8 fix(providers): surface import errors and refresh list on failed import
6428e993 feat(proxy): flag managed-OAuth providers as routing-required
dbb5bd15 feat(codex): xAI (Grok) OAuth managed provider with native Responses compat
8dcedbc0 feat(tools): prefer xAI native Grok installer with npm fallback
eccb296a docs(changelog): expand unreleased Codex usage fix notes to house style
eff1e0cc feat(db): rebuild Codex usage on upgrade and via maintenance action
c9ac6efd fix(proxy): add stable usage keys and idempotent raw-response logging
df3e07ed fix(codex): strip forked history via parent-rollout token-prefix alignment
a10b569a feat(usage): add suspected-duplicate probe for Codex session imports
eb105eae perf(usage): coalesce session-sync notifications and serialize sync execution
01fca696 test(updater): align grok npm anchor expectation with PATH prefix
db444847 feat(codex): add xAI (Grok) native Responses preset
cdf0ee34 fix(pricing): add missing grok-4.5 pricing row to seed
09101e2a docs(xai): document Grok OAuth setup and disclose client identity reuse
e9317f47 feat(xai): add Grok account management UI with four-locale strings
615c99c6 feat(xai): add Grok OAuth presets for Claude and Claude Desktop
a35209a6 feat(xai): add Grok OAuth device-flow backend and proxy routing
c4795e98 fix(codex): backfill parser-required catalog fields from static template
6fddcaa9 feat(pricing): add bare k3 alias for Kimi For Coding plan
aa0e441d docs: add Claude Code Codex routing guide
17b053ed fix(updater): resolve Node for anchored npm commands
e356fc6e fix(openclaw): price preset models at official list prices in $/M
f2045822 feat(presets): add Kimi K3 to Kimi open-platform presets
2bfecead feat(pricing): add Kimi K3 to built-in model pricing table
62747058 feat(logging): capture frontend errors to disk with structured redaction
22d2872c feat(logging): persist diagnostics across restarts and redact secrets
7f028632 feat(presets): restore SudoCode.us to coexist with SudoCode.chat sponsor
3bc828ae fix(windows): eliminate console flash and UI freeze on provider switch
08710d51 fix(proxy): default Codex tool parameters to object schema (#5315)
613fef70 fix(codex-chat): attach reasoning forward for responses bridge (#5508)
997be22b fix(tray): detect system locale for first-run language instead of hardcoding zh (#4355)
edea624a fix(skills): preserve deleted default repositories (#5356)
1c0ee0c5 feat(grokbuild): add first-class Grok Build support (#5453)
f6e37ed9 fix(ci): run backend checks on Windows/macOS and repair platform-gated tests (#5138)
1cc52c7e docs(readme): add SubRouter sponsor entry across four locales
6d316c0b fix(codex): preserve streamed tool call identity and order (#5310)
9ca1a41f fix: normalize function parameters type to "object" for strict OpenAI-compatible providers (#4706)
c8b0d60c docs(guides): add Codex + Claude local routing guide in three languages
7e73a1ff fix(i18n): add missing proxyReasonAnthropicMessages key across locales
```

## Deferred (v3.18.0..product-upstream/main, NOT in this sync)
```
878c26f3 feat(proxy): extend tool-result media handling to all conversion bridges
6c9d444c fix(proxy): move Codex tool-result media out of stringified tool text
34cbb375 feat(usage): surface Grok Build session source in usage UI
cd161f44 feat(usage): import Grok Build official-mode usage from session logs
3cf84ca3 fix(usage): centralize cache-inclusive app set and cover grokbuild in cost backfill
15d5dbe0 feat: add Grok official subscription quota query
a377d793 docs: sync v3.18.0 release notes and guide cross-links to new guide titles
846fbdd1 docs(guides): retitle Codex Claude guide to match
3a9fb13a docs(guides): add en/ja translations for Claude Code GPT guide and retitle
```
