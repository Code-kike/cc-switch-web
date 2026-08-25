//! 故障转移队列命令
//!
//! 管理代理模式下的故障转移队列（基于 providers 表的 in_failover_queue 字段）

use crate::database::FailoverQueueItem;
use crate::provider::Provider;
use crate::store::AppState;

/// 获取故障转移队列
#[tauri::command]
pub async fn get_failover_queue(
    state: tauri::State<'_, AppState>,
    app_type: String,
) -> Result<Vec<FailoverQueueItem>, String> {
    state
        .db
        .get_failover_queue(&app_type)
        .map_err(|e| e.to_string())
}

/// 获取可添加到故障转移队列的供应商（不在队列中的）
#[tauri::command]
pub async fn get_available_providers_for_failover(
    state: tauri::State<'_, AppState>,
    app_type: String,
) -> Result<Vec<Provider>, String> {
    state
        .db
        .get_available_providers_for_failover(&app_type)
        .map_err(|e| e.to_string())
}

/// 添加供应商到故障转移队列
#[tauri::command]
pub async fn add_to_failover_queue(
    state: tauri::State<'_, AppState>,
    app_type: String,
    provider_id: String,
) -> Result<(), String> {
    state
        .db
        .add_to_failover_queue_checked(&app_type, &provider_id)
        .map_err(|e| e.to_string())
}

/// 从故障转移队列移除供应商
#[tauri::command]
pub async fn remove_from_failover_queue(
    state: tauri::State<'_, AppState>,
    app_type: String,
    provider_id: String,
) -> Result<(), String> {
    state
        .db
        .remove_from_failover_queue(&app_type, &provider_id)
        .map_err(|e| e.to_string())
}

/// 获取指定应用的自动故障转移开关状态（从 proxy_config 表读取）
#[tauri::command]
pub async fn get_auto_failover_enabled(
    state: tauri::State<'_, AppState>,
    app_type: String,
) -> Result<bool, String> {
    state
        .db
        .get_proxy_config_for_app(&app_type)
        .await
        .map(|config| config.auto_failover_enabled)
        .map_err(|e| e.to_string())
}

/// 设置指定应用的自动故障转移开关状态（写入 proxy_config 表）
///
/// 注意：关闭故障转移时不会清除队列，队列内容会保留供下次开启时使用。
///
/// 强一致语义（开启时自动加入当前供应商、切到 P1、发 `provider-switched`、刷新托盘）
/// 现统一在 runtime-neutral 的 `ProxyService::set_auto_failover_enabled` 中实现，
/// 桌面与 Web 共用同一份逻辑——事件与托盘刷新经由注入的 `TauriEventSink` 完成
/// （Web 端注入 `ChannelEventSink` 走 SSE）。
#[tauri::command]
pub async fn set_auto_failover_enabled(
    state: tauri::State<'_, AppState>,
    app_type: String,
    enabled: bool,
) -> Result<(), String> {
    state
        .proxy_service
        .set_auto_failover_enabled(&app_type, enabled)
        .await
}

#[cfg(test)]
mod tests {
    use crate::database::Database;
    use crate::provider::{AuthBinding, AuthBindingSource, Provider, ProviderMeta};
    use serde_json::json;

    #[test]
    fn failover_entry_point_rejects_codex_official_account_cards() {
        let db = Database::memory().expect("memory db");
        let mut official = Provider::with_id(
            "official-a".to_string(),
            "OpenAI Official".to_string(),
            json!({ "auth": {}, "config": "" }),
            None,
        );
        official.category = Some("official".to_string());
        official.meta = Some(ProviderMeta {
            auth_binding: Some(AuthBinding {
                source: AuthBindingSource::ManagedAccount,
                auth_provider: Some("codex_oauth".to_string()),
                account_id: Some("account-a".to_string()),
            }),
            ..Default::default()
        });
        db.save_provider("codex", &official).expect("save official");

        assert!(
            db.add_to_failover_queue_checked("codex", &official.id)
                .is_err(),
            "an account card must not be queueable"
        );
        assert!(
            db.get_available_providers_for_failover("codex")
                .expect("list available")
                .iter()
                .all(|provider| provider.id != official.id),
            "an account card must not be offered as a queue candidate either"
        );
    }

    #[test]
    fn failover_entry_point_accepts_third_party_providers() {
        let db = Database::memory().expect("memory db");
        let provider = Provider::with_id(
            "third-party".to_string(),
            "Third Party".to_string(),
            json!({ "auth": {}, "config": "" }),
            None,
        );
        db.save_provider("codex", &provider).expect("save");

        assert!(db
            .add_to_failover_queue_checked("codex", &provider.id)
            .is_ok());
    }
}
