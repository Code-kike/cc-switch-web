# PRD：同步 Product upstream v3.19.2 之后的更新到 Web-first fork（目标 v3.20.0）

## 状态

已确认（2026-08-18 grilling 完成，用户确认分阶段并与 carry-forward 裁定对齐）。

### Grilling 裁定结果

1. **目标边界**：单分支 `sync/upstream-v3.20.0` 直达稳定 tag `v3.20.0`；`product-upstream/main`
   与 `v3.20.0` 指向同一提交（`0b5da510`），本次无 post-tag 提交，边界干净。
2. **分阶段范围**（本次新增裁定）：父任务承担 v3.20.0 修复/小特性主体的直接移植
   （65 个提交，A/B/C/D/F/G/H + 适配 I）；三个全新大特性各开一个子任务独立
   规划/实现/归档：
   - 子任务 `feat-pi-native-agent`：`84e75ad2`（pi native coding agent，155 文件/+18.7k 行）
     + `40d747c0`（pi session usage statistics，1705 行；依赖 pi provider，随子任务落地）。
   - 子任务 `feat-codex-alpha-websearch`：`bdeaac75`（Codex Alpha Search + Claude hosted
     WebSearch，9 文件/+10k 行）。
   - 子任务 `feat-managed-oauth-accounts`：`a2e22f33`（managed OAuth account selection，
     54 文件/+11.6k 行）。
3. **Sponsor/preset/i18n churn**：全量原样移植（含 README/partner 纯文档提交），避免
   未来同步的永久漂移；预设对用户是惰性的。覆盖“默认排除候选”中的营销变更条目。
4. **双运行时策略**：沿用既有 S1–Sn 模式；web-only 代码必须 feature-gate；新命令在
   `web-commands.ts`（路由 SSOT）注册并过 `check:web-routes`。
5. **安全/数据完整性提交**：完整移植（`c9fe340b` sync/restore、`dfb2e523` backup SQL
   fidelity、`fd14f9c4` env-check hang、`d2b070c9` never clobber Codex login 等），保留
   fork 既有 Web hardening/no-auth/写入安全边界，按 v3.19.2 任务 `6b8f3643` 等价口径。
6. **DB 迁移**：上游迁移原样采纳，校验从上次基线严格连续；备份/同步事务批处理必须过
   SQL-restore 锁测试。
7. **定价种子**：移植时执行 FE-preset↔Rust-seed 一致性核对（防 `cost=$0`，注意
   `find_model_pricing_row` 有损清洗）。
8. **Windows 适配深度**：fork 既有 WiX 安装器（`src-tauri/wix/`）与 CLI 检测代码
   （`path-adapter.ts`/`settings.ts`/`adapter.ts`/`web-commands.ts`）→ H 组四提交
   **全量移植 + Web 适配**（`d4fefefc` FOUC、`de9af49a` CLI registry detect、
   `3c592d93` WiX Handlebars escape、`c39c9032` WSL atomic replace）。
9. **CI 适配**（本次新增裁定）：fork 有自己的 CI（`ci.yml`/`claude.yml`/`release.yml`/
   `stale.yml`）→ 仅适配有通用价值的 I 组提交到 fork `ci.yml`（`c98cc3a9` skip-checks、
   `36ed280d` i18n labeler glob，按 fork 实际 locales 路径校正）；排除桌面专属/纯文档
   （`ceef0a52` WSL2 backend tests、`bef46cd5` grokBuild exclusion rationale）。
10. **门禁**：每批全量本地门禁（`cargo fmt --check`、`pnpm format:check`、web
    `cargo check`、`check:web-routes`、`check:locales`、`test:unit`、`test:integration`、
    Rust `web_api::dual_runtime_parity::` `web_proxy_lifecycle::`）；已知环境 flake 不阻塞。
11. **部署**：本任务（含各子任务）止于 PR 合入 `main`；systemd 部署为合并后独立确认步骤。

## 目标

