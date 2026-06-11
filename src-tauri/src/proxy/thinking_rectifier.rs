//! Thinking Signature 整流器
//!
//! 用于自动修复 Anthropic API 中因签名校验失败导致的请求错误。
//! 当上游 API 返回签名相关错误时，系统会自动移除有问题的签名字段并重试请求。

use super::types::RectifierConfig;
use serde_json::Value;

/// 整流结果
#[derive(Debug, Clone, Default)]
pub struct RectifyResult {
    /// 是否应用了整流
    pub applied: bool,
    /// 移除的 thinking block 数量
    pub removed_thinking_blocks: usize,
    /// 移除的 redacted_thinking block 数量
    pub removed_redacted_thinking_blocks: usize,
    /// 移除的 signature 字段数量
    pub removed_signature_fields: usize,
}

/// 检测是否需要触发 thinking 签名整流器
///
/// 返回 `true` 表示需要触发整流器，`false` 表示不需要。
/// 会检查配置开关。
pub fn should_rectify_thinking_signature(
    error_message: Option<&str>,
    config: &RectifierConfig,
) -> bool {
    // 检查总开关
    if !config.enabled {
        return false;
    }
    // 检查子开关
    if !config.request_thinking_signature {
        return false;
    }

    // 检测错误类型
    let Some(msg) = error_message else {
        return false;
    };
    let lower = msg.to_lowercase();

    // 场景1: thinking block 中的签名无效
    // 错误示例: "Invalid 'signature' in 'thinking' block"
    if lower.contains("invalid")
        && lower.contains("signature")
        && lower.contains("thinking")
        && lower.contains("block")
    {
        return true;
    }

    // 场景1b: Gemini/第三方渠道返回 "Thought signature is not valid"
    // 错误示例: "Unable to submit request because Thought signature is not valid"
    if lower.contains("thought signature")
        && (lower.contains("not valid") || lower.contains("invalid"))
    {
        return true;
    }

    // 场景2: assistant 消息必须以 thinking block 开头
    // 错误示例: "must start with a thinking block"
    if lower.contains("must start with a thinking block") {
        return true;
    }

    // 场景3: expected thinking or redacted_thinking, found tool_use
    // 与 CCH 对齐：要求明确包含 tool_use，避免过宽匹配。
    // 错误示例: "Expected `thinking` or `redacted_thinking`, but found `tool_use`"
    if lower.contains("expected")
        && (lower.contains("thinking") || lower.contains("redacted_thinking"))
        && lower.contains("found")
        && lower.contains("tool_use")
    {
        return true;
    }

    // 场景4: signature 字段必需但缺失
    // 错误示例: "signature: Field required"
    if lower.contains("signature") && lower.contains("field required") {
        return true;
    }

    // 场景5: signature 字段不被接受（第三方渠道）
    // 错误示例: "xxx.signature: Extra inputs are not permitted"
    if lower.contains("signature") && lower.contains("extra inputs are not permitted") {
        return true;
    }

    // 场景6: thinking/redacted_thinking 块被修改
    // 错误示例: "thinking or redacted_thinking blocks ... cannot be modified"
    if (lower.contains("thinking") || lower.contains("redacted_thinking"))
        && lower.contains("cannot be modified")
    {
        return true;
    }

    // 场景7（收窄，M8 修复）: "非法请求 / invalid request" 这类宽泛错误，
    // 仅当**同时**提及 thinking / signature 上下文时才兜底触发。
    //
    // 此前任意 "invalid request" 子串都会触发破坏性的 thinking/signature 剥离
    // + 整条重试。malformed JSON、无效 model、内容策略拒绝等与 thinking 无关的
    // 客户端错误会被误判，白白消耗一次上游（premium）配额并静默降级请求体。
    // 收窄后这些无关错误不再触发；而真正夹带 thinking/signature 关键字、但措辞
    // 未命中上面 1–6 精确模式的非法请求，仍可由本兜底分支捕获。
    let mentions_generic_invalid_request = lower.contains("非法请求")
        || lower.contains("illegal request")
        || lower.contains("invalid request");
    let mentions_thinking_or_signature = lower.contains("thinking") || lower.contains("signature");
    if mentions_generic_invalid_request && mentions_thinking_or_signature {
        return true;
    }

    false
}

