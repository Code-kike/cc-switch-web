//! Codex (OpenAI) Provider Adapter
//!
//! 仅透传模式，支持直连 OpenAI API
//!
//! ## 客户端检测
//! 支持检测官方 Codex 客户端 (codex_vscode, codex_cli_rs)

use super::{AuthInfo, AuthStrategy, ProviderAdapter};
use crate::codex_config;
use crate::provider::Provider;
use crate::proxy::error::ProxyError;
use regex::Regex;
use std::sync::LazyLock;

/// 官方 Codex 客户端 User-Agent 正则
#[allow(dead_code)]
static CODEX_CLIENT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(codex_vscode|codex_cli_rs)/[\d.]+").unwrap());

/// Codex 适配器
pub struct CodexAdapter;

/// Resolve the real upstream model selected by a Grok Build config.toml.
pub fn grok_provider_upstream_model(provider: &Provider) -> Option<String> {
    provider
        .settings_config
        .get("config")
        .and_then(|value| value.as_str())
        .and_then(crate::grok_config::extract_model_config)
        .map(|model| model.model)
}

/// Replace the Grok client-visible profile with the configured upstream model.
pub fn apply_grok_upstream_model(
    provider: &Provider,
    body: &mut serde_json::Value,
) -> Option<String> {
    let upstream_model = grok_provider_upstream_model(provider)?;
    body["model"] = serde_json::Value::String(upstream_model.clone());
    Some(upstream_model)
}

/// Whether a native Responses Codex upstream needs Codex namespace/plugin
/// tools flattened before forwarding.
///
/// xAI's strict Responses schema rejects Codex's ChatGPT-private
/// `{"type":"namespace", ...}` tool declarations. Chat/Anthropic bridges do
/// not use this native passthrough path, so the rewrite is limited to managed
/// xAI OAuth providers.
pub fn provider_needs_responses_namespace_flatten(provider: &Provider) -> bool {
    provider.is_xai_oauth()
}

impl CodexAdapter {
    pub fn new() -> Self {
        Self
    }

    /// 检测是否为官方 Codex 客户端
    ///
    /// 匹配 User-Agent 模式: `^(codex_vscode|codex_cli_rs)/[\d.]+`
    #[allow(dead_code)]
    pub fn is_official_client(user_agent: &str) -> bool {
        CODEX_CLIENT_REGEX.is_match(user_agent)
    }

    /// 从 Provider 配置中提取 API Key
    fn extract_key(&self, provider: &Provider) -> Option<String> {
        // 1. 尝试从 env 中获取
        if let Some(env) = provider.settings_config.get("env") {
            if let Some(key) = env
                .get("OPENAI_API_KEY")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|key| !key.is_empty())
            {
                return Some(key.to_string());
            }
        }

        // 2. 尝试从 auth 中获取 (Codex CLI 格式)
        if let Some(auth) = provider.settings_config.get("auth") {
            if let Some(key) = codex_config::extract_codex_auth_api_key(auth) {
                return Some(key.to_string());
            }
        }

        // 3. 尝试直接获取
        if let Some(key) = provider
            .settings_config
            .get("apiKey")
            .or_else(|| provider.settings_config.get("api_key"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|key| !key.is_empty())
        {
            return Some(key.to_string());
        }

        // 4. 尝试从 config 对象中获取
        if let Some(config) = provider.settings_config.get("config") {
            if let Some(key) = config
                .get("api_key")
                .or_else(|| config.get("apiKey"))
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|key| !key.is_empty())
            {
                return Some(key.to_string());
            }

            if let Some(config_str) = config.as_str() {
                if let Some((_, key)) = crate::grok_config::extract_credentials(config_str) {
                    return Some(key);
                }
                if let Some(key) = codex_config::extract_codex_experimental_bearer_token(config_str)
                {
                    return Some(key);
                }
            }
        }

        None
    }
}

