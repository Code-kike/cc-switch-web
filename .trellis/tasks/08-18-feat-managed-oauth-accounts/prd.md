# PRD（stub）— managed OAuth account selection for providers

> 父任务：`08-18-sync-upstream-v3.20.0`。本子任务独立进入 planning 后细化 design/implement。

## 范围
- `a2e22f33` Add managed OAuth account selection for providers (#3879) —
  54 文件 / +11,601 行。

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
- [ ] 全量门禁通过；与父主体无回归。
