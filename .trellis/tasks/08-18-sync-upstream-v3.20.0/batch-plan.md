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

## S2 结果(2026-08-20)

7 ported / 1 skipped(proven inapplicable)。7 个落地提交中有 2 个是部分移植(fork 未全量落地上游的桌面专属 surface),1 个经证实为整提交不适用。

### 提交映射与处置

- `fd14f9c4`(env-check hang)→ 部分移植。fork 把 tool lifecycle/version 扫描迁到 `src-tauri/src/services/tool_version.rs`,**未恢复**上游 `enumerate_tool_installations`/`run_probe_version_command`/`INSTALL_PROBE_TIMEOUT`/preflight 冲突检测 UI(fork 早已按桌面专属剔除该 surface)。仅落地适用的两点根因修复:
  1. `isolate_child_process_group`:`process_group(0)` → `setsid()`(pre_exec),脱离控制终端,交互式 `-lic` shell 不再因 SIGTTIN/SIGTTOU 自停 → `wait()` 永等。
  2. 非窗口 `resolve_path_default`:`.stdin(Stdio::null())`,消除继承终端/管道时交互 rc 读操作永久阻塞。
  两点同时作用于 fork 既有 `resolve_path_default`(command -v 经 `-lic`)与 `run_tool_command`(直执工具)两个 spawn 点。补两个回归测试:`isolated_child_process_group_captures_healthy_tool_output`(健康路径)与 `isolated_hung_child_is_killed_on_deadline`(超时击杀路径,锡定 setsid 改造语义:kill(-pid) 仍命中)。
- `d2b070c9`(never clobber Codex ChatGPT login)→ 已移植。复用 fork 既有 `Self::codex_auth_has_proxy_placeholder`(`services/proxy.rs:3071`,`impl ProxyService` 关联函数,**未重复定义**)+ `crate::codex_config::codex_auth_has_credential_login_material` + `prepare_codex_provider_live_config`,新增唯一方法 `preserve_codex_oauth_login_on_restore`。两个 restore 入口(inner 简单路径 + with_fallback)都接入保护:live 持真实 OAuth 登录凭据(非占位符)时恒胜,备份第三方 API key 降级进 config.toml `experimental_bearer_token`,备份 auth 槽摘除 → `write_codex_live_verbatim` 只写 config.toml。⚠ 子任务 `feat-managed-oauth-accounts` 前置依赖已满足。6 个 OAuth 回归测试(api_key_only / no_oauth_live_verbatim / prefers_live_tokens / metadata_residue_backup / metadata_residue_live / simple_path_keeps_login)全部落地。
- `c8262476`(Kimi thinking injection)→ 已移植。从 `REASONING_VENDOR_HINTS` 撤出 `moonshot`/`kimi`(仅留 deepseek/mimo/xiaomimimo);两个 Kimi gating 测试翻转为负回归(`_skips_reasoning_content_for_kimi_provider`、`_kimi_anthropic_tool_history_not_modified`);transform.rs 注释更新为 DeepSeek/MiMo。
- `1f38c838`(zhipu CREDIT_LIMIT)→ 已移植。`parse_zhipu_token_tiers` 同时接受 `TOKENS_LIMIT` 与 `CREDIT_LIMIT`(大小写不敏感),国际端点与旧套餐不退化。
- `3f75bbdf`(StepFun effort step-3.7-flash)→ **整提交跳过(proven inapplicable)**。`git show v3.19.2:...codex.rs` 证明 `infer_codex_chat_reasoning_config`/`CodexChatReasoningConfig` 推断表是上游既存基线(fork 从未移植该 codex.rs Chat reasoning 推断层,`handlers.rs:1144/1682` 明确记录 "Codex Chat 路由栈 fork 暂未移植")。本提交改的是该延期表里的 StepFun 分支——表不存在 → 修改目标不存在。前端 `codexProviderPresets.ts` 无 StepFun catalog、无 `reasoningLevels` 字段;fork 的 StepFun 在 `openclawProviderPresets.ts` 作 OpenClaw provider(`stepfun/step-3.5-flash-2603` scoped id,无 3.7-flash)。恢复该推断分支需先恢复整套 ~1.1k LOC 推断表 + `CodexChatReasoningConfig` 类型 + provider.rs 基础,越过 S2 边界。**不延期到 S3–S8**(S3 是定价种子,不涉及 codex reasoning 推断层);属 fork 既有延期项,若未来单独恢复 Codex Chat 路由栈时再统一移植。
- `ccc86298`(animate routing activation)→ 已移植 + Web 适配。新增 `RoutingActivationBrand.tsx`(framer-motion 粒子爆发 + reduced-motion gate),`App.tsx` 品牌区替换为该组件,`useProxyStatus` 暴露 `isInitialStatusPending` 并据此 disable `ProxyToggle` 直到初始状态解析。⚠ **保留 fork 品牌链接 URL** `https://github.com/farion1231/cc-switch`(Web-first fork 不指向上游产品营销站 `ccswitch.io`);`RoutingActivationBrand.tsx` 的 `<motion.a href>` 改为 fork URL。补 `ProxyToggle.test.tsx`(初始状态 pending 时 disabled)与 `RoutingActivationBrand.test.tsx`(粒子爆发 / 初始 active 不播放 / 切换 app 清除粒子)。
- `c9fe340b`(sync/restore consistency,1012 行)→ 已移植 + Web 适配,保留 fork canonical-schema 第二边界。新增 `services/sync_protocol.rs`(`sync_mutex` + `run_with_sync_lock` 全局同步锁)、`commands/import_export.rs::run_with_database_restore_lock`(restore 串行)、`services/skill.rs`(`skill_state_lock`/`read_guard`/`write_guard` + `sync_to_app` 拆为 locked wrapper + `sync_to_app_unlocked`)、`services/prompt.rs`(`project_prompt_set_to_path` + `sync_to_live`/`sync_all_to_live`)、`commands/sync_support.rs::run_post_import_sync`、`commands/s3_sync.rs::run_download_with_s3_lock`、`commands/webdav_sync.rs::run_download_with_webdav_lock`、`services/usage_cache.rs::invalidate_all`、`store.rs` `#[derive(Clone)] AppState`。
  ⚠ `AppType::ClaudeDesktop` 跳过分支(prompt.rs 2 处 + skill.rs 1 处)**按 fork "桌面专属不予移植"口径删除**(fork `AppType` 枚举 app_config.rs:350 无 ClaudeDesktop 变体,此前同步已刻意剔除该桌面 surface;`all()` 也不会产生它)——不补变体、不恢复上游结构。
- `dfb2e523`(backup SQL fidelity,1302 行)→ 已移植 + Web 适配,保留 fork 严格 authorizer 与 canonical 复制路径。新增标量保真(`format_sql_real`/`format_sql_blob`/`format_sql_value`)、序列保真(`dump_sqlite_sequences`/`restore_sqlite_sequences` 高水位标记)、文件操作序列化(`BACKUP_FILE_OPERATION_LOCK`/`lock_backup_file_operations`/`BackupFileOperationGuard`)、原子发布(`backup_database_file_from_conn_with_hook` 临时文件 → `validate_sqlite_integrity` → `persist_noclobber` 重试 + `next_available_backup_path`/`same_existing_backup_path` + `cleanup_db_backups` protected_paths)、retention count(`with_backup_retain_count`)、restore 预校验(`restore_from_backup_with_hook` 只读开 + staging + `validate_sqlite_integrity`/`validate_imported_schema`/迁移 **在触碰 live 之前** 完成)。`schema.rs` `ensure_model_pricing_seeded_on_conn` 提升为 `pub(crate)`。

