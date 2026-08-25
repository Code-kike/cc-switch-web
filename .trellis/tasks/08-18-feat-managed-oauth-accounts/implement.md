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

### `migrate_legacy_codex_official_managed_binding` 幂等性 —— **作废（W2 裁定 Q3）**

> W0 只读了 `a2e22f33` 单个提交，漏查它与 `v3.20.0` 之间的 `0455a92c`。后者（mod.rs 79+/737−）
> **删除** `migrate_legacy_codex_official_managed_binding` / `matches_interrupted_codex_official_migration`
> / `validate_codex_official_card_identity` 及 lib.rs 的 10 行 startup 钩子；
> `git grep -n migrate_legacy_codex_official_managed_binding v3.20.0` 为空。因此**不移植**，
> 下文仅作历史记录保留。证据链与取代验收见 prd.md「裁定记录 · Q3」。

上游实现**幂等**：完成标记 = 清除 fixed 卡的绑定；`existing_managed` 查找 + `matches_interrupted_codex_official_migration` 允许中断后 **resume**（复用已生成 id 而非新建）；已迁移库重跑返回 `Ok(None)`（绑定已清）。失败路径带完整回滚（fixed 行 / failover 成员 / current / local current / 已建 managed 行）。

### 批次切分（据上确定）

```
W1 凭据与数据面（~2,600 行，独立，无 W2 依赖）
  codex_config.rs(+699) codex_oauth_auth.rs(+1053) live.rs(+470)
  provider_router.rs(+131) dao/providers.rs(+100) commands/auth.rs(+49/-17)
  commands/codex_oauth.rs commands/failover.rs(+98/-7) commands/provider.rs(+23/-8)
  commands/proxy.rs store.rs tray.rs(+40/-2) forwarder.rs(+73/-26)
  providers/codex.rs(+28/-6) copilot_auth.rs(+7/-0) auth.ts(+3/-0)

W2 提供者事务层（~2,900 行）
  provider/mod.rs(+2831/-155)：managed helper 四函数 + add/update managed arms
  + migrate_legacy + reapply_current + 0455a92c 移交的 add/update 事务（同批解决纠缠冲突）
  + lib.rs startup 钩子（依赖 provider/mod.rs 的迁移函数）
  + W1 移交 5：provider/mod.rs:2359/2376/2533/2594/2931 改调 write_live_with_common_config_for_state
  ⚠ 保留 fork pi 早返回（import_pi 冲突 2 块）+ 保留 S2 never-clobber 语义
  ⚠ 三处 official 阻断**本批不动**（W3 同批放开）
  ⚠ **W2 开工后修正**：`migrate_legacy` / `reapply_current` / `lib.rs` 钩子 均**不移植**；
    `switch` managed 事务**列入本批**；update 的 takeover 分支**移交 W3**。
    详见下方「W2 落地结果与裁定」。

W3 代理服务层 + 协同放开三处阻断（~2,600 行，本任务最硬批）
  services/proxy.rs(+2517/-285，29 冲突块，逐 hunk 手工) —— W0 计划遗漏未列入任何批次，据 W1 移交注记归入此处
  + W1 移交 3：commands/proxy.rs + services/proxy.rs:2627 + services/provider/mod.rs:2808
    三处 official 阻断**必须同一 commit 放开**（只放开命令层 = 零效果 + 未审查浅层缺口）
  + W1 移交 4：set_auto_failover_enabled 前置校验移到 switch_proxy_target 之前（proxy.rs:3003 原子性）
  + W1 移交 1：forwarder.rs(+73/-26) 复评 —— 若 fork 无 inbound passthrough 面则维持丢弃并记录
  + W1 移交 5 余项：services/proxy.rs:2119 改调 write_live_with_common_config_for_state
  ⚠ 22 个生产冲突集中在 fork S2 d2b070c9 never-clobber 与上游改写碰撞区 → 保留 fork 语义

W4 前端最小表面（~2,200 行，Q1 裁定）
  CodexOAuthSection.tsx(+410/-90，漂移 1 行可直接落) ProviderForm.tsx(+260/-77)
  CodexFormFields.tsx 仅补 OAuth 渲染块 + 4 新 prop 接线（不整体对齐 1364 行）
  ClaudeFormFields(+16) CopilotAuthSection(+225/-111) AddProviderDialog(+36/-8)
  EditProviderDialog(+54/-9) ProviderActions(+12/-10) ProviderCard(+153/-32)
  AuthCenterPanel(+46/-4) AuthSettingsPanel(+25 新) FullScreenPanel(+45/-11)
  useManagedAuth(+14) mutations.ts(+25/-6) providerCapabilities(+21/-6)
  codexProviderPresets(+3/-2) i18n en/ja/zh(+32/-1)
  取回 7265596a 的 ProviderForm.codexManagedAccount.test.tsx（10 用例）
  + 前端新测试（ManagedAuthStatusError +86、ProviderCard.codexAccount +214 等）

W5 剩余测试 + 全量门禁（~1,400 行）
  CodexOAuthSection.test(+383) AddProviderDialog.test(+90) EditProviderDialog.test(+128)
  FullScreenPanel.test(+33) useManagedAuth.test(+86) useAddProviderMutation.test(+57)
  其余小测试；test:integration + build:web + smoke + 迁移幂等回归 + 凭据面专项核验
```

