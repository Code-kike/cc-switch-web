# cc-switch-web 全量深度审计报告

- **日期**：2026-07-10
- **分支**：`fix/web-audit-phase1-2`
- **方法**：46-agent 并行审查工作流（7 维度发现 + 逐条对抗性验证）
- **威胁模型**：单用户私网（Tailscale）。"已认证用户可做 X" 降为 Medium；未认证暴露/密钥泄露/供应链 = High/Critical。
- **交付模式**：先报告，后修复（逐项确认后再改）。
- **产品决策**：桌面冗余功能只隔离不删除；继续跟随上游（改动贴着上游最小化，优先落在 web 层/新文件）。

> **最重要的发现（推翻旧认知）**：`fix/web-audit-phase1-2` 上的提交 `8fea1699`（"fix(web): remove basic auth from web api"，任务 07-09）在 phase1-2 加固**之后**又把 HTTP Basic 认证**移除了**。Web API 现在**完全无认证**，属于文档化（但未批准，ADR-0001 status: proposed）的产品决策。此前"认证已接线、非环回强制"的记录已作废。

统计：**High 4 · Medium 8 · Low 20**（32 项经验证为真；另有 1 项 web-bootstrap 自动 seed 经验证被判为非缺陷，未计入）。

---

## HIGH（4）

### H1. 同源意图守卫可被 DNS-rebinding 绕过 → 远程网站获得全部无认证变更能力
- **文件**：`src-tauri/src/web_api/middleware/intent.rs:34-71`；`routes.rs:28-37`
- **类别**：security | **分类**：web 加固缺口
- **证据**：无认证 API 的唯一写保护是 `check_same_origin_intent()`，它校验 `Origin host == Host` 或信任 `Sec-Fetch-Site: same-origin`，但**没有任何 Host 白名单**。经典 DNS rebinding 下浏览器把对 attacker.com（重绑到服务器 IP）的请求视为同源，守卫放行写操作，CORS 也随之允许读响应。GET 读操作根本不做意图检查（`is_state_changing` 排除 GET）。
- **影响**：受害者只需访问恶意网站即可被驱动整个变更 API：读写 provider 密钥、改 MCP 命令、接管 CLI 配置、导出 SQLite。
- **建议**：对所有 `/api/*`（含 GET）强制 Host/authority 白名单（绑定主机 / Tailscale 名），拒绝 Host 不匹配的请求。

### H2. systemd 单元绑定 `HOST=0.0.0.0` 且零认证 → 全网卡暴露全部变更面
- **文件**：`deploy/systemd/cc-switch-web.service`；`scripts/install-cc-switch-web-service.sh:34-40`；`docs/adr/0001-unauthenticated-web-api.md`
- **类别**：security | **分类**：部署/供应链
- **证据**：单元设 `Environment=HOST=0.0.0.0`（注释自认"任何能连到 PORT 的主机都能操作本实例"），安装脚本原样复制并**主动删除旧 auth drop-in**、打印 "Auth: none"。`build_router` 只挂 security_headers + intent。ADR-0001 仍是 proposed，未批准。
- **影响**：在有公网/不可信 LAN 网卡的主机上，任何能到达 :3010 的未认证主机都可窃取 API key、改 MCP、接管 CLI 配置、导入导出数据库。0.0.0.0 并不落实"仅 Tailscale"的假设。
- **建议**：默认绑定 Tailscale 接口地址或 127.0.0.1；0.0.0.0 需显式 opt-in 并要求主机防火墙。（与 H1 是同一无认证根因的两面。）

### H3. 桌面 updater 仍锚定上游仓库 + 上游签名公钥（供应链）
- **文件**：`src-tauri/tauri.conf.json:39,62-67`；`src-tauri/src/lib.rs:300`
- **类别**：security | **分类**：desktop-redundant（上游核心配置，需 fork 侧永久补丁）
- **证据**：`createUpdaterArtifacts: true`，updater endpoints 指向 `github.com/farion1231/cc-switch/releases/.../latest.json` 并用上游 minisign 公钥。updater 插件在桌面 `lib.rs` 中注册（不进 web 二进制）。
- **影响**：本 fork 的任何桌面构建会自动更新为上游签名的二进制，静默回退所有 fork 安全修复；控制上游发布管线者可向用户机器推代码。
- **建议**：把 endpoints+公钥指向 fork 自有发布通道，或在有 fork release 前禁用 updater（`createUpdaterArtifacts:false` + 移除 endpoints），置于 desktop feature 下。

### H4.（= H1/H2 归并项）Web API 全无认证，仅同源意图守卫，绑 0.0.0.0
- **文件**：`routes.rs:28-35`；`intent.rs:1-8`；`deploy/systemd/cc-switch-web.service:16-20`
- **证据**：`middleware/mod.rs` 仅导出 cors/intent/security_headers；`8fea1699` 移除了 auth 层；intent 自述"非访问控制/身份层"，对无 Origin 的直连客户端（curl）放行。
- **影响**：任何能到达 :3010 的主机可用 curl 发未认证变更请求（增删切 provider、改 MCP、`DELETE /api/env/delete-env-vars` 删除 shell 环境变量、驱动接管 CLI token 的路由代理）。
> 说明：H1/H2/H4 是同一"无认证 + 0.0.0.0"根因的不同切面，修复时应一并处理（绑定策略 + 可选认证/Host 白名单）。

