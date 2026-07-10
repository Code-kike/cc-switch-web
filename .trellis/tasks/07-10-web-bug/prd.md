# PRD: 修复 web 项目上游遗留 bug 与冗余功能（不含认证）

## 背景
2026-07-10 对 cc-switch-web（`fix/web-audit-phase1-2` 分支）做了 46-agent 全量深审，
产出分级报告 `docs/audit/2026-07-10-full-audit.md`（4 High / 8 Medium / 20 Low，均对抗性验证）。
本任务实现用户确认的修复范围。

## 威胁模型与总原则
- 单用户私网（Tailscale）。**接受无认证、不加白名单；认证类工作一概不做（含今后）**。
- 桌面冗余功能**只隔离不删除**（feature-gate，保留桌面 Tauri 构建）。
- **继续跟随上游**：贴上游最小改，优先落 web 层 / 新文件，减少 merge 冲突。

## ❌ 明确不做（认证类）
H1 / H2 / H4：无认证、`HOST=0.0.0.0` 绑定、Host 白名单、DNS-rebinding 防护、
same-origin intent 守卫改动——全部保持现状。今后也不做安全认证工作。

## ✅ 修复范围（27 项，分 5 批）

### 批 A' — 供应链/密钥（最高优先，独立）
- **H3 + L11** 禁用桌面 updater：`tauri.conf.json` `createUpdaterArtifacts:false` + 移除
  `plugins.updater.endpoints`；web 模式隐藏更新检查 UI（`web_update.rs` / `updater-adapter.ts` /
  `AboutSection.tsx` / `DatabaseUpgrade.tsx`）。
- **M6** deplink.html:512,516 base64 载荷内真实 Context7 key `ctx7sk-4ddd4f66-...` 换占位符编码。
  ⚠️ **需用户手动在 Upstash 吊销/轮换该 key**（已入 git 历史，视为已泄露）。
- **L20** codex OAuth refresh token（`codex_oauth_auth.json`，已 0600）排除出 WebDAV/S3 同步载荷 +
  SECURITY.md 备份排除指引。

### 批 B — 上游数据丢失（Medium，贴上游最小改）
- **M1** `gemini_config.rs:306` settings.json 非严格 JSON 解析失败被替换为 `{}` → 返错 / json5 容错读。
- **M2** `gemini_mcp.rs:153` MCP timeout 无条件覆盖为 60000 → 尊重已有 `timeout` 键。
- **M3** `gemini_config.rs:26-50` `.env` 有损重写丢注释/export 行 → 行级保留编辑（对齐 opencode M33）。
- **M4** 定价 miss 持久化未标记 $0（`session_usage*.rs` + `usage_rollup.rs`）→ 加 `pricing_missing` 列
  （或存 NULL）+ API/UI 标"未知" + 回填命令 + rollup 排除/携带标志。
- **M5** web 浏览器本地日界 vs 服务器本地 rollup 日期错位（`usageRange.ts` vs `usage_stats.rs`）→
  随 usage API 传客户端 IANA 时区/偏移用于分桶。

### 批 C — web 死接线/无效设置（Medium）
- **M7** init-error 恢复 UI 死接线（`server.rs:271-287` 提前 `return Err`）→ web 启动最小降级服务器
  （静态资源 + `get_init_error`），或删死代码并文档化。
- **M8** web 日志设置无持久效果（`server.rs:439` 不读 DB log config；env_logger 过滤定死；
  死 `cc_switch=` 指令）→ DB init 后读 `get_log_config()` 构建过滤器 + 修死指令。

### 批 D — 闸门与隔离硬化（Low，防上游合并回归）
- **L1** route-coverage 闸门接受 501 通配符覆盖 → CI 加 `--fail-on-parity-fallback`。
- **L2** 闸门只比路径不比方法 → 正则捕获 method 并比对。
- **L3** `webReplacement` 豁免使真实端点不校验 → 把 webFetch 硬编码路径纳入闸门。
- **L4** 非 parity.rs 的精确 501 stub 计为完整覆盖 → 按 handler 符号/返回类型检测 stub。
- **L5** 参数/响应形状分歧无闸门 → manifest 生成参数级 diff 或加集成测试清单规则。
- **L6/L8/L9/L10** 死路由改 `desktop_only` 显式响应（open_external/open_hermes_web_ui、
  update_tray_menu、set_auto_launch/set_window_theme + gate useSettings 副作用、restart_app）。
- **L7** 移除 `web_credentials` 死 501 路由 + 更新 `adapter.ts` 过时 Basic-Auth 注释（死代码清理）。
- **L12** web-feature cargo check 纳入 CI（隔离防漂移）。
- **L20(CI)** web-only Rust 加 clippy 闸门（去 `-Awarnings`）；更新 `server.rs:26-34` 过时双构建契约文档；
  treemap.html/非 prod sourcemap 不进服务目录。

### 批 E — 上游核心正确性 + 一致性（Low）
- **L13** `provider.rs:560-593` codex TOML 字符串插值无转义 → `toml_edit`/`toml::Table` 序列化。
- **L14** `app_config.rs:911-913` MCP 迁移不清理 `mcp.opencode` → 加 reset。
- **L15** `codex_config.rs:172-176,190-199` 死读 `_old_config` + 吞回滚错误 → 删死读 + 记日志。
- **L16** `gemini_config.rs:312-329` settings.json 根非对象静默 no-op → 返错。
- **L17** `session_usage.rs:506-521` LIKE 定价回退不确定/不转义 → 去 LIKE 或转义+ORDER BY 对齐代理路径。
- **L18** `usage_stats.rs:195-229` dedup 跨 rollup 修剪边界双计 → 修剪时一并处理 dedup-match session twin。
- **L19** `openclawProviderPresets.ts:273` deepseek-v4-pro 1.68/3.36 vs seed 0.435/0.87 → 对齐 +
  审 0.001 占位价 + 加预设↔seed 一致性测试。

## 验收
- 每批实现后跑 gate suite（`cargo fmt --check` + `pnpm format:check` + web cargo check +
  check:web-routes/locales + 相关测试），过门后提交。
- 认证类项零改动（H1/H2/H4 diff 必须为空）。
- 桌面构建能力保留（feature-gate 隔离，不删桌面代码）。
- M6 的 key 轮换由用户手动完成，任务中标注为外部依赖。
