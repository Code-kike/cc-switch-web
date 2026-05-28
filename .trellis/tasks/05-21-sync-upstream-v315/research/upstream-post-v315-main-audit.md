# Upstream post-v3.15 main audit

Date: 2026-05-28

Range audited:

```bash
upstream-v3.15.0..refs/remotes/upstream/main
```

Fetched source:

```bash
https://github.com/farion1231/cc-switch.git main
```

Latest fetched upstream main:

```text
3c3d4174 Enable Codex goals in provider templates (#3089)
```

Local upstream release tags still only include:

```text
upstream-v3.15.0
```

Decision baseline:

- Do not blind-merge upstream main.
- Do not bump local Web fork version for unreleased upstream main commits.
- Port only focused slices that affect Web-visible frontend, Web-shared backend, proxy/provider behavior, usage accounting, session/auth/config/database, or Web API command parity.
- Keep old docs deletions and unrelated dirty files out of this audit.

## Candidate matrix

### Must consider now

These are Web/shared-backend relevant and have low-to-medium implementation scope.

| Commit | Upstream subject | Evidence | Why it matters for Web fork |
| --- | --- | --- | --- |
| `e9d84af5` | `fix(session): include Codex archived sessions (#2861)` | `src-tauri/src/session_manager/mod.rs`, `src-tauri/src/session_manager/providers/codex.rs` | Shared session manager behavior can affect Web session browsing/searching for Codex history. |
| `8e21b061` | `Fix custom usage script summaries (#3129)` | `src-tauri/src/usage_script.rs`, `src/components/UsageScriptModal.tsx`, `src/utils/usageDisplay.ts`, `tests/utils/usageDisplay.test.ts` | Directly overlaps Web usage-query/custom script behavior and current usage summary contracts. |
| `9c2add9a` | `fix(proxy): Claude-compatible streaming empty tool_calls resets block state (#2915)` | `src-tauri/src/proxy/providers/streaming.rs` | Shared proxy streaming correctness fix; likely relevant for server mode. |
| `3c3d4174` | `Enable Codex goals in provider templates (#3089)` | `src/components/providers/forms/CodexConfigSections.tsx`, `src/utils/providerConfigUtils.ts`, i18n, Codex template tests | Existing Web provider UI consumes these templates; portable if reconciled with current forms/tests. |
| `707a5593` | `feat: add MiMo reasoning_content support for Claude Code proxy (#2990)` | `src-tauri/src/claude_desktop_config.rs`, `src-tauri/src/proxy/providers/claude.rs`, `src-tauri/src/proxy/providers/transform.rs` | Shared proxy transform behavior; useful if Web-managed Claude-compatible providers use MiMo reasoning payloads. |
| `95f2dd41` | `feat(codex): preserve OAuth login state during third-party provider switching` | `src-tauri/src/codex_config.rs`, config/provider/proxy services, `ProviderCard`, Codex config hooks, `providerConfigUtils` | Relevant if Web supports Codex OAuth/provider switching. Scope is broader than a one-file bugfix, so port after small backend fixes. |
| `177eef66` | `fix(quota): sort ZhiPu tiers so missing nextResetTime maps to five_hour bucket` | `src-tauri/src/services/coding_plan.rs` | Small service fix; include if the Web fork exposes quota/coding-plan state. |
| `62928c62` | `fix(ui): remove fixed width constraint on AppSwitcher text to prevent clipping (#3161)` | `src/components/AppSwitcher.tsx` | Small frontend UX fix; safe if current Web AppSwitcher still has the clipping constraint. |
| `e25682d3` | `fix(proxy): source takeover model fields from target provider on managed accounts` | `src-tauri/src/services/proxy.rs` | Shared proxy takeover behavior; should be checked against Web-managed account routing. |

Recommended first port order inside this group:

1. `e9d84af5`, `9c2add9a`, `177eef66`, `62928c62`, `e25682d3` as small isolated fixes.
2. `8e21b061` because it overlaps usage summary semantics and needs focused tests.
3. `3c3d4174` because it changes provider template semantics and frontend tests.
4. `707a5593` if Web use cases require MiMo reasoning support.
5. `95f2dd41` last, because it touches config, deeplink, provider live reads, proxy service, and frontend provider state.