### 冲突解决与安全边界保留(carry-forward)

- **authorizer 严格 allow-list 未退化**:fork `install_sql_restore_authorizer` 方法(table/column/function allow-list + `AuthAction::Select` 显式开窗 multi-row VALUES + 拒 ATTACH/Detach/vtable/Unknown/越界 PRAGMA)**完整保留**;未恢复上游宽松 `import_authorizer` 自由函数。为 #6146 序列保真精确开窗:`sqlite_sequence` 的 `Insert`/`Update(seq)`/`Delete`/`Read` 加入 allow-list(CreateTable 原本已允许)。
- **canonical-schema 第二边界未退化**:import 流程仍是 `temp_conn`(不可信 SQL,经 authorizer)→ `normalized_conn`(可信代码创建的 canonical schema,`create_tables_on_conn`/`apply_schema_migrations_on_conn`/`restore_tables`/`restore_sqlite_sequences`)→ `replace_main_with_safety_backup` 替换主库。上游 `Backup::new(&temp_conn, &mut main_conn)` 直接灌库方案**不采纳**(绕过 canonical 重建)。
- **DB 迁移连续性**:fork `SCHEMA_VERSION` 仍为 16,v3.19.2 基线已落地;`dfb2e523` 仅改 `schema.rs` 一行可见性(无迁移号变化),`c9fe340b` 无 schema 变更。从 v3.19.2 基线严格连续,不跳号、不回退。
- **2s/heap/stack 边界**:JS runtime 2s deadline + 16 MiB heap / 256 KiB stack(fork `6b8f3643` 既有)未触及、未放宽。Codex catalog lexical+canonical symlink containment + 32 MiB 流式读 cap 保留。Proxy raw/decompressed body 128 MiB cap 保留。Web SSRF/no-auth + 双运行时边界保留。
- **carry-forward 锁测试并留**:第 10 冲突块是 Git 上下文错位导致的并发测试插入冲突(HEAD 侧 = `sql_import_holds_main_lock_across_safety_backup_and_replace` carry-forward 锁测试,上游侧 = 8 个新 #6146 测试)。两者并留(删 carry-forward 会回归锁约束)。为保证锁测试断言成立,`replace_main_with_safety_backup` 重构为 **单一 `main_conn` 守卫跨安全备份 + hook + preserve_tables live 读取 + 最终替换**(原 fork 在两段间释放 main_conn,writer 会漏入)。
- **preserve_tables 同步语义**:`c9fe340b` 的 `on_staging_ready` 协调点 + "staging 后到达的本地写入必须保留"语义,适配到 fork canonical 流程:`on_staging_ready` 在 canonical staging 完成、替换临界区之前触发;`preserve_tables` 从 live 读取改到 **替换临界区内、`on_staging_ready` 之后、持有 main_conn 锁时** 进行(早期快照会漏掉 staging 后到达的写入)。
- **locale**:`c9fe340b`/`dfb2e523` 不触及 locale;未恢复 zh-TW(fork 仅 en/ja/zh)。
- **新命令**:S2 未新增 Tauri command/Web route(纯后端 + 既有命令的 sync coordination 重构),`check:web-routes` 不变(282 commands / parityFallback 0)。

### SQL-restore 锁测试证据

- `sql_import_holds_main_lock_across_safety_backup_and_replace`:PASS。canonical restore 经 `on_staging_ready` 后,安全备份与替换整段持有 `main_conn` 锁;并发 writer 在 hook 触发期间被阻塞(100ms 内未完成),restore 解锁后写入存活(`restore-concurrent-write= survives`)。
- `import_rejects_truncated_open_transaction_and_keeps_live_database`:PASS。`is_autocommit()` 截断检测拒绝未完成事务 SQL,Rollback 后保留 live。
- `restore_rejects_corrupt_db_before_touching_live_database`/`restore_rejects_future_schema_before_touching_live_database`:PASS。restore 预校验在触碰 live 前拒绝损坏/未来 schema。
- `restore_blocks_backup_deletion_until_live_replacement_finishes`:PASS。retention 不删除受保护的 restore source。
- `failed_backup_publish_leaves_no_visible_or_temporary_file`/`backup_publish_retries_a_noclobber_name_collision`/`concurrent_backup_renames_never_overwrite_the_shared_target`:PASS。原子发布 + 重试 + 并发不覆盖。

### 门禁证据

- `(cd src-tauri && cargo fmt --all -- --check)`:通过(rustfmt 自动修复 2 处后绿)。
- `pnpm format:check`(Prettier):通过。
- `pnpm exec cargo check --manifest-path src-tauri/Cargo.toml`(desktop lib):通过,0 error/0 warning。
- `cargo check --no-default-features --features web-server --example server`(Web cargo check):通过,0 error。
- `pnpm check:web-routes`:282 commands / missing 0 / methodMismatch 0 / parityFallback 0(不变,无新命令)。
- `pnpm check:locales`:2507 keys,en/ja/zh 严格 parity(不变)。
- `pnpm typecheck`:通过。
- `pnpm test:unit`:**973 passed**(163 files),含 `isolated_child_process_group_captures_healthy_tool_output`、`isolated_hung_child_is_killed_on_deadline`、`RoutingActivationBrand` 3、`ProxyToggle` 1。
- Rust focused:`database::` **90 passed / 0 failed**(含 `sql_import_holds_main_lock_across_safety_backup_and_replace`、`sync_import_keeps_local_writes_that_arrive_after_staging`、`sync_import_safety_backup_captures_late_local_writes`、`sql_import_preserves_incremental_auto_vacuum`、`restore_*` 4、`backup_*` 3、`import_*` 5);`tool_version::` **14 passed**;`services::skill::` **40 passed**;`services::prompt::` **3 passed**;`services::sync_protocol::` **23 passed**;`services::proxy::tests::restore_*` **7 passed**(OAuth login 保护)+ `simple_restore_path_keeps_codex_oauth_login` 1。
- Web Rust `cargo test --no-default-features --features web-server --example server`:**1972 passed / 0 failed / 5 ignored**(含 web_api::、dual_runtime_parity::、web_proxy_lifecycle::)。
- `pnpm test:integration`:**49/54 passed**。5 个失败 = PRD 已知非阻塞 flake:ProviderList Claude official-seed empty-state/import-current **1**;SkillsPage skills.sh automatic-fallback/repo-install/pagination **3**;AuthCenterPanel OAuth 5s 超时 **1**(pre-existing,base 分支亦失败,非 S2 引入)。相关 S2 测试全绿:ImportExportSection **2/2**、WebdavSyncSection **2/2**、ModelsDevPricing **3/3**(parity)。无新增产品失败。

### 拋余风险与 carry-forward

- `3f75bbdf` StepFun reasoning 推断分支属 fork 既有延期项(Codex Chat 路由栈未移植),不进入 S3–S8;若未来恢复该路由栈需统一移植整套推断表 + codex.rs 基线,并重跑 StepFun per-model effort 回归。
- S3(定价/usage,5 提交)按 grilling #7 仍需 FE-preset↔Rust-seed 一致性核对;`dfb2e523` 的 `dump_sqlite_sequences`/`restore_sqlite_sequences` 已为 S3 的 usage/backup 种子回填提供高水位保真基础。
- 子任务 `feat-managed-oauth-accounts` 的前置依赖(`d2b070c9` never clobber Codex login)已在 S2 落地,可启动。
- 下一批:S3 定价/usage(`bad9c151`/`5602324b`/`7dc0a725`/`3d126f45`/`46f19a15`)。

## S1 follow-up：Qianfan scoped-id API 安全性证据复核（2026-08-20，grounded-reviewer 触发）

S1 落地时对"OpenClaw 千帆模型 id 限定为 `qianfan-tokenplan/deepseek-v4-pro`"的 API 安全性主张
（`settingsConfig.models[].id` 是 UI/目录元数据，API 模型名来自 `primary` 后缀）原缺证据。
grounded-reviewer 指出：`rebaseOpenClawModelRef` 仅 rebase `suggestedDefaults`（primary/
fallbacks/catalog），不触及 `models[].id`；`modelsDevPricing.ts:359 lastIndexOf` 属定价归一函数，
非 API 请求路径证据。事后补验：

- **`models[].id` 既有约定是混合的，且 scoped 形态早于 S1 存在**：bare（DeepSeek-direct
  `deepseek-v4-pro`、AiHubMix `claude-opus-5`）与 scoped（OpenRouter `anthropic/claude-opus-5`、
  TheRouter `openai/gpt-5.3-codex`、PPIO `deepseek/deepseek-v4-flash-0731`、Novita
  `zai-org/glm-5.1`、Nvidia `moonshotai/kimi-k2.5` 等）共存。S1 的 `qianfan-tokenplan/deepseek-v4-pro`
  沿用既有 reseller scoped `vendor/model` 形态，非新发明。
- **`models[].id` 是目录/展示元数据，非 API 模型名**：DeepSeek-direct 预设 `models[].id =
  "deepseek-v4-pro"`（bare）但其 `modelCatalog` 键为 `"deepseek/deepseek-v4-pro"`（scoped），
  两者本就不匹配；说明 `models[].id` 与 catalog 键是各自独立的展示字段，API 模型名不走它。
- **API 模型名来自路由键后缀**：`OpenClawDefaultModel.primary`（scoped `<provider-key>/<model>`，
  经 `rebaseOpenClawModelRef` 重写为用户 key）的后缀 `<model>` 是 API 模型名；千帆 primary
  `qianfan-tokenplan/deepseek-v4-pro` 后缀 `deepseek-v4-pro` = 百度端点期望的裸名。
- **fork 契约止于配置文件结构**：`openclaw_config.rs` 写 `agents.defaults.model`/`models` +
  `models.providers.<key>`（含 `OpenClawProviderConfig.models: Vec<OpenClawModelEntry>`）；
  实际 API 请求由 OpenClaw 二进制（外部）发起，其路由行为与既有全部 scoped reseller 预设
  （OpenRouter/TheRouter/PPIO 等）一致，本 fork 无法也无需在其内单独验证外部二进制行为。

结论：S1 千帆 scoped-id 安全，既有约定支持；`qianfanTokenPlanPresets.test.ts` 锁定 scoped id +
bare name + cost 是 fork 可控的契约边界。若未来 OpenClaw 二进制行为变更或新增真实 Web 冒烟覆盖
该路径，再回归。

## S3 结果（2026-08-20，25cb05fb）

4 ported / 1 partial（`46f19a15` 的 `transform_codex_chat.rs` hunk 跳过）。5 个提交全部处理，无整提交跳过。

### 提交映射与处置

- `bad9c151`（DeepSeek V4 peak-tier + Gemini 3.7 Flash）→ 已移植。seed 表 5 行 DeepSeek V4 改高峰档（flash/chat/reasoner/v4-flash-0731 = 0.44/1.32/0.014；pro = 1.32/3.96/0.044），新增 `gemini-3.7-flash` = 0.75/3.75/0.075（介绍价，2026-12-31 到期后 1.50/7.50/0.15，刻意不进 audit-ignore）。`pricing_fixes` 末尾追加 5 条 repair（old-value guard 0.14/0.28/0.0028 与 0.435/0.87/0.003625），保留 fork 既有 kimi-for-coding/ling-2.5-1t/kimi-k2.5/kat-coder/qwen3.5-plus/qwen3.6-plus repair 在前。`tests.rs` 的 `model_pricing_seed_repairs_known_outdated_builtin_prices` 断言改 1.32/3.96/0.044（两跳链 1.68/3.36/0.14 → 0.435/0.87 → 1.32/3.96）。fork 专属 `schema_model_pricing_is_seeded_on_init` 的 for-loop 断言同步改高峰档（上游该测试无此 for-loop）。
- `5602324b`（DeepSeek catalog mirror angle-bracket restore）→ 已移植。`codex_deepseek_catalog_template.json` 8 处角括号文本恢复（`<this good thing>`、`<this obviously bad thing>`、`<X>, not <Y>`、`rather than <`、spaced-path markdown link `<My Project/My Report.md:3>` 各 4 处 = 2 模型 × 4 处），与上游 post-`5602324b` 逐字节一致。`codexProviderPresets.ts` 的 preset 注释 hunk 对 fork 为 **no-op**（fork HEAD 的 `deepseek-v4-pro` 已无"官方预计 2026-08"警告注释，早前同步已简化）；cherry-pick 冲突在不相关的 provider 区（lionccapi vs crazyrouter/DMXAPI），按 fork 既有 provider 列表保留 HEAD 解决。
- `7dc0a725`（Grok 4.5 cached rate + Grok 4.6 + DeepSeek alias）→ 已移植。`grok-4.5` seed cached 0.50→0.30（+ repair old-value guard 0.50），新增 `grok-4.6` seed 2/6/0.50，`grok-4.5-build` 注释更新。`deepseek-v4-flash-0731` alias seed 由 `bad9c151` 已落高峰档 0.44/1.32/0.014（`7dc0a725` 原录旧价 0.14/0.28/0.0028，被 `bad9c151` 超越；cherry-pick 产生重复行，删除 `7dc0a725` 旧价副本，保留 `bad9c151` 高峰档）。`usage_stats.rs` grok-4.5 backfill 测试期望改 0.30 基（cache_read 0.000125→0.000075，total 0.001625→0.001575），保留 fork 的 `get_request_detail` 结构（上游用直查 SQL）；`find_model_pricing_row` 的 `xai/grok-4.5` 断言改 0.30。
- `3d126f45`（multi-year trend tooltip/dot alignment）→ 已移植，无冲突。新增 `buildUsageTrendChartData`/`formatUsageTrendTickLabel` 导出函数 + `UsageTrendChartPoint` 接口，XAxis `dataKey="xKey"` + `tickFormatter` + `allowDuplicatedCategory={false}`，tooltip 从 point payload 读 `tooltipLabel`。**保留 fork 的 `initialDimension={{ width: 960, height: 350 }}`** prop（上游无此 prop）。新增 `tests/components/UsageTrendChart.test.ts`（4 测试：跨年 xKey 唯一、跨年 tick 含年、单年短 MM/DD、tick 索引稀疏后按 xKey 解析）。
- `46f19a15`（DeepSeek cache-hit tokens）→ **部分移植**。`proxy/usage/parser.rs` 的 `openai_cache_read_tokens` 末位兜底 `.or_else(|| usage.get("prompt_cache_hit_tokens"))` 已落地 + 3 个回归测试（response/stream/cache-hit、标准字段优先、stream DeepSeek）。`transform_codex_chat.rs` hunk **跳过**：fork HEAD 无该文件（Codex Chat 路由栈是既有延期项，S2 `3f75bbdf`/S4 `9db9c56f` 已证实并记录）；恢复该叶文件缺整套 ~5.7k LOC 依赖基础，越过 S3 边界。`parser.rs` 兜底独立生效（usage 页/proxy_request_logs 计费路径），不依赖 Codex Chat 路由栈。

### ⚠ grilling #7 FE-preset↔Rust-seed 一致性核对（关键关卡）

- **DeepSeek-direct OpenClaw preset cost 同步改高峰档**：`deepseek-v4-pro` preset `cost: {0.435, 0.87, 0.003625}` → `{1.32, 3.96, 0.044}`；`deepseek-v4-flash` preset `cost: {0.14, 0.28}` → `{0.44, 1.32, 0.014}`。**上游 `bad9c151` 未改 preset cost**（上游无 `ModelsDevPricing.web-server` parity 测试，preset 与 seed 可发散）；fork 的 parity 测试会因裸 id `deepseek-v4-pro`/`deepseek-v4-flash` 匹配 seed 而失败，故按"seed 是裸 id SSOT"裁定改 preset 跟随 seed。
- **`S6A_SEED_SENTINELS` 更新**：`deepseek-chat`/`deepseek-reasoner` 由 0.14/0.28/0.0028 改 0.44/1.32/0.014（跟随 `bad9c151` seed）。
- **Qianfan scoped id 不退化**：`qianfan-tokenplan/deepseek-v4-pro`（百度转售价 0.0025/0.01）保持 scoped，不与裸 seed `deepseek-v4-pro` 碰撞，parity 测试正确跳过。S1 的 scoped-id 设计未被 S3 seed 更新回退。
- **无 cost=$0 回归**：DeepSeek V4（0.44/1.32/0.014、1.32/3.96/0.044）、Gemini 3.7 Flash（0.75/3.75/0.075）、Grok 4.5（0.30）/4.6（0.50）均非零。`find_model_pricing_row` 有损清洗未引入零价（`deepseek-v4-flash-0731` alias 已补 seed，不再静默按 0 计费）。
- **`ModelsDevPricing.web-server` parity 测试 3/3 通过**（grilling #7 关卡绿）。
- **`openclawPresetPricing.test.ts`**（focused，非零 tuple 检查）通过；`qianfanTokenPlanPresets.test.ts` 不受影响（scoped id + 转售价不变）。

### 冲突解决与 carry-forward

- **`repair_current_model_pricing` 只匹配旧内置 tuple**：新增 5 条 DeepSeek peak-tier repair 的 old-value guard 是 0.14/0.28/0.0028 与 0.435/0.87/0.003625（v3.19.2 基线值），用户 override 与历史非零 cost 不被改写。grok-4.5 repair old-value guard 是 0.50（旧 cached 价）。carry-forward 未退化。
- **models.dev 本地文件只存 override/tombstone**：S3 不触及 models.dev 同步逻辑，无退化。
- **无 zh-TW locale 恢复**：S3 不触及 locale（en/ja/zh 2507 keys 严格 parity）。
- **无新 Tauri command/Web route**：S3 是 seed/repair/parser/前端组件改动，`check:web-routes` 不变（282 commands / missing 0 / methodMismatch 0 / parityFallback 0）。
- **Codex Chat 路由栈延期项**：`46f19a15` 的 `transform_codex_chat.rs` hunk 与 S2 `3f75bbdf`、S4 `9db9c56f` 同属该延期项；若未来恢复该路由栈，需统一移植 `transform_codex_chat.rs`/`codex_chat_common.rs`/`streaming_codex_chat.rs` 整套基线 + `chat_usage_to_responses_usage` 的 DeepSeek cache-hit 兜底，并重跑 dropped-tool-call 流回归。
- **DB 迁移连续性**：S3 无 schema 迁移号变化（`SCHEMA_VERSION` 仍 16），仅 seed/repair 数据更新，从 v3.19.2 基线严格连续。

### 门禁证据

- `(cd src-tauri && cargo fmt --all -- --check)`：通过。
- `pnpm format:check`（Prettier）：通过。
- `pnpm exec cargo check --manifest-path src-tauri/Cargo.toml`（web lib）：通过。
- `cargo check --manifest-path src-tauri/Cargo.toml --all-targets --locked`（desktop）：通过，0 error/0 warning。
- `pnpm check:web-routes`：282 commands / missing 0 / methodMismatch 0 / parityFallback 0（不变，无新命令）。
- `pnpm check:locales`：2507 keys，en/ja/zh 严格 parity（不变）。
- `pnpm typecheck`：通过。
- `pnpm test:unit`：**977 passed**（164 files），含 `UsageTrendChart` 4、`openclawPresetPricing` 1。
- Rust focused：`database::` **90 passed**（含 `model_pricing_seed_repairs_known_outdated_builtin_prices`、`schema_model_pricing_is_seeded_on_init`）；`services::usage_stats::` **20 passed**（含 `test_backfill_deducts_cache_read_for_grokbuild_total_rows` 0.30 基、`test_model_pricing_matching`）；`proxy::usage::parser::` **24 passed**（含 3 个新 DeepSeek cache-hit 测试）。
- Rust lib 全量：**2010 passed / 0 failed / 5 ignored**。
- Web Rust `cargo test --no-default-features --features web-server --example server`：**1975 passed / 0 failed / 5 ignored**（含 web_api::、dual_runtime_parity::、web_proxy_lifecycle::）。
- `pnpm test:integration`：**49/54 passed**。5 个失败 = PRD 已知非阻塞 flake：ProviderList Claude official-seed empty-state/import-current **1**；SkillsPage skills.sh automatic-fallback/repo-install/pagination **3**；AuthCenterPanel OAuth 5s 超时 **1**（pre-existing，base 分支亦失败，非 S3 引入）。**`ModelsDevPricing.web-server` parity 3/3 通过**（grilling #7 关卡绿）。

### 残余风险与 carry-forward

- Codex Chat 路由栈仍是既有延期项（`46f19a15` transform_codex_chat.rs + S2 `3f75bbdf` + S4 `9db9c56f`）；未来恢复需统一移植整套基线。
- DeepSeek V4 高峰档 seed 是单 tier 录入（无时段维度），夜间/凌晨用量高估一倍（上游 `bad9c151` 拍板口径，fork 沿用）。
- Gemini 3.7 Flash 介绍价 2026-12-31 到期后需走 seed + repair 双写改回 1.50/7.50/0.15（届时 models.dev 先更新，审计 A 段会自动报出该行；刻意不进 audit-ignore）。
- 下一批：S4 codex/provider 功能（12 提交）。

## S4a 结果（2026-08-18，`5a5874a5`，codex/provider 功能第 1 检查点）

### 提交映射与处置

- `d1c550ba` → `5a5874a5`：已移植 Goal mode toggle 清理。删除 JSX/state/handlers
  （`enableGoalMode`/`setEnableGoalMode`/Goal toggle UI），providerConfigUtils 移除
  TOML_GOALS 常量与 hasGoalMode/extractGoalMode helpers，en/ja/zh `enableGoalMode`
  键删除，CommonConfigModalBehavior test 移除 Goal 用例，toml-edge-cases 删除
  CRLF Goal 块。删除冗余 `providerConfigUtils.codex.test.ts`（被主 test 文件覆盖）。
- `a98829ba` → `5a5874a5`：已移植 IME-safe provider fields。新增 `ime-safe-input.tsx`
  （ImeSafeInput + composition-commit onBlur），OpenCodeFormFields 采用
  ImeSafeInput+composition-commit onBlur，保留 Input import。
- `897ca892` → `5a5874a5`（仅 frontend-independent 部分）：CodexOauthQuotaFooter、
  subscription.ts、ProviderCard codexAccount identity 显示。Rust tray.rs 待 S4c。

### 范围拆分说明（task-contract guardian 标注）

`5a5874a5` 把 S4 的单一提交检查点拆为 S4a（d1c550ba + a98829ba + 897ca892-frontend）+
S4b（0455a92c frontend + 897ca892-frontend 剩余）。拆分原因：0455a92c 的 Rust
managed-codex 事务依赖 `a2e22f33`（`feat-managed-oauth-accounts` 子任务）引入的辅助函数
（`preflight_managed_codex_live`/`managed_codex_oauth_account_id` 等），fork 全缺，
无法在 S4 主体独立移植；前端独立增量可先落。反复的 checkout+apply-3+keep-theirs
合并配方在 ProviderForm.tsx 上失败 5+ 次后，监控指令冻结该配方，改为按文件隔离适配。
检查点提交是防中断丢工作的刻意决定（会话内已发生 3 次未提交工作被破坏）。

### 门禁

- typecheck 0 错、format、cargo check、cargo fmt、web-routes（0/0/0）、
  locales（2506 parity）、test:unit 969/969、Web Rust 1975/0 全绿。
- 残余：897ca892 Rust tray.rs、f62c854a/b109dcd3/f748f3ac/d9d4a660/40cac1a6
  待 S4c 评估。

## S4b 结果（2026-08-18，`84d54e7d`，0455a92c 前端独立增量）

### 提交映射与范围拆分

- `0455a92c` multiple follow-login providers（829 行）→ **拆分**：
  - **前端独立增量**（已落 `84d54e7d`）：providerCapabilities.ts 新增
    `CODEX_OFFICIAL_PROVIDER_ID`/`CodexOfficialIdentity`/`resolveCodexOfficialIdentity`/
    `supportsOfficialProxyTakeover`/`hasExplicitNonOpenAiCodexModelProvider`；
    ProviderCard.tsx 集成 useManagedAuth（`enabled` 门控：仅当
    `codexOfficialIdentity === "managed_account"` 时查询，避免 N 卡片 N 次 API 调用）、
    identity 显示区块（h3 title/truncate、isBoundCodexOfficial 门控）、
    CodexOauthQuotaFooter autoQueryInterval + `!isBoundCodexOfficial || usageEnabled`
    渲染门控、onConfigureUsage `isCodexOauth && !isBoundCodexOfficial`；useProviderActions.ts
    用 `supportsOfficialProxyTakeover` 取代 blanket `category === "official"` takeover 拦截
    （native Codex official + managed 卡片现在允许在代理接管时切换）；useManagedAuth.ts
    新增 `enabled` option（默认 true）；providerConfigUtils/presetEntries/mutations/
    AddProviderDialog/EditProviderDialog fork AppId 适配（claude-desktop/pi 移除）。
  - **Rust managed-codex 事务** + `ProviderForm.codexManagedAccount.test.tsx`（8 用例）
    → 移交 `feat-managed-oauth-accounts` 子任务。证据链：`0455a92c` 的 mod.rs hunk
    引用 `preflight_managed_codex_live`/`managed_codex_oauth_account_id`/
    `managed_codex_add_transaction_error`/`write_preflighted_or_current_live`，
    这些函数在 `413c09e0..v3.20.0` 仅由 `a2e22f33` 引入（`git log -S` 证实），
    fork 全缺。测试断言 CodexFormFields managed-account 选择 UI（fork 的 CodexFormFields
    是 294 行精简版，完整 1394 行版含 CodexOAuthSection 渲染块随子任务落地）。
    测试文件 fork 适配版暂存在父任务历史 commit `7265596a`（已被 `84d54e7d` 取代），
    子任务用 `git show 7265596a:tests/components/ProviderForm.codexManagedAccount.test.tsx` 取回。
- `897ca892` frontend 剩余（CodexOauthQuotaFooter/subscription.ts/ProviderCard codexAccount
  identity）→ 已落 `84d54e7d`。

### 门禁（全绿）

- `pnpm typecheck`：0 错。
- `pnpm format:check`、`cargo fmt --all -- --check`：通过。
- `pnpm exec cargo check --manifest-path src-tauri/Cargo.toml`：通过。
- `pnpm check:web-routes`：missing 0 / methodMismatch 0 / parityFallback 0。
- `pnpm check:locales`：en/ja/zh parity。
- `pnpm test:unit`：**985 passed / 985**（167 files）。16 个失败已清零：删除 2 个断言
  fork 无的 AuthSettingsPanel 用例（AddProviderDialog/EditProviderDialog "clears the
  nested auth panel"），源码修复 8 个（ProviderCard.codexAccount identity/quota 门控 4、
  useProviderActions supportsOfficialProxyTakeover 2、AddProviderDialog/EditProviderDialog
  无 UI 的 auth-panel 用例移除后对齐），移交 8 个（ProviderForm.codexManagedAccount → 子任务）。

### 残余与下一批

- 0455a92c Rust managed-codex + ProviderForm.codexManagedAccount.test.tsx → 子任务
  `feat-managed-oauth-accounts`（prd 已更新移交记录与取回路径）。
- S4c 待评估：`897ca892` Rust tray.rs、`f62c854a`（cancel stale device login，
  reverted — 需完整 test-helpers 移植）、`b109dcd3`、`f748f3ac`、`d9d4a660`、
  `6a7da87c`、`40cac1a6`（依赖延期 Codex Chat reasoning 栈）、`d01eab97`、`6e424fd3`。

## S4c 结果（2026-08-21）

7 ported / 2 skipped（proven inapplicable，延期 Codex Chat reasoning 栈）。9 个提交全部处理。

### 提交映射与处置

- `6e424fd3`（restore 1M context toggle）→ `feffa171`：已移植。fork 的 `21c5b3b6` 把 1M
  toggle **整块删除**（106 行，非注释），故 `6e424fd3`（上游取消注释）不能 plain cherry-pick。
  toggle JSX、`toggleStates` useMemo、`handleContextWindowToggle`/`handleCompactLimitChange`
  回调、cleanup useEffect 重新加入，与上游 `6e424fd3` 结果态逐字节一致（剔除 Goal mode，fork
  S4a `d1c550ba` 已删；JsonEditor rows 6/8 保留 fork 约定）。3 个 providerConfigUtils helper
  与 i18n 键（contextWindow1M/autoCompactLimit/autoCompactLimitHint）已在 fork HEAD 保留。
  `CodexConfigSections.test.tsx` 从断言 hidden 翻转为断言 visible+checked+enabled。

  **task-contract 守护复核**：守护一度怀疑"fork HEAD 1M toggle 已激活"——复核 `git show
  HEAD:...CodexConfigSections.tsx` grep `contextWindow1M`/`extractCodexTopLevelInt` 等 6 符号
  计数=0，确认 fork HEAD 完全无该块；守护读到的是改动后工作树。最初诊断正确。

- `897ca892`（OAuth usage configurable，Rust tray.rs 半）→ `f462a4ce`：已移植。fork 无
  `CODEX_OFFICIAL_PROVIDER_ID` 常量（仅 `GROKBUILD_OFFICIAL_PROVIDER_ID`），但字面量
  `"codex-official"` seed id 与 `managed_account_id_for("codex_oauth")` helper 均已存在。
  在 `providers_seed.rs` 新增 `CODEX_OFFICIAL_PROVIDER_ID` 常量并从 `database/mod.rs`
  re-export（单源真），`provider_uses_official_subscription` 加入 managed-codex guard
  （非 codex-official + official category + 有 codex_oauth managed_account_id → 返回 false，
  不进 tray app-wide 订阅缓存）。1 个上游测试 `managed_codex_quota_stays_out_of_the_app_
  wide_tray_cache` 移植。

- `f62c854a`（cancel stale device login）→ `4e060fa5`：已移植。fork 此前 reverted 该提交，
  原因"需完整 test-helpers 移植"——重新评估：上游改动是单文件自包含 Rust
  （`codex_oauth_auth.rs`），无外部 test-helper 依赖；manager struct、pending_device_codes、
  start_device_flow、clear_auth、ExpiredToken 变体均已存在且结构匹配。新增 `login_epoch:
  AtomicU64`，`clear_auth` 时 `fetch_add(1)`，`start_device_flow` 在网络请求前捕获 epoch，
  `register_pending_device_code` helper 在重新登记时校验 epoch 不变（否则返回 ExpiredToken）。
  产品代码 + 1 上游测试直接移植，无 test-helpers 基础设施需求。

- `b109dcd3`（Grok Build stop Codex copy）→ `11cd4ba3`：已移植 + 适配。fork 的
  `CodexFormFields.tsx` 是 294 行精简组件（无 codexDefaultModel 字段、无 advancedSectionHint/
  maxOutputTokensHint/reasoningEffortHint/defaultModelHint——那 4 个在上游 1.2k 行组件里）。
  仅 model-name placeholder 适用：按 appId 分流（grokbuild → `grokBuild.defaultModelPlaceholder`
  "例如: grok-4.5"，codex → `codexConfig.modelNamePlaceholder` "例如: gpt-5.4"）。新增
  `grokBuild.defaultModelPlaceholder` 三 locale 键（en/ja/zh）。另 4 个 Codex 专属 hint 在
  fork 精简组件无消费者，按 code-reuse 指引不补 orphan 键。modelNameHint 对 Grok Build 同样
  成立（其 config 即 config.toml）。GrokBuildProviderForm 确传 `appId="grokbuild"`，分支生效。

- `f748f3ac`（grokbuild form align with Codex）→ `e97e01e4`：已移植 + 适配。fork 的
  `CodexFormFields` 精简（无 `apiFormat`/`onApiFormatChange`/`anthropicAuthField`/
  CodexChatReasoning/localProxy props——延期栈），故上游 `f748f3ac` 的完整 Codex 风格高级区
  不适用；仅 apiBackend 移除适用。删除 `grokApiBackendFromApiFormat` helper + `apiBackend`
  state + `grokbuild-api-backend` FormItem；所有 config builder 调用恒用
  `GROK_BUILD_DEFAULT_API_BACKEND`（"responses"）；apiFormat state 保留本地、经 meta 持久化。
  3-col 网格降为 2-col（profile + contextWindow）。**task-contract 守护复核**：守护指出测试仍
  import `grokApiBackendFromApiFormat` 且断言它——更新测试为 "always writes the default
  api_backend regardless of preset format"，保留组件级断言 `selected.api_backend=="responses"`
  （验证新 always-default 语义，非删测试）。

- `d9d4a660`（prevent macOS IME corruption）→ `0277a8e1`：已移植 + 补采用缺口。S4a 移植
  `ime-safe-input.tsx`（102 行完整版含 normalize prop）+ OpenCodeFormFields 3 子组件采用；
  d9d4a660 剩余采用缺口补上：HermesFormFields（baseUrl + model id + model name → ImeSafeInput，
  number 输入留 Input）、OpenClawFormFields（同）、OpenCodeFormFields 的 model-name 行输入
  （line 945 仍 `<Input>`，S4a 只转了 3 子组件——这是 OpenCode IME 测试失败的根因）、
  ProviderForm 的 opencode-key/openclaw-key/hermes-key（`normalize={normalizeProviderKey}`，
  从 3 个内联 onChange 抽取）。新建 HermesFormFields.test.tsx + OpenClawFormFields.test.tsx
  （IME composition 回归），OpenCodeFormFields.test.tsx 增上游 IME 测试。ImeSafeInput.test.tsx
  S4a 已覆盖 normalize prop。

- `40cac1a6`（per-model reasoning levels）→ `69534266`：**拆分移植（catalog 数据层 only）**。
  `codex_config.rs` 新增 `CODEX_REASONING_LEVEL_DESCRIPTIONS` 常量 + `codex_canonical_efforts`/
  `codex_supported_reasoning_levels`/`apply_codex_reasoning_level_override` + spec 字段
  `reasoning_levels`/`default_reasoning_level` + `codex_catalog_model_specs` 解析（camelCase/
  snake_case 双格式）+ `codex_vendor_catalog_model_entry` vendor_default 捕获 + override 应用。
  `types.ts` `CodexCatalogModel` 加 `reasoningLevels`/`defaultReasoningLevel`。2 个上游测试移植。
  **task-contract 守护复核**：守护指出 fork `codex_config.rs:1965` 测试已断言
  `supported_reasoning_levels==[low,high,max]`（模板透传），数据层有消费者、非死代码——
  纠正了"整提交 proven-inapplicable"的误判，改为单独移植数据层。转换层（transform_codex_chat.rs/
  transform_codex_anthropic.rs）+ 前端 catalog 模型编辑器 UI 依赖延期栈，跳过。

- `d01eab97`（OpenCode Zen reasoning effort）→ **整提交跳过（proven inapplicable）**。触及
  `transform_codex_chat.rs` zen 映射 + `CodexChatReasoningConfig.effort_levels` + `codex.rs`
  resolve 末端 + `infer_aggregator_platform_config` opencode.ai 条目 + 前端
  `codexChatReasoning`/`effortValueMode`/`reasoningLevels`/`mapCodexCatalogModelForForm`——
  全部在延期 Codex Chat reasoning 栈内（fork 全缺，已 grep 确认）。恢复需先落地整套
  ~1.1k LOC 推断表 + 类型 + `codex.rs` 基础，越过 S4 边界。与 S2 `3f75bbdf`/S3 `46f19a15`/
  S4 `9db9c56f`/`6a7da87c` 同属延期项，若未来恢复该路由栈统一移植。

- `6a7da87c`（grokbuild input token details）→ **整提交跳过（proven inapplicable）**。
  `git show --stat` + `--name-only` 双重确认仅命中 `streaming_codex_chat.rs` +
  `transform_codex_chat.rs` 两延期文件，无任何 grokbuild/session_usage 文件
  （尽管提交标题含 "grokbuild"）。`chat_usage_to_responses_usage` 在 fork 全树无定义。
  fork 的 `services/session_usage_grokbuild.rs`（S5 `cd161f44` 已落地）是另一条 Grok native
  `updates.jsonl` turn-level 计费路径，与此提交修复的 Chat→Responses 转换层 usage 字段是独立路径。
  委派清单归为"必移植"是基于提交标题而非实际文件清单的误判。

### 冲突解决与 carry-forward

- **延期 Codex Chat reasoning 栈边界一致**：`d01eab97`/`6a7da87c` 整提交跳过 + `40cac1a6`
  转换层/前端 UI 跳过，与 S2 `3f75bbdf`/S3 `46f19a15`/S4 `9db9c56f` 同口径；若未来恢复该路由栈
  需统一移植 `transform_codex_chat.rs`/`codex_chat_common.rs`/`streaming_codex_chat.rs`/
  `transform_codex_anthropic.rs`/`codex.rs` 基础 + `CodexChatReasoningConfig` + `chat_usage_to_
  responses_usage` 的 cache-hit 兜底，并重跑 dropped-tool-call 流 + per-model effort 回归。
- **fork 精简 CodexFormFields 边界保留**：`b109dcd3`/`f748f3ac` 的适配均保留 fork 精简组件结构，
  不引入延期栈符号（CodexChatReasoning/setCodexChatReasoning/setPromptCacheRouting/
  buildLocalProxyRequestOverrides/overridesResult/codexCatalogModels）。
- **无认证 Web 姿态、canonical-schema allow-list、2s/heap/stack 边界、catalog 32 MiB cap**
  未触及、未退化。S4c 无新 Tauri command/Web route（`check:web-routes` 不变）。
- **locale**：未恢复 zh-TW（fork en/ja/zh only）。S4c 仅 `b109dcd3` 加 1 个 grokBuild 键 × 3 locale。

### 门禁证据

- `(cd src-tauri && cargo fmt --all -- --check)`：通过。
- `pnpm format:check`（Prettier）：通过。
- `pnpm exec cargo check --manifest-path src-tauri/Cargo.toml`：通过。
- `pnpm check:web-routes`：282 commands / missing 0 / methodMismatch 0 / parityFallback 0（不变，无新命令）。
- `pnpm check:locales`：2507 keys，en/ja/zh 严格 parity。
- `pnpm typecheck`：通过。
- `pnpm test:unit`：**992 passed**（169 files），含 CodexConfigSections 2、tray::tests 25
  （managed_codex guard 1）、codex_oauth_auth::tests 14（device_start_rejects 1）、
  GrokBuildProviderForm 7、HermesFormFields 2、OpenClawFormFields 2、OpenCodeFormFields 10
  （IME 1）、ImeSafeInput 6、codex_config::tests 45（reasoning-levels 2）。
- `pnpm test:integration`：**49/54 passed**。5 个失败 = PRD 已知非阻塞 flake：ProviderList
  Claude official-seed empty-state/import-current **1**；SkillsPage skills.sh
  automatic-fallback/repo-install/pagination **3**；AuthCenterPanel OAuth 5s 超时 **1**
  （pre-existing，base 分支亦失败，非 S4c 引入）。无新增产品失败。
- Web Rust `cargo test --manifest-path src-tauri/Cargo.toml --no-default-features
  --features web-server --example server`：**1978 passed / 0 failed / 5 ignored**。其中
  `web_api::` **27 passed**（含 handlers/auth/common/s3 等）、
  `dual_runtime_parity::` **3 passed**、`web_proxy_lifecycle::` **7 passed**。
- Rust focused（批末统一跑）：见各 commit 消息。

### 残余风险与 carry-forward

- `d01eab97`/`6a7da87c` + `40cac1a6` 转换层/前端 UI 仍是既有延期项（Codex Chat reasoning 栈）。
- `40cac1a6` 数据层已落但前端 catalog 模型编辑器 UI 缺失——`reasoningLevels` 目前无 UI 生产者；
  若用户手工在 `settings_config.modelCatalog.models[].reasoningLevels` 写入，catalog 生成会
  消费（有 `codex_config.rs` 测试覆盖）。前端 UI 随延期栈恢复时统一落地。
- 下一批：S5 UI/refactor（14 提交）。

## S5 结果（2026-08-23，UI/refactor）

### 提交映射与处置（14 提交：11 移植 / 3 整提交跳过）

按上游实际拓扑顺序处理（非 dispatch 人工顺序）——关键纠正：dispatch 把
`580a4d7b` 列为第 1 项，但它在拓扑上是 `7e152d75`/`076c2744` 的下游且依赖
其创建的 `shared/ModelDropdown.tsx`。正确顺序为 `7e152d75` → `076c2744` →
`580a4d7b`。`git merge-base --is-ancestor` 证实 `580a4d7b` 不是 `7e152d75`/
`076c2744` 的祖先（时间戳 580a4d7b=08-12 晚于 7e152d75/076c2744=08-11）。

- `7e152d75` → `bcb5ae53`：已移植 + 前置创建。fork 从未落地
  `shared/ModelDropdown.tsx`——v3.19.2 同步时 `2deee109`（创建该组件的提交）
  被跳过，因其仅触及 fork 不存在的 `ClaudeDesktopProviderForm.tsx`，而
  `OpenCodeFormFields.tsx` 内联了等价 `ModelDropdown` 函数（用旧 DropdownMenu
  无 fuzzy）。本提交连带创建 `shared/ModelDropdown.tsx`（上游 076c2744+7e152d75
  最终态：Command+Popover fuzzy search + vendor keywords + aria-label），替换
  `ModelInputWithFetch` 与 `ClaudeFormFields` Copilot block 的内联 DropdownMenu
  （CopilotModel → FetchedModel 映射），新增 3 个 locale keys
  （searchModelPlaceholder/Empty/AriaLabel，en/ja/zh）。
- `076c2744` → `7386a797`：已移植，clean cherry-pick（shared ModelDropdown 已存在）。
  HermesFormFields/OpenClawFormFields 内联 DropdownMenu 分组拣选器统一到
  `<ModelDropdown>`；OmoFormFields ModelCombobox 与 ProfileSwitcher 加 Command
  label（accessibility）；新增 `tests/components/ModelDropdown.test.tsx`
  覆盖 labelled search input + vendor 过滤。
- `580a4d7b` → `86520729`：已移植并适配。Hermes 表单采用上游 Pi-style 行布局
  （header row Model ID/Display name/spacer + ChevronRight 展开式 + context_length
  可折叠 detail panel + rateLimitDelay 直显，移除 role badge 与 Collapsible 高级区），
  但保留 fork S4c 的 `ImeSafeInput`（model id/name/baseUrl 三处）、`generateUUID`
  稳定 modelKeysRef、`expandedModelKeys: Set<string>`（按键不按 index）。合并上游
  结构测试（Pi-style rows / request interval）与 fork IME 组合回归（4 测试全绿）。
- `ec842156` → `79424b99`：已移植，clean cherry-pick。OpenCode Extra SDK Options 由
  Collapsible 改为常驻可见区（FormLabel + hint + add button，移除 extraOptionsOpen
  state 与 auto-open effect）；Models section 加 border-l family divider。新增 2 个
  结构测试（always-visible addable section + model divider）。
- `c0050623` → `da6136cf`：已移植并适配。Radix `CheckboxPrimitive` → 原生
  `<input type="checkbox">`，保留 `onCheckedChange(boolean)` + `checked(CheckedState)`
  契约；indeterminate 经 `useLayoutEffect` 设原生 `.indeterminate` 属性 +
  `aria-checked="mixed"`。全量核查消费方（~14 处）：无 `checked="indeterminate"`
  传递、无 `data-[state=checked]` peer CSS 依赖（仅 switch.tsx 用 Radix data-state，
  未受影响）、无 peer: 兄弟选择器。typecheck + 7 个消费方测试文件（19+22+25 测试）全绿。
- `5b77da2b` → `53c52afb`：已移植，clean cherry-pick。OpenClaw User-Agent 行加
  border-l family divider。
- `95b95da6` → `e0f07dbc`：已移植并适配。OpenClaw 表单采用 Pi-style family 模型
  编辑器（header row + ChevronRight 展开式 + 原生 OpenClaw detail panel：reasoning
  switch / input-types checkboxes / contextWindow / maxTokens / cost grid），保留
  fork `ImeSafeInput`；ProviderActions/ProviderCard/useProviderActions 加默认模型
  选择流；合并上游结构测试（family rows / native detail panel）与 fork IME 回归
  （4 测试全绿）。expandedModels 改为 `Set<string>` 按键。
- `8673e9d8` → `4c7a0b44`：已移植并适配（手动，cherry-pick 冲突因 locale 结构偏移
  与缺失键过大）。Claude 高级选项 Collapsible 加 bordered card 样式（rounded-lg
  border border-border-default p-4）+ full-width justified-start trigger +
  leading-relaxed hint；模型映射 divider 改 border-border-default；
  providerForm.apiFormat/apiFormatHint 重命名为 upstream-format 文案（en/ja/zh）。
  **跳过 ClaudeDesktopProviderForm.tsx hunk**（proven inapplicable，fork 无该文件）；
  **未引入**相邻 `customUserAgent*/localProxy*` 键——它们属 `6fd4e6f4`
  （local proxy request overrides）未移植特性，8673e9d8 只改 apiFormat 行，这些键
  在其父提交已存在但 fork 基线全缺；隐式移植会越界 S5。
- `bc7f5f41` → `351ae9b0`：已移植，clean cherry-pick。JsonEditor 默认 rows 12→3，
  各表单 editor override（Codex 6/8、Common 14、Gemini 6/8、GrokBuild 12、
  ProviderForm OMO preview + provider/common JSON 3×14）统一降至 3。10 处全部应用。
- `7de63227` → `ce5da1b7`：已移植，clean cherry-pick。GrokBuild form 加
  `glass rounded-xl p-6 border border-white/10` 容器（fork 既有模式，ProviderForm/
  PromptFormPanel/EndpointSpeedTest/McpFormModal 已用）。
- `967daa1a` → `e58491bc`：已移植，clean cherry-pick。Rust `local_hash_for_update_check`
  先验 SSOT 目录存在再信任缓存哈希——换机恢复备份后数据库仍存 content_hash 但 Skill
  文件未随库迁移，缓存哈希会误报「无更新」掩盖缺失。目录缺失返回 None → 进入更新
  列表 → `update_skill` 重建（已容忍缺失目录）。4 个回归测试覆盖 missing-dir /
  cache-hit / cache-empty backfill / invalid-directory，全绿。

### 整提交跳过（proven inapplicable，3 提交）

- `7e5007d5` fix(claude-desktop): clarify model configuration modes —— 仅改
  fork 不存在的 `ClaudeDesktopProviderForm.tsx`（675 行 + en locale）。
- `619a592c` fix(claude-desktop): align provider form frame —— 同上，仅改
  `ClaudeDesktopProviderForm.tsx` 1 行。
- `390102a2` fix(codex): fill DeepSeek contextWindow in OpenCode Go catalog ——
  fork 无 "OpenCode Go" 预设（grep 全树 0 匹配，38 个 preset name 中无此项），
  且 fork 的 DeepSeek 官方预设已含 `deepseek-v4-pro`/`deepseek-v4-flash` 的
  `contextWindow: 1048576`（3 个 DeepSeek 模型目录条目均有 contextWindow，无缺失）。
  上游修复的 bug 在 fork 不存在。

### 关键决策记录

1. **拓扑顺序纠正**：dispatch 列出 `580a4d7b` 为 S5 第 1 项，但 `git merge-base
   --is-ancestor` 证明 `580a4d7b` 是 `7e152d75`/`076c2744` 的**下游**（依赖其创建
   的 shared ModelDropdown）。按 dispatch 顺序处理会导致 `580a4d7b` 的 Hermes 表单
   `import { ModelDropdown } from "./shared"` 在 typecheck 报错。改为拓扑顺序
   `7e152d75` → `076c2744` → `580a4d7b` 后全部 clean。
2. **shared/ModelDropdown.tsx 前置创建**：fork 从未携带该组件（`2deee109` 在 v3.19.2
   同步被跳过，因仅触 ClaudeDesktopForm）。`7e152d75`/`076c2744`/`580a4d7b` 均依赖它。
   作为 `bcb5ae53`（7e152d75）的前置一并创建（上游最终态 99 行），非新特性移植，
   而是上游既有组件的迟到落地。
3. **ImeSafeInput 保留**：fork S4c（`d9d4a660`→`0277a8e1`）为 model id/name/baseUrl
   三处加了 ImeSafeInput 防 IME 标记文本被父重渲染覆盖。S5 的 `580a4d7b`/`95b95da6`
   重构这些表单时，上游用 `<Input onChange>`，fork 改用 `<ImeSafeInput onValueChange>`
   保留 S4c 回归。合并测试文件保留双方测试用例。
4. **8673e9d8 未引入 customUserAgent/localProxy 键**：cherry-pick 冲突暴露上游
   `8673e9d8` 的父提交已含 `customUserAgent*/localProxy*` 键（来自 `6fd4e6f4` 未移植
   特性），而 fork 基线全缺。仅采用 `8673e9d8` 实际改的 `apiFormat`/`apiFormatHint`
   2 行，不把相邻未移植特性的键隐式带入（越 S5 边界）。

### S5 门禁证据

- 每提交后全量 `pnpm test:unit`：**171 files / 1002 tests passed**（两次确认性
  rerun 一致）。c0050623 checkbox 重写触及 ~14 消费方，全量 unit 无回归。
- `pnpm format:check`、`pnpm typecheck`、`pnpm check:web-routes`
  （missing/methodMismatch/parityFallback 均 0）、`pnpm check:locales`
  （en/ja/zh parity，keys 增至 2510）全绿，每提交后确认。
- `cargo fmt --check`、desktop `cargo check --all-targets --locked`、Web example
  `cargo check --locked --no-default-features --features web-server --example server`
  全绿（Web 仅 67 个既有 standalone shim dead-code warnings）。
- Rust focused：`services::skill::` **44 passed**（含 967daa1a 新增 4 个
  `local_hash_for_update_check` 回归）。
- Web Rust parity：`web_api::` **27 passed**、`dual_runtime_parity::` **3 passed**、
  `web_proxy_lifecycle::` **7 passed**。
- `pnpm test:integration`（批末统一跑，3 次 rerun）：
  - 稳定失败 = PRD 已知 4 个 fixture flake：ProviderList Claude official-seed/
    import-current empty-state **1** 个；SkillsPage default-repo/fixture discovery
    **3** 个（automatic skills.sh fallback empty-state、repo skill install/update、
    skills.sh pagination install）。
  - PromptPanel "creates/edits/enables a Gemini prompt" **合并套件 flake**：3 次
    full rerun 中 2 次失败（500 服务器启动竞争）、1 次通过；**单独隔离 rerun
    3/3 通过**。S5 未触 PromptPanel 组件或 prompt 服务（git log 证实），属同类
    并发 Web 服务器启动 flake，与 PRD 已知 4 项同性质，不阻塞。
  - 最终稳定态：49-50/54 passed，仅剩 4 个 PRD 白名单 flake。

### 残余风险与延期项

- `reasoningLevels` 前端 catalog 编辑器 UI 仍是延期项（继承 S4c，与 Codex Chat
  reasoning 栈同捆）；`967daa1a` 的 Skill SSOT 目录检查不涉及该栈。
- `customUserAgent*/localProxy*` 键（`6fd4e6f4` local proxy request overrides）
  仍是未移植特性——S5 刻意不带入，若未来移植需单独任务。
- Codex Chat/Anthropic reasoning 转换栈仍是既有延期项（`transform_codex_chat.rs`/
  `streaming_codex_chat.rs` 全缺），S5 未触及。
- 下一批：S6 Windows 全量适配（4 提交）。
