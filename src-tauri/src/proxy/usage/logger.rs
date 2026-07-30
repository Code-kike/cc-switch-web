//! Usage Logger - 记录 API 请求使用情况

use super::calculator::{CostBreakdown, CostCalculator, ModelPricing};
use super::parser::TokenUsage;
use crate::database::{Database, PRICING_SOURCE_REQUEST, PRICING_SOURCE_RESPONSE};
use crate::error::AppError;
use crate::services::sql_helpers::{INPUT_TOKEN_SEMANTICS_FRESH, INPUT_TOKEN_SEMANTICS_TOTAL};
use crate::services::usage_stats::find_model_pricing_row;
use rust_decimal::Decimal;
use std::{str::FromStr, time::SystemTime};

/// 请求日志
#[derive(Debug, Clone)]
pub struct RequestLog {
    pub request_id: String,
    pub provider_id: String,
    pub app_type: String,
    pub model: String,
    pub request_model: String,
    pub pricing_model: String,
    pub usage: TokenUsage,
    pub cost: Option<CostBreakdown>,
    pub latency_ms: u64,
    pub first_token_ms: Option<u64>,
    pub status_code: u16,
    pub error_message: Option<String>,
    pub session_id: Option<String>,
    /// 供应商类型 (claude, claude_auth, codex, gemini, gemini_cli, openrouter)
    pub provider_type: Option<String>,
    /// 是否为流式请求
    pub is_streaming: bool,
    /// 成本倍数
    pub cost_multiplier: String,
}

/// 使用量记录器
pub struct UsageLogger<'a> {
    db: &'a Database,
}

