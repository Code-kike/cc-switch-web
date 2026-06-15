use rquickjs::{Context, Function, Runtime};
use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use url::{Host, Url};

use crate::error::AppError;

const JS_EXECUTION_TIMEOUT: Duration = Duration::from_secs(2);

fn create_bounded_js_runtime() -> Result<(Runtime, Instant), AppError> {
    let runtime = Runtime::new().map_err(|e| {
        AppError::localized(
            "usage_script.runtime_create_failed",
            format!("创建 JS 运行时失败: {e}"),
            format!("Failed to create JS runtime: {e}"),
        )
    })?;
    let started_at = Instant::now();
    let deadline = started_at + JS_EXECUTION_TIMEOUT;
    runtime.set_interrupt_handler(Some(Box::new(move || Instant::now() >= deadline)));
    Ok((runtime, started_at))
}

fn js_timeout_error(zh_stage: &str, en_stage: &str) -> AppError {
    AppError::localized(
        "usage_script.execution_timeout",
        format!(
            "JS 脚本执行超时（{}，超过 {} 秒）",
            zh_stage,
            JS_EXECUTION_TIMEOUT.as_secs()
        ),
        format!(
            "JS script execution timed out ({} exceeded {} seconds)",
            en_stage,
            JS_EXECUTION_TIMEOUT.as_secs()
        ),
    )
}

fn map_js_error_or_timeout(
    started_at: Instant,
    zh_stage: &str,
    en_stage: &str,
    fallback_key: &'static str,
    zh_prefix: &str,
    en_prefix: &str,
    error: rquickjs::Error,
) -> AppError {
    if started_at.elapsed() >= JS_EXECUTION_TIMEOUT {
        return js_timeout_error(zh_stage, en_stage);
    }

    AppError::localized(
        fallback_key,
        format!("{zh_prefix}: {error}"),
        format!("{en_prefix}: {error}"),
    )
}

/// Best-effort extraction of `request.url` from a usage script's JS body, used
/// at save-time to reject placeholder scripts that would always fail at
/// execute-time (`Url::parse("")` → "relative URL without a base").
///
/// Substitutes the standard placeholders with sentinel values so a templated
/// URL like `{{baseUrl}}/balance` survives extraction. Returns:
/// - `Ok(Some(url))`  when the script parses and exposes a string URL
/// - `Ok(None)`       when the script does not declare `request.url` (caller
///   decides whether that's a save-blocking issue)
/// - `Err(_)`         only on JS runtime/eval failures the caller should log
///
/// Built-in templates (`balance` / `token_plan` / `github_copilot`) ignore the
/// JS body entirely, so callers should skip this check for those.
pub fn try_extract_request_url(script_code: &str) -> Result<Option<String>, AppError> {
    let prepared = script_code
        .replace("{{baseUrl}}", "https://placeholder.invalid")
        .replace("{{apiKey}}", "placeholder-key")
        .replace("{{accessToken}}", "placeholder-token")
        .replace("{{userId}}", "placeholder-user");

    let (runtime, js_started_at) = create_bounded_js_runtime()?;
    let context = Context::full(&runtime).map_err(|e| {
        AppError::localized(
            "usage_script.context_create_failed",
            format!("创建 JS 上下文失败: {e}"),
            format!("Failed to create JS context: {e}"),
        )
    })?;

    let url_result: Result<Option<String>, AppError> = context.with(|ctx| {
        let config: rquickjs::Object = ctx.eval(prepared).map_err(|e| {
            map_js_error_or_timeout(
                js_started_at,
                "保存前解析 request.url",
                "save-time request.url extraction",
                "usage_script.config_parse_failed",
                "解析配置失败",
                "Failed to parse config",
                e,
            )
        })?;

        let request: rquickjs::Object = match config.get("request") {
            Ok(req) => req,
            Err(_) => return Ok(None),
        };
        let url_value: rquickjs::Value = match request.get("url") {
            Ok(v) => v,
            Err(_) => return Ok(None),
        };
        let Some(js_string) = url_value.as_string() else {
            return Ok(None);
        };
        let url = js_string.to_string().map_err(|e| {
            AppError::localized(
                "usage_script.get_string_failed",
                format!("获取字符串失败: {e}"),
                format!("Failed to get string: {e}"),
            )
        })?;
        Ok(Some(url))
    });
    url_result
}

