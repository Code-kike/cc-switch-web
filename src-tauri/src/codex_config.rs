use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::{
    atomic_write, atomic_write_managed, delete_file, get_home_dir, path_is_within, read_json_file,
    sanitize_provider_name, write_json_file_managed, write_text_file_managed,
};
use crate::error::AppError;
use crate::model_capabilities::{image_input_capability_from_modalities, ImageInputCapability};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use toml_edit::DocumentMut;

#[allow(dead_code)]
pub const CC_SWITCH_CODEX_MODEL_PROVIDER_ID: &str = "custom";
pub const CC_SWITCH_CODEX_MODEL_CATALOG_FILENAME: &str = "cc-switch-model-catalog.json";

/// Top-level `config.toml` key that controls Codex's built-in web-search tool.
pub(crate) const CODEX_WEB_SEARCH_FIELD: &str = "web_search";
/// Value that disables the web-search tool. Some native `/responses` gateways
/// reject a `web_search` tool with `responses_feature_not_supported` ("tool type
/// 'web_search' is not supported by this gateway phase"), so for those we write
/// this per the vendors' official Codex docs. It also doubles as cc-switch's
/// ownership sentinel: only this value is removed, never a user's own setting.
pub(crate) const CODEX_WEB_SEARCH_DISABLED: &str = "disabled";

/// Native `/responses` gateways whose first-party models do not support Codex's
/// built-in `web_search` hosted tool.
const CODEX_WEB_SEARCH_REJECT_HOSTS: &[&str] = &[
    "xiaomimimo.com",
    "longcat.chat",
    "minimax.io",
    "minimaxi.com",
];

/// Brand prefixes of models whose native gateways reject `web_search`, matched
/// against the model id's last `/` segment.
const CODEX_WEB_SEARCH_REJECT_MODEL_PREFIXES: &[&str] =
    &["mimo", "longcat", "minimax", "qwen3-coder"];

fn codex_top_level_model(config_text: &str) -> Option<String> {
    let doc = config_text.parse::<toml::Value>().ok()?;
    doc.get("model")
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn codex_native_gateway_rejects_web_search(config_text: &str) -> bool {
    if let Some(base_url) = extract_codex_base_url(config_text) {
        let base_url = base_url.to_ascii_lowercase();
        if CODEX_WEB_SEARCH_REJECT_HOSTS
            .iter()
            .any(|host| base_url.contains(host))
        {
            return true;
        }
    }

    if let Some(model) = codex_top_level_model(config_text) {
        let model = model.to_ascii_lowercase();
        let model = model.rsplit('/').next().unwrap_or(model.as_str());
        if CODEX_WEB_SEARCH_REJECT_MODEL_PREFIXES
            .iter()
            .any(|prefix| model.starts_with(prefix))
        {
            return true;
        }
    }

    false
}

const CODEX_MODEL_CATALOG_TEMPLATE_SLUG: &str = "gpt-5.5";
const CODEX_MANAGED_OAUTH_LIVE_AUTH_MARKER_FILENAME: &str = "codex_managed_oauth_live_auth.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodexManagedOAuthLiveAuthMarker {
    version: u32,
    account_id: String,
}

// Consumed by the provider transaction layer (`services/provider/mod.rs`
// add/update/switch managed arms), which lands in the next batch.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CodexLiveWriteMode {
    /// Configuration owned by the external Codex install (`auth.json`,
    /// `config.toml`, the generated model catalog). ADR 0003: a valid final
    /// symlink is followed so dotfiles / NixOS ownership survives the write.
    ManagedExternal,
    /// cc-switch's own bookkeeping under the app config dir. A final symlink is
    /// rejected here, because following one would let an allowlisted filename
    /// escape the app config directory.
    CcSwitchOwned,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct CodexLiveFileState {
    path: PathBuf,
    contents: Option<Vec<u8>>,
    /// Which writer `restore` must use. Rollback has to match the forward write
    /// path for the same file: restoring a managed external file with the strict
    /// writer would silently replace the user's symlink with a regular file.
    write_mode: CodexLiveWriteMode,
    #[cfg(unix)]
    mode: Option<u32>,
}

#[allow(dead_code)]
impl CodexLiveFileState {
    fn capture(path: PathBuf, write_mode: CodexLiveWriteMode) -> Result<Self, AppError> {
        if !path.exists() {
            return Ok(Self {
                path,
                contents: None,
                write_mode,
                #[cfg(unix)]
                mode: None,
            });
        }

        let contents = fs::read(&path).map_err(|error| AppError::io(&path, error))?;
        #[cfg(unix)]
        let mode = {
            use std::os::unix::fs::PermissionsExt;
            Some(
                fs::metadata(&path)
                    .map_err(|error| AppError::io(&path, error))?
                    .permissions()
                    .mode(),
            )
        };

        Ok(Self {
            path,
            contents: Some(contents),
            write_mode,
            #[cfg(unix)]
            mode,
        })
    }

    fn restore(&self) -> Result<(), AppError> {
        match self.contents.as_deref() {
            Some(contents) => {
                match self.write_mode {
                    CodexLiveWriteMode::ManagedExternal => {
                        atomic_write_managed(&self.path, contents)?
                    }
                    CodexLiveWriteMode::CcSwitchOwned => atomic_write(&self.path, contents)?,
                }
                #[cfg(unix)]
                if let Some(mode) = self.mode {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(&self.path, fs::Permissions::from_mode(mode))
                        .map_err(|error| AppError::io(&self.path, error))?;
                }
                Ok(())
            }
            None => delete_file(&self.path),
        }
    }
}

/// Rollback point for the cc-switch-owned model catalog. Catalog projection
/// writes this file before the caller commits `config.toml`, so guarded restore
/// paths use this snapshot when a concurrently changing `auth.json` cancels the
/// commit.
#[allow(dead_code)]
pub(crate) struct CodexModelCatalogFileSnapshot(CodexLiveFileState);

#[allow(dead_code)]
impl CodexModelCatalogFileSnapshot {
    pub(crate) fn capture() -> Result<Self, AppError> {
        CodexLiveFileState::capture(
            get_codex_model_catalog_path(),
            CodexLiveWriteMode::ManagedExternal,
        )
        .map(Self)
    }

    pub(crate) fn restore(&self) -> Result<(), AppError> {
        self.0.restore()
    }
}

/// Exact rollback state for a managed Codex live write. The generated catalog
/// and ownership marker are part of the same logical commit as auth/config.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodexLiveStateSnapshot {
    auth: CodexLiveFileState,
    config: CodexLiveFileState,
    catalog: CodexLiveFileState,
    managed_marker: CodexLiveFileState,
}

#[allow(dead_code)]
impl CodexLiveStateSnapshot {
    pub(crate) fn capture() -> Result<Self, AppError> {
        Ok(Self {
            auth: CodexLiveFileState::capture(
                get_codex_auth_path(),
                CodexLiveWriteMode::ManagedExternal,
            )?,
            config: CodexLiveFileState::capture(
                get_codex_config_path(),
                CodexLiveWriteMode::ManagedExternal,
            )?,
            catalog: CodexLiveFileState::capture(
                get_codex_model_catalog_path(),
                CodexLiveWriteMode::ManagedExternal,
            )?,
            managed_marker: CodexLiveFileState::capture(
                get_codex_managed_oauth_live_auth_marker_path(),
                CodexLiveWriteMode::CcSwitchOwned,
            )?,
        })
    }

    /// Roll back config/catalog exactly while retaining a demonstrably newer
    /// ChatGPT auth generation for the same account. OAuth refresh can advance
    /// auth.json after a provider transaction captures its snapshot; restoring
    /// that snapshot blindly would invalidate the CLI's newly rotated token.
    ///
    /// Cross-account writes are still rolled back exactly: an A -> B transaction
    /// that fails must restore A even if B refreshed while it was briefly live.
    /// The marker follows auth as one generation bundle.
    pub(crate) fn restore_preserving_newer_same_account_auth(&self) -> Result<(), AppError> {
        let mut failures = Vec::new();
        let current_auth = match CodexLiveFileState::capture(
            get_codex_auth_path(),
            CodexLiveWriteMode::ManagedExternal,
        ) {
            Ok(state) => Some(state),
            Err(error) => {
                // Inspection failure must not prevent config/catalog and the
                // remaining rollback files from being attempted.
                failures.push(format!("inspect current auth: {error}"));
                None
            }
        };
        let snapshot_generation = Self::chatgpt_auth_generation(&self.auth);
        let current_generation = current_auth
            .as_ref()
            .and_then(Self::chatgpt_auth_generation);
        let preserve_current_auth = match (snapshot_generation, current_generation) {
            (Some((snapshot_account, snapshot_time)), Some((current_account, current_time)))
                if snapshot_account == current_account =>
            {
                match (snapshot_time, current_time) {
                    (Some(snapshot_time), Some(current_time)) => current_time > snapshot_time,
                    (None, Some(_)) => true,
                    _ => false,
                }
            }
            _ => false,
        };

        for (label, state) in [("catalog", &self.catalog), ("config", &self.config)] {
            if let Err(error) = state.restore() {
                failures.push(format!("{label}: {error}"));
            }
        }
        if !preserve_current_auth {
            for (label, state) in [
                ("auth", &self.auth),
                ("managed marker", &self.managed_marker),
            ] {
                if let Err(error) = state.restore() {
                    failures.push(format!("{label}: {error}"));
                }
            }
        }

        if failures.is_empty() {
            Ok(())
        } else {
            Err(AppError::Message(format!(
                "恢复 Codex Live 状态失败: {}",
                failures.join("; ")
            )))
        }
    }

    fn chatgpt_auth_generation(state: &CodexLiveFileState) -> Option<(String, Option<i64>)> {
        let auth: Value = serde_json::from_slice(state.contents.as_deref()?).ok()?;
        if auth.get("auth_mode").and_then(Value::as_str) != Some("chatgpt") {
            return None;
        }
        let account_id = auth
            .pointer("/tokens/account_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|account_id| !account_id.is_empty())?
            .to_string();
        let last_refresh_ms = auth
            .get("last_refresh")
            .and_then(Value::as_str)
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.timestamp_millis());
        Some((account_id, last_refresh_ms))
    }
}

/// Exact `auth.json` generation observed when a guarded write begins.
///
/// Thin wrapper over [`CodexLiveFileState`] so capture/permission handling stays
/// in one place; the extra `value()` accessor exists because restore
/// arbitration inspects the JSON shape (`auth_mode` / tokens) rather than bytes.
pub(crate) struct CodexAuthFileSnapshot(CodexLiveFileState);

impl CodexAuthFileSnapshot {
    pub(crate) fn capture() -> Result<Self, AppError> {
        CodexLiveFileState::capture(get_codex_auth_path(), CodexLiveWriteMode::ManagedExternal)
            .map(Self)
    }

    pub(crate) fn value(&self) -> Result<Option<Value>, AppError> {
        self.0
            .contents
            .as_deref()
            .map(serde_json::from_slice)
            .transpose()
            .map_err(|error| AppError::Message(format!("读取 Codex auth 失败: {error}")))
    }

    fn contents(&self) -> Option<&[u8]> {
        self.0.contents.as_deref()
    }

    #[cfg(unix)]
    fn mode(&self) -> Option<u32> {
        self.0.mode
    }
}

/// Owns the exact `auth.json` generation observed at restore start.
///
/// A plain compare-then-write is unsafe because Codex can replace `auth.json`
/// between those two operations. We instead atomically move the current file
/// aside ("claim"), compare the moved bytes against the expected generation, and
/// only install a replacement while the path is still vacant. A newer Codex
/// login therefore always wins, in both the forward and the rollback direction.
///
/// ADR 0003: the claim/install protocol needs `rename` + `hard_link` on the real
/// file, so the managed final symlink is resolved **once** in [`Self::begin`] and
/// every subsequent step runs against the resolved target. Renaming the link
/// itself aside would break a dotfiles/NixOS layout, which is precisely what
/// [`crate::config::atomic_write_managed`] exists to avoid.
pub(crate) struct CodexAuthFileTransaction {
    /// The managed path itself (`~/.codex/auth.json`), which may be a symlink.
    /// Deletion targets this, per ADR 0003: "deleting a managed path does not
    /// imply deleting its resolved target" — removing only the target would
    /// leave the user's dotfiles link dangling.
    managed_path: PathBuf,
    /// Resolved write target the claim/install protocol operates on.
    path: PathBuf,
    quarantined: Option<PathBuf>,
    installed: Option<Vec<u8>>,
    /// Permission bits of the claimed generation, donated to the replacement the
    /// same way [`crate::config::atomic_write_managed`] donates them.
    #[cfg(unix)]
    donated_mode: Option<u32>,
    finished: bool,
}

impl CodexAuthFileTransaction {
    pub(crate) fn begin(expected: &CodexAuthFileSnapshot) -> Result<Self, AppError> {
        let managed_path = get_codex_auth_path();
        let path = crate::config::resolve_managed_write_path(&managed_path)?;
        let mut transaction = Self {
            managed_path: managed_path.clone(),
            path: path.clone(),
            quarantined: None,
            installed: None,
            #[cfg(unix)]
            donated_mode: expected.mode(),
            finished: false,
        };

        let Some(expected_contents) = expected.contents() else {
            return match fs::read(&path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(transaction),
                Ok(_) => Err(Self::changed_error()),
                Err(error) => Err(AppError::io(&path, error)),
            };
        };

        // The no-clobber install/rollback protocol below requires hard links.
        // Probe before moving the live credentials so unsupported custom Codex
        // directories fail closed with auth.json still in place.
        //
        // The claim has to be a `rename`, not a read-copy: a read-then-compare
        // -then-write leaves a window in which Codex's own write lands between
        // the compare and the write and is then silently overwritten. Renaming
        // makes the path vacant, so the `hard_link` install below fails outright
        // if anything recreated it — the newer login always wins.
        let probe = Self::unique_sibling_path(&path, "restore-probe")?;
        match fs::hard_link(&path, &probe) {
            Ok(()) => {
                fs::remove_file(&probe).map_err(|error| AppError::IoContext {
                    context: format!("清理 Codex auth 事务能力探针失败: {}", probe.display()),
                    source: error,
                })?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(Self::changed_error());
            }
            Err(error) => {
                return Err(AppError::IoContext {
                    context: format!(
                        "Codex auth 所在文件系统不支持安全恢复，原凭据未修改: {}",
                        path.display()
                    ),
                    source: error,
                });
            }
        }

        let quarantine = Self::unique_sibling_path(&path, "restore-backup")?;
        match fs::rename(&path, &quarantine) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(Self::changed_error());
            }
            Err(error) => {
                return Err(AppError::IoContext {
                    context: format!("认领 Codex auth 失败: {}", path.display()),
                    source: error,
                });
            }
        }
        transaction.quarantined = Some(quarantine.clone());

        let actual = fs::read(&quarantine).map_err(|error| AppError::IoContext {
            context: format!("读取已认领的 Codex auth 失败: {}", quarantine.display()),
            source: error,
        })?;
        // Byte equality, deliberately *not*
        // `CodexLiveStateSnapshot::chatgpt_auth_generation` arbitration. The two
        // answer different questions and must not be unified:
        //
        //   - `restore_preserving_newer_same_account_auth` asks a policy question
        //     ("may I roll auth back to my snapshot?") and compares generations.
        //   - this asks a compare-and-swap question ("is the file still exactly
        //     what the caller arbitrated on?"). The caller already decided whether
        //     to keep or drop the live login *from these bytes*; any other content
        //     means that decision was computed against stale input.
        //
        // Relaxing this to generation equality would let a concurrent write slip
        // through the CAS and be overwritten — losing a rotation that did not bump
        // `last_refresh`, or an API-key-mode file that has no generation at all.
        // Being too strict only costs a retryable error with credentials intact.
        if actual != expected_contents {
            let restore_result = transaction.restore_quarantined_if_vacant();
            transaction.finished = true;
            return match restore_result {
                Ok(()) => Err(Self::changed_error()),
                Err(restore_error) => Err(AppError::Message(format!(
                    "{}; 恢复较新的 Codex auth 失败: {restore_error}",
                    Self::changed_error()
                ))),
            };
        }

        Ok(transaction)
    }

    /// Install `replacement` at the claimed path, or leave it vacant when `None`
    /// (an exact-generation deletion: the claim already moved the file aside and
    /// [`Self::commit`] discards the quarantined copy).
    pub(crate) fn install(&mut self, replacement: Option<Vec<u8>>) -> Result<(), AppError> {
        let Some(contents) = replacement else {
            return Ok(());
        };

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|error| AppError::io(parent, error))?;
        }
        let temporary = Self::unique_sibling_path(&self.path, "restore-new")?;
        let write_result = (|| -> Result<(), AppError> {
            let mut options = fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options
                .open(&temporary)
                .map_err(|error| AppError::io(&temporary, error))?;
            use std::io::Write;
            file.write_all(&contents)
                .and_then(|_| file.flush())
                .map_err(|error| AppError::io(&temporary, error))?;
            drop(file);
            // Donate the claimed generation's bits, matching
            // `atomic_write_resolved`. Created 0600 first so the credential is
            // never briefly world-readable.
            #[cfg(unix)]
            if let Some(mode) = self.donated_mode {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&temporary, fs::Permissions::from_mode(mode))
                    .map_err(|error| AppError::io(&temporary, error))?;
            }

            match fs::hard_link(&temporary, &self.path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    Err(Self::changed_error())
                }
                Err(error) => Err(AppError::IoContext {
                    context: format!("安装 Codex auth 失败: {}", self.path.display()),
                    source: error,
                }),
            }
        })();
        let _ = fs::remove_file(&temporary);

        if let Err(error) = write_result {
            // If Codex created a newer file while the expected generation was
            // quarantined, never put the older generation back over it.
            if self.path.exists() {
                self.discard_quarantined();
                self.finished = true;
            }
            return Err(error);
        }

        self.installed = Some(contents);
        Ok(())
    }

    pub(crate) fn commit(mut self) -> Result<(), AppError> {
        self.discard_quarantined();
        // Nothing installed means the caller asked for an exact-generation
        // deletion. The claim removed the resolved target, so a managed symlink
        // would be left dangling; ADR 0003 says deletion applies to the managed
        // path itself, which is also what W1's
        // `clear_codex_live_auth_for_managed_account_if_unchanged` does.
        if self.installed.is_none() && self.managed_path != self.path {
            match fs::remove_file(&self.managed_path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    self.finished = true;
                    return Err(AppError::io(&self.managed_path, error));
                }
            }
        }
        self.finished = true;
        Ok(())
    }

    pub(crate) fn rollback(mut self) -> Result<(), AppError> {
        let result = self.rollback_inner();
        self.finished = true;
        result
    }

    fn rollback_inner(&mut self) -> Result<(), AppError> {
        if let Some(installed) = self.installed.take() {
            let replacement_quarantine = Self::unique_sibling_path(&self.path, "restore-rollback")?;
            match fs::rename(&self.path, &replacement_quarantine) {
                Ok(()) => {
                    let current =
                        fs::read(&replacement_quarantine).map_err(|error| AppError::IoContext {
                            context: format!(
                                "读取待回滚 Codex auth 失败: {}",
                                replacement_quarantine.display()
                            ),
                            source: error,
                        })?;
                    if current == installed {
                        fs::remove_file(&replacement_quarantine).map_err(|error| {
                            AppError::IoContext {
                                context: format!(
                                    "删除待回滚 Codex auth 失败: {}",
                                    replacement_quarantine.display()
                                ),
                                source: error,
                            }
                        })?;
                    } else {
                        // Codex replaced our installed generation. Restore that
                        // newer file if the path is still vacant and discard the
                        // old expected generation.
                        Self::restore_file_if_vacant(&replacement_quarantine, &self.path)?;
                        self.discard_quarantined();
                        return Ok(());
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    // A concurrent logout removed our generation. Missing auth
                    // is newer state too; do not resurrect the old credentials.
                    self.discard_quarantined();
                    return Ok(());
                }
                Err(error) => {
                    return Err(AppError::IoContext {
                        context: format!("回滚 Codex auth 失败: {}", self.path.display()),
                        source: error,
                    });
                }
            }
        }

        self.restore_quarantined_if_vacant()
    }

    fn restore_quarantined_if_vacant(&mut self) -> Result<(), AppError> {
        let Some(quarantined) = self.quarantined.take() else {
            return Ok(());
        };
        Self::restore_file_if_vacant(&quarantined, &self.path)
    }

    fn restore_file_if_vacant(source: &Path, destination: &Path) -> Result<(), AppError> {
        match fs::hard_link(source, destination) {
            Ok(()) => {
                fs::remove_file(source).map_err(|error| AppError::IoContext {
                    context: format!("清理 Codex auth 事务文件失败: {}", source.display()),
                    source: error,
                })?;
                Ok(())
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                // The destination was recreated by Codex and is newer than the
                // quarantined generation.
                fs::remove_file(source).map_err(|error| AppError::IoContext {
                    context: format!("清理旧 Codex auth 事务文件失败: {}", source.display()),
                    source: error,
                })?;
                Ok(())
            }
            Err(error) => Err(AppError::IoContext {
                context: format!(
                    "恢复 Codex auth 事务文件失败: {} -> {}",
                    source.display(),
                    destination.display()
                ),
                source: error,
            }),
        }
    }

    fn discard_quarantined(&mut self) {
        if let Some(path) = self.quarantined.take() {
            if let Err(error) = fs::remove_file(&path) {
                log::warn!(
                    "清理旧 Codex auth 事务文件失败 ({}): {error}",
                    path.display()
                );
            }
        }
    }

    fn unique_sibling_path(path: &Path, label: &str) -> Result<PathBuf, AppError> {
        let parent = path.parent().ok_or_else(|| {
            AppError::Config(format!("无效的 Codex auth 路径: {}", path.display()))
        })?;
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("auth.json");
        Ok(parent.join(format!(
            ".{file_name}.cc-switch-{label}-{}",
            uuid::Uuid::new_v4()
        )))
    }

    fn changed_error() -> AppError {
        AppError::Message(
            "Codex auth 在恢复期间发生变化；为避免覆盖新凭据，本次恢复已取消，请重试".to_string(),
        )
    }
}

