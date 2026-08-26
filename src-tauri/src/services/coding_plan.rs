//! 国产 Token Plan 额度查询服务
//!
//! 支持 Kimi For Coding、智谱 GLM、MiniMax 的 Token Plan 额度查询。
//! 复用 subscription 模块的 SubscriptionQuota / QuotaTier 类型。
//!
//! 错误通道语义：
//! - `Err(String)`: 瞬时传输失败（网络不可达/超时/读体中断）。前端 query
//!   reject 后触发 retry，并保留上一份成功数据。
//! - `Ok(success:false)`: 确定性失败（空 key/未知供应商/鉴权/非 2xx/响应体
//!   非法 JSON），立即透出错误文案。

use super::subscription::{
    CredentialStatus, QuotaTier, SubscriptionQuota, TIER_FIVE_HOUR, TIER_WEEKLY_LIMIT,
};
use std::time::{SystemTime, UNIX_EPOCH};

// ── 供应商检测 ──────────────────────────────────────────────

enum CodingPlanProvider {
    Kimi,
    ZhipuCn,
    ZhipuEn,
    MiniMaxCn,
    MiniMaxEn,
    ZenMux,
}

fn detect_provider(base_url: &str) -> Option<CodingPlanProvider> {
    let url = base_url.to_lowercase();
    if url.contains("api.kimi.com/coding") {
        Some(CodingPlanProvider::Kimi)
    } else if url.contains("open.bigmodel.cn") || url.contains("bigmodel.cn") {
        Some(CodingPlanProvider::ZhipuCn)
    } else if url.contains("api.z.ai") {
        Some(CodingPlanProvider::ZhipuEn)
    } else if url.contains("api.minimaxi.com") {
        Some(CodingPlanProvider::MiniMaxCn)
    } else if url.contains("api.minimax.io") {
        Some(CodingPlanProvider::MiniMaxEn)
    } else if url.contains("zenmux") {
        Some(CodingPlanProvider::ZenMux)
    } else {
        None
    }
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn millis_to_iso8601(ms: i64) -> Option<String> {
    let secs = ms / 1000;
    let nsecs = ((ms % 1000) * 1_000_000) as u32;
    chrono::DateTime::from_timestamp(secs, nsecs).map(|dt| dt.to_rfc3339())
}

/// 从 JSON 值提取重置时间，兼容字符串和数字格式
/// - 字符串：直接返回（ISO 8601）
/// - 数字：自动判断秒/毫秒并转为 ISO 8601
fn extract_reset_time(value: &serde_json::Value) -> Option<String> {
    if let Some(s) = value.as_str() {
        return Some(s.to_string());
    }
    if let Some(n) = value.as_i64() {
        // 区分秒和毫秒：秒级时间戳 < 1e12，毫秒 >= 1e12
        let ms = if n < 1_000_000_000_000 { n * 1000 } else { n };
        return millis_to_iso8601(ms);
    }
    None
}

/// 解析 JSON 值为 f64，兼容数字和字符串格式（如 `100` 和 `"100"`）
fn parse_f64(value: &serde_json::Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
}

fn make_error(msg: String) -> SubscriptionQuota {
    SubscriptionQuota {
        tool: "coding_plan".to_string(),
        credential_status: CredentialStatus::Valid,
        credential_message: None,
        success: false,
        tiers: vec![],
        extra_usage: None,
        error: Some(msg),
        queried_at: Some(now_millis()),
    }
}

enum JsonResponseError {
    Read(String),
    Parse(String),
}

async fn read_json_response(
    resp: reqwest::Response,
) -> Result<serde_json::Value, JsonResponseError> {
    let raw = resp
        .bytes()
        .await
        .map_err(|e| JsonResponseError::Read(format!("Failed to read response: {e}")))?;
    serde_json::from_slice(&raw)
        .map_err(|e| JsonResponseError::Parse(format!("Failed to parse response: {e}")))
}

// ── Kimi For Coding ─────────────────────────────────────────

async fn query_kimi(api_key: &str) -> Result<SubscriptionQuota, String> {
    let client = crate::proxy::http_client::get_guarded();

    let resp = client
        .get("https://api.kimi.com/coding/v1/usages")
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Accept", "application/json")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await;

    let resp = match resp {
        Ok(r) => r,
        Err(e) => return Err(format!("Network error: {e}")),
    };

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Ok(SubscriptionQuota {
            tool: "coding_plan".to_string(),
            credential_status: CredentialStatus::Expired,
            credential_message: Some("Invalid API key".to_string()),
            success: false,
            tiers: vec![],
            extra_usage: None,
            error: Some(format!("Authentication failed (HTTP {status})")),
            queried_at: Some(now_millis()),
        });
    }

    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Ok(make_error(format!("API error (HTTP {status}): {body}")));
    }

    let body: serde_json::Value = match read_json_response(resp).await {
        Ok(v) => v,
        Err(JsonResponseError::Read(e)) => return Err(e),
        Err(JsonResponseError::Parse(e)) => return Ok(make_error(e)),
    };

    let mut tiers = Vec::new();

    // 5 小时窗口限额（优先显示）
    if let Some(limits) = body.get("limits").and_then(|v| v.as_array()) {
        for limit_item in limits {
            if let Some(detail) = limit_item.get("detail") {
                let limit = detail.get("limit").and_then(parse_f64).unwrap_or(1.0);
                let remaining = detail.get("remaining").and_then(parse_f64).unwrap_or(0.0);
                let resets_at = detail.get("resetTime").and_then(extract_reset_time);

                let used = (limit - remaining).max(0.0);
                let utilization = if limit > 0.0 {
                    (used / limit) * 100.0
                } else {
                    0.0
                };
                tiers.push(QuotaTier {
                    name: "five_hour".to_string(),
                    utilization,
                    resets_at,
                    used_value_usd: None,
                    max_value_usd: None,
                });
            }
        }
    }

    // 总体用量（周限额）
    if let Some(usage) = body.get("usage") {
        let limit = usage.get("limit").and_then(parse_f64).unwrap_or(1.0);
        let remaining = usage.get("remaining").and_then(parse_f64).unwrap_or(0.0);
        let resets_at = usage.get("resetTime").and_then(extract_reset_time);

        let used = (limit - remaining).max(0.0);
        let utilization = if limit > 0.0 {
            (used / limit) * 100.0
        } else {
            0.0
        };
        tiers.push(QuotaTier {
            name: "weekly_limit".to_string(),
            utilization,
            resets_at,
            used_value_usd: None,
            max_value_usd: None,
        });
    }

    Ok(SubscriptionQuota {
        tool: "coding_plan".to_string(),
        credential_status: CredentialStatus::Valid,
        credential_message: None,
        success: true,
        tiers,
        extra_usage: None,
        error: None,
        queried_at: Some(now_millis()),
    })
}

