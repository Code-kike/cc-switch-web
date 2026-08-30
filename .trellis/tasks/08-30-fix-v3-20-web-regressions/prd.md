# PRD: v3.20.0 Web 全量回归审阅与修复

## Goal

以当前 `main` 与已部署的 systemd Web 服务为事实源，重新审阅并修复
`25e34700..f0b468ad` 区间（Product upstream sync v3.20.0 + 后续补正）引入的缺陷，
使 Web-first fork 的浏览器工作流、Web API、后台同步与本地配置管理恢复可用，
并以可复现行为、回归测试与重新部署证明结果。

## Source Contracts

- 根 `CONTEXT.md`：**仅**领域语言与 Avoid 词表，不单独充当功能契约。
- 父任务 `.trellis/tasks/archive/2026-08/08-18-sync-upstream-v3.20.0/{prd,design,implement}.md`：S1–S8 范围与验收。
- 三个归档子任务各自的 `{prd,design,implement}.md`：
  - `08-18-feat-pi-native-agent`
  - `08-18-feat-codex-alpha-websearch`
  - `08-18-feat-managed-oauth-accounts`
- 当前源码、测试、运行中的服务、数据库与外部配置是**最终事实**；归档报告是上下文，不是通过证据。

## Confirmed Facts At Task Start

- 基线 `main@f0b468ad`，工作树干净（仅本任务目录未跟踪）。
- `25e34700..f0b468ad` 共 **302** 个改动文件，其中 `src/`/`src-tauri/`/`tests/`/`scripts/` 下 **256** 个 source/test 文件。
- systemd 用户服务 active（PID 1134，NRestarts=0）；`GET /` 200、usage summary 200；浏览器首屏可渲染，providers 列表正常。
- 首屏 29 个请求中核心 API 全 200；存在 **1 个 404** 与 **CSP `eval` issue**，尚未定性。
- **已复现运行时异常**：Codex 会话同步每分钟扫描 518 文件、重复输出 **118 条** `deferred` WARN、导入 0；Pi 同步仍在导入（1–3 条/轮）。
- usage detail 与新增 rollup 的 `total_cost` 已按 2xx 收敛（`e941a3e7`）；修复前已冻结进 `usage_daily_rollups` 的失败成本因源 detail 已删除**不可精确重算**，须保留既有有界说明，不得伪造回填。
- `database/mod.rs` 架构注释规定持久化归 `dao/`；`session_usage_codex.rs:359` 存在反向 `impl Database` 与多处裸 SQL —— 属既有分层违规。
- 规划期研究子代理因本机子代理 provider 未认证（vercel-ai-gateway / openrouter 无 key）无法启动；**主会话必须自行完成全部取证**，不得以此缩小范围。

## Requirements

### R1. Complete Review Coverage

- 对 302 个改动文件建立覆盖账；256 个 source/test 文件必须各有审计归属与结论（"无发现" 或具体 finding）。
- 覆盖父任务 S1–S8 + 三子任务全部目标。
- 高风险文件与跨层数据流须语义审阅；**不得**用 grep 命中或"测试全绿"替代。

### R2. Web Runtime Usability

- 在真实 `http://127.0.0.1:3010` 上验证所有顶层应用与工具页可打开；无导致核心功能失败的 console error、非预期 4xx/5xx、无限加载、不可达控件或首帧状态误判。
- 覆盖 provider 列表/切换/增删改、用量、设置、Sessions、Skills、Prompts、MCP、Pi provider/prompt/session、Managed OAuth 账号选择的代表性读路径；写路径只用可回滚 fixture / 临时 HOME / 测试库，**不破坏用户真实凭据**。
- Web API 的 method/path/参数位置/Axum extractor 必须与 `web-commands.ts` SSOT 一致。

### R3. Background Sync And Usage Correctness

- 修复已复现的 Codex deferred 链不收敛 / 全量重扫 / 逐文件 WARN 放大。若 deferred 是合法中间态，则调度必须增量、日志必须有界，且父数据到齐后能继续导入。
- 端到端审阅 Codex/Pi importer 的解析、父链回放、dedup、cursor/reset、status、token、cost 与 DB 写入，并补回归测试。
- `total_cost` 继续只计 2xx；raw detail 保留单条原始 cost；历史 rollup 的不可逆边界不得被宣称为已修。