impl Drop for CodexAuthFileTransaction {
    fn drop(&mut self) {
        if !self.finished {
            if let Err(error) = self.rollback_inner() {
                log::error!("Codex auth 事务自动回滚失败: {error}");
            }
        }
    }
}

/// Which Codex tool surface the generated model catalog should target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexCatalogToolProfile {
    ProxyChat,
    NativeResponses,
}

impl CodexCatalogToolProfile {
    pub fn from_api_format(api_format: Option<&str>) -> Self {
        match api_format {
            Some("openai_responses") => CodexCatalogToolProfile::NativeResponses,
            _ => CodexCatalogToolProfile::ProxyChat,
        }
    }
}

/// Reserved built-in provider IDs from OpenAI Codex's config/model-provider
/// catalog. Keep in sync with Codex `RESERVED_MODEL_PROVIDER_IDS` and legacy
/// removed provider aliases.
const CODEX_RESERVED_MODEL_PROVIDER_IDS: &[&str] = &[
    "amazon-bedrock",
    "openai",
    "ollama",
    "lmstudio",
    "oss",
    "ollama-chat",
];

/// 获取 Codex 配置目录路径
pub fn get_codex_config_dir() -> PathBuf {
    if let Some(custom) = crate::settings::get_codex_override_dir() {
        return custom;
    }

    get_home_dir().join(".codex")
}

/// 获取 Codex auth.json 路径
pub fn get_codex_auth_path() -> PathBuf {
    get_codex_config_dir().join("auth.json")
}

fn get_codex_managed_oauth_live_auth_marker_path() -> PathBuf {
    crate::config::get_app_config_dir().join(CODEX_MANAGED_OAUTH_LIVE_AUTH_MARKER_FILENAME)
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn codex_managed_oauth_live_auth_marker_exists() -> bool {
    get_codex_managed_oauth_live_auth_marker_path().exists()
}

/// 从 live/备份的 Codex `auth` 中提取 `account_id`，用于 marker 记录/比对。
///
/// 仅接受 ChatGPT 登录形状（`auth_mode == "chatgpt"`、`OPENAI_API_KEY` 可清空）。
/// 托管账号写入的完整 bundle 会额外带 `tokens.refresh_token` 与顶层 `last_refresh`，
/// 这里一并容忍。所有权按 account-scoped 内容判断；Codex CLI 自刷新会轮换
/// access_token，因此短期 token 指纹不能作为稳定的删除谓词。
fn extract_codex_managed_oauth_account_id(auth: &Value) -> Option<String> {
    let auth_obj = auth.as_object()?;

    if auth_obj.keys().any(|key| {
        !matches!(
            key.as_str(),
            "auth_mode" | "OPENAI_API_KEY" | "tokens" | "last_refresh"
        )
    }) {
        return None;
    }

    if auth.get("auth_mode").and_then(|value| value.as_str()) != Some("chatgpt") {
        return None;
    }

    let api_key_is_clearable = auth
        .get("OPENAI_API_KEY")
        .is_none_or(|value| value.is_null() || value.as_str() == Some("PROXY_MANAGED"));
    if !api_key_is_clearable {
        return None;
    }

    let tokens = auth.get("tokens").and_then(|value| value.as_object())?;

    if tokens.keys().any(|key| {
        !matches!(
            key.as_str(),
            "access_token" | "account_id" | "id_token" | "refresh_token"
        )
    }) {
        return None;
    }

    let account_id = tokens
        .get("account_id")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|id| !id.is_empty())?;
    tokens
        .get("access_token")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|token| !token.is_empty())?;

    Some(account_id.to_string())
}

/// Build the native-shaped ChatGPT auth bundle shared by cc-switch and Codex CLI.
pub fn codex_managed_oauth_auth_value(
    account_id: &str,
    access_token: &str,
    id_token: Option<&str>,
    refresh_token: &str,
    last_refresh: &str,
) -> Value {
    let mut tokens = serde_json::Map::new();
    if let Some(id_token) = id_token {
        tokens.insert("id_token".to_string(), Value::String(id_token.to_string()));
    }
    tokens.insert(
        "access_token".to_string(),
        Value::String(access_token.to_string()),
    );
    tokens.insert(
        "refresh_token".to_string(),
        Value::String(refresh_token.to_string()),
    );
    tokens.insert(
        "account_id".to_string(),
        Value::String(account_id.to_string()),
    );
    json!({
        "auth_mode": "chatgpt",
        "OPENAI_API_KEY": null,
        "tokens": Value::Object(tokens),
        "last_refresh": last_refresh,
    })
}

pub fn record_codex_managed_oauth_live_auth(auth: &Value) -> Result<(), AppError> {
    let Some(account_id) = extract_codex_managed_oauth_account_id(auth) else {
        return Ok(());
    };

    let marker = CodexManagedOAuthLiveAuthMarker {
        version: 2,
        account_id,
    };
    crate::config::write_json_file(&get_codex_managed_oauth_live_auth_marker_path(), &marker)
}

pub fn codex_auth_matches_recorded_managed_oauth(
    auth: &Value,
    account_id: &str,
) -> Result<bool, AppError> {
    let account_id = account_id.trim();
    if account_id.is_empty() {
        return Ok(false);
    }

    let Some(auth_account_id) = extract_codex_managed_oauth_account_id(auth) else {
        return Ok(false);
    };
    if auth_account_id != account_id {
        return Ok(false);
    }

    let marker_path = get_codex_managed_oauth_live_auth_marker_path();
    let marker: CodexManagedOAuthLiveAuthMarker = match read_json_file(&marker_path) {
        Ok(marker) => marker,
        Err(err) => {
            log::warn!(
                "Failed to read Codex managed OAuth auth marker at {}: {err}",
                marker_path.display()
            );
            return Ok(false);
        }
    };

    // v1 markers also carry an access-token fingerprint. Serde ignores that
    // legacy extra field, and matching intentionally no longer consults it:
    // the Codex CLI rotates access tokens during normal self-refresh.
    Ok(matches!(marker.version, 1 | 2) && marker.account_id == account_id)
}

fn clear_codex_managed_oauth_live_auth_marker_for_account(
    account_id: &str,
) -> Result<(), AppError> {
    let marker_path = get_codex_managed_oauth_live_auth_marker_path();
    if !marker_path.exists() {
        return Ok(());
    }
    let marker: CodexManagedOAuthLiveAuthMarker = match read_json_file(&marker_path) {
        Ok(marker) => marker,
        Err(error) => {
            log::warn!(
                "Failed to read Codex managed OAuth auth marker at {} while cleaning account {}: {error}",
                marker_path.display(),
                account_id
            );
            // A malformed marker cannot establish ownership for any account
            // and is unusable for rollback/synchronization; remove the stale
            // bookkeeping file while leaving non-matching live auth untouched.
            return delete_file(&marker_path);
        }
    };
    if marker.account_id == account_id.trim() {
        delete_file(&marker_path)?;
    }
    Ok(())
}

/// 切走托管 provider 或从认证中心删除账号时，清理其残留在
/// `~/.codex/auth.json` 的 ChatGPT 登录。
///
/// 删除谓词按 `auth_mode + tokens.account_id` 的内容判断，而不依赖会被 Codex CLI
/// 自刷新破坏的 access-token 指纹。同一账号的原生 `codex login` 也会被视为该账号
/// 的登录；切换路径必须先把盘上轮换后的 refresh token 采纳回 manager，再调用本函数，
/// 因而这种 account-scoped 取舍不会丢失凭据。认证中心显式删除/登出则有意移除它。
pub fn clear_codex_live_auth_for_managed_account(account_id: &str) -> Result<(), AppError> {
    clear_codex_live_auth_for_managed_account_if_unchanged(account_id, None)
}

/// Verify that the outgoing account's live refresh generation has not changed
/// since it was adopted into the OAuth manager.
#[allow(dead_code)]
pub fn ensure_codex_live_auth_unchanged_for_managed_account(
    account_id: &str,
    expected_refresh_token: &str,
) -> Result<(), AppError> {
    let auth_path = get_codex_auth_path();
    if !auth_path.exists() {
        return Err(AppError::Message(format!(
            "Codex CLI 账号 {account_id} 的 live auth 已在切换期间被移除，请重试"
        )));
    }
    let auth: Value = read_json_file(&auth_path)?;
    let current_refresh_token = auth
        .pointer("/tokens/refresh_token")
        .and_then(Value::as_str)
        .map(str::trim);
    if !codex_live_auth_is_managed_chatgpt_login(&auth, account_id)
        || current_refresh_token != Some(expected_refresh_token.trim())
    {
        return Err(AppError::Message(format!(
            "Codex CLI 账号 {account_id} 的 live 凭据在切换期间已刷新；为避免覆盖新 refresh token，本次操作已取消，请重试"
        )));
    }
    Ok(())
}

/// Content-based cleanup with an optional compare-before-delete guard.
pub fn clear_codex_live_auth_for_managed_account_if_unchanged(
    account_id: &str,
    expected_refresh_token: Option<&str>,
) -> Result<(), AppError> {
    let auth_path = get_codex_auth_path();
    let mut removed_matching_auth = false;
    if auth_path.exists() {
        let auth: Value = read_json_file(&auth_path)?;
        if codex_live_auth_is_managed_chatgpt_login(&auth, account_id) {
            if let Some(expected_refresh_token) = expected_refresh_token {
                let current_refresh_token = auth
                    .pointer("/tokens/refresh_token")
                    .and_then(Value::as_str)
                    .map(str::trim);
                if current_refresh_token != Some(expected_refresh_token.trim()) {
                    return Err(AppError::Message(format!(
                        "Codex CLI 账号 {account_id} 的 live 凭据在切换期间已刷新；为避免删除新 refresh token，本次操作已取消，请重试"
                    )));
                }
            }
            delete_file(&auth_path)?;
            removed_matching_auth = true;
        }
    }

    if removed_matching_auth {
        // Once the matching live file is gone, any marker is stale regardless
        // of version or parseability.
        delete_file(&get_codex_managed_oauth_live_auth_marker_path())?;
    } else {
        clear_codex_managed_oauth_live_auth_marker_for_account(account_id)?;
    }
    Ok(())
}