// ── 智谱 GLM ────────────────────────────────────────────────

/// 把智谱 `data` 里的 `limits[]` 解析成 tier 列表。
///
/// 双桶响应中，5 小时桶在 0% 等状态下可能没有 `nextResetTime`；
/// 这类无 reset 条目应优先归为五小时桶。其余条目按 `nextResetTime` 升序。
/// 老套餐（2026-02-12 前订阅）只回 1 条
/// `TOKENS_LIMIT`，自然降级为仅展示 `five_hour`；新套餐回 2 条。
fn parse_zhipu_token_tiers(data: &serde_json::Value) -> Vec<QuotaTier> {
    let mut token_limits: Vec<(Option<i64>, f64, Option<String>)> = Vec::new();
    if let Some(limits) = data.get("limits").and_then(|v| v.as_array()) {
        for limit_item in limits {
            let limit_type = limit_item
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            // 大小写不敏感比较：上游若把 "TOKENS_LIMIT" 改成小写或驼峰，依然能识别
            if !(limit_type.eq_ignore_ascii_case("TOKENS_LIMIT")
                || limit_type.eq_ignore_ascii_case("CREDIT_LIMIT"))
            {
                continue;
            }
            let percentage = limit_item
                .get("percentage")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let reset_ms = limit_item.get("nextResetTime").and_then(|v| v.as_i64());
            let reset_iso = reset_ms.and_then(millis_to_iso8601);
            token_limits.push((reset_ms, percentage, reset_iso));
        }
    }
    token_limits.sort_by_key(|(reset, _, _)| (reset.is_some(), reset.unwrap_or(i64::MIN)));

    token_limits
        .into_iter()
        .enumerate()
        .filter_map(|(idx, (_, percentage, resets_at))| {
            let name = match idx {
                0 => TIER_FIVE_HOUR,
                1 => TIER_WEEKLY_LIMIT,
                _ => return None, // 智谱当前最多两条 TOKENS_LIMIT，多余的忽略
            };
            Some(QuotaTier {
                name: name.to_string(),
                utilization: percentage,
                resets_at,
                used_value_usd: None,
                max_value_usd: None,
            })
        })
        .collect()
}

/// Resolve the Zhipu quota endpoint from the user's configured `base_url`.
///
/// Zhipu ships as two distinct presets (Zhipu GLM = `open.bigmodel.cn`,
/// Zhipu GLM en = `api.z.ai`) that share the same quota path and JSON shape.
/// The quota endpoint lives on the same host as the user's coding endpoint,
/// so we route by `base_url` and let the caller's existing reachability
/// (they're already using this host to run coding) determine success — no
/// cross-host fallback, no auth-error heuristics.
fn zhipu_quota_base(base_url: &str) -> &'static str {
    if base_url.to_lowercase().contains("bigmodel.cn") {
        "https://open.bigmodel.cn"
    } else {
        "https://api.z.ai"
    }
}

async fn query_zhipu(base_url: &str, api_key: &str) -> Result<SubscriptionQuota, String> {
    let client = crate::proxy::http_client::get_guarded();
    let url = format!(
        "{}/api/monitor/usage/quota/limit",
        zhipu_quota_base(base_url)
    );

    let resp = client
        .get(&url)
        .header("Authorization", api_key) // 注意：智谱不加 Bearer 前缀
        .header("Content-Type", "application/json")
        .header("Accept-Language", "en-US,en")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await;

    let resp = match resp {
        Ok(r) => r,
        Err(e) => return Err(format!("Network error: {e}")),
    };

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Ok(SubscriptionQuota {
            tool: "coding_plan".to_string(),
            credential_status: CredentialStatus::Expired,
            credential_message: Some("Invalid API key".to_string()),
            success: false,
            tiers: vec![],
            extra_usage: None,
            error: Some(format!("Authentication failed (HTTP {status})")),
            queried_at: Some(now_millis()),
        });
    }

    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Ok(make_error(format!("API error (HTTP {status}): {body}")));
    }

    let body: serde_json::Value = match read_json_response(resp).await {
        Ok(v) => v,
        Err(JsonResponseError::Read(e)) => return Err(e),
        Err(JsonResponseError::Parse(e)) => return Ok(make_error(e)),
    };

    Ok(zhipu_quota_from_body(&body))
}

/// 解析智谱额度响应体（个人版与团队版共用同一 shape）。
/// 仅在 HTTP 成功、body 已完整读取并解析为 JSON 后调用——本函数不做任何网络 IO，
/// 故无瞬时失败通道，确定性失败直接落进 `Ok(success:false)`。
fn zhipu_quota_from_body(body: &serde_json::Value) -> SubscriptionQuota {
    // 检查业务级别错误
    if body.get("success").and_then(|v| v.as_bool()) == Some(false) {
        let msg = body
            .get("msg")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");
        return make_error(format!("API error: {msg}"));
    }

    let data = match body.get("data") {
        Some(d) => d,
        None => return make_error("Missing 'data' field in response".to_string()),
    };

    let tiers = parse_zhipu_token_tiers(data);

    // 套餐等级存入 credential_message
    let level = data
        .get("level")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    SubscriptionQuota {
        tool: "coding_plan".to_string(),
        credential_status: CredentialStatus::Valid,
        credential_message: level,
        success: true,
        tiers,
        extra_usage: None,
        error: None,
        queried_at: Some(now_millis()),
    }
}

// ── MiniMax ─────────────────────────────────────────────────

