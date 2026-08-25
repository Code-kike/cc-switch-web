//! 供应商路由器模块
//!
//! 负责选择和管理代理目标供应商，实现智能故障转移

use crate::app_config::AppType;
use crate::database::Database;
use crate::error::AppError;
use crate::provider::Provider;
use crate::proxy::circuit_breaker::{AllowResult, CircuitBreaker, CircuitBreakerConfig};
use crate::proxy::types::FailoverStrategy;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 供应商路由器
pub struct ProviderRouter {
    /// 数据库连接
    db: Arc<Database>,
    /// 熔断器管理器 - key 格式: "app_type:provider_id"
    circuit_breakers: Arc<RwLock<HashMap<String, Arc<CircuitBreaker>>>>,
}

impl ProviderRouter {
    /// 创建新的供应商路由器
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 选择可用的供应商（支持故障转移）
    ///
    /// 返回按优先级排序的可用供应商列表：
    /// - 故障转移关闭时：仅返回当前供应商
    /// - 故障转移开启时：仅使用故障转移队列
    ///   - `Sequential`（默认，上游语义）：按队列顺序依次尝试（P1 → P2 → ...）
    ///   - `Random`（D1/D2，PRD 06-11）：当前供应商优先（粘性直到失败），
    ///     其余熔断可用的队列成员随机洗牌——失败后即随机重选下一个
    ///
    /// 自行读取 `proxy_config`（开关 + 策略）。热路径上 `RequestContext::new`
    /// 已加载同一份 `AppProxyConfig`，应改用 `select_providers_with_config`
    /// 复用它，避免每请求重复读库（M1）。生产热路径已切到 `_with_config` 变体，
    /// 此便捷入口目前仅供测试与未来的"无预载配置"调用方使用。
    #[allow(dead_code)]
    pub async fn select_providers(&self, app_type: &str) -> Result<Vec<Provider>, AppError> {
        // 检查该应用的自动故障转移开关与选择策略（从 proxy_config 表读取）
        let (auto_failover_enabled, failover_strategy) =
            match self.db.get_proxy_config_for_app(app_type).await {
                Ok(config) => (config.auto_failover_enabled, config.failover_strategy),
                Err(e) => {
                    log::error!("[{app_type}] 读取 proxy_config 失败: {e}，默认禁用故障转移");
                    (false, FailoverStrategy::Sequential)
                }
            };

        self.select_providers_with_config(app_type, auto_failover_enabled, failover_strategy)
            .await
    }