/// 执行用量查询脚本
///
/// `enforce_outbound_guard` (audit FIX 1): web-runtime-only SSRF gate. When
/// `true` the actually-dialed `request.url` is validated against the tauri-free
/// `proxy::ip_guard::guard_outbound_url` SSOT before the dial, and the request
/// goes through the redirect-hardened `http_client::get_guarded()` client so
/// every redirect hop is re-checked too. Desktop callers pass `false` to keep
/// dialing local endpoints unrestricted (behavior unchanged).
#[allow(clippy::too_many_arguments)]
pub async fn execute_usage_script(
    script_code: &str,
    api_key: &str,
    base_url: &str,
    timeout_secs: u64,
    access_token: Option<&str>,
    user_id: Option<&str>,
    template_type: Option<&str>,
    enforce_outbound_guard: bool,
) -> Result<Value, AppError> {
    // 检测是否为自定义模板模式
    // 优先使用前端传递的 template_type
    let is_custom_template = template_type.map(|t| t == "custom").unwrap_or(false);

    // 1. 替换模板变量，避免泄露敏感信息
    let script_with_vars =
        build_script_with_vars(script_code, api_key, base_url, access_token, user_id);

    // 2. 验证 base_url 的安全性（仅当提供了 base_url 时）
    // 自定义模板模式下，用户可能不使用模板变量，而是直接在脚本中写完整 URL
    if should_validate_base_url(base_url, is_custom_template) {
        validate_base_url(base_url)?;
    }

    // 3. 在独立作用域中提取 request 配置（确保 Runtime/Context 在 await 前释放）
    //
    // L22 — Why the script body is eval'd twice (here and again at step 7):
    // rquickjs is built without the `parallel` feature, so `Runtime`/`Context`
    // and every `'js` value (`Object`/`Function`/...) are `!Send`. This async
    // fn is awaited from both an Axum handler and Tauri commands, so its future
    // MUST be `Send` — the JS runtime therefore cannot live across the
    // `send_http_request().await` (step 6) that sits between extracting the
    // request config and running the `extractor`. The `extractor` is a live JS
    // `Function` bound to this context's lifetime; it cannot be serialized or
    // moved out, so step 7 must re-eval the script to reconstruct it in a fresh
    // runtime. This double-eval is intentional and required for `Send`-safety,
    // NOT a perf bug to "optimize" by merging the two scopes. Only the phase-1
    // `request` is sent over the wire, so the outbound request is deterministic.
    let request_config = {
        let (runtime, js_started_at) = create_bounded_js_runtime()?;
        let context = Context::full(&runtime).map_err(|e| {
            AppError::localized(
                "usage_script.context_create_failed",
                format!("创建 JS 上下文失败: {e}"),
                format!("Failed to create JS context: {e}"),
            )
        })?;

        context.with(|ctx| {
            // 执行用户代码，获取配置对象
            let config: rquickjs::Object = ctx.eval(script_with_vars.clone()).map_err(|e| {
                map_js_error_or_timeout(
                    js_started_at,
                    "解析 request 配置",
                    "request config parsing",
                    "usage_script.config_parse_failed",
                    "解析配置失败",
                    "Failed to parse config",
                    e,
                )
            })?;

            // 提取 request 配置
            let request: rquickjs::Object = config.get("request").map_err(|e| {
                AppError::localized(
                    "usage_script.request_missing",
                    format!("缺少 request 配置: {e}"),
                    format!("Missing request config: {e}"),
                )
            })?;

            // 将 request 转换为 JSON 字符串
            let request_json: String = ctx
                .json_stringify(request)
                .map_err(|e| {
                    map_js_error_or_timeout(
                        js_started_at,
                        "序列化 request 配置",
                        "request config serialization",
                        "usage_script.request_serialize_failed",
                        "序列化 request 失败",
                        "Failed to serialize request",
                        e,
                    )
                })?
                .ok_or_else(|| {
                    AppError::localized(
                        "usage_script.serialize_none",
                        "序列化返回 None",
                        "Serialization returned None",
                    )
                })?
                .get()
                .map_err(|e| {
                    AppError::localized(
                        "usage_script.get_string_failed",
                        format!("获取字符串失败: {e}"),
                        format!("Failed to get string: {e}"),
                    )
                })?;

            Ok::<_, AppError>(request_json)
        })?
    }; // Runtime 和 Context 在这里被 drop

    // 4. 解析 request 配置
    let request: RequestConfig = serde_json::from_str(&request_config).map_err(|e| {
        AppError::localized(
            "usage_script.request_format_invalid",
            format!("request 配置格式错误: {e}"),
            format!("Invalid request config format: {e}"),
        )
    })?;

    // 5. 验证请求 URL（HTTPS 强制 + 同源检查）
    validate_request_url(&request.url, base_url, is_custom_template)?;

    // 6. 发送 HTTP 请求
    let response_data = send_http_request(&request, timeout_secs, enforce_outbound_guard).await?;

    // 7. 在独立作用域中执行 extractor（确保 Runtime/Context 在函数结束前释放）
    // Second eval of the same script — see the L22 note at step 3 for why a
    // fresh `!Send` runtime is required here rather than reusing step 3's.
    let result: Value = {
        let (runtime, js_started_at) = create_bounded_js_runtime()?;
        let context = Context::full(&runtime).map_err(|e| {
            AppError::localized(
                "usage_script.context_create_failed",
                format!("创建 JS 上下文失败: {e}"),
                format!("Failed to create JS context: {e}"),
            )
        })?;

        context.with(|ctx| {
            // 重新 eval 获取配置对象
            let config: rquickjs::Object = ctx.eval(script_with_vars.clone()).map_err(|e| {
                map_js_error_or_timeout(
                    js_started_at,
                    "重新解析 extractor 配置",
                    "extractor config re-parse",
                    "usage_script.config_reparse_failed",
                    "重新解析配置失败",
                    "Failed to re-parse config",
                    e,
                )
            })?;

            // 提取 extractor 函数
            let extractor: Function = config.get("extractor").map_err(|e| {
                AppError::localized(
                    "usage_script.extractor_missing",
                    format!("缺少 extractor 函数: {e}"),
                    format!("Missing extractor function: {e}"),
                )
            })?;

            // 将响应数据转换为 JS 值
            let response_js: rquickjs::Value =
                ctx.json_parse(response_data.as_str()).map_err(|e| {
                    AppError::localized(
                        "usage_script.response_parse_failed",
                        format!("解析响应 JSON 失败: {e}"),
                        format!("Failed to parse response JSON: {e}"),
                    )
                })?;

            // 调用 extractor(response)
            let result_js: rquickjs::Value = extractor.call((response_js,)).map_err(|e| {
                map_js_error_or_timeout(
                    js_started_at,
                    "执行 extractor",
                    "extractor execution",
                    "usage_script.extractor_exec_failed",
                    "执行 extractor 失败",
                    "Failed to execute extractor",
                    e,
                )
            })?;

            // 转换为 JSON 字符串
            let result_json: String = ctx
                .json_stringify(result_js)
                .map_err(|e| {
                    map_js_error_or_timeout(
                        js_started_at,
                        "序列化 extractor 结果",
                        "extractor result serialization",
                        "usage_script.result_serialize_failed",
                        "序列化结果失败",
                        "Failed to serialize result",
                        e,
                    )
                })?
                .ok_or_else(|| {
                    AppError::localized(
                        "usage_script.serialize_none",
                        "序列化返回 None",
                        "Serialization returned None",
                    )
                })?
                .get()
                .map_err(|e| {
                    AppError::localized(
                        "usage_script.get_string_failed",
                        format!("获取字符串失败: {e}"),
                        format!("Failed to get string: {e}"),
                    )
                })?;

            // 解析为 serde_json::Value
            serde_json::from_str(&result_json).map_err(|e| {
                AppError::localized(
                    "usage_script.json_parse_failed",
                    format!("JSON 解析失败: {e}"),
                    format!("JSON parse failed: {e}"),
                )
            })
        })?
    }; // Runtime 和 Context 在这里被 drop

    // 8. 验证返回值格式
    validate_result(&result)?;

    Ok(result)
}

