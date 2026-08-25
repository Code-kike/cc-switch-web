# Implement — managed OAuth account selection for providers

## 前置确认

- [ ] 分支 `sync/upstream-v3.20.0`；父主体 S1–S8 + 两个已归档子任务（`feat-pi-native-agent`、`feat-codex-alpha-websearch`）均在 HEAD 之前。
- [ ] `a2e22f33` 与 `0455a92c` 在 `product-upstream` remote 本地可达；`7265596a`（S4b 移交测试暂存点）本地可达。
- [ ] `.pi/`/`.pi-subagents/` 不在提交范围。
- [ ] 基线快照（回归对照，取自上一子任务终局 check）：
      `cargo test --lib` **2233** passed / 5 ignored；`cargo test --lib proxy::` **1129**；
      test:unit **173 files / 1044 tests**；Rust parity **37**；
      web-routes **292 commands / 280 routes / 0 gaps**；locales **2637** parity；
      `SCHEMA_VERSION` **17**；test:integration 50/54（4 PRD flakes）。

## 移植方法

逐 hunk selective port。**与前两批不同：基线不对齐**（`proxy.rs` +2517 撞 3661 漂移、`provider/mod.rs` +2831 撞 1705、`codex_config.rs` +699 撞 3396），`--3way` 预期大量冲突 → 先做 W0 纯调研。

门禁命令：
```bash
source ~/.cargo/env
(cd src-tauri && cargo fmt --all -- --check)
pnpm format:check
pnpm typecheck
pnpm check:web-routes        # 必须保持 292/280/0 —— 本提交无新命令
pnpm check:locales
pnpm exec cargo check --manifest-path src-tauri/Cargo.toml
(cd src-tauri && cargo check --no-default-features --features web-server --example server)
cargo test --manifest-path src-tauri/Cargo.toml --lib
pnpm test:unit
cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server -- web_api:: dual_runtime_parity:: web_proxy_lifecycle::
```
末批追加：`pnpm test:integration`、`pnpm build:web`、`pnpm smoke:web-server`。

## 执行批次

### W0 — 纯调研：hunk 归属清单（无生产改动，不 commit 代码）

对三个大漂移文件逐 hunk 判定归属。**不写生产代码、不改测试**，只产出结论。

- [x] `src-tauri/src/services/proxy.rs`（+2517/−285，漂移 3661）：逐 hunk 分类
- [x] `src-tauri/src/services/provider/mod.rs`（+2831/−155，漂移 1705）：逐 hunk 分类
- [x] `src-tauri/src/codex_config.rs`（+699/−2，漂移 3396）：逐 hunk 分类
- [x] 每个 hunk 归入三类之一，并给出证据：
  - **(a) 可移植** —— 落在 fork 存在且语义一致的锚点上（给出锚点符号名 + fork 行号）
  - **(b) 需先补依赖** —— 依赖 fork 缺失的上游符号（列出缺失符号 + 其上游定义位置 + 是否由本提交自身引入）
  - **(c) 丢弃** —— 命中 fork 已裁掉的表面（ClaudeDesktop / 延期 Codex Chat 栈 / zh-TW），给出判定依据
- [x] 产出 **依赖拓扑**：哪些 (b) 类 hunk 互为前置，形成必须遵守的落地顺序
- [x] 顺带确认另外三个中等漂移文件的归属：`services/provider/live.rs`（+470/−8，漂移 712）、`proxy/provider_router.rs`（+131/−17，漂移 465）、`proxy/providers/codex_oauth_auth.rs`（+1053/−52，漂移仅 127 → 预期可直接落地）
- [x] 给出**建议批次切分**（含每批预估行数与前置关系）
- [x] 输出结论供主会话写入本文件的 W1..Wn

**W0 硬约束**：只读调研。允许 `git show` / `git apply --check` / `--3way` 干跑到临时副本以测可行性，但**不得**留下任何工作树改动，不得 commit。