    /// 选择可用供应商（复用调用方已加载的 `proxy_config` 字段，M1）。
    ///
    /// 与 `select_providers` 行为完全一致，只是把"故障转移开关 + 策略"作为参数
    /// 传入而不是再读一次库。热路径（`RequestContext::new`）已在创建上下文时
    /// 加载了同一应用的 `AppProxyConfig`，直接复用可省去每请求的一次冗余读库。
    pub async fn select_providers_with_config(
        &self,
        app_type: &str,
        auto_failover_enabled: bool,
        failover_strategy: FailoverStrategy,
    ) -> Result<Vec<Provider>, AppError> {
        let mut result = Vec::new();
        let mut total_providers = 0usize;
        let mut circuit_open_count = 0usize;
        let current_id = self.effective_current_provider_id(app_type);
        let current_provider = current_id
            .as_deref()
            .map(|id| self.db.get_provider_by_id(id, app_type))
            .transpose()?
            .flatten();

        if auto_failover_enabled
            && current_provider
                .as_ref()
                .is_some_and(|provider| !provider.supports_failover(app_type))
        {
            // A selected Codex Official account is an explicit account choice.
            // Keep it as a single route even if an old failover setting remains
            // enabled; retrying would reuse its inbound token for another card.
            total_providers = 1;
            result.push(current_provider.expect("checked above"));
        } else if auto_failover_enabled {
            // 故障转移开启：仅使用故障转移队列（基准顺序 = 队列顺序 P1 → P2 → ...）
            let all_providers = self.db.get_all_providers(app_type)?;

            // 使用 DAO 返回的排序结果，确保和前端展示一致
            let ordered_ids: Vec<String> = self
                .db
                .get_failover_queue(app_type)?
                .into_iter()
                .map(|item| item.provider_id)
                .collect();

            for provider_id in ordered_ids {
                let Some(provider) = all_providers.get(&provider_id).cloned() else {
                    continue;
                };
                // A stale queue entry for a Codex Official card is skipped
                // rather than counted, so it can neither be retried nor make
                // the "all providers tripped" branch fire.
                if !provider.supports_failover(app_type) {
                    continue;
                }
                total_providers += 1;

                let circuit_key = format!("{app_type}:{}", provider.id);
                let breaker = self.get_or_create_circuit_breaker(&circuit_key).await;

                if breaker.is_available().await {
                    result.push(provider);
                } else {
                    circuit_open_count += 1;
                }
            }

            // Random 策略（D2 粘性直到失败）：当前供应商排首位，其余洗牌。
            // Sequential 路径不做任何重排——与上游行为逐字节一致。
            if failover_strategy == FailoverStrategy::Random && result.len() > 1 {
                Self::apply_random_strategy(
                    &mut result,
                    current_id.as_deref(),
                    &mut rand::thread_rng(),
                );
            }
        } else {
            // 故障转移关闭：仅使用当前供应商，跳过熔断器检查
            if let Some(current) = current_provider {
                total_providers = 1;
                result.push(current);
            }
        }

        if result.is_empty() {
            if total_providers > 0 && circuit_open_count == total_providers {
                log::warn!("[{app_type}] [FO-004] 所有供应商均已熔断");
                return Err(AppError::AllProvidersCircuitOpen);
            } else {
                log::warn!("[{app_type}] [FO-005] 未配置供应商");
                return Err(AppError::NoProvidersConfigured);
            }
        }

        Ok(result)
    }

    /// 解析该应用"当前供应商" ID（settings 生效值优先，回退数据库 current 指针）
    fn effective_current_provider_id(&self, app_type: &str) -> Option<String> {
        AppType::from_str(app_type)
            .ok()
            .and_then(|app_enum| {
                crate::settings::get_effective_current_provider(&self.db, &app_enum)
                    .ok()
                    .flatten()
            })
            .or_else(|| self.db.get_current_provider(app_type).ok().flatten())
    }

    /// Random 策略排序（D2）：当前供应商置于候选首位（粘性），
    /// 其余可用供应商 Fisher-Yates 洗牌（失败时按洗牌顺序"随机重选"）。
    ///
    /// 当前供应商不在候选列表（不在队列或已熔断）时，整个列表洗牌；
    /// 候选集合本身不变——只改变顺序。
    /// RNG 由调用方注入，便于测试以固定种子复现。
    fn apply_random_strategy<R: rand::Rng + ?Sized>(
        providers: &mut [Provider],
        current_id: Option<&str>,
        rng: &mut R,
    ) {
        use rand::seq::SliceRandom;

        let shuffle_from = match current_id
            .and_then(|id| providers.iter().position(|provider| provider.id == id))
        {
            Some(pos) => {
                providers.swap(0, pos);
                1
            }
            None => 0,
        };

        providers[shuffle_from..].shuffle(rng);
    }

    /// 请求执行前获取熔断器“放行许可”
    ///
    /// - Closed：直接放行
    /// - Open：超时到达后切到 HalfOpen 并放行一次探测
    /// - HalfOpen：按限流规则放行探测
    ///
    /// 注意：调用方必须在请求结束后通过 `record_result()` 释放 HalfOpen 名额，
    /// 否则会导致该 Provider 长时间无法进入探测状态。
    pub async fn allow_provider_request(&self, provider_id: &str, app_type: &str) -> AllowResult {
        let circuit_key = format!("{app_type}:{provider_id}");
        let breaker = self.get_or_create_circuit_breaker(&circuit_key).await;
        breaker.allow_request().await
    }

