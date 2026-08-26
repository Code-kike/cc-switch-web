# PRD：同步 Product upstream v3.18.0 之后的更新到 Web-first fork

## 状态

已确认（2026-08-08 grilling 完成，用户确认开始执行）。

### Grilling 裁定结果

1. **目标边界**：单分支 `sync/upstream-v3.19.2` 直达 `v3.19.2`，显式纳入
   `413c09e0`（catalog 覆盖修复）。不分阶段落中间 tag。
2. **Sponsor/preset/i18n churn**：全量原样移植（含 README/Sponsor 按钮纯文档
   提交），避免未来同步的永久漂移；预设对用户是惰性的。此裁定覆盖下方
   “默认排除候选”中的营销变更条目。
3. **双运行时策略**：沿用 S1–S8 模式；web-only 代码必须 feature-gate；新命令
   在 `web-commands.ts`（路由 SSOT）注册并过 `check:web-routes`。
4. **安全上限提交 `6b8f3643`**：完整移植。
5. **DB 迁移**：上游迁移原样采纳，校验从 v16 严格连续；`668bbda9` 备份事务
   批处理必须过 SQL-restore 锁测试。
6. **定价种子**：移植时执行 FE-preset↔Rust-seed 一致性核对（防 cost=$0，
   注意 `find_model_pricing_row` 有损清洗）。
7. **门禁**：每批全量本地门禁（cargo fmt --check、pnpm format:check、web
   cargo check、check:web-routes/locales、test:integration、Rust web_api::
   dual_runtime_parity:: web_proxy_lifecycle::）；已知 4 个环境 flake 不阻塞。
8. **部署**：本任务止于 PR 合入 main；systemd 部署为合并后独立确认步骤。

## 目标

将 `farion1231/cc-switch`（Git remote：`product-upstream`）在本仓库上次同步基线
`v3.18.0` 之后的适用产品能力与修复，按 Web-first fork 的运行模型完成选择性移植、
Web 适配与验证。

> 领域语言：本任务使用 `Product upstream`、`Product upstream sync` 和
> `Web-first fork`；不把 `origin` 或 `Laliet/CC-Switch-Web` 称为“上游”。

## 已确认事实

- 本仓库 `main` 当前版本为 `3.18.0`，最近一次同步提交为
  `5687d4c0`（v3.17.0 + v3.18.0，S1–S8）。
- `product-upstream/main` 当前为 `413c09e0`，描述为
  `v3.19.2-1-g413c09e0`。
- Product upstream 已发布 `v3.19.0`、`v3.19.1`、`v3.19.2`；从 `v3.18.0`
  到 `v3.19.2` 共 81 个提交，之后还有 1 个未发版修复提交。
- **同步边界已确认**：以稳定 tag `v3.19.2` 为主基线，并显式纳入
  `413c09e0`；不以可移动的 `product-upstream/main` 分支名作为复现边界。
- `413c09e0` 修复 Codex catalog 生成时覆盖用户自有 `model_catalog_json` 的问题；
  本 fork 当前代码仍存在相同覆盖行为，因此纳入该提交。
- 上次任务明确延期的 9 个 `v3.18.0` 后提交已进入本次候选范围，包括
  tool-result 媒体桥、Grok Build usage 与 Grok 官方订阅配额。
- 仓库词汇表将当前仓库定义为 Web-first fork；Product upstream sync 是“选择并移植”，
  不是无差别 merge。
- 既有 Web 约束包括：无认证部署姿态保持、desktop updater/release 链默认不移植、
  Tauri command 与 Web API 路由保持等价、数据库/配置写入安全边界不可退化。
- 工作树已有用户侧未跟踪目录 `.pi/` 与 `.pi-subagents/`；本任务不得修改或提交它们。

## 初始需求（待 grilling 确认）

1. 建立 `v3.18.0..目标边界` 的完整提交清单，并为每个功能性提交记录：移植、适配、
   排除或延期及理由。
2. 以主题和依赖关系拆分同步批次，避免整体 merge 覆盖 Web-first fork 的既有修复。
3. 保留并回归验证既有 Web security hardening、Web API parity、数据库迁移保护、
   usage/pricing 正确性及远程长期运行能力。
4. 对 Product upstream 的安全修复、数据完整性修复和协议兼容修复做显式优先级判断。
5. 同步完成后更新版本、fork 自有 changelog/release notes，并执行分批门禁与全量门禁。
6. 如涉及数据库迁移或服务重建，制定备份、回滚、重启与真实服务冒烟步骤。

## 初始验收标准（待细化）

- [ ] Product upstream 目标边界由用户明确确认。
- [ ] 目标范围内每个功能性提交都有可审计处置结果和理由。
- [ ] 所有纳入能力完成 Web API / browser UI / headless runtime 适配。
- [ ] Web-first fork 的安全边界、无认证部署决定和 updater 禁用决定未退化。
- [ ] 前端、Rust、Web 路由、locale、集成测试、生产构建及真实 Web 服务冒烟通过。
- [ ] 数据迁移前有备份，失败可恢复；服务重启后关键流程可用。
- [ ] 版本与 fork 自有发布说明反映实际移植范围，不宣称未实现的上游能力。

## 默认排除候选（待确认）

- Product upstream 的桌面 updater、发布资产镜像、签名/打包 CI。
- 纯赞助商排序、推广文案、referral 链接与 README 营销变更。
- 与 Web-first 部署无关的纯桌面外观或平台专属改动。
- Product upstream guide/release-note 原文；本 fork 只写与实际移植相符的说明。

## 开放决策

1. 功能选择策略：所有非桌面产品能力默认纳入，还是逐主题批准。
2. Sponsor/preset 变更中“功能性 catalog delta”和纯商业推广的分界。
3. 大型新增能力与安全修复的批次优先级、每批提交策略及回滚粒度。
4. 最终是否由本任务合入 `main` 并重建/重启本机 systemd Web 服务。

## 暂不做

- 在 grilling 结束、PRD/design/implement 文档获批前修改产品代码。
- 修改或提交现有未跟踪目录 `.pi/`、`.pi-subagents/`。
