//! 请求处理器
//!
//! 处理各种API端点的HTTP请求
//!
//! 重构后的结构：
//! - 通用逻辑提取到 `handler_context` 和 `response_processor` 模块
//! - 各 handler 只保留独特的业务逻辑
//! - Claude 的格式转换逻辑保留在此文件（用于 OpenRouter 旧接口回退）

use super::{
    content_encoding::{decompress_body, get_content_encoding, is_supported_content_encoding},
    error_mapper::{get_error_message, map_proxy_error_to_status},
    forwarder::ActiveConnectionGuard,
    handler_config::{
        CLAUDE_PARSER_CONFIG, CODEX_PARSER_CONFIG, GEMINI_PARSER_CONFIG, OPENAI_PARSER_CONFIG,
    },
    handler_context::RequestContext,
    providers::{
        get_adapter, get_claude_api_format,
        streaming::create_anthropic_sse_stream,
        streaming_gemini::create_anthropic_sse_stream_from_gemini,
        streaming_responses::{
            create_anthropic_sse_stream_from_responses,
            create_anthropic_sse_stream_from_responses_with_web_search_options,
        },
        transform, transform_gemini, transform_responses,
    },
    response_processor::{
        create_logged_passthrough_stream, create_usage_collector, process_response,
        read_decoded_body, spawn_log_usage, strip_entity_headers_for_rebuilt_body,
        strip_hop_by_hop_response_headers, usage_logging_enabled, SseUsageCollector,
    },
    server::ProxyState,
    sse::{strip_sse_field, take_sse_block},
    transform_codex_responses_namespace,
    types::*,
    usage::parser::TokenUsage,
    ProxyError,
};
use crate::app_config::AppType;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use bytes::Bytes;
use futures::StreamExt;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::collections::BTreeMap;

// ============================================================================
// 健康检查和状态查询（简单端点）
// ============================================================================

/// 健康检查
pub async fn health_check() -> (StatusCode, Json<Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "status": "healthy",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })),
    )
}

/// 获取服务状态
pub async fn get_status(State(state): State<ProxyState>) -> Result<Json<ProxyStatus>, ProxyError> {
    let status = state.status.read().await.clone();
    Ok(Json(status))
}

/// GET /v1/models — Codex model list (reachability check)
///
/// Codex CLI probes this endpoint at startup and deserializes the response as a
/// catalog with a top-level `models` field. Serve the cc-switch-managed catalog
/// only while the live config still references it; otherwise return an empty
/// catalog so stale generated files are not exposed after a user switches away.
pub async fn handle_models() -> Result<Json<Value>, ProxyError> {
    let catalog = match crate::codex_config::read_codex_model_catalog_simplified_from_live() {
        Ok(Some(catalog)) => catalog,
        Ok(None) => {
            log::debug!("[models] catalog not served; no active cc-switch model_catalog_json");
            json!({"models": []})
        }
        Err(error) => {
            log::debug!("[models] failed to read active catalog: {error}");
            json!({"models": []})
        }
    };
    Ok(Json(catalog))
}

// ============================================================================
// Claude API 处理器（包含格式转换逻辑）
// ============================================================================

/// 处理 /v1/messages 请求（Claude API）
///
/// Claude 处理器包含独特的格式转换逻辑：
/// - 过去用于 OpenRouter 的 OpenAI Chat Completions 兼容接口（Anthropic ↔ OpenAI 转换）
/// - 现在 OpenRouter 已推出 Claude Code 兼容接口，默认不再启用该转换（逻辑保留以备回退）
pub async fn handle_messages(
    State(state): State<ProxyState>,
    request: axum::extract::Request,
) -> Result<axum::response::Response, ProxyError> {
    let (parts, body) = request.into_parts();
    let method = parts.method.clone();
    let uri = parts.uri;
    let headers = parts.headers;
    let extensions = parts.extensions;
    let body_bytes = body
        .collect()
        .await
        .map_err(|e| ProxyError::Internal(format!("Failed to read request body: {e}")))?
        .to_bytes();
    let body: Value = serde_json::from_slice(&body_bytes)
        .map_err(|e| ProxyError::Internal(format!("Failed to parse request body: {e}")))?;

    let mut ctx =
        RequestContext::new(&state, &body, &headers, AppType::Claude, "Claude", "claude").await?;

    let endpoint = uri
        .path_and_query()
        .map(|path_and_query| path_and_query.as_str())
        .unwrap_or(uri.path());

    let is_stream = body
        .get("stream")
        .and_then(|s| s.as_bool())
        .unwrap_or(false);

    // 转发请求
    let forwarder = ctx.create_forwarder(&state);
    let mut result = match forwarder
        .forward_with_retry(
            &AppType::Claude,
            method,
            endpoint,
            body.clone(),
            headers,
            extensions,
            ctx.get_providers(),
            ctx.failover_enabled(),
        )
        .await
    {
        Ok(result) => result,
        Err(mut err) => {
            if let Some(provider) = err.provider.take() {
                ctx.provider = provider;
            }
            log_forward_error(&state, &ctx, is_stream, &err.error);
            return Err(err.error);
        }
    };

    let connection_guard = result.connection_guard.take();
    ctx.provider = result.provider;
    let api_format = result
        .claude_api_format
        .as_deref()
        .unwrap_or_else(|| get_claude_api_format(&ctx.provider))
        .to_string();
    let response = result.response;

    // 检查是否需要格式转换（OpenRouter 等中转服务）
    let adapter = get_adapter(&AppType::Claude);
    let needs_transform = adapter.needs_transform(&ctx.provider);

    // Claude 特有：格式转换处理
    if needs_transform {
        return handle_claude_transform(
            response,
            &ctx,
            &state,
            &body,
            is_stream,
            &api_format,
            connection_guard,
        )
        .await;
    }

    // 通用响应处理（透传模式）
    process_response(
        response,
        &ctx,
        &state,
        &CLAUDE_PARSER_CONFIG,
        connection_guard,
    )
    .await
}

/// Claude 格式转换处理（独有逻辑）
///
/// 支持 OpenAI Chat Completions 和 Responses API 两种格式的转换
struct ClaudeUsageLog {
    model: String,
    request_model: String,
    provider_id: String,
    usage: TokenUsage,
    latency_ms: u64,
    status_code: u16,
    is_streaming: bool,
}

fn prepare_claude_usage_log(
    ctx: &RequestContext,
    response: &Value,
    status_code: u16,
    is_streaming: bool,
) -> Option<ClaudeUsageLog> {
    let usage = TokenUsage::from_claude_response(response)?;

    // 转换后的响应缺失/合成空 model 时回退到客户端请求别名（上游此处还会先回退
    // 到 `ctx.outbound_model`，本 fork 的 RequestContext 没有该字段）。
    let model = response
        .get("model")
        .and_then(Value::as_str)
        .filter(|model| !model.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| "unknown".to_string());

    Some(ClaudeUsageLog {
        model,
        request_model: ctx.request_model.clone(),
        provider_id: ctx.provider.id.clone(),
        usage,
        latency_ms: ctx.latency_ms(),
        status_code,
        is_streaming,
    })
}

async fn write_claude_usage_log(state: &ProxyState, log: ClaudeUsageLog) {
    log_usage(
        state,
        &log.provider_id,
        "claude",
        &log.model,
        &log.request_model,
        log.usage,
        log.latency_ms,
        None,
        log.is_streaming,
        log.status_code,
    )
    .await;
}

fn spawn_claude_usage_log(
    state: &ProxyState,
    ctx: &RequestContext,
    response: &Value,
    status_code: u16,
    is_streaming: bool,
) {
    if !usage_logging_enabled(state) {
        return;
    }
    let Some(log) = prepare_claude_usage_log(ctx, response, status_code, is_streaming) else {
        return;
    };
    let state = state.clone();
    tokio::spawn(async move {
        write_claude_usage_log(&state, log).await;
    });
}

