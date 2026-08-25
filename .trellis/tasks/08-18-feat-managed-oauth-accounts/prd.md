# PRD — managed OAuth account selection for providers

> 父任务：`08-18-sync-upstream-v3.20.0`。本子任务独立规划/实现/归档。**父任务三子任务中的最后一个。**

## 范围

- `a2e22f33` Add managed OAuth account selection for providers (#3879) — **54 文件 / +11,601 / −1,131**
- **从父任务 S4b 移交（2026-08-18）**：`0455a92c` 的 Rust managed-codex 事务部分 + `ProviderForm.codexManagedAccount.test.tsx`。证据链：`0455a92c` 的 `mod.rs` hunk 引用 `preflight_managed_codex_live` / `managed_codex_oauth_account_id` / `managed_codex_add_transaction_error` / `write_preflighted_or_current_live`，这些函数在 `413c09e0..v3.20.0` 区间**仅由 `a2e22f33` 引入**（`git log -S` 证实），fork 全缺，无法独立移植。

## 已确认事实（代码库调研 2026-08-24）

### 基线对齐 —— 与前两个子任务相反，这次**明确不对齐**

| 文件 | 上游 `a2e22f33^` | fork | 上游 post | 本提交改动 | fork 漂移 |
|---|---|---|---|---|---|
| `services/provider/mod.rs` | 4760 | 4695 | 7436 | **+2831/−155** | 1705 |
| `services/proxy.rs` | 7766 | 7027 | 9998 | **+2517/−285** | **3661** |
| `codex_config.rs` | 4763 | 3215 | 5460 | +699/−2 | **3396** |
| `proxy/providers/codex_oauth_auth.rs` | 1127 | 1208 | 2128 | **+1053/−52** | 127 |
| `services/provider/live.rs` | 2648 | 2840 | 3110 | +470/−8 | 712 |
| `proxy/provider_router.rs` | 523 | 926 | 637 | +131/−17 | 465 |
| `forms/CodexOAuthSection.tsx` | 362 | 363 | 682 | +410/−90 | **1** |
| `forms/ProviderForm.tsx` | 2708 | 2289 | 2891 | +260/−77 | 683 |
| `forms/CodexFormFields.tsx` | 1311 | **304** | 1364 | +54/−1 | **1137** |

**对照**：`feat-pi-native-agent` 与 `feat-codex-alpha-websearch` 的主体文件漂移是 28/78 行，所以近万行能靠 `git apply --3way` 落在上游锚点上。本任务三个大文件是**大改动撞大漂移**（`proxy.rs` +2517 撞 3661、`provider/mod.rs` +2831 撞 1705、`codex_config.rs` +699 撞 3396），`--3way` 预期大量冲突 → 规划信息不足，需先做纯调研批次（见 Q2）。

**好消息**：`CodexOAuthSection.tsx` 漂移仅 **1 行**，本提交对它 +410/−90 → 该文件可直接落地。

### fork 缺失文件（7 个）
- 需新建：`components/providers/AuthSettingsPanel.tsx`（+25 纯新增）、`tests/components/FullScreenPanel.test.tsx`（+33）、`tests/components/ManagedAuthStatusError.test.tsx`（+86）、`tests/components/ProviderForm.codexManagedAccount.test.tsx`（+610 上游 / 521 fork 适配版）
- **丢弃**：`forms/ClaudeDesktopProviderForm.tsx`（+48/−10）+ `tests/components/ClaudeDesktopProviderForm.test.tsx`（+122）—— fork 无 claude-desktop（沿用 S5/pi 子任务既有裁定）
- **丢弃**：`i18n/locales/zh-TW.json`（+32/−1）—— fork 仅 en/ja/zh

### 无新命令、无 DB 迁移（两项均已证实）
- `lib.rs` diff 中 `+\s+commands::` 计数 **0** → 无新 Tauri 命令注册；`commands/auth.rs` 的 +49/−17 无新 `#[tauri::command]`（仅签名变更）。
- `src/lib/api/web-commands.ts` **未在 54 文件内** → 与上一子任务同理，`check:web-routes` 计数应保持 **292/280/0 不变**。
- `database/` 仅触及 `dao/providers.rs`（+100，查询列表扩展），**未触及 `schema.rs`** → 无 schema 迁移，`SCHEMA_VERSION` 保持 17。
- `lib.rs` 新增 startup 调用 `ProviderService::migrate_legacy_codex_official_managed_binding`（**数据迁移而非 schema 迁移**，把遗留 Codex Official 账号绑定迁到新结构）。

### S4b 移交测试的真实需求（推翻规划期假设）
- 文件是 **10 个用例**（PRD stub 原写 8 —— 更正）；fork 适配版 521 行存于 `7265596a`，上游版 610 行。
- 该测试**完全 mock 掉 `CodexOAuthSection`**，自带 `data-testid="allow-unbound-selection"` / `"allow-unbound-without-status"` 桩件 → **不需要真实 `CodexOAuthSection`，也不需要 `CodexFormFields` 的 1364 行完整版**。
- 测试渲染的是 **`ProviderForm`**（覆盖 `appId="codex"` 与 `appId="claude"` + `providerType: "codex_oauth"` 两条路径），不是 `CodexFormFields`。
- fork `ProviderForm` **已有大半状态接线**（S4b `84d54e7d` 落的）：`selectedCodexAccountId`(415)、`selectedAccountIsUsable`(959)、`isCodexOauthPreset`(1836)、`onCodexAccountSelect`(1859)。
- fork `ClaudeFormFields` **已 import 并渲染 `CodexOAuthSection`**（34 行 import，81–84 接受 codex-oauth props）。
- **真实缺口**：`CodexFormFields`（codex 原生路径）完全不接 codex-oauth props、不渲染 `CodexOAuthSection`；两侧均缺 4 个新 prop（`allowUnboundSelection` / `allowUnboundSelectionWithoutStatus` / `onSelectionConfirmed` / `onSelectionInvalidated`）；`ProviderForm` 缺 `canSelectCodexNativeLogin` 等推导。
- → `CodexFormFields` 的 1137 行漂移中**绝大部分是 fork 刻意裁掉的表面**（ClaudeDesktop 相关与其他 UI），与本任务无关。

### 新增安全面：managed OAuth id_token 写入 live auth
`codex_oauth_auth.rs` +1053/−52 是本提交最大 Rust 单文件增量。上游 commit message 明确：把所选 ChatGPT 账号的 `id_token` 写入 Codex official managed `auth.json`，使其与原生浏览器登录在字段上一致，并加 reauth 提示。`lib/api/auth.ts` 新增 `reauth_required`（Codex：账号早于 id_token 持久化支持）与 xAI 的 refresh 凭据失效标记。

## 前置依赖
- **强依赖**：父主体 S2 `d2b070c9`（never clobber Codex official ChatGPT login on takeover restore）**已落地**，否则账号选择会在 takeover restore 时覆盖官方登录。
- 前两个子任务（`feat-pi-native-agent`、`feat-codex-alpha-websearch`）已归档，无文件重叠。

## 约束（carry-forward）
- OAuth 账号存储/选择不得退化 fork 既有 auth 安全边界与无认证部署姿态。
- 双运行时等价；**本提交无新命令** → `check:web-routes` 计数必须保持 292/280/0 不变（计数变化即误引入）。
- 安全上限零退化：128 MiB body、2s JS deadline、16 MiB heap、256 KiB stack、32 MiB catalog，以及上一子任务 W2.5 新增的五个 `MAX_CITATION_DEDUP_*`。
- `SCHEMA_VERSION` 保持 **17**（本提交无 schema 迁移）。
- ClaudeDesktop hunk 全量丢弃；zh-TW 丢弃。
- 延期 Codex Chat routing stack 四文件仍须缺席（`transform_codex_anthropic.rs` / `transform_codex_chat.rs` / `streaming_codex_chat.rs` / `codex_chat_common.rs`）；若某函数被证明必需，按 spec「Deferred Upstream Stack — Private-Helper Exception」处理（私有 fn + 变异验证必需性）。
- 新守卫的降级方向按 spec「Degradation Direction」分类：**auth/账号绑定属授权决策 → fail-closed**。
- `.pi/`、`.pi-subagents/` 不得修改或提交。

## 验收标准
- [ ] managed OAuth 多账号选择在 Web API / browser UI / headless runtime 三态下正确。
- [ ] takeover restore 不覆盖官方 ChatGPT 登录（`d2b070c9` 行为不回归）。
- [ ] `ProviderForm.codexManagedAccount.test.tsx` **10** 个用例全绿（取回自 `7265596a`）。
- [ ] `0455a92c` Rust managed-codex 事务（`preflight_managed_codex_live` 等）落地后，S4b 移交的用例无回归。
- [ ] 遗留绑定数据迁移 `migrate_legacy_codex_official_managed_binding` 在 startup 幂等、失败不阻断启动。
- [ ] **无新命令**：`check:web-routes` 保持 292/280/0；**无 schema 迁移**：`SCHEMA_VERSION` 仍为 17。
- [ ] 安全上限零退化；managed `id_token` 写入不扩大凭据泄露面（日志不含 token、写入走既有 atomic + 0600 路径）。
- [ ] ClaudeDesktop / zh-TW hunk 零回潮；延期栈四文件仍缺席。
- [ ] 上游测试全量移植并全绿（不删测试、不弱化断言，只按 fork API 适配 mock）。
- [ ] 全量门禁：test:unit 全绿（非 flake 项）、test:integration（4 PRD flakes 外全绿）、Rust parity（`web_api::`/`dual_runtime_parity::`/`web_proxy_lifecycle::`）、web-routes、locales parity、build:web exit 0、smoke:web-server exit 0；与父主体及两个已归档子任务无回归。

## 裁定记录（brainstorm 2026-08-24，用户授权采纳推荐方案）

- **Q1 `CodexFormFields.tsx` 1137 行漂移如何处置**：**按 S4b 移交测试的断言反推最小 UI 表面，不整体对齐上游 1364 行**。
  - 调研推翻了规划期假设（原以为要补 ~1000 行缺失基线）：测试完全 mock `CodexOAuthSection`、渲染 `ProviderForm` 而非 `CodexFormFields`、fork `ProviderForm` 已有大半状态接线、fork `ClaudeFormFields` 已渲染 `CodexOAuthSection`。
  - 真实工作量 = `CodexFormFields` 补 OAuth section 渲染块 + 4 个新 prop 的双侧接线 + `ProviderForm` 补推导，而非补基线。
  - 方法论与 pi 子任务 Q3（只取 pi dispatch、不重构非 pi 路径）一致：fork 的精简版是刻意结果，整体对齐会重新引入已裁掉的表面并扩大回归面。测试是契约的可执行形式，让其全绿即满足移交条款。
- **Q2 三个大漂移文件（`proxy.rs` / `provider/mod.rs` / `codex_config.rs`，合计 +6047）如何分批**：**先做纯调研批次 W0，产出逐 hunk 可行性清单后再定实现分批**。
  - 逐 hunk 判定三类归属：(a) 落在 fork 存在且语义一致的锚点 → 可移植；(b) 依赖 fork 缺失的上游符号 → 需先补依赖或降级；(c) 命中 fork 已裁掉表面（ClaudeDesktop / 延期栈）→ 丢弃。
  - 理由：前两个子任务分批顺利是因为规划期已确认基线对齐；本次基线明确不对齐，直接开工很可能在第二批撞上依赖倒置而被迫回滚重排。单批 2000+ 行冲突解决不可审查。
- **PRD stub 更正**：测试为 **10** 用例（非 8）；`CodexFormFields` fork 侧 **304** 行（非 294）、上游 **1364** 行（非 1394）。

## Open questions（W0 后回答）
- 实现批次切分（依赖 W0 的 hunk 归属清单）。