impl<'a> UsageLogger<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// 记录成功的请求
    pub fn log_request(&self, log: &RequestLog) -> Result<(), AppError> {
        let conn = crate::database::lock_conn!(self.db.conn);

        let (input_cost, output_cost, cache_read_cost, cache_creation_cost, total_cost) =
            if let Some(cost) = &log.cost {
                (
                    cost.input_cost.to_string(),
                    cost.output_cost.to_string(),
                    cost.cache_read_cost.to_string(),
                    cost.cache_creation_cost.to_string(),
                    cost.total_cost.to_string(),
                )
            } else {
                (
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                )
            };

        let created_at = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or_else(|e| {
                log::warn!("SystemTime is before UNIX_EPOCH, falling back to 0: {e}");
                0
            });
        let input_token_semantics =
            if matches!(log.app_type.as_str(), "codex" | "gemini" | "grokbuild") {
                INPUT_TOKEN_SEMANTICS_TOTAL
            } else {
                INPUT_TOKEN_SEMANTICS_FRESH
            };

        // L29 (investigated, intentional cross-source dedup — NOT last-writer data loss):
        // `request_id` here is `TokenUsage::dedup_request_id()`, which is
        // `session:{message_id}` whenever a message id is available — the SAME key the
        // session-log sync writer uses (services/session_usage*.rs). A proxy row and a
        // session-log row sharing `session:{message_id}` describe the *same* assistant
        // message observed via two channels, so collapsing them to one row is correct
        // (folding the source into the key, or keeping both, would DOUBLE-COUNT a single
        // billable message). The proxy observation is authoritative (real latency, status,
        // streaming, cost), so `INSERT OR REPLACE` lets a proxy row supersede an earlier
        // session-log estimate for the same message; the reverse direction is already
        // guarded because the session writer uses `INSERT OR IGNORE` + a content dedup
        // (`should_skip_session_insert`) and never clobbers a proxy row.
        conn.execute(
            "INSERT OR REPLACE INTO proxy_request_logs (
                request_id, provider_id, app_type, model, request_model, pricing_model,
                input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                input_token_semantics,
                input_cost_usd, output_cost_usd, cache_read_cost_usd, cache_creation_cost_usd, total_cost_usd,
                latency_ms, first_token_ms, status_code, error_message, session_id,
                provider_type, is_streaming, cost_multiplier, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25)",
            rusqlite::params![
                log.request_id,
                log.provider_id,
                log.app_type,
                log.model,
                log.request_model,
                log.pricing_model,
                log.usage.input_tokens,
                log.usage.output_tokens,
                log.usage.cache_read_tokens,
                log.usage.cache_creation_tokens,
                input_token_semantics,
                input_cost,
                output_cost,
                cache_read_cost,
                cache_creation_cost,
                total_cost,
                log.latency_ms as i64,
                log.first_token_ms.map(|v| v as i64),
                log.status_code as i64,
                log.error_message,
                log.session_id,
                log.provider_type,
                log.is_streaming as i64,
                log.cost_multiplier,
                created_at,
            ],
        )
        .map_err(|e| AppError::Database(format!("记录请求日志失败: {e}")))?;

        // 通知前端使用统计有更新（200ms 防抖合并，不阻塞写入路径）
        crate::usage_events::notify_log_recorded();

        Ok(())
    }

    /// 记录失败的请求
    ///
    /// 用于记录无法从上游获取 usage 信息的失败请求
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn log_error(
        &self,
        request_id: String,
        provider_id: String,
        app_type: String,
        model: String,
        status_code: u16,
        error_message: String,
        latency_ms: u64,
    ) -> Result<(), AppError> {
        let request_model = model.clone();
        let pricing_model = model.clone();
        let log = RequestLog {
            request_id,
            provider_id,
            app_type,
            model,
            request_model,
            pricing_model,
            usage: TokenUsage::default(),
            cost: None,
            latency_ms,
            first_token_ms: None,
            status_code,
            error_message: Some(error_message),
            session_id: None,
            provider_type: None,
            is_streaming: false,
            cost_multiplier: "1.0".to_string(),
        };

        self.log_request(&log)
    }

    /// 记录失败的请求（带更多上下文信息）
    ///
    /// 相比 log_error，这个方法接受更多参数以提供完整的请求上下文
    #[allow(clippy::too_many_arguments)]
    pub fn log_error_with_context(
        &self,
        request_id: String,
        provider_id: String,
        app_type: String,
        model: String,
        status_code: u16,
        error_message: String,
        latency_ms: u64,
        is_streaming: bool,
        session_id: Option<String>,
        provider_type: Option<String>,
    ) -> Result<(), AppError> {
        let request_model = model.clone();
        let pricing_model = model.clone();
        let log = RequestLog {
            request_id,
            provider_id,
            app_type,
            model,
            request_model,
            pricing_model,
            usage: TokenUsage::default(),
            cost: None,
            latency_ms,
            first_token_ms: None,
            status_code,
            error_message: Some(error_message),
            session_id,
            provider_type,
            is_streaming,
            cost_multiplier: "1.0".to_string(),
        };

        self.log_request(&log)
    }

    /// 获取模型定价
    pub fn get_model_pricing(&self, model_id: &str) -> Result<Option<ModelPricing>, AppError> {
        let conn = crate::database::lock_conn!(self.db.conn);
        let row = find_model_pricing_row(&conn, model_id)?;
        match row {
            Some((input, output, cache_read, cache_creation)) => {
                ModelPricing::from_strings(&input, &output, &cache_read, &cache_creation)
                    .map(Some)
                    .map_err(|e| AppError::Database(format!("解析定价数据失败: {e}")))
            }
            None => Ok(None),
        }
    }

    /// 获取有效的倍率与计费模式来源（供应商优先，未配置则回退全局默认）
    pub async fn resolve_pricing_config(
        &self,
        provider_id: &str,
        app_type: &str,
    ) -> (Decimal, String) {
        let default_multiplier_raw = match self.db.get_default_cost_multiplier(app_type).await {
            Ok(value) => value,
            Err(e) => {
                log::warn!("[USG-003] 获取默认倍率失败 (app_type={app_type}): {e}");
                "1".to_string()
            }
        };
        let default_multiplier = match Decimal::from_str(&default_multiplier_raw) {
            Ok(value) => value,
            Err(e) => {
                log::warn!(
                    "[USG-003] 默认倍率解析失败 (app_type={app_type}): {default_multiplier_raw} - {e}"
                );
                Decimal::from(1)
            }
        };

        let default_pricing_source_raw = match self.db.get_pricing_model_source(app_type).await {
            Ok(value) => value,
            Err(e) => {
                log::warn!("[USG-003] 获取默认计费模式失败 (app_type={app_type}): {e}");
                PRICING_SOURCE_RESPONSE.to_string()
            }
        };
        let default_pricing_source = if matches!(
            default_pricing_source_raw.as_str(),
            PRICING_SOURCE_RESPONSE | PRICING_SOURCE_REQUEST
        ) {
            default_pricing_source_raw
        } else {
            log::warn!(
                "[USG-003] 默认计费模式无效 (app_type={app_type}): {default_pricing_source_raw}"
            );
            PRICING_SOURCE_RESPONSE.to_string()
        };

        let provider = self
            .db
            .get_provider_by_id(provider_id, app_type)
            .ok()
            .flatten();

        let (provider_multiplier, provider_pricing_source) = provider
            .as_ref()
            .and_then(|p| p.meta.as_ref())
            .map(|meta| {
                (
                    meta.cost_multiplier.as_deref(),
                    meta.pricing_model_source.as_deref(),
                )
            })
            .unwrap_or((None, None));

        let cost_multiplier = match provider_multiplier {
            Some(value) => match Decimal::from_str(value) {
                Ok(parsed) => parsed,
                Err(e) => {
                    log::warn!(
                        "[USG-003] 供应商倍率解析失败 (provider_id={provider_id}): {value} - {e}"
                    );
                    default_multiplier
                }
            },
            None => default_multiplier,
        };

        let pricing_model_source = match provider_pricing_source {
            Some(value) if matches!(value, PRICING_SOURCE_RESPONSE | PRICING_SOURCE_REQUEST) => {
                value.to_string()
            }
            Some(value) => {
                log::warn!("[USG-003] 供应商计费模式无效 (provider_id={provider_id}): {value}");
                default_pricing_source.clone()
            }
            None => default_pricing_source.clone(),
        };

        (cost_multiplier, pricing_model_source)
    }

    /// 计算并记录请求
    #[allow(clippy::too_many_arguments)]
    pub fn log_with_calculation(
        &self,
        request_id: String,
        provider_id: String,
        app_type: String,
        model: String,
        request_model: String,
        pricing_model: String,
        usage: TokenUsage,
        cost_multiplier: Decimal,
        latency_ms: u64,
        first_token_ms: Option<u64>,
        status_code: u16,
        session_id: Option<String>,
        provider_type: Option<String>,
        is_streaming: bool,
    ) -> Result<(), AppError> {
        let pricing = self.get_model_pricing(&pricing_model)?;

        if pricing.is_none() {
            log::warn!("[USG-002] 模型定价未找到，成本将记录为 0: {pricing_model}");
        }

        let cost = CostCalculator::try_calculate_for_app(
            &app_type,
            &usage,
            pricing.as_ref(),
            cost_multiplier,
        );

        let log = RequestLog {
            request_id,
            provider_id,
            app_type,
            model,
            request_model,
            pricing_model,
            usage,
            cost,
            latency_ms,
            first_token_ms,
            status_code,
            error_message: None,
            session_id,
            provider_type,
            is_streaming,
            cost_multiplier: cost_multiplier.to_string(),
        };

        self.log_request(&log)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_request() -> Result<(), AppError> {
        let db = Database::memory()?;

        // 插入测试定价
        {
            let conn = crate::database::lock_conn!(db.conn);
            conn.execute(
                "INSERT INTO model_pricing (model_id, display_name, input_cost_per_million, output_cost_per_million)
                 VALUES ('test-model', 'Test Model', '3.0', '15.0')",
                [],
            )
            .unwrap();
        }

        let logger = UsageLogger::new(&db);

        let usage = TokenUsage {
            input_tokens: 1000,
            output_tokens: 500,
            cache_read_tokens: 0,
            cache_creation_tokens: 0,
            model: None,
            message_id: None,
        };

        logger.log_with_calculation(
            "req-123".to_string(),
            "provider-1".to_string(),
            "claude".to_string(),
            "test-model".to_string(),
            "req-model".to_string(),
            "test-model".to_string(),
            usage,
            Decimal::from(1),
            100,
            None,
            200,
            None,
            Some("claude".to_string()),
            false,
        )?;

        // 验证记录已插入
        let conn = crate::database::lock_conn!(db.conn);
        let (count, request_model, pricing_model): (i64, String, String) = conn
            .query_row(
                "SELECT COUNT(*), request_model, pricing_model FROM proxy_request_logs WHERE request_id = 'req-123'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(count, 1);
        assert_eq!(request_model, "req-model");
        assert_eq!(pricing_model, "test-model");
        Ok(())
    }

    #[test]
    fn test_log_error() -> Result<(), AppError> {
        let db = Database::memory()?;
        let logger = UsageLogger::new(&db);

        logger.log_error(
            "req-error".to_string(),
            "provider-1".to_string(),
            "claude".to_string(),
            "unknown-model".to_string(),
            500,
            "Internal Server Error".to_string(),
            50,
        )?;

        // 验证错误记录已插入
        let conn = crate::database::lock_conn!(db.conn);
        let (status, error): (i64, Option<String>) = conn
            .query_row(
                "SELECT status_code, error_message FROM proxy_request_logs WHERE request_id = 'req-error'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(status, 500);
        assert_eq!(error, Some("Internal Server Error".to_string()));
        Ok(())
    }

    /// L29 regression: a proxy row and a session-log row sharing the same
    /// `session:{message_id}` key are the SAME logical message; they must
    /// collapse to ONE authoritative proxy row (deliberate cross-source dedup),
    /// never two rows (double count) nor leave the proxy observation lost.
    #[test]
    fn test_proxy_row_supersedes_session_log_row_for_same_message() -> Result<(), AppError> {
        let db = Database::memory()?;
        let request_id = "session:msg_dedup";

        // Pre-existing session-log row (latency 0 estimate) for this message.
        {
            let conn = crate::database::lock_conn!(db.conn);
            conn.execute(
                "INSERT INTO proxy_request_logs (
                    request_id, provider_id, app_type, model, request_model,
                    input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                    total_cost_usd, latency_ms, status_code, created_at, data_source
                ) VALUES (?, '_session', 'claude', 'claude-sonnet-4-5', 'claude-sonnet-4-5',
                          100, 20, 0, 0, '0.05', 0, 200, 1000, 'session_log')",
                rusqlite::params![request_id],
            )
            .unwrap();
        }

        // The proxy observes the SAME message (real latency) under the shared id.
        let logger = UsageLogger::new(&db);
        let log = RequestLog {
            request_id: request_id.to_string(),
            provider_id: "real-provider".to_string(),
            app_type: "claude".to_string(),
            model: "claude-sonnet-4-5".to_string(),
            request_model: "claude-sonnet-4-5".to_string(),
            pricing_model: "claude-sonnet-4-5".to_string(),
            usage: TokenUsage {
                input_tokens: 100,
                output_tokens: 20,
                cache_read_tokens: 0,
                cache_creation_tokens: 0,
                model: None,
                message_id: Some("msg_dedup".to_string()),
            },
            cost: None,
            latency_ms: 1234,
            first_token_ms: Some(50),
            status_code: 200,
            error_message: None,
            session_id: Some("sess".to_string()),
            provider_type: Some("claude".to_string()),
            is_streaming: true,
            cost_multiplier: "1.0".to_string(),
        };
        logger.log_request(&log)?;

        let conn = crate::database::lock_conn!(db.conn);
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM proxy_request_logs WHERE request_id = ?1",
            rusqlite::params![request_id],
            |row| row.get(0),
        )?;
        assert_eq!(
            count, 1,
            "shared session id must collapse to one row, not double-count"
        );

        let (latency, provider_id, data_source): (i64, String, String) = conn.query_row(
            "SELECT latency_ms, provider_id, data_source FROM proxy_request_logs WHERE request_id = ?1",
            rusqlite::params![request_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        // The authoritative proxy row (real latency / provider) supersedes the estimate.
        assert_eq!(
            latency, 1234,
            "proxy observation must win for the same message"
        );
        assert_eq!(provider_id, "real-provider");
        // The proxy logger leaves data_source at its 'proxy' default, so the row
        // is now a proxy-source row — the session_log estimate was superseded,
        // not duplicated.
        assert_eq!(
            data_source, "proxy",
            "proxy row must supersede the session_log estimate"
        );

        Ok(())
    }

    #[test]
    fn grokbuild_logs_total_input_token_semantics() -> Result<(), AppError> {
        let db = Database::memory()?;
        let logger = UsageLogger::new(&db);
        let log = RequestLog {
            request_id: "grok-semantics".to_string(),
            provider_id: "grok-provider".to_string(),
            app_type: "grokbuild".to_string(),
            model: "grok-4.5".to_string(),
            request_model: "grok-4.5".to_string(),
            pricing_model: String::new(),
            usage: TokenUsage::default(),
            cost: None,
            latency_ms: 1,
            first_token_ms: None,
            status_code: 200,
            error_message: None,
            session_id: None,
            provider_type: Some("grokbuild".to_string()),
            is_streaming: false,
            cost_multiplier: "1".to_string(),
        };

        logger.log_request(&log)?;

        let conn = crate::database::lock_conn!(db.conn);
        let semantics: i64 = conn.query_row(
            "SELECT input_token_semantics FROM proxy_request_logs WHERE request_id = 'grok-semantics'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(semantics, INPUT_TOKEN_SEMANTICS_TOTAL);
        Ok(())
    }
}
