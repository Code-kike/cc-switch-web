//! Usage script execution
//!
//! Handles executing and formatting usage query results.

use crate::app_config::AppType;
use crate::error::AppError;
use crate::provider::{Provider, UsageData, UsageResult, UsageScript};
use crate::proxy::providers::copilot_auth::CopilotAuthManager;
use crate::settings;
use crate::store::AppState;
use crate::usage_script;
use serde_json::Value;
use tokio::sync::RwLock;

pub(crate) const TEMPLATE_TYPE_GITHUB_COPILOT: &str = "github_copilot";
pub(crate) const TEMPLATE_TYPE_TOKEN_PLAN: &str = "token_plan";
pub(crate) const TEMPLATE_TYPE_BALANCE: &str = "balance";
pub(crate) const TEMPLATE_TYPE_OFFICIAL_SUBSCRIPTION: &str = "official_subscription";
const COPILOT_UNIT_PREMIUM: &str = "requests";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct UsageCredentials {
    api_key: String,
    base_url: String,
    access_token: Option<String>,
    user_id: Option<String>,
}

/// FIX A (round-2): web-runtime SSRF guard for native-template (`token_plan` /
/// `balance`) dials. Those arms dial the user-controlled `credentials.base_url`
/// directly through `coding_plan`/`balance` (whose service signatures stay
/// desktop-callable and unguarded), so the initial-hop guard must be applied
/// here at the arm. Only enforced in the web runtime (`enforce_outbound_guard`)
/// and only for a non-empty base_url; desktop callers pass `false` (unchanged).
/// Reuses the tauri-free `proxy::ip_guard` SSOT and the shared usage_script
/// error mapping to avoid divergence with the JS-path guard.
async fn guard_native_template_base_url(
    base_url: &str,
    enforce_outbound_guard: bool,
) -> Result<(), AppError> {
    if enforce_outbound_guard && !base_url.is_empty() {
        crate::proxy::ip_guard::guard_outbound_url(base_url)
            .await
            .map_err(usage_script::map_outbound_guard_error)?;
    }
    Ok(())
}

/// Execute usage script and format result (private helper method)
///
/// `enforce_outbound_guard` (audit FIX 1) threads the web-runtime SSRF gate
/// down to `usage_script::send_http_request`. Desktop callers pass `false`.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn execute_and_format_usage_result(
    script_code: &str,
    api_key: &str,
    base_url: &str,
    timeout: u64,
    access_token: Option<&str>,
    user_id: Option<&str>,
    template_type: Option<&str>,
    enforce_outbound_guard: bool,
) -> Result<UsageResult, AppError> {
    match usage_script::execute_usage_script(
        script_code,
        api_key,
        base_url,
        timeout,
        access_token,
        user_id,
        template_type,
        enforce_outbound_guard,
    )
    .await
    {
        Ok(data) => {
            let usage_list: Vec<UsageData> = if data.is_array() {
                serde_json::from_value(data).map_err(|e| {
                    AppError::localized(
                        "usage_script.data_format_error",
                        format!("数据格式错误: {e}"),
                        format!("Data format error: {e}"),
                    )
                })?
            } else {
                let single: UsageData = serde_json::from_value(data).map_err(|e| {
                    AppError::localized(
                        "usage_script.data_format_error",
                        format!("数据格式错误: {e}"),
                        format!("Data format error: {e}"),
                    )
                })?;
                vec![single]
            };

            Ok(UsageResult {
                success: true,
                data: Some(usage_list),
                error: None,
            })
        }
        Err(err) => {
            // 瞬时传输失败（send 失败/超时、读体中断）以 Err 传播，让前端 invoke
            // reject → react-query retry 并保留上次成功值；按错误 key 判定而非
            // 文案匹配。其余脚本/配置/HTTP 业务错误折叠成 success:false 展示文案。
            if let AppError::Localized { key, .. } = &err {
                if matches!(
                    *key,
                    "usage_script.request_failed" | "usage_script.read_response_failed"
                ) {
                    return Err(err);
                }
            }

            let lang = settings::get_settings()
                .language
                .unwrap_or_else(|| "zh".to_string());

            let msg = match err {
                AppError::Localized { zh, en, .. } => {
                    if lang == "en" {
                        en
                    } else {
                        zh
                    }
                }
                other => other.to_string(),
            };

            Ok(UsageResult {
                success: false,
                data: None,
                error: Some(msg),
            })
        }
    }
}

fn non_empty_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

