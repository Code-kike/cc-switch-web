# PRD（stub）— pi native coding agent + session usage

> 父任务：`08-18-sync-upstream-v3.20.0`。本子任务独立进入 planning 后细化 design/implement。

## 范围
- `84e75ad2` feat(pi): add native coding agent support (#6064) — 155 文件 / +18,765 行
- `40d747c0` feat(pi): add session usage statistics (#6463) — 17 文件 / 1,705 行（依赖 pi provider）

## 前置依赖
- 父任务 S2 安全基线落地（无强阻塞；pi 为纯新增 provider，与主体冲突面最小）。

## 约束（carry-forward）
- 双运行时：新命令注册 `web-commands.ts`，过 `check:web-routes`；web-only feature-gate。
- 安全边界、无认证部署、updater 禁用不退化。
- `.pi/`、`.pi-subagents/` 不得修改或提交。

## 验收（待细化）
- [ ] pi provider 完成 Web API / browser UI / headless runtime 适配。
- [ ] session usage statistics 在双运行时下正确计数。
- [ ] 全量门禁通过；与父主体无回归。
