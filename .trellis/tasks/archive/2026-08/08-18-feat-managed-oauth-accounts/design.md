# Design — managed OAuth account selection for providers

## 架构与边界

移植 Product upstream `a2e22f33`（54 文件 / +11,601 / −1,131）+ 父任务 S4b 移交的 `0455a92c` Rust managed-codex 事务。本任务是父任务三子任务中的最后一个。

### 与前两个子任务的关键差异：基线不对齐

`feat-pi-native-agent` 与 `feat-codex-alpha-websearch` 的主体文件与上游 pre-commit 基线漂移仅 28/78 行，因此近万行改动能靠 `git apply --3way` 落在上游锚点上。本任务三个大文件是**大改动撞大漂移**：

```
services/proxy.rs         +2517/−285  撞  3661 行漂移
services/provider/mod.rs  +2831/−155  撞  1705 行漂移
codex_config.rs            +699/−2    撞  3396 行漂移
```

→ `--3way` 预期大量冲突，规划期信息不足以安全切分实现批次。**先做纯调研批次 W0**（Q2 裁定）。

### 移植方法

逐 hunk selective port，方法论沿用前两个子任务，但**先补调研**：W0 产出 hunk 归属清单 → 据此切分 W1..Wn → 每批全量门禁 + 单独 commit。

## 功能架构：managed OAuth 账号选择

### 数据流（上游最终态）

```
Auth Center / AuthSettingsPanel  ── 多 OAuth 账号列表（Codex / Copilot / xAI）
  → ProviderForm (appId=codex 原生 | appId=claude + providerType=codex_oauth)
      selectedCodexAccountId 状态 + canSelectCodexNativeLogin 推导
  → CodexFormFields / ClaudeFormFields
      <CodexOAuthSection
         onAccountSelect / onSelectionConfirmed / onSelectionInvalidated
         allowUnboundSelection / allowUnboundSelectionWithoutStatus />
  → 保存：mutations 剔除 OAuth secrets，仅持久化 accountId 绑定
  → Rust: ProviderService add/update 事务
      preflight_managed_codex_live → write_preflighted_or_current_live
      managed_codex_oauth_account_id / managed_codex_add_transaction_error
  → codex_oauth_auth.rs: 把所选账号 id_token 写入 official managed auth.json
  → proxy/provider_router.rs + services/proxy.rs: 按绑定账号解析出站凭据
```

> **W2 更正（2026-08-25）**：上游 `a2e22f33` 曾在 startup 调用
> `migrate_legacy_codex_official_managed_binding`，但该函数与其钩子被 **`0455a92c`
> 删除**，v3.20.0 全树无此符号 → **不移植**。取代它的是「绑定/解绑保持 provider id
> 不变」（`update_keeps_official_provider_id_when_binding_and_unbinding`）与「允许多张
> 未绑定 official 卡」（`add_accepts_multiple_unbound_codex_official_cards`）。证据链见
> prd.md「裁定记录 · Q3」。

### 关键契约

**账号绑定持久化**：只存 `accountId`，**不存 OAuth secrets**。S4b 移交测试的首个用例即 `persists the selected managed account while stripping OAuth secrets`。

**reauth 阻断（fail-closed）**：`auth.ts` 新增 `reauth_required`（Codex：账号早于 id_token 持久化支持）。测试 `blocks saving a managed account that requires reauthentication` 与 `blocks the reauth-required default account when no account is selected` 钉住"需重新认证的账号不得保存"。
> 按 spec「Degradation Direction」分类：**auth/账号绑定是授权决策 → fail-closed**。降级会让一个无法认证的绑定被持久化，后续出站请求带着失效凭据。与上一子任务 citation dedup 的 fail-open 相反且不矛盾。

**unbound 选择的两个开关**：
- `allowUnboundSelection` —— 是否允许选"原生登录"（不绑定具体账号）
- `allowUnboundSelectionWithoutStatus` —— 无状态信息时是否仍允许 unbound
两者共同覆盖测试的 `keeps a category-less managed card Official when it is unbound` / `allows the fixed Official card to switch to a managed account` / `does not silently strip a legacy binding from the fixed card` 等用例。

**managed id_token 写入（新增安全面）**：`codex_oauth_auth.rs` +1053/−52 把所选账号 `id_token` 写入 official managed `auth.json`，使字段形状与原生浏览器登录一致。移植红线：写入必须走 fork 既有 atomic + 0600 路径；日志不得含 token；不得扩大凭据泄露面。

**takeover restore 不覆盖官方登录**：父主体 S2 `d2b070c9` 已建立该行为。本任务的账号绑定必须不破坏它 —— 这是强依赖的原因。

## 前端最小表面（Q1 裁定的落地形态）

S4b 移交测试的真实需求（已证实）：
- 测试 **mock 掉 `CodexOAuthSection`**，自带 `data-testid="allow-unbound-selection"` / `"allow-unbound-without-status"` 桩件 → 真实 `CodexOAuthSection` 与 `CodexFormFields` 完整版**均非必需**。
- 测试渲染 **`ProviderForm`**，覆盖 `appId="codex"`（原生）与 `appId="claude"` + `providerType: "codex_oauth"` 两路。