### R4. Data Integrity And Managed Files

- 深审 constrained SQL restore、SQL fidelity、authorizer、canonical staging、batch 上限、单事务 restore、sync lock/rollback、schema 17 连续性。
- 外部 managed config 写入继续遵守 ADR 0003（`FollowManagedSymlink` 原子替换）；`write_store_atomic` 仅限 cc-switch 自有 data-dir。
- auth / 账号绑定 / 路由决策失败 **fail-closed**；presentation-only 降级可 fail-open。

### R5. Catalog, Provider, Proxy And Feature Contracts

- FE preset 默认 model 必须能解析到 Rust pricing seed，防止静默 `cost=$0`；repricing 的 preset/seed/repair 三处一致。
- 保持无认证姿态、updater 禁用、双运行时等价、**292/280/0** 路由基线（除非经审阅的真实命令增减）。
- 保持 128 MiB body、2s JS、16 MiB heap、256 KiB stack、32 MiB catalog 与五个 `MAX_CITATION_DEDUP_*`。
- Pi additive 语义、Alpha Search fail-closed URL 派生、WebSearch markdown/citation 上限、Managed OAuth reauth/takeover 分类器、never-clobber Codex login 均不得回归。
- 不恢复 Claude Desktop 表单、zh-TW、延期 Codex Chat routing 四文件。

### R6. Focused Fixes And Regression Tests

- 每个修复须有触发路径、根因，以及**修复前失败 / 修复后通过**的回归测试或等强运行时 fixture 证据。
- 禁止删除、跳过或弱化既有断言来"修绿"。
- 仅当缺陷根因证明必要时才拆模块；模块化须先锁定行为再移动代码，且与语义修复分属不同 commit。

### R7. Deployment And Operational Verification

- 完成后构建 release，备份当前二进制 + SQLite 一致快照，重新部署 systemd 服务。
- 验证 active/enabled、无重启循环、schema 17、`integrity_check = ok`、核心 HTTP/API/browser 流程可用、无新 error-level journal、资源稳定。

## Acceptance Criteria

- [ ] 覆盖账含 302/302 改动文件，256/256 source/test 文件有审计归属与结论。
- [ ] 所有确认的 P0/P1/P2 finding 已修复；不存在仍可复现的核心 Web 不可用问题。
- [ ] 真实浏览器桌面 + 移动视口主要导航/读取流程通过，核心请求无非预期失败。
- [ ] Codex deferred 行为已用 fixture 证明可收敛；扫描/日志放大受界；Pi 同步无回归。
- [ ] SQL restore/sync、pricing seed、provider/auth/proxy、Pi、Alpha/WebSearch、OAuth 专项测试通过。
- [ ] `cargo fmt --check`、Prettier、typecheck、desktop/web cargo check、`cargo test --lib`、test:unit、test:integration（仅已复现为基线的 fixture flake 可豁免且须列名）、check:web-routes、check:locales、build:web、smoke:web-server 全绿。
- [ ] route parity / schema / 全部安全上限符合契约；`.pi/`、`.pi-subagents/` 未提交。
- [ ] 修复已 commit 并 push；systemd 已重新部署并通过运行时终验。
- [ ] journal/spec/docs 仅记录经代码或运行时证明的新事实，不延续错误结论。

## Out Of Scope

- Product upstream `v3.20.0` 之外的新功能。
- 恢复已明确延期/排除的 Claude Desktop、zh-TW、desktop updater、Codex Chat routing 栈。
- 对已删除 detail 的历史 rollup 成本作估算式回填。
- 修改或提交 workspace `.pi/`、`.pi-subagents/`。

## Open Questions

无阻塞项。用户已授权 brainstorming 全部采用推荐方案；执行中按
**数据安全优先 / fail-closed 授权 / 最小可验证修复 / 必要时才拆分** 裁定并记录。