/// 对 Anthropic 请求体做最小侵入整流
///
/// - 移除 messages[*].content 中的 thinking/redacted_thinking block
/// - 移除非 thinking block 上遗留的 signature 字段
/// - 特定条件下删除顶层 thinking 字段
///
/// 注意：该函数会原地修改 body 对象
pub fn rectify_anthropic_request(body: &mut Value) -> RectifyResult {
    let mut result = RectifyResult::default();

    let messages = match body.get_mut("messages").and_then(|m| m.as_array_mut()) {
        Some(m) => m,
        None => return result,
    };

    // 遍历所有消息
    for msg in messages.iter_mut() {
        let content = match msg.get_mut("content").and_then(|c| c.as_array_mut()) {
            Some(c) => c,
            None => continue,
        };

        let mut new_content = Vec::with_capacity(content.len());
        let mut content_modified = false;

        for block in content.iter() {
            let block_type = block.get("type").and_then(|t| t.as_str());

            match block_type {
                Some("thinking") => {
                    result.removed_thinking_blocks += 1;
                    content_modified = true;
                    continue;
                }
                Some("redacted_thinking") => {
                    result.removed_redacted_thinking_blocks += 1;
                    content_modified = true;
                    continue;
                }
                _ => {}
            }

            // 移除非 thinking block 上的 signature 字段
            if block.get("signature").is_some() {
                let mut block_clone = block.clone();
                if let Some(obj) = block_clone.as_object_mut() {
                    obj.remove("signature");
                    result.removed_signature_fields += 1;
                    content_modified = true;
                    new_content.push(Value::Object(obj.clone()));
                    continue;
                }
            }

            new_content.push(block.clone());
        }

        if content_modified {
            // M9：整流后不能把消息 content 留成空数组。Anthropic 会拒绝空 content，
            // OpenAI 兼容端点的转换器更会直接丢弃整条消息、破坏 user/assistant 交替；
            // 二者都会让"整流后重试"必定再次失败（甚至再触发整流形成无谓重试）。
            // 若该消息原本全是 thinking/redacted_thinking，剥完只剩空数组，补一个
            // 最小有效 text block 占位。占位文本须为**非空白**字符：本函数走错误
            // 恢复重试路径，整流后的 body 可能直接回 Anthropic 上游；若该消息恰为
            // 末条 assistant（prefill），纯空白 " " 会再触发
            // "final assistant content cannot end with trailing whitespace" 而把
            // 可恢复错误又变成 400。用 "." 既非空也无尾随空白。
            if new_content.is_empty() {
                new_content.push(serde_json::json!({"type": "text", "text": "."}));
            }
            result.applied = true;
            *content = new_content;
        }
    }

    // 兜底处理：thinking 启用 + 工具调用链路中最后一条 assistant 消息未以 thinking 开头
    let messages_snapshot: Vec<Value> = body
        .get("messages")
        .and_then(|m| m.as_array())
        .map(|a| a.to_vec())
        .unwrap_or_default();

    if should_remove_top_level_thinking(body, &messages_snapshot) {
        if let Some(obj) = body.as_object_mut() {
            obj.remove("thinking");
            result.applied = true;
        }
    }

    result
}