/// 请求配置结构
#[derive(Debug, serde::Deserialize)]
struct RequestConfig {
    url: String,
    method: String,
    #[serde(default)]
    headers: HashMap<String, String>,
    #[serde(default)]
    body: Option<String>,
}

/// 发送 HTTP 请求
///
/// `enforce_outbound_guard` (audit FIX 1): web-runtime-only. When `true`, the
/// final `config.url` is validated against `proxy::ip_guard::guard_outbound_url`
/// (reject non-http(s) schemes + internal/private/loopback/CGNAT targets)
/// BEFORE dialing, and the request uses the redirect-hardened `get_guarded()`
/// client. Desktop callers pass `false` and keep the unguarded `get()` client.
async fn send_http_request(
    config: &RequestConfig,
    timeout_secs: u64,
    enforce_outbound_guard: bool,
) -> Result<String, AppError> {
    // Web-runtime SSRF guard: validate the actually-dialed URL (not just the
    // provider base_url) before connecting. Custom-template scripts skip the
    // HTTPS/same-origin checks above, so this is the only barrier against a
    // script-controlled `request.url` reaching internal endpoints.
    if enforce_outbound_guard {
        crate::proxy::ip_guard::guard_outbound_url(&config.url)
            .await
            .map_err(map_outbound_guard_error)?;
    }

    // 使用全局 HTTP 客户端（已包含代理配置）。Web 守护模式下使用重定向逐跳复检的
    // guarded 客户端，防止公网 URL 通过 30x 重定向到内网。
    let client = if enforce_outbound_guard {
        crate::proxy::http_client::get_guarded()
    } else {
        crate::proxy::http_client::get()
    };
    // 约束超时范围，防止异常配置导致长时间阻塞（最小 2 秒，最大 30 秒）
    let request_timeout = std::time::Duration::from_secs(timeout_secs.clamp(2, 30));

    // 严格校验 HTTP 方法，非法值不回退为 GET
    let method: reqwest::Method = config.method.parse().map_err(|_| {
        AppError::localized(
            "usage_script.invalid_http_method",
            format!("不支持的 HTTP 方法: {}", config.method),
            format!("Unsupported HTTP method: {}", config.method),
        )
    })?;

    let mut req = client
        .request(method.clone(), &config.url)
        .timeout(request_timeout);

    // 添加请求头
    for (k, v) in &config.headers {
        req = req.header(k, v);
    }

    // 添加请求体
    if let Some(body) = &config.body {
        req = req.body(body.clone());
    }

    // 发送请求
    let resp = req.send().await.map_err(|e| {
        AppError::localized(
            "usage_script.request_failed",
            format!("请求失败: {e}"),
            format!("Request failed: {e}"),
        )
    })?;

    let status = resp.status();
    let text = resp.text().await.map_err(|e| {
        AppError::localized(
            "usage_script.read_response_failed",
            format!("读取响应失败: {e}"),
            format!("Failed to read response: {e}"),
        )
    })?;

    if !status.is_success() {
        let preview = if text.len() > 200 {
            let mut safe_cut = 200usize;
            while !text.is_char_boundary(safe_cut) {
                safe_cut = safe_cut.saturating_sub(1);
            }
            format!("{}...", &text[..safe_cut])
        } else {
            text.clone()
        };
        return Err(AppError::localized(
            "usage_script.http_error",
            format!("HTTP {status} : {preview}"),
            format!("HTTP {status} : {preview}"),
        ));
    }

    Ok(text)
}

