//! Managed Codex OAuth account transaction helpers.
//!
//! A "managed Codex Official card" is a provider row whose
//! `meta.authBinding` selects one of the ChatGPT accounts held by
//! `CodexOAuthManager` instead of carrying its own stored credential. Writing
//! such a card's live config therefore has to resolve (and possibly refresh) a
//! token over the network, which can fail — so `add` / `update` / `switch` wrap
//! it in a transaction that can restore `~/.codex/{auth.json,config.toml}`, the
//! model catalog and the cc-switch marker as one unit.
//!
//! # Provenance
//!
//! Upstream (`v3.20.0`) keeps every function below as a private associated
//! function at the top of the first `impl ProviderService` block in
//! `services/provider/mod.rs`, in this same order. They live in their own
//! module here because `mod.rs` already carries the largest `impl` in the
//! directory, and because the sibling split (`live.rs`, `pi.rs`, `usage.rs`,
//! `endpoints.rs`, `gemini_auth.rs`) is the established shape for
//! concern-scoped free functions. Names are kept byte-identical to upstream and
//! `mod.rs` imports them unqualified, so a future upstream hunk touching one of
//! them applies here with only the `Self::` prefix removed.
//!
//! The *call sites* (the managed arms inside `add` / `update` / `switch`) stay
//! in `mod.rs`: they are interleaved edits inside functions whose surrounding
//! code diverges from upstream, so they have to be hand-merged in place either
//! way.

use crate::app_config::AppType;
use crate::error::AppError;
use crate::provider::Provider;
use crate::store::AppState;

use super::live;

/// The non-empty `codex_oauth` managed account id bound to `provider`, if any.
///
/// Upstream spells the derivation out here; the fork routes it through the
/// domain method so the transaction layer, `live.rs` and the tray cannot
/// disagree about a blank binding (see
/// [`Provider::managed_codex_oauth_account_id`]).
pub(super) fn managed_codex_oauth_account_id(provider: &Provider) -> Option<String> {
    provider.managed_codex_oauth_account_id()
}

/// 提交 current（settings/DB）前的预检：若目标是托管 Codex official provider，
/// 先解析一次有效 live 配置（会联网换取并缓存 token）。同时返回这份已解析配置，
/// 让后续落盘直接复用同一 token bundle，避免一次操作重复解析/刷新。
pub(super) fn preflight_managed_codex_live(
    state: &AppState,
    app_type: &AppType,
    provider: &Provider,
) -> Result<Option<Provider>, AppError> {
    if matches!(app_type, AppType::Codex) && managed_codex_oauth_account_id(provider).is_some() {
        return live::build_effective_provider_for_live_for_state(state, app_type, provider)
            .map(Some);
    }
    Ok(None)
}

pub(super) fn write_preflighted_or_current_live(
    state: &AppState,
    app_type: &AppType,
    provider: &Provider,
    preflighted_provider: Option<&Provider>,
) -> Result<(), AppError> {
    if let Some(effective_provider) = preflighted_provider {
        live::write_live_snapshot(app_type, effective_provider)
    } else {
        live::write_live_with_common_config_for_state(state, app_type, provider)
    }
}

pub(super) fn managed_codex_transaction_error(
    operation: &str,
    error: AppError,
    snapshot: &crate::codex_config::CodexLiveStateSnapshot,
    restore_local_current: Option<(&AppType, Option<&str>)>,
) -> AppError {
    let mut rollback_failures = Vec::new();
    if let Some((app_type, previous_local_current)) = restore_local_current {
        if let Err(rollback_error) =
            crate::settings::set_current_provider(app_type, previous_local_current)
        {
            rollback_failures.push(format!("恢复本地 current 失败: {rollback_error}"));
        }
    }
    if let Err(rollback_error) = snapshot.restore_preserving_newer_same_account_auth() {
        rollback_failures.push(rollback_error.to_string());
    }

    if rollback_failures.is_empty() {
        error
    } else {
        AppError::Message(format!(
            "{operation}失败: {error}; 回滚同时失败: {}",
            rollback_failures.join("; ")
        ))
    }
}

pub(super) fn managed_codex_add_transaction_error(
    state: &AppState,
    operation: &str,
    error: AppError,
    provider: &Provider,
    previous_provider: Option<&Provider>,
    provider_saved: bool,
    snapshot: &crate::codex_config::CodexLiveStateSnapshot,
) -> AppError {
    let mut rollback_failures = Vec::new();

    if provider_saved {
        let provider_rollback = match previous_provider {
            Some(previous) => state.db.save_provider(AppType::Codex.as_str(), previous),
            None => state
                .db
                .delete_provider(AppType::Codex.as_str(), &provider.id),
        };
        if let Err(rollback_error) = provider_rollback {
            rollback_failures.push(format!("恢复 Provider 数据失败: {rollback_error}"));
        }
    }

    if let Err(rollback_error) = snapshot.restore_preserving_newer_same_account_auth() {
        rollback_failures.push(rollback_error.to_string());
    }

    if rollback_failures.is_empty() {
        error
    } else {
        AppError::Message(format!(
            "{operation}失败: {error}; 回滚同时失败: {}",
            rollback_failures.join("; ")
        ))
    }
}

/// The managed account id whose live auth is being *left behind* by this
/// operation: `Some` only when the stored row was bound to a different account
/// than the incoming one. `None` for a same-account edit, so an unrelated
/// rename never clears the live credential it is still using.
pub(super) fn outgoing_managed_codex_oauth_account_id(
    app_type: &AppType,
    existing_provider: Option<&Provider>,
    provider: &Provider,
) -> Option<String> {
    if !matches!(app_type, AppType::Codex) {
        return None;
    }

    let old_account_id = existing_provider.and_then(managed_codex_oauth_account_id)?;
    if managed_codex_oauth_account_id(provider).as_deref() == Some(old_account_id.as_str()) {
        return None;
    }

    Some(old_account_id)
}

pub(super) fn prepare_outgoing_managed_codex_live_auth(
    state: &AppState,
    account_id: Option<&str>,
) -> Result<Option<String>, AppError> {
    let Some(account_id) = account_id else {
        return Ok(None);
    };
    live::prepare_codex_managed_oauth_live_auth_switch_away_for_state(state, account_id)
}

pub(super) fn ensure_outgoing_managed_codex_live_auth_unchanged(
    account_id: Option<&str>,
    expected_refresh_token: Option<&str>,
) -> Result<(), AppError> {
    if let (Some(account_id), Some(expected_refresh_token)) = (account_id, expected_refresh_token) {
        crate::codex_config::ensure_codex_live_auth_unchanged_for_managed_account(
            account_id,
            expected_refresh_token,
        )?;
    }
    Ok(())
}

pub(super) fn clear_outgoing_managed_codex_live_auth(
    account_id: Option<&str>,
    expected_refresh_token: Option<&str>,
) -> Result<(), AppError> {
    let Some(account_id) = account_id else {
        return Ok(());
    };
    if let Some(expected_refresh_token) = expected_refresh_token {
        crate::codex_config::clear_codex_live_auth_for_managed_account_if_unchanged(
            account_id,
            Some(expected_refresh_token),
        )
    } else {
        crate::codex_config::clear_codex_live_auth_for_managed_account(account_id)
    }
}
