//! 模型映射模块
//!
//! 在请求转发前，根据 Provider 配置替换请求中的模型名称

use crate::provider::Provider;
use serde_json::Value;

pub(crate) const ONE_M_CONTEXT_MARKER: &str = "[1m]";

/// 模型映射配置
pub struct ModelMapping {
    pub haiku_model: Option<String>,
    pub sonnet_model: Option<String>,
    pub opus_model: Option<String>,
    pub default_model: Option<String>,
}

impl ModelMapping {
    /// 从 Provider 配置中提取模型映射
    pub fn from_provider(provider: &Provider) -> Self {
        let env = provider.settings_config.get("env");

        Self {
            haiku_model: env
                .and_then(|e| e.get("ANTHROPIC_DEFAULT_HAIKU_MODEL"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(String::from),
            sonnet_model: env
                .and_then(|e| e.get("ANTHROPIC_DEFAULT_SONNET_MODEL"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(String::from),
            opus_model: env
                .and_then(|e| e.get("ANTHROPIC_DEFAULT_OPUS_MODEL"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(String::from),
            default_model: env
                .and_then(|e| e.get("ANTHROPIC_MODEL"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(String::from),
        }
    }

    /// 检查是否配置了任何模型映射
    pub fn has_mapping(&self) -> bool {
        self.haiku_model.is_some()
            || self.sonnet_model.is_some()
            || self.opus_model.is_some()
            || self.default_model.is_some()
    }

    /// 根据原始模型名称获取映射后的模型
    ///
    /// 按模型档位（haiku / opus / sonnet）匹配对应映射目标。匹配使用
    /// **词边界** 检测而非裸 `contains()`：早期实现用 `contains("opus")` 会把
    /// `claude-octopus-*` 之类名字里的 `opus` 子串误判成 opus 档（L28）。这里要求
    /// 档位标记两侧是字符串边界或非字母数字分隔符（连字符、点、空格等），既消除
    /// 子串误命中，又保留所有规范 Claude 命名（`claude-3-5-haiku-*`、
    /// `claude-opus-4-5`、`claude-sonnet-4-5-*`，标记两侧均为 `-`）。
    ///
    /// 档位优先级 haiku → opus → sonnet 保持稳定且确定：规范 Claude 名称至多含一个
    /// 档位标记，仅在人为构造的多档位名称下才会触发优先级，固定顺序可复现。
    pub fn map_model(&self, original_model: &str) -> String {
        let model_lower = original_model.to_lowercase();

        // 1. 按模型类型匹配（词边界，避免子串误命中如 octopus→opus）
        if contains_model_tier(&model_lower, "haiku") {
            if let Some(ref m) = self.haiku_model {
                return m.clone();
            }
        }
        if contains_model_tier(&model_lower, "opus") {
            if let Some(ref m) = self.opus_model {
                return m.clone();
            }
        }
        if contains_model_tier(&model_lower, "sonnet") {
            if let Some(ref m) = self.sonnet_model {
                return m.clone();
            }
        }

        // 2. 默认模型
        if let Some(ref m) = self.default_model {
            return m.clone();
        }

        // 3. 无映射，保持原样
        original_model.to_string()
    }
}

/// 判断 `needle`（小写 ASCII 档位标记）是否以 **整词** 形式出现在 `haystack`
/// （已小写化的模型名）中，即左右两侧要么是字符串边界，要么是非字母数字字符。
///
/// 这样 `octopus` 不会命中 `opus`、`dissonant` 不会命中 `sonnet`，而规范的
/// `claude-opus-4-5` / `claude-3-5-haiku-20241022` / `claude-sonnet-4-5[1m]`
/// 仍能正确命中（档位标记被 `-` / `[` 等分隔符包裹）。
fn contains_model_tier(haystack: &str, needle: &str) -> bool {
    let bytes = haystack.as_bytes();
    let nlen = needle.len();
    if nlen == 0 || bytes.len() < nlen {
        return false;
    }

    let mut search_from = 0;
    while let Some(rel) = haystack[search_from..].find(needle) {
        let start = search_from + rel;
        let end = start + nlen;
        let before_boundary = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
        let after_boundary = end == bytes.len() || !bytes[end].is_ascii_alphanumeric();
        if before_boundary && after_boundary {
            return true;
        }
        // 继续向后找下一处可能的匹配。
        search_from = start + 1;
    }
    false
}

/// 对请求体应用模型映射
///
/// 返回 (映射后的请求体, 原始模型名, 映射后模型名)
pub fn apply_model_mapping(
    mut body: Value,
    provider: &Provider,
) -> (Value, Option<String>, Option<String>) {
    let mapping = ModelMapping::from_provider(provider);

    // 如果没有配置映射，直接返回
    if !mapping.has_mapping() {
        let original = body.get("model").and_then(|m| m.as_str()).map(String::from);
        return (body, original, None);
    }

    // 提取原始模型名
    let original_model = body.get("model").and_then(|m| m.as_str()).map(String::from);

    if let Some(ref original) = original_model {
        let mapped = mapping.map_model(original);

        if mapped != *original {
            log::debug!("[ModelMapper] 模型映射: {original} → {mapped}");
            body["model"] = serde_json::json!(mapped);
            return (body, Some(original.clone()), Some(mapped));
        }
    }

    (body, original_model, None)
}

/// Claude Code uses a `[1M]` suffix to declare 1M context support. Upstream
/// APIs generally do not accept this local capability marker, so strip it
/// before forwarding.
pub fn strip_one_m_suffix_for_upstream(model: &str) -> &str {
    let trimmed = model.trim_end();
    let marker = ONE_M_CONTEXT_MARKER.as_bytes();
    let bytes = trimmed.as_bytes();
    if bytes.len() >= marker.len()
        && bytes[bytes.len() - marker.len()..].eq_ignore_ascii_case(marker)
    {
        return trimmed[..trimmed.len() - marker.len()].trim_end();
    }
    model
}

pub fn strip_one_m_suffix_for_upstream_from_body(mut body: Value) -> Value {
    let Some(model) = body.get("model").and_then(Value::as_str) else {
        return body;
    };

    let stripped = strip_one_m_suffix_for_upstream(model);
    if stripped != model {
        log::debug!("[ModelMapper] 去除本地 1M 标记: {model} → {stripped}");
        body["model"] = serde_json::json!(stripped);
    }
    body
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_provider_with_mapping() -> Provider {
        Provider {
            id: "test".to_string(),
            name: "Test".to_string(),
            settings_config: json!({
                "env": {
                    "ANTHROPIC_MODEL": "default-model",
                    "ANTHROPIC_DEFAULT_HAIKU_MODEL": "haiku-mapped",
                    "ANTHROPIC_DEFAULT_SONNET_MODEL": "sonnet-mapped",
                    "ANTHROPIC_DEFAULT_OPUS_MODEL": "opus-mapped"
                }
            }),
            website_url: None,
            category: None,
            created_at: None,
            sort_index: None,
            notes: None,
            meta: None,
            icon: None,
            icon_color: None,
            in_failover_queue: false,
        }
    }

    fn create_provider_without_mapping() -> Provider {
        Provider {
            id: "test".to_string(),
            name: "Test".to_string(),
            settings_config: json!({}),
            website_url: None,
            category: None,
            created_at: None,
            sort_index: None,
            notes: None,
            meta: None,
            icon: None,
            icon_color: None,
            in_failover_queue: false,
        }
    }

    #[test]
    fn test_sonnet_mapping() {
        let provider = create_provider_with_mapping();
        let body = json!({"model": "claude-sonnet-4-5-20250929"});
        let (result, original, mapped) = apply_model_mapping(body, &provider);
        assert_eq!(result["model"], "sonnet-mapped");
        assert_eq!(original, Some("claude-sonnet-4-5-20250929".to_string()));
        assert_eq!(mapped, Some("sonnet-mapped".to_string()));
    }

    #[test]
    fn test_haiku_mapping() {
        let provider = create_provider_with_mapping();
        let body = json!({"model": "claude-haiku-4-5"});
        let (result, _, mapped) = apply_model_mapping(body, &provider);
        assert_eq!(result["model"], "haiku-mapped");
        assert_eq!(mapped, Some("haiku-mapped".to_string()));
    }

    #[test]
    fn test_opus_mapping() {
        let provider = create_provider_with_mapping();
        let body = json!({"model": "claude-opus-4-5"});
        let (result, _, mapped) = apply_model_mapping(body, &provider);
        assert_eq!(result["model"], "opus-mapped");
        assert_eq!(mapped, Some("opus-mapped".to_string()));
    }

    #[test]
    fn test_thinking_does_not_affect_model_mapping() {
        // Issue #2081: thinking 参数不应影响模型映射
        let provider = create_provider_with_mapping();
        let body = json!({
            "model": "claude-sonnet-4-5",
            "thinking": {"type": "enabled"}
        });
        let (result, _, mapped) = apply_model_mapping(body, &provider);
        assert_eq!(result["model"], "sonnet-mapped");
        assert_eq!(mapped, Some("sonnet-mapped".to_string()));
    }

    #[test]
    fn test_thinking_adaptive_does_not_affect_model_mapping() {
        // Issue #2081: adaptive thinking 也不应影响模型映射
        let provider = create_provider_with_mapping();
        let body = json!({
            "model": "claude-sonnet-4-5",
            "thinking": {"type": "adaptive"}
        });
        let (result, _, mapped) = apply_model_mapping(body, &provider);
        assert_eq!(result["model"], "sonnet-mapped");
        assert_eq!(mapped, Some("sonnet-mapped".to_string()));
    }

    #[test]
    fn test_thinking_disabled() {
        let provider = create_provider_with_mapping();
        let body = json!({
            "model": "claude-sonnet-4-5",
            "thinking": {"type": "disabled"}
        });
        let (result, _, mapped) = apply_model_mapping(body, &provider);
        assert_eq!(result["model"], "sonnet-mapped");
        assert_eq!(mapped, Some("sonnet-mapped".to_string()));
    }

    #[test]
    fn test_unknown_model_uses_default() {
        let provider = create_provider_with_mapping();
        let body = json!({"model": "some-unknown-model"});
        let (result, _, mapped) = apply_model_mapping(body, &provider);
        assert_eq!(result["model"], "default-model");
        assert_eq!(mapped, Some("default-model".to_string()));
    }

    #[test]
    fn test_no_mapping_configured() {
        let provider = create_provider_without_mapping();
        let body = json!({"model": "claude-sonnet-4-5"});
        let (result, original, mapped) = apply_model_mapping(body, &provider);
        assert_eq!(result["model"], "claude-sonnet-4-5");
        assert_eq!(original, Some("claude-sonnet-4-5".to_string()));
        assert!(mapped.is_none());
    }

    #[test]
    fn test_case_insensitive() {
        let provider = create_provider_with_mapping();
        let body = json!({"model": "Claude-SONNET-4-5"});
        let (result, _, mapped) = apply_model_mapping(body, &provider);
        assert_eq!(result["model"], "sonnet-mapped");
        assert_eq!(mapped, Some("sonnet-mapped".to_string()));
    }

    #[test]
    fn strips_one_m_suffix_before_upstream() {
        let body = json!({"model": "deepseek-v4-pro[1M]"});
        let result = strip_one_m_suffix_for_upstream_from_body(body);
        assert_eq!(result["model"], "deepseek-v4-pro");
    }

    #[test]
    fn strips_one_m_suffix_after_mapping() {
        let mut provider = create_provider_with_mapping();
        provider.settings_config = json!({
            "env": {
                "ANTHROPIC_DEFAULT_SONNET_MODEL": "deepseek-v4-pro [1M]"
            }
        });

        let body = json!({"model": "claude-sonnet-4-6"});
        let (mapped, _, _) = apply_model_mapping(body, &provider);
        let result = strip_one_m_suffix_for_upstream_from_body(mapped);

        assert_eq!(result["model"], "deepseek-v4-pro");
    }

    #[test]
    fn keeps_model_without_one_m_suffix() {
        let body = json!({"model": "deepseek-v4-pro"});
        let result = strip_one_m_suffix_for_upstream_from_body(body);
        assert_eq!(result["model"], "deepseek-v4-pro");
    }

    // ── word-boundary tier matching tests (L28) ──

    #[test]
    fn contains_model_tier_matches_whole_token_only() {
        // Genuine tier tokens (delimited by separators / boundaries) match…
        assert!(contains_model_tier("claude-opus-4-5", "opus"));
        assert!(contains_model_tier("claude-3-5-haiku-20241022", "haiku"));
        assert!(contains_model_tier("claude-sonnet-4-5", "sonnet"));
        assert!(contains_model_tier("opus", "opus"));
        assert!(contains_model_tier("claude-sonnet-4-5[1m]", "sonnet"));

        // …but substrings embedded inside a larger word do NOT.
        assert!(!contains_model_tier("claude-octopus-exp", "opus"));
        assert!(!contains_model_tier("dissonant-model", "sonnet"));
        assert!(!contains_model_tier("myhaikumodel", "haiku")); // no boundary either side
    }

    #[test]
    fn octopus_does_not_mis_map_to_opus() {
        // Regression for L28: naive `contains("opus")` mapped any name with the
        // "opus" substring (e.g. "octopus") onto the opus target. With
        // word-boundary matching the tier no longer matches, so it falls back to
        // the configured default model instead of being silently routed to opus.
        let provider = create_provider_with_mapping();
        let body = json!({"model": "claude-octopus-experiment"});
        let (result, _, mapped) = apply_model_mapping(body, &provider);
        assert_eq!(result["model"], "default-model");
        assert_eq!(mapped, Some("default-model".to_string()));
    }

    #[test]
    fn octopus_without_default_passes_through_unmapped() {
        // Same regression, but with no default configured: the unmatched name
        // must pass through untouched rather than being coerced to opus.
        let mut provider = create_provider_with_mapping();
        provider.settings_config = json!({
            "env": {
                "ANTHROPIC_DEFAULT_OPUS_MODEL": "opus-mapped"
            }
        });
        let body = json!({"model": "claude-octopus-experiment"});
        let (result, original, mapped) = apply_model_mapping(body, &provider);
        assert_eq!(result["model"], "claude-octopus-experiment");
        assert_eq!(original, Some("claude-octopus-experiment".to_string()));
        assert!(mapped.is_none());
    }

    #[test]
    fn one_m_suffixed_model_still_maps_by_tier() {
        // The local `[1m]` capability marker must not block tier detection; the
        // mapping happens first, the marker is stripped for upstream afterwards.
        let provider = create_provider_with_mapping();
        let body = json!({"model": "claude-sonnet-4-5[1M]"});
        let (result, _, mapped) = apply_model_mapping(body, &provider);
        assert_eq!(result["model"], "sonnet-mapped");
        assert_eq!(mapped, Some("sonnet-mapped".to_string()));
    }

    #[test]
    fn all_canonical_tier_names_still_map() {
        // Pin every intended canonical mapping so the boundary rewrite can't
        // silently regress the contract.
        let provider = create_provider_with_mapping();
        let cases = [
            ("claude-3-5-haiku-20241022", "haiku-mapped"),
            ("claude-haiku-4-5", "haiku-mapped"),
            ("claude-opus-4-1-20250805", "opus-mapped"),
            ("claude-opus-4-5", "opus-mapped"),
            ("claude-sonnet-4-5-20250929", "sonnet-mapped"),
            ("claude-3-7-sonnet-latest", "sonnet-mapped"),
            ("Claude-SONNET-4-5", "sonnet-mapped"),
        ];
        for (input, expected) in cases {
            let body = json!({ "model": input });
            let (result, _, _) = apply_model_mapping(body, &provider);
            assert_eq!(result["model"], expected, "model `{input}` mis-mapped");
        }
    }
}