async fn handle_claude_transform(
    response: super::hyper_client::ProxyResponse,
    ctx: &RequestContext,
    state: &ProxyState,
    original_body: &Value,
    is_stream: bool,
    api_format: &str,
    connection_guard: Option<ActiveConnectionGuard>,
) -> Result<axum::response::Response, ProxyError> {
    let status = response.status();
    let is_codex_oauth = ctx
        .provider
        .meta
        .as_ref()
        .and_then(|meta| meta.provider_type.as_deref())
        == Some("codex_oauth");
    // Codex OAuth 会把 openai_responses 响应强制升级为 SSE，即使客户端发的是 stream:false。
    // should_use_claude_transform_streaming 默认会把这个组合路由到流式转换器——虽然能避免
    // JSON parse 报 422，但会让非流客户端收到 text/event-stream，违反 Anthropic 非流语义。
    // 这里为这个特定组合打开 override：把上游 SSE 聚合成 Anthropic JSON 回给客户端，其它
    // 场景（任意上游 is_sse、非 Codex OAuth 等）仍沿用原有流式兜底。
    let aggregate_codex_oauth_responses_sse =
        !is_stream && is_codex_oauth && api_format == "openai_responses";
    let use_streaming = if aggregate_codex_oauth_responses_sse {
        false
    } else {
        should_use_claude_transform_streaming(
            is_stream,
            response.is_sse(),
            api_format,
            is_codex_oauth,
        )
    };
    let tool_schema_hints = transform_gemini::extract_anthropic_tool_schema_hints(original_body);
    let tool_schema_hints = (!tool_schema_hints.is_empty()).then_some(tool_schema_hints);
    let hosted_web_search_name =
        transform_responses::anthropic_web_search_tool_name(original_body).map(ToString::to_string);
    let hosted_web_search_max_uses =
        transform_responses::anthropic_web_search_max_uses(original_body);

    if use_streaming {
        // 根据 api_format 选择流式转换器
        let stream = response.bytes_stream();
        let sse_stream: Box<
            dyn futures::Stream<Item = Result<Bytes, std::io::Error>> + Send + Unpin,
        > = if api_format == "openai_responses" {
            if hosted_web_search_name.is_none() && hosted_web_search_max_uses.is_none() {
                Box::new(Box::pin(create_anthropic_sse_stream_from_responses(stream)))
            } else {
                Box::new(Box::pin(
                    create_anthropic_sse_stream_from_responses_with_web_search_options(
                        stream,
                        hosted_web_search_name.clone(),
                        hosted_web_search_max_uses,
                    ),
                ))
            }
        } else if api_format == "gemini_native" {
            Box::new(Box::pin(create_anthropic_sse_stream_from_gemini(
                stream,
                Some(state.gemini_shadow.clone()),
                Some(ctx.provider.id.clone()),
                Some(ctx.session_id.clone()),
                tool_schema_hints.clone(),
            )))
        } else {
            Box::new(Box::pin(create_anthropic_sse_stream(stream)))
        };

        // 创建使用量收集器；关闭 usage logging 时不要再解析转换后的 SSE。
        let usage_collector = if usage_logging_enabled(state) {
            let state = state.clone();
            let provider_id = ctx.provider.id.clone();
            let model = ctx.request_model.clone();
            let status_code = status.as_u16();
            let start_time = ctx.start_time;

            Some(SseUsageCollector::new(
                start_time,
                CLAUDE_PARSER_CONFIG.stream_event_filter,
                move |events, first_token_ms| {
                    if let Some(usage) = TokenUsage::from_claude_stream_events(&events) {
                        let latency_ms = start_time.elapsed().as_millis() as u64;
                        let state = state.clone();
                        let provider_id = provider_id.clone();
                        let model = model.clone();

                        tokio::spawn(async move {
                            log_usage(
                                &state,
                                &provider_id,
                                "claude",
                                &model,
                                &model,
                                usage,
                                latency_ms,
                                first_token_ms,
                                true,
                                status_code,
                            )
                            .await;
                        });
                    } else {
                        log::debug!("[Claude] OpenRouter 流式响应缺少 usage 统计，跳过消费记录");
                    }
                },
            ))
        } else {
            None
        };

        // 获取流式超时配置
        let timeout_config = ctx.streaming_timeout_config();

        let logged_stream = create_logged_passthrough_stream(
            sse_stream,
            "Claude/OpenRouter",
            usage_collector,
            timeout_config,
            connection_guard,
        );

        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            "Content-Type",
            axum::http::HeaderValue::from_static("text/event-stream"),
        );
        headers.insert(
            "Cache-Control",
            axum::http::HeaderValue::from_static("no-cache"),
        );

        let body = axum::body::Body::from_stream(logged_stream);
        return Ok((headers, body).into_response());
    }

    // 非流式响应转换 (OpenAI/Responses → Anthropic)
    let body_timeout =
        if ctx.app_config.auto_failover_enabled && ctx.app_config.non_streaming_timeout > 0 {
            std::time::Duration::from_secs(ctx.app_config.non_streaming_timeout as u64)
        } else {
            std::time::Duration::ZERO
        };
    let enforce_codex_web_search_limit_while_aggregating =
        aggregate_codex_oauth_responses_sse && hosted_web_search_max_uses.is_some();
    let (mut response_headers, direct_anthropic_response, upstream_response) =
        if enforce_codex_web_search_limit_while_aggregating {
            if let Some(encoding) = get_content_encoding(response.headers()) {
                // Transformed requests advertise `accept-encoding: identity`.
                // If an upstream ignores that contract, fail closed rather than
                // buffering a compressed stream and losing early cancellation.
                return Err(ProxyError::TransformError(format!(
                    "Cannot enforce Anthropic WebSearch max_uses on a compressed Codex SSE response ({encoding})"
                )));
            }
            let response_headers = response.headers().clone();
            let message = responses_sse_stream_to_anthropic_message(
                response.bytes_stream(),
                hosted_web_search_name.clone(),
                hosted_web_search_max_uses,
                body_timeout,
            )
            .await?;
            (response_headers, Some(message), None)
        } else {
            let (response_headers, _status, body_bytes) =
                read_decoded_body(response, ctx.tag, body_timeout).await?;
            let body_str = String::from_utf8_lossy(&body_bytes);
            let upstream_response: Value = if aggregate_codex_oauth_responses_sse {
                responses_sse_to_response_value(&body_str)?
            } else {
                serde_json::from_slice(&body_bytes).map_err(|e| {
                    // Privacy: log only body length and parse error — a malformed 2xx body
                    // can be a full prompt/model response, so no content (not even a
                    // truncated prefix) may reach the journal at `error!` level (visible
                    // under the shipped `RUST_LOG=info` default).
                    log::error!(
                        "[Claude] 解析上游响应失败: {e}, body_bytes={}",
                        body_bytes.len()
                    );
                    ProxyError::TransformError(format!("Failed to parse upstream response: {e}"))
                })?
            };
            (response_headers, None, Some(upstream_response))
        };

    // Preserve usage so a post-upstream conversion failure still records tokens.
    // The direct Anthropic branch below is already fully transformed and cannot
    // enter the conversion-error path. Snapshot usage only for raw upstream
    // responses that still need conversion; cloning the direct message would
    // duplicate potentially large text and search-result content.
    let raw_usage_response = upstream_response.as_ref().map(|response| {
        json!({
            "id": response.get("id").cloned().unwrap_or(Value::Null),
            "model": response.get("model").cloned().unwrap_or(Value::Null),
            "usage": transform_responses::build_anthropic_usage_from_responses(
                response.get("usage")
            )
        })
    });

    // 根据 api_format 选择非流式转换器
    let transform_result = match (direct_anthropic_response, upstream_response) {
        (Some(response), _) => Ok(response),
        (None, Some(response)) if api_format == "openai_responses" => {
            transform_responses::responses_to_anthropic_with_web_search_options(
                response,
                hosted_web_search_name.as_deref(),
                hosted_web_search_max_uses,
            )
        }
        (None, Some(response)) if api_format == "gemini_native" => {
            transform_gemini::gemini_to_anthropic_with_shadow_and_hints(
                response,
                Some(state.gemini_shadow.as_ref()),
                Some(&ctx.provider.id),
                Some(&ctx.session_id),
                tool_schema_hints.as_ref(),
            )
        }
        (None, Some(response)) => transform::openai_to_anthropic(response),
        (None, None) => Err(ProxyError::Internal(
            "Missing upstream response after Claude format conversion".to_string(),
        )),
    };
    let anthropic_response = match transform_result {
        Ok(response) => response,
        Err(error) => {
            log::error!("[Claude] 转换响应失败: {error}");
            if usage_logging_enabled(state) {
                if let Some(log) = raw_usage_response.as_ref().and_then(|response| {
                    prepare_claude_usage_log(ctx, response, status.as_u16(), false)
                }) {
                    // The upstream request already succeeded and consumed tokens. Persist
                    // usage before returning the terminal transform error to the client.
                    write_claude_usage_log(state, log).await;
                }
            }
            return Err(error);
        }
    };

    // 记录使用量
    spawn_claude_usage_log(state, ctx, &anthropic_response, status.as_u16(), false);

    // 构建响应
    let mut builder = axum::response::Response::builder().status(status);
    strip_entity_headers_for_rebuilt_body(&mut response_headers);
    strip_hop_by_hop_response_headers(&mut response_headers);

    for (key, value) in response_headers.iter() {
        builder = builder.header(key, value);
    }

    builder = builder.header("content-type", "application/json");

    let response_body = serde_json::to_vec(&anthropic_response).map_err(|e| {
        log::error!("[Claude] 序列化响应失败: {e}");
        ProxyError::TransformError(format!("Failed to serialize response: {e}"))
    })?;

    let body = axum::body::Body::from(response_body);
    builder.body(body).map_err(|e| {
        log::error!("[Claude] 构建响应失败: {e}");
        ProxyError::Internal(format!("Failed to build response: {e}"))
    })
}

fn endpoint_with_query(uri: &axum::http::Uri, endpoint: &str) -> String {
    match uri.query() {
        Some(query) => format!("{endpoint}?{query}"),
        None => endpoint.to_string(),
    }
}

/// Codex Desktop OAuth requests may arrive with compressed bodies, especially zstd.
/// Decode before JSON parsing and drop stale entity headers; the forwarder
/// serializes a fresh JSON body and rebuilds those headers for the upstream.
fn decode_codex_request_body(
    headers: &mut axum::http::HeaderMap,
    body_bytes: Bytes,
) -> Result<Bytes, ProxyError> {
    let Some(encoding) = get_content_encoding(headers) else {
        return Ok(body_bytes);
    };

    if !is_supported_content_encoding(&encoding) {
        return Err(ProxyError::InvalidRequest(format!(
            "Unsupported request content-encoding: {encoding}"
        )));
    }

    log::debug!("[Codex] decompress request body: content-encoding={encoding}");
    let decompressed = match decompress_body(&encoding, &body_bytes) {
        Ok(Some(decompressed)) => decompressed,
        Ok(None) => {
            return Err(ProxyError::InvalidRequest(format!(
                "Unsupported request content-encoding: {encoding}"
            )));
        }
        Err(error) => {
            log::warn!("[Codex] request body decompression failed ({encoding}): {error}");
            return Err(ProxyError::InvalidRequest(format!(
                "Failed to decompress request body ({encoding}): {error}"
            )));
        }
    };

    headers.remove(axum::http::header::CONTENT_ENCODING);
    headers.remove(axum::http::header::CONTENT_LENGTH);
    headers.remove(axum::http::header::TRANSFER_ENCODING);

    Ok(Bytes::from(decompressed))
}

// ============================================================================
// Codex API 处理器
// ============================================================================

