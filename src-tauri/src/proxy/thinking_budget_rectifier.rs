//! Thinking Budget 整流器
//!
//! 用于自动修复 Anthropic API 中因 thinking budget 约束导致的请求错误。
//! 当上游 API 返回 budget_tokens 相关错误时，系统会自动调整 budget 参数并重试。

use super::types::RectifierConfig;
use serde_json::Value;

/// 未知模型回退用的默认 thinking budget tokens（历史值，保持既有行为安全）
const DEFAULT_THINKING_BUDGET: u64 = 32000;

/// 未知模型回退用的默认 max_tokens 值（历史值）
const DEFAULT_MAX_TOKENS: u64 = 64000;

/// 根据模型名推导一组合理的 `(budget_tokens, max_tokens)` 默认值。
///
/// L5 修复：此前硬编码 `budget=32000 / max_tokens=64000`，对最大输出上限只有
/// ~32k 的 Opus 4.x 而言 `max_tokens=64000` 会超过模型上限被上游拒绝，且
/// `budget=32000` 几乎吃满输出预算。这里按模型族粗分档，保证
/// `budget_tokens < max_tokens` 且二者都不超过该模型族的已知输出上限。
/// 未知模型回退到历史默认值 `(32000, 64000)`，与既有行为一致。
fn derive_thinking_budget_limits(model: Option<&str>) -> (u64, u64) {
    let model = model.unwrap_or_default().to_ascii_lowercase();
    if model.contains("opus") {
        // Opus 4.x 最大输出 ≈ 32k
        (16_000, 32_000)
    } else if model.contains("haiku") {
        // Haiku 最大输出 ≈ 8k（极少启用 thinking，仅作防御性默认）
        (4_096, 8_192)
    } else if model.contains("sonnet") {
        // Sonnet 4.x 最大输出 ≈ 64k（与历史默认一致）
        (32_000, 64_000)
    } else {
        (DEFAULT_THINKING_BUDGET, DEFAULT_MAX_TOKENS)
    }
}

/// Budget 整流结果
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BudgetRectifySnapshot {
    /// max_tokens
    pub max_tokens: Option<u64>,
    /// thinking.type
    pub thinking_type: Option<String>,
    /// thinking.budget_tokens
    pub thinking_budget_tokens: Option<u64>,
}

/// Budget 整流结果
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BudgetRectifyResult {
    /// 是否应用了整流
    pub applied: bool,
    /// 整流前快照
    pub before: BudgetRectifySnapshot,
    /// 整流后快照
    pub after: BudgetRectifySnapshot,
}

/// 检测是否需要触发 thinking budget 整流器
///
/// 检测条件：error message 同时包含 `budget_tokens` + `thinking` 相关约束
pub fn should_rectify_thinking_budget(
    error_message: Option<&str>,
    config: &RectifierConfig,
) -> bool {
    // 检查总开关
    if !config.enabled {
        return false;
    }
    // 检查子开关
    if !config.request_thinking_budget {
        return false;
    }

    let Some(msg) = error_message else {
        return false;
    };
    let lower = msg.to_lowercase();

    // 与 CCH 对齐：仅在包含 budget_tokens + thinking + 1024 约束时触发
    let has_budget_tokens_reference =
        lower.contains("budget_tokens") || lower.contains("budget tokens");
    let has_thinking_reference = lower.contains("thinking");
    let has_1024_constraint = lower.contains("greater than or equal to 1024")
        || lower.contains(">= 1024")
        || (lower.contains("1024") && lower.contains("input should be"));
    if has_budget_tokens_reference && has_thinking_reference && has_1024_constraint {
        return true;
    }

    false
}

