//! 熔断器模块
//!
//! 实现熔断器模式，用于防止向不健康的供应商发送请求

use super::log_codes::cb as log_cb;
use super::types::AppProxyConfig;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// 熔断器状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CircuitState {
    /// 关闭状态 - 正常工作
    Closed,
    /// 打开状态 - 熔断激活，拒绝请求
    Open,
    /// 半开状态 - 尝试恢复，允许部分请求通过
    HalfOpen,
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "closed"),
            CircuitState::Open => write!(f, "open"),
            CircuitState::HalfOpen => write!(f, "half_open"),
        }
    }
}

/// 熔断器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CircuitBreakerConfig {
    /// 失败阈值 - 连续失败多少次后打开熔断器
    pub failure_threshold: u32,
    /// 成功阈值 - 半开状态下成功多少次后关闭熔断器
    pub success_threshold: u32,
    /// 超时时间 - 熔断器打开后多久尝试半开（秒）
    pub timeout_seconds: u64,
    /// 错误率阈值 - 错误率超过此值时打开熔断器 (0.0-1.0)
    pub error_rate_threshold: f64,
    /// 最小请求数 - 计算错误率前窗口中的最小样本数；同时作为错误率滑动窗口的容量。
    ///
    /// 语义（M2 修复后）：错误率不再基于“自上次关闭以来的累计计数”，而是基于
    /// 最近 `min_requests` 次请求结果的**滑动窗口**。窗口未填满（样本数 < `min_requests`）
    /// 时不做错误率判定；填满后仅统计最近 `min_requests` 次结果，陈旧失败随新结果滚出
    /// 窗口而自动老化。窗口为定长（容量 = `min_requests`），内存有界。
    pub min_requests: u32,
}

impl From<&AppProxyConfig> for CircuitBreakerConfig {
    fn from(config: &AppProxyConfig) -> Self {
        Self {
            failure_threshold: config.circuit_failure_threshold,
            success_threshold: config.circuit_success_threshold,
            timeout_seconds: config.circuit_timeout_seconds as u64,
            error_rate_threshold: config.circuit_error_rate_threshold,
            min_requests: config.circuit_min_requests,
        }
    }
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 4,
            success_threshold: 2,
            timeout_seconds: 60,
            error_rate_threshold: 0.6,
            min_requests: 10,
        }
    }
}

/// 熔断器实例
pub struct CircuitBreaker {
    /// 当前状态
    state: Arc<RwLock<CircuitState>>,
    /// 连续失败计数
    consecutive_failures: Arc<AtomicU32>,
    /// 连续成功计数（半开状态）
    consecutive_successes: Arc<AtomicU32>,
    /// 错误率判定用的滑动窗口：最近若干次请求结果（`true` = 失败，`false` = 成功），
    /// 队首为最旧。窗口容量等于 `min_requests`，使陈旧失败随新结果滚出窗口而老化，
    /// 且内存有界（取代旧的累计 `total_requests` / `failed_requests` 计数器）。
    outcome_window: Arc<RwLock<VecDeque<bool>>>,
    /// 上次打开时间
    last_opened_at: Arc<RwLock<Option<Instant>>>,
    /// 配置（支持热更新）
    config: Arc<RwLock<CircuitBreakerConfig>>,
    /// 半开状态已放行的请求数（用于限流）
    half_open_requests: Arc<AtomicU32>,
}

/// 熔断器放行结果
///
/// `used_half_open_permit` 表示本次放行是否占用了 HalfOpen 探测名额。
/// 调用方应在请求结束后把该值传回 `record_success` / `record_failure` 用于正确释放名额。
#[derive(Debug, Clone, Copy)]
pub struct AllowResult {
    pub allowed: bool,
    pub used_half_open_permit: bool,
}

