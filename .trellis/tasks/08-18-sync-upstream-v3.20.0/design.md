# Design — sync upstream v3.20.0（父任务，主体工作）

## 架构与边界

本任务在 Web-first fork 上完成 Product upstream `413c09e0..v3.20.0` 的选择性移植。
父任务只做**主体直接工作**（65 提交，S1–S8）；三个全新大特性由子任务承担
（见各子任务 design.md）。父任务持有跨子任务集成 review 与统一合 main 职责。

### 移植方法

selective port（cherry-pick + Web 适配），**不直接 merge**。原因：上次 v3.19.2
同步直接 merge 产生 435 冲突文件，不可行。逐批 cherry-pick，每批过全量门禁后
单独 commit，保证可回滚。

### 双运行时边界（carry-forward）

- Tauri command 与 Web API 路由保持等价；新命令在 `src/lib/api/web-commands.ts`
  （路由 SSOT）注册并过 `check:web-routes`。
- web-only 代码必须 feature-gate；desktop-only 代码（updater/release 链）不移植。
- 无认证部署姿态保持；数据库/配置写入安全边界不可退化。

## 数据流与契约

### DB 迁移（S2 `c9fe340b`/`dfb2e523`）
- 上游迁移原样采纳，校验从上次基线（v3.19.2 已落地的 schema 版本）严格连续，
  不跳号、不回退。
- 备份/同步事务批处理：`dfb2e523` 保留 SQL fidelity 与 recovery safety；`c9fe340b`
  保留 sync/restore consistency。两者必须过 SQL-restore 锁测试。
- fork 既有 canonical-schema allow-list、规范化复制、backup/replace 锁边界必须保留
  （上次 S2 已建立的等价保护不得退化）。

### 定价种子（S3）
- FE-preset ↔ Rust-seed 一致性核对：移植后执行，防止 `cost=$0`。
- 注意 `find_model_pricing_row` 有损清洗 —— 移植 `bad9c151`/`7dc0a725` 时核对
  DeepSeek V4 / Gemini 3.7 Flash / Grok 4.5/4.6 的 Rust seed 与 FE preset 一致。

### Windows 适配（S6）
- fork 既有路径：`src-tauri/wix/per-user-main.wxs`、`src/lib/api/path-adapter.ts`、
  `src/lib/api/settings.ts`、`src/lib/api/adapter.ts`、`src/lib/api/web-commands.ts`。
- `de9af49a` CLI registry detect：适配到 fork 的 path-adapter/settings，Web 侧通过
  web-commands 路由暴露等价检测。
- `3c592d93` WiX Handlebars escape：直接落到 `per-user-main.wxs`。
- `c39c9032` WSL atomic replace：落到 fork 文件写入路径，保留 fork 既有 2 秒 deadline
  与 heap/stack 上限（上次 S2 `6b8f3643` 已建立的边界）。
- `d4fefefc` FOUC：前端启动闪屏修复，适配到 fork 渲染入口。

### CI 适配（S7）
- `c98cc3a9` skip-checks：适配到 fork `.github/workflows/ci.yml`，按 fork 实际
  frontend/backend 分区（fork 3 jobs：frontend/backend/web-server，上游 3 jobs：
  frontend/backend/WSL2）。新增 `changes` job（dorny/paths-filter，SHA-pinned）
  inline 路径过滤器，gate 三个 job；push to main 无条件跑保 cache。路径过滤器按
  fork 校正：fork `index.html` 在 `src/index.html`（vite root=`src`），被 `src/**`
  覆盖，无根级裸 glob。
- `36ed280d` i18n labeler glob：**整提交跳过（proven inapplicable，用户裁定
  option A）**：fork `.github/` 下无 `labeler.yml`（配置）也无 `workflows/labeler.yml`
  （工作流），上游 glob 修复（`src/locales/**`→`src/i18n/locales/**`）无目标文件。
  `c98cc3a9` 的 `changes` job inline 了自己的过滤器（含 `src/**` 覆盖
  `src/i18n/locales/**`），不依赖 labeler.yml，已等价覆盖 i18n 路径识别。引入整套
  labeler CI 表面是独立新决策，不应藏在"sync glob 修复"里夹带；若未来想要 PR
  自动打标签，应作为独立 CI 特性任务评估。

## 批次依赖与顺序

```
S1 (docs/preset/i18n)     ──┐
S2 (security/data)        ──┼── S2 优先，建立安全基线
S3 (pricing/usage)        ──┤
S4 (codex/provider)       ──┤── 依赖 S2 的 auth 安全修复
S5 (UI/refactor)          ──┤
S6 (Windows)              ──┤
S7 (CI adapt)             ──┤
S8 (version/changelog/full gate) ── 最后，版本与发布说明收口
```

S2 先行：`d2b070c9`（never clobber Codex login）是子任务
`feat-managed-oauth-accounts` 的前置依赖，必须在主体 S2 落地后子任务才能启动。

## 子任务集成（父职责）

父任务在 S8 完成后、合 main 前，做跨子任务集成 review：
1. 三个子任务各自独立过门禁并归档。
2. 父 review 各子任务的 Web API parity、安全边界、与主体无冲突回归。
3. 统一版本号、changelog，合入 `main`。

## 兼容性与回滚

- 每批独立 commit → 单批失败可 `git reset` 回滚到上一批，不影响已落地批次。
- DB 迁移前备份（fork 既有备份流程）；失败可从备份恢复。
- 子任务回滚不影响父主体（独立分支/独立 PR）。

## 重要权衡

- **分阶段 vs 全量**：分阶段增加 3 个子任务的 planning/门禁开销，但使每个大特性
  可独立 review/回滚，避免单 PR 4 万行 review 压力。已确认分阶段。
- **CI 适配 vs 排除**：fork 有自己 CI，但 `c98cc3a9`/`36ed280d` 有通用价值 → 选择性
  适配而非全排除，避免 fork CI 长期偏离上游有用改进。
- **Windows 全量 vs 仅 CLI**：fork 既有 WiX + CLI 检测，全量适配避免 fork 的 Windows
  支持长期落后于上游修复。
