# cc-switch 统一修复意见（Claude × Codex 辩论达成一致）

> 对象：`main @ 786a09e0`（个人使用 / 默认 loopback）
> 过程：2 轮结构化辩论 → codex 最终 **"CONVERGED"**；全部 16 项就"是否修 / 怎么修 / 优先级"达成一致。
> 性质：**修复建议**，不在本任务中实施。R1 中 codex 修正了我 3 处提案错误（item 9 重复 link-local 规则、item 9 redirect 作用域、item 13 测试命令），R2 全数收敛。

---

## 0. 优先级框架（核心共识）

- **严格 loopback 单用户主机上，唯一无条件 do-now = Item 3**（systemd/README 把无鉴权 API 引向 `0.0.0.0` 的脚枪）。
- **Item 1 / 2 / 6 是"暴露前必修"（must-fix-before-ANY-exposure）**：一旦绑非 loopback、或出现浏览器 drive-by / 本地恶意进程 / 多用户本地访问，立即升级为 do-now。
- **链式风险（须单独点名）**：`Item 1 无鉴权 API` × `Item 2 路径穿越/静态文件读` × `无鉴权 config/MCP 写入`（McpService::upsert）→ 可演化为**延迟代码执行 / 凭据·配置泄露**；`Item 6` 的请求/响应体日志再**放大**（journal 留存明文 secret）。整体严重度 **> 各项单看之和**。

---

## 1. 修复批次

### 🔴 DO-NOW
| # | 问题 | 修复（含 file:line） |
|---|---|---|
| **3** | systemd/README 引向 0.0.0.0（无条件） | `deploy/systemd/cc-switch-web.service:10` 改 `HOST=127.0.0.1`；把 `ALLOW_HTTP_BASIC_OVER_HTTP`(:13) 与"非 loopback 绑定门"**拆成两个独立开关**；`README.md:67` 把"支持 0.0.0.0"改为"**无鉴权设计，仅限 loopback**"的明确警示 |
| **1** | auth/CSRF/rate-limit 定义未挂载（暴露前必修） | `examples/server.rs:248` 把绑定门与上面那个误导性 flag 拆开，并在**非 loopback 绑定且未配置真实鉴权时 fail-closed**（拒绝启动）；loopback 保持开放。**完整 session-auth+CSRF+rate-limit 挂到 `routes.rs:28/59` = LATER**（仅当真要远程部署） |
| **2** | SPA fallback 路径穿越任意读（暴露前必修） | `routes.rs:88-101` **在 read 之前**：拒绝含 `ParentDir`(`..`) 组件的路径，canonicalize candidate 并要求 `starts_with(canonical dist_root)`；保留现有 SPA 兜底(:103-106)；补穿越回归测试 |
| **6** | debug 日志持久化 prompts/responses（暴露前必修；脱敏=SOON） | `examples/server.rs:313` + `systemd:16` 默认改 `info`（去掉 `cc_switch=debug`）；body/SSE 日志改 `CC_SWITCH_LOG_BODIES=1` 显式 opt-in。**SOON**：即便开 body 日志也对 prompt/response/tool 字段脱敏/截断（`response_processor.rs:260/777`、`forwarder.rs:1531/1533`） |