W0 结论已由主会话写入。每批门禁全绿 + 单批 commit；`check:web-routes` 恒 292/280/0；`SCHEMA_VERSION` 恒 17。

## 跨批次移植约束（W1 后经审阅补入，W2/W3/W4 必须遵守）

### C1 —— managed 外部配置写入必须走 managed 原子写（ADR 0003）

`~/.codex/auth.json`、`config.toml`、catalog 属**外部应用拥有**的托管配置。新增写点一律经 `crate::config::write_json_file_managed` / `write_text_file_managed`（内部 `AtomicWriteMode::FollowManagedSymlink`），跟随合法符号链接并原子替换其目标 —— 否则会砸掉 dotfiles / NixOS 的符号链接布局。

**不得**复制 `codex_oauth_auth.rs:1492` 的 `write_store_atomic`（裸 `fs::rename`）到任何 auth.json 写点；该函数的正当作用域仅限 data_dir 内的 `~/.cc-switch/codex_oauth_auth.json`（唯一调用点 :1588）。

> 移植方向提示：**上游用的是严格版 `write_json_file`（`RejectFinalSymlink`）**。W1 在 `codex_config.rs:715/792` 改用 `_managed` 是**正确的 fork 适配**（对齐 fork 基线既有的 190/1277 用法与 ADR 0003），不是安全放宽。W2/W3 遇到同类写点时同样按 fork 侧改写，不要「忠于上游」改回严格版。

### C2 —— `tauri::async_runtime` 可否照搬：按文件是否进 web 构建逐个核实

fork 的 `tauri` 是 optional 依赖，**但这不等于「`async_runtime` 仅限 `commands/` 与 `lib.rs`」**（该前提已被证伪：`tray.rs` 4+1 处、`linux_fix.rs`、`services/s3_auto_sync.rs`、`services/webdav_auto_sync.rs` 均在用，其中 `tray.rs` 就在 W1 清单内且保留了上游用法）。

判定方法 —— 对每个待移植文件核实其是否进入 web 构建：

```bash
grep -nE 'mod <name>|#\[path.*<name>' src-tauri/examples/server.rs
```

已核实：`codex_config.rs`(:143 `#[path]`)、`provider.rs`(:187 `#[path]`) **在** web 构建内；`live.rs` / `provider_router.rs` / `codex_oauth_auth.rs` 经父模块传递编入；`tray.rs` **不在**（`mod tray` 计数 0）→ 其 `async_runtime` 用法可照搬。

进 web 构建的文件里，上游的 `tauri::async_runtime::block_on` 改用 `futures::executor::block_on`（fork 在 `live.rs:1352/1364` 的既有惯用法）。

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

