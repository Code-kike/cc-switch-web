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