/// 判断是否需要删除顶层 thinking 字段
///
/// M10 评估（DESCOPE-with-evidence）：deep-read 建议"工具续写里合法启用的
/// thinking 不应被静默删除"。但本函数只在**错误恢复路径**里被调用
/// （`rectify_anthropic_request` 仅在上游已返回 thinking/signature 错误后才执行，
/// 见 `forwarder.rs`），且调用前已无条件剥掉了所有 thinking/redacted_thinking block。
///
/// 删除顶层 thinking 是整流器**核心用途**（跨渠道 thinking 签名不匹配恢复）所
/// 必需的：当 Claude Code 把"启用了 thinking 的工具续写"路由到会拒绝该签名的
/// 渠道时，整流器剥掉带坏签名的 thinking block 后，若仍保留 `thinking:enabled`，
/// 上游会再次以"assistant message must start with a thinking block"拒绝整条重试。
/// 因此**保留**顶层 thinking（M10 的字面修复）会把这一最常见的跨渠道可恢复错误
/// 变成硬失败——得不偿失。我们也无法伪造合法的 thinking 签名前缀来两全。
///
/// 取舍：在错误恢复路径上"成功但本回合关闭扩展思考" > "直接报错"，这是 failover
/// 代理的合理选择。这里以 `test_rectify_*_top_level_thinking*` 钉死该既定行为。
/// 后续（更大改造）可考虑解析错误里的 block 索引做**外科式**剥离，只移除被点名的
/// block，从而保住其余合法 thinking 前缀——超出本批次（行为批）安全范围。
fn should_remove_top_level_thinking(body: &Value, messages: &[Value]) -> bool {
    // 检查 thinking 是否启用
    let thinking_type = body
        .get("thinking")
        .and_then(|t| t.get("type"))
        .and_then(|t| t.as_str());

    // 与 CCH 对齐：仅 type=enabled 视为开启
    let thinking_enabled = thinking_type == Some("enabled");

    if !thinking_enabled {
        return false;
    }

    // 找到最后一条 assistant 消息
    let last_assistant = messages
        .iter()
        .rev()
        .find(|m| m.get("role").and_then(|r| r.as_str()) == Some("assistant"));

    let last_assistant_content = match last_assistant
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())
    {
        Some(c) if !c.is_empty() => c,
        _ => return false,
    };

    // 检查首块是否为 thinking/redacted_thinking
    let first_block_type = last_assistant_content
        .first()
        .and_then(|b| b.get("type"))
        .and_then(|t| t.as_str());

    let missing_thinking_prefix =
        first_block_type != Some("thinking") && first_block_type != Some("redacted_thinking");

    if !missing_thinking_prefix {
        return false;
    }

    // 检查是否存在 tool_use
    last_assistant_content
        .iter()
        .any(|b| b.get("type").and_then(|t| t.as_str()) == Some("tool_use"))
}