/// 验证脚本返回值（支持单对象或数组）
fn validate_result(result: &Value) -> Result<(), AppError> {
    // 如果是数组，验证每个元素
    if let Some(arr) = result.as_array() {
        if arr.is_empty() {
            return Err(AppError::localized(
                "usage_script.empty_array",
                "脚本返回的数组不能为空",
                "Script returned empty array",
            ));
        }
        for (idx, item) in arr.iter().enumerate() {
            validate_single_usage(item).map_err(|e| {
                AppError::localized(
                    "usage_script.array_validation_failed",
                    format!("数组索引[{idx}]验证失败: {e}"),
                    format!("Validation failed at index [{idx}]: {e}"),
                )
            })?;
        }
        return Ok(());
    }

    // 如果是单对象，直接验证（向后兼容）
    validate_single_usage(result)
}

/// 验证单个用量数据对象
fn validate_single_usage(result: &Value) -> Result<(), AppError> {
    let obj = result.as_object().ok_or_else(|| {
        AppError::localized(
            "usage_script.must_return_object",
            "脚本必须返回对象或对象数组",
            "Script must return object or array of objects",
        )
    })?;

    // 所有字段均为可选，只进行类型检查
    if obj.contains_key("isValid")
        && !result["isValid"].is_null()
        && !result["isValid"].is_boolean()
    {
        return Err(AppError::localized(
            "usage_script.isvalid_type_error",
            "isValid 必须是布尔值或 null",
            "isValid must be boolean or null",
        ));
    }
    if obj.contains_key("invalidMessage")
        && !result["invalidMessage"].is_null()
        && !result["invalidMessage"].is_string()
    {
        return Err(AppError::localized(
            "usage_script.invalidmessage_type_error",
            "invalidMessage 必须是字符串或 null",
            "invalidMessage must be string or null",
        ));
    }
    if obj.contains_key("remaining")
        && !result["remaining"].is_null()
        && !result["remaining"].is_number()
    {
        return Err(AppError::localized(
            "usage_script.remaining_type_error",
            "remaining 必须是数字或 null",
            "remaining must be number or null",
        ));
    }
    if obj.contains_key("unit") && !result["unit"].is_null() && !result["unit"].is_string() {
        return Err(AppError::localized(
            "usage_script.unit_type_error",
            "unit 必须是字符串或 null",
            "unit must be string or null",
        ));
    }
    if obj.contains_key("total") && !result["total"].is_null() && !result["total"].is_number() {
        return Err(AppError::localized(
            "usage_script.total_type_error",
            "total 必须是数字或 null",
            "total must be number or null",
        ));
    }
    if obj.contains_key("used") && !result["used"].is_null() && !result["used"].is_number() {
        return Err(AppError::localized(
            "usage_script.used_type_error",
            "used 必须是数字或 null",
            "used must be number or null",
        ));
    }
    if obj.contains_key("planName")
        && !result["planName"].is_null()
        && !result["planName"].is_string()
    {
        return Err(AppError::localized(
            "usage_script.planname_type_error",
            "planName 必须是字符串或 null",
            "planName must be string or null",
        ));
    }
    if obj.contains_key("extra") && !result["extra"].is_null() && !result["extra"].is_string() {
        return Err(AppError::localized(
            "usage_script.extra_type_error",
            "extra 必须是字符串或 null",
            "extra must be string or null",
        ));
    }

    Ok(())
}