## W0 结果（主会话内联完成，子代理被内容策略拦截）

### `--3way` 可行性（真实仓库内 dry-run，冲突块计数）

| 文件 | 冲突块 | 判定 |
|---|---|---|
| `services/proxy.rs` | **29** | `--3way` 不可用，逐 hunk 手工 |
| `services/provider/mod.rs` | **10** | 手工为主 |
| `codex_oauth_auth.rs` | 5 | 漂移仅 127 行；冲突集中在漂移区，可 `--3way` + 定点解决 |
| `codex_config.rs` | 4 | 可 `--3way` + 定点解决 |
| `provider/live.rs` | 2 | 近直接落地 |
| `provider_router.rs` | 2 | 近直接落地 |

### 冲突分布（按符号，非行号）

**proxy.rs 29 块**：22 个生产冲突集中在 fork S2 `d2b070c9`（never-clobber）与上游改写的碰撞区 —— `hot_switch_provider_inner`(6)、`takeover_live_config_best_effort`(3)、`write_codex_live_verbatim_with_auth_guard`(2)、`rollback_hot_switch_preparation`(2)，其余各 1（`sync_live_config_to_provider`、`takeover_live_configs`、`takeover_live_config_strict`、`restore_live_from_ssot_for_app`、`update_live_backup_from_provider_inner`、`preserve_toml_mcp_servers_from_existing_config`、`write_codex_takeover_live_for_provider`）。**移植红线：fork 的 never-clobber 语义必须保留，不得被上游 hunk 回退。** 剩余 7 个在测试模块。

