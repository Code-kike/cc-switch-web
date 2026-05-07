# Web 用量查询与 Hermes 供应商状态修复设计

日期：2026-05-07

## 背景

当前项目由 `cc-switch` 和 `cc-switch-web` 演进而来，Web route 覆盖已经基本完整，但页面行为仍存在桌面/Web 分叉。用户测试发现两个问题：

- Web 页面中的“配置用量查询”功能未完整实现。
- Hermes 页面中供应商添加、启用、移除、当前状态，以及 Hermes 供应商用量查询仍有问题。

代码阅读结论：

- Web `queryProviderUsage` handler 当前直接调用 `ProviderService::query_usage()`，只执行保存的 JS 脚本路径。
- 桌面 `queryProviderUsage` 额外处理 `github_copilot`、`token_plan`、`balance` 三个内置模板。
- `token_plan` 和 `balance` 模板保存的脚本内容为空，因此 Web 从卡片或底部刷新时会错误落入 JS 执行路径。
- 现有服务层凭据提取主要覆盖 Claude/Gemini 风格的 `env` 字段，不完整支持 Hermes `api_key/base_url`、OpenClaw `apiKey/baseUrl`、OpenCode `options.apiKey/baseURL`、Codex TOML `base_url`。
- Hermes 页面已有 live provider ids、model config、Memory 等 route，但 provider 生命周期页面测试不足，尤其是添加到 live、设为当前、移除后状态刷新、只读 providers dict 与用量查询的组合。

## 目标

实现 Web 与桌面一致的供应商用量查询行为，并修复 Hermes 供应商管理闭环。

完成后应满足：

- Web `queryProviderUsage` 支持 `github_copilot`、`token_plan`、`balance` 和 JS 自定义脚本。
- 内置模板不再因脚本为空而进入 JS 执行并报缺少 `request` 配置。
- Hermes provider 使用 `settingsConfig.api_key/base_url` 作为用量查询凭据来源。
- Hermes provider 添加、启用、移除、当前状态显示与 `~/.hermes/config.yaml` 保持同步。
- Hermes 当前供应商状态以 live config 的 `model.provider` 为准，不依赖 additive app 的 DB `is_current`。
- Web 查询结果通过 SSE 更新 React Query 缓存，桌面保留 Tauri event 与托盘缓存刷新。

## 非目标

本轮不处理以下范围：

- 不重做 Hermes Memory、MCP、Skills、Sessions。
- 不改变 Hermes Web UI/Dashboard 在 Web 模式下的远程提示策略，除非实现中发现它阻塞 provider 状态闭环。
- 不新增新的用量模板。
- 不改变 Usage Script Modal 的 UI 结构。
- 不重新设计 Hermes v12+ `providers:` dict 的只读策略。

## 方案

采用“共享服务收口 + Hermes 页面闭环测试”的方案。

### 统一用量查询调度器

在 Rust 服务层新增或重构统一入口，例如：

```rust
ProviderService::query_usage_with_templates(...)
```

该入口负责：

- 读取 provider 与 `meta.usage_script`。
- 检查 usage script 是否存在、是否启用。
- 根据 `template_type` 分发：
  - `github_copilot`：走 Copilot 专用查询。
  - `token_plan`：走 `coding_plan::get_coding_plan_quota`。
  - `balance`：走 `balance::get_balance`。
  - 其他模板：走现有 JS script 执行。
- 统一返回 `UsageResult { success, data, error }`。
- 查询失败时也生成失败快照，避免旧成功结果滞留。

桌面命令和 Web handler 都调用该统一入口：

- 桌面保留 Tauri event、UsageCache、tray refresh 逻辑，但不再重复模板分发。
- Web handler 使用 `state.sink.emit_json("usage-cache-updated", payload)` 通过 SSE 更新前端缓存。

### App-aware 凭据提取

新增应用感知的用量凭据提取逻辑。优先级：

1. `usage_script` 中显式填写的 `api_key/base_url/access_token/user_id`。
2. provider 的 app-specific `settingsConfig`。

支持格式：