/// 处理 /v1/chat/completions 请求（OpenAI Chat Completions API - Codex CLI）
pub async fn handle_chat_completions(
    State(state): State<ProxyState>,
    request: axum::extract::Request,
) -> Result<axum::response::Response, ProxyError> {
    let (parts, req_body) = request.into_parts();
    let method = parts.method.clone();
    let uri = parts.uri;
    let mut headers = parts.headers;
    let extensions = parts.extensions;
    let body_bytes = req_body
        .collect()
        .await
        .map_err(|e| ProxyError::Internal(format!("Failed to read request body: {e}")))?
        .to_bytes();
    let body_bytes = decode_codex_request_body(&mut headers, body_bytes)?;
    let body: Value = serde_json::from_slice(&body_bytes)
        .map_err(|e| ProxyError::Internal(format!("Failed to parse request body: {e}")))?;

    let mut ctx =
        RequestContext::new(&state, &body, &headers, AppType::Codex, "Codex", "codex").await?;
    let endpoint = endpoint_with_query(&uri, "/chat/completions");

    let is_stream = body
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let namespace_restore_map = transform_codex_responses_namespace::namespace_restore_map(&body);

    let forwarder = ctx.create_forwarder(&state);
    let mut result = match forwarder
        .forward_with_retry(
            &AppType::Codex,
            method,
            &endpoint,
            body,
            headers,
            extensions,
            ctx.get_providers(),
            ctx.failover_enabled(),
        )
        .await
    {
        Ok(result) => result,
        Err(mut err) => {
            if let Some(provider) = err.provider.take() {
                ctx.provider = provider;
            }
            log_forward_error(&state, &ctx, is_stream, &err.error);
            return build_codex_proxy_error_response(&ctx, &endpoint, &err.error);
        }
    };

    let connection_guard = result.connection_guard.take();
    ctx.provider = result.provider;
    let response = result.response;

    if super::providers::provider_needs_responses_namespace_flatten(&ctx.provider)
        && !namespace_restore_map.is_empty()
    {
        return handle_codex_responses_namespace_restore(
            response,
            &ctx,
            &state,
            connection_guard,
            namespace_restore_map,
        )
        .await;
    }

    process_response(
        response,
        &ctx,
        &state,
        &OPENAI_PARSER_CONFIG,
        connection_guard,
    )
    .await
}

/// 处理 /v1/responses 请求（OpenAI Responses API - Codex CLI 透传）
pub async fn handle_responses(
    State(state): State<ProxyState>,
    request: axum::extract::Request,
) -> Result<axum::response::Response, ProxyError> {
    handle_responses_for_app(state, request, AppType::Codex, "Codex", "codex").await
}

pub async fn handle_grokbuild_responses(
    State(state): State<ProxyState>,
    request: axum::extract::Request,
) -> Result<axum::response::Response, ProxyError> {
    handle_responses_for_app(
        state,
        request,
        AppType::GrokBuild,
        "Grok Build",
        "grokbuild",
    )
    .await
}

async fn handle_responses_for_app(
    state: ProxyState,
    request: axum::extract::Request,
    app_type: AppType,
    tag: &'static str,
    app_type_str: &'static str,
) -> Result<axum::response::Response, ProxyError> {
    let (parts, req_body) = request.into_parts();
    let method = parts.method.clone();
    let uri = parts.uri;
    let mut headers = parts.headers;
    let extensions = parts.extensions;
    let body_bytes = req_body
        .collect()
        .await
        .map_err(|e| ProxyError::Internal(format!("Failed to read request body: {e}")))?
        .to_bytes();
    let body_bytes = decode_codex_request_body(&mut headers, body_bytes)?;
    let body: Value = serde_json::from_slice(&body_bytes)
        .map_err(|e| ProxyError::Internal(format!("Failed to parse request body: {e}")))?;

    let mut ctx =
        RequestContext::new(&state, &body, &headers, app_type.clone(), tag, app_type_str).await?;
    let endpoint = endpoint_with_query(&uri, "/responses");

    let is_stream = body
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let namespace_restore_map = transform_codex_responses_namespace::namespace_restore_map(&body);

    let forwarder = ctx.create_forwarder(&state);
    let mut result = match forwarder
        .forward_with_retry(
            &app_type,
            method,
            &endpoint,
            body,
            headers,
            extensions,
            ctx.get_providers(),
            ctx.failover_enabled(),
        )
        .await
    {
        Ok(result) => result,
        Err(mut err) => {
            if let Some(provider) = err.provider.take() {
                ctx.provider = provider;
            }
            log_forward_error(&state, &ctx, is_stream, &err.error);
            return build_codex_proxy_error_response(&ctx, &endpoint, &err.error);
        }
    };

    let connection_guard = result.connection_guard.take();
    ctx.provider = result.provider;
    let response = result.response;

    if super::providers::provider_needs_responses_namespace_flatten(&ctx.provider)
        && !namespace_restore_map.is_empty()
    {
        return handle_codex_responses_namespace_restore(
            response,
            &ctx,
            &state,
            connection_guard,
            namespace_restore_map,
        )
        .await;
    }

    process_response(
        response,
        &ctx,
        &state,
        &CODEX_PARSER_CONFIG,
        connection_guard,
    )
    .await
}

/// 处理 /v1/responses/compact 请求（OpenAI Responses Compact API - Codex CLI 透传）
pub async fn handle_responses_compact(
    State(state): State<ProxyState>,
    request: axum::extract::Request,
) -> Result<axum::response::Response, ProxyError> {
    handle_responses_compact_for_app(state, request, AppType::Codex, "Codex", "codex").await
}

/// Handle Codex's standalone Alpha Search protocol as a semantic passthrough.
///
/// Recent Codex clients send web-search commands to a dedicated endpoint instead
/// of embedding them in a Responses request. Keep this path out of the
/// Responses-to-Chat/Anthropic bridges: those formats cannot represent the Alpha
/// Search protocol.
pub async fn handle_alpha_search(
    State(state): State<ProxyState>,
    request: axum::extract::Request,
) -> Result<axum::response::Response, ProxyError> {
    let (parts, req_body) = request.into_parts();
    let method = parts.method.clone();
    let uri = parts.uri;
    let mut headers = parts.headers;
    let extensions = parts.extensions;
    let body_bytes = req_body
        .collect()
        .await
        .map_err(|e| ProxyError::Internal(format!("Failed to read request body: {e}")))?
        .to_bytes();
    let body_bytes = decode_codex_request_body(&mut headers, body_bytes)?;
    let body: Value = serde_json::from_slice(&body_bytes)
        .map_err(|e| ProxyError::InvalidRequest(format!("Failed to parse request body: {e}")))?;

    let mut ctx =
        RequestContext::new(&state, &body, &headers, AppType::Codex, "Codex", "codex").await?;
    let endpoint = endpoint_with_query(&uri, "/alpha/search");

    let forwarder = ctx.create_forwarder(&state);
    let mut result = match forwarder
        .forward_with_retry(
            &AppType::Codex,
            method,
            &endpoint,
            body,
            headers,
            extensions,
            ctx.get_providers(),
            ctx.failover_enabled(),
        )
        .await
    {
        Ok(result) => result,
        Err(mut err) => {
            if let Some(provider) = err.provider.take() {
                ctx.provider = provider;
            }
            log_forward_error(&state, &ctx, false, &err.error);
            return build_codex_proxy_error_response(&ctx, &endpoint, &err.error);
        }
    };

    let connection_guard = result.connection_guard.take();
    ctx.provider = result.provider;

    process_response(
        result.response,
        &ctx,
        &state,
        &CODEX_PARSER_CONFIG,
        connection_guard,
    )
    .await
}

pub async fn handle_grokbuild_responses_compact(
    State(state): State<ProxyState>,
    request: axum::extract::Request,
) -> Result<axum::response::Response, ProxyError> {
    handle_responses_compact_for_app(
        state,
        request,
        AppType::GrokBuild,
        "Grok Build",
        "grokbuild",
    )
    .await
}

async fn handle_responses_compact_for_app(
    state: ProxyState,
    request: axum::extract::Request,
    app_type: AppType,
    tag: &'static str,
    app_type_str: &'static str,
) -> Result<axum::response::Response, ProxyError> {
    let (parts, req_body) = request.into_parts();
    let method = parts.method.clone();
    let uri = parts.uri;
    let mut headers = parts.headers;
    let extensions = parts.extensions;
    let body_bytes = req_body
        .collect()
        .await
        .map_err(|e| ProxyError::Internal(format!("Failed to read request body: {e}")))?
        .to_bytes();
    let body_bytes = decode_codex_request_body(&mut headers, body_bytes)?;
    let body: Value = serde_json::from_slice(&body_bytes)
        .map_err(|e| ProxyError::Internal(format!("Failed to parse request body: {e}")))?;

    let mut ctx =
        RequestContext::new(&state, &body, &headers, app_type.clone(), tag, app_type_str).await?;
    let endpoint = endpoint_with_query(&uri, "/responses/compact");

    let is_stream = body
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let namespace_restore_map = transform_codex_responses_namespace::namespace_restore_map(&body);

    let forwarder = ctx.create_forwarder(&state);
    let mut result = match forwarder
        .forward_with_retry(
            &app_type,
            method,
            &endpoint,
            body,
            headers,
            extensions,
            ctx.get_providers(),
            ctx.failover_enabled(),
        )
        .await
    {
        Ok(result) => result,
        Err(mut err) => {
            if let Some(provider) = err.provider.take() {
                ctx.provider = provider;
            }
            log_forward_error(&state, &ctx, is_stream, &err.error);
            return build_codex_proxy_error_response(&ctx, &endpoint, &err.error);
        }
    };

    let connection_guard = result.connection_guard.take();
    ctx.provider = result.provider;
    let response = result.response;

    if super::providers::provider_needs_responses_namespace_flatten(&ctx.provider)
        && !namespace_restore_map.is_empty()
    {
        return handle_codex_responses_namespace_restore(
            response,
            &ctx,
            &state,
            connection_guard,
            namespace_restore_map,
        )
        .await;
    }

    process_response(
        response,
        &ctx,
        &state,
        &CODEX_PARSER_CONFIG,
        connection_guard,
    )
    .await
}