---

## MEDIUM（8）

### M1. Gemini settings.json 在严格 JSON 解析失败时被静默清空
- `src-tauri/src/gemini_config.rs:303-309,332` · upstream-bug
- `serde_json::from_str::<Value>(...).unwrap_or_else(|_| json!({}))` 把解析失败转为空对象并写回。Gemini CLI 接受 `//` 注释与尾逗号，serde_json 不接受；用户切换 Gemini provider 时整个 settings.json（mcpServers/theme/telemetry/auth）被替换为 `{}` 起步。
- 建议：解析失败返回错误（对齐 `gemini_mcp::read_json_value`），或用 json5 容错读取。

### M2. Gemini MCP 投影无条件把已有 `timeout` 覆盖为默认 60000
- `src-tauri/src/gemini_mcp.rs:131-153` · upstream-bug
- 只从 `startup_timeout_*`/`tool_timeout_*` 提取，忽略 Gemini 原生 `timeout` 键，line 153 无条件写 `max(10000,60000)=60000`。用户为慢 stdio server 调过的 `timeout:300000` 每次同步被打回 60s。
- 建议：先尊重已有 `timeout`(ms)，仅当三类键都缺失时才写默认。

### M3. Gemini `.env` 重写有损：注释和非规范行永久丢失
- `src-tauri/src/gemini_config.rs:26-50,126-140` · upstream-bug
- `parse_env_file` 丢弃注释/空行结构/非 `[alnum_]+` 键（含 `export FOO=bar`），`serialize_env_file` 只回写存活 map。首次切 Gemini provider 或代理改 env 即不可逆丢失。与已修的 opencode M33 同类。
- 建议：做行级保留编辑，未识别/注释行原样保留，仅更新受管键。

### M4. 定价查不到时在 4 条 session 路径持久化未标记的 $0，rollup 后永久冻结
- `session_usage.rs:423-434`、`session_usage_codex.rs:496-507`、`session_usage_opencode.rs:383-394`、`database/dao/usage_rollup.rs:158-163` · upstream-bug
- `None => ("0",...)` + `INSERT OR IGNORE`，无 `pricing_missing` 列；只有 gemini 路径自愈。用户补上定价后历史行仍 $0，30 天 rollup 后错误总额永久冻结，UI 无"未知定价"标识。
- 建议：加 `pricing_missing` 列（或存 NULL），API/UI 标"未知"，加回填命令，rollup 排除或携带该标志。

### M5. Web 部署浏览器本地日界 vs 服务器本地 rollup 日期，时区不同则 today/区间聚合错位
- `src/lib/usageRange.ts:11-35` vs `usage_stats.rs:323-353,789,1417-1422` · web-bug
- FE 用浏览器午夜算区间起点，后端按服务器本地时间分桶。服务器 UTC、用户 UTC+8 时，已被 rollup 修剪的边界日数据被静默丢弃，today 卡片与趋势图不一致。
- 建议：随 usage API 传客户端 IANA 时区/偏移用于分桶，或 FE 与 rollup 统一 UTC 日并文档化。

### M6. deplink.html 提交了真实泄露的 Context7 API key
- `deplink.html:512,516` · security（upstream 引入）
- 两处 base64 config 载荷解出真实 key `ctx7sk-4ddd4f66-e752-4022-b1f6-c8cf6279b80d`（可读 JSON 用的是占位符，编码载荷是真 key）。deplink.html 不被 web 二进制服务，属源码树/供应链泄露。
- 建议：**立即在 Upstash 吊销/轮换该 key**，两处载荷换成占位符编码；已入 git 历史，按已泄露处理。

### M7. Web 初始化错误恢复 UI 是死接线：db_version_too_new 在 web 上不可达
- `examples/server.rs:271-287`；`web_api/handlers/system.rs:100-101`；`src/main.tsx:80-97` · web-bug
- server.rs 设 `set_init_error` 后立即 `return Err`，进程在 Axum 监听启动前退出，`get_init_error` 永远返回 null，`<DatabaseUpgrade>` 恢复屏在 web 永不显示。共享数据目录被新 schema 触过时 web 只会启动即死+systemd 重启循环。
- 建议：web 上启动最小降级服务器（静态资源 + get_init_error）让恢复屏可用；或删除死代码并文档化只经日志/退出码暴露。

### M8. Web 上"日志设置"无持久效果：持久化 LogConfig 启动不加载 + env_logger 过滤封顶
- `examples/server.rs:439-443`；`web_api/handlers/config.rs:663-672`；`lib.rs:588-597` · web-bug
- web 的 `init_logging` 只用 `RUST_LOG` 默认值且从不读 DB log config；`set_log_config` 调 `set_max_level` 但 env_logger 指令过滤在 init 时定死，提高 verbosity 无效、重启即丢；`cc_switch=debug` 指令对 example crate 名不匹配。
- 建议：server.rs 在 DB init 后读 `get_log_config()` 构建过滤器；修死指令；或把面板标为 desktop-only 直到 web 接线。