- Claude：`env.ANTHROPIC_AUTH_TOKEN` / `env.ANTHROPIC_API_KEY`，`env.ANTHROPIC_BASE_URL`
- Codex：`auth.OPENAI_API_KEY`，TOML `base_url`
- Gemini：`env.GEMINI_API_KEY`，`env.GOOGLE_GEMINI_BASE_URL`
- OpenCode：`settingsConfig.options.apiKey/baseURL`
- OpenClaw：`settingsConfig.apiKey/baseUrl`
- Hermes：`settingsConfig.api_key/base_url`

该逻辑供内置模板和 JS fallback 共同使用，减少前端测试按钮可用但保存后卡片查询失败的问题。

### Hermes provider 状态闭环

Hermes 仍保持 additive mode，不引入覆盖式当前供应商概念。

页面状态规则：

- `isInConfig` 来自 Hermes live provider ids。
- 当前供应商来自 `get_hermes_model_config()` 的 `model.provider`。
- 蓝色边框、“已在用”按钮状态、`onSetAsDefault` 语义都以 `model.provider` 为准。
- 添加或启用 provider 后写入 `custom_providers`，并调用 `apply_switch_defaults` 更新 `model.provider/model.default`。
- 移除 provider 后刷新 live ids 与 model config；如果移除项是当前 provider，页面必须显示一致的降级状态。
- Hermes `providers:` dict 来源的 provider 保持只读编辑/删除限制，但允许配置和执行用量查询。

### 前端缓存失效

Hermes add/switch/remove 成功后失效：

- `["providers", "hermes"]`
- `hermesKeys.liveProviderIds`
- `hermesKeys.modelConfig`
- 相关 provider usage query key

Web 用量查询成功或失败后通过 SSE `usage-cache-updated` 写回 React Query：

- `kind: "script"`
- `appType`
- `providerId`
- `data: UsageResult`

## 错误处理

- provider 不存在、usage script 未配置、usage script 未启用：保持当前错误语义。
- `balance` 和 `token_plan` 的业务失败返回 `UsageResult.success=false`，用于卡片展示可刷新错误态。
- DB、route、认证管理器等 transport-level 异常可继续抛出，但需要生成失败快照用于缓存事件。
- JS 自定义脚本保持现有安全校验和错误格式。
- Hermes read-only provider 只限制编辑/删除 live 配置，不限制用量查询。

## 测试计划

### Rust 测试

- 统一调度器能识别 `balance`、`token_plan`、custom/JS 模板。
- `balance` 和 `token_plan` 空脚本不会进入 JS 执行路径。
- Hermes `settingsConfig.api_key/base_url` 能被凭据提取。
- Claude、Codex、Gemini、OpenCode、OpenClaw 的凭据提取不回退。
- 失败结果形成 `UsageResult.success=false`，而不是 JS `Missing request config`。

### Web-server 页面/集成测试

- 创建或导入 Hermes provider 后，卡片显示 `Live Config`。
- 点击添加/启用后，`~/.hermes/config.yaml` 写入 `custom_providers`，并更新 `model.provider`。
- 当前 Hermes provider 蓝框和“已在用”状态来自 `model.provider`。
- 移除 provider 后 live ids 刷新，卡片回到 DB-only 状态。
- Hermes provider 保存 `balance` 用量查询后，通过真实 `/api/providers/queryproviderusage` 返回内置模板结果或业务错误，而不是 JS script request 缺失错误。

### 前端组件测试

- `UsageScriptModal` 对 Hermes 使用 `api_key/base_url`。
- `ProviderList` 对 Hermes 的 `isInConfig`、`isDefaultModel`、`onSetAsDefault` 状态刷新。
- Hermes `providers:` dict 只读供应商禁用编辑/删除，但不阻止用量查询配置入口。

## 验收标准

- Web 和桌面 `queryProviderUsage` 的模板行为一致。
- Web Hermes provider 可完成添加、启用/设为当前、移除、状态刷新和用量查询。
- 内置模板保存后卡片/底部刷新不再报 JS request 配置缺失。
- 相关 Rust、组件、Web-server 测试通过。
- `pnpm check:web-routes -- --list-parity` 不出现新增缺口。
- `pnpm typecheck`、`pnpm build:web`、`cargo check --no-default-features --features web-server --example server` 可通过，或失败原因记录在后续执行计划中。