/// Restore flattened Codex namespace tool names on the managed xAI native
/// Responses path. Error responses retain the fork's generic passthrough and
/// bounded logging behavior; successful responses keep the normal Codex usage
/// parser and session attribution.
async fn handle_codex_responses_namespace_restore(
    response: super::hyper_client::ProxyResponse,
    ctx: &RequestContext,
    state: &ProxyState,
    connection_guard: Option<ActiveConnectionGuard>,
    restore_map: std::collections::HashMap<
        String,
        transform_codex_responses_namespace::NamespacedName,
    >,
) -> Result<axum::response::Response, ProxyError> {
    let status = response.status();

    if !status.is_success() {
        return process_response(response, ctx, state, &CODEX_PARSER_CONFIG, connection_guard)
            .await;
    }

    if response.is_sse() {
        let mut response_headers = response.headers().clone();
        strip_hop_by_hop_response_headers(&mut response_headers);

        let mut builder = axum::response::Response::builder().status(status);
        for (key, value) in &response_headers {
            builder = builder.header(key, value);
        }

        let restore_stream =
            transform_codex_responses_namespace::create_namespace_restore_sse_stream(
                response.bytes_stream(),
                restore_map,
            );
        let usage_collector =
            create_usage_collector(ctx, state, status.as_u16(), &CODEX_PARSER_CONFIG);
        let logged_stream = create_logged_passthrough_stream(
            restore_stream,
            ctx.tag,
            usage_collector,
            ctx.streaming_timeout_config(),
            connection_guard,
        );

        return builder
            .body(axum::body::Body::from_stream(logged_stream))
            .map_err(|error| {
                log::error!("[{}] 构建 namespace 还原流式响应失败: {error}", ctx.tag);
                ProxyError::Internal(format!("Failed to build namespace restore stream: {error}"))
            });
    }

    let _connection_guard = connection_guard;
    let body_timeout =
        if ctx.app_config.auto_failover_enabled && ctx.app_config.non_streaming_timeout > 0 {
            std::time::Duration::from_secs(ctx.app_config.non_streaming_timeout as u64)
        } else {
            std::time::Duration::ZERO
        };
    let (mut response_headers, status, body_bytes) =
        read_decoded_body(response, ctx.tag, body_timeout).await?;
    strip_hop_by_hop_response_headers(&mut response_headers);

    // Full response bodies remain debug-only under the fork's shipped
    // RUST_LOG=info posture.
    log::debug!(
        "[{}] 上游响应体内容: {}",
        ctx.tag,
        String::from_utf8_lossy(&body_bytes)
    );

    let (response_bytes, rebuilt_json) = match serde_json::from_slice::<Value>(&body_bytes) {
        Ok(mut value) => {
            let changed = transform_codex_responses_namespace::restore_response_namespaces(
                &mut value,
                &restore_map,
            );

            if usage_logging_enabled(state) {
                if let Some(usage) = TokenUsage::from_codex_response_auto(&value) {
                    let model = usage
                        .model
                        .clone()
                        .or_else(|| {
                            value
                                .get("model")
                                .and_then(Value::as_str)
                                .map(str::to_string)
                        })
                        .unwrap_or_else(|| ctx.request_model.clone());
                    spawn_log_usage(
                        state,
                        ctx,
                        usage,
                        &model,
                        &ctx.request_model,
                        status.as_u16(),
                        false,
                    );
                } else {
                    spawn_log_usage(
                        state,
                        ctx,
                        TokenUsage::default(),
                        &ctx.request_model,
                        &ctx.request_model,
                        status.as_u16(),
                        false,
                    );
                    log::debug!("[Codex] namespace 还原响应缺少 usage，跳过消费记录");
                }
            }

            if changed {
                match serde_json::to_vec(&value) {
                    Ok(bytes) => (Bytes::from(bytes), true),
                    Err(error) => {
                        log::error!("[{}] 序列化 namespace 还原响应失败: {error}", ctx.tag);
                        (body_bytes, false)
                    }
                }
            } else {
                (body_bytes, false)
            }
        }
        Err(_) => {
            if usage_logging_enabled(state) {
                spawn_log_usage(
                    state,
                    ctx,
                    TokenUsage::default(),
                    &ctx.request_model,
                    &ctx.request_model,
                    status.as_u16(),
                    false,
                );
            }
            log::debug!(
                "[{}] namespace 还原响应不是 JSON，按原字节透传 ({} bytes)",
                ctx.tag,
                body_bytes.len()
            );
            (body_bytes, false)
        }
    };

    if rebuilt_json {
        strip_entity_headers_for_rebuilt_body(&mut response_headers);
        response_headers.remove(axum::http::header::CONTENT_TYPE);
    }

    let mut builder = axum::response::Response::builder().status(status);
    for (key, value) in response_headers.iter() {
        builder = builder.header(key, value);
    }
    if rebuilt_json {
        builder = builder.header(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("application/json"),
        );
    }

    builder
        .body(axum::body::Body::from(response_bytes))
        .map_err(|error| {
            log::error!("[{}] 构建 namespace 还原响应失败: {error}", ctx.tag);
            ProxyError::Internal(format!(
                "Failed to build namespace restore response: {error}"
            ))
        })
}

/// 把转发层（非上游响应）的失败构造成富化的 Codex 错误响应。
///
/// 这里没有上游响应可参照，只产出一个 `application/json` 错误体。状态码走
/// `map_proxy_error_to_status`，该函数已与 `ProxyError::into_response` 对齐。
///
/// 注意：`endpoint` 经 `endpoint_with_query` 可能携带 query（如 `?beta=true`）并被
/// 原样写入错误体。当前 Codex 端点不在 query 里放凭证，故安全；若将来复用到
/// query 携带密钥的端点（如 Gemini 的 `?key=`），需先脱敏再回显。
fn build_codex_proxy_error_response(
    ctx: &RequestContext,
    endpoint: &str,
    error: &ProxyError,
) -> Result<axum::response::Response, ProxyError> {
    let status = axum::http::StatusCode::from_u16(map_proxy_error_to_status(error))
        .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    let body = codex_proxy_error_json(&ctx.provider.name, &ctx.request_model, endpoint, error);
    let body = serde_json::to_vec(&body).map_err(|e| {
        log::error!("[Codex] 序列化代理错误体失败: {e}");
        ProxyError::Internal(format!("Failed to serialize proxy error: {e}"))
    })?;

    axum::response::Response::builder()
        .status(status)
        .header(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("application/json"),
        )
        .body(axum::body::Body::from(body))
        .map_err(|e| {
            log::error!("[Codex] 构建代理错误响应失败: {e}");
            ProxyError::Internal(format!("Failed to build proxy error response: {e}"))
        })
}

fn codex_proxy_error_json(
    provider_name: &str,
    request_model: &str,
    endpoint: &str,
    error: &ProxyError,
) -> Value {
    let (mut body, upstream_status) = match error {
        ProxyError::UpstreamError { status, body } => {
            let parsed_body = body
                .as_deref()
                .map(|body| serde_json::from_str::<Value>(body).unwrap_or_else(|_| json!(body)));
            (
                chat_error_to_response_error(parsed_body.as_ref()),
                Some(*status),
            )
        }
        _ => (
            json!({
                "error": {
                    "message": get_error_message(error),
                    "type": "proxy_error",
                    "code": codex_proxy_error_code(error),
                    "param": Value::Null,
                }
            }),
            None,
        ),
    };

    let Some(error_obj) = body
        .get_mut("error")
        .and_then(|value| value.as_object_mut())
    else {
        return body;
    };

    let message = if upstream_status == Some(413) {
        // 413 来自上游渠道商的网关（典型是 nginx 的 client_max_body_size），不是 CC
        // Switch 本地代理的限制（本地 DefaultBodyLimit 已放到 200MB）。上游响应体往往是
        // 一整段 nginx HTML，对用户毫无价值，这里替换成明确指向上游 + 可操作的指引，
        // 避免「以为是 CC Switch 封装了 nginx / 是本地代理的锅」这种反复出现的误解。
        format!(
            concat!(
                "Upstream provider rejected the request with HTTP 413 (Payload Too Large). ",
                "The request body exceeds the upstream gateway's size limit; this is the ",
                "provider's server-side limit, not a CC Switch limit. ",
                "Provider: {provider}; model: {model}; endpoint: {endpoint}. ",
                "To recover, shrink the request: run /compact, remove large pasted logs or ",
                "inline images, or ask the provider to raise its request body limit ",
                "(e.g. nginx client_max_body_size)."
            ),
            provider = provider_name,
            model = request_model,
            endpoint = endpoint,
        )
    } else {
        let cause = error_obj
            .get("message")
            .and_then(|value| value.as_str())
            .map(ToString::to_string)
            .filter(|message| !message.trim().is_empty())
            .unwrap_or_else(|| get_error_message(error));
        let status_fragment = upstream_status
            .map(|status| format!("; upstream_status: HTTP {status}"))
            .unwrap_or_default();
        format!(
            "CC Switch local proxy failed while handling Codex endpoint {endpoint}. Provider: {provider_name}; model: {request_model}{status_fragment}; cause: {cause}"
        )
    };

    error_obj.insert(
        "message".to_string(),
        Value::String(compact_error_message(&message, 1800)),
    );

    if error_obj
        .get("type")
        .and_then(|value| value.as_str())
        .map(|value| value.trim().is_empty())
        .unwrap_or(true)
    {
        error_obj.insert("type".to_string(), Value::String("proxy_error".to_string()));
    }

    if error_obj.get("code").map(Value::is_null).unwrap_or(true) {
        error_obj.insert(
            "code".to_string(),
            Value::String(codex_proxy_error_code(error).to_string()),
        );
    }

    if !error_obj.contains_key("param") {
        error_obj.insert("param".to_string(), Value::Null);
    }

    error_obj.insert(
        "provider".to_string(),
        Value::String(provider_name.to_string()),
    );
    error_obj.insert(
        "model".to_string(),
        Value::String(request_model.to_string()),
    );
    // 仅用于 Codex 本地路由；不要复用到 query 可能携带凭证的端点。
    error_obj.insert("endpoint".to_string(), Value::String(endpoint.to_string()));
    if let Some(status) = upstream_status {
        error_obj.insert(
            "upstream_status".to_string(),
            Value::Number(serde_json::Number::from(status)),
        );
    }

    body
}