/// 对请求体执行 budget 整流
///
/// 整流动作：
/// - `thinking.type = "enabled"`
/// - `thinking.budget_tokens = <按模型推导的预算>`
/// - 如果 `max_tokens <= budget_tokens`，设为 `<按模型推导的 max_tokens>`
pub fn rectify_thinking_budget(body: &mut Value) -> BudgetRectifyResult {
    let before = snapshot_budget(body);

    // 与 CCH 对齐：adaptive 请求不改写
    if before.thinking_type.as_deref() == Some("adaptive") {
        return BudgetRectifyResult {
            applied: false,
            before: before.clone(),
            after: before,
        };
    }

    // L5：按模型推导合理的 budget / max_tokens（必须在可变借用 body 之前求值）
    let (budget_tokens, max_tokens_target) =
        derive_thinking_budget_limits(body.get("model").and_then(|m| m.as_str()));

    // 与 CCH 对齐：缺少/非法 thinking 时自动创建后再整流
    if !body.get("thinking").is_some_and(Value::is_object) {
        body["thinking"] = Value::Object(serde_json::Map::new());
    }

    let Some(thinking) = body.get_mut("thinking").and_then(|t| t.as_object_mut()) else {
        return BudgetRectifyResult {
            applied: false,
            before: before.clone(),
            after: before,
        };
    };

    thinking.insert("type".to_string(), Value::String("enabled".to_string()));
    thinking.insert(
        "budget_tokens".to_string(),
        Value::Number(budget_tokens.into()),
    );

    // max_tokens 必须严格大于 budget_tokens；不足则提升到模型对应的目标值
    if before.max_tokens.is_none() || before.max_tokens <= Some(budget_tokens) {
        body["max_tokens"] = Value::Number(max_tokens_target.into());
    }

    let after = snapshot_budget(body);
    BudgetRectifyResult {
        applied: before != after,
        before,
        after,
    }
}