fork 现状：

| 位置 | 状态 |
|---|---|
| `ProviderForm` `selectedCodexAccountId`(415) / `selectedAccountIsUsable`(959) / `isCodexOauthPreset`(1836) / `onCodexAccountSelect`(1859) | **已有**（S4b `84d54e7d`） |
| `ClaudeFormFields` import + 渲染 `CodexOAuthSection`（34 / 81–84） | **已有** |
| `CodexFormFields` 接 codex-oauth props / 渲染 `CodexOAuthSection` | **缺** ← 主要缺口 |
| 4 个新 prop（`allowUnboundSelection` / `allowUnboundSelectionWithoutStatus` / `onSelectionConfirmed` / `onSelectionInvalidated`）双侧接线 | **缺** |
| `ProviderForm` `canSelectCodexNativeLogin` 等推导 | **缺** |
| `CodexOAuthSection.tsx` 本体 +410/−90（漂移仅 1 行） | **可直接落地** |

→ `CodexFormFields` 的 1137 行漂移中绝大部分是 fork 刻意裁掉的表面（ClaudeDesktop 相关与其他 UI），**不在本任务范围**。

## 无新命令 / 无 schema 迁移（两项已证实）

- `lib.rs` diff 中 `+\s+commands::` 计数 0；`commands/auth.rs` 无新 `#[tauri::command]`。
- `src/lib/api/web-commands.ts` 不在 54 文件内 → `check:web-routes` 必须保持 **292/280/0 不变**。
- `database/` 仅 `dao/providers.rs`（+100 查询列扩展），**`schema.rs` 未触及** → `SCHEMA_VERSION` 保持 **17**。
- `lib.rs` startup ~~新增 `migrate_legacy_codex_official_managed_binding`~~ —— **作废（W2 裁定 Q3）**：`0455a92c` 已删除该函数与钩子，v3.20.0 无此符号，本任务不移植。
  > 若**将来**确有 fork 侧数据迁移要在 startup 执行，落点**不是** `lib.rs::setup()`（上游形状，仅桌面执行 → 违反双运行时等价），而是 `bootstrap::run_post_db_bootstrap`（bootstrap.rs:170，桌面 `lib.rs:494` 与 `examples/server.rs:335` 共同调用，`examples/server.rs:1136` 已有顺序断言），失败仅 `log::warn!`，与该函数内既有各步一致。

## 丢弃项（carry-forward）

| 文件 | 改动 | 理由 |
|---|---|---|
| `forms/ClaudeDesktopProviderForm.tsx` | +48/−10 | fork 无 claude-desktop（S5 / pi 子任务既有裁定） |
| `tests/components/ClaudeDesktopProviderForm.test.tsx` | +122 | 同上 |
| `i18n/locales/zh-TW.json` | +32/−1 | fork 仅 en/ja/zh |

其余 51 文件中，`ClaudeFormFields` 等文件内的 claude-desktop hunk 同样丢弃，只取 managed-account hunk。

## 批次依赖与顺序

```
W0 纯调研（无生产改动） ── Q2 裁定
  对 services/proxy.rs / services/provider/mod.rs / codex_config.rs
  逐 hunk 判定归属：
    (a) 落在 fork 存在且语义一致的锚点 → 可移植
    (b) 依赖 fork 缺失的上游符号 → 需先补依赖或降级（记录依赖链）
    (c) 命中 fork 已裁掉表面（ClaudeDesktop / 延期栈）→ 丢弃
  产出：hunk 归属清单 + 依赖拓扑 + 建议批次切分
  → 据此补写 implement.md 的 W1..Wn，再开工

W1..Wn（W0 后确定）
```

W0 不改生产代码、不 commit 代码，只产出调研结论并由主会话写入 implement.md。

## 兼容性与回滚

- 每批独立 commit → 单批失败 `git reset --hard <上一批>`。
- **无 schema 迁移** → 无 DB 回滚面；~~`migrate_legacy_codex_official_managed_binding` 会改数据，须幂等~~ —— **作废（W2 裁定 Q3）**：本任务不引入任何 startup 数据迁移，因此无幂等性面。
- 无新命令 → 无 Web route parity 回滚面。
- 子任务在同分支、独立 PR；回滚不影响父主体与两个已归档子任务。

## 重要权衡

- **先调研 vs 直接分批**：W0 增加一轮开销，但避免"第二批撞上依赖倒置被迫重排"。前两个子任务能跳过这步是因为规划期已确认基线对齐；本次不成立。
- **按测试反推最小表面 vs 整体对齐 `CodexFormFields`**：见 PRD Q1。整体对齐会把 fork 刻意裁掉的 ~1000 行表面重新引入，回归面远超本任务。
- **`codex_oauth_auth.rs` +1053 的凭据面**：这是本任务最大 Rust 增量且直接处理 `id_token`。不能只看编译通过，须逐条核验写入路径的权限与日志脱敏，按 fork 既有 auth 边界口径。