/// 构建替换变量后的脚本，保持与旧版脚本的兼容性
fn build_script_with_vars(
    script_code: &str,
    api_key: &str,
    base_url: &str,
    access_token: Option<&str>,
    user_id: Option<&str>,
) -> String {
    let mut replaced = script_code
        .replace("{{apiKey}}", api_key)
        .replace("{{baseUrl}}", base_url);

    if let Some(token) = access_token {
        replaced = replaced.replace("{{accessToken}}", token);
    }
    if let Some(uid) = user_id {
        replaced = replaced.replace("{{userId}}", uid);
    }

    replaced
}

/// 验证 base_url 的基本安全性
fn validate_base_url(base_url: &str) -> Result<(), AppError> {
    if base_url.is_empty() {
        return Err(AppError::localized(
            "usage_script.base_url_empty",
            "base_url 不能为空",
            "base_url cannot be empty",
        ));
    }

    // 解析 URL
    let parsed_url = Url::parse(base_url).map_err(|e| {
        AppError::localized(
            "usage_script.base_url_invalid",
            format!("无效的 base_url: {e}"),
            format!("Invalid base_url: {e}"),
        )
    })?;

    let is_loopback = is_loopback_host(&parsed_url);

    // 必须是 HTTPS（允许 localhost 用于开发）
    if parsed_url.scheme() != "https" && !is_loopback {
        return Err(AppError::localized(
            "usage_script.base_url_https_required",
            "base_url 必须使用 HTTPS 协议（localhost 除外）",
            "base_url must use HTTPS (localhost allowed)",
        ));
    }

    // 检查主机名格式有效性
    let hostname = parsed_url.host_str().ok_or_else(|| {
        AppError::localized(
            "usage_script.base_url_hostname_missing",
            "base_url 必须包含有效的主机名",
            "base_url must include a valid hostname",
        )
    })?;

    // 基本的主机名格式检查
    if hostname.is_empty() {
        return Err(AppError::localized(
            "usage_script.base_url_hostname_empty",
            "base_url 主机名不能为空",
            "base_url hostname cannot be empty",
        ));
    }

    Ok(())
}