**W2 补充**：`docs/guides/claude-codex-routing-guide-{en,zh,ja}.md:65` 的「登录凭据保存在
`~/.cc-switch/codex_oauth_auth.json`（不是 `~/.codex/`），与 Codex CLI 自己的登录互不影响」同样需修正：
W1 的 `codex_config.rs:682 sync_codex_managed_oauth_live_auth_after_refresh` 会把所选托管账号的
bundle（含 `id_token`）写入 Codex live `auth.json`，所以「不写 `~/.codex/`」在**托管 Codex Official
卡**路径下不再成立。措辞建议：保留「凭据存储位置仍是 `~/.cc-switch/codex_oauth_auth.json`」，
只把「不写 `~/.codex/`」限定到 Claude 侧 codex_oauth 预设路径。与 `SECURITY.md:169-171` 是同一事实的两处记载，
一并同步。**本批不改的理由**：账号选择的用户可见面在 W4 才落地，现在改会先于实现描述行为。

### W1 关键适配（与上游不同之处）
- **ADR 0003**：`sync_codex_managed_oauth_live_auth_after_refresh` 的 live auth.json 写入用 `write_json_file_managed`（上游 `write_json_file`）；`CodexLiveFileState` 增 `CodexLiveWriteMode::{ManagedExternal,CcSwitchOwned}`，回滚写入与正向写入同模式（auth/config/catalog = managed，cc-switch marker = 严格拒绝末端符号链接）。
- **`futures::executor::block_on`** 取代 `tauri::async_runtime::block_on`（live.rs 4 处），因 live.rs 在 web 构建中经 `#[path]` 编入且 `tauri` 为 optional。
- **谓词单一定义**：`Provider::{is_managed_codex_official_account_card,is_codex_official_card,supports_failover}` 落在 `provider.rs`（领域层），router/tray/DAO/commands 统一调用；未保留上游 `proxy::providers::is_codex_official_provider` / `provider_router::provider_supports_failover` 两个别名（W2 移植时需把上游调用路径改为领域方法）。
- **`reauth_required` 双运行时**：`commands/auth.rs` 与 `web_api/handlers/auth.rs` 各自的 `ManagedAuthAccount` 均补该字段并从 `GitHubAccount.reauth_required` 映射；`requires_reauth` 仍为 xAI 专用（两字段不可合并）。新增 `web_api::` 断言钉住。
- **凭据删除串行化**：`proxy/runtime_ctx.rs` 的 `remove_managed_account_serialized` / `clear_managed_auth_serialized`（锁序 switch-lock → manager），桌面命令与 web handler 共用。放在 runtime_ctx.rs 是因该文件本就同时 import `ProxyService` 与 `CodexOAuthManager`，不新增依赖边。

## W2 落地结果与裁定（2026-08-25）

### 移植基准修正：按 `a2e22f33^..v3.20.0` 的**净 diff**，不是 `a2e22f33` 单提交

`git log --oneline 413c09e0..v3.20.0 -- src-tauri/src/services/provider/mod.rs` → 区间内仅
`84e75ad2`（已归档子任务）、`a2e22f33`、`0455a92c`。因此目标态 = v3.20.0，净 diff
**+2188/−150**（而非 `a2e22f33` 单提交的 +2831/−155 再被 `0455a92c` 删 737 行）。
`git apply --3way` 净 diff → **11 冲突块**。

行数拆解（修正 W0 的「~2,900 行」估算）：测试 **+1494**（上游 2302→3796 行，占 73%），
生产 `impl` 区 **+520**（上游 2334→2854）。fork 的 `impl` 区 2590 → 约 3110，全文件约 6.7k
（与上游 6778 同量级）。

### 逐冲突块处置（11 块）