将 `farion1231/cc-switch`（Git remote：`product-upstream`）在本仓库上次同步基线
`v3.19.2 + 413c09e0` 之后的适用产品能力与修复，按 Web-first fork 的运行模型完成
选择性移植、Web 适配与验证，目标边界为稳定 tag `v3.20.0`。

> 领域语言：本任务使用 `Product upstream`、`Product upstream sync` 和
> `Web-first fork`；不把 `origin` 或 `Code-kike/cc-switch-web` 称为“上游”。

## 已确认事实

- 本仓库 `main` 当前版本 `3.19.2`；上次同步任务（`08-08-sync-upstream-latest`）已
  完成归档，移植到 `v3.19.2` tag + post-tag 提交 `413c09e0`（Codex catalog 覆盖修复）。
- `413c09e0` 是 `v3.20.0` 的祖先 —— 上次基线完全包含在 v3.20.0 中，无回退缺口。
- `product-upstream/main` HEAD = `0b5da510`（= `v3.20.0`）—— 本次无 post-tag 提交。
- 新区间 `413c09e0..v3.20.0` 共 **71 个非 merge 提交**。
- 直接 merge 不可行（上次直接 merge 产生 435 冲突文件）；沿用 selective port：
  逐批 cherry-pick + Web 适配，每批过全量门禁后单独 commit。
- 代码库自查结论：
  - fork **有** Windows CLI 检测代码 + WiX 安装器 → H 组需移植 + Web 适配。
  - fork **没有** pi / native coding agent provider → `84e75ad2` 是全新大特性。
  - fork 有自己的 CI → I 组需选择性适配。
  - fork 已落地 v3.19.2 安全上限（`6b8f3643`），本次新区间内的安全/数据完整性修复
    按等价口径移植，不重复已落地部分。

## 任务结构（父 + 3 子）

```
08-18-sync-upstream-v3.20.0 (父, planning → in_progress)
├── 直接工作：v3.20.0 主体修复/小特性（65 提交，8 批 S1–S8）
├── 子：feat-pi-native-agent        (84e75ad2 + 40d747c0)
├── 子：feat-codex-alpha-websearch  (bdeaac75)
└── 子：feat-managed-oauth-accounts (a2e22f33)
```

执行顺序（写在各子任务 implement.md）：
1. 父主体（S1–S8）先落，建立 v3.20.0 修复基线，含 auth 安全修复（`d2b070c9` 等）。
2. 子任务按冲突面从小到大、依赖先后：`feat-pi-native-agent`（纯新增 provider，最少冲突）
   → `feat-codex-alpha-websearch`（proxy 层，F 组已落地）→
   `feat-managed-oauth-accounts`（auth 层，依赖 `d2b070c9` 已在主体落地）。
3. 父最终集成 review + 统一合入 `main`。

## 父任务直接工作提交清单（65 个）

### S1 发布/文档 + sponsor/preset/i18n churn（裁定 #3 全量原样，~20）
A：`0b5da510` `18ca2da0` `af31a87b` `a7f073e9`
B：`0cd922c5` `52745efe` `4080a8e9` `af06356d` `9dcd3486` `e12fc623` `1435223b`
`c6247d13` `e163a671` `eb69e492` `5b8bf1fe` `c99550e0` `5f6072ce` `58d92e56`
`3711e1a0` `16cc0d7f`

### S2 安全/数据完整性（裁定 #5，优先，8）
`fd14f9c4`(env-check hang) `d2b070c9`(never clobber Codex login) `c8262476`(Kimi
thinking injection) `1f38c838`(zhipu CREDIT_LIMIT) `3f75bbdf`(StepFun effort)
`ccc86298`(routing animate) `c9fe340b`(sync/restore, 1012 行) `dfb2e523`(backup SQL
fidelity, 1302 行)

### S3 定价/usage（裁定 #7 一致性核对，5）
`bad9c151`(DeepSeek V4 + Gemini 3.7 Flash) `5602324b`(DeepSeek catalog mirror)
`7dc0a725`(Grok 4.5/4.6 cached) `3d126f45`(multi-year trend tooltip)
`46f19a15`(DeepSeek cache-hit tokens)
⚠ FE-preset↔Rust-seed 一致性核对（防 `cost=$0`，`find_model_pricing_row` 有损清洗）