### 🟡 SOON
| # | 问题 | 修复 |
|---|---|---|
| **9** | SSRF 缺口 | (a) `common.rs:139/146` 给 `is_blocked_ipv4/ipv6` 加 `is_unspecified()`（0.0.0.0 / ::）——**不动** link-local（`169.254/16` 已被 `is_link_local()` 覆盖）。(b) redirect 绕过：**不改全局** `http_client::get()`（`forwarder.rs:1571` 反代转发需跟随 redirect）；为受保护 web 路径（`model_fetch`/`stream_check`/`webdav`/`usage_script`）用**独立 client（`redirect Policy::none()`）或逐跳再校验**。(c) usage-script web 模式加 URL SSRF 校验，带**显式 LAN opt-out**。**LATER**：统一 outbound 守卫重构 |
| **10** | Gemini 缓存 token 双计 | `transform_gemini.rs:1100-1120` `build_anthropic_usage` 改 `input_tokens = promptTokenCount.saturating_sub(cachedContentTokenCount)`，对齐两个兄弟（`transform.rs:749`、`transform_responses.rs:328`）。仅影响 `gemini_native`-经-Claude-路由（app_type="claude"）；补测试 |
| **11** | 定价 `[1M]` 未与归一组合（latent） | `usage_stats.rs:1701-1748` `pricing_lookup_candidates` 增加"对 `dot_dash`/`lower` 候选再剥 `[1M]`"的组合候选（~2 行）；补 dotted+`[1M]` 组合用例。**trivial/防御性**——当前无 model id 活体触发，但失败是静默 cost=0，顺手做 |
| **12** | multipart 2MB 默认 → 不透明 500 | 对 `config.rs:683` / `skills.rs:380` / `prompts.rs:104` 路由加显式 `DefaultBodyLimit`（按 SQL/skill/prompt 体量），超限返回清晰 413 |
| **13** | web_api Rust 单测不进 CI | CI web-server job 加：`cargo test --no-default-features --features web-server --example server --locked --manifest-path src-tauri/Cargo.toml`（普通 `--features web-server` 跑不到，因 `lib.rs` desktop-gated、web_api 仅存在于 example 目标） |
| **7** | useSettings 陈旧缓存兄弟副作用 | `useSettings.ts:227/382` 在 mutation 前用 `queryClient.getQueryData(["settings"])` 取旧值（对齐已正确的 plugin 路径 :217）；补快速切换测试（`useSettings.test.tsx:388`） |
| **14** | 无 React error boundary | `main.tsx:90` 在 `<App>` 外包顶层 `<ErrorBoundary>`，提供 fallback + reload |

### 🟢 LATER
| # | 问题 | 修复 |
|---|---|---|
| **5** | resume 命令注入（Low–Med） | 尽量用 argv 直构而非 `sh -c`/AppleScript（`terminal/mod.rs:30/251`）；插值前校验各 provider session-id 格式（regex）+ shell-quote（`providers/*::resume_command`） |
| **8** | usage-script 响应/输出无字节上限 | `usage_script.rs:403` `resp.text()` 加 max-byte；`:198` 请求配置 / `:306` 提取输出限长；返回"response too large" |
| **15** | 双运行时三份模块清单无 parity | build-script 或 test 断言 `examples/{server,web_proxy,web_services}.rs` 的模块清单 == `src/{proxy,services}/mod.rs` + `lib.rs` 实际模块集 |
| **16** | rust-embed 编入未用 | `Cargo.toml:36/130` 移除该依赖；或真正用 `RustEmbed` 内嵌 dist-web（当前 `routes.rs:162` 从磁盘服务） |
| 1′ | 完整 web 鉴权 | 仅当决定远程部署：实现 session-auth + per-session CSRF + per-IP rate-limit 并挂到 `api_router`；`/api/health` 公开 |
| 9′ | 统一 outbound SSRF 守卫 | 把 redirect-none/逐跳校验抽象成所有 web 出站向量共用的封装 |

---

## 2. 双方一致的范围澄清（避免误修）

- **Item 10** 只影响 `gemini_native` 经 Claude/Anthropic 路由的计费；**原生 Gemini 应用流量不受影响**（其 app_type="gemini"，calculator 已扣减）。
- **Item 9(c)** usage-script 跳过 HTTPS/同源是**桌面/自定义场景的有意设计**——只在 **web 模式**加校验，并保留 opt-out，别一刀切。
- **Item 11** 是 latent/防御性，**非活体 bug**；价值在"顺手堵静默 cost=0"，别夸大严重度。
- **Item 15** 的 `#[path]` 双运行时设计**本身是有意的**（已文档化）；要修的只是"清单漂移无编译期防护"。

---

## 3. 一句话结论

代码核心稳健、**零 Critical**；真正的系统性风险是 **web 运行时攻击面**，且**仅在脱离 loopback 时才致命**。因此：**无条件先做 Item 3（堵脚枪，近零成本）**；其余安全项（1/2/6/9）作为"暴露前必修"清单；成本/正确性（10/11/12/13）与健壮性（7/14）排 SOON；其余 LATER。完整逐项见本文件。