fn should_validate_base_url(base_url: &str, is_custom_template: bool) -> bool {
    !base_url.is_empty() && !is_custom_template
}

/// 验证请求 URL 是否安全（HTTPS 强制 + 同源检查）
fn validate_request_url(
    request_url: &str,
    base_url: &str,
    is_custom_template: bool,
) -> Result<(), AppError> {
    if request_url.trim().is_empty() {
        return Err(AppError::localized(
            "usage_script.request_url_empty",
            "脚本 request.url 是空的：请在用量查询脚本里填写完整 URL，或选择 Balance / Token Plan / GitHub Copilot 内置模板",
            "Script's request.url is empty: fill in a complete URL in the usage script, or pick a built-in template like Balance / Token Plan / GitHub Copilot",
        ));
    }
    // 解析请求 URL
    let parsed_request = Url::parse(request_url).map_err(|e| {
        AppError::localized(
            "usage_script.request_url_invalid",
            format!("无效的请求 URL: {e}"),
            format!("Invalid request URL: {e}"),
        )
    })?;

    let is_request_loopback = is_loopback_host(&parsed_request);

    // 必须使用 HTTPS（允许 localhost 用于开发）
    // 自定义模板模式下，允许用户自行决定是否使用 HTTP（用户需自行承担安全风险）
    if !is_custom_template && parsed_request.scheme() != "https" && !is_request_loopback {
        return Err(AppError::localized(
            "usage_script.request_https_required",
            "请求 URL 必须使用 HTTPS 协议（localhost 除外）",
            "Request URL must use HTTPS (localhost allowed)",
        ));
    }

    // 如果提供了 base_url（非空），则进行同源检查
    // 🔧 自定义模板模式下，用户可以自由访问任意 HTTPS 域名，跳过同源检查
    if !base_url.is_empty() && !is_custom_template {
        // 解析 base URL
        let parsed_base = Url::parse(base_url).map_err(|e| {
            AppError::localized(
                "usage_script.base_url_invalid",
                format!("无效的 base_url: {e}"),
                format!("Invalid base_url: {e}"),
            )
        })?;

        // 核心安全检查：必须与 base_url 同源（相同域名和端口）
        if parsed_request.host_str() != parsed_base.host_str() {
            return Err(AppError::localized(
                "usage_script.request_host_mismatch",
                format!(
                    "请求域名 {} 与 base_url 域名 {} 不匹配（必须是同源请求）",
                    parsed_request.host_str().unwrap_or("unknown"),
                    parsed_base.host_str().unwrap_or("unknown")
                ),
                format!(
                    "Request host {} must match base_url host {} (same-origin required)",
                    parsed_request.host_str().unwrap_or("unknown"),
                    parsed_base.host_str().unwrap_or("unknown")
                ),
            ));
        }

        // 检查端口是否匹配（考虑默认端口）
        // 使用 port_or_known_default() 会自动处理默认端口（http->80, https->443）
        match (
            parsed_request.port_or_known_default(),
            parsed_base.port_or_known_default(),
        ) {
            (Some(request_port), Some(base_port)) if request_port == base_port => {
                // 端口匹配，继续执行
            }
            (Some(request_port), Some(base_port)) => {
                return Err(AppError::localized(
                    "usage_script.request_port_mismatch",
                    format!("请求端口 {request_port} 必须与 base_url 端口 {base_port} 匹配"),
                    format!("Request port {request_port} must match base_url port {base_port}"),
                ));
            }
            _ => {
                // 理论上不会发生，因为 port_or_known_default() 应该总是返回 Some
                return Err(AppError::localized(
                    "usage_script.request_port_unknown",
                    "无法确定端口号",
                    "Unable to determine port number",
                ));
            }
        }
    }

    Ok(())
}