### Needs deeper review

These appear relevant but are broad enough to deserve a separate design/port batch.

| Commits | Theme | Evidence | Recommendation |
| --- | --- | --- | --- |
| `1c82b8a3`, `74acf1e3`, `22fbe6f1`, `90b7f251`, `2a4651a2`, `f2935a3d`, `9d357098`, `72bc912e`, `ead9e22b`, `b710c654`, `5048ed63`, `44d9aabb`, `f9db9913`, `279b9eab` | Codex Chat bridge/routing stack | New `codex_chat_*`, `streaming_codex_chat`, `transform_codex_chat` modules plus proxy/session/provider/frontend preset changes | Treat as its own feature sync. Porting piecemeal risks breaking protocol conversion and cache/session semantics. |
| `ee2d634d`, `e3df8658`, `820c4db1`, `f8b4d67b`, `768c5f9f`, `ea604a18`, `ce232a14`, `014c82d2`, `108dda17`, `c6fd2415`, `67185974`, `3a77861d`, `7cad61be`, `5de0a0dc`, `88ba908b` | Managed CLI tool install/update settings panel | `src-tauri/src/commands/misc.rs`, `src/components/settings/AboutSection.tsx`, new `ToolInstallRow` and `ToolUpgradeConfirmDialog`, settings API/i18n | Web relevance depends on whether server mode should expose machine-level install/update actions. Needs Web security/product decision before porting. |
| `b15d9dfa` | Codex third-party history bucket migration to `custom` | New `src-tauri/src/codex_history_migration.rs`, `commands/settings.rs`, `settings.rs` | Database/settings migration behavior needs careful one-shot/idempotency review before applying to Web user data. |
| `8cdaf90d` | `deepClone` helper and `useTauriEvent` hook refactor | `App.tsx`, provider/universal panels, `useUsageCacheBridge`, new helper/hook/tests | Refactor may be worthwhile but should not block bugfix sync. Verify Web adapter event behavior before porting. |
| `5fd3ec0d` | Traditional Chinese locale | `settings.rs`, `LanguageSettings`, i18n schema/types/locales | Independent i18n feature. Low product risk but touches schema/types/settings; port only if desired now. |
| `c12d20ef` | Replace panic-prone proxy unwrap/expect patterns | Multiple proxy internals including `streaming_codex_chat` | Some fixes may depend on Codex Chat modules not present in the Web fork; split out only existing-module hardening if needed. |

### Defer or exclude for now

| Commits/theme | Reason |
| --- | --- |
| `fed892d3`, `3640a4e2`, `1232b49b`, `6172bfd5`, `4f0f103a`, `ddde7f13`, `398f40da`, `5315fa28`, `04af87bc`, `11edc96a`, `d7ede248`, `48473a5c` and similar docs-only/README/manual updates | Docs-only or release/manual content for upstream desktop. Current Web fork has held-aside docs deletions, so do not mix these into code sync. |
| `05ba2801`, `864593bb` | Claude Desktop profile-specific fixes. Revisit only if Web exposes Claude Desktop profile management or the shared backend fix is required by Web proxy takeover. |
| `5ae9c260`, `0977dcd1`, `c9efec29` | Desktop terminal/skills sync/install behavior. Web relevance appears low unless the Web UI directly exposes those flows. |
| `76b4c8b5`, `910ca3b4` | Sponsor/preset cleanup. Port only if product wants upstream preset removals; otherwise low technical urgency. |
| `0fb7fd12` | Xiaomi MiMo Token Plan presets. Likely low risk but product/preset-content decision rather than a bugfix. |

## Suggested next decision

Start with the "Must consider now" group, but port only the smallest isolated backend/UI fixes first. Keep Codex Chat bridge, tool management, history migration, Traditional Chinese localization, and docs-only updates as separate batches.

