# PRD（stub）— managed OAuth account selection for providers

> 父任务：`08-18-sync-upstream-v3.20.0`。本子任务独立进入 planning 后细化 design/implement。

## 范围
- `a2e22f33` Add managed OAuth account selection for providers (#3879) —
  54 文件 / +11,601 行。
- **从父任务 S4b 移交（2026-08-18）**：`0455a92c` 的 Rust managed-codex 事务部分
  与 `ProviderForm.codexManagedAccount.test.tsx`（8 个用例）。证据链：
  `0455a92c` 的 mod.rs hunk 引用 `preflight_managed_codex_live`/
  `managed_codex_oauth_account_id`/`managed_codex_add_transaction_error`/
  `write_preflighted_or_current_live`，这些函数在 `413c09e0..v3.20.0` 区间仅由
  `a2e22f33` 引入（`git log -S` 证实），fork 全缺，无法独立移植。
  - Rust managed-codex：`0455a92c` 的 codex mod.rs `add`/`update` 事务。
  - `tests/components/ProviderForm.codexManagedAccount.test.tsx`（521 行，8 用例）：
    断言 CodexFormFields 的 managed-account 选择 UI（select-managed-account 按钮、
    OAuth secrets 剔除、reauth-required 阻断等）。fork 的 CodexFormFields 是 294 行
    精简版（无 CodexOAuthSection 渲染块），完整版（上游 1394 行，含
    isCodexOauthPreset/selectedCodexAccountId/onCodexAccountSelect 接线 +
    CodexOAuthSection render block）随本子任务落地。
  - **测试文件取回路径**：fork 适配版已暂存在父任务历史 commit `7265596a`（已被
    `84d54e7d` 取代），可用 `git show 7265596a:tests/components/ProviderForm.codexManagedAccount.test.tsx`
    取回（已含 fork AppId/mock 适配，子任务只需补齐被断言的 CodexFormFields UI）。

## 前置依赖
- **强依赖**：父任务 S2 `d2b070c9`（never clobber Codex official ChatGPT login on
  takeover restore）必须先落地，否则 OAuth 账号选择会在 takeover restore 时覆盖官方登录。

## 约束（carry-forward）
- OAuth 账号存储/选择不得退化 fork 既有 auth 安全边界与无认证部署姿态。
- managed account 选择 UI 在双运行时等价；新命令注册 `web-commands.ts`，过 `check:web-routes`。
- 不得修改或提交 `.pi/`、`.pi-subagents/`。

## 验收（待细化）
- [ ] 多 OAuth 账号选择在 Web API / browser UI / headless runtime 三态下正确。
- [ ] takeover restore 不覆盖官方 ChatGPT 登录（回归 `d2b070c9` 行为）。
- [ ] `ProviderForm.codexManagedAccount.test.tsx` 8 用例全绿（取回自 `7265596a`）。
- [ ] `0455a92c` Rust managed-codex 事务移植后，父任务 S4b 移交的 8 个测试用例无回归。
- [ ] 全量门禁通过；与父主体无回归。
