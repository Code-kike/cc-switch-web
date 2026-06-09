# cc-switch 双重审查交叉对比（Workflow × Codex）

> 对象：`main @ 786a09e0`（上一轮 ~45 条非安全修复已合入后的代码库）
> 日期：2026-06-06/07
> 源报告：`/tmp/cc-switch-review-workflow.md`（Workflow）· `/tmp/cc-switch-review-codex-final.md`（Codex）

## 两份审查的画像

| 维度 | Workflow（Opus 4.8 多 Agent） | Codex（gpt-5.5, xhigh） |
|---|---|---|
| 形态 | 17 个子系统并行审查员 + 14 条对抗式验证 + 主循环综合 | 单 agent 整库自主探索，read-only 沙箱 |
| 取向 | **广度优先**（全子系统铺开） | **深度/精炼优先**（少而准，附置信判定） |
| 产出 | 118 findings（3 High / 18 Med / 54 Low / 43 Info），14 条全部 Confirmed | 8 findings（2 High / 2 Med / 3 Low），无 Critical |
| 额外 | 对抗验证逐条反驳、压制假阳性 | **实跑门禁脚本验证**：route-parity（missing 0）、locale-parity（en/ja/zh 各 2241 keys 对齐）、git clean；耗 1.92M tokens |
| 基础设施 | 本地网关（曾因 synthesis 超大 prompt 停滞，已从 journal 恢复） | anyrouter/gpt-5.5（独立，未受网关影响） |

## ✅ 共识（两侧独立命中 = 最高置信）

| 问题 | Workflow ID | Codex ID | 定级 | 备注 |
|---|---|---|---|---|
| auth/CSRF/rate-limit 中间件定义了**从未挂载** → `/api` 全程无鉴权读/写/执行 | WEBMIDDLEWAREROUTES-1 | **H-01** | High = High | 两侧都指认 `routes.rs:28/59` 只挂了 CORS；csrf 返回常量 `stub-csrf-token` |
| SPA fallback **路径穿越任意文件读** | WEBMIDDLEWAREROUTES-2 | **H-02** | High = High | 两侧独立发现、都建议 `tower_http::ServeDir`；Workflow 用项目精确依赖实测 `GET /../../etc/hostname → 200` |
| systemd/README 引导无鉴权 API 暴露到 `0.0.0.0` | BUILDDEPSDOCSIN-1 | （并入 H-01） | High | — |
| session `sourcePath` 任意文件/SQLite 读 | SESSIONMANAGER-2 / WEBHANDLERSAUX-1 | **M-01** | Med = Med | **关键佐证**：codex 指出 delete 路径会 canonicalize + 校验 provider 根，而 load 路径不会 → 证明这是疏漏而非设计 |
| session resume **命令注入** | SESSIONMANAGER-1 | **L-02** | Med ↔ Low | 见下「分歧」 |

> 5 条核心安全问题两侧独立吻合 —— 两个不同模型、不同基础设施、互不知情 → **置信度极高，可视为确定**。

## 🟦 仅 Codex 命中（补充，值得纳入完整画像）

- **M-02｜默认 debug 日志会持久化完整 prompts/responses**（隐私）：shipped systemd 单元 + `examples/server.rs` 默认 `cc_switch=debug`，debug 路径打印完整请求/响应体 + SSE（`forwarder.rs:1531`、`response_processor.rs:260/777`）；过滤器剥离部分私有字段但**不含用户 prompt/模型回复** → systemd journal 落地明文敏感内容。建议默认 `info`、body 日志改显式 opt-in。**Workflow 未发现。**
- **L-01｜useSettings 陈旧 react-query 缓存的兄弟副作用**：plugin 集成路径已正确从 live cache 读旧值，但相邻的 `launchOnStartup`/`skipClaudeOnboarding` 仍比对闭包捕获的 `data`（`useSettings.ts:217-382`）→ 快速切换可能持久化设置却跳过 OS 自启/onboarding 副作用。**Workflow 的 frontend-data-state 未精确命中这条。**
- **L-03｜usage-script HTTP 响应/提取输出无字节上限**：rquickjs 有 CPU 超时、HTTP 有超时钳制，但 `resp.text().await` 全量读入内存、提取输出无字节封顶 → 内存耗尽（多为自 DoS，叠加无鉴权 web 则放大）。**Workflow 的 proxy-usage 关注了定价正确性，未覆盖此可靠性角度。**

