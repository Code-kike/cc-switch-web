# Implement: v3.20.0 Web 全量回归审阅与修复

## Phase A: Freeze Baseline And Characterize Runtime

- [ ] 生成 `review-scope.txt`：`25e34700..f0b468ad` 的 302 文件，标记 256 source/test。
- [ ] 建立 `review-findings.md` ledger 与 coverage 自动校验。
- [ ] 记录基线：systemd 状态、build、schema、DB integrity、route count、安全上限。
- [ ] Chrome DevTools 巡检桌面 + 移动视口全部顶层页面；记录 console/network/404/CSP。
- [ ] 用 API/DB/日志复现 Codex 118 deferred / 518 文件 / 每分钟 WARN 放大。
- [ ] 跑任务前完整门禁，区分**基线失败 / 环境 fixture flake / 新 finding**。

## Phase B: Systematic Review

### B1 Parent S1–S8

- [ ] S1/S5：UI、preset、i18n、startup FOUC、app navigation。
- [ ] S2：`backup.rs`、`sync_protocol.rs`、`import_export.rs`、`schema.rs`、proxy never-clobber。
- [ ] S3：pricing preset/seed/repair/lookup + usage totals/trends/detail/rollup。
- [ ] S4：provider/Codex/OAuth/config TOML/IME/model reasoning。
- [ ] S5：skills/prompt/forms/shared UI。
- [ ] S6：`tool_version.rs`、Windows registry、WSL atomic write、WiX。
- [ ] S7：CI path filter（只查会导致门禁漏跑的功能缺陷）。
- [ ] S8：version/release/runtime build。

### B2 Pi

- [ ] additive CRUD、models revision/原子写、current state/error gate。
- [ ] native prompts/templates/AGENTS activation。
- [ ] skill path alias/symlink/preserve/remove/migrate。
- [ ] session discovery/load/delete + usage parse/dedup/status/cost。
- [ ] Web handlers / UI route parity。

### B3 Alpha Search / WebSearch

- [ ] full URL `strip_suffix` fail-closed + 四别名路由。
- [ ] hosted WebSearch fail-closed 矩阵、流式/非流式、`max_uses`、usage 计数。
- [ ] markdown/citation 五上限覆盖**全部**入口。

### B4 Managed OAuth

- [ ] reauth / account selection / dialog / error gate。
- [ ] managed auth 写入/回滚/symlink/权限/日志脱敏。
- [ ] Rust takeover predicate 五消费者 + 前端镜像。
- [ ] unbound native-login Official 保持 blocked；managed Official 可路由。

### B5 Cross-Cutting

- [ ] 292 commands / 280 routes / 0 gaps，method/body/extractor 一致。
- [ ] AppType/AppId exhaustive dispatch；desktop-only Web stub 合理。
- [ ] 安全上限、延期栈、ClaudeDesktop/zh-TW/.pi 排除项。
- [ ] coverage ledger 达 302/302；finding 按 severity 排序。

## Phase C: Fix In Risk Order

- [ ] **C0**：可复现的 Web 核心阻断、数据损坏、凭据覆盖、无限循环/资源放大。
- [ ] **C1**：Codex deferred 收敛与调度/日志。顺序固定：
      1. characterization tests 锁定现行为；
      2. 纯搬迁 commit —— SQL/reset/cursor 移入 `database/dao/session_usage_codex.rs`
         （零行为变化，测试不变即通过）；
      3. 语义修复 commit —— deferred 收敛 + 增量调度 + 日志有界，附新回归测试。
- [ ] **C2**：restore/sync/pricing/aggregation 等数据正确性 finding。
- [ ] **C3**：provider/Pi/Alpha/OAuth 跨层 workflow finding。
- [ ] 每个 finding 补回归测试，并在 `review-findings.md` 记 before/after 证据。
- [ ] 每批跑 focused tests + fmt/typecheck；独立 commit，禁混入无关重构。

## Phase D: Full Verification

```bash
source ~/.cargo/env
(cd src-tauri && cargo fmt --all -- --check)
pnpm format:check
pnpm typecheck
pnpm check:web-routes
pnpm check:locales
pnpm exec cargo check --manifest-path src-tauri/Cargo.toml
(cd src-tauri && cargo check --no-default-features --features web-server --example server)
cargo test --manifest-path src-tauri/Cargo.toml --lib
pnpm test:unit
cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server -- web_api:: dual_runtime_parity:: web_proxy_lifecycle::
pnpm test:integration
pnpm build:web
pnpm smoke:web-server
```

- [ ] SQL restore/sync、pricing、session importer、Pi、Alpha/WebSearch、OAuth focused suites。
- [ ] 完整门禁全绿；flake 须基线复现 + 隔离重跑 + 列名记录，**不接受口头豁免**。
- [ ] coverage 自动校验 100%；任务内新增 diff 复审。
- [ ] `git diff --check`、secret scan、`.pi*` staged-file guard。

## Phase E: Deploy And Verify

- [ ] 备份运行二进制 + SQLite 一致快照；快照 integrity ok / schema 17。
- [ ] build / install / restart `cc-switch-web.service`。
- [ ] 验证 active/enabled、NRestarts、RSS、journal error、DB integrity/schema。
- [ ] 浏览器桌面 + 移动终验；核心 API 与修复工作流实际通过。
- [ ] 观察 ≥2 个后台 sync 周期，确认 deferred/日志行为稳定且 Pi 不回归。
- [ ] 更新必要 spec/journal/docs；commit、push、归档任务。

## Review Gates

- **规划 gate**：PRD/design/implement 齐备；用户 blanket approval 已记入 PRD。
- **修复 gate**：每个 finding 有失败证据 + 回归测试。
- **完成 gate**：coverage / 完整测试 / 真实部署 / 运行观察，四项缺一不可。

## Rollback Points

- 代码：每类修复独立 commit，可无损 revert。
- 部署：本次 `pre-review-fix` 二进制 + DB 快照**成对**恢复。
- 外部配置：测试仅用 temp HOME；不依赖回滚真实 `~/.codex` / `~/.pi` 写入。