/// 把上游 Chat Completions 风格的错误体规范化成 Responses 风格的错误对象。
///
/// 注：上游（farion1231/cc-switch）把本函数放在 `proxy/providers/transform_codex_chat.rs`
/// （Codex Chat Completions 路由栈的一部分）。该特性栈在本 fork 中尚未移植
/// （v3.15 同步时记录为独立特性同步、暂缓），因此先内联在这里；
/// 将来移植 Codex Chat 路由栈时应把它挪回 `transform_codex_chat.rs` 并复用。
fn chat_error_to_response_error(body: Option<&Value>) -> Value {
    let Some(value) = body else {
        return json!({
            "error": {
                "message": "Upstream returned an empty error response",
                "type": "upstream_error",
                "code": serde_json::Value::Null,
                "param": serde_json::Value::Null,
            }
        });
    };

    if let Some(text) = value.as_str() {
        return json!({
            "error": {
                "message": text,
                "type": "upstream_error",
                "code": serde_json::Value::Null,
                "param": serde_json::Value::Null,
            }
        });
    }

    let source = value.get("error").unwrap_or(value);

    let message = source
        .get("message")
        .or_else(|| source.get("detail"))
        .or_else(|| source.get("status_msg"))
        .or_else(|| source.pointer("/base_resp/status_msg"))
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
        .or_else(|| source.as_str().map(ToString::to_string))
        .unwrap_or_else(|| {
            // 没法从字段提取出文本，就把整个 JSON 序列化回去，方便用户排查。
            serde_json::to_string(source).unwrap_or_else(|_| "Upstream error".to_string())
        });

    let error_type = source
        .get("type")
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| "upstream_error".to_string());

    let code = source
        .get("code")
        .cloned()
        .or_else(|| source.pointer("/base_resp/status_code").cloned())
        .unwrap_or(serde_json::Value::Null);

    let param = source
        .get("param")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    json!({
        "error": {
            "message": message,
            "type": error_type,
            "code": code,
            "param": param,
        }
    })
}

fn codex_proxy_error_code(error: &ProxyError) -> &'static str {
    match error {
        ProxyError::ForwardFailed(_) => "cc_switch_forward_failed",
        ProxyError::Timeout(_) | ProxyError::StreamIdleTimeout(_) => "cc_switch_timeout",
        ProxyError::NoAvailableProvider => "cc_switch_no_available_provider",
        ProxyError::AllProvidersCircuitOpen => "cc_switch_all_providers_circuit_open",
        ProxyError::NoProvidersConfigured => "cc_switch_no_providers_configured",
        ProxyError::MaxRetriesExceeded => "cc_switch_max_retries_exceeded",
        ProxyError::ProviderUnhealthy(_) => "cc_switch_provider_unhealthy",
        ProxyError::ConfigError(_) => "cc_switch_config_error",
        ProxyError::TransformError(_) => "cc_switch_transform_error",
        ProxyError::InvalidRequest(_) => "cc_switch_invalid_request",
        ProxyError::AuthError(_) => "cc_switch_auth_error",
        ProxyError::UpstreamError { .. } => "cc_switch_upstream_error",
        ProxyError::DatabaseError(_) => "cc_switch_database_error",
        ProxyError::Internal(_) => "cc_switch_internal_error",
        ProxyError::AlreadyRunning
        | ProxyError::NotRunning
        | ProxyError::BindFailed(_)
        | ProxyError::StopTimeout
        | ProxyError::StopFailed(_)
        | ProxyError::ResponseBodyTooLarge(_) => "cc_switch_proxy_error",
    }
}

fn compact_error_message(message: &str, max_chars: usize) -> String {
    let normalized = message.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= max_chars {
        return normalized;
    }

    let truncated = normalized
        .chars()
        .take(max_chars)
        .collect::<String>()
        .trim_end()
        .to_string();
    format!("{truncated}…(truncated)")
}

// ============================================================================
// Gemini API 处理器
// ============================================================================

/// 处理 Gemini API 请求（透传，包括查询参数）
pub async fn handle_gemini(
    State(state): State<ProxyState>,
    uri: axum::http::Uri,
    request: axum::extract::Request,
) -> Result<axum::response::Response, ProxyError> {
    let (parts, req_body) = request.into_parts();
    let method = parts.method.clone();
    let headers = parts.headers;
    let extensions = parts.extensions;
    let body_bytes = req_body
        .collect()
        .await
        .map_err(|e| ProxyError::Internal(format!("Failed to read request body: {e}")))?
        .to_bytes();
    let body: Value = if body_bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&body_bytes)
            .map_err(|e| ProxyError::Internal(format!("Failed to parse request body: {e}")))?
    };

    // Gemini 的模型名称在 URI 中
    let mut ctx = RequestContext::new(&state, &body, &headers, AppType::Gemini, "Gemini", "gemini")
        .await?
        .with_model_from_uri(&uri);

    // 提取完整的路径和查询参数
    let endpoint = uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or(uri.path());

    let is_stream = body
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let forwarder = ctx.create_forwarder(&state);
    let mut result = match forwarder
        .forward_with_retry(
            &AppType::Gemini,
            method,
            endpoint,
            body,
            headers,
            extensions,
            ctx.get_providers(),
            ctx.failover_enabled(),
        )
        .await
    {
        Ok(result) => result,
        Err(mut err) => {
            if let Some(provider) = err.provider.take() {
                ctx.provider = provider;
            }
            log_forward_error(&state, &ctx, is_stream, &err.error);
            return Err(err.error);
        }
    };

    let connection_guard = result.connection_guard.take();
    ctx.provider = result.provider;
    let response = result.response;

    process_response(
        response,
        &ctx,
        &state,
        &GEMINI_PARSER_CONFIG,
        connection_guard,
    )
    .await
}

fn should_use_claude_transform_streaming(
    requested_streaming: bool,
    upstream_is_sse: bool,
    api_format: &str,
    is_codex_oauth: bool,
) -> bool {
    requested_streaming || upstream_is_sse || (is_codex_oauth && api_format == "openai_responses")
}

async fn responses_sse_stream_to_anthropic_message(
    stream: impl futures::Stream<Item = Result<Bytes, std::io::Error>> + Send + 'static,
    hosted_web_search_name: Option<String>,
    max_web_search_uses: Option<u64>,
    body_timeout: std::time::Duration,
) -> Result<Value, ProxyError> {
    let collect = async move {
        let converted = create_anthropic_sse_stream_from_responses_with_web_search_options(
            stream,
            hosted_web_search_name,
            max_web_search_uses,
        );
        tokio::pin!(converted);

        let mut body = Vec::new();
        while let Some(chunk) = converted.next().await {
            let chunk = chunk.map_err(|error| {
                ProxyError::ForwardFailed(format!(
                    "Failed to transform upstream Responses SSE: {error}"
                ))
            })?;
            body.extend_from_slice(&chunk);
        }
        String::from_utf8(body).map_err(|error| {
            ProxyError::TransformError(format!(
                "Transformed Anthropic SSE was not valid UTF-8: {error}"
            ))
        })
    };

    let body = if body_timeout.is_zero() {
        collect.await?
    } else {
        tokio::time::timeout(body_timeout, collect)
            .await
            .map_err(|_| {
                ProxyError::Timeout(format!(
                    "响应体读取超时: {}s（上游发完响应头后 body 未到达）",
                    body_timeout.as_secs()
                ))
            })??
    };

    anthropic_sse_to_message_value(&body)
}