impl Default for CodexAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderAdapter for CodexAdapter {
    fn name(&self) -> &'static str {
        "Codex"
    }

    fn extract_base_url(&self, provider: &Provider) -> Result<String, ProxyError> {
        // Managed Official ChatGPT cards: pin the ChatGPT backend origin so a
        // stored/empty config cannot relay the bound account token elsewhere.
        // Unbound native-login Official cards do not take this path — they have
        // no server-side credential and stay blocked at switch time.
        if provider.is_managed_codex_official_account_card() {
            return Ok(super::CHATGPT_CODEX_BASE_URL.to_string());
        }

        // Managed xAI OAuth credentials are valid only for the pinned xAI
        // origin. Ignore editable config so a stored token cannot be relayed
        // to an arbitrary endpoint.
        if provider.is_xai_oauth() {
            return Ok(super::XAI_API_BASE_URL.to_string());
        }

        // 1. 尝试直接获取 base_url 字段
        if let Some(url) = provider
            .settings_config
            .get("base_url")
            .and_then(|v| v.as_str())
        {
            return Ok(url.trim_end_matches('/').to_string());
        }

        // 2. 尝试 baseURL
        if let Some(url) = provider
            .settings_config
            .get("baseURL")
            .and_then(|v| v.as_str())
        {
            return Ok(url.trim_end_matches('/').to_string());
        }

        // 3. 尝试从 config 对象中获取
        if let Some(config) = provider.settings_config.get("config") {
            if let Some(url) = config.get("base_url").and_then(|v| v.as_str()) {
                return Ok(url.trim_end_matches('/').to_string());
            }

            // 尝试解析 TOML 字符串格式
            if let Some(config_str) = config.as_str() {
                if let Some(url) = crate::grok_config::extract_base_url(config_str) {
                    return Ok(url.trim_end_matches('/').to_string());
                }
                if let Some(start) = config_str.find("base_url = \"") {
                    let rest = &config_str[start + 12..];
                    if let Some(end) = rest.find('"') {
                        return Ok(rest[..end].trim_end_matches('/').to_string());
                    }
                }
                if let Some(start) = config_str.find("base_url = '") {
                    let rest = &config_str[start + 12..];
                    if let Some(end) = rest.find('\'') {
                        return Ok(rest[..end].trim_end_matches('/').to_string());
                    }
                }
            }
        }

        Err(ProxyError::ConfigError(
            "Codex Provider 缺少 base_url 配置".to_string(),
        ))
    }

    fn extract_auth(&self, provider: &Provider) -> Option<AuthInfo> {
        // Managed Official ChatGPT cards: placeholder only. The forwarder
        // already resolves AuthStrategy::CodexOAuth via CodexOAuthManager and
        // injects chatgpt-account-id from the provider binding (same path as
        // the Claude-side codex_oauth preset). Do not copy ClaudeAdapter's
        // ChatGPT protocol here — originator/version live in the shared module.
        if provider.is_managed_codex_official_account_card() {
            return Some(AuthInfo::new(
                "codex_oauth_placeholder".to_string(),
                AuthStrategy::CodexOAuth,
            ));
        }

        // The real access token is resolved per request by the forwarder from
        // the managed xAI account. This placeholder only selects that path.
        if provider.is_xai_oauth() {
            return Some(AuthInfo::new(
                "xai_oauth_placeholder".to_string(),
                AuthStrategy::XaiOAuth,
            ));
        }

        self.extract_key(provider)
            .map(|key| AuthInfo::new(key, AuthStrategy::Bearer))
    }

    fn build_url(&self, base_url: &str, endpoint: &str) -> String {
        let base_trimmed = base_url.trim_end_matches('/');
        let endpoint_trimmed = endpoint.trim_start_matches('/');

        // ChatGPT backend: keep the client's path (e.g. /responses, /responses/compact,
        // /alpha/search) under the pinned origin. Do not force /responses the way
        // ClaudeAdapter does — that rewrite exists because Claude clients speak
        // /v1/messages, which Codex clients never send.
        if base_trimmed == super::CHATGPT_CODEX_BASE_URL {
            return format!("{base_trimmed}/{endpoint_trimmed}");
        }

        // OpenAI/Codex 的 base_url 可能是：
        // - 纯 origin: https://api.openai.com  (需要自动补 /v1)
        // - 已含 /v1: https://api.openai.com/v1 (直接拼接)
        // - 自定义前缀: https://xxx/openai (不添加 /v1，直接拼接)

        // 检查 base_url 是否已经包含 /v1
        let already_has_v1 = base_trimmed.ends_with("/v1");

        // 检查是否是纯 origin（没有路径部分）
        let origin_only = match base_trimmed.split_once("://") {
            Some((_scheme, rest)) => !rest.contains('/'),
            None => !base_trimmed.contains('/'),
        };

        let mut url = if already_has_v1 {
            // 已经有 /v1，直接拼接
            format!("{base_trimmed}/{endpoint_trimmed}")
        } else if origin_only {
            // 纯 origin，添加 /v1
            format!("{base_trimmed}/v1/{endpoint_trimmed}")
        } else {
            // 自定义前缀，不添加 /v1，直接拼接
            format!("{base_trimmed}/{endpoint_trimmed}")
        };

        // 去除重复的 /v1/v1（可能由 base_url 与 endpoint 都带版本导致）
        while url.contains("/v1/v1") {
            url = url.replace("/v1/v1", "/v1");
        }

        url
    }

    fn get_auth_headers(
        &self,
        auth: &AuthInfo,
    ) -> Result<Vec<(http::HeaderName, http::HeaderValue)>, ProxyError> {
        use super::adapter::auth_header_value;
        use http::HeaderValue;
        let bearer = format!("Bearer {}", auth.api_key);
        if auth.strategy == AuthStrategy::CodexOAuth {
            // Bearer is overwritten by the forwarder with the live access_token.
            // originator+version must be sent as a pair (see CODEX_OAUTH_* docs).
            return Ok(vec![
                (
                    http::HeaderName::from_static("authorization"),
                    auth_header_value(&bearer)?,
                ),
                (
                    http::HeaderName::from_static("originator"),
                    HeaderValue::from_static(super::CODEX_OAUTH_ORIGINATOR),
                ),
                (
                    http::HeaderName::from_static("version"),
                    HeaderValue::from_static(super::CODEX_OAUTH_CLIENT_VERSION),
                ),
            ]);
        }
        Ok(vec![(
            http::HeaderName::from_static("authorization"),
            auth_header_value(&bearer)?,
        )])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_provider(config: serde_json::Value) -> Provider {
        Provider {
            id: "test".to_string(),
            name: "Test Codex".to_string(),
            settings_config: config,
            website_url: None,
            category: Some("codex".to_string()),
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
    fn test_extract_base_url_direct() {
        let adapter = CodexAdapter::new();
        let provider = create_provider(json!({
            "base_url": "https://api.openai.com/v1"
        }));

        let url = adapter.extract_base_url(&provider).unwrap();
        assert_eq!(url, "https://api.openai.com/v1");
    }

    #[test]
    fn grok_build_toml_exposes_upstream_credentials_and_model() {
        let adapter = CodexAdapter::new();
        let provider = create_provider(json!({
            "config": r#"
[models]
default = "grok-4.5"

[model."grok-4.5"]
model = "upstream-grok-model"
base_url = "https://relay.example.com/v1/"
name = "Example Relay"
api_key = "grok-secret"
api_backend = "responses"
context_window = 500000
"#
        }));

        assert_eq!(
            adapter.extract_base_url(&provider).unwrap(),
            "https://relay.example.com/v1"
        );
        let auth = adapter.extract_auth(&provider).unwrap();
        assert_eq!(auth.api_key, "grok-secret");
        assert_eq!(auth.strategy, AuthStrategy::Bearer);
        assert_eq!(
            grok_provider_upstream_model(&provider).as_deref(),
            Some("upstream-grok-model")
        );

        let mut body = json!({ "model": "grok-4.5", "input": "hello" });
        assert_eq!(
            apply_grok_upstream_model(&provider, &mut body).as_deref(),
            Some("upstream-grok-model")
        );
        assert_eq!(body["model"], "upstream-grok-model");
    }

    #[test]
    fn xai_oauth_pins_base_url_and_managed_auth_placeholder() {
        let adapter = CodexAdapter::new();
        let mut provider = create_provider(json!({
            "auth": { "OPENAI_API_KEY": "user-edited" },
            "config": r#"
model = "grok-4.5"

[model_providers.custom]
base_url = "https://attacker.example/v1"
wire_api = "responses"
"#
        }));
        provider.meta = Some(crate::provider::ProviderMeta {
            provider_type: Some("xai_oauth".to_string()),
            ..Default::default()
        });

        assert_eq!(
            adapter.extract_base_url(&provider).unwrap(),
            super::super::XAI_API_BASE_URL
        );
        let auth = adapter.extract_auth(&provider).unwrap();
        assert_eq!(auth.api_key, "xai_oauth_placeholder");
        assert_eq!(auth.strategy, AuthStrategy::XaiOAuth);
    }

    #[test]
    fn namespace_flatten_gate_only_fires_for_xai_oauth() {
        let mut xai = create_provider(json!({ "auth": {}, "config": "" }));
        xai.meta = Some(crate::provider::ProviderMeta {
            provider_type: Some("xai_oauth".to_string()),
            ..Default::default()
        });
        assert!(provider_needs_responses_namespace_flatten(&xai));

        let plain = create_provider(json!({
            "auth": { "OPENAI_API_KEY": "sk-x" },
            "config": "base_url = \"https://api.x.ai/v1\"\nwire_api = \"responses\""
        }));
        assert!(!provider_needs_responses_namespace_flatten(&plain));
    }

    #[test]
    fn test_extract_auth_from_auth_field() {
        let adapter = CodexAdapter::new();
        let provider = create_provider(json!({
            "auth": {
                "OPENAI_API_KEY": "sk-test-key-12345678"
            }
        }));

        let auth = adapter.extract_auth(&provider).unwrap();
        assert_eq!(auth.api_key, "sk-test-key-12345678");
        assert_eq!(auth.strategy, AuthStrategy::Bearer);
    }

    #[test]
    fn test_extract_auth_falls_back_to_config_bearer_when_auth_key_empty() {
        let adapter = CodexAdapter::new();
        let provider = create_provider(json!({
            "auth": {
                "OPENAI_API_KEY": ""
            },
            "config": r#"model_provider = "custom"

[model_providers.custom]
experimental_bearer_token = "sk-config-key"
"#
        }));

        let auth = adapter.extract_auth(&provider).unwrap();
        assert_eq!(auth.api_key, "sk-config-key");
        assert_eq!(auth.strategy, AuthStrategy::Bearer);
    }

    #[test]
    fn test_extract_auth_from_env() {
        let adapter = CodexAdapter::new();
        let provider = create_provider(json!({
            "env": {
                "OPENAI_API_KEY": "sk-env-key-12345678"
            }
        }));

        let auth = adapter.extract_auth(&provider).unwrap();
        assert_eq!(auth.api_key, "sk-env-key-12345678");
    }

    fn managed_official_card() -> Provider {
        let mut provider = create_provider(json!({ "auth": {}, "config": "" }));
        provider.category = Some("official".to_string());
        provider.meta = Some(crate::provider::ProviderMeta {
            auth_binding: Some(crate::provider::AuthBinding {
                source: crate::provider::AuthBindingSource::ManagedAccount,
                auth_provider: Some("codex_oauth".to_string()),
                account_id: Some("acct-managed".to_string()),
            }),
            ..Default::default()
        });
        provider
    }

    #[test]
    fn managed_official_card_pins_chatgpt_origin_and_codex_oauth_strategy() {
        let adapter = CodexAdapter::new();
        let provider = managed_official_card();
        assert_eq!(
            adapter.extract_base_url(&provider).expect("pinned origin"),
            super::super::CHATGPT_CODEX_BASE_URL,
        );
        let auth = adapter.extract_auth(&provider).expect("placeholder auth");
        assert_eq!(auth.strategy, AuthStrategy::CodexOAuth);
        assert_eq!(auth.api_key, "codex_oauth_placeholder");
        assert_eq!(
            adapter.build_url(super::super::CHATGPT_CODEX_BASE_URL, "/responses/compact"),
            "https://chatgpt.com/backend-api/codex/responses/compact",
        );
        let headers = adapter.get_auth_headers(&auth).expect("oauth headers");
        let names: Vec<_> = headers.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["authorization", "originator", "version"]);
    }

    #[test]
    fn unbound_official_card_has_no_server_side_credential() {
        let adapter = CodexAdapter::new();
        let mut provider = create_provider(json!({ "auth": {}, "config": "" }));
        provider.category = Some("official".to_string());
        provider.id = crate::database::CODEX_OFFICIAL_PROVIDER_ID.to_string();
        assert!(adapter.extract_base_url(&provider).is_err());
        assert!(adapter.extract_auth(&provider).is_none());
    }

    #[test]
    fn test_build_url() {
        let adapter = CodexAdapter::new();
        let url = adapter.build_url("https://api.openai.com/v1", "/responses");
        assert_eq!(url, "https://api.openai.com/v1/responses");
    }

    #[test]
    fn test_build_url_origin_adds_v1() {
        let adapter = CodexAdapter::new();
        let url = adapter.build_url("https://api.openai.com", "/responses");
        assert_eq!(url, "https://api.openai.com/v1/responses");
    }

    #[test]
    fn test_build_url_custom_prefix_no_v1() {
        let adapter = CodexAdapter::new();
        let url = adapter.build_url("https://example.com/openai", "/responses");
        assert_eq!(url, "https://example.com/openai/responses");
    }

    #[test]
    fn test_build_url_dedup_v1() {
        let adapter = CodexAdapter::new();
        // base_url 已包含 /v1，endpoint 也包含 /v1
        let url = adapter.build_url("https://www.packyapi.com/v1", "/v1/responses");
        assert_eq!(url, "https://www.packyapi.com/v1/responses");
    }

    // 官方客户端检测测试
    #[test]
    fn test_is_official_client_vscode() {
        assert!(CodexAdapter::is_official_client("codex_vscode/1.0.0"));
        assert!(CodexAdapter::is_official_client("codex_vscode/2.3.4"));
        assert!(CodexAdapter::is_official_client("codex_vscode/0.1"));
    }

    #[test]
    fn test_is_official_client_cli() {
        assert!(CodexAdapter::is_official_client("codex_cli_rs/1.0.0"));
        assert!(CodexAdapter::is_official_client("codex_cli_rs/0.5.2"));
    }

    #[test]
    fn test_is_not_official_client() {
        assert!(!CodexAdapter::is_official_client("Mozilla/5.0"));
        assert!(!CodexAdapter::is_official_client("curl/7.68.0"));
        assert!(!CodexAdapter::is_official_client("python-requests/2.25.1"));
        assert!(!CodexAdapter::is_official_client("codex_other/1.0.0"));
        assert!(!CodexAdapter::is_official_client(""));
    }

    #[test]
    fn test_is_official_client_partial_match() {
        // 必须从开头匹配
        assert!(!CodexAdapter::is_official_client("some codex_vscode/1.0.0"));
        assert!(!CodexAdapter::is_official_client(
            "prefix_codex_cli_rs/1.0.0"
        ));
    }
}
