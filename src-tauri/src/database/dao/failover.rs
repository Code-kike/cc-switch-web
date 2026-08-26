//! 故障转移队列 DAO
//!
//! 管理代理模式下的故障转移队列（基于 providers 表的 in_failover_queue 字段）

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::provider::Provider;
use serde::{Deserialize, Serialize};

/// 故障转移队列条目（简化版，用于前端展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailoverQueueItem {
    pub provider_id: String,
    pub provider_name: String,
    pub sort_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_notes: Option<String>,
}

impl Database {
    /// 获取故障转移队列（按 sort_index 排序）
    pub fn get_failover_queue(&self, app_type: &str) -> Result<Vec<FailoverQueueItem>, AppError> {
        let conn = lock_conn!(self.conn);

        let mut stmt = conn
            .prepare(
                "SELECT id, name, sort_index, notes
                 FROM providers
                 WHERE app_type = ?1 AND in_failover_queue = 1
                 ORDER BY COALESCE(sort_index, 999999), id ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let items = stmt
            .query_map([app_type], |row| {
                Ok(FailoverQueueItem {
                    provider_id: row.get(0)?,
                    provider_name: row.get(1)?,
                    sort_index: row.get(2)?,
                    provider_notes: row.get(3)?,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(items)
    }

    /// 获取故障转移队列中的供应商（完整 Provider 信息，按顺序）
    pub fn get_failover_providers(&self, app_type: &str) -> Result<Vec<Provider>, AppError> {
        let all_providers = self.get_all_providers(app_type)?;

        let result: Vec<Provider> = all_providers
            .into_values()
            .filter(|p| p.in_failover_queue)
            .collect();

        Ok(result)
    }

    /// 添加供应商到故障转移队列
    ///
    /// Deliberately unguarded: `ProxyService::set_auto_failover_enabled` calls
    /// this *after* a successful `switch_proxy_target` (its "FIX 4" atomicity
    /// comment), so rejecting here would leave the switch applied while
    /// `auto_failover_enabled` stays unpersisted. That path instead validates the
    /// candidate up front with [`Self::ensure_provider_supports_failover`], so
    /// the rule is enforced before any state changes. Codex Official account
    /// cards are additionally kept out of retry by
    /// `ProviderRouter::select_providers_with_config`, and out of the candidate
    /// list by `get_available_providers_for_failover`.
    pub fn add_to_failover_queue(&self, app_type: &str, provider_id: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);

        conn.execute(
            "UPDATE providers SET in_failover_queue = 1 WHERE id = ?1 AND app_type = ?2",
            rusqlite::params![provider_id, app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// Reject a provider the router refuses to retry.
    ///
    /// Single definition of the queue-membership rule, shared by
    /// [`Self::add_to_failover_queue_checked`] (user-facing add) and
    /// `ProxyService::set_auto_failover_enabled` (pre-switch validation of the
    /// auto-adopted P1). Keeping one predicate call site per rule is what stops
    /// the desktop command, the web handler and the enable path from drifting.
    pub fn ensure_provider_supports_failover(
        &self,
        app_type: &str,
        provider_id: &str,
    ) -> Result<Provider, AppError> {
        let provider = self
            .get_provider_by_id(provider_id, app_type)?
            .ok_or_else(|| AppError::Message(format!("供应商不存在: {provider_id}")))?;
        if !provider.supports_failover(app_type) {
            return Err(AppError::Message(
                "Codex Official 账号卡不支持自动故障转移".to_string(),
            ));
        }
        Ok(provider)
    }

    /// Add to the queue, rejecting rows the router refuses to retry.
    ///
    /// This is the entry point for user requests (desktop command + web
    /// handler), so both runtimes enforce the same rule from one definition.
    /// The internal auto-add in `ProxyService::set_auto_failover_enabled`
    /// deliberately keeps using the unvalidated `add_to_failover_queue`: it runs
    /// after a committed `switch_proxy_target`, where an error would leave the
    /// switch applied and `auto_failover_enabled` unpersisted. It validates the
    /// same rule via [`Self::ensure_provider_supports_failover`] before the
    /// switch instead.
    pub fn add_to_failover_queue_checked(
        &self,
        app_type: &str,
        provider_id: &str,
    ) -> Result<(), AppError> {
        self.ensure_provider_supports_failover(app_type, provider_id)?;

        self.add_to_failover_queue(app_type, provider_id)
    }

    /// 从故障转移队列中移除供应商
    pub fn remove_from_failover_queue(
        &self,
        app_type: &str,
        provider_id: &str,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);

        // 1. 从队列中移除
        conn.execute(
            "UPDATE providers SET in_failover_queue = 0 WHERE id = ?1 AND app_type = ?2",
            rusqlite::params![provider_id, app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 2. 清除该供应商的健康状态（退出队列后不再需要健康监控）
        conn.execute(
            "DELETE FROM provider_health WHERE provider_id = ?1 AND app_type = ?2",
            rusqlite::params![provider_id, app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        log::info!("已从故障转移队列移除供应商 {provider_id} ({app_type}), 并清除其健康状态");

        Ok(())
    }

    /// 清空故障转移队列
    pub fn clear_failover_queue(&self, app_type: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);

        conn.execute(
            "UPDATE providers SET in_failover_queue = 0 WHERE app_type = ?1",
            [app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// 检查供应商是否在故障转移队列中
    pub fn is_in_failover_queue(
        &self,
        app_type: &str,
        provider_id: &str,
    ) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);

        let in_queue: bool = conn
            .query_row(
                "SELECT in_failover_queue FROM providers WHERE id = ?1 AND app_type = ?2",
                rusqlite::params![provider_id, app_type],
                |row| row.get(0),
            )
            .unwrap_or(false);

        Ok(in_queue)
    }

    /// 获取可添加到故障转移队列的供应商（不在队列中的）
    ///
    /// Codex Official account cards are filtered out: the router refuses to
    /// retry them, so offering one as a queue candidate would only produce an
    /// entry that is silently skipped at request time.
    pub fn get_available_providers_for_failover(
        &self,
        app_type: &str,
    ) -> Result<Vec<Provider>, AppError> {
        let all_providers = self.get_all_providers(app_type)?;

        let available: Vec<Provider> = all_providers
            .into_values()
            .filter(|p| !p.in_failover_queue)
            .filter(|p| p.supports_failover(app_type))
            .collect();

        Ok(available)
    }
}
