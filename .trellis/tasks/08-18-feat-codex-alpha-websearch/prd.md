# PRD（stub）— Codex Alpha Search + Claude hosted WebSearch

> 父任务：`08-18-sync-upstream-v3.20.0`。本子任务独立进入 planning 后细化 design/implement。

## 范围
- `bdeaac75` fix(proxy): support Codex Alpha Search and Claude hosted WebSearch (#5681) —
  9 文件 / +10,041 行（含 fixture）。

## 前置依赖
- 父任务 S2/F 组 proxy 基线落地后启动（`d2b070c9` never clobber login 等已就位）。

## 约束（carry-forward）
- proxy raw/decompressed body 保留 fork 既有 128 MiB cap、2s deadline、heap/stack 上限
  （上次 S2 `6b8f3643` 已建立的边界不退化）。
- 双运行时：新命令注册 `web-commands.ts`，过 `check:web-routes`。
- WebSearch fixture 不得引入 SSRF / 无认证泄露。
- `.pi/`、`.pi-subagents/` 不得修改或提交。

## 验收（待细化）
- [ ] Codex Alpha Search 与 Claude hosted WebSearch 在双运行时下正确路由。
- [ ] proxy body cap/deadline 未退化。
- [ ] 全量门禁通过；与父主体无回归。