## 🟨 仅 Workflow 命中（广度，Codex 精炼集未覆盖）

- **WEBHANDLERSCORE-1** SSRF guard 漏过 `0.0.0.0`/`[::]`（Linux 抵达 loopback）
- **PROXYPROVIDERS-1** Gemini 缓存 token 未从 input 扣除 → 缓存成本**双重计费**（两个兄弟转换器都扣了，唯独 gemini 没扣）
- **PROXYUSAGE-1** 定价归一化未与 `[1M]` 标记剥离组合 → 静默 `cost=0`
- **WEBHANDLERSCORE-2 / WEBHANDLERSAUX-2** multipart 沿用 axum 2MB 默认 → 真实 SQL 导入/上传以不透明 500 失败
- **TESTINGCI-1** `web_api` 的 Rust `#[cfg(test)]` 单测在任何 CI job 中都不被编译/运行
- **SERVICESPROVIDER-1** switch backfill 把含 proxy 占位符的 live 文件内容无校验写回出站 provider
- **FRONTENDARCHITECTURE-1** 全应用无 React error boundary（任一渲染抛错→白屏）
- **ARCHITECTUREDUALRUNTIME-1** server.rs/web_proxy.rs/web_services.rs 三份手维护模块清单无编译期一致性保证
- **BUILDDEPSDOCSIN-2** rust-embed 编入 web-server feature 但未用（前端从磁盘服务）
- + 约 100 条 Low/Info（详见 Workflow 报告）

## ⚖️ 严重度分歧

- **命令注入**：Workflow 评 Medium，Codex 评 Low。Codex 论证更细——web 模式只**复制**命令而非执行、桌面 macOS 才真正 `sh -c`/osascript 执行、且需攻击者**已能写入/污染本地 session 文件**（用户点击 resume 触发）。→ 实际世界严重度 **Low–Med**，Codex 的下调有据。

## 净评估

1. **两个独立模型强一致**：无 Critical；整体核心架构稳健（成熟的 SQLite 迁移备份/外键/savepoint、WebDAV ZIP `enclosed_name` 防穿越 + 哈希校验、集中式 CSRF-aware adapter、充分的 round-trip 测试）。
2. **唯一系统性高风险 = web 运行时攻击面**：鉴权缺失 + 路径穿越 + 任意读 + 命令注入。在**默认 loopback 个人使用**下真实风险大幅降低；但仓库自带 systemd/`ALLOW_HTTP_BASIC_OVER_HTTP`/README 主动推向 `0.0.0.0`/LAN 暴露，是缓解的**反作用力**——两侧都把"纠正远程部署默认/文档"列为 Do-now。
3. **互补价值**：Codex 补 3 条新洞（**debug 日志持久化 prompts 最值得关注**）；Workflow 补成本正确性（Gemini 双计 / cost=0）+ CI 缺口（web_api Rust 测试不进 CI）+ 健壮性广度。
4. **与既有项目记忆一致**：绝大部分 web 安全面在 2026-06-05 deep-read 已记录（含 SSRF 0.0.0.0 遗漏、session sourcePath 无根校验、mcp upsert 致 RCE、cost=0 双因）；本轮**新增** = SPA 静态穿越、resume 命令注入、debug 日志隐私 —— 已补入记忆。

> 边界：本项目个人使用，**本轮仅审查、不做任何安全修复**；安全项按 loopback 真实影响定级，供完整画像。