    /// 记录供应商请求结果
    pub async fn record_result(
        &self,
        provider_id: &str,
        app_type: &str,
        used_half_open_permit: bool,
        success: bool,
        error_msg: Option<String>,
    ) -> Result<(), AppError> {
        // 1. 按应用独立获取熔断器配置
        let failure_threshold = match self.db.get_proxy_config_for_app(app_type).await {
            Ok(app_config) => app_config.circuit_failure_threshold,
            Err(_) => 5, // 默认值
        };

        // 2. 更新熔断器状态
        let circuit_key = format!("{app_type}:{provider_id}");
        let breaker = self.get_or_create_circuit_breaker(&circuit_key).await;

        if success {
            breaker.record_success(used_half_open_permit).await;
        } else {
            breaker.record_failure(used_half_open_permit).await;
        }

        // 3. 更新数据库健康状态（使用配置的阈值）
        self.db
            .update_provider_health_with_threshold(
                provider_id,
                app_type,
                success,
                error_msg.clone(),
                failure_threshold,
            )
            .await?;

        Ok(())
    }

    /// 重置熔断器（手动恢复）
    pub async fn reset_circuit_breaker(&self, circuit_key: &str) {
        let breakers = self.circuit_breakers.read().await;
        if let Some(breaker) = breakers.get(circuit_key) {
            breaker.reset().await;
        }
    }

    /// 重置指定供应商的熔断器
    pub async fn reset_provider_breaker(&self, provider_id: &str, app_type: &str) {
        let circuit_key = format!("{app_type}:{provider_id}");
        self.reset_circuit_breaker(&circuit_key).await;
    }

    /// 仅释放 HalfOpen permit，不影响健康统计（neutral 接口）
    ///
    /// 用于整流器等场景：请求结果不应计入 Provider 健康度，
    /// 但仍需释放占用的探测名额，避免 HalfOpen 状态卡死
    pub async fn release_permit_neutral(
        &self,
        provider_id: &str,
        app_type: &str,
        used_half_open_permit: bool,
    ) {
        if !used_half_open_permit {
            return;
        }
        let circuit_key = format!("{app_type}:{provider_id}");
        let breaker = self.get_or_create_circuit_breaker(&circuit_key).await;
        breaker.release_half_open_permit();
    }

    /// 更新所有熔断器的配置（热更新）
    pub async fn update_all_configs(&self, config: CircuitBreakerConfig) {
        let breakers = self.circuit_breakers.read().await;
        for breaker in breakers.values() {
            breaker.update_config(config.clone()).await;
        }
    }

    /// 获取熔断器状态
    pub async fn get_circuit_breaker_stats(
        &self,
        provider_id: &str,
        app_type: &str,
    ) -> Option<crate::proxy::circuit_breaker::CircuitBreakerStats> {
        let circuit_key = format!("{app_type}:{provider_id}");
        let breakers = self.circuit_breakers.read().await;

        if let Some(breaker) = breakers.get(&circuit_key) {
            Some(breaker.get_stats().await)
        } else {
            None
        }
    }

