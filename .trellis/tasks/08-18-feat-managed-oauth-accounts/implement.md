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

- [ ] `src-tauri/src/services/proxy.rs`（+2517/−285，漂移 3661）：逐 hunk 分类
- [ ] `src-tauri/src/services/provider/mod.rs`（+2831/−155，漂移 1705）：逐 hunk 分类
- [ ] `src-tauri/src/codex_config.rs`（+699/−2，漂移 3396）：逐 hunk 分类
- [ ] 每个 hunk 归入三类之一，并给出证据：
  - **(a) 可移植** —— 落在 fork 存在且语义一致的锚点上（给出锚点符号名 + fork 行号）
  - **(b) 需先补依赖** —— 依赖 fork 缺失的上游符号（列出缺失符号 + 其上游定义位置 + 是否由本提交自身引入）
  - **(c) 丢弃** —— 命中 fork 已裁掉的表面（ClaudeDesktop / 延期 Codex Chat 栈 / zh-TW），给出判定依据
- [ ] 产出 **依赖拓扑**：哪些 (b) 类 hunk 互为前置，形成必须遵守的落地顺序
- [ ] 顺带确认另外三个中等漂移文件的归属：`services/provider/live.rs`（+470/−8，漂移 712）、`proxy/provider_router.rs`（+131/−17，漂移 465）、`proxy/providers/codex_oauth_auth.rs`（+1053/−52，漂移仅 127 → 预期可直接落地）
- [ ] 给出**建议批次切分**（含每批预估行数与前置关系）
- [ ] 输出结论供主会话写入本文件的 W1..Wn

**W0 硬约束**：只读调研。允许 `git show` / `git apply --check` / `--3way` 干跑到临时副本以测可行性，但**不得**留下任何工作树改动，不得 commit。

### W1..Wn — 待 W0 结论后填写

（W0 完成后由主会话据 hunk 归属清单与依赖拓扑补写。）

预置的跨批次红线（无论如何切分都适用）：
- [ ] **无新命令**：`check:web-routes` 每批保持 292/280/0。本提交不触 `web-commands.ts`（已证实不在 54 文件内），计数变化即误引入。
- [ ] **无 schema 迁移**：`SCHEMA_VERSION` 保持 17。`database/` 仅 `dao/providers.rs`。
- [ ] `migrate_legacy_codex_official_managed_binding` 幂等（重复执行不产生第二条绑定），失败只 `log::warn!` 不阻断启动。
- [ ] **auth 降级方向 = fail-closed**（spec「Degradation Direction」）：`reauth_required` 账号不得保存；无法认证的绑定不得持久化。两个移交用例钉住此点。
- [ ] **managed `id_token` 写入不扩大凭据面**：走 fork 既有 atomic + 0600 路径；日志不含 token；逐条核验而非只看编译通过。
- [ ] ClaudeDesktop hunk 全量丢弃（含 `ClaudeDesktopProviderForm.tsx` +48/−10 与其测试 +122）；zh-TW 丢弃。
- [ ] 延期栈四文件仍缺席；若某函数被证明必需，按 spec「Deferred Upstream Stack — Private-Helper Exception」处理（私有 fn + 无新 `mod` + 变异验证必需性）。
- [ ] 安全上限零退化：128 MiB body / 2s / 16 MiB / 256 KiB / 32 MiB + 五个 `MAX_CITATION_DEDUP_*`。
- [ ] S4b 移交测试 `ProviderForm.codexManagedAccount.test.tsx` **10 用例**全绿（`git show 7265596a:tests/components/ProviderForm.codexManagedAccount.test.tsx` 取回）。
- [ ] 前端按 Q1 裁定：只补测试断言到的最小表面（`CodexFormFields` 的 `CodexOAuthSection` 渲染块 + 4 个新 prop 双侧接线 + `ProviderForm` 推导），**不整体对齐上游 1364 行**。
- [ ] 上游测试全量移植、不删测试、不弱化断言。

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