fn non_empty_opt_string(value: Option<&String>) -> Option<String> {
    value
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

fn normalize_base_url(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let normalized = trimmed.trim_end_matches('/').to_string();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn non_empty_base_url(value: Option<&Value>) -> Option<String> {
    non_empty_string(value).and_then(|s| normalize_base_url(&s))
}

fn non_empty_opt_base_url(value: Option<&String>) -> Option<String> {
    non_empty_opt_string(value).and_then(|s| normalize_base_url(&s))
}

fn setting_string(settings: &Value, path: &[&str]) -> Option<String> {
    let mut current = settings;
    for segment in path {
        current = current.get(*segment)?;
    }
    non_empty_string(Some(current))
}

fn setting_base_url(settings: &Value, path: &[&str]) -> Option<String> {
    let mut current = settings;
    for segment in path {
        current = current.get(*segment)?;
    }
    non_empty_base_url(Some(current))
}

fn extract_provider_usage_credentials(provider: &Provider, app_type: &AppType) -> UsageCredentials {
    let settings = &provider.settings_config;

    let (api_key, base_url) = match app_type {
        AppType::Claude => (
            setting_string(settings, &["env", "ANTHROPIC_AUTH_TOKEN"])
                .or_else(|| setting_string(settings, &["env", "ANTHROPIC_API_KEY"]))
                .or_else(|| setting_string(settings, &["env", "OPENROUTER_API_KEY"]))
                .or_else(|| setting_string(settings, &["env", "GOOGLE_API_KEY"]))
                .or_else(|| setting_string(settings, &["env", "OPENAI_API_KEY"]))
                .or_else(|| setting_string(settings, &["apiKey"]))
                .or_else(|| setting_string(settings, &["api_key"])),
            setting_base_url(settings, &["env", "ANTHROPIC_BASE_URL"])
                .or_else(|| setting_base_url(settings, &["base_url"]))
                .or_else(|| setting_base_url(settings, &["baseURL"]))
                .or_else(|| setting_base_url(settings, &["apiEndpoint"])),
        ),
        AppType::Codex => (
            setting_string(settings, &["env", "OPENAI_API_KEY"])
                .or_else(|| setting_string(settings, &["auth", "OPENAI_API_KEY"]))
                // Config-only installs keep the key in config.toml's
                // experimental_bearer_token; mirror the frontend fallback
                // (getProviderCredentials: auth.OPENAI_API_KEY else bearer token)
                // so the saved-card refresh path resolves the same key as "Test".
                .or_else(|| {
                    settings
                        .get("config")
                        .and_then(|v| v.as_str())
                        .and_then(crate::codex_config::extract_codex_experimental_bearer_token)
                })
                .or_else(|| setting_string(settings, &["apiKey"]))
                .or_else(|| setting_string(settings, &["api_key"]))
                .or_else(|| setting_string(settings, &["config", "apiKey"]))
                .or_else(|| setting_string(settings, &["config", "api_key"])),
            setting_base_url(settings, &["base_url"])
                .or_else(|| setting_base_url(settings, &["baseURL"]))
                .or_else(|| setting_base_url(settings, &["config", "base_url"]))
                .or_else(|| {
                    settings
                        .get("config")
                        .and_then(|v| v.as_str())
                        .and_then(extract_codex_base_url_from_toml)
                }),
        ),
        AppType::Gemini => (
            setting_string(settings, &["env", "GEMINI_API_KEY"])
                .or_else(|| setting_string(settings, &["env", "GOOGLE_API_KEY"]))
                .or_else(|| setting_string(settings, &["apiKey"]))
                .or_else(|| setting_string(settings, &["api_key"])),
            setting_base_url(settings, &["env", "GOOGLE_GEMINI_BASE_URL"])
                .or_else(|| setting_base_url(settings, &["env", "GEMINI_BASE_URL"]))
                .or_else(|| setting_base_url(settings, &["base_url"]))
                .or_else(|| setting_base_url(settings, &["baseURL"])),
        ),
        AppType::OpenCode => (
            setting_string(settings, &["options", "apiKey"]),
            setting_base_url(settings, &["options", "baseURL"]),
        ),
        AppType::OpenClaw => (
            setting_string(settings, &["apiKey"]),
            setting_base_url(settings, &["baseUrl"]),
        ),
        AppType::Hermes => {
            // Hermes provider entries under `custom_providers:` may carry their
            // own `api_key`/`base_url`, but the common case is shared
            // credentials living on the live YAML's top-level `model:` section.
            // Without that fallback, every "shared-creds" Hermes provider would
            // surface "无效的请求 URL: relative URL without a base" — the JS
            // path receives an empty base_url because it is never set on the
            // provider record.
            let provider_api_key = setting_string(settings, &["api_key"]);
            let provider_base_url = setting_base_url(settings, &["base_url"]);

            let mut api_key = provider_api_key;
            let mut base_url = provider_base_url;

            if api_key.is_none() || base_url.is_none() {
                if let Ok(Some(model)) = crate::hermes_config::get_model_config() {
                    // Only borrow shared credentials when this provider is the
                    // active one — otherwise we'd serve another provider's key.
                    let is_active_provider =
                        model.provider.as_deref() == Some(provider.id.as_str());
                    if is_active_provider {
                        if api_key.is_none() {
                            api_key = model
                                .extra
                                .get("api_key")
                                .and_then(|v| v.as_str())
                                .map(str::trim)
                                .filter(|s| !s.is_empty())
                                .map(ToString::to_string);
                        }
                        if base_url.is_none() {
                            base_url = model.base_url.as_deref().and_then(normalize_base_url);
                        }
                    }
                }
            }

            (api_key, base_url)
        }
    };

    UsageCredentials {
        api_key: api_key.unwrap_or_default(),
        base_url: base_url.unwrap_or_default(),
        access_token: None,
        user_id: None,
    }
}

fn resolve_usage_credentials(
    provider: &Provider,
    app_type: &AppType,
    usage_script: &UsageScript,
) -> UsageCredentials {
    let provider_credentials = extract_provider_usage_credentials(provider, app_type);

    UsageCredentials {
        api_key: non_empty_opt_string(usage_script.api_key.as_ref())
            .unwrap_or(provider_credentials.api_key),
        base_url: non_empty_opt_base_url(usage_script.base_url.as_ref())
            .unwrap_or(provider_credentials.base_url),
        access_token: non_empty_opt_string(usage_script.access_token.as_ref()),
        user_id: non_empty_opt_string(usage_script.user_id.as_ref()),
    }
}

/// Resolve credentials for the Token Plan (coding plan) path.
///
/// ZenMux quota queries hit a user-supplied full quota URL, so the manually
/// configured script credentials take precedence as a pair; mixing a script
/// base URL with a provider API key (or vice versa) is avoided, mirroring
/// upstream `resolve_coding_plan_credentials` semantics. Other coding-plan
/// providers keep the fork's standard per-field script-over-provider
/// resolution.
fn resolve_coding_plan_credentials(
    provider: &Provider,
    app_type: &AppType,
    usage_script: &UsageScript,
) -> UsageCredentials {
    let is_zenmux = usage_script
        .coding_plan_provider
        .as_deref()
        .map(|p| p.eq_ignore_ascii_case("zenmux"))
        .unwrap_or(false);

    if !is_zenmux {
        return resolve_usage_credentials(provider, app_type, usage_script);
    }

    let script_base_url = non_empty_opt_base_url(usage_script.base_url.as_ref());
    let script_api_key = non_empty_opt_string(usage_script.api_key.as_ref());

    if let (Some(base_url), Some(api_key)) = (script_base_url.as_ref(), script_api_key.as_ref()) {
        return UsageCredentials {
            api_key: api_key.clone(),
            base_url: base_url.clone(),
            access_token: None,
            user_id: None,
        };
    }

    let native = extract_provider_usage_credentials(provider, app_type);
    if !native.api_key.is_empty() && !native.base_url.is_empty() {
        native
    } else {
        UsageCredentials {
            api_key: script_api_key.unwrap_or_default(),
            base_url: script_base_url.unwrap_or_default(),
            access_token: None,
            user_id: None,
        }
    }
}

/// Explicit routing hints a coding-plan query needs beyond `(base_url, api_key)`.
///
/// Zhipu's team plan shares `open.bigmodel.cn` with the personal plan, so
/// `detect_provider` cannot tell them apart — the saved script's explicit
/// `codingPlanProvider` plus the organization/project IDs are what route it.
#[derive(Default)]
struct CodingPlanRouting {
    provider: Option<String>,
    team_organization_id: Option<String>,
    team_project_id: Option<String>,
}

fn coding_plan_routing(usage_script: &UsageScript) -> CodingPlanRouting {
    CodingPlanRouting {
        provider: non_empty_opt_string(usage_script.coding_plan_provider.as_ref()),
        team_organization_id: non_empty_opt_string(usage_script.team_organization_id.as_ref()),
        team_project_id: non_empty_opt_string(usage_script.team_project_id.as_ref()),
    }
}

/// Flatten a coding-plan `SubscriptionQuota` into the shared `UsageResult`
/// shape used by both the saved-query and test-query paths.
///
/// ZenMux tiers carry USD quota info, which is encoded as a JSON `extra`
/// payload (`resetsAt`/`usedValueUsd`/`maxValueUsd`/`planLabel`) for the
/// frontend's rich display; other providers keep the plain resets_at string.
fn coding_plan_quota_to_usage_result(
    quota: &crate::services::subscription::SubscriptionQuota,
) -> UsageResult {
    if !quota.success {
        return UsageResult {
            success: false,
            data: None,
            error: quota.error.clone(),
        };
    }

    // ZenMux 的 tier 携带 USD 额度信息，需要编码为 JSON extra
    let has_usd = quota
        .tiers
        .first()
        .map(|t| t.used_value_usd.is_some())
        .unwrap_or(false);
    let plan_label = quota
        .credential_message
        .as_deref()
        .and_then(|msg| msg.split(' ').next())
        .map(|tier| format!("ZenMux·{}", tier.to_uppercase()));
    let mut first_tier = true;

    let data: Vec<UsageData> = quota
        .tiers
        .iter()
        .map(|tier| {
            let total = 100.0;
            let used = tier.utilization;
            let remaining = total - used;
            let extra = if has_usd {
                let mut extra_json = serde_json::json!({
                    "resetsAt": tier.resets_at,
                });
                if let Some(v) = tier.used_value_usd {
                    extra_json["usedValueUsd"] = serde_json::json!(v);
                }
                if let Some(v) = tier.max_value_usd {
                    extra_json["maxValueUsd"] = serde_json::json!(v);
                }
                if first_tier {
                    if let Some(ref label) = plan_label {
                        extra_json["planLabel"] = serde_json::json!(label);
                    }
                    first_tier = false;
                }
                Some(extra_json.to_string())
            } else {
                tier.resets_at.clone()
            };
            UsageData {
                plan_name: Some(tier.name.clone()),
                remaining: Some(remaining),
                total: Some(total),
                used: Some(used),
                unit: Some("%".to_string()),
                is_valid: Some(true),
                invalid_message: None,
                extra,
            }
        })
        .collect();

    UsageResult {
        success: true,
        data: if data.is_empty() { None } else { Some(data) },
        error: None,
    }
}

fn extract_codex_base_url_from_toml(config_toml: &str) -> Option<String> {
    let parsed = toml::from_str::<toml::Value>(config_toml).ok()?;

    if let Some(provider_name) = parsed
        .get("model_provider")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if let Some(base_url) = parsed
            .get("model_providers")
            .and_then(|v| v.as_table())
            .and_then(|providers| providers.get(provider_name))
            .and_then(|provider| provider.get("base_url"))
            .and_then(|v| v.as_str())
            .and_then(normalize_base_url)
        {
            return Some(base_url);
        }
    }

    if let Some(base_url) = parsed
        .get("base_url")
        .and_then(|v| v.as_str())
        .and_then(normalize_base_url)
    {
        return Some(base_url);
    }

    let provider_urls: Vec<String> = parsed
        .get("model_providers")
        .and_then(|v| v.as_table())
        .map(|providers| {
            providers
                .values()
                .filter_map(|provider| {
                    provider
                        .get("base_url")
                        .and_then(|v| v.as_str())
                        .and_then(normalize_base_url)
                })
                .collect()
        })
        .unwrap_or_default();

    if provider_urls.len() == 1 {
        provider_urls.into_iter().next()
    } else {
        None
    }
}

async fn query_copilot_usage(
    copilot_auth: Option<&RwLock<CopilotAuthManager>>,
    copilot_account_id: Option<&str>,
) -> Result<UsageResult, AppError> {
    let auth_manager = copilot_auth.ok_or_else(|| {
        AppError::Message("GitHub Copilot auth manager is unavailable".to_string())
    })?;
    let auth_manager = auth_manager.read().await;
    let usage = match copilot_account_id {
        Some(account_id) => auth_manager
            .fetch_usage_for_account(account_id)
            .await
            .map_err(|e| AppError::Message(format!("Failed to fetch Copilot usage: {e}")))?,
        None => auth_manager
            .fetch_usage()
            .await
            .map_err(|e| AppError::Message(format!("Failed to fetch Copilot usage: {e}")))?,
    };
    let premium = &usage.quota_snapshots.premium_interactions;
    let used = premium.entitlement - premium.remaining;

    Ok(UsageResult {
        success: true,
        data: Some(vec![UsageData {
            plan_name: Some(usage.copilot_plan),
            remaining: Some(premium.remaining as f64),
            total: Some(premium.entitlement as f64),
            used: Some(used as f64),
            unit: Some(COPILOT_UNIT_PREMIUM.to_string()),
            is_valid: Some(true),
            invalid_message: None,
            extra: Some(format!("Reset: {}", usage.quota_reset_date)),
        }]),
        error: None,
    })
}

/// Query provider usage (using saved script configuration)
pub async fn query_usage(
    state: &AppState,
    app_type: AppType,
    provider_id: &str,
    enforce_outbound_guard: bool,
) -> Result<UsageResult, AppError> {
    let (script_code, timeout, api_key, base_url, access_token, user_id, template_type) = {
        let providers = state.db.get_all_providers(app_type.as_str())?;
        let provider = providers.get(provider_id).ok_or_else(|| {
            AppError::localized(
                "provider.not_found",
                format!("供应商不存在: {provider_id}"),
                format!("Provider not found: {provider_id}"),
            )
        })?;

        let usage_script = provider
            .meta
            .as_ref()
            .and_then(|m| m.usage_script.as_ref())
            .ok_or_else(|| {
                AppError::localized(
                    "provider.usage.script.missing",
                    "未配置用量查询脚本",
                    "Usage script is not configured",
                )
            })?;
        if !usage_script.enabled {
            return Err(AppError::localized(
                "provider.usage.disabled",
                "用量查询未启用",
                "Usage query is disabled",
            ));
        }

        let credentials = resolve_usage_credentials(provider, &app_type, usage_script);

        (
            usage_script.code.clone(),
            usage_script.timeout.unwrap_or(10),
            credentials.api_key,
            credentials.base_url,
            credentials.access_token,
            credentials.user_id,
            usage_script.template_type.clone(),
        )
    };

    execute_and_format_usage_result(
        &script_code,
        &api_key,
        &base_url,
        timeout,
        access_token.as_deref(),
        user_id.as_deref(),
        template_type.as_deref(),
        enforce_outbound_guard,
    )
    .await
}

/// Query provider usage with built-in template dispatch.
///
/// This is the shared entry point for desktop Tauri commands and Web API
/// handlers. It keeps the saved-script fallback unchanged while making native
/// templates (Copilot, Token Plan, Balance) available in both runtimes.
pub async fn query_usage_with_templates(
    state: &AppState,
    app_type: AppType,
    provider_id: &str,
    copilot_auth: Option<&RwLock<CopilotAuthManager>>,
    enforce_outbound_guard: bool,
) -> Result<UsageResult, AppError> {
    let (template_type, credentials, coding_plan_routing, copilot_account_id) = {
        let providers = state.db.get_all_providers(app_type.as_str())?;
        let provider = providers.get(provider_id).ok_or_else(|| {
            AppError::localized(
                "provider.not_found",
                format!("供应商不存在: {provider_id}"),
                format!("Provider not found: {provider_id}"),
            )
        })?;

        let usage_script = provider
            .meta
            .as_ref()
            .and_then(|m| m.usage_script.as_ref())
            .ok_or_else(|| {
                AppError::localized(
                    "provider.usage.script.missing",
                    "未配置用量查询脚本",
                    "Usage script is not configured",
                )
            })?;
        if !usage_script.enabled {
            return Err(AppError::localized(
                "provider.usage.disabled",
                "用量查询未启用",
                "Usage query is disabled",
            ));
        }

        let template_type = usage_script.template_type.clone().unwrap_or_default();
        // ZenMux Token Plan 的脚本凭证按"成对优先"解析，与上游语义一致
        let credentials = if template_type == TEMPLATE_TYPE_TOKEN_PLAN {
            resolve_coding_plan_credentials(provider, &app_type, usage_script)
        } else {
            resolve_usage_credentials(provider, &app_type, usage_script)
        };

        (
            template_type,
            credentials,
            coding_plan_routing(usage_script),
            provider
                .meta
                .as_ref()
                .and_then(|m| m.managed_account_id_for(TEMPLATE_TYPE_GITHUB_COPILOT)),
        )
    };

    match template_type.as_str() {
        TEMPLATE_TYPE_GITHUB_COPILOT => {
            query_copilot_usage(copilot_auth, copilot_account_id.as_deref()).await
        }
        TEMPLATE_TYPE_TOKEN_PLAN => {
            guard_native_template_base_url(&credentials.base_url, enforce_outbound_guard).await?;
            let quota = crate::services::coding_plan::get_coding_plan_quota(
                &credentials.base_url,
                &credentials.api_key,
                coding_plan_routing.provider.as_deref(),
                coding_plan_routing.team_organization_id.as_deref(),
                coding_plan_routing.team_project_id.as_deref(),
            )
            .await
            .map_err(|e| AppError::Message(format!("Failed to query coding plan: {e}")))?;

            Ok(coding_plan_quota_to_usage_result(&quota))
        }
        TEMPLATE_TYPE_BALANCE => {
            guard_native_template_base_url(&credentials.base_url, enforce_outbound_guard).await?;
            crate::services::balance::get_balance(&credentials.base_url, &credentials.api_key)
                .await
                .map_err(|e| AppError::Message(format!("Failed to query balance: {e}")))
        }
        // ── 官方订阅额度查询路径 ──
        // enabled 已在上方统一校验（禁用脚本直接返回 usage disabled），
        // 与上游 query_provider_usage_inner 的深度防护等效。
        TEMPLATE_TYPE_OFFICIAL_SUBSCRIPTION => {
            let quota = crate::services::subscription::get_subscription_quota(app_type.as_str())
                .await
                .map_err(|e| {
                    AppError::Message(format!("Failed to query subscription quota: {e}"))
                })?;

            if !quota.success {
                return Ok(UsageResult {
                    success: false,
                    data: None,
                    error: quota.error.or(quota.credential_message),
                });
            }

            let data: Vec<UsageData> = quota
                .tiers
                .iter()
                .map(|tier| UsageData {
                    plan_name: Some(tier.name.clone()),
                    remaining: Some(100.0 - tier.utilization),
                    total: Some(100.0),
                    used: Some(tier.utilization),
                    unit: Some("%".to_string()),
                    is_valid: Some(true),
                    invalid_message: None,
                    extra: tier.resets_at.clone(),
                })
                .collect();

            Ok(UsageResult {
                success: true,
                data: if data.is_empty() { None } else { Some(data) },
                error: None,
            })
        }
        _ => query_usage(state, app_type, provider_id, enforce_outbound_guard).await,
    }
}

/// Test usage script (using temporary script content, not saved)
#[allow(clippy::too_many_arguments)]
pub async fn test_usage_script(
    state: &AppState,
    app_type: AppType,
    provider_id: &str,
    script_code: &str,
    timeout: u64,
    api_key: Option<&str>,
    base_url: Option<&str>,
    access_token: Option<&str>,
    user_id: Option<&str>,
    template_type: Option<&str>,
    copilot_auth: Option<&RwLock<CopilotAuthManager>>,
    enforce_outbound_guard: bool,
) -> Result<UsageResult, AppError> {
    if matches!(template_type, Some(TEMPLATE_TYPE_GITHUB_COPILOT)) {
        let providers = state.db.get_all_providers(app_type.as_str())?;
        let provider = providers.get(provider_id).ok_or_else(|| {
            AppError::localized(
                "provider.not_found",
                format!("供应商不存在: {provider_id}"),
                format!("Provider not found: {provider_id}"),
            )
        })?;
        let copilot_account_id = provider
            .meta
            .as_ref()
            .and_then(|m| m.managed_account_id_for(TEMPLATE_TYPE_GITHUB_COPILOT));

        return query_copilot_usage(copilot_auth, copilot_account_id.as_deref()).await;
    }

    if matches!(
        template_type,
        Some(TEMPLATE_TYPE_TOKEN_PLAN | TEMPLATE_TYPE_BALANCE)
    ) {
        let providers = state.db.get_all_providers(app_type.as_str())?;
        let provider = providers.get(provider_id).ok_or_else(|| {
            AppError::localized(
                "provider.not_found",
                format!("供应商不存在: {provider_id}"),
                format!("Provider not found: {provider_id}"),
            )
        })?;
        let mut test_script = provider
            .meta
            .as_ref()
            .and_then(|meta| meta.usage_script.as_ref())
            .cloned()
            .unwrap_or_else(|| UsageScript {
                enabled: true,
                language: "javascript".to_string(),
                code: String::new(),
                timeout: Some(timeout),
                api_key: None,
                base_url: None,
                access_token: None,
                user_id: None,
                template_type: template_type.map(str::to_string),
                auto_query_interval: None,
                coding_plan_provider: None,
                team_organization_id: None,
                team_project_id: None,
            });
        test_script.enabled = true;
        test_script.timeout = Some(timeout);
        test_script.template_type = template_type.map(str::to_string);
        if let Some(value) = api_key {
            test_script.api_key = Some(value.to_string());
        }
        if let Some(value) = base_url {
            test_script.base_url = Some(value.to_string());
        }
        if let Some(value) = access_token {
            test_script.access_token = Some(value.to_string());
        }
        if let Some(value) = user_id {
            test_script.user_id = Some(value.to_string());
        }

        // ZenMux Token Plan 的脚本凭证按"成对优先"解析，与上游语义一致
        let credentials = if matches!(template_type, Some(TEMPLATE_TYPE_TOKEN_PLAN)) {
            resolve_coding_plan_credentials(provider, &app_type, &test_script)
        } else {
            resolve_usage_credentials(provider, &app_type, &test_script)
        };
        return match template_type {
            Some(TEMPLATE_TYPE_TOKEN_PLAN) => {
                let routing = coding_plan_routing(&test_script);
                guard_native_template_base_url(&credentials.base_url, enforce_outbound_guard)
                    .await?;
                let quota = crate::services::coding_plan::get_coding_plan_quota(
                    &credentials.base_url,
                    &credentials.api_key,
                    routing.provider.as_deref(),
                    routing.team_organization_id.as_deref(),
                    routing.team_project_id.as_deref(),
                )
                .await
                .map_err(|e| AppError::Message(format!("Failed to query coding plan: {e}")))?;

                Ok(coding_plan_quota_to_usage_result(&quota))
            }
            Some(TEMPLATE_TYPE_BALANCE) => {
                guard_native_template_base_url(&credentials.base_url, enforce_outbound_guard)
                    .await?;
                crate::services::balance::get_balance(&credentials.base_url, &credentials.api_key)
                    .await
                    .map_err(|e| AppError::Message(format!("Failed to query balance: {e}")))
            }
            _ => unreachable!("template_type was checked above"),
        };
    }

    // Use provided credential parameters directly for testing
    execute_and_format_usage_result(
        script_code,
        api_key.unwrap_or(""),
        base_url.unwrap_or(""),
        timeout,
        access_token,
        user_id,
        template_type,
        enforce_outbound_guard,
    )
    .await
}

/// True when the template type routes through the JS-script execution path.
/// Built-in templates (`balance` / `token_plan` / `github_copilot` /
/// `official_subscription`) ignore the JS body, so save-time URL validation
/// does not apply to them.
fn template_uses_js_path(template_type: Option<&str>) -> bool {
    !matches!(
        template_type,
        Some(TEMPLATE_TYPE_GITHUB_COPILOT)
            | Some(TEMPLATE_TYPE_TOKEN_PLAN)
            | Some(TEMPLATE_TYPE_BALANCE)
            | Some(TEMPLATE_TYPE_OFFICIAL_SUBSCRIPTION)
    )
}

/// Validate UsageScript configuration (boundary checks)
pub(crate) fn validate_usage_script(script: &UsageScript) -> Result<(), AppError> {
    // Validate auto query interval (0-1440 minutes, max 24 hours)
    if let Some(interval) = script.auto_query_interval {
        if interval > 1440 {
            return Err(AppError::localized(
                "usage_script.interval_too_large",
                format!("自动查询间隔不能超过 1440 分钟（24小时），当前值: {interval}"),
                format!(
                    "Auto query interval cannot exceed 1440 minutes (24 hours), current: {interval}"
                ),
            ));
        }
    }

    // Reject obviously broken JS scripts (`request.url: ""`) at save time so
    // the failure surfaces in the editor instead of every refresh on the
    // provider card. Built-in templates skip this — their JS body is unused.
    // Disabled scripts can keep stale placeholders without bothering us.
    if script.enabled && template_uses_js_path(script.template_type.as_deref()) {
        match crate::usage_script::try_extract_request_url(&script.code) {
            Ok(Some(url)) if url.trim().is_empty() => {
                return Err(AppError::localized(
                    "usage_script.request_url_empty_at_save",
                    "脚本里 request.url 是空的：请在用量查询脚本里填写完整 URL，或选择 Balance / Token Plan / GitHub Copilot 内置模板",
                    "Script's request.url is empty: fill in a complete URL in the usage script, or pick a built-in template like Balance / Token Plan / GitHub Copilot",
                ));
            }
            // Missing `request.url` is treated as user-intentional (e.g. a
            // partially-edited draft) — execute-time validation handles it.
            // Eval errors only get logged: blocking saves on a JS parser
            // glitch would lock users out of fixing their own scripts.
            Ok(_) => {}
            Err(err) => {
                log::debug!("Skipping usage-script request.url save-time validation: {err}");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        coding_plan_quota_to_usage_result, guard_native_template_base_url,
        resolve_coding_plan_credentials, resolve_usage_credentials, validate_usage_script,
        TEMPLATE_TYPE_BALANCE, TEMPLATE_TYPE_OFFICIAL_SUBSCRIPTION,
    };
    use crate::app_config::AppType;
    use crate::provider::{Provider, ProviderMeta, UsageScript};
    use serde_json::{json, Value};

    fn provider_with_config(settings_config: Value) -> Provider {
        Provider::with_id(
            "provider-id".to_string(),
            "Provider".to_string(),
            settings_config,
            None,
        )
    }

    fn usage_script() -> UsageScript {
        UsageScript {
            enabled: true,
            language: "javascript".to_string(),
            code: "return { remaining: 1, unit: 'USD' };".to_string(),
            timeout: None,
            api_key: None,
            base_url: None,
            access_token: None,
            user_id: None,
            template_type: None,
            auto_query_interval: None,
            coding_plan_provider: None,
            team_organization_id: None,
            team_project_id: None,
        }
    }

    #[test]
    fn zenmux_coding_plan_uses_script_credentials_first() {
        let provider = provider_with_config(json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://provider.zenmux.example/v1",
                "ANTHROPIC_AUTH_TOKEN": "sk-provider"
            }
        }));
        let mut script = usage_script();
        script.template_type = Some("token_plan".to_string());
        script.coding_plan_provider = Some("zenmux".to_string());
        script.api_key = Some("sk-script".to_string());
        script.base_url = Some("https://script.zenmux.example/api/usage/".to_string());

        let credentials = resolve_coding_plan_credentials(&provider, &AppType::Claude, &script);

        assert_eq!(
            credentials.base_url,
            "https://script.zenmux.example/api/usage"
        );
        assert_eq!(credentials.api_key, "sk-script");
    }

    #[test]
    fn zenmux_coding_plan_falls_back_to_provider_credentials() {
        let provider = provider_with_config(json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://provider.zenmux.example/v1",
                "ANTHROPIC_AUTH_TOKEN": "sk-provider"
            }
        }));
        let mut script = usage_script();
        script.template_type = Some("token_plan".to_string());
        script.coding_plan_provider = Some("zenmux".to_string());
        script.base_url = Some("https://script.zenmux.example".to_string());

        let credentials = resolve_coding_plan_credentials(&provider, &AppType::Claude, &script);

        assert_eq!(credentials.base_url, "https://provider.zenmux.example/v1");
        assert_eq!(credentials.api_key, "sk-provider");
    }

    #[test]
    fn non_zenmux_coding_plan_keeps_per_field_script_overrides() {
        let provider = provider_with_config(json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://api.minimaxi.com/v1",
                "ANTHROPIC_AUTH_TOKEN": "sk-provider"
            }
        }));
        let mut script = usage_script();
        script.template_type = Some("token_plan".to_string());
        script.coding_plan_provider = Some("minimax".to_string());
        script.api_key = Some("sk-script".to_string());

        let credentials = resolve_coding_plan_credentials(&provider, &AppType::Claude, &script);

        assert_eq!(credentials.base_url, "https://api.minimaxi.com/v1");
        assert_eq!(credentials.api_key, "sk-script");
    }

    #[test]
    fn zenmux_quota_encodes_usd_extra_with_plan_label_on_first_tier() {
        use crate::services::subscription::{CredentialStatus, QuotaTier, SubscriptionQuota};

        let quota = SubscriptionQuota {
            tool: "coding_plan".to_string(),
            credential_status: CredentialStatus::Valid,
            credential_message: Some("pro (active)".to_string()),
            success: true,
            tiers: vec![
                QuotaTier {
                    name: "five_hour".to_string(),
                    utilization: 25.0,
                    resets_at: Some("2026-06-11T00:00:00Z".to_string()),
                    used_value_usd: Some(1.25),
                    max_value_usd: Some(5.0),
                },
                QuotaTier {
                    name: "weekly_limit".to_string(),
                    utilization: 10.0,
                    resets_at: None,
                    used_value_usd: Some(3.0),
                    max_value_usd: Some(30.0),
                },
            ],
            extra_usage: None,
            error: None,
            queried_at: Some(0),
        };

        let result = coding_plan_quota_to_usage_result(&quota);
        assert!(result.success);
        let data = result.data.expect("tiers should map to usage data");
        assert_eq!(data.len(), 2);

        let first: serde_json::Value =
            serde_json::from_str(data[0].extra.as_deref().expect("first tier JSON extra"))
                .expect("first tier extra parses as JSON");
        assert_eq!(first["usedValueUsd"], json!(1.25));
        assert_eq!(first["maxValueUsd"], json!(5.0));
        assert_eq!(first["planLabel"], json!("ZenMux·PRO"));
        assert_eq!(first["resetsAt"], json!("2026-06-11T00:00:00Z"));

        let second: serde_json::Value =
            serde_json::from_str(data[1].extra.as_deref().expect("second tier JSON extra"))
                .expect("second tier extra parses as JSON");
        assert_eq!(second["planLabel"], json!(null));
        assert_eq!(second["usedValueUsd"], json!(3.0));
    }

    #[test]
    fn non_usd_quota_keeps_plain_resets_at_extra() {
        use crate::services::subscription::{CredentialStatus, QuotaTier, SubscriptionQuota};

        let quota = SubscriptionQuota {
            tool: "coding_plan".to_string(),
            credential_status: CredentialStatus::Valid,
            credential_message: None,
            success: true,
            tiers: vec![QuotaTier {
                name: "five_hour".to_string(),
                utilization: 40.0,
                resets_at: Some("2026-06-11T00:00:00Z".to_string()),
                used_value_usd: None,
                max_value_usd: None,
            }],
            extra_usage: None,
            error: None,
            queried_at: Some(0),
        };

        let result = coding_plan_quota_to_usage_result(&quota);
        let data = result.data.expect("tier should map to usage data");
        assert_eq!(data[0].extra.as_deref(), Some("2026-06-11T00:00:00Z"));
    }

    #[test]
    fn usage_script_credentials_override_provider_config() {
        let provider = provider_with_config(json!({
            "env": {
                "ANTHROPIC_AUTH_TOKEN": "provider-key",
                "ANTHROPIC_BASE_URL": "https://provider.example/v1/"
            }
        }));
        let mut script = usage_script();
        script.api_key = Some(" script-key ".to_string());
        script.base_url = Some(" https://script.example/v1/ ".to_string());
        script.access_token = Some(" access-token ".to_string());
        script.user_id = Some(" user-1 ".to_string());

        let credentials = resolve_usage_credentials(&provider, &AppType::Claude, &script);

        assert_eq!(credentials.api_key, "script-key");
        assert_eq!(credentials.base_url, "https://script.example/v1");
        assert_eq!(credentials.access_token.as_deref(), Some("access-token"));
        assert_eq!(credentials.user_id.as_deref(), Some("user-1"));
    }

    #[test]
    fn extracts_claude_credentials_from_env() {
        let provider = provider_with_config(json!({
            "env": {
                "ANTHROPIC_AUTH_TOKEN": "claude-key",
                "ANTHROPIC_BASE_URL": "https://claude.example/"
            }
        }));

        let credentials = resolve_usage_credentials(&provider, &AppType::Claude, &usage_script());

        assert_eq!(credentials.api_key, "claude-key");
        assert_eq!(credentials.base_url, "https://claude.example");
    }

    #[test]
    fn extracts_codex_credentials_from_auth_and_active_model_provider_toml() {
        let provider = provider_with_config(json!({
            "auth": {
                "OPENAI_API_KEY": "codex-key"
            },
            "config": r#"
model_provider = "azure"

[model_providers.other]
base_url = "https://wrong.example/v1"

[model_providers.azure]
name = "Azure"
base_url = "https://azure.example/openai/"
"#
        }));

        let credentials = resolve_usage_credentials(&provider, &AppType::Codex, &usage_script());

        assert_eq!(credentials.api_key, "codex-key");
        assert_eq!(credentials.base_url, "https://azure.example/openai");
    }

    #[test]
    fn claude_key_fallback_skips_empty_primary_fields() {
        // Presets seed ANTHROPIC_AUTH_TOKEN / ANTHROPIC_API_KEY as
        // present-but-empty placeholders; the fallback chain must skip empty
        // values (matching the frontend `a || b` semantics), not just absent
        // keys, so the key stored in OPENROUTER_API_KEY is still found.
        let provider = provider_with_config(json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://openrouter.ai/api/v1/",
                "ANTHROPIC_AUTH_TOKEN": "",
                "ANTHROPIC_API_KEY": "",
                "OPENROUTER_API_KEY": "sk-or",
            }
        }));

        let credentials = resolve_usage_credentials(&provider, &AppType::Claude, &usage_script());

        assert_eq!(credentials.api_key, "sk-or");
        assert_eq!(credentials.base_url, "https://openrouter.ai/api/v1");
    }

    #[test]
    fn claude_key_falls_back_to_google_api_key() {
        let provider = provider_with_config(json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://gateway.example/v1",
                "ANTHROPIC_AUTH_TOKEN": "",
                "GOOGLE_API_KEY": "g-real",
            }
        }));

        let credentials = resolve_usage_credentials(&provider, &AppType::Claude, &usage_script());

        assert_eq!(credentials.api_key, "g-real");
    }

    #[test]
    fn gemini_key_skips_empty_primary_and_uses_google_fallback() {
        let provider = provider_with_config(json!({
            "env": {
                "GOOGLE_GEMINI_BASE_URL": "https://generativelanguage.googleapis.com",
                "GEMINI_API_KEY": "",
                "GOOGLE_API_KEY": "g-legacy",
            }
        }));

        let credentials = resolve_usage_credentials(&provider, &AppType::Gemini, &usage_script());

        assert_eq!(credentials.api_key, "g-legacy");
        assert_eq!(
            credentials.base_url,
            "https://generativelanguage.googleapis.com"
        );
    }

    #[test]
    fn codex_key_falls_back_to_experimental_bearer_token() {
        // Config-only Codex installs keep the key in config.toml's
        // experimental_bearer_token (no auth.OPENAI_API_KEY). The backend
        // resolver must mirror the frontend fallback or the saved-card refresh
        // path resolves empty credentials while "Test" works.
        let provider = provider_with_config(json!({
            "auth": {},
            "config": r#"
model_provider = "packycode"

[model_providers.packycode]
name = "PackyCode"
base_url = "https://api.packycode.com/v1"
experimental_bearer_token = "sk-bearer"
"#
        }));

        let credentials = resolve_usage_credentials(&provider, &AppType::Codex, &usage_script());

        assert_eq!(credentials.api_key, "sk-bearer");
        assert_eq!(credentials.base_url, "https://api.packycode.com/v1");
    }

    #[test]
    fn extracts_gemini_credentials_from_env() {
        let provider = provider_with_config(json!({
            "env": {
                "GEMINI_API_KEY": "gemini-key",
                "GOOGLE_GEMINI_BASE_URL": "https://gemini.example/"
            }
        }));

        let credentials = resolve_usage_credentials(&provider, &AppType::Gemini, &usage_script());

        assert_eq!(credentials.api_key, "gemini-key");
        assert_eq!(credentials.base_url, "https://gemini.example");
    }

    #[test]
    fn extracts_opencode_credentials_from_options() {
        let provider = provider_with_config(json!({
            "npm": "@ai-sdk/openai-compatible",
            "options": {
                "apiKey": "opencode-key",
                "baseURL": "https://opencode.example/v1/"
            }
        }));

        let credentials = resolve_usage_credentials(&provider, &AppType::OpenCode, &usage_script());

        assert_eq!(credentials.api_key, "opencode-key");
        assert_eq!(credentials.base_url, "https://opencode.example/v1");
    }

    #[test]
    fn extracts_openclaw_credentials_from_camel_case_fields() {
        let provider = provider_with_config(json!({
            "apiKey": "openclaw-key",
            "baseUrl": "https://openclaw.example/api/"
        }));

        let credentials = resolve_usage_credentials(&provider, &AppType::OpenClaw, &usage_script());

        assert_eq!(credentials.api_key, "openclaw-key");
        assert_eq!(credentials.base_url, "https://openclaw.example/api");
    }

    #[test]
    fn extracts_hermes_credentials_from_snake_case_fields() {
        let provider = provider_with_config(json!({
            "api_key": "hermes-key",
            "base_url": "https://hermes.example/api/"
        }));

        let credentials = resolve_usage_credentials(&provider, &AppType::Hermes, &usage_script());

        assert_eq!(credentials.api_key, "hermes-key");
        assert_eq!(credentials.base_url, "https://hermes.example/api");
    }

    #[test]
    fn query_template_resolver_reads_usage_script_from_meta_without_network() {
        let mut provider = provider_with_config(json!({
            "api_key": "hermes-key",
            "base_url": "https://hermes.example/api/"
        }));
        provider.meta = Some(ProviderMeta {
            usage_script: Some(UsageScript {
                template_type: Some(TEMPLATE_TYPE_BALANCE.to_string()),
                ..usage_script()
            }),
            ..Default::default()
        });

        let usage_script = provider
            .meta
            .as_ref()
            .and_then(|meta| meta.usage_script.as_ref())
            .expect("usage script should exist");
        let credentials = resolve_usage_credentials(&provider, &AppType::Hermes, usage_script);

        assert_eq!(credentials.api_key, "hermes-key");
        assert_eq!(credentials.base_url, "https://hermes.example/api");
    }

    // ---- validate_usage_script: empty request.url rejection (Bug 3) ----

    fn js_script_with_url(url: &str) -> String {
        format!(
            "({{\n  request: {{\n    url: \"{url}\",\n    method: \"GET\",\n    headers: {{}}\n  }},\n  extractor: function(r) {{ return {{ remaining: 0, unit: \"USD\" }}; }}\n}})"
        )
    }

    #[test]
    fn validate_usage_script_rejects_empty_url_for_custom_template() {
        let mut script = usage_script();
        script.enabled = true;
        script.template_type = Some("custom".to_string());
        script.code = js_script_with_url("");

        let err = validate_usage_script(&script).expect_err("empty url must be rejected");
        let msg = err.to_string();
        assert!(
            msg.contains("request.url")
                || msg.contains("Configure usage query")
                || msg.contains("用量查询"),
            "expected actionable empty-url message, got: {msg}"
        );
    }

    #[test]
    fn validate_usage_script_accepts_templated_url_for_custom_template() {
        let mut script = usage_script();
        script.enabled = true;
        script.template_type = Some("custom".to_string());
        // `{{baseUrl}}` survives the placeholder substitution as a non-empty
        // URL — we only block literal empty strings.
        script.code = js_script_with_url("{{baseUrl}}/v1/balance");

        validate_usage_script(&script).expect("templated url should pass");
    }

    #[test]
    fn validate_usage_script_skips_check_for_builtin_templates() {
        // Built-in templates ignore the JS body — they take a native dispatch
        // path that doesn't touch request.url. Empty url must NOT block save.
        let mut script = usage_script();
        script.enabled = true;
        script.template_type = Some(TEMPLATE_TYPE_BALANCE.to_string());
        script.code = js_script_with_url("");

        validate_usage_script(&script).expect("balance template should bypass url check");
    }

    #[test]
    fn validate_usage_script_skips_check_for_official_subscription_template() {
        // official_subscription has no JS body at all (code is ""): the save
        // path must not run the JS request.url validation against it.
        let mut script = usage_script();
        script.enabled = true;
        script.template_type = Some(TEMPLATE_TYPE_OFFICIAL_SUBSCRIPTION.to_string());
        script.code = String::new();

        validate_usage_script(&script)
            .expect("official_subscription template should bypass url check");
    }

    #[test]
    fn validate_usage_script_skips_check_when_disabled() {
        // Disabled scripts are user drafts — saving them with stale templates
        // should stay possible so the user can iterate.
        let mut script = usage_script();
        script.enabled = false;
        script.template_type = Some("custom".to_string());
        script.code = js_script_with_url("");

        validate_usage_script(&script).expect("disabled script should bypass url check");
    }

    // FIX A (round-2): native-template (token_plan/balance) base_url SSRF guard.
    // The guard is applied at the arm before the coding_plan/balance dial; it is
    // web-runtime-only (enforce=true) and skips empty base_urls.

    #[tokio::test]
    async fn native_template_guard_rejects_internal_base_url_when_enforced() {
        for internal in [
            "http://169.254.169.254/",
            "http://127.0.0.1:9999/",
            "http://10.0.0.1/",
        ] {
            let result = guard_native_template_base_url(internal, true).await;
            assert!(
                result.is_err(),
                "internal base_url must be rejected before dial: {internal}"
            );
        }
    }

    #[tokio::test]
    async fn native_template_guard_allows_public_base_url_when_enforced() {
        // Public IP literal (no DNS dependency): not in any blocked range.
        guard_native_template_base_url("https://1.1.1.1/api/usage", true)
            .await
            .expect("public base_url should pass the guard");
    }

    #[tokio::test]
    async fn native_template_guard_noop_for_desktop_and_empty() {
        // Desktop path (enforce=false) is unrestricted even for internal targets.
        guard_native_template_base_url("http://127.0.0.1:9999/", false)
            .await
            .expect("desktop path must not enforce the SSRF guard");
        // Empty base_url is skipped regardless of runtime.
        guard_native_template_base_url("", true)
            .await
            .expect("empty base_url must be skipped");
    }
}