    /// 获取或创建熔断器
    async fn get_or_create_circuit_breaker(&self, key: &str) -> Arc<CircuitBreaker> {
        // 先尝试读锁获取
        {
            let breakers = self.circuit_breakers.read().await;
            if let Some(breaker) = breakers.get(key) {
                return breaker.clone();
            }
        }

        // 如果不存在，获取写锁创建
        let mut breakers = self.circuit_breakers.write().await;

        // 双重检查，防止竞争条件
        if let Some(breaker) = breakers.get(key) {
            return breaker.clone();
        }

        // 从 key 中提取 app_type (格式: "app_type:provider_id")
        let app_type = key.split(':').next().unwrap_or("claude");

        // 按应用独立读取熔断器配置
        let config = match self.db.get_proxy_config_for_app(app_type).await {
            Ok(app_config) => crate::proxy::circuit_breaker::CircuitBreakerConfig {
                failure_threshold: app_config.circuit_failure_threshold,
                success_threshold: app_config.circuit_success_threshold,
                timeout_seconds: app_config.circuit_timeout_seconds as u64,
                error_rate_threshold: app_config.circuit_error_rate_threshold,
                min_requests: app_config.circuit_min_requests,
            },
            Err(_) => crate::proxy::circuit_breaker::CircuitBreakerConfig::default(),
        };

        let breaker = Arc::new(CircuitBreaker::new(config));
        breakers.insert(key.to_string(), breaker.clone());

        breaker
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use serde_json::json;
    use serial_test::serial;
    use std::env;
    use tempfile::TempDir;

    struct TempHome {
        #[allow(dead_code)]
        dir: TempDir,
        original_home: Option<String>,
        original_userprofile: Option<String>,
        original_test_home: Option<String>,
    }

    impl TempHome {
        fn new() -> Self {
            let dir = TempDir::new().expect("failed to create temp home");
            let original_home = env::var("HOME").ok();
            let original_userprofile = env::var("USERPROFILE").ok();
            let original_test_home = env::var("CC_SWITCH_TEST_HOME").ok();

            env::set_var("HOME", dir.path());
            env::set_var("USERPROFILE", dir.path());
            env::set_var("CC_SWITCH_TEST_HOME", dir.path());
            crate::settings::reload_settings().expect("reload settings");

            Self {
                dir,
                original_home,
                original_userprofile,
                original_test_home,
            }
        }
    }

    impl Drop for TempHome {
        fn drop(&mut self) {
            match &self.original_home {
                Some(value) => env::set_var("HOME", value),
                None => env::remove_var("HOME"),
            }

            match &self.original_userprofile {
                Some(value) => env::set_var("USERPROFILE", value),
                None => env::remove_var("USERPROFILE"),
            }

            match &self.original_test_home {
                Some(value) => env::set_var("CC_SWITCH_TEST_HOME", value),
                None => env::remove_var("CC_SWITCH_TEST_HOME"),
            }
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_provider_router_creation() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());
        let router = ProviderRouter::new(db);

        let breaker = router.get_or_create_circuit_breaker("claude:test").await;
        assert!(breaker.allow_request().await.allowed);
    }

    #[tokio::test]
    #[serial]
    async fn test_failover_disabled_uses_current_provider() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());

        let provider_a =
            Provider::with_id("a".to_string(), "Provider A".to_string(), json!({}), None);
        let provider_b =
            Provider::with_id("b".to_string(), "Provider B".to_string(), json!({}), None);

        db.save_provider("claude", &provider_a).unwrap();
        db.save_provider("claude", &provider_b).unwrap();
        db.set_current_provider("claude", "a").unwrap();
        db.add_to_failover_queue("claude", "b").unwrap();

