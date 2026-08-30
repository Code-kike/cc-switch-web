# Design: v3.20.0 Web 全量回归审阅与修复

## Review Architecture

"全面 review" 不等于跑一次测试。采用四层证据链：

1. **契约层** — `CONTEXT.md` 术语 + 父 S1–S8 + 三子任务归档 artifacts。
2. **静态层** — 冻结 `25e34700..f0b468ad` 文件清单，逐文件归属，深审高风险调用图。
3. **行为层** — 聚焦测试、临时 HOME/DB fixture、浏览器与 Web API 真实请求。
4. **运行层** — systemd 日志、DB、外部配置边界、部署后浏览器终验。

**准入规则**：finding 需至少两层证据互证（代码路径 + 失败测试，或运行日志 + 可复现 fixture）。
纯风格意见不计为 bug。

## Workstreams

### W1. Runtime And Browser Surface

- Chrome DevTools 巡检 3010 上全部顶层 app/tool 页面 + console + network。
- 默认只读；需要写状态时走临时 HOME / 临时 DB 的 smoke/integration harness。
- 对首屏 404、CSP `eval`、loading/error gate、不可达控件逐项定性（真缺陷 or 无害）。

### W2. Session Usage And Aggregation

数据流：

```text
session files -> scanner -> parser -> parent/fork replay -> dedup/cursor
 -> proxy_request_logs -> detail aggregation / daily rollup -> browser dashboard
```

- 先给 `session_usage_codex.rs` 现有行为加 characterization tests（含 118-deferred 形状）。
- **分层纠偏（architecture-review 采纳，有据）**：`database/mod.rs` 架构注释规定持久化归
  `dao/`，而 `session_usage_codex.rs:359` 反向实现 `impl Database`，并散落
  `reset_codex_usage_on_conn`、`insert_codex_session_entry_on_conn`、`find_codex_pricing`、
  `sqlite_table_exists/column_exists` 等裸 SQL。因此拆分方向是：
  - SQL / reset / cursor 持久化 → **`database/dao/session_usage_codex.rs`**（与
    `dao/usage_rollup.rs` 同层，不新建 services 层 repository）；
  - `services/session_usage_codex.rs` 保留 scanner / parser / replay，并最终瘦身为 orchestrator。
- parser 不依赖 DB；replay 只处理父链与签名匹配；orchestrator 只做调度与日志汇总。
- 成本口径：detail 与 rollup 新增量两层均按 2xx；历史 rollup 保持已记录的不可逆边界。

### W3. Data Integrity, Provider And Proxy

- constrained restore 按 `parse -> canonical stage -> authorizer -> single transaction -> swap/commit`
  审核；失败不得留部分状态。
- managed config 写入区分 `FollowManagedSymlink`（外部应用文件）与 cc-switch-owned storage。
- auth/takeover classifier 保持 Rust 单一真理源 + 前端镜像；未知/错误状态 fail-closed。
- pricing：建立 `FE preset default -> cleaned lookup candidates -> Rust seed/repair` 的自动一致性检查。

### W4. Feature And Cross-Layer Integration

- **Pi**：provider CRUD、models revision、prompt、skill path、session、usage、Web UI。
- **Alpha/WebSearch**：URL 派生、流式/非流式、citation caps、usage 计数。
- **Managed OAuth**：account selection、reauth、live auth 原子写、takeover carve-out。
- AppType/AppId 扩展点、command registry、Axum route/extractor、React Query key 全链一致。

## Coverage Ledger

- 冻结基线清单 `review-scope.txt`（302 文件 + source/test 标记），任务期内**不改分母**。
- `review-findings.md` 按 domain 记录：文件、结论、证据、finding id、修复 commit/test。
- 任务内新增改动用 `git diff f0b468ad..HEAD` 单独复审。
- 终验脚本校验基线清单每行都出现在 ledger，杜绝"抽查冒充全量"。

## Compatibility And Safety

- schema 固定 **17**；除非发现 schema 本身不可修复的 blocker，不新增迁移。
- 调查阶段对真实用户 DB / 外部配置只读；部署前 SQLite `.backup` + 二进制备份。
- 保持 no-auth posture 与 updater-disabled。
- 不提交 `.pi/`、`.pi-subagents/` 或浏览器临时资产。
- 破坏性修复先在临时 HOME/DB 验证；生产部署保留上一二进制 + 同时点 DB 快照。

## Rollback

- 每类修复独立 commit；回归时 revert 单 commit，不 reset 用户工作。
- 无 schema 变更时回滚仅需二进制；若写入行为改变，二进制与 DB 快照**成对**恢复。
- 外部 managed config 测试只在 temp HOME；真实文件不作测试载体。

## Trade-Offs

- **单任务 vs 子任务**：四个工作流共同服务一个"恢复 Web 可用"交付，且 finding 尚未稳定，
  先用单任务避免任务树替代审阅。若 Codex importer 分层改造形成可独立验收的大改，
  再建子任务并由本任务做集成终验。
- **模块化 vs 最小修复**：3155 行本身不是理由；只有 characterization 证明边界后才拆，
  且拆分与语义修复分属独立 commit。
- **全面覆盖 vs 同等深度**：302 文件全部记账，但深度按风险分配 —— generated catalog/i18n 做
  机器一致性检查，事务/凭据/解析代码做逐函数审阅 + 行为测试。

## Completion Proof

需**同时**具备：coverage ledger 100%、finding 全闭环、完整质量门、真实 browser/API 证据、
systemd 重新部署 + 稳定性观察。任一缺失都不得宣称完成。