async fn query_minimax(api_key: &str, is_cn: bool) -> Result<SubscriptionQuota, String> {
    let client = crate::proxy::http_client::get_guarded();

    let api_domain = if is_cn {
        "api.minimaxi.com"
    } else {
        "api.minimax.io"
    };
    let url = format!("https://{api_domain}/v1/api/openplatform/coding_plan/remains");

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await;

    let resp = match resp {
        Ok(r) => r,
        Err(e) => return Err(format!("Network error: {e}")),
    };

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Ok(SubscriptionQuota {
            tool: "coding_plan".to_string(),
            credential_status: CredentialStatus::Expired,
            credential_message: Some("Invalid API key".to_string()),
            success: false,
            tiers: vec![],
            extra_usage: None,
            error: Some(format!("Authentication failed (HTTP {status})")),
            queried_at: Some(now_millis()),
        });
    }

    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Ok(make_error(format!("API error (HTTP {status}): {body}")));
    }

    let body: serde_json::Value = match read_json_response(resp).await {
        Ok(v) => v,
        Err(JsonResponseError::Read(e)) => return Err(e),
        Err(JsonResponseError::Parse(e)) => return Ok(make_error(e)),
    };

    // 检查业务级别错误
    if let Some(base_resp) = body.get("base_resp") {
        let status_code = base_resp
            .get("status_code")
            .and_then(|v| v.as_i64())
            .unwrap_or(-1);
        if status_code != 0 {
            let msg = base_resp
                .get("status_msg")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            return Ok(make_error(format!("API error (code {status_code}): {msg}")));
        }
    }

    // 提取纯函数便于无 mock 单元测试;新接口直接给"剩余百分比",反转为已用百分比
    let tiers = parse_minimax_tiers(&body);

    Ok(SubscriptionQuota {
        tool: "coding_plan".to_string(),
        credential_status: CredentialStatus::Valid,
        credential_message: None,
        success: true,
        tiers,
        extra_usage: None,
        error: None,
        queried_at: Some(now_millis()),
    })
}

// ── ZenMux ──────────────────────────────────────────────────