/// 判断 URL 是否指向本机（localhost / loopback）
fn is_loopback_host(url: &Url) -> bool {
    match url.host() {
        Some(Host::Domain(d)) => d.eq_ignore_ascii_case("localhost"),
        Some(Host::Ipv4(ip)) => ip.is_loopback(),
        Some(Host::Ipv6(ip)) => ip.is_loopback(),
        _ => false,
    }
}

/// Map the tauri-free SSRF guard outcome into a localized `AppError` so the
/// usage-script path stays tauri-free and does not depend on `web_api`.
pub(crate) fn map_outbound_guard_error(err: crate::proxy::ip_guard::OutboundUrlError) -> AppError {
    use crate::proxy::ip_guard::OutboundUrlError;
    match err {
        OutboundUrlError::InvalidUrl { raw, reason } => AppError::localized(
            "usage_script.request_url_invalid",
            format!("无效的请求 URL '{raw}': {reason}"),
            format!("Invalid request URL '{raw}': {reason}"),
        ),
        OutboundUrlError::UnsupportedScheme { scheme } => AppError::localized(
            "usage_script.request_scheme_unsupported",
            format!("不支持的请求 URL 协议 '{scheme}'：仅允许 http 与 https"),
            format!("Unsupported request URL scheme '{scheme}': only http and https are allowed"),
        ),
        OutboundUrlError::MissingHost { raw } => AppError::localized(
            "usage_script.request_url_no_host",
            format!("请求 URL '{raw}' 缺少主机名"),
            format!("Request URL '{raw}' has no host"),
        ),
        OutboundUrlError::ResolveFailed { host, reason } => AppError::localized(
            "usage_script.request_host_resolve_failed",
            format!("无法解析主机 '{host}': {reason}"),
            format!("Failed to resolve host '{host}': {reason}"),
        ),
        OutboundUrlError::BlockedAddress { host } => AppError::localized(
            "usage_script.request_internal_blocked",
            format!("拒绝访问内网地址 '{host}'"),
            format!("Refusing to reach internal address '{host}'"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_https_bypass_prevention() {
        // 非本地域名的 HTTP 应该被拒绝
        let result = validate_base_url("http://127.0.0.1.evil.com/api");
        assert!(
            result.is_err(),
            "Should reject HTTP for non-localhost domains"
        );
    }

    fn request_config(url: &str) -> RequestConfig {
        RequestConfig {
            url: url.to_string(),
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: None,
        }
    }

    #[tokio::test]
    async fn web_guard_rejects_internal_request_url_before_dial() {
        // FIX 1: custom-template scripts skip HTTPS/same-origin checks, so the
        // web-runtime guard is the only barrier. Internal/metadata/CGNAT
        // targets must be rejected before any connection is attempted.
        for url in [
            "http://169.254.169.254/latest/meta-data/",
            "http://127.0.0.1:9999/",
            "http://10.0.0.1/",
            "http://100.64.0.1/",
        ] {
            let err = send_http_request(&request_config(url), 5, true)
                .await
                .expect_err("internal target must be rejected by the web guard");
            assert!(
                err.to_string().contains("内网") || err.to_string().contains("internal"),
                "expected an internal-address rejection for {url}, got: {err}"
            );
        }
    }

    #[tokio::test]
    async fn web_guard_rejects_non_http_scheme() {
        let err = send_http_request(&request_config("file:///etc/passwd"), 5, true)
            .await
            .expect_err("non-http(s) scheme must be rejected by the web guard");
        assert!(
            err.to_string().contains("http") || err.to_string().contains("协议"),
            "expected an unsupported-scheme rejection, got: {err}"
        );
    }

    #[test]
    fn test_custom_template_allows_http_lan_request_with_different_base_url() {
        assert!(
            !should_validate_base_url("http://10.37.192.156:8090/anthropic", true),
            "Custom scripts should not validate an unused provider base_url fallback"
        );

        let result = validate_request_url(
            "http://10.37.192.156:18344/user/balance",
            "http://10.37.192.156:8090/anthropic",
            true,
        );
        assert!(
            result.is_ok(),
            "Custom usage scripts should be able to call an explicit HTTP quota endpoint"
        );
    }

    #[test]
    fn request_url_extraction_interrupts_infinite_loop() {
        let started_at = std::time::Instant::now();
        let result = try_extract_request_url("while (true) {}");

        assert!(result.is_err(), "infinite loop should be interrupted");
        assert!(
            started_at.elapsed() < JS_EXECUTION_TIMEOUT + std::time::Duration::from_secs(3),
            "interrupt should return promptly"
        );
        assert!(
            result.unwrap_err().to_string().contains("JS 脚本执行超时"),
            "error should identify the timeout"
        );
    }

    #[test]
    fn extractor_execution_interrupts_infinite_loop() {
        let (runtime, js_started_at) = create_bounded_js_runtime().expect("runtime");
        let context = Context::full(&runtime).expect("context");
        let started_at = std::time::Instant::now();

        let result: Result<(), AppError> = context.with(|ctx| {
            let extractor: Function =
                ctx.eval("(function () { while (true) {} })").map_err(|e| {
                    map_js_error_or_timeout(
                        js_started_at,
                        "测试 extractor 解析",
                        "test extractor parse",
                        "usage_script.config_parse_failed",
                        "解析配置失败",
                        "Failed to parse config",
                        e,
                    )
                })?;
            let _: rquickjs::Value = extractor.call(()).map_err(|e| {
                map_js_error_or_timeout(
                    js_started_at,
                    "执行 extractor",
                    "extractor execution",
                    "usage_script.extractor_exec_failed",
                    "执行 extractor 失败",
                    "Failed to execute extractor",
                    e,
                )
            })?;
            Ok(())
        });

        assert!(result.is_err(), "infinite extractor should be interrupted");
        assert!(
            started_at.elapsed() < JS_EXECUTION_TIMEOUT + std::time::Duration::from_secs(3),
            "interrupt should return promptly"
        );
        assert!(
            result.unwrap_err().to_string().contains("JS 脚本执行超时"),
            "error should identify the timeout"
        );
    }

    #[test]
    fn test_port_comparison() {
        // 测试端口比较逻辑是否正确处理默认端口和显式端口

        // 测试用例：(base_url, request_url, should_match)
        let test_cases = vec![
            // HTTPS默认端口测试
            (
                "https://api.example.com",
                "https://api.example.com/v1/test",
                true,
            ),
            (
                "https://api.example.com",
                "https://api.example.com:443/v1/test",
                true,
            ),
            (
                "https://api.example.com:443",
                "https://api.example.com/v1/test",
                true,
            ),
            (
                "https://api.example.com:443",
                "https://api.example.com:443/v1/test",
                true,
            ),
            // 端口不匹配测试
            (
                "https://api.example.com",
                "https://api.example.com:8443/v1/test",
                false,
            ),
            (
                "https://api.example.com:443",
                "https://api.example.com:8443/v1/test",
                false,
            ),
        ];

        for (base_url, request_url, should_match) in test_cases {
            let result = validate_request_url(request_url, base_url, false);

            if should_match {
                assert!(
                    result.is_ok(),
                    "应该匹配的URL被拒绝: base_url={}, request_url={}, error={}",
                    base_url,
                    request_url,
                    result.unwrap_err()
                );
            } else {
                assert!(
                    result.is_err(),
                    "应该不匹配的URL被允许: base_url={}, request_url={}",
                    base_url,
                    request_url
                );
            }
        }
    }
}
