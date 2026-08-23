# Implement — sync upstream v3.20.0（父任务，主体工作）

## 前置确认

- [ ] 确认当前分支 `sync/upstream-v3.20.0` 已从 `main`（含 v3.19.2 同步成果）切出。
- [ ] 确认 `product-upstream` remote 已 fetch，`v3.20.0` tag 本地可达。
- [ ] 确认工作树未跟踪目录 `.pi/`、`.pi-subagents/` 不在本任务提交范围。

## 执行批次（ordered checklist）

每批：cherry-pick → Web 适配 → 全量门禁 → 单独 commit。
门禁命令：
```bash
source ~/.cargo/env
cargo fmt --check
pnpm format:check
pnpm exec cargo check --manifest-path src-tauri/Cargo.toml   # web cargo check
pnpm check:web-routes
pnpm check:locales
pnpm test:unit
pnpm test:integration        # 已知环境 flake 不阻塞
cargo test --manifest-path src-tauri/Cargo.toml web_api::dual_runtime_parity::
cargo test --manifest-path src-tauri/Cargo.toml web_proxy_lifecycle::
```

### S1 — 发布/文档 + sponsor/preset/i18n churn（~20 提交）
- [ ] cherry-pick A 组：`0b5da510` `18ca2da0` `af31a87b` `a7f073e9`（改写为 fork 短条目）
- [ ] cherry-pick B 组：`0cd922c5` `52745efe` `4080a8e9` `af06356d` `9dcd3486` `e12fc623`
      `1435223b` `c6247d13` `e163a671` `eb69e492` `5b8bf1fe` `c99550e0` `5f6072ce`
      `58d92e56` `3711e1a0` `16cc0d7f`
- [ ] Web 适配：preset 增量同步到 fork provider（ctok/lionccapi/lemondata 等自有
      provider），i18n 走 fork locales 路径
- [ ] 门禁全绿 → commit

### S2 — 安全/数据完整性（优先，8 提交）
- [ ] `fd14f9c4` env-check preflight hang
- [ ] `d2b070c9` never clobber Codex ChatGPT login ⚠ 子任务 feat-managed-oauth 前置
- [ ] `c8262476` Kimi thinking injection stop
- [ ] `1f38c838` zhipu CREDIT_LIMIT quota
- [ ] `3f75bbdf` StepFun effort inference
- [ ] `ccc86298` animate routing activation
- [ ] `c9fe340b` sync/restore consistency（1012 行）⚠ SQL-restore 锁测试
- [ ] `dfb2e523` backup SQL fidelity（1302 行）⚠ SQL-restore 锁测试
- [ ] 保留 fork 既有 canonical-schema allow-list、backup/replace 锁边界、2s deadline
- [ ] DB 迁移连续性校验（从 v3.19.2 基线不跳号）
- [ ] 门禁全绿 → commit

### S3 — 定价/usage（5 提交）
- [ ] `bad9c151` DeepSeek V4 peak + Gemini 3.7 Flash
- [ ] `5602324b` DeepSeek catalog mirror extraction
- [ ] `7dc0a725` Grok 4.5 cached + Grok 4.6 + DeepSeek alias
- [ ] `3d126f45` multi-year trend tooltip
- [ ] `46f19a15` DeepSeek cache-hit tokens
- [ ] ⚠ FE-preset ↔ Rust-seed 一致性核对（防 `cost=$0`，注意 `find_model_pricing_row` 有损清洗）
- [ ] 门禁全绿 → commit

### S4 — codex/provider 功能（12 提交）
- [x] `d1c550ba` drop Goal mode toggle（-270 行）→ S4a `5a5874a5`
- [ ] `6e424fd3` restore 1M context toggle
- [~] `0455a92c` multiple follow-login providers（829 行）→ **拆分**：
      前端独立增量（providerCapabilities 3 符号 + ProviderCard identity/useManagedAuth +
      useProviderActions supportsOfficialProxyTakeover + providerConfigUtils +
      useManagedAuth enabled + presetEntries）已落 S4b `84d54e7d`；**Rust managed-codex 事务**
      + `ProviderForm.codexManagedAccount.test.tsx`（8 用例）移交 `feat-managed-oauth-accounts`
      子任务（依赖 `a2e22f33` 的 `preflight_managed_codex_live` 等辅助函数，fork 全缺）。