/// Aggregates an Anthropic Messages **SSE stream** back into a single Anthropic
/// non-streaming message JSON.
///
/// Upstream keeps this helper in `providers::transform_codex_anthropic`, which this
/// fork does not carry (the whole Codex Chat routing stack is deferred, see the
/// task PRD Q1). Only this one function is needed here: the hosted-WebSearch
/// non-streaming path must run the upstream Responses SSE through the streaming
/// converter (so `max_uses` can stop the upstream mid-stream) and then fold the
/// converted Anthropic SSE back into one message. Ported at upstream's
/// post-`bdeaac75` state, i.e. `message_delta.usage` is merged as a whole object
/// rather than only `output_tokens` — the Responses bridge reports final input
/// tokens and `server_tool_use.web_search_requests` there.
///
/// It also tolerates the last event missing a trailing blank line (truncated
/// stream): after looping over complete event blocks, it processes the residual
/// buffer as the last event.
fn anthropic_sse_to_message_value(body: &str) -> Result<Value, ProxyError> {
    let mut message: Option<Value> = None;
    // Collect blocks by content index along with the partial_json accumulator for their tool_use.
    let mut blocks: BTreeMap<u64, Value> = BTreeMap::new();
    let mut json_accum: BTreeMap<u64, String> = BTreeMap::new();
    let mut stop_reason: Option<String> = None;
    let mut delta_usage: Option<Value> = None;
    let mut saw_message_stop = false;

    let mut buffer = body.to_string();
    let process_block = |block: &str,
                         message: &mut Option<Value>,
                         blocks: &mut BTreeMap<u64, Value>,
                         json_accum: &mut BTreeMap<u64, String>,
                         stop_reason: &mut Option<String>,
                         delta_usage: &mut Option<Value>,
                         saw_message_stop: &mut bool|
     -> Result<(), ProxyError> {
        let mut data = String::new();
        for line in block.lines() {
            if let Some(chunk) = strip_sse_field(line, "data") {
                if !data.is_empty() {
                    data.push('\n');
                }
                data.push_str(chunk);
            }
        }
        if data.trim().is_empty() || data.trim() == "[DONE]" {
            return Ok(());
        }
        let value: Value = match serde_json::from_str(data.trim()) {
            Ok(v) => v,
            Err(_) => return Ok(()), // Skip events that cannot be parsed (ping, etc.)
        };
        match value.get("type").and_then(|t| t.as_str()).unwrap_or("") {
            "message_start" => {
                // Only accept an object message; a malformed upstream could send a
                // scalar/array here, and the later `message["content"] = …` index
                // assignment would panic on a non-object Value.
                if let Some(msg) = value.get("message").filter(|m| m.is_object()) {
                    *message = Some(msg.clone());
                }
            }
            "content_block_start" => {
                if let Some(index) = value.get("index").and_then(|v| v.as_u64()) {
                    // Sanitize to an object: any later index-assignment (`["text"]`,
                    // `["signature"]`, `["input"]`) requires a JSON object, so a
                    // malformed non-object block from the upstream cannot be stored
                    // verbatim (it would panic on the next delta).
                    //
                    // The replacement carries `type: "text"` rather than being empty:
                    // the deltas that follow are usually well-formed, and a block with
                    // no `type` is silently dropped by the final conversion, which turns
                    // a garbled block header into a `completed` response with empty
                    // output — the client sees the model saying nothing and has no way to
                    // tell that data was discarded. A text block recovers the common
                    // case; a tool-use block still yields nothing, exactly as before.
                    let block = match value.get("content_block") {
                        Some(block) if block.is_object() => block.clone(),
                        malformed => {
                            if malformed.is_some() {
                                log::warn!(
                                    "Anthropic upstream sent a non-object content_block at index {index}; recovering it as a text block"
                                );
                            }
                            json!({ "type": "text" })
                        }
                    };
                    blocks.insert(index, block);
                    json_accum.entry(index).or_default();
                }
            }
            "content_block_delta" => {
                if let Some(index) = value.get("index").and_then(|v| v.as_u64()) {
                    let delta = value.get("delta").cloned().unwrap_or(json!({}));
                    match delta.get("type").and_then(|t| t.as_str()).unwrap_or("") {
                        "text_delta" => {
                            if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                                append_str_field(
                                    blocks.entry(index).or_insert(json!({})),
                                    "text",
                                    text,
                                );
                            }
                        }
                        "thinking_delta" => {
                            if let Some(text) = delta.get("thinking").and_then(|t| t.as_str()) {
                                append_str_field(
                                    blocks.entry(index).or_insert(json!({})),
                                    "thinking",
                                    text,
                                );
                            }
                        }
                        "signature_delta" => {
                            if let Some(sig) = delta.get("signature").and_then(|t| t.as_str()) {
                                blocks.entry(index).or_insert(json!({}))["signature"] = json!(sig);
                            }
                        }
                        "input_json_delta" => {
                            if let Some(partial) =
                                delta.get("partial_json").and_then(|t| t.as_str())
                            {
                                json_accum.entry(index).or_default().push_str(partial);
                            }
                        }
                        _ => {}
                    }
                }
            }
            "content_block_stop" => {
                if let Some(index) = value.get("index").and_then(|v| v.as_u64()) {
                    if let Some(accum) = json_accum.get(&index) {
                        if !accum.trim().is_empty() {
                            let parsed: Value =
                                serde_json::from_str(accum).unwrap_or_else(|_| json!({}));
                            if let Some(block) = blocks.get_mut(&index) {
                                block["input"] = parsed;
                            }
                        }
                    }
                }
            }
            "message_delta" => {
                if let Some(reason) = value.pointer("/delta/stop_reason").and_then(|v| v.as_str()) {
                    *stop_reason = Some(reason.to_string());
                }
                if let Some(usage) = value.get("usage").and_then(Value::as_object) {
                    let target = delta_usage.get_or_insert_with(|| json!({}));
                    if let Some(target) = target.as_object_mut() {
                        for (key, value) in usage {
                            target.insert(key.clone(), value.clone());
                        }
                    }
                }
            }
            "message_stop" => *saw_message_stop = true,
            "error" => {
                let msg = value
                    .pointer("/error/message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("upstream anthropic SSE error");
                return Err(ProxyError::TransformError(format!(
                    "anthropic SSE error event: {msg}"
                )));
            }
            _ => {}
        }
        Ok(())
    };

    while let Some(block) = take_sse_block(&mut buffer) {
        process_block(
            &block,
            &mut message,
            &mut blocks,
            &mut json_accum,
            &mut stop_reason,
            &mut delta_usage,
            &mut saw_message_stop,
        )?;
    }
    // Tolerate the last event missing a trailing blank line (truncated stream).
    if !buffer.trim().is_empty() {
        process_block(
            &buffer.clone(),
            &mut message,
            &mut blocks,
            &mut json_accum,
            &mut stop_reason,
            &mut delta_usage,
            &mut saw_message_stop,
        )?;
    }

    let mut message = message.ok_or_else(|| {
        ProxyError::TransformError(
            "anthropic SSE aggregation: missing message_start event".to_string(),
        )
    })?;

    if !saw_message_stop && stop_reason.is_none() {
        if blocks.is_empty() {
            return Err(ProxyError::TransformError(
                "anthropic SSE aggregation: stream ended before message_stop".to_string(),
            ));
        }
        // Preserve partial content but make the truncation visible to the client
        // instead of returning a normal completed response.
        stop_reason = Some("max_tokens".to_string());
    }

    // Merge in the content blocks (ordered by index), stop_reason, and the
    // cumulative message_delta usage. The Responses bridge reports final input
    // tokens and server-tool counts there rather than in message_start.
    let content: Vec<Value> = blocks.into_values().collect();
    message["content"] = json!(content);
    if let Some(reason) = stop_reason {
        message["stop_reason"] = json!(reason);
    }
    if let Some(delta_usage) = delta_usage.and_then(|usage| usage.as_object().cloned()) {
        if !message.get("usage").is_some_and(Value::is_object) {
            message["usage"] = json!({});
        }
        if let Some(usage) = message.get_mut("usage").and_then(Value::as_object_mut) {
            for (key, value) in delta_usage {
                if value.as_u64() == Some(0)
                    && usage
                        .get(&key)
                        .and_then(Value::as_u64)
                        .is_some_and(|existing| existing > 0)
                {
                    continue;
                }
                usage.insert(key, value);
            }
        }
    }

    Ok(message)
}

/// Appends content to a string field of a JSON object (creating it if absent).
fn append_str_field(block: &mut Value, field: &str, text: &str) {
    let existing = block
        .get(field)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    block[field] = json!(format!("{existing}{text}"));
}