---

## LOW（20）

**路由/闸门稳健性（上游合并漂移风险）**
- L1 route-coverage 闸门接受仅被 501 `web_not_supported` 通配符覆盖的命令，CI 从不加 `--fail-on-parity-fallback`（`check-web-route-coverage.mjs:160-166`）。
- L2 闸门只比路径不比 HTTP 方法，方法不匹配过 CI 但运行时 405（同脚本:142-158）。
- L3 `webReplacement` 豁免使真正的 upload/download/update-info 端点不被任何闸门校验（`settings.ts:126,154` 等硬编码路径）。
- L4 注册在非 parity.rs 文件的精确路径 501 stub 被计为完整覆盖，`missing:0` 高估了 parity。
- L5 参数/响应形状分歧无闸门覆盖：上游同步改字段名/类型可过全部闸门、仅 web 运行时出错。

**死路由 / 桌面冗余（隔离候选）**
- L6 `open_external`/`open_hermes_web_ui` web 路由必失败（ChannelEventSink 不实现 `open_url`）。
- L7 `web_credentials` PUT 死 501 + adapter 里过时的 Basic-Auth 注释（认证已被移除）。
- L8 `update_tray_menu` web 恒返回 true（web 无 tray）等桌面 shell 死路由——注：`open_url` 未来切勿实现服务器端 xdg-open，否则成命令执行面。
- L9 `set_auto_launch`/`set_window_theme` 映射到只存在为 parity 501 的路由；`useSettings.ts` 未 gate 可致导入桌面设置时误报错误提示。
- L10 `restart_app` web 是静默 no-op 却报成功，用户以为已重启（`system.rs:123-125`）。
- L11 web 更新检查把 fork 版本比上游 release 并引导用户去上游下载（`web_update.rs:5`）。
- **L12（信息项，无需动作）**：已验证 tray/auto_launch/linux_fix/updater/单实例/窗口管理**未**编译进 web 二进制，deeplink 核心是有意共享且 tauri-free——隔离正确。唯一残留是上游合并漂移，需把 web-feature cargo check 纳入 CI。

**上游核心正确性**
- L13 codex TOML 用字符串插值无转义，model/base_url 含引号会损坏或注入 config.toml（`provider.rs:560-593`）。
- L14 MCP v3.7.0 迁移不清理旧 `mcp.opencode`，留重复陈旧数据（`app_config.rs:911-913`）。
- L15 `write_codex_live_atomic` 读旧 config 用于回滚却从不使用；auth 回滚错误被 `let _ =` 吞掉。
- L16 `update_selected_type` 在 settings.json 根非对象时静默 no-op 仍报成功。
- L17 session 路径 `LIKE '{model}%'` 定价回退无 ORDER BY、不转义 `%`/`_`，定价不确定且与代理路径分歧（`session_usage.rs:506-521`）。
- L18 session/proxy 去重在 rollup 修剪边界跨午夜断裂 → 同请求重复计数（`usage_stats.rs:195-229`）。

**一致性 / CI / 加固**
- L19 FE OpenClaw `deepseek-v4-pro` 预设 1.68/3.36 与 Rust seed 修复后 0.435/0.87 分歧（约 4x）；另有 0.001/0.001 占位垃圾价（`openclawProviderPresets.ts:273`）。
- L20 web-only Rust 代码无 clippy 闸门（example 被跳过 + web job `RUSTFLAGS=-Awarnings`）；`server.rs:26-34` 双构建契约文档已过时与 ci.yml 矛盾；`treemap.html`+非 prod sourcemap 被静态服务；codex OAuth refresh token 明文 0600 存盘（always-on 服务器延长暴露窗口，建议排除出 WebDAV/S3 同步 + SECURITY.md 备份指引）。

---

## 建议的修复批次（供确认）

1. **批次 A — 安全根因（High）**：绑定策略默认 Tailscale/环回 + 可选认证/Host 白名单（H1/H2/H4 一并）；updater 指向 fork 或禁用（H3）；轮换 + 清理 Context7 key（M6）。
2. **批次 B — 上游数据丢失（Medium，贴上游最小改）**：gemini settings.json/`.env`/MCP timeout 三项保留式改写（M1/M2/M3）；定价 $0 标记 + 回填（M4）；TZ 传客户端时区（M5）。
3. **批次 C — web 死接线/无效设置（Medium）**：init-error 降级服务（M7）；log 设置接线或标 desktop-only（M8）。
4. **批次 D — 闸门与隔离硬化（Low，防上游合并回归）**：route-coverage 闸门补方法/通配/webReplacement/parity-fallback（L1-L5）；web-feature clippy + cargo check 纳入 CI（L12/L20）；死路由改 `desktop_only` 显式响应（L6-L11）。
5. **批次 E — 上游核心正确性 + 一致性（Low）**：codex TOML 转义、MCP 迁移清理、LIKE 定价、预设价对齐等（L13-L19）。