| 冲突位置 | 处置 | 依据 |
|---|---|---|
| `pub(crate) use live::{...}` 重导出列表 | 合并：保留 fork 的 `sanitize_claude_settings_for_live` + `write_live_with_common_config`，补 `write_live_with_common_config_for_state` | fork 侧两个符号上游无；`_for_codex_oauth_manager` 不需在 mod.rs 重导出 |
| `official_provider_supports_proxy_takeover` + `reapply_current_codex_official_live`（整块） | **丢弃**（保留 fork 的删除） | 两者均存在于 `a2e22f33^`，属**已有基线漂移**而非本提交 hunk。前者是 takeover 下放开 official 的使能开关（W3）；后者唯一调用方是 `commands/settings.rs:82` 的「统一会话开关」特性，fork 无此特性（`grep unified_session` 零命中）→ 移植即死代码 |
| 测试 `use` 列表 | 取 `AuthBinding, AuthBindingSource`；**丢弃** `UsageScript` 与 `ClaudeDesktopMode/ClaudeDesktopModelRoute` | 前两个为本批新测试所需；后者属漂移/ClaudeDesktop 已裁表面 |
| 测试 helper 块 | 只取 `managed_codex_provider`；**丢弃** `codex_settings` / `usage_script_with_credentials` / `codex_provider_with_usage` | 后三个在 `a2e22f33^` 已存在，属漂移，fork 测试不用 |
| 测试模块尾部三处文本撞车 | 不用 `--3way` 结果，改为从 v3.20.0 提取 17 个新测试单独适配后追加 | `--3way` 把 fork 测试体（hermes 种子）与上游新测试交织，不可审查 |
| `add` / `update` / `switch` 三处生产冲突 | 手工逐 hunk，保留 fork 周边（Hermes 分支、OMO 分支、pi 早返回） | 见下「语义保留」 |

### 已确认无回退的 fork 语义

- **pi 早返回**：`import_pi_providers_from_live`（mod.rs:35）仍为 `pi::import_from_live(state)` 单行委派；`add`/`update`/`switch`/`delete` 头部的 `if app_type == AppType::Pi { return pi::...; }` 未被任何上游 hunk 触及（managed 分支全部置于 pi 早返回**之后**）。
- **S2 `d2b070c9` never-clobber**：本批**未修改** `services/proxy.rs`（never-clobber 的实现位置），也未改 `provider/mod.rs:2808` 的 official 阻断。新增的 managed 事务均经 `CodexLiveStateSnapshot::restore_preserving_newer_same_account_auth()` 回滚（W1 落地，语义就是「不覆盖更新的同账号 auth」）。
- **三处 official 阻断**：`commands/proxy.rs` / `services/proxy.rs:2627` / `services/provider/mod.rs:2808` 均保持关闭。

### 本批内的两项范围调整（均需记录）

**调整 1 —— `switch` 的 managed 事务列入本批**（批次行在 `36a7e4b3` 被缩成「add/update managed arms」）。
理由：它只依赖 W1 已落地的 `CodexLiveStateSnapshot` / `prepare_codex_managed_oauth_live_auth_switch_away` /
`clear_codex_live_auth_for_managed_account*`，**无 `services/proxy.rs` 依赖**；而 17 个新测试中有 5 个
（`switch_*` / `managed_codex_switch_*` / `switch_away_*`）直接驱动它。若置后，`add`/`update` 已能写入
托管凭据而「激活托管卡」的主路径仍写占位 auth —— 半工作态比一次性落地风险更大。

**调整 2 —— `update` 的 takeover 分支移交 W3，本批 fail-closed**。
缺的是两个 `services/proxy.rs`（W3）符号，不在本批范围：

| 上游符号 | v3.20.0 | fork 现状 |
|---|---|---|
| `sync_codex_live_from_provider_while_proxy_active_guarded` | proxy.rs:719 `pub(crate)` | **不存在**（fork 仅有无守卫版 proxy.rs:406） |
| `update_live_backup_from_provider_inner` | proxy.rs:2801，`pub(crate)`，**4 参**（多 `clear_codex_auth_for_account: Option<&str>`） | proxy.rs:2520，私有，**3 参** |

后者正是 W0 列出的 22 个 proxy.rs 生产冲突之一 → 本批改它必在 W3 重新合一次。
降级方向选 **fail-closed**（`provider.codex.managedOfficial.takeoverUpdateUnsupported`）：若改为
落回 fork 旧路径，会把存储的占位 auth 写进 takeover 备份并跳过 compare-before-write 守卫
（CLI 轮换后的登录会被覆盖）—— 账号绑定属授权决策，按 spec「Degradation Direction」应 fail-closed。

