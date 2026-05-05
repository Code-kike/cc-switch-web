//! Bootstrap — common initialization for desktop and web-server runtimes.
//!
//! Layer 1 / Task 2 (partial scaffolding).
//!
//! Full integration (extracting `lib.rs::setup()` body, wiring `services/proxy`
//! and the proxy crate into `UiEventSink`) is deferred to a follow-up patch.
//! For now we expose:
//!   - `RuntimeMode` flag distinguishing the two front ends
//!   - `init_database_locking_pragmas` helper to bind WAL + busy_timeout
//!   - `acquire_data_dir_lock` cross-process advisory lock (web-server only)
//!   - `migration_marker_path` / sidecar helpers for migration audit

use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeMode {
    Desktop,
    Web,
}

impl RuntimeMode {
    pub fn is_web(self) -> bool {
        matches!(self, Self::Web)
    }

    pub fn is_desktop(self) -> bool {
        matches!(self, Self::Desktop)
    }
}

/// Default data directory, `~/.cc-switch`. Override via `CC_SWITCH_DATA_DIR`
/// environment variable (used by Docker `/data` volume).
pub fn data_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("CC_SWITCH_DATA_DIR") {
        return PathBuf::from(custom);
    }
    dirs::home_dir()
        .map(|h| h.join(".cc-switch"))
        .unwrap_or_else(|| PathBuf::from(".cc-switch"))
}

/// Path of the migration audit sidecar, `<data_dir>/migration.marker`.
pub fn migration_marker_path(data_dir: &Path) -> PathBuf {
    data_dir.join("migration.marker")
}

/// Path of the cross-process DB lock, `<data_dir>/cc-switch.db.lock`.
pub fn db_lock_path(data_dir: &Path) -> PathBuf {
    data_dir.join("cc-switch.db.lock")
}

/// SQLite pragma bundle to enable WAL, busy_timeout, NORMAL fsync, in-memory
/// temp store. Apply on every fresh connection.
///
/// Layer 1 / Task 2 (Round 2 P0-1 + Round 5 P0-1).
pub const SQLITE_PRAGMAS: &[(&str, &str)] = &[
    ("journal_mode", "WAL"),
    ("busy_timeout", "5000"),
    ("synchronous", "NORMAL"),
    ("temp_store", "MEMORY"),
    ("foreign_keys", "ON"),
];

/// DDL for the cookie session store. Idempotent (`CREATE TABLE IF NOT EXISTS`).
///
/// Layer 1 / Task 2 (Round 4 P0-2 + Round 5 P0-1 + Round 5 P1-6 rename).
pub const WEB_SESSIONS_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS web_sessions (
    session_id TEXT PRIMARY KEY,
    user TEXT NOT NULL DEFAULT 'admin',
    csrf_token TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    last_active_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    ip_hint TEXT,
    user_agent_hint TEXT
);
CREATE INDEX IF NOT EXISTS idx_web_sessions_expires ON web_sessions(expires_at);
"#;

/// DDL for the audit log used by Round 5 P1-3 observability.
pub const AUDIT_LOG_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ts INTEGER NOT NULL,
    kind TEXT NOT NULL,
    actor TEXT,
    payload TEXT,
    request_id TEXT
);
CREATE INDEX IF NOT EXISTS idx_audit_log_ts ON audit_log(ts);
"#;

/// Schema version baked into `PRAGMA user_version`. Matches v3.14.1 + web
/// extensions. Bump when adding columns or tables; see Round 5 P0-1.
pub const SCHEMA_VERSION: u32 = 3_140_110;

/// Cross-process advisory lock for the data directory (web-server only).
///
/// Returns the locked file handle; dropping the handle releases the lock.
/// On NFS / 9p / sshfs the call may fail or be silently lost; callers should
/// also gate on `check_filesystem_local` to refuse non-local volumes.
#[cfg(feature = "fs2")]
pub fn acquire_data_dir_lock(data_dir: &Path) -> Result<std::fs::File, String> {
    use fs2::FileExt;
    std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
    let lock_path = db_lock_path(data_dir);
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .read(true)
        .open(&lock_path)
        .map_err(|e| format!("open {}: {}", lock_path.display(), e))?;
    file.try_lock_exclusive()
        .map_err(|e| format!("data dir already locked: {e}"))?;
    Ok(file)
}

/// Best-effort check that the data directory lives on a local filesystem.
/// Layer 1 / Task 2 (Round 3 P0-1 + Round 4 P1-8).
#[cfg(target_os = "linux")]
pub fn check_filesystem_local(path: &Path) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
    let target = path.canonicalize().map_err(|e| e.to_string())?;
    let mounts = match std::fs::read_to_string("/proc/self/mounts") {
        Ok(s) => s,
        Err(_) => return Ok(()),
    };

    let mut best_match_len: usize = 0;
    let mut best_fstype: Option<String> = None;
    for line in mounts.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }
        let mount_point = parts[1];
        let fstype = parts[2];
        if target.starts_with(mount_point) && mount_point.len() > best_match_len {
            best_match_len = mount_point.len();
            best_fstype = Some(fstype.to_lowercase());
        }
    }

    if let Some(fs) = best_fstype {
        let blocked = matches!(
            fs.as_str(),
            "nfs"
                | "nfs4"
                | "cifs"
                | "smbfs"
                | "smb2"
                | "smb3"
                | "9p"
                | "fuse.sshfs"
                | "fuse.gvfs"
                | "gvfs"
        );
        if blocked {
            return Err(format!(
                "cc-switch DB only supports local filesystems; detected `{fs}` for {}",
                target.display()
            ));
        }
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn check_filesystem_local(_path: &Path) -> Result<(), String> {
    // macOS / Windows: defer to higher-level checks; native APIs vary widely.
    Ok(())
}