**provider/mod.rs 10 块**：`import_pi_providers_from_live`(2，fork pi 早返回 vs 上游新增）、`reapply_current_codex_official_live`(1)、`add`(1)、`update`(2)、`switch`(1)、`sync_current_provider_for_app`(1)、测试(2)。`add`/`update` 冲突正是 **a2e22f33 自身 managed arms 与 0455a92c 移交事务纠缠处**。

### 依赖拓扑（含上游提交时序证据）

`git merge-base --is-ancestor` 证实：**`a2e22f33`（08-16）是 `0455a92c`（08-18）的祖先**（PRD 移交链方向正确，日期看似相反）。因此：
1. `a2e22f33` 的 managed helper（`managed_codex_oauth_account_id`:2323、`preflight_managed_codex_live`:2510、`write_preflighted_or_current_live`:2529、`managed_codex_add_transaction_error`:2570，均在 provider/mod.rs）先落。
2. `0455a92c` 的 add/update 事务后落，依赖上面四个函数。
3. 二者都在 `add`(4909)/`update`(5392/5437) 内 → **必须同批解决**，不能拆两批。

### `codex_oauth_auth.rs` 凭据面枚举（W1 核验清单）

写入点：accounts store 持久化 `id_token`（空串过滤为缺失）、refresh 后更新 `id_token`、managed bundle 写入 live auth.json 前的「R0 不覆盖 N0」重读守卫、写入前 live re-read 校验。
日志：全 diff 仅 **1 条** `log::warn!`（+624），只含 `account_id` 与 `{err}`，**无 token**。W1 落地时须复核 `{err}` 的 Display 不携带 token 字段。
`reauth_required = data.id_token.is_none()`（旧账号引导重登）—— fail-closed 契约的锚点。

### `migrate_legacy_codex_official_managed_binding` 幂等性

上游实现**幂等**：完成标记 = 清除 fixed 卡的绑定；`existing_managed` 查找 + `matches_interrupted_codex_official_migration` 允许中断后 **resume**（复用已生成 id 而非新建）；已迁移库重跑返回 `Ok(None)`（绑定已清）。失败路径带完整回滚（fixed 行 / failover 成员 / current / local current / 已建 managed 行）。
fork 回归测试须断言：(a) 连续两次运行不产生第二条 managed 行；(b) 中断态（managed 行已建、fixed 未清）重跑复用同一 id；(c) 回滚失败时错误串包含两者。

### 批次切分（据上确定）

```
W1 凭据与数据面（~2,600 行，独立，无 W2 依赖）
  codex_config.rs(+699) codex_oauth_auth.rs(+1053) live.rs(+470)
  provider_router.rs(+131) dao/providers.rs(+100) commands/auth.rs(+49/-17)
  commands/codex_oauth.rs commands/failover.rs(+98/-7) commands/provider.rs(+23/-8)
  commands/proxy.rs store.rs tray.rs(+40/-2) forwarder.rs(+73/-26)
  providers/codex.rs(+28/-6) copilot_auth.rs(+7/-0) auth.ts(+3/-0)

W2 提供者事务层（~2,900 行，本任务最硬批）
  provider/mod.rs(+2831/-155)：managed helper 四函数 + add/update/switch managed arms
  + migrate_legacy + reapply_current + 0455a92c 移交的 add/update 事务（同批解决纠缠冲突）
  + lib.rs startup 钩子（依赖 provider/mod.rs 的迁移函数）
  ⚠ 保留 fork pi 早返回（import_pi 冲突 2 块）+ 保留 S2 never-clobber 语义

W3 前端最小表面（~2,200 行，Q1 裁定）
  CodexOAuthSection.tsx(+410/-90，漂移 1 行可直接落) ProviderForm.tsx(+260/-77)
  CodexFormFields.tsx 仅补 OAuth 渲染块 + 4 新 prop 接线（不整体对齐 1364 行）
  ClaudeFormFields(+16) CopilotAuthSection(+225/-111) AddProviderDialog(+36/-8)
  EditProviderDialog(+54/-9) ProviderActions(+12/-10) ProviderCard(+153/-32)
  AuthCenterPanel(+46/-4) AuthSettingsPanel(+25 新) FullScreenPanel(+45/-11)
  useManagedAuth(+14) mutations.ts(+25/-6) providerCapabilities(+21/-6)
  codexProviderPresets(+3/-2) i18n en/ja/zh(+32/-1)
  取回 7265596a 的 ProviderForm.codexManagedAccount.test.tsx（10 用例）
  + 前端新测试（ManagedAuthStatusError +86、ProviderCard.codexAccount +214 等）

W4 剩余测试 + 全量门禁（~1,400 行）
  CodexOAuthSection.test(+383) AddProviderDialog.test(+90) EditProviderDialog.test(+128)
  FullScreenPanel.test(+33) useManagedAuth.test(+86) useAddProviderMutation.test(+57)
  其余小测试；test:integration + build:web + smoke + 迁移幂等回归 + 凭据面专项核验
```

W0 结论已由主会话写入。每批门禁全绿 + 单批 commit；`check:web-routes` 恒 292/280/0；`SCHEMA_VERSION` 恒 17。

## W1 落地结果与移交 W2 的项（2026-08-25）

W1 已落 17 文件（+2832/−107），全量门禁绿。以下 W1 清单项经证据判定**移交 W2**，非遗漏：

### 1. `forwarder.rs`(+73/−26) → W2
上游 hunk 修改的是 fork **不存在**的前置面：`validate_codex_official_authorization`、`codex_official_auth_passthrough`、`categorize_proxy_error` 的 codex-official 分支，全部 0 命中（`a2e22f33^` 侧有 8 个文件引用 `requires_openai_auth`，fork 仅 3 个）。

该守卫保护的是「客户端 inbound Authorization 直通上游」路径，fork 无此路径：fork forwarder 在 1381–1410 由 `CodexOAuthManager` **服务端解析**绑定账号 token，并在 1509–1512 **自行注入** `chatgpt-account-id`（取自 provider 绑定）。因此不存在可跨账号复用的 inbound token，账号边界由「注入哪个 token」保证。

落地前提 = 先移植 `a2e22f33^` 的 passthrough 特性（超出本 commit diff），且与 fork「takeover 下阻止 official」策略冲突（策略实现在 `services/proxy.rs:2627` / `services/provider/mod.rs:2808`，均 W1 禁改）。

### 2. `store.rs`(+9/−1) + `commands/codex_oauth.rs`(+7/−4) → W2
两者的全部内容都是上游「去掉外层 `Arc<RwLock<CodexOAuthManager>>`」重构。fork 的 manager 生命周期是 `lib.rs`(desktop) / `examples/server.rs`(web) 构造后注入 `set_runtime_ctx` + `app.manage`/`ApiState`；upstream `store.rs` 改为在 `AppState::new` 内构造，在 fork 会产生**第二个实例**（状态分叉）。锁降级需同时改 `runtime_ctx.rs`/`web_api/state.rs`/`services/proxy.rs`/`lib.rs`/`examples/server.rs`。
W1 保留外层 RwLock，所有新调用点统一用 `.read().await`（manager 内部已由本批新增的 `lifecycle_lock`/`storage_lock` 串行化，语义等价于无外层锁）。

### 3. Codex Official 卡在 takeover 下的 switch carve-out → W2
`commands/proxy.rs` 的 official 阻断保持 fork 原样。上游在此处放开，但 fork 的**承重**阻断在 `ProxyService::hot_switch_provider_inner`(proxy.rs:2627) 与 `ProviderService::switch`(mod.rs:2808) 的 defense-in-depth 副本里，二者 W1 禁改 → 只放开命令层等于零效果且留下未审查的浅层缺口。三处须在 W2 同批放开。

### 4. `set_auto_failover_enabled` 的前置校验 → W2
本批把「Codex Official 账号卡不可入 failover 队列」落为 `Database::add_to_failover_queue_checked`（桌面命令 + web handler 共用），**未**加在裸 `add_to_failover_queue` 上：`services/proxy.rs:3003` 在 `switch_proxy_target` 成功**之后**才补写队列（其 "FIX 4 原子性" 注释），此处报错会造成「切换已生效但 `auto_failover_enabled` 未持久化」。上游把校验放在切换**前**，属 `services/proxy.rs`（W2）。运行时安全网已就位：`ProviderRouter::select_providers_with_config` 跳过账号卡，`get_available_providers_for_failover` 不再把它列为候选。

### 5. W2 需切换的调用点
`write_live_with_common_config`（无 manager 版）保留原语义。`services/provider/mod.rs:2359/2376/2533/2594/2931` 与 `services/proxy.rs:2119` 均持 `&AppState`，W2 须改调 `write_live_with_common_config_for_state`，否则 managed Codex Official 卡写 live 时仍走占位 auth。

### 6. 文档
`docs/guides/codex-deepseek-routing-guide-{en,zh,ja}.md:129` 与 `SECURITY.md:169-171` 需在 W3/收尾同步（前者「official 一律阻止」措辞待 3 号放开后修正；后者需说明 `~/.codex/auth.json` 现在承载 cc-switch 写入的 managed OAuth 凭据）。

### W1 关键适配（与上游不同之处）
- **ADR 0003**：`sync_codex_managed_oauth_live_auth_after_refresh` 的 live auth.json 写入用 `write_json_file_managed`（上游 `write_json_file`）；`CodexLiveFileState` 增 `CodexLiveWriteMode::{ManagedExternal,CcSwitchOwned}`，回滚写入与正向写入同模式（auth/config/catalog = managed，cc-switch marker = 严格拒绝末端符号链接）。
- **`futures::executor::block_on`** 取代 `tauri::async_runtime::block_on`（live.rs 4 处），因 live.rs 在 web 构建中经 `#[path]` 编入且 `tauri` 为 optional。
- **谓词单一定义**：`Provider::{is_managed_codex_official_account_card,is_codex_official_card,supports_failover}` 落在 `provider.rs`（领域层），router/tray/DAO/commands 统一调用；未保留上游 `proxy::providers::is_codex_official_provider` / `provider_router::provider_supports_failover` 两个别名（W2 移植时需把上游调用路径改为领域方法）。
- **`reauth_required` 双运行时**：`commands/auth.rs` 与 `web_api/handlers/auth.rs` 各自的 `ManagedAuthAccount` 均补该字段并从 `GitHubAccount.reauth_required` 映射；`requires_reauth` 仍为 xAI 专用（两字段不可合并）。新增 `web_api::` 断言钉住。
- **凭据删除串行化**：`proxy/runtime_ctx.rs` 的 `remove_managed_account_serialized` / `clear_managed_auth_serialized`（锁序 switch-lock → manager），桌面命令与 web handler 共用。放在 runtime_ctx.rs 是因该文件本就同时 import `ProxyService` 与 `CodexOAuthManager`，不新增依赖边。

## 验证命令汇总

见门禁块。关键额外检查：
- **无新命令**：`pnpm check:web-routes` 保持 292/280/0。
- **无 schema 迁移**：`grep SCHEMA_VERSION src-tauri/src/database/mod.rs` 仍为 17。
- **凭据面**：`grep -rn "id_token" src-tauri/src/` 复核写入点权限与日志脱敏；确认无 token 进入 `log::`。
- **安全上限零退化**：逐条 grep `MAX_RESPONSE_BODY_BYTES`(128 MiB) / `JS_EXECUTION_TIMEOUT`(2s) / `JS_MEMORY_LIMIT_BYTES`(16 MiB) / `JS_MAX_STACK_BYTES`(256 KiB) / `MAX_CODEX_CATALOG_BYTES`(32 MiB) / 五个 `MAX_CITATION_DEDUP_*`。
- **ClaudeDesktop / zh-TW 零回潮**：`grep -rn "ClaudeDesktop\|claude-desktop" src/ src-tauri/src/` 仅剩既有残留；`ls src/i18n/locales/` 仍为 en/ja/zh。
- **延期栈零回潮**：`ls src-tauri/src/proxy/providers/` 不出现四文件；`providers/mod.rs` 无对应 `mod`。

## review gates

- W0 后：hunk 归属清单 + 依赖拓扑经主会话复核，据此确定 W1..Wn，再开工。
- 每批 commit 前：全量门禁全绿（test:unit 必须全绿，非 flake 项）。
- auth 相关批次后：`reauth_required` fail-closed 双用例 + `id_token` 写入路径权限/日志脱敏专项确认。
- 末批后：`migrate_legacy_codex_official_managed_binding` 幂等性专项 + 真实 Web 服务冒烟。
- 全部完成后：**父任务跨子任务集成 review**（三子任务的 Web API parity、安全边界、相互无冲突）+ 统一 changelog 补入三子任务结果 + 合 `main`。

## 风险点与回滚

- **单批失败**：`git reset --hard <上一批 commit>`。
- **最大风险 = 三个大漂移文件的 hunk 对齐**（`proxy.rs` 3661 / `codex_config.rs` 3396 / `provider/mod.rs` 1705）。W0 的作用正是把这个风险前移到调研阶段。若 W0 发现某文件无法安全逐 hunk 对齐，**停下报告**并考虑降级范围，不得猜测插入点。
- **依赖倒置**：`0455a92c` 的 managed-codex 事务依赖 `a2e22f33` 引入的 `preflight_managed_codex_live` 等函数 → 必须先落 `a2e22f33` 的对应 hunk。W0 须把这条依赖显式列入拓扑。
- **凭据泄露**：`codex_oauth_auth.rs` +1053 直接处理 `id_token`。不得只依赖编译与测试通过；须逐条核验写入权限与日志脱敏。
- **数据迁移不幂等**：`migrate_legacy_codex_official_managed_binding` 每次启动都跑，若不幂等会重复绑定。须有幂等回归测试。
- **误注册 Web API 命令**：本提交无新命令，`check:web-routes` 计数是硬约束。