impl CircuitBreaker {
    /// 创建新的熔断器
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            consecutive_failures: Arc::new(AtomicU32::new(0)),
            consecutive_successes: Arc::new(AtomicU32::new(0)),
            outcome_window: Arc::new(RwLock::new(VecDeque::new())),
            last_opened_at: Arc::new(RwLock::new(None)),
            config: Arc::new(RwLock::new(config)),
            half_open_requests: Arc::new(AtomicU32::new(0)),
        }
    }

    /// 更新熔断器配置（热更新，不重置状态）
    pub async fn update_config(&self, new_config: CircuitBreakerConfig) {
        *self.config.write().await = new_config;
    }

    /// 判断当前 Provider 是否“可被纳入候选链路”
    ///
    /// 这是**纯只读**的可用性判断（L2）：仅用于路由选择阶段，不会修改熔断器状态，
    /// 也不会占用 HalfOpen 探测名额。
    /// - Closed / HalfOpen：可用（返回 true）
    /// - Open：若超时已到达则可用（返回 true），否则返回 false
    ///
    /// 注意：Open → HalfOpen 的真正状态转换以及单次探测名额的获取，统一发生在
    /// 请求**实际尝试**时的 `allow_request()` 中。把转换从“选择阶段”移除后，被选中
    /// 但最终未被尝试的 Provider（例如前序 Provider 已成功）不会被无意义地切到
    /// HalfOpen，状态更贴近真实探测发生的时刻。
    pub async fn is_available(&self) -> bool {
        let state = *self.state.read().await;

        match state {
            CircuitState::Closed | CircuitState::HalfOpen => true,
            CircuitState::Open => {
                let config = self.config.read().await;
                let opened = *self.last_opened_at.read().await;
                opened.is_some_and(|opened_at| {
                    opened_at.elapsed().as_secs() >= config.timeout_seconds
                })
            }
        }
    }

    /// 检查是否允许请求通过
    pub async fn allow_request(&self) -> AllowResult {
        let state = *self.state.read().await;

        match state {
            CircuitState::Closed => AllowResult {
                allowed: true,
                used_half_open_permit: false,
            },
            CircuitState::Open => {
                let config = self.config.read().await;
                // 检查是否应该尝试半开
                if let Some(opened_at) = *self.last_opened_at.read().await {
                    if opened_at.elapsed().as_secs() >= config.timeout_seconds {
                        drop(config); // 释放读锁再转换状态
                        log::info!(
                            "[{}] 熔断器 Open → HalfOpen (超时恢复)",
                            log_cb::OPEN_TO_HALF_OPEN
                        );
                        self.transition_to_half_open().await;

                        // 转换后按当前状态决定是否需要获取 HalfOpen 探测名额
                        let current_state = *self.state.read().await;
                        return match current_state {
                            CircuitState::Closed => AllowResult {
                                allowed: true,
                                used_half_open_permit: false,
                            },
                            CircuitState::HalfOpen => self.allow_half_open_probe(),
                            CircuitState::Open => AllowResult {
                                allowed: false,
                                used_half_open_permit: false,
                            },
                        };
                    }
                }

                AllowResult {
                    allowed: false,
                    used_half_open_permit: false,
                }
            }
            CircuitState::HalfOpen => self.allow_half_open_probe(),
        }
    }

    /// 记录成功
    pub async fn record_success(&self, used_half_open_permit: bool) {
        let state = *self.state.read().await;
        let config = self.config.read().await;

        if used_half_open_permit {
            self.release_half_open_permit();
        }

        // 重置失败计数，并把本次成功推入错误率滑动窗口
        self.consecutive_failures.store(0, Ordering::SeqCst);
        self.push_outcome_and_snapshot(false, config.min_requests)
            .await;

        if state == CircuitState::HalfOpen {
            let successes = self.consecutive_successes.fetch_add(1, Ordering::SeqCst) + 1;

            if successes >= config.success_threshold {
                drop(config); // 释放读锁再转换状态
                log::info!(
                    "[{}] 熔断器 HalfOpen → Closed (恢复正常)",
                    log_cb::HALF_OPEN_TO_CLOSED
                );
                self.transition_to_closed().await;
            }
        }
    }

    /// 记录失败
    pub async fn record_failure(&self, used_half_open_permit: bool) {
        let state = *self.state.read().await;
        let config = self.config.read().await;

        if used_half_open_permit {
            self.release_half_open_permit();
        }

        // 更新连续失败计数；连续成功计数清零
        let failures = self.consecutive_failures.fetch_add(1, Ordering::SeqCst) + 1;
        self.consecutive_successes.store(0, Ordering::SeqCst);

        // 将本次失败推入滑动窗口，并取窗口内 (总数, 失败数) 快照用于错误率判定。
        // 陈旧失败会随新结果滚出窗口而老化，避免基于历史累计计数的误熔断。
        let (window_total, window_failed) = self
            .push_outcome_and_snapshot(true, config.min_requests)
            .await;

        // 检查是否应该打开熔断器
        match state {
            CircuitState::HalfOpen => {
                // HalfOpen 状态下失败，立即转为 Open
                log::warn!(
                    "[{}] 熔断器 HalfOpen 探测失败 → Open",
                    log_cb::HALF_OPEN_PROBE_FAILED
                );
                drop(config);
                self.transition_to_open().await;
            }
            CircuitState::Closed => {
                // 检查连续失败次数
                if failures >= config.failure_threshold {
                    log::warn!(
                        "[{}] 熔断器触发: 连续失败 {failures} 次 → Open",
                        log_cb::TRIGGERED_FAILURES
                    );
                    drop(config); // 释放读锁再转换状态
                    self.transition_to_open().await;
                } else if window_total >= config.min_requests {
                    // 错误率基于最近 min_requests 次结果的滑动窗口（窗口已填满）
                    let error_rate = window_failed as f64 / window_total as f64;

                    if error_rate >= config.error_rate_threshold {
                        log::warn!(
                            "[{}] 熔断器触发: 错误率 {:.1}% → Open",
                            log_cb::TRIGGERED_ERROR_RATE,
                            error_rate * 100.0
                        );
                        drop(config); // 释放读锁再转换状态
                        self.transition_to_open().await;
                    }
                }
            }
            _ => {}
        }
    }

    /// 获取当前状态
    #[allow(dead_code)]
    pub async fn get_state(&self) -> CircuitState {
        *self.state.read().await
    }

    /// 获取统计信息
    ///
    /// `total_requests` / `failed_requests` 反映**当前滑动窗口**内的样本数与失败数
    /// （而非历史累计），与 M2 修复后的错误率判定口径一致。
    pub async fn get_stats(&self) -> CircuitBreakerStats {
        let (total_requests, failed_requests) = self.window_snapshot().await;
        CircuitBreakerStats {
            state: *self.state.read().await,
            consecutive_failures: self.consecutive_failures.load(Ordering::SeqCst),
            consecutive_successes: self.consecutive_successes.load(Ordering::SeqCst),
            total_requests,
            failed_requests,
        }
    }

    /// 重置熔断器（手动恢复）
    #[allow(dead_code)]
    pub async fn reset(&self) {
        log::info!("[{}] 熔断器手动重置 → Closed", log_cb::MANUAL_RESET);
        self.transition_to_closed().await;
    }

    fn allow_half_open_probe(&self) -> AllowResult {
        // 半开状态限流：只允许有限请求通过进行探测
        let max_half_open_requests = 1u32;
        let current = self.half_open_requests.fetch_add(1, Ordering::SeqCst);

        if current < max_half_open_requests {
            AllowResult {
                allowed: true,
                used_half_open_permit: true,
            }
        } else {
            // 超过限额，回退计数，拒绝请求
            self.half_open_requests.fetch_sub(1, Ordering::SeqCst);
            AllowResult {
                allowed: false,
                used_half_open_permit: false,
            }
        }
    }

    /// 仅释放 HalfOpen permit，不影响健康统计
    ///
    /// 用于整流器等场景：请求结果不应计入 Provider 健康度，
    /// 但仍需释放占用的探测名额，避免 HalfOpen 状态卡死
    pub fn release_half_open_permit(&self) {
        let mut current = self.half_open_requests.load(Ordering::SeqCst);
        loop {
            if current == 0 {
                return;
            }

            match self.half_open_requests.compare_exchange(
                current,
                current - 1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return,
                Err(actual) => current = actual,
            }
        }
    }

    /// 将一次请求结果推入错误率滑动窗口，并返回 `(窗口内总样本数, 窗口内失败数)` 快照。
    ///
    /// 窗口容量等于 `min_requests`（至少为 1），保证只统计最近的结果且内存有界：
    /// 超出容量的最旧结果会从队首滚出。
    async fn push_outcome_and_snapshot(&self, is_failure: bool, min_requests: u32) -> (u32, u32) {
        let capacity = (min_requests as usize).max(1);
        let mut window = self.outcome_window.write().await;
        window.push_back(is_failure);
        while window.len() > capacity {
            window.pop_front();
        }
        let total = window.len() as u32;
        let failed = window.iter().filter(|&&f| f).count() as u32;
        (total, failed)
    }

    /// 读取错误率滑动窗口的 `(总样本数, 失败数)` 快照（只读，不修改窗口）。
    async fn window_snapshot(&self) -> (u32, u32) {
        let window = self.outcome_window.read().await;
        let total = window.len() as u32;
        let failed = window.iter().filter(|&&f| f).count() as u32;
        (total, failed)
    }

    /// 转换到打开状态
    async fn transition_to_open(&self) {
        *self.state.write().await = CircuitState::Open;
        *self.last_opened_at.write().await = Some(Instant::now());
        self.consecutive_failures.store(0, Ordering::SeqCst);
        self.consecutive_successes.store(0, Ordering::SeqCst);
    }

    /// 转换到半开状态
    async fn transition_to_half_open(&self) {
        let mut state = self.state.write().await;
        if *state != CircuitState::Open {
            return;
        }

        *state = CircuitState::HalfOpen;
        self.consecutive_successes.store(0, Ordering::SeqCst);
        // 重置半开状态的请求限流计数
        self.half_open_requests.store(0, Ordering::SeqCst);
    }

    /// 转换到关闭状态
    async fn transition_to_closed(&self) {
        *self.state.write().await = CircuitState::Closed;
        self.consecutive_failures.store(0, Ordering::SeqCst);
        self.consecutive_successes.store(0, Ordering::SeqCst);
        // 清空错误率滑动窗口，恢复后重新开始统计
        self.outcome_window.write().await.clear();
    }
}

