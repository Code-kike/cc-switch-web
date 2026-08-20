# Batch plan — sync v3.19.2+413c09e0..v3.20.0

方式:selective port(直接 merge 不可行,沿用 v3.19.2 口径)。逐批 cherry-pick + Web 适配,每批过全量门禁后单独 commit。

## 每批门禁
source ~/.cargo/env; (cd src-tauri && cargo fmt --all -- --check); pnpm format:check; pnpm exec cargo check --manifest-path src-tauri/Cargo.toml; pnpm check:web-routes; pnpm check:locales; pnpm test:unit; pnpm test:integration(已知环境 flake 不阻塞); cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server (web_api::/dual_runtime_parity::/web_proxy_lifecycle::)

## S1 结果(2026-08-19,47e977ae)

26 ported / 0 skipped。A 组 4 + B 组 16 = 20 提交全部落地(部分提交合并到同一文件改动,最终 26 个文件)。

### 提交映射与处置
- A:`0b5da510` `18ca2da0` `af31a87b` → 版本 3.20.0(package.json + Cargo.toml + tauri.conf.json)、CHANGELOG、release-notes/v3.20.0-{en,ja,zh}.md,改写为 fork 短条目格式,反映实际移植范围(含 3 个子任务待落地)。
- A:`a7f073e9` → FennoAI partner offer 文档($1.99 trial + $50 credits)。
- B:`0cd922c5` PPIO sponsor + `3711e1a0` PPIO provider support(374 行,+ppioProviderPresets.test.ts)。
- B:`52745efe` OpenCode Go direct Anthropic access regression test。
- B:`4080a8e9` Baidu Qianfan Token Plan preset(six apps,含 OpenClaw/Claude/OpenCode/Hermes)。
- B:`af06356d` `9dcd3486` `e12fc623` Kimi/Qianfan/BytePlus thinking switches + dialect correction。
- B:`1435223b` replace unavailable SiliconFlow/ModelScope catalog models。
- B:`c6247d13` `e163a671` Codex reasoning levels pre-fill(remaining + vendor-documented)。
- B:`eb69e492` XycAi partner preset(seven apps,+xycai-icon.png + icons/extracted index/metadata)。
- B:`5b8bf1fe` Volcengine split into Agent Plan + Coding Plan。
- B:`c99550e0` RunAPI domain migrate to runapi.host(runapi.co fallback)。
- B:`5f6072ce` remove partner star badge from main panel ProviderCard。
- B:`58d92e56` JieKou AI presets(+jiekouProviderPresets.test.ts)。
- B:`16cc0d7f` OpenCode Go preset route directly to /messages。

### Web 适配(grilling #7 parity gate)
- **Qianfan Token Plan OpenClaw 模型 id 限定为 `qianfan-tokenplan/deepseek-v4-pro`**:百度 Token Plan 是转售档位,cost 0.0025/0.01 是百度商业价,与 DeepSeek 裸种子(v3.19.2 起由 `bad9c151` 录高峰档 1.32/3.96,属 S3 未落地)不等。裸 id `deepseek-v4-pro` 会触发 `ModelsDevPricing.web-server` 的 FE-preset↔Rust-seed 一致性门禁(grilling #7,防 cost=$0)。按 fork 约定用 provider-scoped id 避开裸种子碰撞;实际 API 模型名仍由 `suggestedDefaults.model.primary` 后缀 `deepseek-v4-pro` 经 `rebaseOpenClawModelRef` 传入百度端点(已确认 `rebaseOpenClawModelRef` 保留 `/` 后缀语义,`modelsDevPricing.ts:359` `lastIndexOf("/")` 同款口径)。
- `tests/config/qianfanTokenPlanPresets.test.ts` 同步锁定 scoped id + bare name + cost 0.0025/0.01,保留官方 OpenClaw 接入页(2026-07-22 版)原样口径(窗口 98304 / maxTokens 65536)。
- **根因澄清**(任务契约守护裁定):早先"把 S3 `bad9c151` 的 DeepSeek V4 种子提前到 S1"的判断是误诊——`bad9c151` 裸种子 deepseek-v4-pro=1.32/3.96 ≠ 百度转售价 0.0025/0.01,挪种子后 parity 仍失败。真正根因是裸 id 与转售异价冲突,非种子。种子仍留 S3,grilling #7 关卡不退化。
- 模型升级/新预设同步应用到 fork 自有 provider(ctok/lionccapi/lemondata 等),保留 fork-owned 行。
- CHANGELOG 用 fork 短条目格式,非上游 prose。
- 未恢复 zh-TW locale(fork 仅 en/ja/zh)。
- 桌面专属 updater/release/installer 路径未恢复;Web-only 行为经 `web-commands.ts` + Axum route 对齐。

### 会话恢复说明
S1 由先前会话的子代理完成 cherry-pick + `git add` 并跑过 `pnpm test:unit`(969 通过)后在中途停止(会话重置导致 task.json status 回退到 planning、S1 结果未记录、未 commit)。本次会话:重新激活任务(set-branch + task.py start → in_progress)、核对 staged 工作完整、定位并修复唯一真实回归(Qianfan parity)、跑完全量门禁、commit、写本结果节。

### 门禁证据
- `(cd src-tauri && cargo fmt --all -- --check)`:通过。
- `pnpm format:check`:通过(Prettier)。
- `pnpm exec cargo check --manifest-path src-tauri/Cargo.toml`:通过。
- `pnpm check:web-routes`:282 commands / missing 0 / methodMismatch 0 / parityFallback 0。
- `pnpm check:locales`:2507 keys,en/ja/zh 严格 parity。
- `pnpm test:unit`:**969 passed**(161 files),含 `openclawPresetPricing` 1、`qianfanTokenPlanPresets` 5、`providerPresetOrder`、`ppioProviderPresets`、`jiekouProviderPresets`。
- `pnpm test:integration`:`ModelsDevPricing.web-server` parity **3/3 通过**(修复前失败,修复后绿)。仅剩 5 个失败=4 个 PRD 已知 baseline flake(ProviderList Claude official-seed empty-state 1、SkillsPage skills.sh default-repo/fixture 3)+ 1 个 **pre-existing 环境超时** `AuthCenterPanel` OAuth(device-code polling 5s 预算;已在 base 分支 `sync/upstream-v3.19.2` 隔离复跑确认同样失败,非 S1 引入,属环境 flake)。
- Rust web-server tests:`cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server` → **1939 passed / 0 failed / 5 ignored**(含 web_api::、dual_runtime_parity::、web_proxy_lifecycle::)。

### 残余风险与 carry-forward
- DeepSeek V4 高峰档种子(`bad9c151`,1.32/3.96/0.044 等)仍留 S3;S3 落地时按 grilling #7 再次核对 FE-preset↔Rust-seed 一致性(注意 `find_model_pricing_row` 有损清洗)。S1 的 Qianfan scoped id 不与裸种子碰撞,故 S3 种子落地不会回退 S1。
- `AuthCenterPanel` OAuth 5s 超时是 pre-existing 环境问题,记录为非阻塞 flake;若 S2/S4 触及 auth 再复现,需单独评估是否放宽 testTimeout(产品端 polling 语义不动)。
- 下一批:S2 安全/数据完整性(8 提交,优先,含 `d2b070c9` never clobber Codex login = 子任务 feat-managed-oauth-accounts 前置)。