/// 把 OpenAI Responses SSE 流聚合成一个完整的 Responses JSON 对象，供下游转成 Anthropic
/// 非流响应。仅在 Codex OAuth 把 `stream:false` 强制升级为 SSE 的场景下调用。
///
/// 复用 `proxy::sse` 的 `take_sse_block`/`strip_sse_field`：`take_sse_block` 同时支持
/// `\n\n` 与 `\r\n\r\n` 两种分隔符，`strip_sse_field` 兼容带/不带空格的字段写法。
fn responses_sse_to_response_value(body: &str) -> Result<Value, ProxyError> {
    let mut buffer = body.to_string();
    let mut completed_response: Option<Value> = None;
    let mut output_items = Vec::new();

    while let Some(block) = take_sse_block(&mut buffer) {
        let mut event_name = "";
        let mut data_lines: Vec<&str> = Vec::new();

        for line in block.lines() {
            if let Some(evt) = strip_sse_field(line, "event") {
                event_name = evt.trim();
            } else if let Some(d) = strip_sse_field(line, "data") {
                data_lines.push(d);
            }
        }

        if data_lines.is_empty() {
            continue;
        }

        let data_str = data_lines.join("\n");
        if data_str.trim() == "[DONE]" {
            continue;
        }

        let data: Value = serde_json::from_str(&data_str).map_err(|e| {
            ProxyError::TransformError(format!("Failed to parse upstream SSE event: {e}"))
        })?;

        match event_name {
            "response.output_item.done" => {
                if let Some(item) = data.get("item") {
                    output_items.push(item.clone());
                }
            }
            "response.completed" => {
                completed_response = Some(data.get("response").cloned().unwrap_or(data));
            }
            "response.failed" => {
                let message = data
                    .pointer("/response/error/message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("response.failed event received");
                return Err(ProxyError::TransformError(message.to_string()));
            }
            _ => {}
        }
    }

    let mut response = completed_response.ok_or_else(|| {
        ProxyError::TransformError("No response.completed event in upstream SSE".to_string())
    })?;

    if !output_items.is_empty() {
        if let Some(obj) = response.as_object_mut() {
            obj.insert("output".to_string(), Value::Array(output_items));
        } else {
            return Err(ProxyError::TransformError(
                "response.completed payload is not an object".to_string(),
            ));
        }
    }

    Ok(response)
}

// ============================================================================
// 使用量记录（保留用于 Claude 转换逻辑）
// ============================================================================

fn log_forward_error(
    state: &ProxyState,
    ctx: &RequestContext,
    is_streaming: bool,
    error: &ProxyError,
) {
    use super::usage::logger::UsageLogger;

    let logger = UsageLogger::new(&state.db);
    let status_code = map_proxy_error_to_status(error);
    // FIX 3 (R2 gap): for ProxyError::UpstreamError, get_error_message returns
    // the FULL upstream body, which can carry prompts/request fragments/tokens
    // or large HTML. Bound it before it is persisted into the request-log DB.
    // The client-facing error response (codex_proxy_error_json etc.) is built
    // elsewhere and stays untruncated.
    let error_message = compact_error_message(&get_error_message(error), 400);
    let request_id = uuid::Uuid::new_v4().to_string();

    if let Err(e) = logger.log_error_with_context(
        request_id,
        ctx.provider.id.clone(),
        ctx.app_type_str.to_string(),
        ctx.request_model.clone(),
        status_code,
        error_message,
        ctx.latency_ms(),
        is_streaming,
        Some(ctx.session_id.clone()),
        None,
    ) {
        log::warn!("记录失败请求日志失败: {e}");
    }
}

/// 记录请求使用量
#[allow(clippy::too_many_arguments)]
async fn log_usage(
    state: &ProxyState,
    provider_id: &str,
    app_type: &str,
    model: &str,
    request_model: &str,
    usage: TokenUsage,
    latency_ms: u64,
    first_token_ms: Option<u64>,
    is_streaming: bool,
    status_code: u16,
) {
    use super::usage::logger::UsageLogger;

    let logger = UsageLogger::new(&state.db);

    let (multiplier, pricing_model_source) =
        logger.resolve_pricing_config(provider_id, app_type).await;
    let pricing_model = if pricing_model_source == "request" {
        request_model
    } else {
        model
    };

    let dedup_scope = super::usage::parser::dedup_scope_for_app(app_type, provider_id);
    let request_id = usage.dedup_request_id(dedup_scope);

    if let Err(e) = logger.log_with_calculation(
        request_id,
        provider_id.to_string(),
        app_type.to_string(),
        model.to_string(),
        request_model.to_string(),
        pricing_model.to_string(),
        usage,
        multiplier,
        latency_ms,
        first_token_ms,
        status_code,
        None,
        None, // provider_type
        is_streaming,
    ) {
        log::warn!("[USG-001] 记录使用量失败: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::{
        anthropic_sse_to_message_value, chat_error_to_response_error, codex_proxy_error_json,
        compact_error_message, responses_sse_stream_to_anthropic_message,
        responses_sse_to_response_value, should_use_claude_transform_streaming,
    };
    use crate::proxy::error_mapper::get_error_message;
    use crate::proxy::ProxyError;
    use bytes::Bytes;
    use serde_json::json;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[test]
    fn upstream_error_body_is_truncated_before_db_persistence() {
        // FIX 3 (R2 gap): the request-log DB must not store an untruncated
        // upstream body (prompts/tokens/large HTML). log_forward_error wraps
        // get_error_message(error) in compact_error_message(.., 400).
        let big_body = "x".repeat(5000);
        let error = ProxyError::UpstreamError {
            status: 500,
            body: Some(big_body),
        };
        let persisted = compact_error_message(&get_error_message(&error), 400);
        assert!(
            persisted.chars().count() <= 400 + "…(truncated)".chars().count(),
            "persisted error_message must be bounded, got {} chars",
            persisted.chars().count()
        );
        assert!(persisted.contains("(truncated)"));
    }

    #[test]
    fn codex_oauth_responses_force_streaming_even_if_client_sent_false() {
        assert!(should_use_claude_transform_streaming(
            false,
            false,
            "openai_responses",
            true,
        ));
    }

    #[tokio::test]
    async fn non_streaming_codex_web_search_limit_stops_polling_upstream() {
        let chunks = vec![
            concat!(
                "event: response.created\n",
                "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_limit\",\"model\":\"gpt-5.6\"}}\n\n",
                "event: response.output_item.added\n",
                "data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"id\":\"ws_allowed\",\"type\":\"web_search_call\",\"status\":\"in_progress\"}}\n\n"
            ),
            concat!(
                "event: response.output_item.added\n",
                "data: {\"type\":\"response.output_item.added\",\"output_index\":1,\"item\":{\"id\":\"ws_over_limit\",\"type\":\"web_search_call\",\"status\":\"in_progress\"}}\n\n"
            ),
            concat!(
                "event: response.output_text.delta\n",
                "data: {\"type\":\"response.output_text.delta\",\"delta\":\"must never be polled\"}\n\n",
                "event: response.completed\n",
                "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_limit\",\"status\":\"completed\"}}\n\n"
            ),
        ];
        let polls = Arc::new(AtomicUsize::new(0));
        let upstream_polls = Arc::clone(&polls);
        let upstream = futures::stream::unfold(
            (chunks.into_iter(), upstream_polls),
            |(mut chunks, polls)| async move {
                chunks.next().map(|chunk| {
                    polls.fetch_add(1, Ordering::SeqCst);
                    (
                        Ok::<_, std::io::Error>(Bytes::from_static(chunk.as_bytes())),
                        (chunks, polls),
                    )
                })
            },
        );

        let message = responses_sse_stream_to_anthropic_message(
            upstream,
            Some("web_search".to_string()),
            Some(1),
            std::time::Duration::ZERO,
        )
        .await
        .unwrap();

        assert_eq!(polls.load(Ordering::SeqCst), 2);
        assert_eq!(message["stop_reason"], "end_turn");
        assert_eq!(
            message["usage"]["server_tool_use"]["web_search_requests"],
            1
        );
        let content = message["content"].as_array().unwrap();
        assert_eq!(content.len(), 4);
        assert_eq!(content[0]["type"], "server_tool_use");
        assert_eq!(content[1]["content"]["error_code"], "unavailable");
        assert_eq!(content[2]["type"], "server_tool_use");
        assert_eq!(content[3]["content"]["error_code"], "max_uses_exceeded");
    }

    // ==================== Anthropic SSE aggregation ====================
    //
    // Coverage for the fork-local `anthropic_sse_to_message_value`. Upstream keeps
    // this helper (and these tests) in `providers::transform_codex_anthropic`, a
    // module this fork does not carry. The three upstream cases that asserted
    // through `anthropic_response_to_responses` assert on the aggregated message
    // directly instead — that conversion lives in the same deferred module.

    #[test]
    fn anthropic_sse_aggregation_merges_message_delta_usage_as_a_whole_object() {
        let sse = "event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_1\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"claude\",\"content\":[],\"stop_reason\":null,\"usage\":{\"input_tokens\":10,\"output_tokens\":0}}}\n\n\
event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}\n\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\" world\"}}\n\n\
event: content_block_stop\n\
data: {\"type\":\"content_block_stop\",\"index\":0}\n\n\
event: message_delta\n\
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"input_tokens\":12,\"output_tokens\":7,\"server_tool_use\":{\"web_search_requests\":1}}}\n\n\
event: message_stop\n\
data: {\"type\":\"message_stop\"}\n\n";
        let msg = anthropic_sse_to_message_value(sse).unwrap();
        assert_eq!(msg["content"][0]["type"], "text");
        assert_eq!(msg["content"][0]["text"], "Hello world");
        assert_eq!(msg["stop_reason"], "end_turn");
        // The whole `message_delta.usage` object is merged, not just output_tokens:
        // the Responses WebSearch bridge reports final input tokens and the
        // server-tool counter there, and both must survive aggregation.
        assert_eq!(msg["usage"]["input_tokens"], 12);
        assert_eq!(msg["usage"]["output_tokens"], 7);
        assert_eq!(msg["usage"]["server_tool_use"]["web_search_requests"], 1);
    }

    #[test]
    fn anthropic_sse_aggregation_keeps_nonzero_start_usage_over_zero_delta() {
        let sse = concat!(
            "data: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"content\":[],\"usage\":{\"input_tokens\":31,\"output_tokens\":0}}}\n\n",
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"hi\"}}\n\n",
            "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"input_tokens\":0,\"output_tokens\":5}}\n\n",
            "data: {\"type\":\"message_stop\"}\n\n",
        );
        let msg = anthropic_sse_to_message_value(sse).unwrap();
        // A zero in message_delta must not clobber a non-zero message_start value,
        // otherwise a partial delta would erase already-billed input tokens.
        assert_eq!(msg["usage"]["input_tokens"], 31);
        assert_eq!(msg["usage"]["output_tokens"], 5);
    }

    #[test]
    fn anthropic_sse_aggregation_tool_use_partial_json() {
        let sse = "data: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"c\",\"content\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":0}}}\n\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"call_1\",\"name\":\"get_weather\",\"input\":{}}}\n\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"city\\\":\"}}\n\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"\\\"Tokyo\\\"}\"}}\n\n\
data: {\"type\":\"content_block_stop\",\"index\":0}\n\n\
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"tool_use\"},\"usage\":{\"output_tokens\":3}}\n\n";
        let msg = anthropic_sse_to_message_value(sse).unwrap();
        assert_eq!(msg["content"][0]["type"], "tool_use");
        assert_eq!(msg["content"][0]["name"], "get_weather");
        assert_eq!(msg["content"][0]["input"]["city"], "Tokyo");
        assert_eq!(msg["stop_reason"], "tool_use");
    }

    #[test]
    fn anthropic_sse_aggregation_tool_use_input_only_in_start() {
        // A gateway that carries the full tool `input` on content_block_start and
        // emits NO input_json_delta must still resolve the same arguments (the
        // empty-accum fallback keeps the start-carried input).
        let sse = "data: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"c\",\"content\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":0}}}\n\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"call_1\",\"name\":\"get_weather\",\"input\":{\"city\":\"Tokyo\"}}}\n\n\
data: {\"type\":\"content_block_stop\",\"index\":0}\n\n\
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"tool_use\"},\"usage\":{\"output_tokens\":3}}\n\n";
        let msg = anthropic_sse_to_message_value(sse).unwrap();
        assert_eq!(msg["content"][0]["type"], "tool_use");
        assert_eq!(msg["content"][0]["name"], "get_weather");
        // Identical to the deltas-only case above — neither path may drop start input.
        assert_eq!(msg["content"][0]["input"]["city"], "Tokyo");
        assert_eq!(msg["stop_reason"], "tool_use");
    }

    #[test]
    fn anthropic_sse_aggregation_missing_message_start_errors() {
        let sse = "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n";
        assert!(anthropic_sse_to_message_value(sse).is_err());
    }

    #[test]
    fn anthropic_sse_aggregation_error_event_errors() {
        let sse = "data: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"overloaded\"}}\n\n";
        assert!(anthropic_sse_to_message_value(sse).is_err());
    }

    #[test]
    fn anthropic_sse_aggregation_tolerates_missing_trailing_blank_line() {
        // The last event missing a trailing blank line (truncated stream) should still be processed.
        let sse = "data: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"c\",\"content\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":0}}}\n\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"hi\"}}\n\n\
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":2}}";
        let msg = anthropic_sse_to_message_value(sse).unwrap();
        assert_eq!(msg["stop_reason"], "end_turn");
        assert_eq!(msg["usage"]["output_tokens"], 2);
    }

    #[test]
    fn anthropic_sse_aggregation_truncated_output_is_marked_max_tokens() {
        let sse = "data: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"content\":[]}}\n\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"partial\"}}\n\n";
        let msg = anthropic_sse_to_message_value(sse).unwrap();
        // Partial content is preserved, but the truncation must be visible to the
        // client rather than surfacing as a normal completed response.
        assert_eq!(msg["content"][0]["text"], "partial");
        assert_eq!(msg["stop_reason"], "max_tokens");
    }

    #[test]
    fn anthropic_sse_aggregation_truncated_without_output_errors() {
        let sse =
            "data: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"content\":[]}}\n\n";
        assert!(anthropic_sse_to_message_value(sse).is_err());
    }

    #[test]
    fn anthropic_sse_aggregation_non_object_content_block_does_not_panic() {
        // A malformed upstream can send a non-object `content_block`; the index
        // assignment on the next delta would have panicked before the shape guard.
        let sse = concat!(
            "data: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"content\":[]}}\n\n",
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":[1]}\n\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"x\"}}\n\n",
            "data: {\"type\":\"message_stop\"}\n\n",
        );
        let msg = anthropic_sse_to_message_value(sse)
            .expect("aggregation must not panic on a non-object content_block");
        assert_eq!(msg["content"][0]["text"], json!("x"));
        // Not panicking is only half of it: the sanitized block must still carry a
        // `type`, because downstream consumers match on it and silently drop
        // anything they do not recognise — the client would otherwise get an empty
        // response with no indication that the text was thrown away.
        assert_eq!(msg["content"][0]["type"], json!("text"));
    }

    #[test]
    fn anthropic_sse_aggregation_non_object_message_errors_not_panic() {
        // A malformed upstream can send a scalar `message`; the later
        // `message["content"] = …` would have panicked before the shape guard.
        let sse = concat!(
            "data: {\"type\":\"message_start\",\"message\":\"oops\"}\n\n",
            "data: {\"type\":\"message_stop\"}\n\n",
        );
        assert!(anthropic_sse_to_message_value(sse).is_err());
    }

    #[test]
    fn upstream_sse_response_always_uses_streaming_path() {
        assert!(should_use_claude_transform_streaming(
            false,
            true,
            "openai_chat",
            false,
        ));
    }

    #[test]
    fn non_streaming_response_stays_non_streaming_for_regular_openai_responses() {
        assert!(!should_use_claude_transform_streaming(
            false,
            false,
            "openai_responses",
            false,
        ));
    }

    #[test]
    fn responses_sse_to_response_value_collects_output_items() {
        let sse = r#"event: response.output_item.done
data: {"type":"response.output_item.done","item":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"hello"}]}}