/// 判断给定的 Codex `auth`（来自 live auth.json 或 Live 备份）是否是「属于
/// `account_id` 的 ChatGPT 托管登录」。
///
/// 托管账号写入的是**完整可刷新 bundle**，与原生浏览器登录形状一致（都含
/// refresh_token），且 Codex CLI 会轮换 token 使旧的 access_token 指纹失效，因此
/// 无法再凭形状/哈希区分。这里采用**基于内容的 account_id 判定**：只要是 chatgpt
/// 模式、且 `tokens.account_id` 命中托管账号，即视为该账号的登录。对同一账号的原生
/// 登录会被同等处理（同账号，无损）。
///
/// 用于 Live 备份剥离：避免把托管账号的可刷新 token 持久化进备份配置。
pub fn codex_live_auth_is_managed_chatgpt_login(auth: &Value, account_id: &str) -> bool {
    let account_id = account_id.trim();
    if account_id.is_empty() {
        return false;
    }
    let Some(obj) = auth.as_object() else {
        return false;
    };
    if obj.get("auth_mode").and_then(|value| value.as_str()) != Some("chatgpt") {
        return false;
    }
    let api_key_clearable = obj
        .get("OPENAI_API_KEY")
        .is_none_or(|value| value.is_null() || value.as_str() == Some("PROXY_MANAGED"));
    if !api_key_clearable {
        return false;
    }
    obj.get("tokens")
        .and_then(|tokens| tokens.as_object())
        .and_then(|tokens| tokens.get("account_id"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        == Some(account_id)
}

/// 读回 Codex CLI 当前 `~/.codex/auth.json` 中属于 `account_id` 的 refresh_token /
/// id_token（仅当磁盘上的登录账号与之一致时）。
///
/// 用于切换回托管 provider 前，采纳 CLI 自行刷新时轮换出的最新 refresh_token，避免
/// 用陈腐 token 覆盖 CLI 的有效登录（“裸跑 codex” 反复切换场景）。
pub fn read_codex_live_auth_refresh_for_account(
    account_id: &str,
) -> Option<(String, Option<String>, Option<i64>)> {
    let account_id = account_id.trim();
    if account_id.is_empty() {
        return None;
    }
    let auth_path = get_codex_auth_path();
    if !auth_path.exists() {
        return None;
    }
    let auth: Value = read_json_file(&auth_path).ok()?;
    // 仅在磁盘上确是「该 account_id 的 ChatGPT 登录」时才采纳其 refresh_token，
    // 避免从非 chatgpt/异常 auth 里误取 token。
    if !codex_live_auth_is_managed_chatgpt_login(&auth, account_id) {
        return None;
    }
    let tokens = auth.get("tokens")?.as_object()?;
    let refresh_token = tokens.get("refresh_token")?.as_str()?.trim().to_string();
    if refresh_token.is_empty() {
        return None;
    }
    let id_token = tokens
        .get("id_token")
        .and_then(|value| value.as_str())
        .map(|token| token.to_string());
    let last_refresh_ms = auth
        .get("last_refresh")
        .and_then(Value::as_str)
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.timestamp_millis());
    Some((refresh_token, id_token, last_refresh_ms))
}

/// Keep Codex CLI's live auth in the same refresh-token generation after the
/// manager refreshes a managed account.
///
/// The write is compare-and-swap-like: immediately before replacing auth.json,
/// it verifies that the file still contains the refresh token used for the
/// network request. Codex CLI does not share cc-switch's process lock, so this
/// is a best-effort guard that narrows (but cannot make atomic) the cross-process
/// check-to-replace window.
/// Ownership is account-scoped: a file recorded for the same managed account
/// keeps its marker across access-token rotation. A same-account native login
/// has the same content identity and is intentionally treated equivalently.
pub fn sync_codex_managed_oauth_live_auth_after_refresh(
    account_id: &str,
    expected_refresh_token: &str,
    refreshed_auth: &Value,
) -> Result<bool, AppError> {
    let account_id = account_id.trim();
    let expected_refresh_token = expected_refresh_token.trim();
    if account_id.is_empty() || expected_refresh_token.is_empty() {
        return Ok(false);
    }

    let auth_path = get_codex_auth_path();
    if !auth_path.exists() {
        return Ok(false);
    }
    let current_auth: Value = read_json_file(&auth_path)?;
    if !codex_live_auth_is_managed_chatgpt_login(&current_auth, account_id) {
        return Ok(false);
    }
    let current_refresh_token = current_auth
        .pointer("/tokens/refresh_token")
        .and_then(Value::as_str)
        .map(str::trim);
    if current_refresh_token != Some(expected_refresh_token) {
        return Ok(false);
    }

    let marker_path = get_codex_managed_oauth_live_auth_marker_path();
    let was_recorded_managed = marker_path.exists()
        && codex_auth_matches_recorded_managed_oauth(&current_auth, account_id)?;

    // ADR 0003: `~/.codex/auth.json` is owned by the external Codex install, so
    // the managed writer follows a valid final symlink instead of replacing it.
    write_json_file_managed(&auth_path, refreshed_auth)?;
    if was_recorded_managed {
        record_codex_managed_oauth_live_auth(refreshed_auth)?;
    }
    Ok(true)
}

/// 获取 Codex config.toml 路径
pub fn get_codex_config_path() -> PathBuf {
    get_codex_config_dir().join("config.toml")
}

pub fn get_codex_model_catalog_path() -> PathBuf {
    get_codex_config_dir().join(CC_SWITCH_CODEX_MODEL_CATALOG_FILENAME)
}

/// 获取 Codex 供应商配置文件路径
#[allow(dead_code)]
pub fn get_codex_provider_paths(
    provider_id: &str,
    provider_name: Option<&str>,
) -> (PathBuf, PathBuf) {
    let base_name = provider_name
        .map(sanitize_provider_name)
        .unwrap_or_else(|| sanitize_provider_name(provider_id));

    let auth_path = get_codex_config_dir().join(format!("auth-{base_name}.json"));
    let config_path = get_codex_config_dir().join(format!("config-{base_name}.toml"));

    (auth_path, config_path)
}

/// 删除 Codex 供应商配置文件
#[allow(dead_code)]
pub fn delete_codex_provider_config(
    provider_id: &str,
    provider_name: &str,
) -> Result<(), AppError> {
    let (auth_path, config_path) = get_codex_provider_paths(provider_id, Some(provider_name));

    delete_file(&auth_path).ok();
    delete_file(&config_path).ok();

    Ok(())
}

/// 原子写 Codex 的 `auth.json` 与 `config.toml`，在第二步失败时回滚第一步
pub fn write_codex_live_atomic(
    auth: &Value,
    config_text_opt: Option<&str>,
) -> Result<(), AppError> {
    let auth_path = get_codex_auth_path();
    let config_path = get_codex_config_path();

    if let Some(parent) = auth_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }

    // 读取旧内容用于回滚。config.toml 是第二步（最后）写入的，写它失败时
    // auth.json 才需要回滚；config.toml 自身无需回滚，因此不预读它（L15：
    // 删除原先未被使用的 _old_config 死读）。
    let old_auth = if auth_path.exists() {
        Some(fs::read(&auth_path).map_err(|e| AppError::io(&auth_path, e))?)
    } else {
        None
    };

    // 准备写入内容
    let cfg_text = match config_text_opt {
        Some(s) => s.to_string(),
        None => String::new(),
    };
    if !cfg_text.trim().is_empty() {
        toml::from_str::<toml::Table>(&cfg_text).map_err(|e| AppError::toml(&config_path, e))?;
    }

    // 第一步：写 auth.json
    write_json_file_managed(&auth_path, auth)?;

    // 第二步：写 config.toml（失败则回滚 auth.json）
    if let Err(e) = write_text_file_managed(&config_path, &cfg_text) {
        // 回滚 auth.json。回滚本身失败时不要静默吞掉——记录日志，否则
        // auth.json/config.toml 会不一致且无从排查（L15）。
        let rollback = if let Some(bytes) = old_auth {
            atomic_write_managed(&auth_path, &bytes)
        } else {
            delete_file(&auth_path)
        };
        if let Err(rollback_err) = rollback {
            log::error!(
                "codex auth.json rollback failed after config.toml write error \
                 (auth/config now inconsistent): {rollback_err}"
            );
        }
        return Err(e);
    }

    Ok(())
}

/// 读取 `~/.codex/config.toml`，若不存在返回空字符串
pub fn read_codex_config_text() -> Result<String, AppError> {
    let path = get_codex_config_path();
    if path.exists() {
        std::fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))
    } else {
        Ok(String::new())
    }
}

/// 对非空的 TOML 文本进行语法校验
pub fn validate_config_toml(text: &str) -> Result<(), AppError> {
    if text.trim().is_empty() {
        return Ok(());
    }
    toml::from_str::<toml::Table>(text)
        .map(|_| ())
        .map_err(|e| AppError::toml(Path::new("config.toml"), e))
}

/// 读取并校验 `~/.codex/config.toml`，返回文本（可能为空）
pub fn read_and_validate_codex_config_text() -> Result<String, AppError> {
    let s = read_codex_config_text()?;
    validate_config_toml(&s)?;
    Ok(s)
}

fn active_codex_model_provider_id(doc: &DocumentMut) -> Option<String> {
    doc.get("model_provider")
        .and_then(|item| item.as_str())
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
}

pub(crate) fn is_custom_codex_model_provider_id(id: &str) -> bool {
    let id = id.trim();
    !id.is_empty()
        && !CODEX_RESERVED_MODEL_PROVIDER_IDS
            .iter()
            .any(|reserved| reserved.eq_ignore_ascii_case(id))
}

/// Write only Codex `config.toml` for provider switching.
///
/// Codex login state lives in `auth.json`; provider routing, endpoint, model,
/// and provider-scoped bearer tokens live in `config.toml`. Provider switches
/// should not overwrite the user's ChatGPT login cache.
pub fn write_codex_live_config_atomic(config_text_opt: Option<&str>) -> Result<(), AppError> {
    let config_path = get_codex_config_path();
    let cfg_text = match config_text_opt {
        Some(config_text) => config_text.to_string(),
        None => String::new(),
    };

    if !cfg_text.trim().is_empty() {
        toml::from_str::<toml::Table>(&cfg_text).map_err(|e| AppError::toml(&config_path, e))?;
    }

    write_text_file_managed(&config_path, &cfg_text)
}

pub fn extract_codex_auth_api_key(auth: &Value) -> Option<String> {
    auth.get("OPENAI_API_KEY")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(str::to_string)
}

pub fn extract_codex_api_key(auth: Option<&Value>, config_text: Option<&str>) -> Option<String> {
    auth.and_then(extract_codex_auth_api_key)
        .or_else(|| config_text.and_then(extract_codex_experimental_bearer_token))
}

/// Extract the upstream base URL from a Codex `config.toml` string.
///
/// Prefers the active `[model_providers.<model_provider>].base_url`, falling
/// back to a top-level `base_url`.
pub fn extract_codex_base_url(config_text: &str) -> Option<String> {
    let doc = config_text.parse::<toml::Value>().ok()?;

    if let Some(active_provider) = doc.get("model_provider").and_then(|v| v.as_str()) {
        if let Some(base_url) = doc
            .get("model_providers")
            .and_then(|providers| providers.get(active_provider))
            .and_then(|provider| provider.get("base_url"))
            .and_then(|v| v.as_str())
        {
            return Some(base_url.to_string());
        }
    }

    doc.get("base_url")
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
}

pub fn codex_auth_has_login_material(auth: &Value) -> bool {
    let Some(obj) = auth.as_object() else {
        return false;
    };

    obj.iter().any(|(key, value)| {
        if key == "auth_mode" {
            return false;
        }

        if key == "OPENAI_API_KEY" {
            return value
                .as_str()
                .map(str::trim)
                .is_some_and(|token| !token.is_empty());
        }

        match value {
            Value::Null => false,
            Value::String(text) => !text.trim().is_empty(),
            Value::Array(items) => !items.is_empty(),
            Value::Object(map) => !map.is_empty(),
            _ => true,
        }
    })
}

pub fn codex_auth_has_oauth_login_material(auth: &Value) -> bool {
    let Some(obj) = auth.as_object() else {
        return false;
    };

    obj.iter().any(|(key, value)| {
        if key == "auth_mode" || key == "OPENAI_API_KEY" {
            return false;
        }

        match value {
            Value::Null => false,
            Value::String(text) => !text.trim().is_empty(),
            Value::Array(items) => !items.is_empty(),
            Value::Object(map) => !map.is_empty(),
            _ => true,
        }
    })
}

/// Detect first-class Codex login credentials without treating metadata or a
/// third-party OPENAI_API_KEY as an official login.
pub fn codex_auth_has_credential_login_material(auth: &Value) -> bool {
    let Some(obj) = auth.as_object() else {
        return false;
    };

    let value_present = |value: &Value| match value {
        Value::Null => false,
        Value::String(text) => !text.trim().is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(map) => !map.is_empty(),
        _ => true,
    };

    if ["personal_access_token", "agent_identity", "bedrock_api_key"]
        .iter()
        .any(|key| obj.get(*key).is_some_and(value_present))
    {
        return true;
    }

    obj.get("tokens")
        .and_then(Value::as_object)
        .is_some_and(|tokens| {
            ["id_token", "access_token", "refresh_token"]
                .iter()
                .any(|key| tokens.get(*key).is_some_and(value_present))
        })
}

pub fn codex_live_auth_is_stale_third_party_residue(live_auth: &Value) -> bool {
    if codex_auth_has_credential_login_material(live_auth) {
        return false;
    }
    live_auth
        .get("OPENAI_API_KEY")
        .and_then(Value::as_str)
        .map(str::trim)
        .is_some_and(|key| !key.is_empty())
}

/// Delete a stale third-party live key after it has been safely backfilled and
/// the switch to a material-less official provider has completed. A missing
/// auth.json makes Codex show its login screen; writing `{}` does not.
pub fn clear_stale_codex_live_auth_after_official_switch(
    db_auth: &Value,
) -> Result<bool, AppError> {
    if codex_auth_has_login_material(db_auth) {
        return Ok(false);
    }

    let auth_path = get_codex_auth_path();
    if !auth_path.exists() {
        return Ok(false);
    }
    let live_auth: Value = read_json_file(&auth_path)?;
    if !codex_live_auth_is_stale_third_party_residue(&live_auth) {
        return Ok(false);
    }
    delete_file(&auth_path)?;
    Ok(true)
}

pub fn should_restore_codex_provider_token_for_backfill(
    category: Option<&str>,
    template_settings: &Value,
) -> bool {
    if category == Some("official") {
        return false;
    }

    let Some(auth) = template_settings.get("auth") else {
        return true;
    };

    let has_provider_api_key = extract_codex_auth_api_key(auth).is_some();
    let has_oauth_login = codex_auth_has_oauth_login_material(auth);
    !has_oauth_login || has_provider_api_key
}

fn parse_codex_positive_u64(value: Option<&Value>) -> Option<u64> {
    match value {
        Some(Value::Number(n)) => n.as_u64().filter(|v| *v > 0),
        Some(Value::String(s)) => s.trim().parse::<u64>().ok().filter(|v| *v > 0),
        _ => None,
    }
}

fn extract_codex_top_level_u64(config_text: &str, field: &str) -> Option<u64> {
    let doc = config_text.parse::<toml::Value>().ok()?;
    doc.get(field)
        .and_then(|value| value.as_integer())
        .and_then(|value| u64::try_from(value).ok())
        .filter(|value| *value > 0)
}

fn codex_catalog_input_modalities(
    model: &str,
    declared_modalities: Option<&[String]>,
) -> Vec<String> {
    let modalities = match image_input_capability_from_modalities(model, declared_modalities) {
        ImageInputCapability::Unsupported => &["text"][..],
        ImageInputCapability::Supported | ImageInputCapability::Unknown => &["text", "image"][..],
    };
    modalities.iter().map(|item| (*item).to_string()).collect()
}

/// Canonical reasoning effort levels Codex understands, with the same
/// descriptions the official gpt-5.5 template uses. `none` disables thinking.
const CODEX_REASONING_LEVEL_DESCRIPTIONS: &[(&str, &str)] = &[
    ("none", "Disable Thinking"),
    ("minimal", "Minimal reasoning"),
    ("low", "Fast responses with lighter reasoning"),
    (
        "medium",
        "Balances speed and reasoning depth for everyday tasks",
    ),
    ("high", "Greater reasoning depth for complex problems"),
    ("xhigh", "Extra high reasoning depth for complex problems"),
    ("max", "Maximum reasoning depth for the hardest problems"),
    ("ultra", "Ultra reasoning depth"),
];

fn codex_reasoning_level_description(effort: &str) -> Option<&'static str> {
    CODEX_REASONING_LEVEL_DESCRIPTIONS
        .iter()
        .find(|(candidate, _)| *candidate == effort)
        .map(|(_, description)| *description)
}

/// User-declared levels reduced to the canonical efforts Codex understands,
/// in canonical (lowest → highest) order regardless of declaration order.
/// Unknown efforts are dropped so a typo can never produce an entry Codex
/// would reject.
fn codex_canonical_efforts(levels: &[String]) -> Vec<&str> {
    CODEX_REASONING_LEVEL_DESCRIPTIONS
        .iter()
        .filter(|(effort, _)| levels.iter().any(|candidate| candidate == effort))
        .map(|(effort, _)| *effort)
        .collect()
}

/// Build a `supported_reasoning_levels` array from user-declared effort values.
fn codex_supported_reasoning_levels(levels: &[String]) -> Value {
    let entries: Vec<Value> = codex_canonical_efforts(levels)
        .into_iter()
        .map(|effort| {
            let description = codex_reasoning_level_description(effort)
                .expect("canonical effort always has a description");
            json!({ "effort": effort, "description": description })
        })
        .collect();
    json!(entries)
}

/// Apply a per-model reasoning-level override onto a catalog entry. Returns
/// true when the override was applied (so callers can skip further work).
/// `template_default` is the base entry's `default_reasoning_level` (from the
/// profile template or an official vendor entry) used as the fallback when the
/// user did not declare one explicitly.
fn apply_codex_reasoning_level_override(
    entry_obj: &mut serde_json::Map<String, Value>,
    template_default: Option<&str>,
    spec: &CodexCatalogModelSpec,
) -> bool {
    let Some(levels) = spec.reasoning_levels.as_deref() else {
        return false;
    };
    let canonical = codex_canonical_efforts(levels);
    if canonical.is_empty() {
        return false;
    }
    let supported = codex_supported_reasoning_levels(levels);
    entry_obj.insert("supported_reasoning_levels".to_string(), supported);

    // Default: explicit user value wins; otherwise keep the base default when
    // it is still supported; otherwise fall back to the highest supported
    // level in canonical order. All candidates are validated against the
    // canonical set so the default can never reference a dropped effort.
    let default_level = spec
        .default_reasoning_level
        .as_deref()
        .filter(|level| canonical.contains(level))
        .or_else(|| template_default.filter(|level| canonical.contains(level)))
        .or_else(|| canonical.last().copied());
    if let Some(default_level) = default_level {
        entry_obj.insert("default_reasoning_level".to_string(), json!(default_level));
    }
    true
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CodexCatalogModelSpec {
    model: String,
    /// Explicit user value only. Synthetic catalog entries fall back to the
    /// model id; official vendor entries retain the vendor display name.
    display_name: Option<String>,
    /// Explicit user value only. Synthetic entries fall back to the active
    /// config's context window; official entries retain the vendor value.
    context_window: Option<u64>,
    supports_parallel_tool_calls: Option<bool>,
    /// Hidden per-row capability declaration from built-in provider metadata.
    /// When omitted, all catalog profiles consult the shared text-only model
    /// registry and otherwise default to `["text", "image"]`.
    input_modalities: Option<Vec<String>>,
    base_instructions: Option<String>,
    /// Per-row override for the generated catalog's `supported_reasoning_levels`
    /// (e.g. ["none", "low", "medium", "high", "xhigh", "max"]). When omitted
    /// the template's conservative default (none/high) is kept. Consulted for
    /// every profile; the vendor-catalog path applies it on top of the
    /// official entry.
    reasoning_levels: Option<Vec<String>>,
    /// Per-row override for the generated catalog's `default_reasoning_level`.
    /// Only meaningful together with `reasoning_levels`; when absent the
    /// template default is kept if it is still in the list, otherwise the last
    /// (highest) declared level wins.
    default_reasoning_level: Option<String>,
}

fn codex_catalog_model_entry(
    template: &Value,
    spec: &CodexCatalogModelSpec,
    priority: usize,
    profile: CodexCatalogToolProfile,
    default_context_window: u64,
) -> Value {
    let mut entry = template.clone();
    let Some(entry_obj) = entry.as_object_mut() else {
        return json!({});
    };

    let display_name = spec.display_name.as_deref().unwrap_or(&spec.model);
    let context_window = spec.context_window.unwrap_or(default_context_window);
    entry_obj.insert("slug".to_string(), json!(spec.model));
    entry_obj.insert("display_name".to_string(), json!(display_name));
    entry_obj.insert("description".to_string(), json!(display_name));
    entry_obj.insert("context_window".to_string(), json!(context_window));
    entry_obj.insert("max_context_window".to_string(), json!(context_window));
    entry_obj.insert("priority".to_string(), json!(1000 + priority));
    entry_obj.insert("additional_speed_tiers".to_string(), json!([]));
    entry_obj.insert("service_tiers".to_string(), json!([]));
    entry_obj.insert("availability_nux".to_string(), Value::Null);
    entry_obj.insert("upgrade".to_string(), Value::Null);

    // Image support is a model capability, not a tool-profile capability.
    // Trust hidden preset metadata first, then the confirmed text-only registry;
    // every unknown model fails open so GPT/relay aliases are never declared
    // text-only merely because a template had a conservative default.
    entry_obj.insert(
        "input_modalities".to_string(),
        json!(codex_catalog_input_modalities(
            &spec.model,
            spec.input_modalities.as_deref(),
        )),
    );

    if profile == CodexCatalogToolProfile::NativeResponses {
        for key in [
            "apply_patch_tool_type",
            "web_search_tool_type",
            "tools",
            "model_messages",
        ] {
            entry_obj.remove(key);
        }
        entry_obj.insert("shell_type".to_string(), json!("shell_command"));

        if let Some(base_instructions) = spec
            .base_instructions
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            entry_obj.insert("base_instructions".to_string(), json!(base_instructions));
        }
        if let Some(parallel) = spec.supports_parallel_tool_calls {
            entry_obj.insert("supports_parallel_tool_calls".to_string(), json!(parallel));
        }
    }

    // Per-model reasoning levels override the template's conservative
    // none/high default (e.g. a LiteLLM gateway serving a model that accepts
    // low/medium/high/xhigh/max). Applies to every profile.
    let template_default = template
        .get("default_reasoning_level")
        .and_then(|value| value.as_str());
    apply_codex_reasoning_level_override(entry_obj, template_default, spec);

    entry
}