**因此上游测试 `managed_codex_takeover_update_db_failure_restores_backup_live_and_binding`
（150 行）延至 W3**（唯一依赖 takeover 分支的新测试；其取据 = 测试体使用 `save_live_backup`）。
本批代以 fork 自有用例 `managed_codex_update_under_takeover_fails_closed_until_proxy_batch` 钉住
fail-closed 行为，W3 落地时用上游用例取代。其余 **16** 个新测试本批全量移植并全绿。

### 测试适配记录（逐项，供验收比对）

| 适配 | 范围 | 取据 |
|---|---|---|
| `tauri::async_runtime::block_on` → `futures::executor::block_on` | 16 个测试全部 | C2：`provider/mod.rs` 经 `examples/web_services.rs:43` 的 `#[path]` 进 web 构建，**且其 `#[cfg(test)]` 模块会被 web example 的 test 构建编译**（已验证：`cargo test --example server` 曾因此报 `NoopEventSink` 未找到） |
| `state.codex_oauth_manager.X()` → `codex_oauth.read().await.X()` + 新增 `with_codex_oauth_test_home` 验证台 | 16 个测试中用到 manager 的 14 个 | fork 无 `AppState::codex_oauth_manager` 字段，manager 在 proxy runtime ctx（W1 保留外层 `Arc<RwLock<_>>`，移交项 2）；验证台按 `lib.rs`/`examples/server.rs` 同样方式 `set_runtime_ctx` |
| `crate::commands::remove_codex_oauth_account_with_switch_lock` → `proxy::runtime_ctx::remove_managed_account_serialized`；`logout_codex_oauth_with_switch_lock` → `clear_managed_auth_serialized` | `codex_auth_center_*` 3 个 | W1 已把锁序守卫落在 `runtime_ctx.rs`（桌面命令与 web handler 共用），而非上游的命令层函数 |
| `ProviderService::managed_codex_oauth_account_id` → 非限定自由函数 | `update_keeps_official_provider_id_when_binding_and_unbinding` | helper 已移入 `managed_codex.rs` |
| **删去 unified-session-bucket 注入**（下详） | 同上 1 个测试 | fork 无该特性 |

**unified-session-bucket 适配的完整取据与影响**：上游该测试先向 `unbound.settings_config["config"]`
注入 `codex_config::inject_codex_unified_session_bucket("")`，再断言 unbind 后 `config == ""`，
以覆盖「unbind 剔除 live-only 统一会话路由」。fork 两侧都没有：
`git grep strip_codex_unified_session_bucket_from_settings a2e22f33^` → 存于 `codex_config.rs`/`live.rs`（即
**本提交之前就已存在**，属基线漂移），而 fork `grep -rn unified_session src-tauri/src` 零命中。
因此：
- 本批**丢弃**注入与 `update` 里对应的 `strip_codex_unified_session_bucket_from_settings` 调用 hunk（分类 (c)）。
- 保留的 `assert_eq!(saved_unbound.settings_config["config"], json!(""))` 在 fork 下退为**空断言**
  （`unbound` 本就以 `config: ""` 构造）—— 已在测试内注释声明。该测试其余断言（绑定/解绑
  保持 provider id、DB current、本地 current、绑定存在性）**未弱化**，且它才是本测试在
  `0455a92c` 里的立意（取代 `migrate_legacy` 的那两个新用例之一）。
- 待补：若未来移植 unified-session 特性，需同时恢复该注入与 `update` 的 strip 调用。

### 架构：新增 `provider/managed_codex.rs`

10 个 managed helper（上游作为 `impl ProviderService` 顶部的连续 216 行块）移入新模块，与同
目录既有分层（`live.rs`/`pi.rs`/`usage.rs`/`endpoints.rs`/`gemini_auth.rs`）一致。名字与上游逐
字相同、mod.rs 以 `use managed_codex::{...}` 非限定调用 → 未来上游 hunk 只需去掉 `Self::` 前缀。
**不**移的部分：`add`/`update`/`switch` 内的 managed arms（嵌在 fork 已漂移的函数体中，无论如何都要原地手合），
以及 16 个新测试（全部依赖 mod.rs 测试模块里的 `with_test_home`/`TempHome` 验证台；兄弟文件无共享验证台惯例）。