event: response.completed
data: {"type":"response.completed","response":{"id":"resp_1","status":"completed","model":"gpt-5.4","output":[],"usage":{"input_tokens":10,"output_tokens":2}}}

"#;

        let response = responses_sse_to_response_value(sse).unwrap();

        assert_eq!(response["id"], "resp_1");
        assert_eq!(response["output"][0]["type"], "message");
        assert_eq!(response["output"][0]["content"][0]["text"], "hello");
    }

    #[test]
    fn responses_sse_to_response_value_handles_crlf_delimiters() {
        // 真实 HTTP SSE 按规范使用 \r\n\r\n 分隔事件；take_sse_block 必须同时处理两种分隔符，
        // 否则此路径在任何标准上游（含 Codex OAuth HTTPS 后端）下都会 TransformError。
        let sse = "event: response.output_item.done\r\n\
data: {\"type\":\"response.output_item.done\",\"item\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"hi\"}]}}\r\n\
\r\n\
event: response.completed\r\n\
data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_crlf\",\"status\":\"completed\",\"model\":\"gpt-5.4\",\"output\":[],\"usage\":{\"input_tokens\":5,\"output_tokens\":1}}}\r\n\
\r\n";

        let response = responses_sse_to_response_value(sse).unwrap();

        assert_eq!(response["id"], "resp_crlf");
        assert_eq!(response["output"][0]["type"], "message");
        assert_eq!(response["output"][0]["content"][0]["text"], "hi");
    }

    #[test]
    fn responses_sse_to_response_value_returns_err_on_response_failed() {
        let sse = "event: response.failed\n\
data: {\"type\":\"response.failed\",\"response\":{\"error\":{\"message\":\"upstream blew up\"}}}\n\n";

        let err = responses_sse_to_response_value(sse).unwrap_err();
        match err {
            ProxyError::TransformError(msg) => assert!(msg.contains("upstream blew up")),
            other => panic!("expected TransformError, got {other:?}"),
        }
    }

    #[test]
    fn responses_sse_to_response_value_errors_when_no_completed_event() {
        let sse = "event: response.output_item.done\n\
data: {\"type\":\"response.output_item.done\",\"item\":{\"type\":\"message\"}}\n\n";

        assert!(responses_sse_to_response_value(sse).is_err());
    }

    #[test]
    fn codex_proxy_forward_error_includes_context_and_cause() {
        let error = ProxyError::ForwardFailed("连接失败: dns lookup failed".to_string());
        let body = codex_proxy_error_json("DeepSeek", "deepseek-chat", "/responses", &error);

        let message = body["error"]["message"].as_str().unwrap();
        assert!(message.contains("CC Switch local proxy failed"));
        assert!(message.contains("DeepSeek"));
        assert!(message.contains("deepseek-chat"));
        assert!(message.contains("/responses"));
        assert!(message.contains("dns lookup failed"));
        assert_eq!(body["error"]["code"], "cc_switch_forward_failed");
        assert_eq!(body["error"]["provider"], "DeepSeek");
        assert_eq!(body["error"]["model"], "deepseek-chat");
    }

    #[test]
    fn codex_proxy_upstream_error_normalizes_nonstandard_body() {
        let error = ProxyError::UpstreamError {
            status: 502,
            body: Some(
                r#"{"base_resp":{"status_code":2013,"status_msg":"upstream gateway failed"}}"#
                    .to_string(),
            ),
        };
        let body = codex_proxy_error_json("MiniMax", "abab6.5s", "/responses", &error);

        let message = body["error"]["message"].as_str().unwrap();
        assert!(message.contains("upstream_status: HTTP 502"));
        assert!(message.contains("upstream gateway failed"));
        assert_eq!(body["error"]["code"], 2013);
        assert_eq!(body["error"]["upstream_status"], 502);
    }

    #[test]
    fn codex_proxy_413_points_to_upstream_not_local_proxy() {
        // 模拟上游渠道商 nginx 因 client_max_body_size 返回的 413 HTML 页面
        // （见 issue #666：长上下文 / 大图 / 大日志撞上游体积上限）
        let error = ProxyError::UpstreamError {
            status: 413,
            body: Some(
                "<html>\r\n<head><title>413 Request Entity Too Large</title></head>\r\n\
                 <body>\r\n<center><h1>413 Request Entity Too Large</h1></center>\r\n\
                 <hr><center>nginx/1.29.6</center>\r\n</body>\r\n</html>"
                    .to_string(),
            ),
        };
        let body = codex_proxy_error_json("HCAI", "gpt-5.5", "/responses", &error);

        let message = body["error"]["message"].as_str().unwrap();
        // 不再误导成「本地代理失败」
        assert!(!message.contains("CC Switch local proxy failed"));
        // 明确指向上游 + 体积超限 + 可操作指引
        assert!(message.contains("413"));
        assert!(message.to_lowercase().contains("upstream"));
        assert!(message.contains("/compact"));
        // 关键：不把整段 nginx HTML 回显给用户
        assert!(!message.contains("<html>"));
        assert!(!message.contains("nginx/1.29.6"));
        // 结构化字段仍然保留，便于程序化消费 / UI 呈现
        assert_eq!(body["error"]["upstream_status"], 413);
        assert_eq!(body["error"]["provider"], "HCAI");
        assert_eq!(body["error"]["model"], "gpt-5.5");
        assert_eq!(body["error"]["endpoint"], "/responses");
    }

    // chat_error_to_response_error 的回归测试。上游把函数和这些测试放在
    // proxy/providers/transform_codex_chat.rs（Codex Chat 路由栈，fork 暂未移植）；
    // 函数内联到本文件后，测试一并随迁。

    #[test]
    fn chat_error_to_response_error_normalizes_standard_openai_shape() {
        let input = json!({
            "error": {
                "message": "Invalid API key",
                "type": "invalid_request_error",
                "code": "invalid_api_key",
                "param": "api_key"
            }
        });

        let result = chat_error_to_response_error(Some(&input));

        assert_eq!(result["error"]["message"], "Invalid API key");
        assert_eq!(result["error"]["type"], "invalid_request_error");
        assert_eq!(result["error"]["code"], "invalid_api_key");
        assert_eq!(result["error"]["param"], "api_key");
    }

    #[test]
    fn chat_error_to_response_error_normalizes_minimax_base_resp() {
        // MiniMax 把错误塞在 base_resp 里，code 是数字而不是字符串
        let input = json!({
            "base_resp": {
                "status_code": 2013,
                "status_msg": "invalid params, chat content has invalid message role: system"
            }
        });

        let result = chat_error_to_response_error(Some(&input));

        assert_eq!(
            result["error"]["message"],
            "invalid params, chat content has invalid message role: system"
        );
        assert_eq!(result["error"]["code"], 2013);
        // type 没有显式给出，应该回落到 upstream_error
        assert_eq!(result["error"]["type"], "upstream_error");
    }

    #[test]
    fn chat_error_to_response_error_handles_plain_text_body() {
        let input = json!("Upstream timeout");

        let result = chat_error_to_response_error(Some(&input));

        assert_eq!(result["error"]["message"], "Upstream timeout");
        assert_eq!(result["error"]["type"], "upstream_error");
        assert!(result["error"]["code"].is_null());
        assert!(result["error"]["param"].is_null());
    }

    #[test]
    fn chat_error_to_response_error_handles_missing_body() {
        let result = chat_error_to_response_error(None);

        assert_eq!(
            result["error"]["message"],
            "Upstream returned an empty error response"
        );
        assert_eq!(result["error"]["type"], "upstream_error");
    }

    #[test]
    fn chat_error_to_response_error_falls_back_to_detail_field() {
        // 部分中转把错误塞在顶层 detail 字段（OpenAI 兼容层常见）
        let input = json!({
            "detail": "rate limit exceeded"
        });

        let result = chat_error_to_response_error(Some(&input));

        assert_eq!(result["error"]["message"], "rate limit exceeded");
        assert_eq!(result["error"]["type"], "upstream_error");
    }
}
