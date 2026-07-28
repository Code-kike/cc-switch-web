# PRD: 同步产品上游 cc-switch v3.16.5..v3.18.0 到 web fork

## 背景

产品上游 `farion1231/cc-switch` 自本 fork 上次同步（v3.16.5，任务
`07-06-sync-upstream-cc-switch-v3-16-5`）后发布了 **v3.17.0** 和 **v3.18.0**，
`v3.16.5..v3.18.0` 共 125 个提交。本任务把这些增量分批移植进 web-first fork。

用户已通过 grilling 会话确认全部关键决策（2026-07-25）。

## 已确认决策（不可再议）

1. **同步目标**：`v3.18.0` tag。`main` 上 9 个未发版提交（tool-result 媒体桥扩展、
   Grok Build usage 导入、Grok 订阅配额）**本次不含**，记入下次同步待办。
2. **基线**：审计分支已 ff 合入 `main`（4ad02386）；工作分支 `sync/upstream-v3.18.0`，
   PR 目标 `main`。
3. **方法**：按主题分批移植（非整体 merge tag），每批一个提交。
4. **冲突原则**：**上游优先 + 本地适配**——上游同修的问题采上游实现；上游未触及的
   本地修复（`pricing_missing` 标记、时区分桶、gemini 配置保护、原子写等）在上游新
   代码之上重新适配保留；web 约束（feature-gate、禁 updater、无认证）不可退让。
5. **验证**：每批轻门禁（fmt/clippy/单测/typecheck/build），末尾全量门禁
   （integration 套件、`cargo test web_api:: dual_runtime_parity:: web_proxy_lifecycle::`、
   `check:web-routes`/`check:locales`、启动真实 web 服务 E2E）。
6. **收尾**：验收通过合回 `main` 后，备份 DB（上游含 Codex usage 升级重建）、
   重建并重启本地 systemd web 服务。

## 威胁模型与既定约束（沿袭 07-10-web-bug）

- 单用户私网（Tailscale）。**无认证是设计决定**（ADR-0001），任何批次不得引入认证。
- 桌面 updater 已因供应链风险禁用（H3），**不同步任何 updater/release 打包链改动**。
- 桌面专属功能 feature-gate 隔离，不删除；web-only 代码必须 gate 或放 web 层文件，
  否则破坏桌面 clippy/test CI 门禁。
- web 二进制 = `examples/server.rs` 经 `#[path]` shim；`web-commands.ts` 是路由 SSOT，
  新增 Tauri command 必须同步补 web 路由与 `check:web-routes` 覆盖。

## 范围

### ✅ 纳入（用户确认）

- **核心修复全收**：cache-write 计费（跨 schema）、Anthropic cache TTL、prompt cache
  断点注入加固、Codex 子代理 usage、稳定 usage key、工具 schema 归一化、流式 tool-call
  顺序/身份、MCP 同步加固群、定价种子（GPT-5.6 系/Hunyuan Hy3/Kimi K3/grok-4.5）、
  skills 默认仓库删除保留、dashboard 刷新间隔持久化、usage 瞬时故障拒绝。
- **项目 Profiles**（v3.17.0 大功能，~13 提交）：快照式配置切换、切换自动保存、
  切换前停用代理接管。Claude Desktop scope 在 web 模式降级处理（gate 或隐藏）。
- **Grok OAuth + Grok Build**（v3.18.0 主体）：xAI OAuth 设备流后端、Grok 账号管理 UI、
  managed provider + native Responses、Grok Build 一等支持、相关预设/定价。
- **前端错误日志落盘**（62747058 + 22d2872c）：web 模式落服务端磁盘，保留结构化脱敏。
- **Codex usage 升级重建**（eff1e0cc 群）：含 DB 迁移；迁移前备份、失败保原库阻断启动
  （沿用 v3.16.5 同步的恢复策略）。

### ❌ 排除

- 赞助商/推广/referral：README 赞助行、referral 链接更新、SudoCode 推广、
  预设按赞助商分组（f3108bf7，纯展示）。