fn codex_catalog_model_specs(settings: &Value) -> Vec<CodexCatalogModelSpec> {
    let Some(models) = settings
        .get("modelCatalog")
        .and_then(|catalog| catalog.get("models"))
        .and_then(|models| models.as_array())
    else {
        return Vec::new();
    };

    let mut seen = HashSet::new();
    let mut specs = Vec::new();

    for model_config in models {
        let Some(model) = model_config
            .get("model")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|model| !model.is_empty())
        else {
            continue;
        };

        if !seen.insert(model.to_string()) {
            continue;
        }

        let display_name = model_config
            .get("displayName")
            .or_else(|| model_config.get("display_name"))
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string);
        let context_window = parse_codex_positive_u64(
            model_config
                .get("contextWindow")
                .or_else(|| model_config.get("context_window")),
        );
        let supports_parallel_tool_calls = model_config
            .get("supportsParallelToolCalls")
            .or_else(|| model_config.get("supports_parallel_tool_calls"))
            .and_then(|value| value.as_bool());
        let input_modalities = model_config
            .get("inputModalities")
            .or_else(|| model_config.get("input_modalities"))
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .filter(|items| !items.is_empty());
        let base_instructions = model_config
            .get("baseInstructions")
            .or_else(|| model_config.get("base_instructions"))
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_string);

        let reasoning_levels = model_config
            .get("reasoningLevels")
            .or_else(|| model_config.get("reasoning_levels"))
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str())
                    .map(str::trim)
                    .filter(|level| !level.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .filter(|levels| !levels.is_empty());
        let default_reasoning_level = model_config
            .get("defaultReasoningLevel")
            .or_else(|| model_config.get("default_reasoning_level"))
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|level| !level.is_empty())
            .map(str::to_string);

        specs.push(CodexCatalogModelSpec {
            model: model.to_string(),
            display_name,
            context_window,
            supports_parallel_tool_calls,
            input_modalities,
            base_instructions,
            reasoning_levels,
            default_reasoning_level,
        });
    }

    specs
}

fn find_codex_model_template(catalog: &Value) -> Option<Value> {
    catalog
        .get("models")
        .and_then(|models| models.as_array())
        .and_then(|models| {
            models.iter().find(|model| {
                model.get("slug").and_then(|slug| slug.as_str())
                    == Some(CODEX_MODEL_CATALOG_TEMPLATE_SLUG)
            })
        })
        .cloned()
}

fn load_codex_model_template_from_cache() -> Result<Option<Value>, AppError> {
    let path = get_codex_config_dir().join("models_cache.json");
    if !path.exists() {
        return Ok(None);
    }

    let text = fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))?;
    let catalog: Value = serde_json::from_str(&text).map_err(|e| AppError::json(&path, e))?;
    Ok(find_codex_model_template(&catalog))
}

const CODEX_CLI_FIXED_CANDIDATES: &[&str] = &[
    "codex",
    "/opt/homebrew/bin/codex",
    "/usr/local/bin/codex",
    "/home/linuxbrew/.linuxbrew/bin/codex",
];

fn push_codex_cli_candidate(
    candidates: &mut Vec<PathBuf>,
    seen: &mut HashSet<String>,
    candidate: PathBuf,
) {
    let key = candidate.to_string_lossy().into_owned();
    if seen.insert(key) {
        candidates.push(candidate);
    }
}

fn push_existing_codex_cli_candidate(
    candidates: &mut Vec<PathBuf>,
    seen: &mut HashSet<String>,
    candidate: PathBuf,
) {
    if candidate.exists() {
        push_codex_cli_candidate(candidates, seen, candidate);
    }
}

fn push_codex_cli_candidates_from_version_dirs(
    candidates: &mut Vec<PathBuf>,
    seen: &mut HashSet<String>,
    versions_dir: PathBuf,
    suffix: &[&str],
) {
    let Ok(entries) = fs::read_dir(versions_dir) else {
        return;
    };

    let mut discovered = entries
        .filter_map(Result::ok)
        .map(|entry| {
            let mut candidate = entry.path();
            for component in suffix {
                candidate.push(component);
            }
            candidate
        })
        .filter(|candidate| candidate.exists())
        .collect::<Vec<_>>();

    discovered.sort_by(|a, b| b.cmp(a));
    for candidate in discovered {
        push_codex_cli_candidate(candidates, seen, candidate);
    }
}

fn push_home_codex_cli_candidates(
    candidates: &mut Vec<PathBuf>,
    seen: &mut HashSet<String>,
    home: &Path,
) {
    for relative in [
        ".nvm/current/bin/codex",
        ".volta/bin/codex",
        ".asdf/shims/codex",
        ".local/share/mise/shims/codex",
        ".config/mise/shims/codex",
        ".local/bin/codex",
        ".npm-global/bin/codex",
        ".npm-packages/bin/codex",
        ".local/share/pnpm/codex",
        "Library/pnpm/codex",
    ] {
        push_existing_codex_cli_candidate(candidates, seen, home.join(relative));
    }

    push_codex_cli_candidates_from_version_dirs(
        candidates,
        seen,
        home.join(".nvm/versions/node"),
        &["bin", "codex"],
    );
    push_codex_cli_candidates_from_version_dirs(
        candidates,
        seen,
        home.join(".local/share/fnm/node-versions"),
        &["installation", "bin", "codex"],
    );
    push_codex_cli_candidates_from_version_dirs(
        candidates,
        seen,
        home.join("Library/Application Support/fnm/node-versions"),
        &["installation", "bin", "codex"],
    );
}

fn push_env_codex_cli_candidates(candidates: &mut Vec<PathBuf>, seen: &mut HashSet<String>) {
    for (env_key, suffix) in [
        ("NPM_CONFIG_PREFIX", &["bin", "codex"][..]),
        ("VOLTA_HOME", &["bin", "codex"][..]),
        ("ASDF_DATA_DIR", &["shims", "codex"][..]),
        ("MISE_DATA_DIR", &["shims", "codex"][..]),
        ("PNPM_HOME", &["codex"][..]),
    ] {
        let Some(prefix) = std::env::var_os(env_key) else {
            continue;
        };
        let mut candidate = PathBuf::from(prefix);
        for component in suffix {
            candidate.push(component);
        }
        push_existing_codex_cli_candidate(candidates, seen, candidate);
    }

    if let Some(nvm_dir) = std::env::var_os("NVM_DIR") {
        push_codex_cli_candidates_from_version_dirs(
            candidates,
            seen,
            PathBuf::from(nvm_dir).join("versions/node"),
            &["bin", "codex"],
        );
    }

    if let Some(fnm_dir) = std::env::var_os("FNM_DIR") {
        push_codex_cli_candidates_from_version_dirs(
            candidates,
            seen,
            PathBuf::from(fnm_dir).join("node-versions"),
            &["installation", "bin", "codex"],
        );
    }

    #[cfg(windows)]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let npm_dir = PathBuf::from(appdata).join("npm");
            for name in ["codex.cmd", "codex.exe", "codex"] {
                push_existing_codex_cli_candidate(candidates, seen, npm_dir.join(name));
            }
        }
    }
}

fn codex_cli_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    for candidate in CODEX_CLI_FIXED_CANDIDATES {
        push_codex_cli_candidate(&mut candidates, &mut seen, PathBuf::from(candidate));
    }

    push_env_codex_cli_candidates(&mut candidates, &mut seen);
    push_home_codex_cli_candidates(&mut candidates, &mut seen, &get_home_dir());

    candidates
}

fn load_codex_model_template_from_bundled() -> Result<Option<Value>, AppError> {
    for candidate in codex_cli_candidates() {
        let candidate_label = candidate.to_string_lossy();
        let output = match Command::new(&candidate)
            .args(["debug", "models", "--bundled"])
            .output()
        {
            Ok(output) => output,
            Err(err) => {
                log::debug!("failed to run `{candidate_label} debug models --bundled`: {err}");
                continue;
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::debug!("`{candidate_label} debug models --bundled` failed: {stderr}");
            continue;
        }

        let catalog: Value = match serde_json::from_slice(&output.stdout) {
            Ok(catalog) => catalog,
            Err(e) => {
                log::debug!(
                    "Failed to parse `{candidate_label} debug models --bundled` output: {e}"
                );
                continue;
            }
        };
        if let Some(template) = find_codex_model_template(&catalog) {
            return Ok(Some(template));
        }
    }

    Ok(None)
}

fn load_codex_model_template_static() -> Option<Value> {
    let text = include_str!("resources/gpt5_5_template.json");
    match serde_json::from_str(text) {
        Ok(template) => Some(template),
        Err(e) => {
            log::warn!("Failed to parse bundled gpt-5.5 template: {e}");
            None
        }
    }
}

fn load_codex_native_responses_template() -> Value {
    let text = include_str!("resources/codex_native_responses_template.json");
    serde_json::from_str(text).expect("bundled codex native responses template must be valid JSON")
}

/// Hosts whose native `/responses` gateway publishes an official Codex model
/// catalog. Match the gateway host, not the model brand: these entries grant
/// capabilities such as freeform `apply_patch` that aggregators may reject.
const CODEX_DEEPSEEK_OFFICIAL_CATALOG_HOSTS: &[&str] = &["deepseek.com"];

fn codex_url_matches_vendor_host(base_url: &str, vendor_host: &str) -> bool {
    let Ok(parsed) = url::Url::parse(base_url) else {
        return false;
    };
    let Some(host) = parsed.host_str() else {
        return false;
    };
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    host == vendor_host
        || host
            .strip_suffix(vendor_host)
            .is_some_and(|prefix| prefix.ends_with('.'))
}

/// Bundled copy of DeepSeek's official Codex models.json.
fn load_codex_deepseek_official_catalog_models() -> Vec<Value> {
    let text = include_str!("resources/codex_deepseek_catalog_template.json");
    let catalog: Value =
        serde_json::from_str(text).expect("bundled DeepSeek official catalog must be valid JSON");
    catalog
        .get("models")
        .and_then(|models| models.as_array())
        .cloned()
        .unwrap_or_default()
}

/// Return an official vendor catalog only for the vendor's own native
/// Responses gateway. ProxyChat keeps the fork's converter-oriented template.
fn codex_official_vendor_catalog_models(
    config_text: &str,
    profile: CodexCatalogToolProfile,
) -> Option<Vec<Value>> {
    if profile != CodexCatalogToolProfile::NativeResponses {
        return None;
    }
    let base_url = extract_codex_base_url(config_text)?;
    if CODEX_DEEPSEEK_OFFICIAL_CATALOG_HOSTS
        .iter()
        .any(|host| codex_url_matches_vendor_host(&base_url, host))
    {
        let models = load_codex_deepseek_official_catalog_models();
        if !models.is_empty() {
            return Some(models);
        }
    }
    None
}

/// Build one row from an official vendor entry. Explicit per-row settings win;
/// otherwise the vendor's capabilities, harness, and context window survive.
fn codex_vendor_catalog_model_entry(
    vendor_models: &[Value],
    spec: &CodexCatalogModelSpec,
    priority: usize,
) -> Value {
    let matched = vendor_models.iter().find(|entry| {
        entry
            .get("slug")
            .and_then(|slug| slug.as_str())
            .is_some_and(|slug| slug.eq_ignore_ascii_case(&spec.model))
    });
    let mut entry = match matched {
        Some(found) => found.clone(),
        None => vendor_models.first().cloned().unwrap_or_else(|| json!({})),
    };
    // Capture before the mutable borrow: the vendor entry's own default is the
    // fallback when the user declares reasoning levels without a default.
    let vendor_default = entry
        .get("default_reasoning_level")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let Some(entry_obj) = entry.as_object_mut() else {
        return json!({});
    };

    if matched.is_none() {
        let display_name = spec.display_name.as_deref().unwrap_or(&spec.model);
        entry_obj.insert("slug".to_string(), json!(spec.model));
        entry_obj.insert("display_name".to_string(), json!(display_name));
        entry_obj.insert("description".to_string(), json!(display_name));
        entry_obj.insert("priority".to_string(), json!(1000 + priority));
    }

    if let Some(display_name) = spec.display_name.as_deref() {
        entry_obj.insert("display_name".to_string(), json!(display_name));
    }
    if let Some(context_window) = spec.context_window {
        entry_obj.insert("context_window".to_string(), json!(context_window));
        entry_obj.insert("max_context_window".to_string(), json!(context_window));
    }
    if let Some(parallel) = spec.supports_parallel_tool_calls {
        entry_obj.insert("supports_parallel_tool_calls".to_string(), json!(parallel));
    }
    if let Some(modalities) = spec.input_modalities.as_deref() {
        entry_obj.insert("input_modalities".to_string(), json!(modalities));
    }
    if let Some(base_instructions) = spec
        .base_instructions
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        entry_obj.insert("base_instructions".to_string(), json!(base_instructions));
    }

    // Per-model reasoning levels win over the official vendor entry too.
    // The vendor file is the base (its own levels stay when no override is
    // declared); its default_reasoning_level is the fallback.
    apply_codex_reasoning_level_override(entry_obj, vendor_default.as_deref(), spec);

    fill_template_fields_from_static(&mut entry);
    entry
}

/// Fields Codex's external-catalog parser REQUIRES (no serde default): when
/// one is missing Codex rejects the whole catalog file at startup ("missing
/// field ..."). `base_instructions` is the other known required field; the
/// templates always carry it and `codex_catalog_model_entry` handles it.
/// When Codex requires a new field, add it here AND to the static templates.
const CODEX_CATALOG_PARSER_REQUIRED_FIELDS: &[&str] = &["supports_reasoning_summaries"];

/// `models_cache.json` is shared by every Codex install on the machine (npm
/// CLI, desktop-bundled binary, ...), and each version serializes its own
/// `ModelInfo` shape — the cache's field set follows whichever process wrote
/// it last, so it cannot be assumed to satisfy the current external-catalog
/// schema (observed live: 0.144.5 requires `supports_reasoning_summaries`
/// while a coexisting build kept rewriting the cache without it). Backfill
/// ONLY parser-required fields from the bundled static template: optional
/// capability fields keep their missing-means-default semantics, and existing
/// values always win.
fn fill_template_fields_from_static(template: &mut Value) {
    let Some(static_template) = load_codex_model_template_static() else {
        return;
    };
    let (Some(template_obj), Some(static_obj)) =
        (template.as_object_mut(), static_template.as_object())
    else {
        return;
    };
    for key in CODEX_CATALOG_PARSER_REQUIRED_FIELDS {
        if !template_obj.contains_key(*key) {
            if let Some(value) = static_obj.get(*key) {
                template_obj.insert((*key).to_string(), value.clone());
            }
        }
    }
}

fn load_codex_model_catalog_template() -> Result<Value, AppError> {
    if let Some(mut template) = load_codex_model_template_from_cache()? {
        fill_template_fields_from_static(&mut template);
        return Ok(template);
    }
    if let Some(mut template) = load_codex_model_template_from_bundled()? {
        fill_template_fields_from_static(&mut template);
        return Ok(template);
    }
    if let Some(template) = load_codex_model_template_static() {
        return Ok(template);
    }

    Err(AppError::Message(format!(
        "Codex model catalog template `{CODEX_MODEL_CATALOG_TEMPLATE_SLUG}` not found. Please start Codex once so models_cache.json is available, or ensure the `codex` CLI is on PATH."
    )))
}

