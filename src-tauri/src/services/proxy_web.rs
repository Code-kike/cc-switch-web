use crate::app_config::AppType;
use crate::database::Database;
use crate::provider::Provider;
use crate::proxy::circuit_breaker::CircuitBreakerConfig;
use crate::proxy::switch_lock::SwitchLockManager;
use crate::proxy::types::{ProxyConfig, ProxyServerInfo, ProxyStatus, ProxyTakeoverStatus};
use std::sync::Arc;

#[derive(Clone)]
pub struct ProxyService {
    #[allow(dead_code)]
    db: Arc<Database>,
    /// Web 模式没有本地代理，但 provider 切换路径仍依赖 per-app 切换锁
    /// 串行化（与桌面端 `lock_switch_for_app` 同一契约），因此这里持有
    /// 一把真实的（轻量）锁，而不是空实现。
    switch_locks: SwitchLockManager,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct HotSwitchOutcome {
    pub logical_target_changed: bool,
}

impl ProxyService {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            switch_locks: SwitchLockManager::new(),
        }
    }

    /// Real (trivial) serialization guard: provider switches in web mode still
    /// serialize per app, mirroring the desktop ProxyService contract.
    pub(crate) async fn lock_switch_for_app(
        &self,
        app_type: &str,
    ) -> tokio::sync::OwnedMutexGuard<()> {
        self.switch_locks.lock_for_app(app_type).await
    }

    pub fn cleanup_claude_model_overrides_in_live(&self) -> Result<(), String> {
        Ok(())
    }

    pub async fn sync_claude_live_from_provider_while_proxy_active(
        &self,
        _provider: &Provider,
    ) -> Result<(), String> {
        Ok(())
    }

    pub async fn start(&self) -> Result<ProxyServerInfo, String> {
        Err("proxy service is unavailable in web-server mode".to_string())
    }

    pub async fn start_with_takeover(&self) -> Result<ProxyServerInfo, String> {
        Err("proxy takeover is unavailable in web-server mode".to_string())
    }

    pub async fn get_takeover_status(&self) -> Result<ProxyTakeoverStatus, String> {
        Ok(ProxyTakeoverStatus::default())
    }

    pub async fn set_takeover_for_app(
        &self,
        _app_type: &str,
        _enabled: bool,
    ) -> Result<(), String> {
        Err("proxy takeover is unavailable in web-server mode".to_string())
    }

    pub async fn stop(&self) -> Result<(), String> {
        Ok(())
    }

    pub async fn stop_with_restore(&self) -> Result<(), String> {
        Ok(())
    }

    pub async fn stop_with_restore_keep_state(&self) -> Result<(), String> {
        Ok(())
    }

    pub fn detect_takeover_in_live_config_for_app(&self, _app_type: &AppType) -> bool {
        false
    }

    pub async fn is_takeover_active(&self) -> Result<bool, String> {
        Ok(false)
    }

    pub async fn recover_from_crash(&self) -> Result<(), String> {
        Ok(())
    }

    pub fn detect_takeover_in_live_configs(&self) -> bool {
        false
    }

    pub async fn update_live_backup_from_provider(
        &self,
        _app_type: &str,
        _provider: &Provider,
    ) -> Result<(), String> {
        Ok(())
    }

    pub async fn hot_switch_provider(
        &self,
        _app_type: &str,
        _id: &str,
    ) -> Result<HotSwitchOutcome, String> {
        Ok(HotSwitchOutcome::default())
    }

    /// Silent no-op like `hot_switch_provider`: there is no local proxy in
    /// web-server mode, so the takeover hot-switch refresh is fire-and-forget.
    /// The caller (dual-compiled provider switch path) already holds the
    /// per-app switch lock from `lock_switch_for_app`.
    pub(crate) async fn hot_switch_provider_inner(
        &self,
        _app_type: &str,
        _id: &str,
    ) -> Result<HotSwitchOutcome, String> {
        Ok(HotSwitchOutcome::default())
    }

    pub async fn switch_proxy_target(
        &self,
        _app_type: &str,
        _provider_id: &str,
    ) -> Result<(), String> {
        Err("proxy switching is unavailable in web-server mode".to_string())
    }

    pub async fn get_status(&self) -> Result<ProxyStatus, String> {
        Ok(ProxyStatus::default())
    }

    pub async fn get_config(&self) -> Result<ProxyConfig, String> {
        Ok(ProxyConfig::default())
    }

    pub async fn update_config(&self, _config: &ProxyConfig) -> Result<(), String> {
        Ok(())
    }

    pub async fn is_running(&self) -> bool {
        false
    }

    pub async fn update_circuit_breaker_configs(
        &self,
        _config: &CircuitBreakerConfig,
    ) -> Result<(), String> {
        Ok(())
    }

    /// No-op in web mode (the local proxy does not run here). Param order is
    /// kept as `(provider_id, app_type)` to match the desktop
    /// `ProxyService::reset_provider_circuit_breaker` convention, so a future
    /// real implementation cannot silently swap the two `&str` arguments.
    pub async fn reset_provider_circuit_breaker(
        &self,
        _provider_id: &str,
        _app_type: &str,
    ) -> Result<(), String> {
        Ok(())
    }
}