### 谓词单一定义（审阅意见采纳）

新增 `Provider::managed_codex_oauth_account_id()`（provider.rs，trim+非空，不含 category 门），
`is_managed_codex_official_account_card` 改为在其上加 category 门；`managed_codex.rs` / `live.rs:833` /
`tray.rs:258` 统一调用，消除三份重复派生。
三处**有意保留**裸 `.is_some()`（`live.rs:1170` backfill 剔除、`live.rs:1452` managed auth 记录、
`forwarder.rs:1390` token 取用）：前两处把空串绑定当作托管会剔除/记录而非持久化，是该路径的
**fail-safe** 方向；第三处属 forwarder.rs（W1 已定丢弃、W3 复评）。已在 provider.rs 的 doc 记录原因。

`switch` 的 outgoing 账号推导同样改调 `outgoing_managed_codex_oauth_account_id`（上游在 `switch`
里内联重算），使 `AppType::Codex` 门控只有一份。**这在 fork 是必需的**：`codex_oauth` 也是
Claude 侧 provider type（`Provider::is_codex_oauth`、`proxy/providers/claude.rs:48`、
`live.rs:91 apply_codex_oauth_claude_context_defaults`），所以 Claude provider 可带
`authBinding[codex_oauth]`（PRD 明列 `appId="claude"` + `providerType: "codex_oauth"` 路径）。
上游不门控无害（它读永远存在的 `AppState::codex_oauth_manager`），但 fork 的
`prepare_..._for_state` 在无 runtime ctx 时 **fail-closed** → 不门控会让一次普通 Claude 切换直接失败；
且 `clear_outgoing_...` 只在 Codex 门控的事务分支内调用，预备态无人收尾。
回归用例：`claude_switch_off_a_codex_oauth_bound_provider_never_touches_codex_live`（故意用
不注入 ctx 的 `with_test_home`）。**变异验证**：临时抽掉 helper 首行的 Codex 门控 → 该用例
FAILED（panic at mod.rs 的 switch）；恢复后通过。

### 另两项审阅修正

1. **删除死条件**：`clear_stale_codex_live_auth_after_official_switch` 的守卫本欲按上游扩宽为
   `category == "official" || is_codex_official_provider(provider)`。上游该谓词（v3.20.0
   `proxy/providers/codex.rs:265`）会按 settings 形状识别「category 非 official 但实为 official」的卡，
   fork 有意未移植（W1 改用领域谓词）；而 `Provider::is_codex_official_card` 首行就要求
   `category == Some("official")`，严格窄于第一个分支 → `A || (A && …) ≡ A`。故只保留原条件 +
   新增的 `target_managed_codex_account_id.is_none()` 豁免，并在注释里记下未移植的形状识别扩宽。
2. **`NoopEventSink` 门控位置修正**：`runtime_events.rs` 的 `NoopEventSink` 体为空、不依赖 tauri，
   却与 `TauriEventSink` 一同被 `#[cfg(feature = "desktop")]` 门控，致使 web example 的 test 构建拿不到它。
   改为 `#[cfg(any(feature = "desktop", test))]`（含 `runtime/mod.rs` 的 re-export）—— 两个测试构建都能用，
   而非 test 的 web 构建仍门控掉，**警告数不变**（实测：完全不门控 = +2 条 dead-code 警告，
   `any(desktop, test)` = 70 条与基线相同）。比在测试模块里另造一份空 sink 更少重复。

### 移交 W3 的项（累计）
1. `services/proxy.rs` 全量（+2517/−285，29 冲突块）。
2. 上述两个符号 + `update` takeover 分支 + 上游测试 `managed_codex_takeover_update_db_failure_restores_backup_live_and_binding`。
3. `managed_codex_takeover_transaction_error`（上游 helper 之一，仅 takeover 分支使用，本批不落地以免死代码）。
4. `write_live_with_common_config` 的最后一个调用方 `ProxyService::restore_live_from_ssot_for_app`（proxy.rs:2119）—— 它持 `&self`/`self.db` 而**非** `&AppState`，切换需先取得 AppState 或另开入口。
5. W1 移交项 1/2/3/4 仍在 W3（forwarder.rs 复评、去掉外层 `Arc<RwLock<CodexOAuthManager>>`、三处 official 阻断同批放开、`set_auto_failover_enabled` 前置校验前移）。