async fn query_zenmux(base_url: &str, api_key: &str) -> Result<SubscriptionQuota, String> {
    let client = crate::proxy::http_client::get_guarded();

    let resp = client
        .get(base_url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Accept", "application/json")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await;

    let resp = match resp {
        Ok(r) => r,
        Err(e) => return Err(format!("Network error: {e}")),
    };

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Ok(SubscriptionQuota {
            tool: "coding_plan".to_string(),
            credential_status: CredentialStatus::Expired,
            credential_message: Some("Invalid API key".to_string()),
            success: false,
            tiers: vec![],
            extra_usage: None,
            error: Some(format!("Authentication failed (HTTP {status})")),
            queried_at: Some(now_millis()),
        });
    }

    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Ok(make_error(format!("API error (HTTP {status}): {body}")));
    }

    let body: serde_json::Value = match read_json_response(resp).await {
        Ok(v) => v,
        Err(JsonResponseError::Read(e)) => return Err(e),
        Err(JsonResponseError::Parse(e)) => return Ok(make_error(e)),
    };

    // 检查业务级别错误
    if body.get("success").and_then(|v| v.as_bool()) != Some(true) {
        let msg = body
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");
        return Ok(make_error(format!("API error: {msg}")));
    }

    let data = match body.get("data") {
        Some(d) => d,
        None => return Ok(make_error("Missing 'data' field in response".to_string())),
    };

    let mut tiers = Vec::new();

    // 5 小时窗口限额
    if let Some(q5h) = data.get("quota_5_hour") {
        let usage_pct = q5h
            .get("usage_percentage")
            .and_then(parse_f64)
            .unwrap_or(0.0);
        let resets_at = q5h
            .get("resets_at")
            .and_then(|v| v.as_str())
            .map(String::from);
        let used_usd = q5h.get("used_value_usd").and_then(parse_f64);
        let max_usd = q5h.get("max_value_usd").and_then(parse_f64);
        tiers.push(QuotaTier {
            name: "five_hour".to_string(),
            utilization: usage_pct * 100.0,
            resets_at,
            used_value_usd: used_usd,
            max_value_usd: max_usd,
        });
    }

    // 7 天窗口限额
    if let Some(q7d) = data.get("quota_7_day") {
        let usage_pct = q7d
            .get("usage_percentage")
            .and_then(parse_f64)
            .unwrap_or(0.0);
        let resets_at = q7d
            .get("resets_at")
            .and_then(|v| v.as_str())
            .map(String::from);
        let used_usd = q7d.get("used_value_usd").and_then(parse_f64);
        let max_usd = q7d.get("max_value_usd").and_then(parse_f64);
        tiers.push(QuotaTier {
            name: "weekly_limit".to_string(),
            utilization: usage_pct * 100.0,
            resets_at,
            used_value_usd: used_usd,
            max_value_usd: max_usd,
        });
    }

    // 套餐等级和账户状态存入 credential_message
    let plan_tier = data
        .get("plan")
        .and_then(|p| p.get("tier"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let account_status = data
        .get("account_status")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let plan_info = if !plan_tier.is_empty() {
        format!("{plan_tier} ({account_status})")
    } else {
        String::new()
    };

    Ok(SubscriptionQuota {
        tool: "coding_plan".to_string(),
        credential_status: CredentialStatus::Valid,
        credential_message: if plan_info.is_empty() {
            None
        } else {
            Some(plan_info)
        },
        success: true,
        tiers,
        extra_usage: None,
        error: None,
        queried_at: Some(now_millis()),
    })
}

/// 从 `/coding_plan/remains` 响应中解析 MiniMax 编程套餐的额度 tier。
///
/// 新接口语义:`current_*_remaining_percent` 是"剩余百分比"(0-100),
/// `model_remains` 数组里有 `general`(编程套餐)和 `video` 等其他模型,
/// 这里只取 `general`,跳过 video。
///
/// 5h 桶始终存在;周桶并非所有套餐都有,靠 `current_weekly_status == 1`
/// 判定激活(无周限额套餐该字段为 3,`remaining_percent` 恒为 100,不应展示)。
fn parse_minimax_tiers(body: &serde_json::Value) -> Vec<QuotaTier> {
    let mut tiers = Vec::new();

    let Some(model_remains) = body.get("model_remains").and_then(|v| v.as_array()) else {
        return tiers;
    };

    // 只取 model_name == "general" 的条目,跳过 video 等非编程模型
    let Some(item) = model_remains.iter().find(|item| {
        item.get("model_name")
            .and_then(|v| v.as_str())
            .map(|s| s == "general")
            .unwrap_or(false)
    }) else {
        return tiers;
    };

    // 5h 桶:剩余百分比 → 已用百分比
    if let Some(remain_pct) = item
        .get("current_interval_remaining_percent")
        .and_then(|v| v.as_f64())
    {
        let resets_at = item
            .get("end_time")
            .and_then(|v| v.as_i64())
            .and_then(millis_to_iso8601);
        tiers.push(QuotaTier {
            name: TIER_FIVE_HOUR.to_string(),
            utilization: 100.0 - remain_pct,
            resets_at,
            used_value_usd: None,
            max_value_usd: None,
        });
    }

    // 周桶:仅当 status=1 时激活;status=3 等表示该套餐无周限额,跳过
    if item.get("current_weekly_status").and_then(|v| v.as_i64()) == Some(1) {
        if let Some(remain_pct) = item
            .get("current_weekly_remaining_percent")
            .and_then(|v| v.as_f64())
        {
            let resets_at = item
                .get("weekly_end_time")
                .and_then(|v| v.as_i64())
                .and_then(millis_to_iso8601);
            tiers.push(QuotaTier {
                name: TIER_WEEKLY_LIMIT.to_string(),
                utilization: 100.0 - remain_pct,
                resets_at,
                used_value_usd: None,
                max_value_usd: None,
            });
        }
    }

    tiers
}

// ── 公开入口 ────────────────────────────────────────────────

/// 构造"凭据缺失 / 域名未命中"的失败结果（NotFound 状态 + 明确错误文案）。
fn coding_plan_not_found(error: &str) -> SubscriptionQuota {
    SubscriptionQuota {
        tool: "coding_plan".to_string(),
        credential_status: CredentialStatus::NotFound,
        credential_message: None,
        success: false,
        tiers: vec![],
        extra_usage: None,
        error: Some(error.to_string()),
        queried_at: None,
    }
}

// ── 智谱团队套餐（Team Plan）──────────────────────────────────
//
// 与个人版的差异仅在请求构造（参考 token-monitor/src/shared/zaiTeamLimits.js）：
// - 固定走国内站 open.bigmodel.cn（团队版仅存在于国内站，z.ai 国际站无 team 档）
// - 同一 quota 路径加 `?type=2`
// - 额外请求头 bigmodel-organization / bigmodel-project（两者 + api_key 缺一不可）
// 响应 shape 与个人版完全一致 → 复用 zhipu_quota_from_body / parse_zhipu_token_tiers。
const ZHIPU_TEAM_QUOTA_URL: &str = "https://open.bigmodel.cn/api/monitor/usage/quota/limit";

async fn query_zhipu_team(
    api_key: &str,
    organization_id: &str,
    project_id: &str,
) -> Result<SubscriptionQuota, String> {
    query_zhipu_team_at(ZHIPU_TEAM_QUOTA_URL, api_key, organization_id, project_id).await
}

/// 团队版额度查询。`quota_url_base` 为不含 query 的 quota 端点；团队版与个人版同路径，
/// 靠 `?type=2` 区分（在此拼上）。拆出 url 参数便于用本地 server 测试请求形状。
async fn query_zhipu_team_at(
    quota_url_base: &str,
    api_key: &str,
    organization_id: &str,
    project_id: &str,
) -> Result<SubscriptionQuota, String> {
    let client = crate::proxy::http_client::get_guarded();
    let url = format!("{quota_url_base}?type=2");

    let resp = client
        .get(&url)
        .header("Authorization", api_key) // 与个人版一致：智谱不加 Bearer 前缀
        .header("bigmodel-organization", organization_id)
        .header("bigmodel-project", project_id)
        .header("Content-Type", "application/json")
        .header("Accept-Language", "en-US,en")
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await;

    let resp = match resp {
        Ok(r) => r,
        Err(e) => return Err(format!("Network error: {e}")),
    };

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Ok(SubscriptionQuota {
            tool: "coding_plan".to_string(),
            credential_status: CredentialStatus::Expired,
            credential_message: Some("Invalid API key".to_string()),
            success: false,
            tiers: vec![],
            extra_usage: None,
            error: Some(format!("Authentication failed (HTTP {status})")),
            queried_at: Some(now_millis()),
        });
    }

    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Ok(make_error(format!("API error (HTTP {status}): {body}")));
    }

    // 先 bytes() 再解析：读体失败（超时/连接中断）是瞬时 → Err；拿到完整响应体
    // 后解析失败才是确定性。reqwest 的 json() 把读体错误也包成 decode，无法区分。
    let raw = match resp.bytes().await {
        Ok(b) => b,
        Err(e) => return Err(format!("Failed to read response: {e}")),
    };
    let body: serde_json::Value = match serde_json::from_slice(&raw) {
        Ok(v) => v,
        Err(e) => return Ok(make_error(format!("Failed to parse response: {e}"))),
    };

    Ok(zhipu_quota_from_body(&body))
}