/// 与 CCH 对齐：请求前不做 thinking type 主动改写。
pub fn normalize_thinking_type(body: Value) -> Value {
    body
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
            request_media_fallback: true,
            request_media_heuristic: true,
        }
    }

    fn disabled_config() -> RectifierConfig {
        RectifierConfig {
            enabled: true,
            request_thinking_signature: false,
            request_thinking_budget: false,
            request_media_fallback: true,
            request_media_heuristic: true,
        }
    }

    fn master_disabled_config() -> RectifierConfig {
        RectifierConfig {
            enabled: false,
            request_thinking_signature: true,
            request_thinking_budget: true,
            request_media_fallback: true,
            request_media_heuristic: true,
        }
    }

    // ==================== should_rectify_thinking_signature 测试 ====================

    #[test]
    fn test_detect_invalid_signature() {
        assert!(should_rectify_thinking_signature(
            Some("messages.1.content.0: Invalid `signature` in `thinking` block"),
            &enabled_config()
        ));
    }

    #[test]
    fn test_detect_invalid_signature_no_backticks() {
        assert!(should_rectify_thinking_signature(
            Some("Messages.1.Content.0: invalid signature in thinking block"),
            &enabled_config()
        ));
    }

    #[test]
    fn test_detect_invalid_thought_signature_message() {
        assert!(should_rectify_thinking_signature(
            Some(
                "Unable to submit request because Thought signature is not valid.. Learn more: https://example.com/help"
            ),
            &enabled_config()
        ));
    }

    #[test]
    fn test_detect_invalid_signature_nested_json() {
        // 测试嵌套 JSON 格式的错误消息（第三方渠道常见格式）
        let nested_error = r#"{"error":{"message":"{\"type\":\"error\",\"error\":{\"type\":\"invalid_request_error\",\"message\":\"***.content.0: Invalid `signature` in `thinking` block\"},\"request_id\":\"req_xxx\"}"}}"#;
        assert!(should_rectify_thinking_signature(
            Some(nested_error),
            &enabled_config()
        ));
    }

    #[test]
    fn test_detect_invalid_thought_signature_nested_json() {
        let nested_error = r#"{"error":{"message":"Unable to submit request because Thought signature is not valid.. Learn more: https://example.com/help","type":"upstream_error","param":"","code":400}}"#;
        assert!(should_rectify_thinking_signature(
            Some(nested_error),
            &enabled_config()
        ));
    }

    #[test]
    fn test_detect_thinking_expected() {
        assert!(should_rectify_thinking_signature(
            Some("messages.69.content.0.type: Expected `thinking` or `redacted_thinking`, but found `tool_use`."),
            &enabled_config()
        ));
    }

    #[test]
    fn test_no_detect_thinking_expected_without_tool_use() {
        assert!(!should_rectify_thinking_signature(
            Some("messages.69.content.0.type: Expected `thinking` or `redacted_thinking`, but found `text`."),
            &enabled_config()
        ));
    }

    #[test]
    fn test_detect_must_start_with_thinking() {
        assert!(should_rectify_thinking_signature(
            Some("a final `assistant` message must start with a thinking block"),
            &enabled_config()
        ));
    }

    #[test]
    fn test_no_trigger_for_unrelated_error() {
        assert!(!should_rectify_thinking_signature(
            Some("Request timeout"),
            &enabled_config()
        ));
        assert!(!should_rectify_thinking_signature(
            Some("Connection refused"),
            &enabled_config()
        ));
        assert!(!should_rectify_thinking_signature(None, &enabled_config()));
    }

    #[test]
    fn test_detect_signature_field_required() {
        // 场景4: signature 字段缺失
        assert!(should_rectify_thinking_signature(
            Some("***.***.***.***.***.signature: Field required"),
            &enabled_config()
        ));
        // 嵌套 JSON 格式
        let nested_error = r#"{"error":{"type":"<nil>","message":"{\"type\":\"error\",\"error\":{\"type\":\"invalid_request_error\",\"message\":\"***.***.***.***.***.signature: Field required\"},\"request_id\":\"req_xxx\"}"}}"#;
        assert!(should_rectify_thinking_signature(
            Some(nested_error),
            &enabled_config()
        ));
    }

    #[test]
    fn test_disabled_config() {
        // 即使错误匹配，配置关闭时也不触发
        assert!(!should_rectify_thinking_signature(
            Some("Invalid `signature` in `thinking` block"),
            &disabled_config()
        ));
    }

    #[test]
    fn test_master_disabled() {
        // 总开关关闭时，即使子开关开启也不触发
        assert!(!should_rectify_thinking_signature(
            Some("Invalid `signature` in `thinking` block"),
            &master_disabled_config()
        ));
    }

    // ==================== rectify_anthropic_request 测试 ====================

    #[test]
    fn test_rectify_removes_thinking_blocks() {
        let mut body = json!({
            "model": "claude-test",
            "messages": [{
                "role": "assistant",
                "content": [
                    { "type": "thinking", "thinking": "t", "signature": "sig" },
                    { "type": "text", "text": "hello", "signature": "sig_text" },
                    { "type": "tool_use", "id": "toolu_1", "name": "WebSearch", "input": {}, "signature": "sig_tool" },
                    { "type": "redacted_thinking", "data": "r", "signature": "sig_redacted" }
                ]
            }]
        });

        let result = rectify_anthropic_request(&mut body);

        assert!(result.applied);
        assert_eq!(result.removed_thinking_blocks, 1);
        assert_eq!(result.removed_redacted_thinking_blocks, 1);
        assert_eq!(result.removed_signature_fields, 2);

        let content = body["messages"][0]["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        assert_eq!(content[0]["type"], "text");
        assert!(content[0].get("signature").is_none());
        assert_eq!(content[1]["type"], "tool_use");
        assert!(content[1].get("signature").is_none());
    }

    #[test]
    fn test_rectify_removes_top_level_thinking() {
        let mut body = json!({
            "model": "claude-test",
            "thinking": { "type": "enabled", "budget_tokens": 1024 },
            "messages": [{
                "role": "assistant",
                "content": [
                    { "type": "tool_use", "id": "toolu_1", "name": "WebSearch", "input": {} }
                ]
            }, {
                "role": "user",
                "content": [{ "type": "tool_result", "tool_use_id": "toolu_1", "content": "ok" }]
            }]
        });

        let result = rectify_anthropic_request(&mut body);

        assert!(result.applied);
        assert!(body.get("thinking").is_none());
    }

    #[test]
    fn test_rectify_no_change_when_no_issues() {
        let mut body = json!({
            "model": "claude-test",
            "messages": [{
                "role": "user",
                "content": [{ "type": "text", "text": "hello" }]
            }]
        });

        let result = rectify_anthropic_request(&mut body);

        assert!(!result.applied);
        assert_eq!(result.removed_thinking_blocks, 0);
    }

    #[test]
    fn test_rectify_no_messages() {
        let mut body = json!({ "model": "claude-test" });
        let result = rectify_anthropic_request(&mut body);
        assert!(!result.applied);
    }

    #[test]
    fn test_rectify_cross_provider_signature_recovery_removes_top_level_thinking() {
        // M10 钉死（DESCOPE-with-evidence）：跨渠道 thinking 签名不匹配的恢复路径上，
        // 剥掉带（坏）签名的 thinking block 后，必须同时删除顶层 thinking，
        // 否则上游会以 "must start with a thinking block" 再次拒绝整条重试。
        // 这是整流器的核心用途，保留顶层 thinking 会把可恢复错误变成硬失败。
        let mut body = json!({
            "model": "claude-test",
            "thinking": { "type": "enabled" },
            "messages": [{
                "role": "assistant",
                "content": [
                    { "type": "thinking", "thinking": "some thought", "signature": "cross-provider-sig" },
                    { "type": "tool_use", "id": "toolu_1", "name": "Test", "input": {} }
                ]
            }]
        });

        let result = rectify_anthropic_request(&mut body);

        assert!(result.applied);
        assert_eq!(result.removed_thinking_blocks, 1);
        // 顶层 thinking 被删除 —— 这是预期的恢复行为（见 should_remove_top_level_thinking 文档）
        assert!(body.get("thinking").is_none());
    }

    #[test]
    fn test_rectify_never_leaves_empty_content_array() {
        // M9：仅含 thinking 的 assistant 消息整流后不能留下 content:[]，
        // 否则重试必定再次失败（空 content 被上游拒绝/被转换器丢弃）。
        let mut body = json!({
            "model": "claude-test",
            "messages": [{
                "role": "assistant",
                "content": [
                    { "type": "thinking", "thinking": "solo", "signature": "s" }
                ]
            }]
        });

        let result = rectify_anthropic_request(&mut body);

        assert!(result.applied);
        let content = body["messages"][0]["content"].as_array().unwrap();
        assert_eq!(content.len(), 1, "不应留下空 content 数组");
        assert_eq!(content[0]["type"], "text");
    }

    #[test]
    fn test_rectify_does_not_touch_originally_empty_content() {
        // 原本就是空数组的消息不被整流（content_modified=false），不补占位
        let mut body = json!({
            "model": "claude-test",
            "messages": [{ "role": "assistant", "content": [] }]
        });

        let result = rectify_anthropic_request(&mut body);

        assert!(!result.applied);
        assert_eq!(body["messages"][0]["content"].as_array().unwrap().len(), 0);
    }

    // ==================== 新增错误场景检测测试 ====================

    #[test]
    fn test_detect_signature_extra_inputs() {
        // 场景5: signature 字段不被接受
        assert!(should_rectify_thinking_signature(
            Some("xxx.signature: Extra inputs are not permitted"),
            &enabled_config()
        ));
    }

    #[test]
    fn test_detect_thinking_cannot_be_modified() {
        // 场景6: thinking blocks cannot be modified
        assert!(should_rectify_thinking_signature(
            Some("thinking or redacted_thinking blocks in the response cannot be modified"),
            &enabled_config()
        ));
    }

    #[test]
    fn test_detect_invalid_request_with_thinking_context() {
        // 场景7（收窄）: 夹带 thinking/signature 上下文的非法请求仍兜底触发
        assert!(should_rectify_thinking_signature(
            Some("非法请求：thinking signature 不合法"),
            &enabled_config()
        ));
        assert!(should_rectify_thinking_signature(
            Some("invalid request: thinking signature mismatch"),
            &enabled_config()
        ));
    }

    #[test]
    fn test_no_detect_generic_invalid_request_without_thinking_context() {
        // M8 修复：与 thinking/signature 无关的"非法请求"不再触发破坏性整流+重试
        assert!(!should_rectify_thinking_signature(
            Some("invalid request: malformed JSON"),
            &enabled_config()
        ));
        assert!(!should_rectify_thinking_signature(
            Some("illegal request: tool_use block mismatch"),
            &enabled_config()
        ));
        assert!(!should_rectify_thinking_signature(
            Some("invalid_request_error: model `foo` not found"),
            &enabled_config()
        ));
        assert!(!should_rectify_thinking_signature(
            Some("非法请求：内容违反使用策略"),
            &enabled_config()
        ));
    }

    #[test]
    fn test_do_not_detect_thinking_type_tag_mismatch() {
        // 与 CCH 对齐：adaptive tag mismatch 不触发签名整流器
        assert!(!should_rectify_thinking_signature(
            Some("Input tag 'adaptive' found using 'type' does not match expected tags"),
            &enabled_config()
        ));
    }

    // ==================== adaptive thinking type 测试 ====================

    #[test]
    fn test_rectify_keeps_adaptive_when_no_legacy_blocks() {
        let mut body = json!({
            "model": "claude-test",
            "thinking": { "type": "adaptive" },
            "messages": [{
                "role": "user",
                "content": [{ "type": "text", "text": "hello" }]
            }]
        });

        let result = rectify_anthropic_request(&mut body);

        assert!(!result.applied);
        assert_eq!(body["thinking"]["type"], "adaptive");
        assert!(body["thinking"].get("budget_tokens").is_none());
    }

    #[test]
    fn test_rectify_adaptive_preserves_existing_budget_tokens() {
        let mut body = json!({
            "model": "claude-test",
            "thinking": { "type": "adaptive", "budget_tokens": 5000 },
            "messages": [{
                "role": "user",
                "content": [{ "type": "text", "text": "hello" }]
            }]
        });

        let result = rectify_anthropic_request(&mut body);

        assert!(!result.applied);
        assert_eq!(body["thinking"]["type"], "adaptive");
        assert_eq!(body["thinking"]["budget_tokens"], 5000);
    }

    #[test]
    fn test_rectify_does_not_change_enabled_type() {
        let mut body = json!({
            "model": "claude-test",
            "thinking": { "type": "enabled", "budget_tokens": 1024 },
            "messages": [{
                "role": "user",
                "content": [{ "type": "text", "text": "hello" }]
            }]
        });

        let result = rectify_anthropic_request(&mut body);

        assert!(!result.applied);
        assert_eq!(body["thinking"]["type"], "enabled");
    }

    #[test]
    fn test_rectify_removes_top_level_thinking_adaptive() {
        // 顶层 thinking 仅在 type=enabled 且 tool_use 场景才会删除，adaptive 不删除
        let mut body = json!({
            "model": "claude-test",
            "thinking": { "type": "adaptive" },
            "messages": [{
                "role": "assistant",
                "content": [
                    { "type": "tool_use", "id": "toolu_1", "name": "WebSearch", "input": {} }
                ]
            }, {
                "role": "user",
                "content": [{ "type": "tool_result", "tool_use_id": "toolu_1", "content": "ok" }]
            }]
        });

        let result = rectify_anthropic_request(&mut body);

        assert!(!result.applied);
        assert_eq!(body["thinking"]["type"], "adaptive");
    }

    #[test]
    fn test_rectify_adaptive_still_cleans_legacy_signature_blocks() {
        let mut body = json!({
            "model": "claude-test",
            "thinking": { "type": "adaptive" },
            "messages": [{
                "role": "assistant",
                "content": [
                    { "type": "thinking", "thinking": "t", "signature": "sig_thinking" },
                    { "type": "text", "text": "hello", "signature": "sig_text" }
                ]
            }]
        });

        let result = rectify_anthropic_request(&mut body);

        assert!(result.applied);
        assert_eq!(result.removed_thinking_blocks, 1);
        let content = body["messages"][0]["content"].as_array().unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["type"], "text");
        assert!(content[0].get("signature").is_none());
        assert_eq!(body["thinking"]["type"], "adaptive");
    }

    // ==================== normalize_thinking_type 测试 ====================

    #[test]
    fn test_normalize_thinking_type_adaptive_unchanged() {
        let body = json!({
            "model": "claude-test",
            "thinking": { "type": "adaptive" }
        });

        let result = normalize_thinking_type(body);

        assert_eq!(result["thinking"]["type"], "adaptive");
        assert!(result["thinking"].get("budget_tokens").is_none());
    }

    #[test]
    fn test_normalize_thinking_type_enabled_unchanged() {
        let body = json!({
            "model": "claude-test",
            "thinking": { "type": "enabled", "budget_tokens": 2048 }
        });

        let result = normalize_thinking_type(body);

        assert_eq!(result["thinking"]["type"], "enabled");
        assert_eq!(result["thinking"]["budget_tokens"], 2048);
    }

    #[test]
    fn test_normalize_thinking_type_disabled_unchanged() {
        let body = json!({
            "model": "claude-test",
            "thinking": { "type": "disabled" }
        });

        let result = normalize_thinking_type(body);

        assert_eq!(result["thinking"]["type"], "disabled");
    }

    #[test]
    fn test_normalize_thinking_type_preserves_budget() {
        let body = json!({
            "model": "claude-test",
            "thinking": { "type": "adaptive", "budget_tokens": 5000 }
        });

        let result = normalize_thinking_type(body);

        assert_eq!(result["thinking"]["type"], "adaptive");
        assert_eq!(result["thinking"]["budget_tokens"], 5000);
    }

    #[test]
    fn test_normalize_thinking_type_no_thinking() {
        let body = json!({
            "model": "claude-test"
        });

        let result = normalize_thinking_type(body);

        assert!(result.get("thinking").is_none());
    }

    #[test]
    fn test_normalize_thinking_type_unknown_unchanged() {
        let body = json!({
            "model": "claude-test",
            "thinking": { "type": "unexpected", "budget_tokens": 100 }
        });

        let result = normalize_thinking_type(body);

        assert_eq!(result["thinking"]["type"], "unexpected");
        assert_eq!(result["thinking"]["budget_tokens"], 100);
    }
}