### 不移植（详见 prd.md「裁定记录 · Q3」）

`migrate_legacy_codex_official_managed_binding` / `matches_interrupted_codex_official_migration` /
`validate_codex_official_card_identity` / lib.rs startup 钩子 / 其 4 个上游测试 —— 均被 `0455a92c`
删除，v3.20.0 全树无此符号。因此 **`lib.rs` 本批零改动**。
若将来确需 startup 数据迁移，落点是 `bootstrap::run_post_db_bootstrap`（双运行时共用扩展点，
bootstrap.rs:170，lib.rs:494 + examples/server.rs:335），**不是** `lib.rs::setup()`（上游形状，仅桌面执行
→ 违反双运行时等价）。

### 方法论缺口备案：W0 未对齐目标 tag

W0 只**孤立地**调研了 `a2e22f33` 单个提交，没有逐项核对「这个交付物在目标 tag `v3.20.0`
上是否仍然存在」。正因如此，`migrate_legacy_codex_official_managed_binding` 被当作交付物
写进了 PRD 验收标准、design 数据流与 implement W2 批次行，直到 W2 开工才发现它在区间内的
下游提交 `0455a92c` 被删除（v3.20.0 零命中）。

**修正规则（后续批次与同类移植任务适用）**：移植的是**提交区间**（`base..target`）而不是单个
提交时，每个拟定交付物必须在规划阶段先对着**目标 tag** 验证存在性（`git grep -n <symbol>
<target-tag>`），再写入验收标准与批次表 —— 中间提交引入的符号可能在下游提交里被删除，孤立
读单个提交的 diff 无法发现这一点。

## W3 落地结果（2026-08-25）

13 文件 +2521/−358。子代理中途被截，主会话收口：补 `seed_codex_model_template` 测试 helper、按任务契约打开 managed Official carve-out。

### Codex carve-out：比上游窄，只放开 managed Official

`Provider::blocked_by_proxy_takeover` 是四处分发点的单一定义（命令 / `hot_switch_provider_inner` / `ProviderService::switch` / tray）。

- **打开**：`app_type == "codex" && is_managed_codex_official_account_card()` —— fork 能服务：`CodexAdapter` 返回 `AuthStrategy::CodexOAuth` 占位，forwarder 按 strategy 解析绑定账号 token 并注入 `chatgpt-account-id`。
- **保持关闭（fail-closed）**：unbound 原生登录 Official 卡。fork 没有上游的 inbound Authorization passthrough（`codex_official_auth_passthrough` / `validate_codex_official_authorization`），forwarder **替换** inbound Authorization。打开它们会把切换时的明确拒绝变成一次失败的 Codex 会话。
- Claude Official 仍一律拦截（carve-out 仅 Codex 作用域）。Claude 侧 `codex_oauth` 预设是 `third_party`，不受此谓词影响。

出站路径（CodexAdapter，按本文件 xAI 模式，不复制 ClaudeAdapter 的 ChatGPT 协议）：
- `extract_base_url` 对 managed Official 钉死 `CHATGPT_CODEX_BASE_URL`
- `extract_auth` 返回 `CodexOAuth` 占位（**不**依赖 `meta.provider_type`，测试卡与生产卡都只靠 `auth_binding`）
- `build_url` 在钉死 origin 下保留客户端 path（`/responses`、`/responses/compact`、`/alpha/search`），不强制 `/responses`
- `get_auth_headers` 成对发送 `originator`+`version`

`CODEX_OAUTH_ORIGINATOR` / `CODEX_OAUTH_CLIENT_VERSION` 从 `claude.rs` 私有常量提升到 `proxy/providers/mod.rs`（与 `CHATGPT_CODEX_BASE_URL` 并列），两处适配器共用，避免双份漂移（成对缺一即 404）。