/// 查询编程套餐额度。瞬时传输失败（网络/超时/读体中断）返回 `Err`（前端 reject →
/// retry + 保留上次成功值）；确定性失败（凭据缺失/未知域名/鉴权/非 2xx/业务错误）
/// 返回 `Ok(success:false)` 立即透出文案。判定按 reqwest 错误种类在折叠点完成。
///
/// `coding_plan_provider` 显式标识用于无法靠 base_url 区分的供应商（当前为智谱团队版
/// `zhipu_team`——其 base_url 与个人版智谱相同）；其余情况走 `detect_provider`。
pub async fn get_coding_plan_quota(
    base_url: &str,
    api_key: &str,
    coding_plan_provider: Option<&str>,
    team_organization_id: Option<&str>,
    team_project_id: Option<&str>,
) -> Result<SubscriptionQuota, String> {
    // 智谱团队版：base_url 与个人版智谱（open.bigmodel.cn）相同，detect_provider 无法
    // 区分，必须靠显式 coding_plan_provider == "zhipu_team" 路由。需 api_key + 组织 ID
    // + 项目 ID 三者齐全，缺任一返回 NotFound 引导补全。
    if coding_plan_provider
        .map(|p| p.eq_ignore_ascii_case("zhipu_team"))
        .unwrap_or(false)
    {
        let organization_id = team_organization_id.unwrap_or("").trim();
        let project_id = team_project_id.unwrap_or("").trim();
        if api_key.trim().is_empty() || organization_id.is_empty() || project_id.is_empty() {
            return Ok(coding_plan_not_found(
                "Zhipu team plan needs the API key + organization ID + project ID",
            ));
        }
        return query_zhipu_team(api_key, organization_id, project_id).await;
    }

    if api_key.trim().is_empty() {
        return Ok(SubscriptionQuota {
            tool: "coding_plan".to_string(),
            credential_status: CredentialStatus::NotFound,
            credential_message: None,
            success: false,
            tiers: vec![],
            extra_usage: None,
            error: None,
            queried_at: None,
        });
    }

    let provider = match detect_provider(base_url) {
        Some(p) => p,
        None => {
            return Ok(SubscriptionQuota {
                tool: "coding_plan".to_string(),
                credential_status: CredentialStatus::NotFound,
                credential_message: None,
                success: false,
                tiers: vec![],
                extra_usage: None,
                error: None,
                queried_at: None,
            })
        }
    };

    match provider {
        CodingPlanProvider::Kimi => query_kimi(api_key).await,
        CodingPlanProvider::ZhipuCn | CodingPlanProvider::ZhipuEn => {
            query_zhipu(base_url, api_key).await
        }
        CodingPlanProvider::MiniMaxCn => query_minimax(api_key, true).await,
        CodingPlanProvider::MiniMaxEn => query_minimax(api_key, false).await,
        CodingPlanProvider::ZenMux => query_zenmux(base_url, api_key).await,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        get_coding_plan_quota, parse_minimax_tiers, parse_zhipu_token_tiers, query_zhipu_team_at,
        zhipu_quota_base, CredentialStatus, TIER_FIVE_HOUR, TIER_WEEKLY_LIMIT,
    };
    use serde_json::json;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn no_proxy_for_loopback() {
        std::env::set_var("NO_PROXY", "127.0.0.1,localhost");
        std::env::set_var("no_proxy", "127.0.0.1,localhost");
    }

    fn http_response(status: &str, body: &str) -> String {
        format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
            body.len()
        )
    }

    fn spawn_once_server(response: Option<String>) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let addr = listener.local_addr().expect("local addr");
        let handle = thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0_u8; 1024];
                let _ = stream.read(&mut buf);
                if let Some(response) = response {
                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.flush();
                }
            }
        });
        (format!("http://{addr}/zenmux"), handle)
    }

    #[test]
    fn zhipu_new_plan_two_tiers_sorted_by_reset_time() {
        // 新套餐：两条 TOKENS_LIMIT，nextResetTime 较近的归 five_hour、较远的归 weekly_limit。
        // 故意把"周限"放数组前面，验证不依赖输入顺序。
        let data = json!({
            "limits": [
                { "type": "TOKENS_LIMIT", "percentage": 53.0, "nextResetTime": 2_000_000_000_000_i64 },
                { "type": "TOKENS_LIMIT", "percentage": 44.0, "nextResetTime": 1_000_000_000_000_i64 },
                { "type": "TIME_LIMIT",   "percentage":  7.0 },
            ]
        });
        let tiers = parse_zhipu_token_tiers(&data);
        assert_eq!(tiers.len(), 2);
        assert_eq!(tiers[0].name, TIER_FIVE_HOUR);
        assert_eq!(tiers[0].utilization, 44.0);
        assert_eq!(tiers[1].name, TIER_WEEKLY_LIMIT);
        assert_eq!(tiers[1].utilization, 53.0);
    }

    #[test]
    fn zhipu_old_plan_single_tier_falls_back_to_five_hour() {
        // 老套餐（2026-02-12 前订阅）：仅一条 TOKENS_LIMIT，无周限。
        let data = json!({
            "limits": [
                {
                    "type": "TOKENS_LIMIT",
                    "percentage": 2.0,
                    "nextResetTime": 1_774_967_594_803_i64
                },
                { "type": "TIME_LIMIT", "percentage": 0.0 }
            ]
        });
        let tiers = parse_zhipu_token_tiers(&data);
        assert_eq!(tiers.len(), 1);
        assert_eq!(tiers[0].name, TIER_FIVE_HOUR);
        assert_eq!(tiers[0].utilization, 2.0);
    }

    #[test]
    fn zhipu_no_token_limits_returns_empty() {
        let data = json!({ "limits": [{ "type": "TIME_LIMIT", "percentage": 5.0 }] });
        assert!(parse_zhipu_token_tiers(&data).is_empty());
    }

    #[test]
    fn zhipu_missing_reset_time_is_five_hour_when_weekly_has_reset() {
        // 真实反馈：5 小时桶为 0% 时可能没有 nextResetTime；每周桶带 reset。
        // 这种形态不能按 reset 升序把每周桶误判为 five_hour。
        let data = json!({
            "limits": [
                { "type": "TOKENS_LIMIT", "percentage": 25.0, "nextResetTime": 2_000_000_000_000_i64 },
                { "type": "TOKENS_LIMIT", "percentage": 0.0 }
            ]
        });
        let tiers = parse_zhipu_token_tiers(&data);
        assert_eq!(tiers.len(), 2);
        assert_eq!(tiers[0].name, TIER_FIVE_HOUR);
        assert_eq!(tiers[0].utilization, 0.0);
        assert!(tiers[0].resets_at.is_none());
        assert_eq!(tiers[1].name, TIER_WEEKLY_LIMIT);
        assert_eq!(tiers[1].utilization, 25.0);
        assert!(tiers[1].resets_at.is_some());
    }

    #[test]
    fn zhipu_type_is_case_insensitive() {
        // 防御性：上游若把 "TOKENS_LIMIT" 改成 "tokens_limit"（仅大小写变化）仍能识别。
        // 注意：分隔符差异（如 "TokensLimit" 去掉下划线）不在兼容范围。
        let data = json!({
            "limits": [
                { "type": "tokens_limit", "percentage": 12.0, "nextResetTime": 1_000_000_000_000_i64 },
                { "type": "Tokens_Limit", "percentage": 34.0, "nextResetTime": 2_000_000_000_000_i64 }
            ]
        });
        let tiers = parse_zhipu_token_tiers(&data);
        assert_eq!(tiers.len(), 2);
        assert_eq!(tiers[0].name, TIER_FIVE_HOUR);
        assert_eq!(tiers[0].utilization, 12.0);
        assert_eq!(tiers[1].name, TIER_WEEKLY_LIMIT);
        assert_eq!(tiers[1].utilization, 34.0);
    }

    #[test]
    fn zhipu_invalid_percentage_falls_back_to_zero() {
        // percentage 为字符串或 null 时不应崩溃，按 0 处理（仍展示 tier，但用量为 0）。
        let data = json!({
            "limits": [
                { "type": "TOKENS_LIMIT", "percentage": "invalid", "nextResetTime": 1_000_000_000_000_i64 },
                { "type": "TOKENS_LIMIT", "percentage": null,      "nextResetTime": 2_000_000_000_000_i64 }
            ]
        });
        let tiers = parse_zhipu_token_tiers(&data);
        assert_eq!(tiers.len(), 2);
        assert_eq!(tiers[0].utilization, 0.0);
        assert_eq!(tiers[1].utilization, 0.0);
    }

    #[test]
    fn zhipu_extreme_percentage_values_pass_through() {
        // 负数 / 超 100 不做范围裁剪：下游渲染层负责显示策略，解析层只负责忠实搬运。
        let data = json!({
            "limits": [
                { "type": "TOKENS_LIMIT", "percentage": -5.0,  "nextResetTime": 1_000_000_000_000_i64 },
                { "type": "TOKENS_LIMIT", "percentage": 150.0, "nextResetTime": 2_000_000_000_000_i64 }
            ]
        });
        let tiers = parse_zhipu_token_tiers(&data);
        assert_eq!(tiers.len(), 2);
        assert_eq!(tiers[0].utilization, -5.0);
        assert_eq!(tiers[1].utilization, 150.0);
    }

    #[test]
    fn zhipu_more_than_two_token_limits_keeps_first_two() {
        // 防御性：智谱当前最多两条 TOKENS_LIMIT，若上游意外增加第三条应被丢弃，避免命名空缺。
        let data = json!({
            "limits": [
                { "type": "TOKENS_LIMIT", "percentage": 1.0, "nextResetTime": 1_000_000_000_000_i64 },
                { "type": "TOKENS_LIMIT", "percentage": 2.0, "nextResetTime": 2_000_000_000_000_i64 },
                { "type": "TOKENS_LIMIT", "percentage": 3.0, "nextResetTime": 3_000_000_000_000_i64 }
            ]
        });
        let tiers = parse_zhipu_token_tiers(&data);
        assert_eq!(tiers.len(), 2);
        assert_eq!(tiers[0].name, TIER_FIVE_HOUR);
        assert_eq!(tiers[1].name, TIER_WEEKLY_LIMIT);
    }

    #[tokio::test]
    async fn transient_connection_closed_before_response_returns_err() {
        no_proxy_for_loopback();
        let (base_url, handle) = spawn_once_server(None);

        let err = get_coding_plan_quota(&base_url, "k", None, None, None)
            .await
            .expect_err("响应前连接中断必须走 Err 通道");

        assert!(err.contains("Network error"), "err={err}");
        handle.join().expect("server thread");
    }

    #[tokio::test]
    async fn transient_truncated_body_returns_err() {
        no_proxy_for_loopback();
        let (base_url, handle) = spawn_once_server(Some(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 100\r\n\r\npartial"
                .to_string(),
        ));

        let err = get_coding_plan_quota(&base_url, "k", None, None, None)
            .await
            .expect_err("读体中断必须走 Err 通道");

        assert!(err.contains("Failed to read response"), "err={err}");
        handle.join().expect("server thread");
    }

    #[tokio::test]
    async fn deterministic_http_401_stays_ok_with_auth_error() {
        no_proxy_for_loopback();
        let (base_url, handle) = spawn_once_server(Some(http_response("401 Unauthorized", "{}")));

        let quota = get_coding_plan_quota(&base_url, "k", None, None, None)
            .await
            .expect("鉴权失败必须保持 Ok(success:false)");

        assert!(!quota.success);
        assert!(matches!(quota.credential_status, CredentialStatus::Expired));
        let err = quota.error.expect("应有错误文案");
        assert!(err.contains("Authentication failed (HTTP 401"), "err={err}");
        handle.join().expect("server thread");
    }

    #[tokio::test]
    async fn deterministic_http_429_stays_ok_with_status_in_error() {
        no_proxy_for_loopback();
        let (base_url, handle) =
            spawn_once_server(Some(http_response("429 Too Many Requests", "slow down")));

        let quota = get_coding_plan_quota(&base_url, "k", None, None, None)
            .await
            .expect("非 2xx 必须保持 Ok(success:false)");

        assert!(!quota.success);
        let err = quota.error.expect("应有错误文案");
        assert!(err.contains("HTTP 429"), "err={err}");
        handle.join().expect("server thread");
    }

    #[tokio::test]
    async fn deterministic_invalid_json_body_stays_ok_with_parse_error() {
        no_proxy_for_loopback();
        let (base_url, handle) = spawn_once_server(Some(http_response("200 OK", "not-json")));

        let quota = get_coding_plan_quota(&base_url, "k", None, None, None)
            .await
            .expect("完整但非法响应体必须保持 Ok(success:false)");

        assert!(!quota.success);
        let err = quota.error.expect("应有错误文案");
        assert!(err.contains("Failed to parse response"), "err={err}");
        handle.join().expect("server thread");
    }

    // ── MiniMax ──

    #[test]
    fn minimax_general_two_tiers_from_remaining_percent() {
        // 主路径:general 桶 5h 剩 98% / weekly 剩 95% → 已用 2% / 5%
        let body = json!({
            "model_remains": [
                {
                    "model_name": "general",
                    "current_interval_remaining_percent": 98.0,
                    "current_weekly_remaining_percent": 95.0,
                    "current_interval_status": 1,
                    "current_weekly_status": 1,
                    "end_time": 1_780_329_600_000_i64,
                    "weekly_end_time": 1_780_848_000_000_i64
                },
                {
                    "model_name": "video",
                    "current_interval_remaining_percent": 100.0,
                    "current_weekly_remaining_percent": 100.0
                }
            ],
            "base_resp": { "status_code": 0, "status_msg": "success" }
        });
        let tiers = parse_minimax_tiers(&body);
        assert_eq!(tiers.len(), 2);
        assert_eq!(tiers[0].name, TIER_FIVE_HOUR);
        assert_eq!(tiers[0].utilization, 2.0);
        assert!(tiers[0].resets_at.is_some());
        assert_eq!(tiers[1].name, TIER_WEEKLY_LIMIT);
        assert_eq!(tiers[1].utilization, 5.0);
        assert!(tiers[1].resets_at.is_some());
    }

    #[test]
    fn minimax_skips_video_and_finds_general_in_any_position() {
        // 防御性:即使 video 排在数组前面,general 排在后面,仍应被定位到。
        let body = json!({
            "model_remains": [
                {
                    "model_name": "video",
                    "current_interval_remaining_percent": 50.0,
                    "current_weekly_remaining_percent": 50.0
                },
                {
                    "model_name": "general",
                    "current_interval_remaining_percent": 80.0,
                    "current_weekly_remaining_percent": 70.0,
                    "current_interval_status": 1,
                    "current_weekly_status": 1
                }
            ]
        });
        let tiers = parse_minimax_tiers(&body);
        assert_eq!(tiers.len(), 2);
        // 取的是 general 桶,不是 video(20%/30% 而非 50%/50%)
        assert_eq!(tiers[0].utilization, 20.0);
        assert_eq!(tiers[1].utilization, 30.0);
    }

    #[test]
    fn minimax_missing_general_returns_empty() {
        // model_remains 只有 video / 空 / 缺字段 → 不应崩溃,tiers 为空
        let body = json!({
            "model_remains": [
                {
                    "model_name": "video",
                    "current_interval_remaining_percent": 100.0,
                    "current_weekly_remaining_percent": 100.0
                }
            ]
        });
        assert!(parse_minimax_tiers(&body).is_empty());

        let body_empty: serde_json::Value = json!({ "model_remains": [] });
        assert!(parse_minimax_tiers(&body_empty).is_empty());

        let body_no_field = json!({});
        assert!(parse_minimax_tiers(&body_no_field).is_empty());
    }

    #[test]
    fn minimax_missing_percent_fields_skips_tier() {
        // 字段缺失时只跳过对应桶,另一边仍能展示
        let body = json!({
            "model_remains": [{
                "model_name": "general",
                "current_interval_remaining_percent": 60.0,
                "current_weekly_status": 1
                // 缺 current_weekly_remaining_percent
            }]
        });
        let tiers = parse_minimax_tiers(&body);
        assert_eq!(tiers.len(), 1);
        assert_eq!(tiers[0].name, TIER_FIVE_HOUR);
        assert_eq!(tiers[0].utilization, 40.0);
    }

    #[test]
    fn minimax_negative_percent_passes_through() {
        // 防御性:与 parse_zhipu_token_tiers 约定一致,负数 / 超 100 不做范围裁剪
        let body = json!({
            "model_remains": [{
                "model_name": "general",
                "current_interval_remaining_percent": -5.0,
                "current_weekly_remaining_percent": 150.0,
                "current_interval_status": 1,
                "current_weekly_status": 1
            }]
        });
        let tiers = parse_minimax_tiers(&body);
        assert_eq!(tiers.len(), 2);
        assert_eq!(tiers[0].utilization, 105.0); // 100 - (-5)
        assert_eq!(tiers[1].utilization, -50.0); // 100 - 150
    }

    #[test]
    fn minimax_weekly_status_3_skips_weekly_tier() {
        // 无周限额套餐:current_weekly_status=3,remaining_percent 恒为 100,
        // 不应推 weekly_limit tier(否则会显示"0% 已用"的假周桶)
        let body = json!({
            "model_remains": [
                {
                    "model_name": "general",
                    "start_time": 1_780_347_600_000_i64,
                    "end_time": 1_780_365_600_000_i64,
                    "remains_time": 4_161_372_i64,
                    "current_interval_remaining_percent": 99,
                    "current_interval_status": 1,
                    "current_weekly_total_count": 0,
                    "current_weekly_usage_count": 0,
                    "weekly_start_time": 1_780_243_200_000_i64,
                    "weekly_end_time": 1_780_848_000_000_i64,
                    "weekly_remains_time": 486_561_372_i64,
                    "current_weekly_status": 3,
                    "current_weekly_remaining_percent": 100
                },
                {
                    "model_name": "video",
                    "current_interval_remaining_percent": 100,
                    "current_weekly_status": 3,
                    "current_weekly_remaining_percent": 100
                }
            ],
            "base_resp": { "status_code": 0, "status_msg": "success" }
        });
        let tiers = parse_minimax_tiers(&body);
        assert_eq!(tiers.len(), 1);
        assert_eq!(tiers[0].name, TIER_FIVE_HOUR);
        assert_eq!(tiers[0].utilization, 1.0);
        assert!(tiers[0].resets_at.is_some());
    }

    #[test]
    fn minimax_weekly_status_2_also_skips_weekly_tier() {
        // 防御性:除 1 之外的 status 都视为周桶未激活,跳过
        let body = json!({
            "model_remains": [{
                "model_name": "general",
                "current_interval_remaining_percent": 80.0,
                "current_weekly_remaining_percent": 50.0,
                "current_weekly_status": 2
            }]
        });
        let tiers = parse_minimax_tiers(&body);
        assert_eq!(tiers.len(), 1);
        assert_eq!(tiers[0].name, TIER_FIVE_HOUR);
        assert_eq!(tiers[0].utilization, 20.0);
    }

    #[test]
    fn zhipu_quota_base_routes_bigmodel_url_to_cn_endpoint() {
        assert_eq!(
            zhipu_quota_base("https://open.bigmodel.cn/api/paas/v4"),
            "https://open.bigmodel.cn"
        );
    }

    #[test]
    fn zhipu_quota_base_routes_z_ai_url_to_en_endpoint() {
        assert_eq!(
            zhipu_quota_base("https://api.z.ai/api/paas/v4"),
            "https://api.z.ai"
        );
    }

    #[test]
    fn zhipu_quota_base_defaults_to_en_for_unknown_url() {
        // 没有明显 Zhipu 域名特征时,默认走国际站(更通用的入口)
        assert_eq!(
            zhipu_quota_base("https://example.com/zhipu"),
            "https://api.z.ai"
        );
    }

    #[test]
    fn zhipu_quota_base_routes_uppercase_cn_url_to_cn_endpoint() {
        // 大小写不敏感:与 detect_provider 保持一致的约定,避免大写 preset URL 静默路由到国际站
        assert_eq!(
            zhipu_quota_base("HTTPS://OPEN.BIGMODEL.CN/api/paas/v4"),
            "https://open.bigmodel.cn"
        );
        assert_eq!(
            zhipu_quota_base("https://Open.BigModel.cn/api/paas/v4"),
            "https://open.bigmodel.cn"
        );
    }

    // ── 智谱团队套餐（Team Plan）──

    /// 起一个只服务一次连接的本地 HTTP server，捕获原始请求文本（请求行 + 头），
    /// 用于断言 team 查询发出的 URL query 与组织/项目请求头。`response=None` 时只捕获不回包。
    fn spawn_request_capturing_server(
        response: Option<String>,
    ) -> (
        String,
        std::sync::Arc<std::sync::Mutex<Option<String>>>,
        std::thread::JoinHandle<()>,
    ) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind local listener");
        let port = listener.local_addr().expect("local addr").port();
        let captured = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));
        let captured_clone = captured.clone();
        let handle = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf: Vec<u8> = Vec::new();
                let mut tmp = [0u8; 4096];
                // GET 无 body，读到 header 末尾（\r\n\r\n）即可
                while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
                    match stream.read(&mut tmp) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            buf.extend_from_slice(&tmp[..n]);
                            if buf.len() > 16 * 1024 {
                                break;
                            }
                        }
                    }
                }
                *captured_clone.lock().unwrap() = Some(String::from_utf8_lossy(&buf).into_owned());
                if let Some(resp) = response {
                    let _ = stream.write_all(resp.as_bytes());
                    let _ = stream.flush();
                }
            }
        });
        (format!("http://127.0.0.1:{port}/team"), captured, handle)
    }

    #[tokio::test]
    async fn zhipu_team_missing_creds_returns_not_found() {
        // 团队版必需 api_key + 组织 ID + 项目 ID，缺任一在触网前返回 NotFound（不联网）。
        let cases: [(&str, Option<&str>, Option<&str>); 3] = [
            ("key", Some("org"), None),        // 缺 project
            ("key", None, Some("proj")),       // 缺 organization
            ("  ", Some("org"), Some("proj")), // 空 api_key
        ];
        for (api_key, org, project) in cases {
            let q = get_coding_plan_quota(
                "https://open.bigmodel.cn/api/coding",
                api_key,
                Some("zhipu_team"),
                org,
                project,
            )
            .await
            .expect("凭据缺失是确定性失败，保持 Ok(success:false)");
            assert!(!q.success, "应失败: api_key={api_key:?}");
            assert!(
                matches!(q.credential_status, CredentialStatus::NotFound),
                "应为 NotFound: api_key={api_key:?}"
            );
        }

        // 标识大小写不敏感（eq_ignore_ascii_case），大写仍命中 team 分支并返回引导文案。
        let msg = get_coding_plan_quota(
            "https://open.bigmodel.cn/api/coding",
            "key",
            Some("Zhipu_Team"),
            None,
            None,
        )
        .await
        .expect("ok")
        .error
        .expect("应有错误文案");
        assert!(
            msg.contains("API key + organization ID + project ID"),
            "err={msg}"
        );
    }

    #[tokio::test]
    async fn zhipu_team_request_carries_type2_and_org_project_headers() {
        no_proxy_for_loopback();
        // 响应 shape 与个人版一致：两条 TOKENS_LIMIT（unit 3/6）→ five_hour + weekly。
        let body = serde_json::json!({
            "success": true,
            "data": {
                "level": "max",
                "limits": [
                    { "type": "TOKENS_LIMIT", "unit": 3, "number": 5, "percentage": 26.0 },
                    { "type": "TOKENS_LIMIT", "unit": 6, "number": 1, "percentage": 5.0 }
                ]
            }
        });
        let body_str = body.to_string();
        let resp = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body_str}",
            body_str.len()
        );
        let (base, captured, handle) = spawn_request_capturing_server(Some(resp));

        let quota = query_zhipu_team_at(&base, "team-key", "org-xxx", "proj_xxx")
            .await
            .expect("2xx + 合法 body 应成功");
        handle.join().expect("server thread");

        // 请求形状契约（与 token-monitor zaiTeamLimits 对齐）：
        // 同路径加 ?type=2 + bigmodel-organization / bigmodel-project 头 + 鉴权头。
        // reqwest/hyper 发头会小写化，故整体转小写做包含匹配。
        let raw = captured.lock().unwrap().clone().expect("应捕获到请求");
        let raw_lc = raw.to_lowercase();
        assert!(raw_lc.contains("/team?type=2"), "缺 ?type=2: {raw}");
        assert!(
            raw_lc.contains("bigmodel-organization: org-xxx"),
            "缺组织头: {raw}"
        );
        assert!(
            raw_lc.contains("bigmodel-project: proj_xxx"),
            "缺项目头: {raw}"
        );
        assert!(
            raw_lc.contains("authorization: team-key"),
            "缺鉴权头: {raw}"
        );

        // 解析复用个人版 zhipu_quota_from_body / parse_zhipu_token_tiers。
        assert!(quota.success);
        assert_eq!(quota.tiers.len(), 2);
        assert_eq!(quota.tiers[0].name, TIER_FIVE_HOUR);
        assert_eq!(quota.tiers[0].utilization, 26.0);
        assert_eq!(quota.tiers[1].name, TIER_WEEKLY_LIMIT);
        assert_eq!(quota.tiers[1].utilization, 5.0);
    }
}