fn snapshot_budget(body: &Value) -> BudgetRectifySnapshot {
    let max_tokens = body.get("max_tokens").and_then(|v| v.as_u64());
    let thinking = body.get("thinking").and_then(|t| t.as_object());
    let thinking_type = thinking
        .and_then(|t| t.get("type"))
        .and_then(|v| v.as_str())
        .map(ToString::to_string);
    let thinking_budget_tokens = thinking
        .and_then(|t| t.get("budget_tokens"))
        .and_then(|v| v.as_u64());
    BudgetRectifySnapshot {
        max_tokens,
        thinking_type,
        thinking_budget_tokens,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn enabled_config() -> RectifierConfig {
        RectifierConfig {
            enabled: true,
            request_thinking_signature: true,
            request_thinking_budget: true,
        }
    }

    fn budget_disabled_config() -> RectifierConfig {
        RectifierConfig {
            enabled: true,
            request_thinking_signature: true,
            request_thinking_budget: false,
        }
    }

    fn master_disabled_config() -> RectifierConfig {
        RectifierConfig {
            enabled: false,
            request_thinking_signature: true,
            request_thinking_budget: true,
        }
    }

    // ==================== should_rectify_thinking_budget 测试 ====================

    #[test]
    fn test_detect_budget_tokens_thinking_error() {
        assert!(should_rectify_thinking_budget(
            Some("thinking.budget_tokens: Input should be greater than or equal to 1024"),
            &enabled_config()
        ));
    }

    #[test]
    fn test_detect_budget_tokens_max_tokens_error() {
        assert!(!should_rectify_thinking_budget(
            Some("budget_tokens must be less than max_tokens"),
            &enabled_config()
        ));
    }

    #[test]
    fn test_detect_budget_tokens_1024_error() {
        assert!(!should_rectify_thinking_budget(
            Some("budget_tokens: value must be at least 1024"),
            &enabled_config()
        ));
    }

    #[test]
    fn test_detect_budget_tokens_with_thinking_and_1024_error() {
        assert!(should_rectify_thinking_budget(
            Some("thinking budget_tokens must be >= 1024"),
            &enabled_config()
        ));
    }

    #[test]
    fn test_no_trigger_for_unrelated_error() {
        assert!(!should_rectify_thinking_budget(
            Some("Request timeout"),
            &enabled_config()
        ));
        assert!(!should_rectify_thinking_budget(None, &enabled_config()));
    }

    #[test]
    fn test_disabled_budget_config() {
        assert!(!should_rectify_thinking_budget(
            Some("thinking.budget_tokens: Input should be greater than or equal to 1024"),
            &budget_disabled_config()
        ));
    }

    #[test]
    fn test_master_disabled() {
        assert!(!should_rectify_thinking_budget(
            Some("thinking.budget_tokens: Input should be greater than or equal to 1024"),
            &master_disabled_config()
        ));
    }

    // ==================== rectify_thinking_budget 测试 ====================

    #[test]
    fn test_rectify_budget_basic() {
        let mut body = json!({
            "model": "claude-test",
            "thinking": { "type": "enabled", "budget_tokens": 512 },
            "max_tokens": 1024
        });

        let result = rectify_thinking_budget(&mut body);

        assert!(result.applied);
        assert_eq!(result.before.thinking_type.as_deref(), Some("enabled"));
        assert_eq!(result.after.thinking_type.as_deref(), Some("enabled"));
        assert_eq!(result.before.thinking_budget_tokens, Some(512));
        assert_eq!(
            result.after.thinking_budget_tokens,
            Some(DEFAULT_THINKING_BUDGET)
        );
        assert_eq!(result.before.max_tokens, Some(1024));
        assert_eq!(result.after.max_tokens, Some(DEFAULT_MAX_TOKENS));
        assert_eq!(body["thinking"]["type"], "enabled");
        assert_eq!(body["thinking"]["budget_tokens"], DEFAULT_THINKING_BUDGET);
        assert_eq!(body["max_tokens"], DEFAULT_MAX_TOKENS);
    }

    #[test]
    fn test_rectify_budget_skips_adaptive() {
        let mut body = json!({
            "model": "claude-test",
            "thinking": { "type": "adaptive", "budget_tokens": 512 },
            "max_tokens": 1024
        });

        let result = rectify_thinking_budget(&mut body);

        assert!(!result.applied);
        assert_eq!(result.before, result.after);
        assert_eq!(body["thinking"]["type"], "adaptive");
        assert_eq!(body["thinking"]["budget_tokens"], 512);
        assert_eq!(body["max_tokens"], 1024);
    }

    #[test]
    fn test_rectify_budget_preserves_large_max_tokens() {
        let mut body = json!({
            "model": "claude-test",
            "thinking": { "type": "enabled", "budget_tokens": 512 },
            "max_tokens": 100000
        });

        let result = rectify_thinking_budget(&mut body);

        assert!(result.applied);
        assert_eq!(result.before.max_tokens, Some(100000));
        assert_eq!(result.after.max_tokens, Some(100000));
        assert_eq!(body["max_tokens"], 100000);
    }

    #[test]
    fn test_rectify_budget_creates_thinking_object_when_missing() {
        let mut body = json!({
            "model": "claude-test",
            "max_tokens": 1024
        });

        let result = rectify_thinking_budget(&mut body);

        assert!(result.applied);
        assert_eq!(result.before.thinking_type, None);
        assert_eq!(result.after.thinking_type.as_deref(), Some("enabled"));
        assert_eq!(
            result.after.thinking_budget_tokens,
            Some(DEFAULT_THINKING_BUDGET)
        );
        assert_eq!(result.after.max_tokens, Some(DEFAULT_MAX_TOKENS));
        assert_eq!(body["thinking"]["type"], "enabled");
        assert_eq!(body["thinking"]["budget_tokens"], DEFAULT_THINKING_BUDGET);
        assert_eq!(body["max_tokens"], DEFAULT_MAX_TOKENS);
    }

    #[test]
    fn test_rectify_budget_no_max_tokens() {
        let mut body = json!({
            "model": "claude-test",
            "thinking": { "type": "enabled", "budget_tokens": 512 }
        });

        let result = rectify_thinking_budget(&mut body);

        assert!(result.applied);
        assert_eq!(result.before.max_tokens, None);
        assert_eq!(result.after.max_tokens, Some(DEFAULT_MAX_TOKENS));
        assert_eq!(body["max_tokens"], DEFAULT_MAX_TOKENS);
    }

    #[test]
    fn test_rectify_budget_normalizes_non_enabled_type() {
        let mut body = json!({
            "model": "claude-test",
            "thinking": { "type": "disabled", "budget_tokens": 512 },
            "max_tokens": 1024
        });

        let result = rectify_thinking_budget(&mut body);

        assert!(result.applied);
        assert_eq!(result.before.thinking_type.as_deref(), Some("disabled"));
        assert_eq!(result.after.thinking_type.as_deref(), Some("enabled"));
        assert_eq!(body["thinking"]["type"], "enabled");
        assert_eq!(body["thinking"]["budget_tokens"], DEFAULT_THINKING_BUDGET);
        assert_eq!(body["max_tokens"], DEFAULT_MAX_TOKENS);
    }

    #[test]
    fn test_rectify_budget_no_change_when_already_valid() {
        let mut body = json!({
            "model": "claude-test",
            "thinking": { "type": "enabled", "budget_tokens": 32000 },
            "max_tokens": 64001
        });

        let result = rectify_thinking_budget(&mut body);

        assert!(!result.applied);
        assert_eq!(result.before, result.after);
        assert_eq!(body["thinking"]["budget_tokens"], 32000);
        assert_eq!(body["max_tokens"], 64001);
    }

    // ==================== L5: 按模型推导 budget/max_tokens 测试 ====================

    #[test]
    fn test_derive_limits_per_model_family() {
        // Opus 输出上限 ≈ 32k → 预算与 max_tokens 都被压到模型上限以内
        assert_eq!(
            derive_thinking_budget_limits(Some("claude-opus-4-20250514")),
            (16_000, 32_000)
        );
        // Haiku 输出上限 ≈ 8k
        assert_eq!(
            derive_thinking_budget_limits(Some("claude-3-5-haiku-latest")),
            (4_096, 8_192)
        );
        // Sonnet 维持历史默认 (32k, 64k)
        assert_eq!(
            derive_thinking_budget_limits(Some("claude-sonnet-4-5")),
            (32_000, 64_000)
        );
        // 未知模型 / 缺省 → 历史默认值
        assert_eq!(
            derive_thinking_budget_limits(Some("some-unknown-model")),
            (DEFAULT_THINKING_BUDGET, DEFAULT_MAX_TOKENS)
        );
        assert_eq!(
            derive_thinking_budget_limits(None),
            (DEFAULT_THINKING_BUDGET, DEFAULT_MAX_TOKENS)
        );
    }

    #[test]
    fn test_rectify_budget_opus_respects_model_ceiling() {
        // 此前硬编码会把 Opus 的 max_tokens 拉到 64000（超过 ~32k 上限）。
        // 修复后应被压到模型上限以内：budget=16000, max_tokens=32000。
        let mut body = json!({
            "model": "claude-opus-4-1-20250805",
            "thinking": { "type": "enabled", "budget_tokens": 512 },
            "max_tokens": 1024
        });

        let result = rectify_thinking_budget(&mut body);

        assert!(result.applied);
        assert_eq!(body["thinking"]["budget_tokens"], 16_000);
        assert_eq!(body["max_tokens"], 32_000);
        // 不变量：budget_tokens < max_tokens
        assert!(
            body["thinking"]["budget_tokens"].as_u64().unwrap()
                < body["max_tokens"].as_u64().unwrap()
        );
    }

    #[test]
    fn test_rectify_budget_opus_preserves_large_user_max_tokens() {
        // 用户已给出足够大的 max_tokens（> budget）时不强制下调
        let mut body = json!({
            "model": "claude-opus-4-1-20250805",
            "thinking": { "type": "enabled", "budget_tokens": 512 },
            "max_tokens": 30000
        });

        let result = rectify_thinking_budget(&mut body);

        assert!(result.applied);
        assert_eq!(body["thinking"]["budget_tokens"], 16_000);
        // 30000 > 16000(budget) → 保留用户原值
        assert_eq!(body["max_tokens"], 30000);
    }

    #[test]
    fn test_rectify_budget_haiku_uses_small_ceiling() {
        let mut body = json!({
            "model": "claude-3-5-haiku-latest",
            "thinking": { "type": "enabled", "budget_tokens": 100 },
            "max_tokens": 512
        });

        let result = rectify_thinking_budget(&mut body);

        assert!(result.applied);
        assert_eq!(body["thinking"]["budget_tokens"], 4_096);
        assert_eq!(body["max_tokens"], 8_192);
    }
}