钉住测试：
- `blocked_by_proxy_takeover_opens_only_managed_codex_official_cards`
- `managed_official_card_pins_chatgpt_origin_and_codex_oauth_strategy`
- `unbound_official_card_has_no_server_side_credential`

### 其余 W3 范围

- `services/proxy.rs` 净移植（never-clobber 测试仍在：`codex_*_preserves_oauth_auth_json*`）
- `update` takeover 分支落地：`sync_codex_live_from_provider_while_proxy_active_guarded`、`update_live_backup_from_provider_inner` 第 4 参、`managed_codex_takeover_transaction_error`；W2 过渡用例被上游 `managed_codex_takeover_update_db_failure_restores_backup_live_and_binding` 取代
- `set_auto_failover_enabled` 前置校验经 `Database::ensure_provider_supports_failover`（切换前拒绝，队列写入仍在切换成功后）
- `restore_live_from_ssot_for_app` 改调 `write_live_with_common_config_for_codex_oauth_manager`；无调用方的 `write_live_with_common_config` 已删
- `forwarder.rs` inbound passthrough **维持丢弃**（fork 无此面；账号边界由「注入哪个 token」保证）
- 外层 `Arc<RwLock<CodexOAuthManager>>` **永久分叉保留**（去掉会在 `AppState::new` 构造第二份 manager）
- C1：live auth.json 写入走 `write_json_file_managed`；新增 `resolve_managed_write_path` 供 no-clobber rename 协议打到解析后的目标，避免砸掉符号链接
- C2：web 构建文件无新增 `tauri::async_runtime`

### 门禁（W3）

`cargo test --lib` **2288** passed / 5 ignored（基线 2273；+16 量级，含 3 个新 carve-out/adapter 测试）。全量跑中 `database::tests::sql_import_*` 偶发 `创建数据库安全备份失败: not an error`，隔离重跑即过，属既有 flake，非本批引入。
`proxy::` **1153**（基线 1138）。parity **38**。test:unit **173 / 1044**。locales **2637**。web-routes **292/280/0**。`SCHEMA_VERSION` **17**。fmt / typecheck / web-server example check 全绿。延期栈四文件仍缺席；`migrate_legacy_codex_official_managed_binding` 仍缺席。

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
- 末批后：~~`migrate_legacy_codex_official_managed_binding` 幂等性专项~~（**作废，W2 裁定 Q3**；改为确认该符号不存在于 fork 树）+ 真实 Web 服务冒烟。
- 全部完成后：**父任务跨子任务集成 review**（三子任务的 Web API parity、安全边界、相互无冲突）+ 统一 changelog 补入三子任务结果 + 合 `main`。

## 风险点与回滚

- **单批失败**：`git reset --hard <上一批 commit>`。
- **最大风险 = 三个大漂移文件的 hunk 对齐**（`proxy.rs` 3661 / `codex_config.rs` 3396 / `provider/mod.rs` 1705）。W0 的作用正是把这个风险前移到调研阶段。若 W0 发现某文件无法安全逐 hunk 对齐，**停下报告**并考虑降级范围，不得猜测插入点。
- **依赖倒置**：`0455a92c` 的 managed-codex 事务依赖 `a2e22f33` 引入的 `preflight_managed_codex_live` 等函数 → 必须先落 `a2e22f33` 的对应 hunk。W0 须把这条依赖显式列入拓扑。
- **凭据泄露**：`codex_oauth_auth.rs` +1053 直接处理 `id_token`。不得只依赖编译与测试通过；须逐条核验写入权限与日志脱敏。
- ~~**数据迁移不幂等**：`migrate_legacy_codex_official_managed_binding` 每次启动都跑，若不幂等会重复绑定。须有幂等性回归测试。~~ —— **作废（W2 裁定 Q3）**：不移植该迁移，本任务无 startup 数据迁移。取代风险项：**误移植上游已删代码** —— 移植它会在下次启动把 fixed `codex-official` 行搬到新 UUID（`save_provider` 新行 + 双端 `set_current_provider` + `remove_from_failover_queue`），而 v3.20.0 的取代实现明确保持 id 不变。
- **误注册 Web API 命令**：本提交无新命令，`check:web-routes` 计数是硬约束。