### S4 codex/provider 功能（12）
`d1c550ba`(drop Goal toggle, -270) `6e424fd3`(1M context toggle) `0455a92c`(multiple
follow-login) `897ca892`(OAuth usage configurable) `a98829ba`(IME-safe fields)
`f62c854a`(cancel stale device login) `d01eab97`(OpenCode Zen reasoning)
`b109dcd3`(Grok Build Codex copy) `40cac1a6`(per-model reasoning levels, 637)
`f748f3ac`(grokbuild form align) `d9d4a660`(macOS IME) `6a7da87c`(grokbuild input tokens)

### S5 UI/refactor（14）
`7e5007d5` `580a4d7b` `ec842156` `c0050623` `5b77da2b` `619a592c` `95b95da6`
`076c2744`(dropdown consolidation) `7e152d75`(fuzzy search) `8673e9d8` `bc7f5f41`
`7de63227` `967daa1a`(skills SSOT check_updates) `390102a2`(DeepSeek contextWindow)

### S6 Windows（裁定 #8 全量适配，4）
`d4fefefc`(FOUC) `de9af49a`(CLI registry detect) `3c592d93`(WiX Handlebars escape)
`c39c9032`(WSL atomic replace fallback)

### S7 CI 适配（裁定 #9，2）
`c98cc3a9`(skip-checks) `36ed280d`(i18n labeler glob，按 fork 实际 locales 路径)
排除桌面专属：`ceef0a52`(WSL2 backend tests) `bef46cd5`(grokBuild exclusion 文档)

### S8 版本 + changelog + 全量门禁
`18ca2da0`/`0b5da510` 改写为 fork 短条目；`af31a87b` 反映实际移植范围。

## 子任务提交清单（4，各自独立规划）

- `feat-pi-native-agent`：`84e75ad2`（pi native coding agent，155 文件/+18.7k）+
  `40d747c0`（pi session usage statistics，1705 行）。
- `feat-codex-alpha-websearch`：`bdeaac75`（Codex Alpha Search + Claude hosted
  WebSearch，9 文件/+10k，含 fixture）。
- `feat-managed-oauth-accounts`：`a2e22f33`（managed OAuth account selection，
  54 文件/+11.6k）。依赖父主体 `d2b070c9`（never clobber Codex login）已落地。

## 验收标准

- [ ] Product upstream 目标边界（`v3.20.0`）由用户明确确认。
- [ ] 71 个提交中每个功能性提交都有可审计处置结果（移植/适配/排除/延期）和理由。
- [ ] 所有纳入能力完成 Web API / browser UI / headless runtime 适配，新命令在
      `web-commands.ts` 注册并过 `check:web-routes`。
- [ ] Web-first fork 的安全边界、无认证部署决定和 updater 禁用决定未退化。
- [ ] 前端、Rust、Web 路由、locale、集成测试、生产构建及真实 Web 服务冒烟通过。
- [ ] 数据迁移前有备份，失败可恢复；服务重启后关键流程可用。
- [ ] 版本与 fork 自有发布说明反映实际移植范围，不宣称未实现的上游能力。
- [ ] 三个子任务各自独立过门禁并归档；父任务做跨子任务集成 review 后统一合 main。

## 默认排除候选（已并入裁定）

- 桌面 updater、发布资产镜像、签名/打包 CI（R2 mirror 等）。
- CI 桌面专属：`ceef0a52`（WSL2 backend tests）、`bef46cd5`（grokBuild exclusion 文档）。
- Product upstream guide/release-note 原文；本 fork 只写与实际移植相符的说明。
- 工作树未跟踪目录 `.pi/`、`.pi-subagents/`（本任务及子任务不得修改或提交）。

## 暂不做

- 在 grilling 结束、PRD/design/implement 文档获批前修改产品代码。
- 修改或提交现有未跟踪目录 `.pi/`、`.pi-subagents/`。
- 子任务的细节设计（各自进入 own planning 后再写 design/implement）。