- [~] `897ca892` OAuth usage queries configurable → 前端（CodexOauthQuotaFooter/subscription.ts/
      ProviderCard codexAccount identity）已落 S4b `84d54e7d`；Rust tray.rs 待 S4c。
- [x] `a98829ba` IME-safe provider fields → S4a `5a5874a5`
- [ ] `f62c854a` cancel stale device login（reverted — 需完整 test-helpers 移植，待 S4c）
- [ ] `d01eab97` OpenCode Zen reasoning effort
- [ ] `b109dcd3` Grok Build Codex copy
- [ ] `40cac1a6` per-model reasoning levels（637 行）⚠ 依赖延期 Codex Chat reasoning 栈
- [ ] `f748f3ac` grokbuild form align
- [ ] `d9d4a660` macOS IME corruption
- [ ] `6a7da87c` grokbuild input token details
- [ ] 新命令注册到 `web-commands.ts`，过 `check:web-routes`
- [ ] 门禁全绿 → commit

### S5 — UI/refactor（14 提交）
- [ ] `7e5007d5` `580a4d7b` `ec842156` `c0050623` `5b77da2b` `619a592c` `95b95da6`
      `076c2744` `7e152d75` `8673e9d8` `bc7f5f41` `7de63227` `967daa1a` `390102a2`
- [ ] UI 适配到 fork 组件树，保留 fork 既有样式约定
- [ ] 门禁全绿 → commit

### S6 — Windows 全量适配（4 提交）
- [ ] `d4fefefc` startup FOUC（前端渲染入口）
- [ ] `de9af49a` Windows CLI registry detect → `path-adapter.ts`/`settings.ts`/`web-commands.ts`
- [ ] `3c592d93` WiX Handlebars backslash escape → `src-tauri/wix/per-user-main.wxs`
- [ ] `c39c9032` WSL atomic replace fallback（保留 fork 2s deadline + heap/stack 上限）
- [ ] 门禁全绿 → commit

### S7 — CI 适配（2 提交，排除 2 桌面专属）
- [ ] `c98cc3a9` skip-checks → fork `.github/workflows/ci.yml`（按 fork frontend/backend 分区）
- [ ] `36ed280d` i18n labeler glob → **按 fork 实际 locales 路径校正**（`src/i18n/locales/`）
- [ ] 排除 `ceef0a52`（WSL2 backend tests）、`bef46cd5`（grokBuild exclusion 文档）
- [ ] 门禁全绿 → commit

### S8 — 版本 + changelog + 全量门禁
- [ ] `18ca2da0`/`0b5da510` 版本号 → `3.20.0`（package.json + Cargo.toml）
- [ ] `af31a87b` changelog 改写为 fork 短条目，反映实际移植范围（含 3 个子任务结果）
- [ ] 全量门禁最终跑一遍
- [ ] 真实 Web 服务冒烟
- [ ] commit

## 子任务编排（父职责，非父直接实现）

- [ ] 确认子任务 `feat-pi-native-agent` 规划完成并启动（S2 落地后无前置阻塞）
- [ ] 确认子任务 `feat-codex-alpha-websearch` 规划完成（S2/F 组落地后启动）
- [ ] 确认子任务 `feat-managed-oauth-accounts` 规划完成（**依赖 S2 `d2b070c9` 落地**）
- [ ] 各子任务归档后，父做跨子任务集成 review
- [ ] 统一版本/changelog，合入 `main`

## 验证命令汇总

见每批门禁块。关键额外检查：
- DB 迁移连续性：`sqlite3` schema 版本号查询，对比 v3.19.2 基线
- 定价一致性：FE-preset 与 Rust-seed 对照（防 `cost=$0`）
- Web API parity：`check:web-routes` + `web_api::dual_runtime_parity::`

## 风险点与回滚

- **单批失败**：`git reset --hard <上一批 commit>` 回滚，不影响已落地批次。
- **DB 迁移失败**：从 S2 前备份恢复；fork 既有备份/replace 锁流程。
- **子任务阻塞**：子任务独立分支，不阻塞父主体合 main（可先合主体，子任务后续跟进）。
- **CI glob 路径错误**：`36ed280d` 必须按 fork locales 路径校正，照搬上游 glob 会失效。

## review gates

- 每批 commit 前：全量门禁全绿（已知 flake 除外）。
- S2 后：DB 迁移连续性 + SQL-restore 锁测试专项确认。
- S3 后：定价一致性核对专项确认。
- S8 后：真实 Web 服务冒烟 + 子任务集成 review 后才合 main。