/// 熔断器统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CircuitBreakerStats {
    pub state: CircuitState,
    pub consecutive_failures: u32,
    pub consecutive_successes: u32,
    pub total_requests: u32,
    pub failed_requests: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_closed_to_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // 初始状态应该是关闭
        assert_eq!(breaker.get_state().await, CircuitState::Closed);
        assert!(breaker.allow_request().await.allowed);

        // 记录 3 次失败
        for _ in 0..3 {
            breaker.record_failure(false).await;
        }

        // 应该转换到打开状态
        assert_eq!(breaker.get_state().await, CircuitState::Open);
        assert!(!breaker.allow_request().await.allowed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_to_closed() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // 打开熔断器
        breaker.record_failure(false).await;
        breaker.record_failure(false).await;
        assert_eq!(breaker.get_state().await, CircuitState::Open);

        // 手动转换到半开状态
        breaker.transition_to_half_open().await;
        assert_eq!(breaker.get_state().await, CircuitState::HalfOpen);

        // 记录 2 次成功
        breaker.record_success(false).await;
        breaker.record_success(false).await;

        // 应该转换到关闭状态
        assert_eq!(breaker.get_state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_half_open_transition_does_not_reset_inflight_permit() {
        let config = CircuitBreakerConfig {
            timeout_seconds: 0,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // 进入 Open，然后由于 timeout_seconds=0，allow_request 会立即切换到 HalfOpen 并占用探测名额
        breaker.transition_to_open().await;
        let first = breaker.allow_request().await;
        assert!(first.allowed);
        assert!(first.used_half_open_permit);
        assert_eq!(breaker.get_state().await, CircuitState::HalfOpen);

        // 模拟并发下的“重复 HalfOpen 转换调用”，不应重置 in-flight 计数
        breaker.transition_to_half_open().await;

        // 由于名额仍被占用，第二次请求应被拒绝
        let second = breaker.allow_request().await;
        assert!(!second.allowed);
        assert!(!second.used_half_open_permit);
    }

    #[tokio::test]
    async fn is_available_is_pure_read_and_does_not_transition_open_to_half_open() {
        // timeout_seconds=0 → 一旦 Open，超时立即视为“已到达”
        let breaker = CircuitBreaker::new(CircuitBreakerConfig {
            timeout_seconds: 0,
            ..Default::default()
        });
        breaker.transition_to_open().await;
        assert_eq!(breaker.get_state().await, CircuitState::Open);

        // 选择阶段的可用性判断：超时已到 → 视为候选可用，但**不得**修改状态
        assert!(breaker.is_available().await);
        assert_eq!(
            breaker.get_state().await,
            CircuitState::Open,
            "is_available 不应在选择阶段触发 Open → HalfOpen 转换"
        );

        // 真正的转换 + 单次探测名额，发生在请求实际尝试时的 allow_request()
        let allow = breaker.allow_request().await;
        assert!(allow.allowed);
        assert!(allow.used_half_open_permit);
        assert_eq!(breaker.get_state().await, CircuitState::HalfOpen);
    }

    #[tokio::test]
    async fn is_available_returns_false_while_open_and_timeout_not_elapsed() {
        let breaker = CircuitBreaker::new(CircuitBreakerConfig {
            timeout_seconds: 3600,
            ..Default::default()
        });
        breaker.transition_to_open().await;

        assert!(!breaker.is_available().await);
        assert_eq!(breaker.get_state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_reset() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // 打开熔断器
        breaker.record_failure(false).await;
        breaker.record_failure(false).await;
        assert_eq!(breaker.get_state().await, CircuitState::Open);

        // 重置
        breaker.reset().await;
        assert_eq!(breaker.get_state().await, CircuitState::Closed);
        assert!(breaker.allow_request().await.allowed);
    }

    // ===== M2: 错误率滑动窗口 =====

    #[tokio::test]
    async fn error_rate_burst_within_window_opens() {
        // failure_threshold 设为极大值，关闭“连续失败”路径，只验证错误率（滑动窗口）路径
        let config = CircuitBreakerConfig {
            failure_threshold: u32::MAX,
            error_rate_threshold: 0.5,
            min_requests: 4, // 窗口容量 = 4
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        breaker.record_success(false).await; // 窗口 [S]
        breaker.record_failure(false).await; // [S,F] 未填满(<4)，不判定
        breaker.record_failure(false).await; // [S,F,F] 未填满
        assert_eq!(breaker.get_state().await, CircuitState::Closed);

        // [S,F,F,F] 填满 → 3/4 = 0.75 ≥ 0.5 → Open（错误率路径，非连续失败）
        breaker.record_failure(false).await;
        assert_eq!(breaker.get_state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn stale_failures_age_out_of_error_rate_window() {
        let config = CircuitBreakerConfig {
            failure_threshold: u32::MAX, // 关闭连续失败路径
            error_rate_threshold: 0.5,
            min_requests: 4, // 窗口容量 = 4
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // 3 次失败：窗口未填满(<4)，不触发错误率判定
        breaker.record_failure(false).await;
        breaker.record_failure(false).await;
        breaker.record_failure(false).await;
        assert_eq!(breaker.get_state().await, CircuitState::Closed);

        // 用成功填满并淹没窗口，使 3 个陈旧失败滚出窗口
        for _ in 0..5 {
            breaker.record_success(false).await;
        }
        let stats = breaker.get_stats().await;
        assert_eq!(
            stats.failed_requests, 0,
            "陈旧失败必须随窗口滚动而老化（窗口内不应再统计到它们）"
        );
        assert_eq!(
            stats.total_requests, 4,
            "窗口必须有界，长度等于 min_requests"
        );

        // 单次新失败：窗口 [S,S,S,F] → 1/4 = 0.25 < 0.5 → 不熔断。
        // 旧实现下 3 个早期失败仍被累计计入，会抬高错误率；滑动窗口修复后不会。
        breaker.record_failure(false).await;
        assert_eq!(breaker.get_state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn outcome_window_is_bounded_by_min_requests() {
        // error_rate_threshold > 1.0 使错误率永不触发，确保熔断器保持 Closed 持续累积请求
        let config = CircuitBreakerConfig {
            failure_threshold: u32::MAX,
            error_rate_threshold: 1.1,
            min_requests: 5,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        for _ in 0..1000 {
            breaker.record_success(false).await;
            breaker.record_failure(false).await;
        }

        assert_eq!(breaker.get_state().await, CircuitState::Closed);
        let stats = breaker.get_stats().await;
        assert!(
            stats.total_requests <= 5,
            "滑动窗口必须有界（≤ min_requests），实际为 {}",
            stats.total_requests
        );
    }

    #[tokio::test]
    async fn half_open_success_closes_and_clears_window() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 1,
            min_requests: 4,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // 触发到 Open
        breaker.record_failure(false).await;
        breaker.record_failure(false).await;
        assert_eq!(breaker.get_state().await, CircuitState::Open);

        // 进入 HalfOpen 并探测成功 → Closed（single-probe 语义保持不变）
        breaker.transition_to_half_open().await;
        breaker.record_success(true).await;
        assert_eq!(breaker.get_state().await, CircuitState::Closed);

        // 关闭后窗口被清空，重新开始统计
        let stats = breaker.get_stats().await;
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.failed_requests, 0);
    }
}