- 桌面专属：Windows 控制台闪烁（3bc828ae）、tray 语言检测（997be22b）、
  updater/release 供应链 CI（468c93d4 按 fork 情况裁剪）、Windows ARM64 等。
- 上游营销性 guides 文档（Codex Kimi/Claude 路由指南）默认跳过；release notes 要写
  本 fork 自己的。
- `origin` Dependabot 分支。

## 分批计划（每批一个提交 + 轻门禁）

| 批 | 主题 | 上游关键提交 |
|---|---|---|
| S1 | usage/pricing 核心正确性 | f991726f, 13e7c1fc, b9263a80, 0e563b50, 6eb217b2(revert TTL), f39d463c, 98ccde00, 2df2212c, 31ee4285, a7b4dd94, 62e44c48, 99573d22, 940ddd33, 5c39dfbf |
| S2 | Codex 路由/协议桥 | f15184ed, 51d6c458, af740522(OAuth identity), 650905af, 27ce0a51, a078b4b2, 99e11e08, c6197ac3, ded0b63a, ac52c851, 7479d14d(default model), b3e5e32c, 3538b392, 50270d5e, 95c917b3(Zhipu quota) |
| S3 | MCP/配置同步加固 | 94fc1cc0, 11c173c7, 1f36f0cf, 6d2ee247, 473c2aaa, 93f56198, 8b1ce764, 88d5ffba, 6245caa6, e78aa8a7, e191af4a, ffc22ea7 |
| S4 | 项目 Profiles | 8f018a2c, 6179c188, 65a5464f, dbb5999d, 4cf6f175, 4f45601f, f05ed3db, 3ec83578, 754af2cc, 22159430, 9f7642e2, afabe801, 44279987, 06039540(health-check 移除) |
| S5 | Grok OAuth + Grok Build | a35209a6, 615c99c6, e9317f47, cdf0ee34, db444847, dbb5bd15, 6428e993, 8dcedbc0, 17b053ed, 1c0ee0c5, 325ba484, f733def4, a5aa1fd8, a8daf7da |
| S6 | Codex usage 重建 + 会话修复 | eff1e0cc, c9ac6efd, df3e07ed, a10b569a, eb105eae, e606adfa, 7a7d41c8(free-plan 窗口) |
| S7 | 日志落盘 + 杂项修复 | 62747058, 22d2872c, 08710d51, 9ca1a41f, 6d316c0b, 613fef70, edea624a, 7e73a1ff, e356fc6e, f2045822, 2bfecead, 6fddcaa9, c4795e98, f6e37ed9(裁剪) |
| S8 | 版本 3.18.0 + changelog + 全量门禁 | — |

注：批内提交 hash 以 `research/upstream-commit-inventory.md` 为准；实现时如发现
批间依赖倒置（如 S2 依赖 S1 的 schema 变更），按依赖顺序调整并在 journal 记录。

## 验收标准

- [ ] S1–S7 各批完成移植、轻门禁通过、独立提交。
- [ ] 上游 v3.17.0/v3.18.0 的每个功能性提交：已移植 / 已适配 / 明确记录排除理由。
- [ ] web 约束保持：无认证不变、updater 保持禁用、桌面构建 clippy/test 不破。
- [ ] 本地保留修复仍生效：pricing_missing、时区分桶、gemini 配置保护、原子写、
      SSRF/日志隐私相关加固。
- [ ] DB 迁移有备份与失败保护；Codex usage 重建可执行。
- [ ] 版本号/changelog 升至 3.18.0，标注 web-first fork 对齐产品上游 v3.18.0。
- [ ] 全量门禁通过（integration + Rust 三套件 + web-routes/locales + 服务启动 E2E）。
- [ ] 合回 main 后：备份 DB → 重建 → 重启 systemd 服务并冒烟。

## 下次同步待办（本次明确不做）

- 上游 main 未发版 9 提交：878c26f3/6c9d444c（tool-result 媒体桥，与本地 Volcano
  GLM 5.2 媒体修复同域，同步时注意合并语义）、34cbb375/cd161f44/3cf84ca3（Grok Build
  usage）、15d5dbe0（Grok 配额）、docs×3。