fn codex_model_catalog_from_specs(
    specs: &[CodexCatalogModelSpec],
    template: &Value,
    profile: CodexCatalogToolProfile,
    default_context_window: u64,
) -> Value {
    let entries: Vec<Value> = specs
        .iter()
        .enumerate()
        .map(|(index, spec)| {
            codex_catalog_model_entry(template, spec, index, profile, default_context_window)
        })
        .collect();

    json!({ "models": entries })
}

fn codex_model_catalog_from_settings(
    settings: &Value,
    config_text: &str,
    profile: CodexCatalogToolProfile,
) -> Result<Option<Value>, AppError> {
    let specs = codex_catalog_model_specs(settings);
    if specs.is_empty() {
        return Ok(None);
    }

    if let Some(vendor_models) = codex_official_vendor_catalog_models(config_text, profile) {
        let entries = specs
            .iter()
            .enumerate()
            .map(|(index, spec)| codex_vendor_catalog_model_entry(&vendor_models, spec, index))
            .collect::<Vec<_>>();
        return Ok(Some(json!({ "models": entries })));
    }

    let default_context_window =
        extract_codex_top_level_u64(config_text, "model_context_window").unwrap_or(128_000);

    let template = match profile {
        CodexCatalogToolProfile::NativeResponses => load_codex_native_responses_template(),
        CodexCatalogToolProfile::ProxyChat => load_codex_model_catalog_template()?,
    };
    Ok(Some(codex_model_catalog_from_specs(
        &specs,
        &template,
        profile,
        default_context_window,
    )))
}

fn is_cc_switch_owned_catalog_reference(path: &str) -> bool {
    Path::new(path).file_name().and_then(|name| name.to_str())
        == Some(CC_SWITCH_CODEX_MODEL_CATALOG_FILENAME)
}

fn set_codex_model_catalog_json_field(
    config_text: &str,
    catalog_path: Option<&Path>,
) -> Result<String, AppError> {
    let mut doc = config_text
        .parse::<DocumentMut>()
        .map_err(|e| AppError::Message(format!("Invalid Codex config.toml: {e}")))?;

    match catalog_path {
        Some(_) => {
            let should_set = match doc.get("model_catalog_json") {
                None => true,
                Some(item) => item
                    .as_str()
                    .is_some_and(is_cc_switch_owned_catalog_reference),
            };
            if should_set {
                doc["model_catalog_json"] =
                    toml_edit::value(CC_SWITCH_CODEX_MODEL_CATALOG_FILENAME);
            }
        }
        None => {
            let should_remove = doc
                .get("model_catalog_json")
                .and_then(|item| item.as_str())
                .is_some_and(is_cc_switch_owned_catalog_reference);
            if should_remove {
                doc.as_table_mut().remove("model_catalog_json");
            }
        }
    }

    Ok(doc.to_string())
}

fn set_codex_native_web_search_field(config_text: &str, disable: bool) -> Result<String, AppError> {
    let mut doc = config_text
        .parse::<DocumentMut>()
        .map_err(|e| AppError::Message(format!("Invalid Codex config.toml: {e}")))?;

    if disable {
        doc[CODEX_WEB_SEARCH_FIELD] = toml_edit::value(CODEX_WEB_SEARCH_DISABLED);
    } else {
        let owned = doc
            .get(CODEX_WEB_SEARCH_FIELD)
            .and_then(|item| item.as_str())
            == Some(CODEX_WEB_SEARCH_DISABLED);
        if owned {
            doc.as_table_mut().remove(CODEX_WEB_SEARCH_FIELD);
        }
    }

    Ok(doc.to_string())
}

pub fn prepare_codex_config_text_with_model_catalog(
    settings: &Value,
    config_text: &str,
    profile: CodexCatalogToolProfile,
) -> Result<String, AppError> {
    let catalog_path = get_codex_model_catalog_path();

    if let Some(catalog) = codex_model_catalog_from_settings(settings, config_text, profile)? {
        let config_text = set_codex_model_catalog_json_field(config_text, Some(&catalog_path))?;
        let disable_web_search = profile == CodexCatalogToolProfile::NativeResponses
            && codex_native_gateway_rejects_web_search(&config_text);
        let config_text = set_codex_native_web_search_field(&config_text, disable_web_search)?;
        write_json_file_managed(&catalog_path, &catalog)?;
        Ok(config_text)
    } else {
        let config_text = set_codex_model_catalog_json_field(config_text, None)?;
        set_codex_native_web_search_field(&config_text, false)
    }
}

/// Maximum size of a generated Codex model catalog (32 MiB). Catalogs are
/// normally only a few hundred KiB; larger files are treated as untrusted.
const MAX_CODEX_CATALOG_BYTES: u64 = 32 * 1024 * 1024;

#[allow(dead_code)]
pub fn read_codex_model_catalog_simplified_from_live() -> Result<Option<Value>, AppError> {
    let config_text = read_codex_config_text()?;
    let config_dir = get_codex_config_dir();
    let Some(catalog_path) = resolve_cc_switch_catalog_path(&config_text, &config_dir) else {
        return Ok(None);
    };
    if !catalog_path.exists() {
        return Ok(None);
    }
    let catalog_text = match read_codex_model_catalog_text(&catalog_path) {
        Ok(text) => text,
        Err(error) => {
            log::warn!(
                "Refusing to read an out-of-bounds or oversized Codex model catalog {}: {error}",
                catalog_path.display()
            );
            return Ok(None);
        }
    };
    Ok(build_simplified_catalog_from_texts(
        &config_text,
        &catalog_text,
    ))
}

/// Read a UTF-8 text file while enforcing the byte limit on the actual stream.
pub(crate) fn read_limited_string(path: &Path, max_bytes: u64) -> Result<String, AppError> {
    let file = fs::File::open(path).map_err(|error| AppError::io(path, error))?;
    let metadata = file.metadata().map_err(|error| AppError::io(path, error))?;
    if metadata.len() > max_bytes {
        return Err(AppError::Config(format!(
            "File {} exceeds the {max_bytes}-byte limit",
            path.display()
        )));
    }

    let mut text = String::new();
    let bytes_read = file
        .take(max_bytes.saturating_add(1))
        .read_to_string(&mut text)
        .map_err(|error| AppError::io(path, error))?;
    if bytes_read as u64 > max_bytes {
        return Err(AppError::Config(format!(
            "File {} exceeds the {max_bytes}-byte limit",
            path.display()
        )));
    }
    Ok(text)
}

/// Read the cc-switch Codex model catalog file with a size cap.
pub(crate) fn read_codex_model_catalog_text(path: &Path) -> Result<String, AppError> {
    read_limited_string(path, MAX_CODEX_CATALOG_BYTES)
}

/// Resolve the cc-switch-owned catalog under `base_dir`.
///
/// Relative paths are resolved under `base_dir`; absolute paths must still be
/// contained within it. Existing files are canonicalized and checked again so
/// a symlink inside the config directory cannot escape the boundary.
pub(crate) fn resolve_cc_switch_catalog_path(
    config_text: &str,
    base_dir: &Path,
) -> Option<PathBuf> {
    if config_text.trim().is_empty() {
        return None;
    }
    let doc = config_text.parse::<DocumentMut>().ok()?;
    let catalog_path_str = doc
        .get("model_catalog_json")
        .and_then(|item| item.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())?;

    let referenced_path = Path::new(catalog_path_str);
    if !is_cc_switch_owned_catalog_reference(catalog_path_str) {
        return None;
    }

    // Treat Unix-style absolute paths as absolute on Windows too. Accepting them
    // as relative would make `/tmp/...` appear to be under the config directory.
    let is_unix_absolute = catalog_path_str.starts_with('/');
    let resolved = if referenced_path.is_absolute() || is_unix_absolute {
        referenced_path.to_path_buf()
    } else {
        base_dir.join(referenced_path)
    };

    if !path_is_within(base_dir, &resolved) {
        log::warn!(
            "Codex model_catalog_json points outside the config directory: {} (base: {})",
            resolved.display(),
            base_dir.display()
        );
        return None;
    }

    if resolved.exists() {
        let canonical = match fs::canonicalize(&resolved) {
            Ok(path) => path,
            Err(error) => {
                log::warn!(
                    "Failed to canonicalize Codex model_catalog_json {}: {error}",
                    resolved.display()
                );
                return None;
            }
        };
        let canonical_base = fs::canonicalize(base_dir).unwrap_or_else(|_| base_dir.to_path_buf());
        if !path_is_within(&canonical_base, &canonical) {
            log::warn!(
                "Codex model_catalog_json escapes the config directory through a symlink: {} -> {} (base: {})",
                resolved.display(),
                canonical.display(),
                canonical_base.display()
            );
            return None;
        }
        return Some(canonical);
    }

    Some(resolved)
}

#[allow(dead_code)]
fn build_simplified_catalog_from_texts(config_text: &str, catalog_text: &str) -> Option<Value> {
    let catalog: Value = serde_json::from_str(catalog_text).ok()?;
    let models = catalog.get("models").and_then(|m| m.as_array())?;
    let default_context_window =
        extract_codex_top_level_u64(config_text, "model_context_window").unwrap_or(128_000);

    let mut entries = Vec::with_capacity(models.len());
    for entry in models {
        let Some(model) = entry
            .get("slug")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            continue;
        };

        let mut obj = serde_json::Map::new();
        obj.insert("model".to_string(), json!(model));

        if let Some(display_name) = entry
            .get("display_name")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty() && *s != model)
        {
            obj.insert("displayName".to_string(), json!(display_name));
        }

        if let Some(context_window) = entry
            .get("context_window")
            .and_then(|v| v.as_u64())
            .filter(|v| *v > 0 && *v != default_context_window)
        {
            obj.insert("contextWindow".to_string(), json!(context_window));
        }

        if let Some(parallel) = entry
            .get("supports_parallel_tool_calls")
            .and_then(|v| v.as_bool())
        {
            obj.insert("supportsParallelToolCalls".to_string(), json!(parallel));
        }
        if let Some(modalities) = entry.get("input_modalities").and_then(|v| v.as_array()) {
            let modalities: Vec<String> = modalities
                .iter()
                .filter_map(|m| m.as_str())
                .map(str::to_string)
                .collect();
            let inferred = codex_catalog_input_modalities(model, None);
            if !modalities.is_empty() && modalities != inferred {
                obj.insert("inputModalities".to_string(), json!(modalities));
            }
        }

        entries.push(Value::Object(obj));
    }

    if entries.is_empty() {
        return None;
    }

    Some(json!({ "models": entries }))
}

pub fn prepare_codex_live_config_text_with_optional_catalog(
    settings: &Value,
    config_text: &str,
    profile: CodexCatalogToolProfile,
) -> Result<String, AppError> {
    if settings.get("modelCatalog").is_some() {
        prepare_codex_config_text_with_model_catalog(settings, config_text, profile)
    } else {
        Ok(config_text.to_string())
    }
}

pub fn write_codex_provider_live_with_catalog(
    settings: &Value,
    category: Option<&str>,
    auth: &Value,
    config_text: Option<&str>,
    profile: CodexCatalogToolProfile,
) -> Result<(), AppError> {
    let prepared_config = config_text
        .map(|text| prepare_codex_config_text_with_model_catalog(settings, text, profile))
        .transpose()?;

    write_codex_live_for_provider(category, auth, prepared_config.as_deref())
}

/// Extract a provider-scoped `experimental_bearer_token` from Codex `config.toml`.
///
/// Third-party providers may store the API key inside
/// `[model_providers.<id>].experimental_bearer_token` while keeping the user's
/// ChatGPT login cache intact in `auth.json`. Falls back to the top-level
/// `experimental_bearer_token` when no active custom model provider is set.
pub fn extract_codex_experimental_bearer_token(config_text: &str) -> Option<String> {
    if !config_text.contains("experimental_bearer_token") {
        return None;
    }

    let doc = config_text.parse::<DocumentMut>().ok()?;
    let provider_id = active_codex_model_provider_id(&doc);

    let top_level_token = || {
        doc.get("experimental_bearer_token")
            .and_then(|item| item.as_str())
    };
    let token = match provider_id.as_deref() {
        Some(id) if is_custom_codex_model_provider_id(id) => doc
            .get("model_providers")
            .and_then(|item| item.as_table())
            .and_then(|table| table.get(id))
            .and_then(|item| item.as_table())
            .and_then(|table| table.get("experimental_bearer_token"))
            .and_then(|item| item.as_str())
            .or_else(top_level_token),
        Some(_) => top_level_token(),
        None => top_level_token(),
    };

    token
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(str::to_string)
}

fn set_codex_experimental_bearer_token(config_text: &str, token: &str) -> Result<String, AppError> {
    if config_text.trim().is_empty() {
        return Err(AppError::localized(
            "provider.codex.config.missing",
            "Codex 第三方供应商缺少 config.toml 配置，无法写入 bearer token",
            "Codex third-party provider is missing config.toml, cannot write bearer token",
        ));
    }

    let mut doc = config_text
        .parse::<DocumentMut>()
        .map_err(|e| AppError::Message(format!("Invalid Codex config.toml: {e}")))?;

    let Some(provider_id) = active_codex_model_provider_id(&doc) else {
        doc["experimental_bearer_token"] = toml_edit::value(token);
        return Ok(doc.to_string());
    };

    if !is_custom_codex_model_provider_id(&provider_id) {
        doc["experimental_bearer_token"] = toml_edit::value(token);
        return Ok(doc.to_string());
    }

    if let Some(provider_table) = doc
        .get_mut("model_providers")
        .and_then(|item| item.as_table_mut())
        .and_then(|table| table.get_mut(provider_id.as_str()))
        .and_then(|item| item.as_table_mut())
    {
        provider_table["experimental_bearer_token"] = toml_edit::value(token);
        return Ok(doc.to_string());
    }

    doc["experimental_bearer_token"] = toml_edit::value(token);
    Ok(doc.to_string())
}

pub fn remove_codex_experimental_bearer_token_if(
    config_text: &str,
    predicate: impl Fn(&str) -> bool,
) -> Result<String, AppError> {
    if config_text.trim().is_empty() || !config_text.contains("experimental_bearer_token") {
        return Ok(config_text.to_string());
    }

    let mut doc = config_text
        .parse::<DocumentMut>()
        .map_err(|e| AppError::Message(format!("Invalid Codex config.toml: {e}")))?;

    if let Some(provider_id) = active_codex_model_provider_id(&doc) {
        if let Some(provider_table) = doc
            .get_mut("model_providers")
            .and_then(|item| item.as_table_mut())
            .and_then(|table| table.get_mut(provider_id.as_str()))
            .and_then(|item| item.as_table_mut())
        {
            let should_remove = provider_table
                .get("experimental_bearer_token")
                .and_then(|item| item.as_str())
                .map(str::trim)
                .is_some_and(&predicate);
            if should_remove {
                provider_table.remove("experimental_bearer_token");
            }
        }
    }

    let should_remove_top_level = doc
        .get("experimental_bearer_token")
        .and_then(|item| item.as_str())
        .map(str::trim)
        .is_some_and(&predicate);
    if should_remove_top_level {
        doc.as_table_mut().remove("experimental_bearer_token");
    }
    Ok(doc.to_string())
}

pub fn remove_codex_experimental_bearer_token(config_text: &str) -> Result<String, AppError> {
    remove_codex_experimental_bearer_token_if(config_text, |_| true)
}

/// Read the current Codex live settings as a `{ auth, config }` object.
///
/// Missing `auth.json` collapses to `{}` so a config-only third-party install
/// is still importable. An existing empty config.toml is also valid after an
/// official switch clears stale auth; only two missing files mean no install.
pub fn read_codex_live_settings() -> Result<Value, AppError> {
    let auth_path = get_codex_auth_path();
    let auth_present = auth_path.exists();
    let auth: Value = if auth_present {
        read_json_file(&auth_path)?
    } else {
        json!({})
    };
    let cfg_text = read_and_validate_codex_config_text()?;

    if !auth_present && !get_codex_config_path().exists() {
        return Err(AppError::localized(
            "codex.live.missing",
            "Codex 配置文件不存在",
            "Codex configuration is missing",
        ));
    }

    Ok(json!({ "auth": auth, "config": cfg_text }))
}

/// Route a Codex live write between full auth+config or config-only.
///
/// Official providers with usable login material own `auth.json`. Third-party
/// providers only touch `config.toml` when the compatibility setting is enabled
/// so the user's ChatGPT login cache survives provider switches.
pub fn write_codex_live_for_provider(
    category: Option<&str>,
    auth: &Value,
    config_text: Option<&str>,
) -> Result<(), AppError> {
    let should_write_auth = (category == Some("official") && codex_auth_has_login_material(auth))
        || (category != Some("official")
            && !crate::settings::preserve_codex_official_auth_on_switch());

    if should_write_auth {
        write_codex_live_atomic(auth, config_text)
    } else {
        let live_config = prepare_codex_provider_live_config(auth, config_text.unwrap_or(""))?;
        write_codex_live_config_atomic(Some(&live_config))
    }
}

/// Project a Codex official account card through the local proxy while keeping
/// authentication owned by Codex itself.
///
/// The stored provider keeps its API key in `auth.OPENAI_API_KEY`. Live Codex
/// requests can use a provider-scoped `experimental_bearer_token`, so switching
/// providers only needs to update `config.toml`; `auth.json` stays as the user's
/// long-lived ChatGPT login cache.
pub fn prepare_codex_provider_live_config(
    auth: &Value,
    config_text: &str,
) -> Result<String, AppError> {
    let token = extract_codex_auth_api_key(auth)
        .or_else(|| extract_codex_experimental_bearer_token(config_text));

    Ok(match token {
        Some(token) => set_codex_experimental_bearer_token(config_text, &token)?,
        None => config_text.to_string(),
    })
}

