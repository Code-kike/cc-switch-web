# Handoff: sync upstream v3.20.0（grilling 共识，2026-08-18）

来源：Claude 会话中的 /grill-with-docs，用户已确认全部采纳以下推荐方案。
用法：以此为 PRD 骨架创建任务 `sync upstream v3.20.0`，brainstorm 阶段只需补细节，不必重新决策。

## 已确认决策（Q1–Q9）

- **Q1 分阶段**：先合并已完成的 `sync/upstream-v3.19.2` 分支到 main（复刻 v3.18.0 的 PR #33 模式），合并前重跑完整门禁（该分支 8 月 8 日后未再验证）。然后从 main 切 `sync/upstream-v3.20.0`。
- **Q2 范围**：全量移植 `v3.19.2..v3.20.0`（72 commits / 291 文件 / +54k−6.7k），锚定 tag v3.20.0。桌面专属（Windows FOUC 的 tauri.conf 部分、Windows-only CLI 探测路径）归类 Release-operation delta 跳过；共享代码照常移植。
- **Q3 Pi application**（最大切片，42 文件）：完整双运行时对等——`commands/pi.rs`、`pi_config`、session_manager provider、usage 导入、前端表单/提示词/目录，以及 web-commands.ts 路由对等（`check:web-routes` 兜底）。集成测试必须隔离 Pi 环境路径：防真实 `~/.pi` 泄漏（同 opencode.db 前科），也防仓库内开发者自用 `.pi/` 目录被扫描。
- **Q4 Managed OAuth account (#3879) + follow-login (#6535)**：完整移植 + web 对等（沿用 v3.19.2 S5 凭证对等模式）。OAuth 明文存储遗留问题不在本次扩大范围。
- **Q5 切片**：S1 DB/DAO schema；S2 presets/pricing（Qianfan、火山拆分、XycAi、RunAPI 域名迁移、DeepSeek/Gemini 定价）；S3 proxy（WebSearch/媒体桥、StepFun effort）；S4 Codex/OAuth（per-model reasoning、managed account、device-login 取消）；S5 Pi application；S6 UI/i18n/表单（IME 加固、usage tooltip）；S7 Web 对等+安全复核；S8 文档/测试/收尾。
- **Q6 门禁**：每批次跑完整套件——双运行时 `cargo fmt/clippy/test`（含 `web_api:: dual_runtime_parity:: web_proxy_lifecycle::`）、`pnpm format:check/lint/type/test:unit`、`check:web-routes`、`check:locales`。4 个已知环境性 integration flake 不阻塞（见 memory/gate-suite）。cargo 在 `~/.cargo/bin`，需 `source "$HOME/.cargo/env"`。
- **Q7 `.pi/` 与 `.pi-subagents/`**：开发者自用 Pi 工具目录，与产品的 Pi application 无关；加入 `.gitignore` 防误提交/误扫描。
- **Q8 版本锚点**：tag v3.20.0（不追 upstream HEAD）。
- **Q9 部署**：v3.20.0 合并后按 v3.18.0 systemd runbook 部署（含回滚目录），是任务完成后的独立收尾。

## 词汇表

CONTEXT.md 已新增：Pi application / Workspace Pi tooling / Managed OAuth account / Follow-login provider。术语冲突注意：仓库里 `.pi/` ≠ 产品管理的 Pi application 配置。

无 ADR：各决策均为既有同步惯例延续。