        let router = ProviderRouter::new(db.clone());
        let providers = router.select_providers("claude").await.unwrap();

        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].id, "a");
    }

    #[tokio::test]
    #[serial]
    async fn test_failover_enabled_uses_queue_order_ignoring_current() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());

        // 设置 sort_index 来控制顺序：b=1, a=2
        let mut provider_a =
            Provider::with_id("a".to_string(), "Provider A".to_string(), json!({}), None);
        provider_a.sort_index = Some(2);
        let mut provider_b =
            Provider::with_id("b".to_string(), "Provider B".to_string(), json!({}), None);
        provider_b.sort_index = Some(1);

        db.save_provider("claude", &provider_a).unwrap();
        db.save_provider("claude", &provider_b).unwrap();
        db.set_current_provider("claude", "a").unwrap();

        db.add_to_failover_queue("claude", "b").unwrap();
        db.add_to_failover_queue("claude", "a").unwrap();

        // 启用自动故障转移（使用新的 proxy_config API）
        let mut config = db.get_proxy_config_for_app("claude").await.unwrap();
        config.auto_failover_enabled = true;
        db.update_proxy_config_for_app(config).await.unwrap();

        let router = ProviderRouter::new(db.clone());
        let providers = router.select_providers("claude").await.unwrap();

        assert_eq!(providers.len(), 2);
        // 故障转移开启时：仅按队列顺序选择（忽略当前供应商）
        assert_eq!(providers[0].id, "b");
        assert_eq!(providers[1].id, "a");
    }

    #[tokio::test]
    #[serial]
    async fn test_failover_enabled_uses_queue_only_even_if_current_not_in_queue() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());

        let provider_a =
            Provider::with_id("a".to_string(), "Provider A".to_string(), json!({}), None);
        let mut provider_b =
            Provider::with_id("b".to_string(), "Provider B".to_string(), json!({}), None);
        provider_b.sort_index = Some(1);

        db.save_provider("claude", &provider_a).unwrap();
        db.save_provider("claude", &provider_b).unwrap();
        db.set_current_provider("claude", "a").unwrap();

        // 只把 b 加入故障转移队列（模拟“当前供应商不在队列里”的常见配置）
        db.add_to_failover_queue("claude", "b").unwrap();

        let mut config = db.get_proxy_config_for_app("claude").await.unwrap();
        config.auto_failover_enabled = true;
        db.update_proxy_config_for_app(config).await.unwrap();

        let router = ProviderRouter::new(db.clone());
        let providers = router.select_providers("claude").await.unwrap();

        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].id, "b");
    }

    #[tokio::test]
    #[serial]
    async fn test_select_providers_does_not_consume_half_open_permit() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());

        db.update_circuit_breaker_config(&CircuitBreakerConfig {
            failure_threshold: 1,
            timeout_seconds: 0,
            ..Default::default()
        })
        .await
        .unwrap();

        let provider_a =
            Provider::with_id("a".to_string(), "Provider A".to_string(), json!({}), None);
        let provider_b =
            Provider::with_id("b".to_string(), "Provider B".to_string(), json!({}), None);

        db.save_provider("claude", &provider_a).unwrap();
        db.save_provider("claude", &provider_b).unwrap();

        db.add_to_failover_queue("claude", "a").unwrap();
        db.add_to_failover_queue("claude", "b").unwrap();

        // 启用自动故障转移（使用新的 proxy_config API）
        let mut config = db.get_proxy_config_for_app("claude").await.unwrap();
        config.auto_failover_enabled = true;
        db.update_proxy_config_for_app(config).await.unwrap();

        let router = ProviderRouter::new(db.clone());

        router
            .record_result("b", "claude", false, false, Some("fail".to_string()))
            .await
            .unwrap();

        let providers = router.select_providers("claude").await.unwrap();
        assert_eq!(providers.len(), 2);

        assert!(router.allow_provider_request("b", "claude").await.allowed);
    }

    #[tokio::test]
    #[serial]
    async fn test_release_permit_neutral_frees_half_open_slot() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());

        // 配置熔断器：1 次失败即熔断，0 秒超时立即进入 HalfOpen
        db.update_circuit_breaker_config(&CircuitBreakerConfig {
            failure_threshold: 1,
            timeout_seconds: 0,
            ..Default::default()
        })
        .await
        .unwrap();

        let provider_a =
            Provider::with_id("a".to_string(), "Provider A".to_string(), json!({}), None);
        db.save_provider("claude", &provider_a).unwrap();
        db.add_to_failover_queue("claude", "a").unwrap();

        // 启用自动故障转移
        let mut config = db.get_proxy_config_for_app("claude").await.unwrap();
        config.auto_failover_enabled = true;
        db.update_proxy_config_for_app(config).await.unwrap();

        let router = ProviderRouter::new(db.clone());

        // 触发熔断：1 次失败
        router
            .record_result("a", "claude", false, false, Some("fail".to_string()))
            .await
            .unwrap();

        // 第一次请求：获取 HalfOpen 探测名额
        let first = router.allow_provider_request("a", "claude").await;
        assert!(first.allowed);
        assert!(first.used_half_open_permit);

        // 第二次请求应被拒绝（名额已被占用）
        let second = router.allow_provider_request("a", "claude").await;
        assert!(!second.allowed);

        // 使用 release_permit_neutral 释放名额（不影响健康统计）
        router
            .release_permit_neutral("a", "claude", first.used_half_open_permit)
            .await;

        // 第三次请求应被允许（名额已释放）
        let third = router.allow_provider_request("a", "claude").await;
        assert!(third.allowed);
        assert!(third.used_half_open_permit);
    }

    #[tokio::test]
    #[serial]
    async fn test_select_providers_does_not_mutate_breaker_state() {
        use crate::proxy::circuit_breaker::CircuitState;

        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());

        // failure_threshold=1 → 一次失败即熔断；timeout=0 → Open 后立即“超时到达”
        db.update_circuit_breaker_config(&CircuitBreakerConfig {
            failure_threshold: 1,
            timeout_seconds: 0,
            ..Default::default()
        })
        .await
        .unwrap();

        let provider_a =
            Provider::with_id("a".to_string(), "Provider A".to_string(), json!({}), None);
        db.save_provider("claude", &provider_a).unwrap();
        db.add_to_failover_queue("claude", "a").unwrap();

        let mut config = db.get_proxy_config_for_app("claude").await.unwrap();
        config.auto_failover_enabled = true;
        db.update_proxy_config_for_app(config).await.unwrap();

        let router = ProviderRouter::new(db.clone());

        // 触发熔断：Provider a → Open
        router
            .record_result("a", "claude", false, false, Some("fail".to_string()))
            .await
            .unwrap();
        assert_eq!(
            router
                .get_circuit_breaker_stats("a", "claude")
                .await
                .unwrap()
                .state,
            CircuitState::Open
        );

        // 选择阶段：a 超时已到达 → 作为候选返回，但熔断器状态必须仍为 Open（纯只读）
        let providers = router.select_providers("claude").await.unwrap();
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].id, "a");
        assert_eq!(
            router
                .get_circuit_breaker_stats("a", "claude")
                .await
                .unwrap()
                .state,
            CircuitState::Open,
            "select_providers 不应把 Open 熔断器切换到 HalfOpen"
        );

        // 真正尝试时才发生 Open → HalfOpen 转换并占用探测名额
        let permit = router.allow_provider_request("a", "claude").await;
        assert!(permit.allowed);
        assert!(permit.used_half_open_permit);
        assert_eq!(
            router
                .get_circuit_breaker_stats("a", "claude")
                .await
                .unwrap()
                .state,
            CircuitState::HalfOpen
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_random_strategy_keeps_current_first_and_membership() {
        use crate::proxy::types::FailoverStrategy;

        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());

        for (id, sort) in [("a", 1), ("b", 2), ("c", 3), ("d", 4)] {
            let mut provider = Provider::with_id(
                id.to_string(),
                format!("Provider {id}"),
                serde_json::json!({}),
                None,
            );
            provider.sort_index = Some(sort);
            db.save_provider("claude", &provider).unwrap();
            db.add_to_failover_queue("claude", id).unwrap();
        }
        db.set_current_provider("claude", "c").unwrap();

        let mut config = db.get_proxy_config_for_app("claude").await.unwrap();
        config.auto_failover_enabled = true;
        config.failover_strategy = FailoverStrategy::Random;
        db.update_proxy_config_for_app(config).await.unwrap();

        let router = ProviderRouter::new(db.clone());

        // D2 粘性：当前供应商必须始终排在候选首位；候选集合 = 整个队列
        for _ in 0..20 {
            let providers = router.select_providers("claude").await.unwrap();
            assert_eq!(providers.len(), 4);
            assert_eq!(providers[0].id, "c", "当前供应商必须排首位（粘性）");

            let mut ids: Vec<&str> = providers.iter().map(|p| p.id.as_str()).collect();
            ids.sort_unstable();
            assert_eq!(ids, vec!["a", "b", "c", "d"], "随机策略不得增删候选集合");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_random_strategy_shuffles_tail_and_handles_current_outside_queue() {
        use crate::proxy::types::FailoverStrategy;

        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());

        for (id, sort) in [("a", 1), ("b", 2), ("c", 3), ("d", 4)] {
            let mut provider = Provider::with_id(
                id.to_string(),
                format!("Provider {id}"),
                serde_json::json!({}),
                None,
            );
            provider.sort_index = Some(sort);
            db.save_provider("claude", &provider).unwrap();
            db.add_to_failover_queue("claude", id).unwrap();
        }
        // 当前供应商不在故障转移队列里（常见配置）
        let outside = Provider::with_id(
            "x".to_string(),
            "Outside".to_string(),
            serde_json::json!({}),
            None,
        );
        db.save_provider("claude", &outside).unwrap();
        db.set_current_provider("claude", "x").unwrap();

        let mut config = db.get_proxy_config_for_app("claude").await.unwrap();
        config.auto_failover_enabled = true;
        config.failover_strategy = FailoverStrategy::Random;
        db.update_proxy_config_for_app(config).await.unwrap();

        let router = ProviderRouter::new(db.clone());

        let mut seen_orders = std::collections::HashSet::new();
        for _ in 0..100 {
            let providers = router.select_providers("claude").await.unwrap();
            // 队列仍是唯一候选来源：不得把队列外的当前供应商插进来
            assert_eq!(providers.len(), 4);
            assert!(providers.iter().all(|p| p.id != "x"));
            seen_orders.insert(
                providers
                    .iter()
                    .map(|p| p.id.clone())
                    .collect::<Vec<_>>()
                    .join(","),
            );
        }
        // 4 个候选全洗牌共 24 种排列，100 次只看到 1 种的概率 ≈ (1/24)^99 ≈ 0
        assert!(
            seen_orders.len() > 1,
            "随机策略应产生不同顺序，实际只看到: {seen_orders:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_random_strategy_excludes_unavailable_circuit_open_providers() {
        use crate::proxy::types::FailoverStrategy;

        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());

        // 1 次失败即熔断；冷却 60 秒 → Open 状态在测试窗口内不可用
        db.update_circuit_breaker_config(&CircuitBreakerConfig {
            failure_threshold: 1,
            timeout_seconds: 60,
            ..Default::default()
        })
        .await
        .unwrap();

        for id in ["a", "b", "c"] {
            let provider = Provider::with_id(
                id.to_string(),
                format!("Provider {id}"),
                serde_json::json!({}),
                None,
            );
            db.save_provider("claude", &provider).unwrap();
            db.add_to_failover_queue("claude", id).unwrap();
        }
        db.set_current_provider("claude", "a").unwrap();

        let mut config = db.get_proxy_config_for_app("claude").await.unwrap();
        config.auto_failover_enabled = true;
        config.failover_strategy = FailoverStrategy::Random;
        config.circuit_failure_threshold = 1;
        config.circuit_timeout_seconds = 60;
        db.update_proxy_config_for_app(config).await.unwrap();

        let router = ProviderRouter::new(db.clone());

        // b 熔断（Open，冷却未到）→ 不进入随机候选池
        router
            .record_result("b", "claude", false, false, Some("fail".to_string()))
            .await
            .unwrap();

        for _ in 0..20 {
            let providers = router.select_providers("claude").await.unwrap();
            let ids: Vec<&str> = providers.iter().map(|p| p.id.as_str()).collect();
            assert!(!ids.contains(&"b"), "熔断中的供应商不得进入随机候选池");
            assert_eq!(providers.len(), 2);
            assert_eq!(providers[0].id, "a", "当前供应商仍排首位");
        }
    }

    #[test]
    fn test_apply_random_strategy_is_deterministic_with_seeded_rng() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        let build = || -> Vec<Provider> {
            ["a", "b", "c", "d", "e"]
                .iter()
                .map(|id| {
                    Provider::with_id(
                        id.to_string(),
                        format!("Provider {id}"),
                        serde_json::json!({}),
                        None,
                    )
                })
                .collect()
        };

        // 同种子 → 同顺序（确定性）
        let mut first = build();
        let mut second = build();
        ProviderRouter::apply_random_strategy(
            &mut first,
            Some("c"),
            &mut StdRng::seed_from_u64(42),
        );
        ProviderRouter::apply_random_strategy(
            &mut second,
            Some("c"),
            &mut StdRng::seed_from_u64(42),
        );
        let order = |list: &[Provider]| list.iter().map(|p| p.id.clone()).collect::<Vec<_>>();
        assert_eq!(order(&first), order(&second));
        assert_eq!(first[0].id, "c", "当前供应商必须排首位");

        // 当前供应商缺失 → 整个列表参与洗牌，集合不变
        let mut third = build();
        ProviderRouter::apply_random_strategy(
            &mut third,
            Some("missing"),
            &mut StdRng::seed_from_u64(7),
        );
        let mut ids = order(&third);
        ids.sort_unstable();
        assert_eq!(ids, vec!["a", "b", "c", "d", "e"]);
    }

    /// M1 回归：`select_providers_with_config`（复用调用方已加载的开关/策略）必须
    /// 与自行读库的 `select_providers` 产出完全一致的候选列表。热路径据此省去每请求
    /// 一次冗余的 `get_proxy_config_for_app` 读库。
    #[tokio::test]
    #[serial]
    async fn select_providers_with_config_matches_db_read_path() {
        use crate::proxy::types::FailoverStrategy;

        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());

        let mut provider_a =
            Provider::with_id("a".to_string(), "Provider A".to_string(), json!({}), None);
        provider_a.sort_index = Some(2);
        let mut provider_b =
            Provider::with_id("b".to_string(), "Provider B".to_string(), json!({}), None);
        provider_b.sort_index = Some(1);

        db.save_provider("claude", &provider_a).unwrap();
        db.save_provider("claude", &provider_b).unwrap();
        db.set_current_provider("claude", "a").unwrap();
        db.add_to_failover_queue("claude", "b").unwrap();
        db.add_to_failover_queue("claude", "a").unwrap();

        let mut config = db.get_proxy_config_for_app("claude").await.unwrap();
        config.auto_failover_enabled = true;
        config.failover_strategy = FailoverStrategy::Sequential;
        db.update_proxy_config_for_app(config.clone())
            .await
            .unwrap();

        let router = ProviderRouter::new(db.clone());

        let via_db = router.select_providers("claude").await.unwrap();
        let via_config = router
            .select_providers_with_config(
                "claude",
                config.auto_failover_enabled,
                config.failover_strategy,
            )
            .await
            .unwrap();

        let ids = |list: &[Provider]| list.iter().map(|p| p.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(&via_db), ids(&via_config));
        // Sequential 队列顺序：b(P1) → a(P2)
        assert_eq!(ids(&via_config), vec!["b".to_string(), "a".to_string()]);
    }

    #[tokio::test]
    #[serial]
    async fn get_circuit_breaker_stats_reports_window_and_none_for_unknown() {
        use crate::proxy::circuit_breaker::CircuitState;

        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());
        let router = ProviderRouter::new(db);

        // 未知 Provider（尚未创建熔断器）→ None（L31：不再是恒定 None 桩）
        assert!(router
            .get_circuit_breaker_stats("ghost", "claude")
            .await
            .is_none());

        // 在熔断器上记录 1 成功 + 1 失败
        let breaker = router.get_or_create_circuit_breaker("claude:a").await;
        breaker.record_success(false).await;
        breaker.record_failure(false).await;

        let stats = router
            .get_circuit_breaker_stats("a", "claude")
            .await
            .expect("熔断器创建后应返回统计");
        assert_eq!(stats.state, CircuitState::Closed);
        assert_eq!(stats.total_requests, 2, "窗口应包含最近 2 次结果");
        assert_eq!(stats.failed_requests, 1);
        assert_eq!(stats.consecutive_failures, 1);
    }
}