/// During DB backfill, lift a live `experimental_bearer_token` back into
/// `auth.OPENAI_API_KEY` so the stored provider keeps its canonical shape and
/// generated live tokens don't leak into stored provider TOML.
pub fn restore_codex_provider_token_for_backfill(
    settings: &mut Value,
    template_settings: &Value,
) -> Result<(), AppError> {
    let Some(config_text) = settings
        .get("config")
        .and_then(|value| value.as_str())
        .map(str::to_string)
    else {
        return Ok(());
    };

    let Some(token) = extract_codex_experimental_bearer_token(&config_text) else {
        return Ok(());
    };

    let cleaned_config = remove_codex_experimental_bearer_token(&config_text)?;
    if let Some(obj) = settings.as_object_mut() {
        obj.insert("config".to_string(), Value::String(cleaned_config));

        let mut auth = template_settings
            .get("auth")
            .filter(|value| value.is_object())
            .cloned()
            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
        if let Some(auth_obj) = auth.as_object_mut() {
            auth_obj.insert("OPENAI_API_KEY".to_string(), Value::String(token));
        }
        obj.insert("auth".to_string(), auth);
    }

    Ok(())
}

pub fn restore_codex_settings_for_backfill(
    settings: &mut Value,
    template_settings: &Value,
    restore_provider_token: bool,
) -> Result<(), AppError> {
    if restore_provider_token {
        restore_codex_provider_token_for_backfill(settings, template_settings)?;
    }
    Ok(())
}

/// Strip MCP projections from live Codex settings before backfilling a
/// provider snapshot. MCP servers are owned by the database; the live TOML
/// tables are derived state and must not be persisted with a provider.
pub fn strip_codex_mcp_servers_from_settings(settings: &mut Value) -> Result<(), AppError> {
    let Some(config_text) = settings
        .get("config")
        .and_then(Value::as_str)
        .map(str::to_string)
    else {
        return Ok(());
    };
    if !config_text.contains("mcp") {
        return Ok(());
    }

    let mut doc = config_text
        .parse::<DocumentMut>()
        .map_err(|e| AppError::Message(format!("Invalid Codex config.toml: {e}")))?;
    let mut changed = doc.as_table_mut().remove("mcp_servers").is_some();

    if let Some(mcp_table) = doc.get_mut("mcp").and_then(|item| item.as_table_like_mut()) {
        changed |= mcp_table.remove("servers").is_some();
        if mcp_table.is_empty() {
            doc.as_table_mut().remove("mcp");
        }
    }

    if changed {
        if let Some(object) = settings.as_object_mut() {
            object.insert("config".to_string(), Value::String(doc.to_string()));
        }
    }

    Ok(())
}

/// Update a field in Codex config.toml using toml_edit (syntax-preserving).
///
/// Supported fields:
/// - `"base_url"`: writes to `[model_providers.<current>].base_url` if `model_provider` exists,
///   otherwise falls back to top-level `base_url`.
/// - `"model"`: writes to top-level `model` field.
///
/// Empty value removes the field.
pub fn update_codex_toml_field(toml_str: &str, field: &str, value: &str) -> Result<String, String> {
    let mut doc = toml_str
        .parse::<DocumentMut>()
        .map_err(|e| format!("TOML parse error: {e}"))?;

    let trimmed = value.trim();

    match field {
        "base_url" => {
            let model_provider = doc
                .get("model_provider")
                .and_then(|item| item.as_str())
                .map(str::to_string);

            if let Some(provider_key) = model_provider {
                // Ensure [model_providers] table exists
                //
                // 用 as_table_like_mut 而非 as_table_mut：用户把配置写成 inline table
                // （`model_providers = { foo = {...} }`，TOML 合法）时 as_table_mut
                // 返回 None，会一路掉进下面的顶层 fallback——用户改的 base_url 被写到
                // 了错误层级且毫无提示。
                if doc
                    .get("model_providers")
                    .is_none_or(|item| item.as_table_like().is_none())
                {
                    // 键存在但不是表（`model_providers = 42`）时，下面这行会把用户
                    // 手写的值替换掉。旧代码在这种形状下会掉进顶层 fallback 而不动
                    // 它，所以归一化必须留痕——与 mcp/codex.rs、mcp/grokbuild.rs、
                    // opencode_config.rs 的同款处理保持一致。
                    if doc
                        .get("model_providers")
                        .is_some_and(|item| !item.is_none())
                    {
                        log::warn!("config.toml 的 model_providers 不是表，已重置为空表");
                    }
                    doc["model_providers"] = toml_edit::table();
                }

                if let Some(model_providers) = doc
                    .get_mut("model_providers")
                    .and_then(toml_edit::Item::as_table_like_mut)
                {
                    // Ensure [model_providers.<provider_key>] table exists
                    if !model_providers.contains_key(&provider_key) {
                        model_providers.insert(&provider_key, toml_edit::table());
                    }

                    if let Some(provider_table) = model_providers
                        .get_mut(&provider_key)
                        .and_then(toml_edit::Item::as_table_like_mut)
                    {
                        if trimmed.is_empty() {
                            provider_table.remove("base_url");
                        } else {
                            provider_table.insert(field, toml_edit::value(trimmed));
                        }
                        return Ok(doc.to_string());
                    }
                }

                log::warn!(
                    "config.toml 的 [model_providers.{provider_key}] 结构异常，{field} 改写为顶层字段"
                );
            }

            // Fallback: no model_provider or structure mismatch → top-level base_url
            if trimmed.is_empty() {
                doc.as_table_mut().remove("base_url");
            } else {
                doc["base_url"] = toml_edit::value(trimmed);
            }
        }
        "model" => {
            if trimmed.is_empty() {
                doc.as_table_mut().remove("model");
            } else {
                doc["model"] = toml_edit::value(trimmed);
            }
        }
        _ => return Err(format!("unsupported field: {field}")),
    }

    Ok(doc.to_string())
}

/// Remove `base_url` from the active model_provider section only if it matches `predicate`.
/// Also removes top-level `base_url` if it matches.
/// Used by proxy cleanup to strip local proxy URLs without touching user-configured URLs.
pub fn remove_codex_toml_base_url_if(toml_str: &str, predicate: impl Fn(&str) -> bool) -> String {
    let mut doc = match toml_str.parse::<DocumentMut>() {
        Ok(doc) => doc,
        Err(_) => return toml_str.to_string(),
    };

    let model_provider = doc
        .get("model_provider")
        .and_then(|item| item.as_str())
        .map(str::to_string);

    if let Some(provider_key) = model_provider {
        if let Some(model_providers) = doc
            .get_mut("model_providers")
            .and_then(|v| v.as_table_mut())
        {
            if let Some(provider_table) = model_providers
                .get_mut(provider_key.as_str())
                .and_then(|v| v.as_table_mut())
            {
                let should_remove = provider_table
                    .get("base_url")
                    .and_then(|item| item.as_str())
                    .map(&predicate)
                    .unwrap_or(false);
                if should_remove {
                    provider_table.remove("base_url");
                }
            }
        }
    }

    // Fallback: also clean up top-level base_url if it matches
    let should_remove_root = doc
        .get("base_url")
        .and_then(|item| item.as_str())
        .map(&predicate)
        .unwrap_or(false);
    if should_remove_root {
        doc.as_table_mut().remove("base_url");
    }

    doc.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use serial_test::serial;
    use std::ffi::OsString;

    struct CodexLiveTestHome {
        _dir: tempfile::TempDir,
        original_test_home: Option<OsString>,
    }

    impl CodexLiveTestHome {
        fn new() -> Self {
            let dir = tempfile::tempdir().expect("create isolated Codex live test home");
            let original_test_home = std::env::var_os("CC_SWITCH_TEST_HOME");
            std::env::set_var("CC_SWITCH_TEST_HOME", dir.path());
            crate::settings::reload_settings().expect("reload settings for isolated test home");

            Self {
                _dir: dir,
                original_test_home,
            }
        }
    }

    impl Drop for CodexLiveTestHome {
        fn drop(&mut self) {
            match &self.original_test_home {
                Some(value) => std::env::set_var("CC_SWITCH_TEST_HOME", value),
                None => std::env::remove_var("CC_SWITCH_TEST_HOME"),
            }
            let _ = crate::settings::reload_settings();
        }
    }

    #[derive(Debug, PartialEq)]
    struct CodexLiveTestState {
        auth_bytes: Vec<u8>,
        auth_value: Value,
        config_bytes: Vec<u8>,
        config_value: toml::Value,
        catalog_bytes: Vec<u8>,
        catalog_value: Value,
        marker_bytes: Vec<u8>,
        marker_value: Value,
    }

    fn capture_codex_live_test_state() -> CodexLiveTestState {
        let auth_bytes = fs::read(get_codex_auth_path()).expect("read live auth bytes");
        let config_bytes = fs::read(get_codex_config_path()).expect("read live config bytes");
        let catalog_bytes =
            fs::read(get_codex_model_catalog_path()).expect("read live catalog bytes");
        let marker_bytes = fs::read(get_codex_managed_oauth_live_auth_marker_path())
            .expect("read managed auth marker bytes");

        CodexLiveTestState {
            auth_value: serde_json::from_slice(&auth_bytes).expect("parse live auth"),
            config_value: toml::from_str(
                std::str::from_utf8(&config_bytes).expect("live config must be UTF-8"),
            )
            .expect("parse live config"),
            catalog_value: serde_json::from_slice(&catalog_bytes).expect("parse live catalog"),
            marker_value: serde_json::from_slice(&marker_bytes).expect("parse managed auth marker"),
            auth_bytes,
            config_bytes,
            catalog_bytes,
            marker_bytes,
        }
    }

    fn seed_rotated_managed_codex_live_state() -> CodexLiveTestState {
        let auth = codex_managed_oauth_auth_value(
            "account-a",
            "access-r1",
            Some("id-r1"),
            "refresh-r1",
            "2026-08-06T00:00:01Z",
        );
        crate::config::write_json_file(&get_codex_auth_path(), &auth).expect("seed live auth R1");
        crate::config::write_text_file(
            &get_codex_config_path(),
            "# cas-guard-sentinel\nmodel = \"gpt-5.5\"\nmodel_catalog_json = \"cc-switch-model-catalog.json\"\n",
        )
        .expect("seed live config");
        crate::config::write_json_file(
            &get_codex_model_catalog_path(),
            &json!({ "models": [{ "slug": "cas-guard-sentinel" }] }),
        )
        .expect("seed live catalog");
        record_codex_managed_oauth_live_auth(&auth).expect("seed managed auth marker");

        capture_codex_live_test_state()
    }

    #[test]
    #[serial]
    fn ensure_live_auth_guard_rejects_rotated_refresh_without_mutating_live_bundle() {
        let _home = CodexLiveTestHome::new();
        let before = seed_rotated_managed_codex_live_state();

        let result =
            ensure_codex_live_auth_unchanged_for_managed_account("account-a", "refresh-r0");

        assert!(result.is_err(), "R1 live auth must reject an expected R0");
        assert_eq!(capture_codex_live_test_state(), before);
    }

    #[test]
    #[serial]
    fn clear_live_auth_guard_rejects_rotated_refresh_without_mutating_live_bundle() {
        let _home = CodexLiveTestHome::new();
        let before = seed_rotated_managed_codex_live_state();

        let result =
            clear_codex_live_auth_for_managed_account_if_unchanged("account-a", Some("refresh-r0"));

        assert!(result.is_err(), "R1 live auth must reject an expected R0");
        assert_eq!(capture_codex_live_test_state(), before);
    }

    #[test]
    fn managed_chatgpt_login_matched_by_account_id_including_full_refresh_bundle() {
        // 托管写入的是含 refresh_token 的完整 bundle；备份剥离必须凭 account_id
        // 认出它，避免把可刷新 token 持久化进 Live 备份。
        let full_bundle = json!({
            "auth_mode": "chatgpt",
            "OPENAI_API_KEY": null,
            "tokens": {
                "id_token": "id",
                "access_token": "access",
                "refresh_token": "refresh-secret",
                "account_id": "acct-managed"
            },
            "last_refresh": "2026-01-02T03:04:05.000000000Z"
        });
        assert!(
            codex_live_auth_is_managed_chatgpt_login(&full_bundle, "acct-managed"),
            "a full refreshable bundle for the managed account must be recognized"
        );
        assert!(
            !codex_live_auth_is_managed_chatgpt_login(&full_bundle, "acct-other"),
            "a login for a different account must not match"
        );

        // 非 chatgpt 模式（API key）不应命中。
        let api_key_auth = json!({ "OPENAI_API_KEY": "sk-live" });
        assert!(!codex_live_auth_is_managed_chatgpt_login(
            &api_key_auth,
            "acct-managed"
        ));
    }

    #[test]
    fn dynamic_template_backfills_parser_required_fields_from_static() {
        // Simulate a template cloned from a models_cache.json written by a
        // Codex build whose ModelInfo lacks parser-side required fields such
        // as `supports_reasoning_summaries` (codex >= 0.144.5 rejects the
        // whole catalog file without it).
        let mut template = serde_json::json!({
            "slug": "gpt-5.5",
            "context_window": 272_000,
            "supports_parallel_tool_calls": false
        });
        fill_template_fields_from_static(&mut template);

        assert_eq!(
            template
                .get("supports_reasoning_summaries")
                .and_then(Value::as_bool),
            Some(true)
        );
        // Keys already present in the dynamic template are never overwritten.
        assert_eq!(
            template
                .get("supports_parallel_tool_calls")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            template.get("context_window").and_then(Value::as_u64),
            Some(272_000)
        );
        // Optional capability fields must NOT be backfilled: for the catalog
        // parser "missing" means the parser default, not the static
        // template's value.
        assert!(template.get("supports_search_tool").is_none());
        assert!(template.get("supports_image_detail_original").is_none());
        assert!(template.get("web_search_tool_type").is_none());
    }

    #[test]
    fn proxy_chat_catalog_entries_carry_reasoning_summaries_flag() {
        // End to end: a stale dynamic template, once backfilled, must yield
        // catalog entries codex 0.144.5+ can parse.
        let mut template = serde_json::json!({ "slug": "gpt-5.5" });
        fill_template_fields_from_static(&mut template);
        let specs = vec![CodexCatalogModelSpec {
            model: "k3".to_string(),
            display_name: Some("Kimi K3".to_string()),
            context_window: Some(262_144),
            supports_parallel_tool_calls: None,
            input_modalities: None,
            base_instructions: None,
            reasoning_levels: None,
            default_reasoning_level: None,
        }];
        let catalog = codex_model_catalog_from_specs(
            &specs,
            &template,
            CodexCatalogToolProfile::ProxyChat,
            128_000,
        );
        assert_eq!(
            catalog["models"][0]
                .get("supports_reasoning_summaries")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn native_responses_catalog_strips_freeform_tools_and_applies_overrides() {
        let settings = serde_json::json!({
            "modelCatalog": {
                "models": [
                    {
                        "model": "MiniMaxAI/MiniMax-M3",
                        "displayName": "MiniMax M3",
                        "contextWindow": 1_000_000,
                        "supportsParallelToolCalls": true,
                        "inputModalities": ["text", "image"],
                        "baseInstructions": "You are MiniMax M3."
                    }
                ]
            }
        });
        let catalog = codex_model_catalog_from_settings(
            &settings,
            r#"model = "MiniMaxAI/MiniMax-M3""#,
            CodexCatalogToolProfile::NativeResponses,
        )
        .expect("catalog should build")
        .expect("catalog should be present");
        let entry = catalog
            .get("models")
            .and_then(|models| models.as_array())
            .and_then(|models| models.first())
            .expect("model entry");

        assert_eq!(
            entry.get("slug").and_then(Value::as_str),
            Some("MiniMaxAI/MiniMax-M3")
        );
        assert!(entry.get("apply_patch_tool_type").is_none());
        assert!(entry.get("web_search_tool_type").is_none());
        assert_eq!(
            entry.get("shell_type").and_then(Value::as_str),
            Some("shell_command")
        );
        assert_eq!(
            entry
                .get("supports_parallel_tool_calls")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            entry.get("base_instructions").and_then(Value::as_str),
            Some("You are MiniMax M3.")
        );
    }

    #[test]
    fn synthetic_catalog_uses_config_defaults_when_row_omits_metadata() {
        let settings = json!({
            "modelCatalog": { "models": [{ "model": "custom-model" }] }
        });
        let catalog = codex_model_catalog_from_settings(
            &settings,
            "model_context_window = 256000\n",
            CodexCatalogToolProfile::NativeResponses,
        )
        .expect("catalog should build")
        .expect("catalog should be present");
        let entry = &catalog["models"][0];

        assert_eq!(
            entry.get("display_name").and_then(Value::as_str),
            Some("custom-model")
        );
        assert_eq!(
            entry.get("context_window").and_then(Value::as_u64),
            Some(256_000)
        );
    }

    const DEEPSEEK_NATIVE_CONFIG: &str = r#"model = "deepseek-v4-flash"
model_provider = "custom"

[model_providers.custom]
name = "deepseek"
base_url = "https://api.deepseek.com"
wire_api = "responses"
"#;

    #[test]
    fn deepseek_host_native_catalog_mirrors_official_entries() {
        let settings = json!({
            "modelCatalog": {
                "models": [
                    { "model": "deepseek-v4-flash", "displayName": "DeepSeek V4 Flash" },
                    { "model": "deepseek-v4-pro", "contextWindow": 500_000 }
                ]
            }
        });

        let catalog = codex_model_catalog_from_settings(
            &settings,
            DEEPSEEK_NATIVE_CONFIG,
            CodexCatalogToolProfile::NativeResponses,
        )
        .expect("vendor catalog should build")
        .expect("vendor catalog should be present");

        let flash = &catalog["models"][0];
        assert_eq!(
            flash.get("slug").and_then(Value::as_str),
            Some("deepseek-v4-flash")
        );
        assert_eq!(
            flash.get("apply_patch_tool_type").and_then(Value::as_str),
            Some("freeform")
        );
        assert!(flash
            .get("base_instructions")
            .and_then(Value::as_str)
            .is_some_and(|text| text.starts_with("You are Codex, an agent based on GPT-5")));
        let efforts = flash["supported_reasoning_levels"]
            .as_array()
            .expect("reasoning levels")
            .iter()
            .filter_map(|level| level.get("effort").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert_eq!(efforts, vec!["low", "high", "max"]);
        assert_eq!(flash.get("supports_search_tool"), Some(&json!(true)));
        assert_eq!(flash.get("web_search_tool_type"), Some(&json!("text")));
        assert_eq!(flash.get("input_modalities"), Some(&json!(["text"])));
        assert!(flash.get("model_messages").is_some());
        assert_eq!(
            flash.get("context_window").and_then(Value::as_u64),
            Some(1_048_576)
        );
        assert_eq!(
            flash.get("display_name").and_then(Value::as_str),
            Some("DeepSeek V4 Flash")
        );

        let pro = &catalog["models"][1];
        assert_eq!(
            pro.get("slug").and_then(Value::as_str),
            Some("deepseek-v4-pro")
        );
        assert_eq!(
            pro.get("context_window").and_then(Value::as_u64),
            Some(500_000)
        );
        assert_eq!(
            pro.get("max_context_window").and_then(Value::as_u64),
            Some(500_000)
        );
        assert_eq!(
            pro.get("display_name").and_then(Value::as_str),
            Some("DeepSeek-V4-Pro")
        );
    }

    #[test]
    fn native_responses_catalog_honors_per_model_reasoning_levels() {
        // The native template only declares none/high. A per-model
        // reasoningLevels override must replace supported_reasoning_levels and
        // pick a sensible default_reasoning_level.
        let settings = json!({
            "modelCatalog": {
                "models": [
                    {
                        "model": "deepseek-v4-flash",
                        "reasoningLevels": ["none", "low", "medium", "high", "xhigh", "max"],
                        "defaultReasoningLevel": "xhigh"
                    },
                    {
                        "model": "no-default-model",
                        "reasoningLevels": ["low", "medium", "high"]
                    },
                    {
                        "model": "template-default-model",
                        "reasoningLevels": ["none", "high", "xhigh"]
                    },
                    {
                        "model": "dirty-levels",
                        "reasoningLevels": ["none", "bogus", "high", ""]
                    },
                    {
                        "model": "unordered-model",
                        "reasoningLevels": ["xhigh", "low", "bogus", "low"],
                        "defaultReasoningLevel": "bogus"
                    }
                ]
            }
        });

        let catalog = codex_model_catalog_from_settings(
            &settings,
            "",
            CodexCatalogToolProfile::NativeResponses,
        )
        .expect("catalog generation should not error")
        .expect("non-empty modelCatalog must yield a catalog");

        let models = catalog["models"].as_array().expect("models array");
        let efforts = |index: usize| -> Vec<String> {
            models[index]["supported_reasoning_levels"]
                .as_array()
                .expect("supported_reasoning_levels array")
                .iter()
                .filter_map(|level| level.get("effort").and_then(|v| v.as_str()))
                .map(str::to_string)
                .collect()
        };

        // Explicit default wins.
        assert_eq!(
            efforts(0),
            vec!["none", "low", "medium", "high", "xhigh", "max"]
        );
        assert_eq!(
            models[0]
                .get("default_reasoning_level")
                .and_then(|v| v.as_str()),
            Some("xhigh")
        );

        // No explicit default: falls back to the last (highest) declared level.
        assert_eq!(efforts(1), vec!["low", "medium", "high"]);
        assert_eq!(
            models[1]
                .get("default_reasoning_level")
                .and_then(|v| v.as_str()),
            Some("high")
        );

        // Template default ("high") is kept when it is still in the list.
        assert_eq!(efforts(2), vec!["none", "high", "xhigh"]);
        assert_eq!(
            models[2]
                .get("default_reasoning_level")
                .and_then(|v| v.as_str()),
            Some("high")
        );

        // Unknown / empty efforts are dropped; the default still resolves to
        // a supported level (the template default, "high").
        assert_eq!(efforts(3), vec!["none", "high"]);
        assert_eq!(
            models[3]
                .get("default_reasoning_level")
                .and_then(|v| v.as_str()),
            Some("high")
        );

        // Declaration order is normalized to canonical order, duplicates and
        // an unknown explicit default are dropped, and the fallback picks the
        // highest supported level in canonical order (not the last declared
        // one, and never an unknown effort).
        assert_eq!(efforts(4), vec!["low", "xhigh"]);
        assert_eq!(
            models[4]
                .get("default_reasoning_level")
                .and_then(|v| v.as_str()),
            Some("xhigh")
        );
    }

    #[test]
    fn vendor_catalog_honors_per_model_reasoning_levels() {
        // The DeepSeek official catalog declares low/high/max; a per-model
        // override must win over the official entry.
        let settings = json!({
            "modelCatalog": {
                "models": [
                    {
                        "model": "deepseek-v4-flash",
                        "reasoningLevels": ["none", "low", "medium", "high", "xhigh", "max"],
                        "defaultReasoningLevel": "xhigh"
                    }
                ]
            }
        });

        let catalog = codex_model_catalog_from_settings(
            &settings,
            DEEPSEEK_NATIVE_CONFIG,
            CodexCatalogToolProfile::NativeResponses,
        )
        .expect("vendor catalog generation should not error")
        .expect("non-empty modelCatalog must yield a catalog");

        let entry = &catalog["models"][0];
        let efforts: Vec<&str> = entry["supported_reasoning_levels"]
            .as_array()
            .expect("supported_reasoning_levels array")
            .iter()
            .filter_map(|level| level.get("effort").and_then(|v| v.as_str()))
            .collect();
        assert_eq!(
            efforts,
            vec!["none", "low", "medium", "high", "xhigh", "max"]
        );
        assert_eq!(
            entry
                .get("default_reasoning_level")
                .and_then(|v| v.as_str()),
            Some("xhigh")
        );
    }

    #[test]
    fn deepseek_official_catalog_unknown_model_clones_flagship() {
        let settings = json!({
            "modelCatalog": { "models": [{ "model": "deepseek-v4-lite" }] }
        });
        let catalog = codex_model_catalog_from_settings(
            &settings,
            DEEPSEEK_NATIVE_CONFIG,
            CodexCatalogToolProfile::NativeResponses,
        )
        .expect("vendor catalog should build")
        .expect("vendor catalog should be present");
        let entry = &catalog["models"][0];

        assert_eq!(
            entry.get("slug").and_then(Value::as_str),
            Some("deepseek-v4-lite")
        );
        assert_eq!(
            entry.get("display_name").and_then(Value::as_str),
            Some("deepseek-v4-lite")
        );
        assert!(entry
            .get("priority")
            .and_then(Value::as_u64)
            .is_some_and(|value| value >= 1000));
        assert_eq!(entry.get("apply_patch_tool_type"), Some(&json!("freeform")));
        assert_eq!(
            entry.get("context_window").and_then(Value::as_u64),
            Some(1_048_576)
        );
    }

    #[test]
    fn official_vendor_catalog_is_gated_by_native_profile_and_host() {
        assert!(codex_official_vendor_catalog_models(
            DEEPSEEK_NATIVE_CONFIG,
            CodexCatalogToolProfile::NativeResponses,
        )
        .is_some_and(|models| !models.is_empty()));
        assert!(codex_official_vendor_catalog_models(
            DEEPSEEK_NATIVE_CONFIG,
            CodexCatalogToolProfile::ProxyChat,
        )
        .is_none());

        let aggregator = DEEPSEEK_NATIVE_CONFIG
            .replace("https://api.deepseek.com", "https://aggregator.example/v1");
        assert!(codex_official_vendor_catalog_models(
            &aggregator,
            CodexCatalogToolProfile::NativeResponses,
        )
        .is_none());

        for hostile_url in [
            "https://api.deepseek.com.evil.example/v1",
            "https://deepseek.com@evil.example/v1",
        ] {
            let hostile = DEEPSEEK_NATIVE_CONFIG.replace("https://api.deepseek.com", hostile_url);
            assert!(codex_official_vendor_catalog_models(
                &hostile,
                CodexCatalogToolProfile::NativeResponses,
            )
            .is_none());
        }
    }

    #[test]
    fn proxy_chat_profile_keeps_apply_patch() {
        let mut template = load_codex_native_responses_template();
        template["apply_patch_tool_type"] = json!("freeform");
        let specs = vec![CodexCatalogModelSpec {
            model: "x".to_string(),
            display_name: Some("x".to_string()),
            context_window: Some(128_000),
            supports_parallel_tool_calls: None,
            input_modalities: None,
            base_instructions: None,
            reasoning_levels: None,
            default_reasoning_level: None,
        }];
        let catalog = codex_model_catalog_from_specs(
            &specs,
            &template,
            CodexCatalogToolProfile::ProxyChat,
            128_000,
        );

        assert_eq!(
            catalog["models"][0]
                .get("apply_patch_tool_type")
                .and_then(Value::as_str),
            Some("freeform")
        );
    }

    #[test]
    fn native_web_search_field_only_removes_owned_sentinel() {
        let disabled = set_codex_native_web_search_field(r#"model = "MiniMaxAI/MiniMax-M3""#, true)
            .expect("disable web search");
        let parsed: toml::Value = toml::from_str(&disabled).expect("parse disabled config");
        assert_eq!(
            parsed.get("web_search").and_then(|value| value.as_str()),
            Some("disabled")
        );

        let cleaned = set_codex_native_web_search_field(&disabled, false).expect("clean sentinel");
        let parsed: toml::Value = toml::from_str(&cleaned).expect("parse cleaned config");
        assert!(parsed.get("web_search").is_none());

        let manual = set_codex_native_web_search_field(
            r#"web_search = "off"
model = "gpt-5.5""#,
            false,
        )
        .expect("preserve manual field");
        let parsed: toml::Value = toml::from_str(&manual).expect("parse manual config");
        assert_eq!(
            parsed.get("web_search").and_then(|value| value.as_str()),
            Some("off")
        );
    }

    #[test]
    fn build_simplified_catalog_squashes_inferred_modalities_and_keeps_overrides() {
        let catalog = r#"{
            "models": [
                { "slug": "gpt-5.4", "input_modalities": ["text", "image"] },
                { "slug": "deepseek-v4-pro", "input_modalities": ["text"] },
                { "slug": "gpt-text-override", "input_modalities": ["text"] },
                { "slug": "deepseek-v4-flash", "input_modalities": ["text", "image"] }
            ]
        }"#;

        let result = build_simplified_catalog_from_texts("", catalog).expect("entries");
        let models = result.get("models").unwrap().as_array().unwrap();

        assert!(
            models[0].get("inputModalities").is_none(),
            "GPT text+image is inferred and must not become a sticky hidden override"
        );
        assert!(
            models[1].get("inputModalities").is_none(),
            "confirmed text-only capability is inferred and must remain registry-driven"
        );
        assert_eq!(
            models[2].get("inputModalities"),
            Some(&json!(["text"])),
            "an unknown model explicitly forced to text-only must round-trip"
        );
        assert_eq!(
            models[3].get("inputModalities"),
            Some(&json!(["text", "image"])),
            "an explicit image override for a registered text-only model must round-trip"
        );
    }

    #[test]
    fn simplified_catalog_exposes_codex_model_metadata_for_models_endpoint() {
        let catalog_text = serde_json::json!({
            "models": [
                {
                    "slug": "MiniMaxAI/MiniMax-M3",
                    "display_name": "MiniMax M3",
                    "context_window": 1_000_000,
                    "supports_parallel_tool_calls": true,
                    "input_modalities": ["text", "image"]
                },
                {
                    "slug": "gpt-5.5",
                    "display_name": "gpt-5.5",
                    "context_window": 128_000
                }
            ]
        })
        .to_string();

        let output = build_simplified_catalog_from_texts(
            r#"model = "MiniMaxAI/MiniMax-M3"
model_context_window = 128000"#,
            &catalog_text,
        )
        .expect("simplified catalog");
        let models = output
            .get("models")
            .and_then(Value::as_array)
            .expect("models array");

        assert_eq!(
            models[0].get("model").and_then(Value::as_str),
            Some("MiniMaxAI/MiniMax-M3")
        );
        assert_eq!(
            models[0].get("displayName").and_then(Value::as_str),
            Some("MiniMax M3")
        );
        assert_eq!(
            models[0].get("contextWindow").and_then(Value::as_u64),
            Some(1_000_000)
        );
        assert_eq!(
            models[0]
                .get("supportsParallelToolCalls")
                .and_then(Value::as_bool),
            Some(true)
        );
        // `["text","image"]` is what the shared inference already yields for an
        // unregistered model, so it is collapsed rather than frozen into a hidden
        // row override (see `build_simplified_catalog_squashes_inferred_modalities_
        // and_keeps_overrides`).
        assert!(models[0].get("inputModalities").is_none());
        assert!(
            models[1].get("displayName").is_none(),
            "displayName is omitted when it duplicates model"
        );
        assert!(
            models[1].get("contextWindow").is_none(),
            "default contextWindow is omitted"
        );
    }

    #[test]
    fn native_gateway_rejects_web_search_by_host_or_model_brand() {
        assert!(codex_native_gateway_rejects_web_search(
            r#"model_provider = "custom"
model = "gpt-5.5"

[model_providers.custom]
base_url = "https://api.minimax.io/v1"
"#
        ));
        assert!(codex_native_gateway_rejects_web_search(
            r#"model = "qwen/qwen3-coder-plus""#
        ));
        assert!(!codex_native_gateway_rejects_web_search(
            r#"model = "gpt-5.5"
base_url = "https://relay.example/v1""#
        ));
    }

    #[test]
    fn prepare_provider_live_config_rejects_key_without_config() {
        let err = prepare_codex_provider_live_config(
            &serde_json::json!({"OPENAI_API_KEY": "sk-test"}),
            "",
        )
        .expect_err("empty config with API key should not truncate live config");

        assert!(
            err.to_string().contains("config.toml"),
            "error should explain missing config.toml, got: {err}"
        );
    }

    #[test]
    fn prepare_provider_live_config_uses_top_level_token_for_reserved_provider() {
        let input = r#"model_provider = "openai"
model = "gpt-5"
"#;

        let output = prepare_codex_provider_live_config(
            &serde_json::json!({"OPENAI_API_KEY": "sk-test"}),
            input,
        )
        .expect("prepare live config");
        let parsed: toml::Value = toml::from_str(&output).expect("parse output");

        assert_eq!(
            parsed
                .get("experimental_bearer_token")
                .and_then(|v| v.as_str()),
            Some("sk-test")
        );
        assert!(
            parsed.get("model_providers").is_none(),
            "reserved provider tables should not be synthesized"
        );
    }

    #[test]
    fn extract_bearer_uses_top_level_token_for_reserved_provider() {
        let input = r#"model_provider = "openai"
experimental_bearer_token = "top-level-key"

[model_providers.openai]
experimental_bearer_token = "stale-table-key"
"#;

        assert_eq!(
            extract_codex_experimental_bearer_token(input).as_deref(),
            Some("top-level-key")
        );
    }

    #[test]
    fn restore_provider_token_for_backfill_moves_bearer_into_auth() {
        let mut live_settings = serde_json::json!({
            "auth": {},
            "config": r#"model_provider = "vendor_alpha"

[model_providers.vendor_alpha]
base_url = "https://alpha.example/v1"
experimental_bearer_token = "sk-live"
"#
        });
        let template_settings = serde_json::json!({
            "auth": {}
        });

        restore_codex_provider_token_for_backfill(&mut live_settings, &template_settings).unwrap();

        assert_eq!(
            live_settings
                .get("auth")
                .and_then(|auth| auth.get("OPENAI_API_KEY"))
                .and_then(|value| value.as_str()),
            Some("sk-live")
        );
        let restored_config = live_settings
            .get("config")
            .and_then(|value| value.as_str())
            .expect("config should remain");
        assert!(
            !restored_config.contains("experimental_bearer_token"),
            "stored provider TOML should not keep the live-only token"
        );
    }

    #[test]
    fn strip_mcp_servers_from_settings_removes_projection_and_legacy_form() {
        let mut settings = serde_json::json!({
            "auth": {},
            "config": "# keep\nmodel = \"gpt-5\"\n\n[mcp_servers.echo]\ncommand = \"echo\"\n\n[mcp.servers.legacy]\ncommand = \"noop\"\n"
        });

        strip_codex_mcp_servers_from_settings(&mut settings).expect("strip projection");
        let config = settings["config"].as_str().expect("config text");
        assert!(!config.contains("mcp_servers"), "got: {config}");
        assert!(!config.contains("[mcp"), "got: {config}");
        assert!(config.contains("# keep"));
        assert!(config.contains("model = \"gpt-5\""));
    }

    #[test]
    fn strip_mcp_servers_from_settings_is_byte_identical_without_mcp() {
        let original = "# keep\nmodel = \"gpt-5\"\n";
        let mut settings = serde_json::json!({ "config": original });
        strip_codex_mcp_servers_from_settings(&mut settings).expect("no-op strip");
        assert_eq!(settings["config"].as_str(), Some(original));
    }

    #[test]
    fn should_not_restore_provider_token_for_oauth_only_template() {
        let oauth_template = serde_json::json!({
            "auth": {
                "auth_mode": "chatgpt",
                "tokens": {
                    "access_token": "oauth-access"
                }
            }
        });
        let api_key_template = serde_json::json!({
            "auth": {
                "OPENAI_API_KEY": "sk-test"
            }
        });

        assert!(
            !should_restore_codex_provider_token_for_backfill(Some("custom"), &oauth_template),
            "OAuth-only templates should not backfill bearer tokens into OPENAI_API_KEY"
        );
        assert!(
            should_restore_codex_provider_token_for_backfill(Some("custom"), &api_key_template),
            "custom API-key providers should still restore provider bearer tokens"
        );
        assert!(
            !should_restore_codex_provider_token_for_backfill(Some("official"), &api_key_template),
            "official providers should never restore third-party bearer tokens"
        );
    }

    #[test]
    fn credential_login_material_only_counts_real_credentials() {
        assert!(codex_auth_has_credential_login_material(&json!({
            "tokens": { "access_token": "t" }
        })));
        assert!(codex_auth_has_credential_login_material(&json!({
            "tokens": { "refresh_token": "r" }
        })));
        assert!(codex_auth_has_credential_login_material(&json!({
            "personal_access_token": "pat"
        })));

        assert!(!codex_auth_has_credential_login_material(&json!({
            "OPENAI_API_KEY": "sk-x"
        })));
        assert!(!codex_auth_has_credential_login_material(&json!({
            "OPENAI_API_KEY": "sk-x",
            "last_refresh": "2026-01-01T00:00:00Z",
            "tokens": { "account_id": "acct-meta-only" }
        })));
        assert!(!codex_auth_has_credential_login_material(&json!({})));
    }

    #[test]
    fn stale_third_party_residue_detection() {
        assert!(codex_live_auth_is_stale_third_party_residue(&json!({
            "OPENAI_API_KEY": "sk-third-party"
        })));
        assert!(codex_live_auth_is_stale_third_party_residue(&json!({
            "auth_mode": "apikey",
            "OPENAI_API_KEY": "sk-third-party",
            "tokens": { "account_id": "metadata-only" }
        })));
        assert!(!codex_live_auth_is_stale_third_party_residue(&json!({
            "OPENAI_API_KEY": "sk-x",
            "tokens": { "access_token": "official-token" }
        })));
        assert!(!codex_live_auth_is_stale_third_party_residue(&json!({})));
        assert!(!codex_live_auth_is_stale_third_party_residue(&json!({
            "OPENAI_API_KEY": ""
        })));
    }

    #[test]
    fn prepare_provider_live_config_preserves_custom_provider_id() {
        let input = r#"model_provider = "vendor_alpha"
model = "gpt-5.4"
profile = "work"

[model_providers.vendor_alpha]
name = "Vendor Alpha"
base_url = "https://alpha.example/v1"
wire_api = "responses"

[profiles.work]
model_provider = "vendor_alpha"
model = "gpt-5.4"
"#;

        let result = prepare_codex_provider_live_config(
            &serde_json::json!({"OPENAI_API_KEY": "sk-test"}),
            input,
        )
        .expect("prepare live config");
        let parsed: toml::Value = toml::from_str(&result).unwrap();

        assert_eq!(
            parsed.get("model_provider").and_then(|v| v.as_str()),
            Some("vendor_alpha")
        );
        assert!(
            parsed
                .get("model_providers")
                .and_then(|v| v.get("custom"))
                .is_none(),
            "provider writes should not force custom provider ids"
        );
        assert_eq!(
            parsed
                .get("model_providers")
                .and_then(|v| v.get("vendor_alpha"))
                .and_then(|v| v.get("experimental_bearer_token"))
                .and_then(|v| v.as_str()),
            Some("sk-test")
        );
        assert_eq!(
            parsed
                .get("profiles")
                .and_then(|v| v.get("work"))
                .and_then(|v| v.get("model_provider"))
                .and_then(|v| v.as_str()),
            Some("vendor_alpha"),
            "profile provider references should be preserved"
        );
    }

    #[test]
    fn backfill_preserves_live_model_provider_id() {
        let mut live_settings = serde_json::json!({
            "auth": {},
            "config": r#"model_provider = "vendor_beta"

[model_providers.vendor_beta]
name = "Vendor Beta"
base_url = "https://beta.example/v1"
wire_api = "responses"
"#,
        });
        let template_settings = serde_json::json!({
            "auth": {},
            "config": r#"model_provider = "custom"

[model_providers.custom]
name = "Custom"
base_url = "https://custom.example/v1"
wire_api = "responses"
"#,
        });

        restore_codex_settings_for_backfill(&mut live_settings, &template_settings, false).unwrap();
        let config = live_settings.get("config").and_then(Value::as_str).unwrap();
        let parsed: toml::Value = toml::from_str(config).unwrap();

        assert_eq!(
            parsed.get("model_provider").and_then(|v| v.as_str()),
            Some("vendor_beta")
        );
        assert!(
            parsed
                .get("model_providers")
                .and_then(|v| v.get("vendor_beta"))
                .is_some(),
            "backfill should not rewrite user-selected provider tables"
        );
    }

    #[test]
    fn base_url_writes_into_correct_model_provider_section() {
        let input = r#"model_provider = "any"
model = "gpt-5.1-codex"

[model_providers.any]
name = "any"
wire_api = "responses"
"#;

        let result = update_codex_toml_field(input, "base_url", "https://example.com/v1").unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();

        let base_url = parsed
            .get("model_providers")
            .and_then(|v| v.get("any"))
            .and_then(|v| v.get("base_url"))
            .and_then(|v| v.as_str())
            .expect("base_url should be in model_providers.any");
        assert_eq!(base_url, "https://example.com/v1");

        // Should NOT have top-level base_url
        assert!(parsed.get("base_url").is_none());

        // wire_api preserved
        let wire_api = parsed
            .get("model_providers")
            .and_then(|v| v.get("any"))
            .and_then(|v| v.get("wire_api"))
            .and_then(|v| v.as_str());
        assert_eq!(wire_api, Some("responses"));
    }

    #[test]
    fn base_url_creates_section_when_missing() {
        let input = r#"model_provider = "custom"
model = "gpt-4"
"#;

        let result = update_codex_toml_field(input, "base_url", "https://custom.api/v1").unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();

        let base_url = parsed
            .get("model_providers")
            .and_then(|v| v.get("custom"))
            .and_then(|v| v.get("base_url"))
            .and_then(|v| v.as_str())
            .expect("should create section and set base_url");
        assert_eq!(base_url, "https://custom.api/v1");
    }

    #[test]
    fn base_url_falls_back_to_top_level_without_model_provider() {
        let input = r#"model = "gpt-4"
"#;

        let result = update_codex_toml_field(input, "base_url", "https://fallback.api/v1").unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();

        let base_url = parsed
            .get("base_url")
            .and_then(|v| v.as_str())
            .expect("should set top-level base_url");
        assert_eq!(base_url, "https://fallback.api/v1");
    }

    #[test]
    fn base_url_writes_into_inline_table_provider_section() {
        // inline table 是合法 TOML，但 as_table_mut() 对它返回 None。旧代码会因此
        // 掉进「写顶层字段」的 fallback：用户改的 base_url 落在错误层级，
        // Codex 读不到，且界面毫无提示。
        let input = r#"model_provider = "any"
model_providers = { any = { name = "any", base_url = "https://old.api/v1", wire_api = "responses" } }
"#;

        let result = update_codex_toml_field(input, "base_url", "https://new.api/v1").unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();

        assert_eq!(
            parsed["model_providers"]["any"]["base_url"].as_str(),
            Some("https://new.api/v1"),
            "must update the provider section, not a top-level field"
        );
        assert!(
            parsed.get("base_url").is_none(),
            "must not leak a top-level base_url fallback"
        );
        assert_eq!(
            parsed["model_providers"]["any"]["wire_api"].as_str(),
            Some("responses"),
            "sibling fields must survive"
        );
    }

    #[test]
    fn clearing_base_url_removes_only_from_correct_section() {
        let input = r#"model_provider = "any"

[model_providers.any]
name = "any"
base_url = "https://old.api/v1"
wire_api = "responses"

[mcp_servers.context7]
command = "npx"
"#;

        let result = update_codex_toml_field(input, "base_url", "").unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();

        // base_url removed from model_providers.any
        let any_section = parsed
            .get("model_providers")
            .and_then(|v| v.get("any"))
            .expect("model_providers.any should exist");
        assert!(any_section.get("base_url").is_none());

        // wire_api preserved
        assert_eq!(
            any_section.get("wire_api").and_then(|v| v.as_str()),
            Some("responses")
        );

        // mcp_servers untouched
        assert!(parsed.get("mcp_servers").is_some());
    }

    #[test]
    fn model_field_operates_on_top_level() {
        let input = r#"model_provider = "any"
model = "gpt-4"

[model_providers.any]
name = "any"
"#;

        let result = update_codex_toml_field(input, "model", "gpt-5").unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();
        assert_eq!(parsed.get("model").and_then(|v| v.as_str()), Some("gpt-5"));

        // Clear model
        let result2 = update_codex_toml_field(&result, "model", "").unwrap();
        let parsed2: toml::Value = toml::from_str(&result2).unwrap();
        assert!(parsed2.get("model").is_none());
    }

    #[test]
    fn preserves_comments_and_whitespace() {
        let input = r#"# My Codex config
model_provider = "any"
model = "gpt-4"

# Provider section
[model_providers.any]
name = "any"
base_url = "https://old.api/v1"
"#;

        let result = update_codex_toml_field(input, "base_url", "https://new.api/v1").unwrap();

        // Comments should be preserved
        assert!(result.contains("# My Codex config"));
        assert!(result.contains("# Provider section"));
    }

    #[test]
    fn does_not_misplace_when_profiles_section_follows() {
        let input = r#"model_provider = "any"

[model_providers.any]
name = "any"
base_url = "https://old.api/v1"

[profiles.default]
model = "gpt-4"
"#;

        let result = update_codex_toml_field(input, "base_url", "https://new.api/v1").unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();

        // base_url in correct section
        let base_url = parsed
            .get("model_providers")
            .and_then(|v| v.get("any"))
            .and_then(|v| v.get("base_url"))
            .and_then(|v| v.as_str());
        assert_eq!(base_url, Some("https://new.api/v1"));

        // profiles section untouched
        let profile_model = parsed
            .get("profiles")
            .and_then(|v| v.get("default"))
            .and_then(|v| v.get("model"))
            .and_then(|v| v.as_str());
        assert_eq!(profile_model, Some("gpt-4"));
    }

    #[test]
    fn remove_base_url_if_predicate() {
        let input = r#"model_provider = "any"

[model_providers.any]
name = "any"
base_url = "http://127.0.0.1:5000/v1"
wire_api = "responses"
"#;

        let result =
            remove_codex_toml_base_url_if(input, |url| url.starts_with("http://127.0.0.1"));
        let parsed: toml::Value = toml::from_str(&result).unwrap();

        let any_section = parsed
            .get("model_providers")
            .and_then(|v| v.get("any"))
            .unwrap();
        assert!(any_section.get("base_url").is_none());
        assert_eq!(
            any_section.get("wire_api").and_then(|v| v.as_str()),
            Some("responses")
        );
    }

    #[test]
    fn remove_base_url_if_keeps_non_matching() {
        let input = r#"model_provider = "any"

[model_providers.any]
base_url = "https://production.api/v1"
"#;

        let result =
            remove_codex_toml_base_url_if(input, |url| url.starts_with("http://127.0.0.1"));
        let parsed: toml::Value = toml::from_str(&result).unwrap();

        let base_url = parsed
            .get("model_providers")
            .and_then(|v| v.get("any"))
            .and_then(|v| v.get("base_url"))
            .and_then(|v| v.as_str());
        assert_eq!(base_url, Some("https://production.api/v1"));
    }

    #[test]
    fn set_catalog_json_some_preserves_user_owned_full_path() {
        let input = r#"model_provider = "custom"
model_catalog_json = "/Users/me/.codex/my-custom-catalog.json"
"#;

        let result = set_codex_model_catalog_json_field(
            input,
            Some(Path::new("/tmp/cc-switch-model-catalog.json")),
        )
        .expect("update catalog pointer");
        let parsed: toml::Value = toml::from_str(&result).expect("parse updated config");

        assert_eq!(
            parsed
                .get("model_catalog_json")
                .and_then(|value| value.as_str()),
            Some("/Users/me/.codex/my-custom-catalog.json")
        );
    }

    #[test]
    fn set_catalog_json_some_preserves_user_owned_relative_filename() {
        let input = r#"model_provider = "custom"
model_catalog_json = "my-custom-catalog.json"
"#;

        let result = set_codex_model_catalog_json_field(
            input,
            Some(Path::new("/tmp/cc-switch-model-catalog.json")),
        )
        .expect("update catalog pointer");
        let parsed: toml::Value = toml::from_str(&result).expect("parse updated config");

        assert_eq!(
            parsed
                .get("model_catalog_json")
                .and_then(|value| value.as_str()),
            Some("my-custom-catalog.json")
        );
    }

    #[test]
    fn set_catalog_json_some_claims_absent_or_owned_pointer() {
        for input in [
            "model_provider = \"custom\"\n",
            "model_catalog_json = \"nested/cc-switch-model-catalog.json\"\n",
        ] {
            let result = set_codex_model_catalog_json_field(
                input,
                Some(Path::new("/tmp/cc-switch-model-catalog.json")),
            )
            .expect("update catalog pointer");
            let parsed: toml::Value = toml::from_str(&result).expect("parse updated config");

            assert_eq!(
                parsed
                    .get("model_catalog_json")
                    .and_then(|value| value.as_str()),
                Some(CC_SWITCH_CODEX_MODEL_CATALOG_FILENAME)
            );
        }
    }

    #[test]
    fn resolve_catalog_path_requires_cc_switch_owned_filename() {
        let base = PathBuf::from("/tmp/.codex");
        assert!(resolve_cc_switch_catalog_path("", &base).is_none());
        assert!(resolve_cc_switch_catalog_path("model = \"gpt-5\"", &base).is_none());
        assert!(resolve_cc_switch_catalog_path(
            "model_catalog_json = \"my-handwritten-catalog.json\"",
            &base,
        )
        .is_none());
    }

    #[test]
    fn resolve_catalog_path_accepts_relative_owned_file() {
        let base = PathBuf::from("/home/user/.codex");
        let resolved = resolve_cc_switch_catalog_path(
            "model_catalog_json = \"cc-switch-model-catalog.json\"",
            &base,
        );
        assert_eq!(
            resolved,
            Some(base.join(CC_SWITCH_CODEX_MODEL_CATALOG_FILENAME))
        );
    }

    #[test]
    fn resolve_catalog_path_rejects_absolute_and_relative_escapes() {
        let temp = tempfile::tempdir().expect("tempdir");
        let base = temp.path().join("codex");
        let outside = temp.path().join(CC_SWITCH_CODEX_MODEL_CATALOG_FILENAME);
        let absolute_config = format!("model_catalog_json = '{}'", outside.display());

        assert!(resolve_cc_switch_catalog_path(&absolute_config, &base).is_none());
        assert!(resolve_cc_switch_catalog_path(
            "model_catalog_json = \"../cc-switch-model-catalog.json\"",
            &base,
        )
        .is_none());
    }

    #[test]
    fn resolve_catalog_path_accepts_absolute_file_inside_config_dir() {
        let temp = tempfile::tempdir().expect("tempdir");
        let base = temp.path().join("codex");
        let inside = base.join(CC_SWITCH_CODEX_MODEL_CATALOG_FILENAME);
        let config = format!("model_catalog_json = '{}'", inside.display());

        assert_eq!(resolve_cc_switch_catalog_path(&config, &base), Some(inside));
    }

    #[test]
    #[cfg(any(unix, windows))]
    fn resolve_catalog_path_rejects_symlink_escape() {
        let temp = tempfile::tempdir().expect("tempdir");
        let base = temp.path().join("codex");
        let outside = temp.path().join("outside");
        fs::create_dir_all(&base).expect("create base");
        fs::create_dir_all(&outside).expect("create outside");
        fs::write(
            outside.join(CC_SWITCH_CODEX_MODEL_CATALOG_FILENAME),
            r#"{"models":[]}"#,
        )
        .expect("write escaped catalog");

        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, base.join("link")).expect("symlink");
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&outside, base.join("link")).expect("symlink");

        let config = "model_catalog_json = \"link/cc-switch-model-catalog.json\"";
        assert!(resolve_cc_switch_catalog_path(config, &base).is_none());
    }

    #[test]
    fn resolve_catalog_path_accepts_real_file_inside_config_dir() {
        let temp = tempfile::tempdir().expect("tempdir");
        let base = temp.path().join("codex");
        fs::create_dir_all(&base).expect("create base");
        let catalog = base.join(CC_SWITCH_CODEX_MODEL_CATALOG_FILENAME);
        fs::write(&catalog, r#"{"models":[]}"#).expect("write catalog");

        let resolved = resolve_cc_switch_catalog_path(
            "model_catalog_json = \"cc-switch-model-catalog.json\"",
            &base,
        )
        .expect("catalog should resolve");
        assert_eq!(resolved, fs::canonicalize(catalog).expect("canonical file"));
    }

    #[test]
    fn read_limited_string_rejects_oversized_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("huge.json");
        let file = fs::File::create(&path).expect("create catalog");
        file.set_len(MAX_CODEX_CATALOG_BYTES + 1)
            .expect("extend catalog");

        assert!(read_codex_model_catalog_text(&path).is_err());
    }
}
