//! 数据库备份和恢复
//!
//! 提供 SQL 导出/导入和二进制快照备份功能。

use super::{lock_conn, Database};
use crate::config::get_app_config_dir;
use crate::error::AppError;
use chrono::{Local, Utc};
use rusqlite::backup::Backup;
use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use rusqlite::types::ValueRef;
use rusqlite::Connection;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

const CC_SWITCH_SQL_EXPORT_HEADER: &str = "-- CC Switch SQLite 导出";

/// Bound combined INSERT batches while still amortizing statement parsing.
/// A row larger than this cap is emitted alone because it cannot be split.
const INSERT_BATCH_MAX_ROWS: usize = 200;
const INSERT_BATCH_MAX_BYTES: usize = 1024 * 1024;

const SQL_RESTORE_TABLES: &[&str] = &[
    "providers",
    "provider_endpoints",
    "mcp_servers",
    "prompts",
    "skills",
    "skill_repos",
    "settings",
    "proxy_config",
    "provider_health",
    "proxy_request_logs",
    "model_pricing",
    "stream_check_logs",
    "proxy_live_backup",
    "usage_daily_rollups",
    "session_log_sync",
    "profiles",
];

const LEGACY_SQL_RESTORE_TABLES: &[&str] = &["circuit_breaker_config", "failover_queue"];

const SQL_RESTORE_INDEXES: &[(&str, &str)] = &[
    ("idx_providers_failover", "providers"),
    ("idx_request_logs_provider", "proxy_request_logs"),
    ("idx_request_logs_created_at", "proxy_request_logs"),
    ("idx_request_logs_model", "proxy_request_logs"),
    ("idx_request_logs_session", "proxy_request_logs"),
    ("idx_request_logs_status", "proxy_request_logs"),
    ("idx_request_logs_app_created_at", "proxy_request_logs"),
    ("idx_request_logs_dedup_lookup_expr", "proxy_request_logs"),
    ("idx_stream_check_logs_provider", "stream_check_logs"),
    // Older supported exports; migrations remove/replace these objects.
    ("idx_request_logs_dedup_lookup", "proxy_request_logs"),
    ("idx_failover_queue_order", "failover_queue"),
];

type SchemaObject = (String, String);
type TableColumnSignature = (i64, String, String, i64, Option<String>, i64, i64);
type ForeignKeySignature = (
    i64,
    i64,
    String,
    String,
    Option<String>,
    String,
    String,
    String,
);
type IndexColumnSignature = (i64, i64, Option<String>, i64, String, i64);
type TableIndexSignature = (String, i64, String, i64, Vec<IndexColumnSignature>);

#[cfg(test)]
type RestoreCriticalSectionHook = (usize, Box<dyn FnOnce() + Send>);

#[cfg(test)]
static RESTORE_CRITICAL_SECTION_HOOK: std::sync::Mutex<Option<RestoreCriticalSectionHook>> =
    std::sync::Mutex::new(None);

/// Tables whose data rows are skipped when exporting for WebDAV sync.
const SYNC_SKIP_TABLES: &[&str] = &[
    "proxy_request_logs",
    "stream_check_logs",
    "provider_health",
    "proxy_live_backup",
    "usage_daily_rollups",
];

/// Tables whose local data is preserved (restored from local snapshot) during WebDAV import.
/// Excludes ephemeral tables like provider_health that can safely rebuild at runtime.
const SYNC_PRESERVE_TABLES: &[&str] = &[
    "proxy_request_logs",
    "stream_check_logs",
    "proxy_live_backup",
    "usage_daily_rollups",
];

/// A database backup entry for the UI
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupEntry {
    pub filename: String,
    pub size_bytes: u64,
    pub created_at: String, // ISO 8601
}

impl Database {
    /// 导出为 SQLite 兼容的 SQL 文本（内存字符串，完整导出）
    pub fn export_sql_string(&self) -> Result<String, AppError> {
        let snapshot = self.snapshot_to_memory()?;
        Self::dump_sql(&snapshot, &[])
    }

    /// Export SQL for sync (WebDAV), skipping local-only tables' data
    pub fn export_sql_string_for_sync(&self) -> Result<String, AppError> {
        let snapshot = self.snapshot_to_memory()?;
        Self::dump_sql(&snapshot, SYNC_SKIP_TABLES)
    }

    /// 导出为 SQLite 兼容的 SQL 文本
    pub fn export_sql(&self, target_path: &Path) -> Result<(), AppError> {
        let dump = self.export_sql_string()?;

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
        }

        crate::config::atomic_write(target_path, dump.as_bytes())
    }

    /// 从 SQL 文件导入，返回生成的备份 ID（若无备份则为空字符串）
    pub fn import_sql(&self, source_path: &Path) -> Result<String, AppError> {
        if !source_path.exists() {
            return Err(AppError::InvalidInput(format!(
                "SQL 文件不存在: {}",
                source_path.display()
            )));
        }

        let sql_raw = fs::read_to_string(source_path).map_err(|e| AppError::io(source_path, e))?;
        let sql_content = sql_raw.trim_start_matches('\u{feff}');
        self.import_sql_string(sql_content)
    }

    /// 从 SQL 字符串导入，返回生成的备份 ID（若无备份则为空字符串）
    pub fn import_sql_string(&self, sql_raw: &str) -> Result<String, AppError> {
        self.import_sql_string_inner(sql_raw, &[])
    }

    /// Import SQL generated for sync, then restore local-only tables from the
    /// current device snapshot before replacing the main database.
    pub(crate) fn import_sql_string_for_sync(&self, sql_raw: &str) -> Result<String, AppError> {
        self.import_sql_string_inner(sql_raw, SYNC_PRESERVE_TABLES)
    }

    fn import_sql_string_inner(
        &self,
        sql_raw: &str,
        preserve_tables: &[&str],
    ) -> Result<String, AppError> {
        let sql_content = sql_raw.trim_start_matches('\u{feff}');
        Self::validate_cc_switch_sql_export(sql_content)?;

        let local_snapshot = if preserve_tables.is_empty() {
            None
        } else {
            Some(self.snapshot_to_memory()?)
        };

        // 在临时数据库执行导入，确保失败不会污染主库
        let temp_file = NamedTempFile::new().map_err(|e| AppError::IoContext {
            context: "创建临时数据库文件失败".to_string(),
            source: e,
        })?;
        let temp_path = temp_file.path().to_path_buf();
        let temp_conn =
            Connection::open(&temp_path).map_err(|e| AppError::Database(e.to_string()))?;

        // The authorizer covers external SQL only. Clear it before trusted schema
        // maintenance so migrations are checked by the canonical-schema pipeline,
        // rather than by rules intended for untrusted backup text.
        Self::install_sql_restore_authorizer(&temp_conn);
        let import_result = temp_conn
            .execute_batch(sql_content)
            .map_err(|e| AppError::Database(format!("执行 SQL 导入失败: {e}")));
        temp_conn.authorizer(None::<fn(AuthContext<'_>) -> Authorization>);
        import_result?;

        Self::validate_restore_objects(&temp_conn)?;
        temp_conn
            .execute("PRAGMA foreign_keys = ON;", [])
            .map_err(|e| AppError::Database(format!("启用导入库外键约束失败: {e}")))?;

        // 补齐缺失表/索引并校验迁移后的 schema。
        Self::create_tables_on_conn(&temp_conn)?;
        Self::apply_schema_migrations_on_conn(&temp_conn)?;
        Self::validate_current_schema(&temp_conn, true)?;
        Self::validate_basic_state(&temp_conn)?;

        // 不可信 DDL 永不直接进入 live DB：无条件将数据复制到可信代码创建的 canonical schema。
        let normalized_file = NamedTempFile::new().map_err(|e| AppError::IoContext {
            context: "创建规范化数据库文件失败".to_string(),
            source: e,
        })?;
        let normalized_conn = Connection::open(normalized_file.path())
            .map_err(|e| AppError::Database(format!("创建规范化数据库失败: {e}")))?;
        normalized_conn
            .execute("PRAGMA foreign_keys = OFF;", [])
            .map_err(|e| AppError::Database(format!("关闭规范化库外键约束失败: {e}")))?;
        Self::create_tables_on_conn(&normalized_conn)?;
        Self::apply_schema_migrations_on_conn(&normalized_conn)?;
        Self::restore_tables(&temp_conn, &normalized_conn, SQL_RESTORE_TABLES)?;
        normalized_conn
            .execute("PRAGMA foreign_keys = ON;", [])
            .map_err(|e| AppError::Database(format!("启用规范化库外键约束失败: {e}")))?;
        if !Self::validate_current_schema(&normalized_conn, false)? {
            return Err(AppError::Database(
                "规范化数据库未生成当前 canonical schema".to_string(),
            ));
        }

        if let Some(local_snapshot) = local_snapshot.as_ref() {
            Self::restore_tables(local_snapshot, &normalized_conn, preserve_tables)?;
        }
        Self::validate_database_integrity(&normalized_conn)?;

        let backup_path = self.replace_main_with_safety_backup(&normalized_conn)?;

        let backup_id = backup_path
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
            .unwrap_or_default();

        Ok(backup_id)
    }

    /// Install the authorizer used only while executing untrusted backup SQL.
    ///
    /// The upstream escape boundary denies parsed actions that can leave the temporary
    /// database: ATTACH (also emitted by VACUUM/VACUUM INTO), virtual tables, unknown
    /// future actions, and unsafe PRAGMAs. This fork intentionally keeps the stricter
    /// canonical-restore allow-list as a second boundary so imported executable schema
    /// and non-canonical objects can never become live database state.
    fn install_sql_restore_authorizer(conn: &Connection) {
        conn.authorizer(Some(|ctx: AuthContext<'_>| {
            let escapes_temp_db = matches!(
                ctx.action,
                AuthAction::Attach { .. }
                    | AuthAction::Detach { .. }
                    | AuthAction::CreateVtable { .. }
                    | AuthAction::DropVtable { .. }
                    | AuthAction::Unknown { .. }
            ) || matches!(
                ctx.action,
                AuthAction::Pragma {
                    pragma_name,
                    pragma_value,
                } if !Self::is_allowed_restore_pragma(pragma_name, pragma_value)
            ) || ctx.accessor.is_some()
                || matches!(ctx.database_name, Some(database) if database != "main");

            if escapes_temp_db {
                // Do not include filenames or SQL text in logs: imported paths may be sensitive.
                log::warn!(
                    "SQL import rejected a statement outside the temporary restore database"
                );
                return Authorization::Deny;
            }

            let allowed = match ctx.action {
                AuthAction::CreateTable { table_name } => {
                    Self::is_known_restore_table(table_name) || table_name == "sqlite_sequence"
                }
                AuthAction::CreateIndex {
                    index_name,
                    table_name,
                } => {
                    Self::restore_index_table(index_name) == Some(table_name)
                        || (index_name.starts_with("sqlite_autoindex_")
                            && Self::is_known_restore_table(table_name))
                }
                AuthAction::Insert { table_name } => {
                    table_name == "sqlite_master" || Self::is_known_restore_table(table_name)
                }
                AuthAction::Update {
                    table_name,
                    column_name,
                } => {
                    table_name == "sqlite_master"
                        && matches!(
                            column_name,
                            "type" | "name" | "tbl_name" | "rootpage" | "sql"
                        )
                }
                AuthAction::Read {
                    table_name,
                    column_name,
                } => {
                    Self::is_known_restore_table(table_name)
                        || (table_name == "sqlite_master"
                            && column_name.eq_ignore_ascii_case("rowid"))
                }
                // SQLite represents a multi-row VALUES clause as an internal
                // SELECT. Table/column reads and functions still receive their
                // own authorizer actions and remain constrained below.
                AuthAction::Select => true,
                AuthAction::Pragma {
                    pragma_name,
                    pragma_value,
                } => Self::is_allowed_restore_pragma(pragma_name, pragma_value),
                AuthAction::Transaction { .. } => true,
                AuthAction::Reindex { index_name } => {
                    Self::restore_index_table(index_name).is_some()
                }
                AuthAction::Function { function_name } => {
                    function_name.eq_ignore_ascii_case("coalesce")
                }
                _ => false,
            };

            if allowed {
                Authorization::Allow
            } else {
                Authorization::Deny
            }
        }));
    }

    fn is_known_restore_table(table: &str) -> bool {
        SQL_RESTORE_TABLES.contains(&table) || LEGACY_SQL_RESTORE_TABLES.contains(&table)
    }

    fn restore_index_table(index: &str) -> Option<&'static str> {
        SQL_RESTORE_INDEXES
            .iter()
            .find_map(|(name, table)| (*name == index).then_some(*table))
    }

    fn is_allowed_restore_pragma(name: &str, value: Option<&str>) -> bool {
        if name.eq_ignore_ascii_case("foreign_keys") {
            return value.is_some_and(|value| {
                value.eq_ignore_ascii_case("on") || value.eq_ignore_ascii_case("off")
            });
        }

        if name.eq_ignore_ascii_case("user_version") {
            return value
                .and_then(|value| value.parse::<i32>().ok())
                .is_some_and(|version| (0..=super::SCHEMA_VERSION).contains(&version));
        }

        false
    }

    fn validate_restore_objects(conn: &Connection) -> Result<(), AppError> {
        let version = Self::get_user_version(conn)?;
        let mut has_providers = false;
        let mut has_mcp_servers = false;
        let mut stmt = conn
            .prepare(
                "SELECT type, name, tbl_name
                 FROM sqlite_master
                 WHERE name NOT LIKE 'sqlite_%'",
            )
            .map_err(|e| AppError::Database(format!("读取导入对象失败: {e}")))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| AppError::Database(format!("查询导入对象失败: {e}")))?;

        while let Some(row) = rows.next().map_err(|e| AppError::Database(e.to_string()))? {
            let object_type: String = row.get(0).map_err(|e| AppError::Database(e.to_string()))?;
            let name: String = row.get(1).map_err(|e| AppError::Database(e.to_string()))?;
            let table: String = row.get(2).map_err(|e| AppError::Database(e.to_string()))?;

            let allowed = match object_type.as_str() {
                "table" => {
                    let current = SQL_RESTORE_TABLES.contains(&name.as_str());
                    let legacy = version <= 2 && LEGACY_SQL_RESTORE_TABLES.contains(&name.as_str());
                    current || legacy
                }
                "index" => Self::restore_index_table(&name) == Some(table.as_str()),
                _ => false,
            };
            if !allowed {
                return Err(AppError::Database(format!(
                    "SQL 备份包含未授权对象: {object_type} {name}"
                )));
            }

            has_providers |= object_type == "table" && name == "providers";
            has_mcp_servers |= object_type == "table" && name == "mcp_servers";
        }

        if !has_providers || !has_mcp_servers {
            return Err(AppError::Database(
                "SQL 备份缺少 CC Switch 核心表".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_current_schema(
        conn: &Connection,
        allow_normalization: bool,
    ) -> Result<bool, AppError> {
        let expected = Connection::open_in_memory()
            .map_err(|e| AppError::Database(format!("创建 schema 校验库失败: {e}")))?;
        Self::create_tables_on_conn(&expected)?;
        Self::apply_schema_migrations_on_conn(&expected)?;

        let expected_objects = Self::schema_objects(&expected)?;
        let actual_objects = Self::schema_objects(conn)?;
        if actual_objects != expected_objects {
            return Err(AppError::Database(
                "SQL 备份的数据库对象与当前 CC Switch schema 不一致".to_string(),
            ));
        }

        let mut canonical = true;
        for table in SQL_RESTORE_TABLES {
            if Self::table_xinfo_signature(conn, table)?
                != Self::table_xinfo_signature(&expected, table)?
            {
                if !allow_normalization || !Self::legacy_columns_compatible(conn, &expected, table)?
                {
                    return Err(AppError::Database(format!(
                        "SQL 备份的表结构不匹配: {table}"
                    )));
                }
                canonical = false;
            }
            if Self::foreign_key_signature(conn, table)?
                != Self::foreign_key_signature(&expected, table)?
                || Self::table_flags(conn, table)? != Self::table_flags(&expected, table)?
                || Self::table_index_signature(conn, table)?
                    != Self::table_index_signature(&expected, table)?
                || Self::table_check_signature(conn, table)?
                    != Self::table_check_signature(&expected, table)?
                || Self::table_has_keyword(conn, table, "autoincrement")?
                    != Self::table_has_keyword(&expected, table, "autoincrement")?
            {
                return Err(AppError::Database(format!(
                    "SQL 备份的表结构不匹配: {table}"
                )));
            }
        }
        Self::validate_expression_index_semantics(conn)?;

        let version = Self::get_user_version(conn)?;
        if version != super::SCHEMA_VERSION {
            return Err(AppError::Database(format!(
                "SQL 备份迁移后的 schema 版本无效: {version}"
            )));
        }

        Ok(canonical)
    }

    fn schema_objects(conn: &Connection) -> Result<BTreeMap<String, SchemaObject>, AppError> {
        let mut stmt = conn
            .prepare(
                "SELECT type, name, tbl_name
                 FROM sqlite_master
                 WHERE name NOT LIKE 'sqlite_%'
                 ORDER BY name",
            )
            .map_err(|e| AppError::Database(format!("读取 schema 对象失败: {e}")))?;
        let rows = stmt
            .query_map([], |row| {
                let object_type = row.get::<_, String>(0)?;
                let name = row.get::<_, String>(1)?;
                let table = row.get::<_, String>(2)?;
                Ok((name, (object_type, table)))
            })
            .map_err(|e| AppError::Database(format!("查询 schema 对象失败: {e}")))?;

        let mut objects = BTreeMap::new();
        for row in rows {
            let (name, object) = row.map_err(|e| AppError::Database(e.to_string()))?;
            objects.insert(name, object);
        }
        Ok(objects)
    }

    fn table_xinfo_signature(
        conn: &Connection,
        table: &str,
    ) -> Result<Vec<TableColumnSignature>, AppError> {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_xinfo(\"{table}\")"))
            .map_err(|e| AppError::Database(format!("读取表 {table} 结构失败: {e}")))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            })
            .map_err(|e| AppError::Database(format!("查询表 {table} 结构失败: {e}")))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(format!("解析表 {table} 结构失败: {e}")))
    }

    fn legacy_columns_compatible(
        conn: &Connection,
        expected: &Connection,
        table: &str,
    ) -> Result<bool, AppError> {
        let actual = Self::table_xinfo_signature(conn, table)?
            .into_iter()
            .map(|(_, name, column_type, not_null, default, pk, hidden)| {
                (name, (column_type, not_null, default, pk, hidden))
            })
            .collect::<BTreeMap<_, _>>();
        let expected = Self::table_xinfo_signature(expected, table)?
            .into_iter()
            .map(|(_, name, column_type, not_null, default, pk, hidden)| {
                (name, (column_type, not_null, default, pk, hidden))
            })
            .collect::<BTreeMap<_, _>>();

        for (name, signature) in &actual {
            if expected.get(name) != Some(signature) {
                return Ok(false);
            }
        }
        for (name, (_, not_null, default, _, _)) in &expected {
            if !actual.contains_key(name) && *not_null != 0 && default.is_none() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn foreign_key_signature(
        conn: &Connection,
        table: &str,
    ) -> Result<Vec<ForeignKeySignature>, AppError> {
        let mut stmt = conn
            .prepare(&format!("PRAGMA foreign_key_list(\"{table}\")"))
            .map_err(|e| AppError::Database(format!("读取表 {table} 外键失败: {e}")))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            })
            .map_err(|e| AppError::Database(format!("查询表 {table} 外键失败: {e}")))?;

        let mut signature = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(format!("解析表 {table} 外键失败: {e}")))?;
        signature.sort();
        Ok(signature)
    }

    fn table_flags(conn: &Connection, table: &str) -> Result<(i64, i64), AppError> {
        conn.query_row(
            "SELECT wr, strict FROM pragma_table_list WHERE schema = 'main' AND name = ?1",
            [table],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| AppError::Database(format!("读取表 {table} 标志失败: {e}")))
    }

    fn table_index_signature(
        conn: &Connection,
        table: &str,
    ) -> Result<Vec<TableIndexSignature>, AppError> {
        let table = table.replace('"', "\"\"");
        let mut stmt = conn
            .prepare(&format!("PRAGMA index_list(\"{table}\")"))
            .map_err(|e| AppError::Database(format!("读取表 {table} 索引失败: {e}")))?;
        let indexes = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })
            .map_err(|e| AppError::Database(format!("查询表 {table} 索引失败: {e}")))?;

        let mut signature = Vec::new();
        for index in indexes {
            let (name, unique, origin, partial) =
                index.map_err(|e| AppError::Database(e.to_string()))?;
            let escaped_name = name.replace('"', "\"\"");
            let mut columns_stmt = conn
                .prepare(&format!("PRAGMA index_xinfo(\"{escaped_name}\")"))
                .map_err(|e| AppError::Database(format!("读取索引 {name} 结构失败: {e}")))?;
            let columns = columns_stmt
                .query_map([], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                })
                .map_err(|e| AppError::Database(format!("查询索引 {name} 结构失败: {e}")))?
                .collect::<Result<Vec<IndexColumnSignature>, _>>()
                .map_err(|e| AppError::Database(format!("解析索引 {name} 结构失败: {e}")))?;
            signature.push((name, unique, origin, partial, columns));
        }
        signature.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(signature)
    }

    fn validate_expression_index_semantics(conn: &Connection) -> Result<(), AppError> {
        let mut stmt = conn
            .prepare(
                "EXPLAIN QUERY PLAN
                 SELECT request_id FROM proxy_request_logs
                 WHERE app_type = ?1
                   AND COALESCE(data_source, 'proxy') = ?2
                   AND input_tokens = ?3
                   AND output_tokens = ?4
                   AND cache_read_tokens = ?5
                   AND created_at = ?6
                   AND cache_creation_tokens = ?7",
            )
            .map_err(|e| AppError::Database(format!("准备表达式索引校验失败: {e}")))?;
        let plans = stmt
            .query_map(rusqlite::params!["claude", "proxy", 0, 0, 0, 0, 0], |row| {
                row.get::<_, String>(3)
            })
            .map_err(|e| AppError::Database(format!("执行表达式索引校验失败: {e}")))?;
        for plan in plans {
            let detail = plan.map_err(|e| AppError::Database(e.to_string()))?;
            if detail.contains("idx_request_logs_dedup_lookup_expr") && detail.contains("<expr>=?")
            {
                return Ok(());
            }
        }
        Err(AppError::Database(
            "表达式索引 idx_request_logs_dedup_lookup_expr 语义不匹配".to_string(),
        ))
    }

    fn table_check_signature(conn: &Connection, table: &str) -> Result<Vec<String>, AppError> {
        let sql = Self::table_sql(conn, table)?;
        Ok(Self::extract_check_expressions(&sql))
    }

    fn extract_check_expressions(sql: &str) -> Vec<String> {
        let chars = Self::strip_sql_comments(sql).chars().collect::<Vec<_>>();
        let mut expressions = Vec::new();
        let mut index = 0;

        while index < chars.len() {
            match chars[index] {
                '\'' | '"' | '`' | '[' => {
                    index = Self::skip_quoted_sql(&chars, index);
                    continue;
                }
                ch if ch.is_ascii_alphabetic() || ch == '_' => {
                    let start = index;
                    index += 1;
                    while chars
                        .get(index)
                        .is_some_and(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '$')
                    {
                        index += 1;
                    }
                    if !chars[start..index]
                        .iter()
                        .collect::<String>()
                        .eq_ignore_ascii_case("check")
                    {
                        continue;
                    }
                }
                _ => {
                    index += 1;
                    continue;
                }
            }

            {
                let mut cursor = index;
                while chars.get(cursor).is_some_and(|ch| ch.is_whitespace()) {
                    cursor += 1;
                }
                if chars.get(cursor) == Some(&'(') {
                    let start = cursor + 1;
                    let mut depth = 1;
                    cursor += 1;
                    while cursor < chars.len() && depth > 0 {
                        match chars[cursor] {
                            '\'' | '"' | '`' | '[' => {
                                cursor = Self::skip_quoted_sql(&chars, cursor);
                                continue;
                            }
                            '(' => depth += 1,
                            ')' => depth -= 1,
                            _ => {}
                        }
                        cursor += 1;
                    }
                    if depth == 0 {
                        let expression =
                            Self::normalize_check_expression(&chars[start..cursor - 1]);
                        expressions.push(expression);
                        index = cursor;
                        continue;
                    }
                }
            }
            index += 1;
        }

        expressions.sort();
        expressions
    }

    fn table_sql(conn: &Connection, table: &str) -> Result<String, AppError> {
        conn.query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table],
            |row| row.get(0),
        )
        .map_err(|e| AppError::Database(format!("读取表 {table} SQL 失败: {e}")))
    }

    fn table_has_keyword(conn: &Connection, table: &str, keyword: &str) -> Result<bool, AppError> {
        let sql = Self::strip_sql_comments(&Self::table_sql(conn, table)?);
        let chars = sql.chars().collect::<Vec<_>>();
        let mut index = 0;
        while index < chars.len() {
            match chars[index] {
                '\'' | '"' | '`' | '[' => index = Self::skip_quoted_sql(&chars, index),
                ch if ch.is_ascii_alphabetic() || ch == '_' => {
                    let start = index;
                    index += 1;
                    while chars
                        .get(index)
                        .is_some_and(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '$')
                    {
                        index += 1;
                    }
                    if chars[start..index]
                        .iter()
                        .collect::<String>()
                        .eq_ignore_ascii_case(keyword)
                    {
                        return Ok(true);
                    }
                }
                _ => index += 1,
            }
        }
        Ok(false)
    }

    fn strip_sql_comments(sql: &str) -> String {
        let chars = sql.chars().collect::<Vec<_>>();
        let mut output = String::with_capacity(sql.len());
        let mut index = 0;
        while index < chars.len() {
            match chars[index] {
                '\'' | '"' | '`' | '[' => {
                    let end = Self::skip_quoted_sql(&chars, index);
                    output.extend(chars[index..end].iter());
                    index = end;
                }
                '-' if chars.get(index + 1) == Some(&'-') => {
                    output.push(' ');
                    index += 2;
                    while index < chars.len() && chars[index] != '\n' {
                        index += 1;
                    }
                }
                '/' if chars.get(index + 1) == Some(&'*') => {
                    output.push(' ');
                    index += 2;
                    while index + 1 < chars.len()
                        && !(chars[index] == '*' && chars[index + 1] == '/')
                    {
                        index += 1;
                    }
                    index = (index + 2).min(chars.len());
                }
                ch => {
                    output.push(ch);
                    index += 1;
                }
            }
        }
        output
    }

    fn skip_quoted_sql(chars: &[char], start: usize) -> usize {
        let opener = chars[start];
        let closer = if opener == '[' { ']' } else { opener };
        let mut index = start + 1;
        while index < chars.len() {
            if chars[index] == closer {
                if opener != '[' && chars.get(index + 1) == Some(&closer) {
                    index += 2;
                    continue;
                }
                return index + 1;
            }
            index += 1;
        }
        chars.len()
    }

    fn normalize_check_expression(chars: &[char]) -> String {
        let mut output = String::new();
        let mut index = 0;
        while index < chars.len() {
            match chars[index] {
                '\'' => {
                    let end = Self::skip_quoted_sql(chars, index);
                    output.extend(chars[index..end].iter());
                    index = end;
                }
                ch if ch.is_whitespace() => index += 1,
                ch => {
                    output.extend(ch.to_lowercase());
                    index += 1;
                }
            }
        }
        output
    }

    fn validate_database_integrity(conn: &Connection) -> Result<(), AppError> {
        let mut integrity_stmt = conn
            .prepare("PRAGMA integrity_check")
            .map_err(|e| AppError::Database(format!("执行 integrity_check 失败: {e}")))?;
        let integrity_rows = integrity_stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| AppError::Database(format!("查询 integrity_check 失败: {e}")))?;
        let integrity = integrity_rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(format!("解析 integrity_check 失败: {e}")))?;
        if integrity.as_slice() != ["ok"] {
            return Err(AppError::Database(format!(
                "SQL 备份未通过 integrity_check: {}",
                integrity.join("; ")
            )));
        }

        let mut stmt = conn
            .prepare("PRAGMA foreign_key_check")
            .map_err(|e| AppError::Database(format!("执行 foreign_key_check 失败: {e}")))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| AppError::Database(format!("查询 foreign_key_check 失败: {e}")))?;
        if rows
            .next()
            .map_err(|e| AppError::Database(e.to_string()))?
            .is_some()
        {
            return Err(AppError::Database("SQL 备份包含外键完整性错误".to_string()));
        }

        Ok(())
    }

    /// 创建内存快照以避免长时间持有数据库锁
    pub(crate) fn snapshot_to_memory(&self) -> Result<Connection, AppError> {
        let conn = lock_conn!(self.conn);
        let mut snapshot =
            Connection::open_in_memory().map_err(|e| AppError::Database(e.to_string()))?;

        {
            let backup =
                Backup::new(&conn, &mut snapshot).map_err(|e| AppError::Database(e.to_string()))?;
            backup
                .step(-1)
                .map_err(|e| AppError::Database(e.to_string()))?;
        }

        Ok(snapshot)
    }

    fn validate_cc_switch_sql_export(sql: &str) -> Result<(), AppError> {
        let trimmed = sql.trim_start();
        if trimmed.starts_with(CC_SWITCH_SQL_EXPORT_HEADER) {
            return Ok(());
        }

        Err(AppError::localized(
            "backup.sql.invalid_format",
            "仅支持导入由 CC Switch 导出的 SQL 备份文件。",
            "Only SQL backups exported by CC Switch are supported.",
        ))
    }

    fn restore_tables(
        source_conn: &Connection,
        target_conn: &Connection,
        tables: &[&str],
    ) -> Result<(), AppError> {
        // 整批复原放进一个事务：旧实现每行一条隐式自动提交的 INSERT，
        // 目标是磁盘上的暂存库，等于每行一次 fsync——2.6 万行实测 119 秒。
        // 合并成单事务后只剩最后一次提交；中途失败整体回滚，
        // 也不会留下“半张表”的中间状态。
        let tx = target_conn
            .unchecked_transaction()
            .map_err(|e| AppError::Database(format!("开启恢复事务失败: {e}")))?;

        for table in tables {
            if !Self::table_exists(source_conn, table)? || !Self::table_exists(&tx, table)? {
                continue;
            }

            let columns = Self::get_table_columns(source_conn, table)?;
            if columns.is_empty() {
                continue;
            }

            let quoted_table = Self::quote_identifier(table);
            let quoted_columns = columns
                .iter()
                .map(|column| Self::quote_identifier(column))
                .collect::<Vec<_>>()
                .join(", ");

            tx.execute(&format!("DELETE FROM {quoted_table}"), [])
                .map_err(|e| AppError::Database(format!("清空表 {table} 失败: {e}")))?;

            let placeholders = (1..=columns.len())
                .map(|idx| format!("?{idx}"))
                .collect::<Vec<_>>()
                .join(", ");
            let insert_sql =
                format!("INSERT INTO {quoted_table} ({quoted_columns}) VALUES ({placeholders})");

            // INSERT 语句每表只 prepare 一次，不再逐行重复解析。
            let mut insert_stmt = tx
                .prepare(&insert_sql)
                .map_err(|e| AppError::Database(format!("准备表 {table} 插入语句失败: {e}")))?;

            let mut stmt = source_conn
                .prepare(&format!("SELECT {quoted_columns} FROM {quoted_table}"))
                .map_err(|e| AppError::Database(format!("读取表 {table} 失败: {e}")))?;
            let mut rows = stmt
                .query([])
                .map_err(|e| AppError::Database(format!("查询表 {table} 数据失败: {e}")))?;

            while let Some(row) = rows.next().map_err(|e| AppError::Database(e.to_string()))? {
                let mut values = Vec::with_capacity(columns.len());
                for idx in 0..columns.len() {
                    values.push(
                        row.get::<_, rusqlite::types::Value>(idx)
                            .map_err(|e| AppError::Database(e.to_string()))?,
                    );
                }

                insert_stmt
                    .execute(rusqlite::params_from_iter(values.iter()))
                    .map_err(|e| AppError::Database(format!("恢复表 {table} 数据失败: {e}")))?;
            }
        }

        tx.commit()
            .map_err(|e| AppError::Database(format!("提交恢复事务失败: {e}")))?;
        Ok(())
    }

    /// Periodic backup: create a new backup if the latest one is older than the configured interval
    pub(crate) fn periodic_backup_if_needed(&self) -> Result<(), AppError> {
        let interval_hours = crate::settings::effective_backup_interval_hours();
        if interval_hours > 0 {
            let backup_dir = get_app_config_dir().join("backups");
            if !backup_dir.exists() {
                self.backup_database_file()?;
            } else {
                let latest = fs::read_dir(&backup_dir).ok().and_then(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().extension().map(|ext| ext == "db").unwrap_or(false))
                        .filter_map(|e| e.metadata().ok().and_then(|m| m.modified().ok()))
                        .max()
                });

                let interval_secs = u64::from(interval_hours) * 3600;
                let needs_backup = match latest {
                    None => true,
                    Some(last_modified) => {
                        last_modified.elapsed().unwrap_or_default()
                            > std::time::Duration::from_secs(interval_secs)
                    }
                };

                if needs_backup {
                    log::info!(
                        "Periodic backup: latest backup is older than {interval_hours} hours, creating new backup"
                    );
                    self.backup_database_file()?;
                }
            }
        }

        // Periodic maintenance is always enabled, regardless of auto-backup settings.
        let mut reclaimed_rows = 0u64;
        match self.cleanup_old_stream_check_logs(7) {
            Ok(deleted) => {
                reclaimed_rows += deleted;
            }
            Err(e) => {
                log::warn!("Periodic stream_check_logs cleanup failed: {e}");
            }
        }
        match self.rollup_and_prune(30) {
            Ok(deleted) => {
                reclaimed_rows += deleted;
            }
            Err(e) => {
                log::warn!("Periodic rollup_and_prune failed: {e}");
            }
        }
        if reclaimed_rows > 0 {
            let conn = lock_conn!(self.conn);
            if let Err(e) = conn.execute_batch("PRAGMA incremental_vacuum;") {
                log::warn!("Periodic incremental vacuum failed: {e}");
            }
        }

        Ok(())
    }

    /// 生成一致性快照备份，返回备份文件路径（不存在主库时返回 None）
    pub(crate) fn backup_database_file(&self) -> Result<Option<PathBuf>, AppError> {
        let backup_path = {
            let conn = lock_conn!(self.conn);
            Self::backup_database_file_from_conn(&conn)?
        };
        if let Some(path) = backup_path.as_ref() {
            if let Some(dir) = path.parent() {
                Self::cleanup_db_backups(dir)?;
            }
        }
        Ok(backup_path)
    }

    fn replace_main_with_safety_backup(
        &self,
        source_conn: &Connection,
    ) -> Result<Option<PathBuf>, AppError> {
        let mut main_conn = lock_conn!(self.conn);
        let backup_path = Self::backup_database_file_from_conn(&main_conn)?;
        if let Some(path) = backup_path.as_ref() {
            if let Some(dir) = path.parent() {
                Self::cleanup_db_backups(dir)?;
            }
        }
        #[cfg(test)]
        if let Some(hook) = {
            let database_key = std::ptr::from_ref(&self.conn) as usize;
            let mut slot = RESTORE_CRITICAL_SECTION_HOOK
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if slot
                .as_ref()
                .is_some_and(|(target_key, _)| *target_key == database_key)
            {
                slot.take().map(|(_, hook)| hook)
            } else {
                None
            }
        } {
            hook();
        }
        let backup = Backup::new(source_conn, &mut main_conn)
            .map_err(|e| AppError::Database(e.to_string()))?;
        backup
            .step(-1)
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(backup_path)
    }

    fn backup_database_file_from_conn(conn: &Connection) -> Result<Option<PathBuf>, AppError> {
        let db_path = get_app_config_dir().join("cc-switch.db");
        if !db_path.exists() {
            return Ok(None);
        }

        let backup_dir = db_path
            .parent()
            .ok_or_else(|| AppError::Config("无效的数据库路径".to_string()))?
            .join("backups");

        fs::create_dir_all(&backup_dir).map_err(|e| AppError::io(&backup_dir, e))?;

        let base_id = format!("db_backup_{}", Local::now().format("%Y%m%d_%H%M%S"));
        let mut backup_id = base_id.clone();
        let mut backup_path = backup_dir.join(format!("{backup_id}.db"));
        let mut counter = 1;
        while backup_path.exists() {
            backup_id = format!("{base_id}_{counter}");
            backup_path = backup_dir.join(format!("{backup_id}.db"));
            counter += 1;
        }

        let mut dest_conn =
            Connection::open(&backup_path).map_err(|e| AppError::Database(e.to_string()))?;
        let backup =
            Backup::new(conn, &mut dest_conn).map_err(|e| AppError::Database(e.to_string()))?;
        backup
            .step(-1)
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(Some(backup_path))
    }

    /// 清理旧的数据库备份，保留最新的 N 个
    fn cleanup_db_backups(dir: &Path) -> Result<(), AppError> {
        let retain = crate::settings::effective_backup_retain_count();
        let entries = match fs::read_dir(dir) {
            Ok(iter) => iter
                .filter_map(|entry| entry.ok())
                .filter(|entry| {
                    entry
                        .path()
                        .extension()
                        .map(|ext| ext == "db")
                        .unwrap_or(false)
                })
                .collect::<Vec<_>>(),
            Err(_) => return Ok(()),
        };

        if entries.len() <= retain {
            return Ok(());
        }

        let remove_count = entries.len().saturating_sub(retain);
        let mut sorted = entries;
        sorted.sort_by_key(|entry| entry.metadata().and_then(|m| m.modified()).ok());

        for entry in sorted.into_iter().take(remove_count) {
            if let Err(err) = fs::remove_file(entry.path()) {
                log::warn!("删除旧数据库备份失败 {}: {}", entry.path().display(), err);
            }
        }
        Ok(())
    }

    /// 基础状态校验
    fn validate_basic_state(conn: &Connection) -> Result<(), AppError> {
        let _provider_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM providers", [], |row| row.get(0))
            .map_err(|e| AppError::Database(e.to_string()))?;
        let _mcp_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM mcp_servers", [], |row| row.get(0))
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// 导出数据库为 SQL 文本
    pub(super) fn dump_sql(conn: &Connection, skip_tables: &[&str]) -> Result<String, AppError> {
        let mut output = String::new();
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let user_version: i64 = conn
            .query_row("PRAGMA user_version;", [], |row| row.get(0))
            .unwrap_or(0);

        output.push_str(&format!(
            "-- CC Switch SQLite 导出\n-- 生成时间: {timestamp}\n-- user_version: {user_version}\n"
        ));
        output.push_str("PRAGMA foreign_keys=OFF;\n");
        output.push_str(&format!("PRAGMA user_version={user_version};\n"));
        output.push_str("BEGIN TRANSACTION;\n");

        // 导出 schema
        let mut stmt = conn
            .prepare(
                "SELECT type, name, tbl_name, sql
                 FROM sqlite_master
                 WHERE sql NOT NULL AND type IN ('table','index','trigger','view')
                 ORDER BY type='table' DESC, name",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut tables = Vec::new();
        let mut triggers = Vec::new();
        let mut rows = stmt
            .query([])
            .map_err(|e| AppError::Database(e.to_string()))?;
        while let Some(row) = rows.next().map_err(|e| AppError::Database(e.to_string()))? {
            let obj_type: String = row.get(0).map_err(|e| AppError::Database(e.to_string()))?;
            let name: String = row.get(1).map_err(|e| AppError::Database(e.to_string()))?;
            let sql: String = row.get(3).map_err(|e| AppError::Database(e.to_string()))?;

            // 跳过 SQLite 内部对象（如 sqlite_sequence）
            if name.starts_with("sqlite_") {
                continue;
            }

            if obj_type == "trigger" {
                triggers.push(sql);
                continue;
            }

            output.push_str(&sql);
            output.push_str(";\n");
            if obj_type == "table" {
                tables.push(name);
            }
        }

        // 导出数据
        for table in tables {
            if skip_tables.iter().any(|t| *t == table) {
                continue;
            }
            let columns = Self::get_table_columns(conn, &table)?;
            if columns.is_empty() {
                continue;
            }

            // 每行一条 INSERT 是导入慢的根源：恢复侧要为每条语句单独
            // 解析/准备/收尾，2 万行实测 21 秒（内存库上一样慢，说明是
            // 纯 CPU 而非 I/O）。合并成多行 VALUES 后同样数据 <100ms。
            // SQLite 从 3.7.11（2012）起支持多行 VALUES，且导入侧是通用
            // execute_batch，新旧两种格式都能读——向后兼容无忧。
            let quoted_table = Self::quote_identifier(&table);
            let quoted_columns = columns
                .iter()
                .map(|column| Self::quote_identifier(column))
                .collect::<Vec<_>>()
                .join(", ");
            let insert_prefix = format!("INSERT INTO {quoted_table} ({quoted_columns}) VALUES ");

            let mut stmt = conn
                .prepare(&format!("SELECT {quoted_columns} FROM {quoted_table}"))
                .map_err(|e| AppError::Database(e.to_string()))?;
            let mut rows = stmt
                .query([])
                .map_err(|e| AppError::Database(e.to_string()))?;

            let mut pending_rows = 0usize;
            let mut batch = String::new();
            while let Some(row) = rows.next().map_err(|e| AppError::Database(e.to_string()))? {
                let mut values = Vec::with_capacity(columns.len());
                for idx in 0..columns.len() {
                    let value = row
                        .get_ref(idx)
                        .map_err(|e| AppError::Database(e.to_string()))?;
                    values.push(Self::format_sql_value(value)?);
                }

                let row_sql = format!("({})", values.join(", "));
                let separator_bytes = usize::from(pending_rows > 0);
                if pending_rows > 0
                    && batch.len() + separator_bytes + row_sql.len() + 2 > INSERT_BATCH_MAX_BYTES
                {
                    batch.push_str(";\n");
                    output.push_str(&batch);
                    pending_rows = 0;
                }

                if pending_rows == 0 {
                    batch.clear();
                    batch.push_str(&insert_prefix);
                } else {
                    batch.push(',');
                }
                batch.push_str(&row_sql);
                pending_rows += 1;

                if pending_rows >= INSERT_BATCH_MAX_ROWS {
                    batch.push_str(";\n");
                    output.push_str(&batch);
                    pending_rows = 0;
                }
            }
            if pending_rows > 0 {
                batch.push_str(";\n");
                output.push_str(&batch);
            }
        }

        // Triggers must be created after loading table data so they cannot
        // change dump rows or abandon the remainder of a multi-row INSERT.
        for sql in triggers {
            output.push_str(&sql);
            output.push_str(";\n");
        }

        output.push_str("COMMIT;\nPRAGMA foreign_keys=ON;\n");
        Ok(output)
    }

    fn quote_identifier(identifier: &str) -> String {
        format!("\"{}\"", identifier.replace('"', "\"\""))
    }

    /// 获取表的列名列表
    fn get_table_columns(conn: &Connection, table: &str) -> Result<Vec<String>, AppError> {
        let quoted_table = Self::quote_identifier(table);
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({quoted_table})"))
            .map_err(|e| AppError::Database(e.to_string()))?;
        let iter = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut columns = Vec::new();
        for col in iter {
            columns.push(col.map_err(|e| AppError::Database(e.to_string()))?);
        }
        Ok(columns)
    }

    /// 格式化 SQL 值
    fn format_sql_value(value: ValueRef<'_>) -> Result<String, AppError> {
        match value {
            ValueRef::Null => Ok("NULL".to_string()),
            ValueRef::Integer(i) => Ok(i.to_string()),
            ValueRef::Real(f) => Ok(f.to_string()),
            ValueRef::Text(t) => {
                let text = std::str::from_utf8(t)
                    .map_err(|e| AppError::Database(format!("文本字段不是有效的 UTF-8: {e}")))?;
                let escaped = text.replace('\'', "''");
                Ok(format!("'{escaped}'"))
            }
            ValueRef::Blob(bytes) => {
                let mut s = String::from("X'");
                for b in bytes {
                    use std::fmt::Write;
                    let _ = write!(&mut s, "{b:02X}");
                }
                s.push('\'');
                Ok(s)
            }
        }
    }

    /// List all database backup files, sorted by creation time (newest first)
    pub fn list_backups() -> Result<Vec<BackupEntry>, AppError> {
        let backup_dir = get_app_config_dir().join("backups");
        if !backup_dir.exists() {
            return Ok(vec![]);
        }

        let mut entries: Vec<BackupEntry> = fs::read_dir(&backup_dir)
            .map_err(|e| AppError::io(&backup_dir, e))?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ext| ext == "db").unwrap_or(false))
            .filter_map(|e| {
                let metadata = e.metadata().ok()?;
                let filename = e.file_name().to_string_lossy().to_string();
                let size_bytes = metadata.len();
                let created_at = metadata
                    .modified()
                    .ok()
                    .map(|t| {
                        let dt: chrono::DateTime<Utc> = t.into();
                        dt.to_rfc3339()
                    })
                    .unwrap_or_default();
                Some(BackupEntry {
                    filename,
                    size_bytes,
                    created_at,
                })
            })
            .collect();

        // Sort by created_at descending (newest first)
        entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(entries)
    }

    /// Restore database from a backup file. Returns the safety backup ID.
    pub fn restore_from_backup(&self, filename: &str) -> Result<String, AppError> {
        // Security: validate filename to prevent path traversal
        if filename.contains("..")
            || filename.contains('/')
            || filename.contains('\\')
            || !filename.ends_with(".db")
        {
            return Err(AppError::InvalidInput(
                "Invalid backup filename".to_string(),
            ));
        }

        let backup_dir = get_app_config_dir().join("backups");
        let backup_path = backup_dir.join(filename);

        if !backup_path.exists() {
            return Err(AppError::InvalidInput(format!(
                "Backup file not found: {filename}"
            )));
        }

        // Open the backup file before entering the main-connection critical section.
        let source_conn =
            Connection::open(&backup_path).map_err(|e| AppError::Database(e.to_string()))?;

        let safety_backup = self.replace_main_with_safety_backup(&source_conn)?;
        let safety_id = safety_backup
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
            .unwrap_or_default();

        // Run schema migrations (backup may be from an older version)
        self.create_tables()?;
        self.apply_schema_migrations()?;
        self.ensure_model_pricing_seeded()?;

        log::info!("Database restored from backup: {filename}, safety backup: {safety_id}");
        Ok(safety_id)
    }

    /// Rename a backup file. Returns the new filename.
    pub fn rename_backup(old_filename: &str, new_name: &str) -> Result<String, AppError> {
        // Validate old filename (path traversal + .db suffix)
        if old_filename.contains("..")
            || old_filename.contains('/')
            || old_filename.contains('\\')
            || !old_filename.ends_with(".db")
        {
            return Err(AppError::InvalidInput(
                "Invalid backup filename".to_string(),
            ));
        }

        // Clean new name
        let trimmed = new_name.trim();
        if trimmed.is_empty() {
            return Err(AppError::InvalidInput(
                "New name cannot be empty".to_string(),
            ));
        }

        // Length limit (without .db suffix)
        let name_part = trimmed.strip_suffix(".db").unwrap_or(trimmed);
        if name_part.len() > 100 {
            return Err(AppError::InvalidInput(
                "Name too long (max 100 characters)".to_string(),
            ));
        }

        // Prevent path traversal in new name
        if name_part.contains("..")
            || name_part.contains('/')
            || name_part.contains('\\')
            || name_part.contains('\0')
        {
            return Err(AppError::InvalidInput(
                "Invalid characters in new name".to_string(),
            ));
        }

        let new_filename = format!("{name_part}.db");

        let backup_dir = get_app_config_dir().join("backups");
        let old_path = backup_dir.join(old_filename);
        let new_path = backup_dir.join(&new_filename);

        if !old_path.exists() {
            return Err(AppError::InvalidInput(format!(
                "Backup file not found: {old_filename}"
            )));
        }

        if new_path.exists() {
            return Err(AppError::InvalidInput(format!(
                "A backup named '{new_filename}' already exists"
            )));
        }

        fs::rename(&old_path, &new_path).map_err(|e| AppError::io(&old_path, e))?;
        log::info!("Renamed backup: {old_filename} -> {new_filename}");
        Ok(new_filename)
    }

    /// Delete a backup file permanently.
    pub fn delete_backup(filename: &str) -> Result<(), AppError> {
        // Validate filename (path traversal + .db suffix)
        if filename.contains("..")
            || filename.contains('/')
            || filename.contains('\\')
            || !filename.ends_with(".db")
        {
            return Err(AppError::InvalidInput(
                "Invalid backup filename".to_string(),
            ));
        }

        let backup_path = get_app_config_dir().join("backups").join(filename);
        if !backup_path.exists() {
            return Err(AppError::InvalidInput(format!(
                "Backup file not found: {filename}"
            )));
        }

        fs::remove_file(&backup_path).map_err(|e| AppError::io(&backup_path, e))?;
        log::info!("Deleted backup: {filename}");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Database, RESTORE_CRITICAL_SECTION_HOOK};
    use crate::error::AppError;
    use crate::settings::{update_settings, AppSettings};
    use rusqlite::Connection;
    use serial_test::serial;
    use std::sync::{mpsc, Arc};
    use std::time::Duration;

    struct TestHomeGuard {
        previous_test_home: Option<std::ffi::OsString>,
        temp_dir: tempfile::TempDir,
    }

    impl TestHomeGuard {
        fn new() -> Self {
            let temp_dir = tempfile::tempdir().expect("create isolated test home");
            let previous_test_home = std::env::var_os("CC_SWITCH_TEST_HOME");
            std::env::set_var("CC_SWITCH_TEST_HOME", temp_dir.path());
            // Prevent the Windows legacy-HOME fallback without mutating HOME:
            // an existing default DB keeps get_app_config_dir() anchored under
            // CC_SWITCH_TEST_HOME and makes import exercise its safety backup.
            let config_dir = temp_dir.path().join(".cc-switch");
            std::fs::create_dir_all(&config_dir).expect("create isolated config directory");
            std::fs::File::create(config_dir.join("cc-switch.db"))
                .expect("create isolated database sentinel");
            let guard = Self {
                previous_test_home,
                temp_dir,
            };
            let resolved = crate::config::get_app_config_dir();
            assert!(
                resolved.starts_with(guard.temp_dir.path()),
                "isolated test home resolved outside its temp directory: {}",
                resolved.display()
            );
            guard
        }

        fn path(&self) -> &std::path::Path {
            self.temp_dir.path()
        }
    }

    impl Drop for TestHomeGuard {
        fn drop(&mut self) {
            match self.previous_test_home.as_ref() {
                Some(previous) => std::env::set_var("CC_SWITCH_TEST_HOME", previous),
                None => std::env::remove_var("CC_SWITCH_TEST_HOME"),
            }
        }
    }

    #[test]
    #[serial]
    fn import_rejects_cross_file_statements_and_leaves_no_file_behind() -> Result<(), AppError> {
        let test_home = TestHomeGuard::new();
        // `VACUUM INTO` 是关键字扫描方案最容易漏的一条：它不含 "ATTACH" 字样，
        // 却和 ATTACH 一样落到 `AuthAction::Attach`（实测），因此同一条规则挡住两者。
        let cases: [(&str, &str); 2] = [
            ("attach", "ATTACH DATABASE '{path}' AS evil;"),
            ("vacuum-into", "VACUUM INTO '{path}';"),
        ];

        for (label, template) in cases {
            let target = test_home
                .path()
                .join(format!("cc-switch-authorizer-{label}.sqlite"));

            // 合法的导出头 + 越界语句。头部校验只比前缀，这份输入过得了它，
            // 真正拦下来的必须是 authorizer。
            let malicious = format!(
                "{}\n{}\n",
                super::CC_SWITCH_SQL_EXPORT_HEADER,
                template.replace("{path}", &target.to_string_lossy().replace('\'', "''"))
            );

            let db = Database::memory()?;
            let result = db.import_sql_string(&malicious);

            let error = result.expect_err("越界 SQL 必须被拒绝");
            assert!(
                error.to_string().to_ascii_lowercase().contains("authoriz"),
                "{label} 必须由 authorizer 拒绝，实际错误: {error}"
            );
            // 光报错不够：文件创建发生在 prepare 之后、`validate_basic_state` 之前，
            // 守卫若失效，即便导入整体失败，文件也已经躺在磁盘上了。
            assert!(
                !target.exists(),
                "被拒绝的 {label} 不得在磁盘上留下文件: {}",
                target.display()
            );
        }
        Ok(())
    }

    #[test]
    #[serial]
    fn import_still_accepts_a_genuine_export() -> Result<(), AppError> {
        let _test_home = TestHomeGuard::new();
        // 白名单收得紧，必须有一条回归防线证明它没误伤自家导出格式——
        // 这条测试红了就说明 dump_sql 写出了白名单没覆盖的语句。
        let source = Database::memory()?;
        {
            let conn = crate::database::lock_conn!(source.conn);
            conn.execute(
                "INSERT INTO providers (id, app_type, name, settings_config, meta)
                 VALUES ('p1', 'claude', 'Provider One', '{}', '{}')",
                [],
            )?;
        }
        let exported = source.export_sql_string()?;

        let target = Database::memory()?;
        target.import_sql_string(&exported)?;

        let conn = crate::database::lock_conn!(target.conn);
        let name: String = conn.query_row(
            "SELECT name FROM providers WHERE id = 'p1' AND app_type = 'claude'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(name, "Provider One");
        Ok(())
    }

    #[test]
    #[serial]
    fn sql_file_api_round_trips_existing_export_behavior() -> Result<(), AppError> {
        let test_home = TestHomeGuard::new();
        let source = Database::memory()?;
        {
            let conn = crate::database::lock_conn!(source.conn);
            conn.execute_batch(
                "INSERT INTO providers (id, app_type, name, settings_config, meta)
                 VALUES ('file-provider', 'claude', 'File Provider', '{}', '{}');
                 INSERT INTO proxy_request_logs (
                     request_id, provider_id, app_type, model,
                     input_tokens, output_tokens, total_cost_usd,
                     latency_ms, status_code, created_at
                 ) VALUES ('file-request', 'file-provider', 'claude', 'claude-file', 5, 3, '0', 10, 200, 1);",
            )?;
        }

        let backup_path = test_home.path().join("round-trip.sql");
        source.export_sql(&backup_path)?;

        let target = Database::memory()?;
        {
            let conn = crate::database::lock_conn!(target.conn);
            conn.execute(
                "INSERT INTO providers (id, app_type, name, settings_config, meta)
                 VALUES ('target-sentinel', 'claude', 'Must Be Replaced', '{}', '{}')",
                [],
            )?;
        }
        target.import_sql(&backup_path)?;

        let conn = crate::database::lock_conn!(target.conn);
        let providers = conn
            .prepare("SELECT id FROM providers ORDER BY id")?
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(providers, vec!["file-provider"]);
        let request_exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM proxy_request_logs WHERE request_id = 'file-request')",
            [],
            |row| row.get(0),
        )?;
        assert!(request_exists, "文件 API 必须完整恢复导出数据");
        Ok(())
    }

    #[test]
    #[serial]
    fn failed_sql_import_keeps_the_existing_database_unchanged() -> Result<(), AppError> {
        let _test_home = TestHomeGuard::new();
        let target = Database::memory()?;
        {
            let conn = crate::database::lock_conn!(target.conn);
            conn.execute(
                "INSERT INTO providers (id, app_type, name, settings_config, meta)
                 VALUES ('sentinel', 'claude', 'Existing Provider', '{}', '{}')",
                [],
            )?;
        }

        let invalid_sql = format!(
            "{}\nBEGIN TRANSACTION;\nCREATE TABLE partial (id INTEGER);\nTHIS IS NOT SQL;\n",
            super::CC_SWITCH_SQL_EXPORT_HEADER
        );
        assert!(target.import_sql_string(&invalid_sql).is_err());

        let conn = crate::database::lock_conn!(target.conn);
        let provider: (i64, String, String) = conn.query_row(
            "SELECT COUNT(*), MIN(id), MIN(name) FROM providers",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        assert_eq!(provider, (1, "sentinel".into(), "Existing Provider".into()));
        let partial_exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'partial')",
            [],
            |row| row.get(0),
        )?;
        assert!(!partial_exists, "失败导入的临时对象不得进入主库");
        Ok(())
    }

    #[test]
    #[serial]
    fn import_still_accepts_legacy_single_row_insert_exports() -> Result<(), AppError> {
        let _test_home = TestHomeGuard::new();
        // This schema is copied from the v3.8.3 tag. Its data statements use
        // the historical one-row-per-INSERT format and omit all newer columns.
        let legacy = format!(
            "{}\nPRAGMA foreign_keys=OFF;\nPRAGMA user_version=1;\nBEGIN TRANSACTION;\n{}
             INSERT INTO providers (
                 id, app_type, name, settings_config, meta, is_current
             ) VALUES (
                 'legacy-provider', 'claude', 'Legacy Provider',
                 '{{\"anthropicApiKey\":\"sk-old\"}}', '{{}}', 1
             );
             INSERT INTO skills (key, installed, installed_at)
             VALUES ('claude:legacy-skill', 1, 1700000000);
             COMMIT;\nPRAGMA foreign_keys=ON;\n",
            super::CC_SWITCH_SQL_EXPORT_HEADER,
            crate::database::tests::V3_8_SCHEMA_V1_SQL,
        );

        let target = Database::memory()?;
        target.import_sql_string(&legacy)?;

        let conn = crate::database::lock_conn!(target.conn);
        let user_version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        assert_eq!(user_version, crate::database::SCHEMA_VERSION);
        let provider: (String, String) = conn.query_row(
            "SELECT name, settings_config FROM providers WHERE id = 'legacy-provider'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(
            provider,
            (
                "Legacy Provider".into(),
                "{\"anthropicApiKey\":\"sk-old\"}".into()
            )
        );
        let cost_multiplier: String = conn.query_row(
            "SELECT cost_multiplier FROM providers WHERE id = 'legacy-provider'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(cost_multiplier, "1.0");
        let skill_snapshot: String = conn.query_row(
            "SELECT value FROM settings WHERE key = 'skills_ssot_migration_snapshot'",
            [],
            |row| row.get(0),
        )?;
        assert!(
            skill_snapshot.contains("legacy-skill"),
            "重建 skills 表时必须保留旧数据迁移快照"
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn dump_sql_batches_rows_into_multi_row_inserts() -> Result<(), AppError> {
        let _test_home = TestHomeGuard::new();
        // 每行一条 INSERT 是导入慢的根源（恢复侧逐条解析，2 万行实测 21s）。
        // 这条测试钉死批量格式：450 行必须合并成 ceil(450/200) = 3 条语句。
        // 一旦退回到逐行导出，这里立刻变红。
        let db = Database::memory()?;
        {
            let conn = crate::database::lock_conn!(db.conn);
            for i in 0..450 {
                conn.execute(
                    "INSERT INTO providers (id, app_type, name, settings_config, meta)
                     VALUES (?1, 'claude', 'p', '{}', '{}')",
                    [format!("p{i}")],
                )?;
            }
        }

        let sql = db.export_sql_string()?;
        let insert_count = sql.matches("INSERT INTO \"providers\"").count();
        assert_eq!(
            insert_count, 3,
            "450 行应合并为 3 条多行 INSERT（每批 200 行），实际 {insert_count} 条"
        );

        let target = Database::memory()?;
        target.import_sql_string(&sql)?;
        let conn = crate::database::lock_conn!(target.conn);
        let row_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM providers", [], |row| row.get(0))?;
        assert_eq!(row_count, 450, "批次边界不得漏行或重复行");
        for boundary in [0, 199, 200, 399, 400, 449] {
            let exists: bool = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM providers WHERE id = ?1)",
                [format!("p{boundary}")],
                |row| row.get(0),
            )?;
            assert!(exists, "批次边界行 p{boundary} 必须完整恢复");
        }
        Ok(())
    }

    #[test]
    fn dump_sql_splits_large_rows_by_statement_bytes() -> Result<(), AppError> {
        let source = Connection::open_in_memory()?;
        source.execute(
            "CREATE TABLE large_rows (id INTEGER PRIMARY KEY, payload TEXT NOT NULL)",
            [],
        )?;

        // Each row fits below the byte cap, while any pair exceeds it.
        let payload = "x".repeat(super::INSERT_BATCH_MAX_BYTES / 2 + 1024);
        for id in 1..=3 {
            source.execute(
                "INSERT INTO large_rows (id, payload) VALUES (?1, ?2)",
                rusqlite::params![id, payload],
            )?;
        }

        let sql = Database::dump_sql(&source, &[])?;
        let inserts = sql
            .lines()
            .filter(|line| line.starts_with("INSERT INTO \"large_rows\""))
            .collect::<Vec<_>>();
        assert_eq!(inserts.len(), 3, "超大字段应按 SQL 字节数提前切批");
        assert!(
            inserts
                .iter()
                .all(|statement| statement.len() <= super::INSERT_BATCH_MAX_BYTES),
            "每条可独立容纳的 INSERT 都应保持在字节上限内"
        );

        let target = Connection::open_in_memory()?;
        target.execute_batch(&sql)?;
        let (count, min_len, max_len): (i64, i64, i64) = target.query_row(
            "SELECT COUNT(*), MIN(length(payload)), MAX(length(payload)) FROM large_rows",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        assert_eq!(count, 3);
        assert_eq!(min_len, payload.len() as i64);
        assert_eq!(max_len, payload.len() as i64);
        Ok(())
    }

    #[test]
    fn dump_sql_round_trips_generated_columns_and_quoted_identifiers() -> Result<(), AppError> {
        let source = Connection::open_in_memory()?;
        source.execute_batch(
            r#"
            CREATE TABLE "generated""values" (
                "a" TEXT NOT NULL,
                "computed" TEXT GENERATED ALWAYS AS ("a" || '-generated') STORED,
                "b""tail" TEXT NOT NULL
            );
            INSERT INTO "generated""values" ("a", "b""tail")
            VALUES ('source', 'ordinary-tail');
            "#,
        )?;

        let sql = Database::dump_sql(&source, &[])?;
        assert!(sql.contains("INSERT INTO \"generated\"\"values\" (\"a\", \"b\"\"tail\") VALUES"));

        let target = Connection::open_in_memory()?;
        target.execute_batch(&sql)?;
        let values: (String, String, String) = target.query_row(
            "SELECT \"a\", \"computed\", \"b\"\"tail\" FROM \"generated\"\"values\"",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        assert_eq!(
            values,
            (
                "source".to_string(),
                "source-generated".to_string(),
                "ordinary-tail".to_string()
            )
        );
        Ok(())
    }

    #[test]
    fn restore_tables_reads_only_insertable_columns() -> Result<(), AppError> {
        let source = Connection::open_in_memory()?;
        let target = Connection::open_in_memory()?;
        for conn in [&source, &target] {
            conn.execute_batch(
                r#"
                CREATE TABLE generated_values (
                    a TEXT NOT NULL,
                    computed TEXT GENERATED ALWAYS AS (a || '-generated') STORED,
                    "b""tail" TEXT NOT NULL
                );
                "#,
            )?;
        }
        source.execute(
            "INSERT INTO generated_values (a, \"b\"\"tail\") VALUES ('new', 'new-tail')",
            [],
        )?;
        target.execute(
            "INSERT INTO generated_values (a, \"b\"\"tail\") VALUES ('old', 'old-tail')",
            [],
        )?;

        Database::restore_tables(&source, &target, &["generated_values"])?;

        let values: (String, String, String) = target.query_row(
            "SELECT a, computed, \"b\"\"tail\" FROM generated_values",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        assert_eq!(
            values,
            (
                "new".to_string(),
                "new-generated".to_string(),
                "new-tail".to_string()
            )
        );
        Ok(())
    }

    #[test]
    fn restore_tables_rolls_back_all_tables_on_late_failure() -> Result<(), AppError> {
        let source = Connection::open_in_memory()?;
        source.execute_batch(
            "CREATE TABLE first_table (value TEXT NOT NULL);
             CREATE TABLE second_table (value INTEGER NOT NULL);
             INSERT INTO first_table VALUES ('replacement');
             INSERT INTO second_table VALUES (-1);",
        )?;

        let target = Connection::open_in_memory()?;
        target.execute_batch(
            "CREATE TABLE first_table (value TEXT NOT NULL);
             CREATE TABLE second_table (value INTEGER NOT NULL CHECK (value >= 0));
             INSERT INTO first_table VALUES ('sentinel-first');
             INSERT INTO second_table VALUES (7);",
        )?;

        let result = Database::restore_tables(&source, &target, &["first_table", "second_table"]);
        assert!(result.is_err(), "第二张表的约束错误必须终止恢复");

        let first: String =
            target.query_row("SELECT value FROM first_table", [], |row| row.get(0))?;
        let second: i64 =
            target.query_row("SELECT value FROM second_table", [], |row| row.get(0))?;
        assert_eq!(first, "sentinel-first", "第一张表必须随事务整体回滚");
        assert_eq!(second, 7, "失败表的 DELETE 也必须回滚");
        Ok(())
    }

    #[test]
    fn dump_sql_loads_rows_before_creating_triggers() -> Result<(), AppError> {
        let source = Connection::open_in_memory()?;
        source.execute_batch(
            "CREATE TABLE triggered_rows (seq INTEGER PRIMARY KEY);
             INSERT INTO triggered_rows VALUES (1), (2), (3);
             CREATE TRIGGER ignore_second_row
             BEFORE INSERT ON triggered_rows
             WHEN NEW.seq = 2
             BEGIN
                 SELECT RAISE(IGNORE);
             END;",
        )?;

        let sql = Database::dump_sql(&source, &[])?;
        let data_pos = sql.find("INSERT INTO \"triggered_rows\"").unwrap();
        let trigger_pos = sql.find("CREATE TRIGGER ignore_second_row").unwrap();
        assert!(data_pos < trigger_pos, "触发器必须在数据恢复完成后创建");

        let target = Connection::open_in_memory()?;
        target.execute_batch(&sql)?;
        let rows = target
            .prepare("SELECT seq FROM triggered_rows ORDER BY seq")?
            .query_map([], |row| row.get::<_, i64>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(rows, vec![1, 2, 3]);
        let trigger_exists: bool = target.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'trigger' AND name = 'ignore_second_row')",
            [],
            |row| row.get(0),
        )?;
        assert!(trigger_exists, "触发器本身仍必须随备份恢复");
        Ok(())
    }

    #[test]
    fn dump_sql_preserves_indexes_and_views() -> Result<(), AppError> {
        let source = Connection::open_in_memory()?;
        source.execute_batch(
            "CREATE TABLE indexed_rows (id INTEGER PRIMARY KEY, value TEXT NOT NULL);
             CREATE UNIQUE INDEX indexed_rows_value_idx ON indexed_rows(value);
             CREATE VIEW indexed_rows_view AS
                 SELECT id, value FROM indexed_rows WHERE value LIKE 'kept%';
             CREATE TRIGGER a_insert_indexed_rows_view
             INSTEAD OF INSERT ON indexed_rows_view
             BEGIN
                 INSERT INTO indexed_rows (id, value) VALUES (NEW.id, NEW.value);
             END;
             INSERT INTO indexed_rows VALUES (1, 'kept-value'), (2, 'hidden-value');",
        )?;

        let sql = Database::dump_sql(&source, &[])?;
        let target = Connection::open_in_memory()?;
        target.execute_batch(&sql)?;

        for (object_type, object_name) in [
            ("index", "indexed_rows_value_idx"),
            ("view", "indexed_rows_view"),
            ("trigger", "a_insert_indexed_rows_view"),
        ] {
            let exists: bool = target.query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM sqlite_master WHERE type = ?1 AND name = ?2
                )",
                [object_type, object_name],
                |row| row.get(0),
            )?;
            assert!(exists, "{object_type} {object_name} 必须随 SQL dump 恢复");
        }

        target.execute(
            "INSERT INTO indexed_rows_view (id, value) VALUES (3, 'kept-via-trigger')",
            [],
        )?;
        let view_rows = target
            .prepare("SELECT id, value FROM indexed_rows_view ORDER BY id")?
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(
            view_rows,
            vec![
                (1, "kept-value".to_string()),
                (3, "kept-via-trigger".to_string()),
            ]
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn multi_row_dump_round_trips_special_values() -> Result<(), AppError> {
        let _test_home = TestHomeGuard::new();
        // 多行 VALUES 的转义面比单行宽：单引号、换行、英文逗号（列分隔符）、
        // 中文、emoji、BLOB、NULL——任何一个处理错都会让整批语法崩掉或数据变形。
        let source = Database::memory()?;
        {
            let conn = crate::database::lock_conn!(source.conn);
            conn.execute(
                "INSERT INTO providers (id, app_type, name, settings_config, meta)
                 VALUES ('special', 'claude', ?1, ?2, '{}')",
                rusqlite::params!["O'Brien,\n第二行 \"quoted\" 😀", "{\"key\": \"it's, ok\"}"],
            )?;
            conn.execute(
                "INSERT INTO providers (id, app_type, name, settings_config, meta)
                 VALUES ('with-blob', 'claude', 'blob', X'00FF10', '{}')",
                [],
            )?;
            conn.execute(
                "INSERT INTO providers (id, app_type, name, settings_config, meta, category)
                 VALUES ('with-null', 'claude', 'nullcat', '{}', '{}', NULL)",
                [],
            )?;
        }

        let sql = source.export_sql_string()?;
        let target = Database::memory()?;
        target.import_sql_string(&sql)?;

        let conn = crate::database::lock_conn!(target.conn);
        let name: String = conn.query_row(
            "SELECT name FROM providers WHERE id = 'special'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(name, "O'Brien,\n第二行 \"quoted\" 😀");
        let cfg: String = conn.query_row(
            "SELECT settings_config FROM providers WHERE id = 'special'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(cfg, "{\"key\": \"it's, ok\"}");

        let blob_type: String = conn.query_row(
            "SELECT typeof(settings_config) FROM providers WHERE id = 'with-blob'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(blob_type, "blob", "BLOB 存储类型必须在往返后保留");
        let blob: Vec<u8> = conn.query_row(
            "SELECT settings_config FROM providers WHERE id = 'with-blob'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(blob, vec![0x00, 0xFF, 0x10]);

        let category: Option<String> = conn.query_row(
            "SELECT category FROM providers WHERE id = 'with-null'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(category, None, "NULL 必须在往返后保留");
        Ok(())
    }

    #[test]
    #[serial]
    fn sync_import_preserves_local_only_tables() -> Result<(), AppError> {
        let _test_home = TestHomeGuard::new();
        let remote_db = Database::memory()?;
        {
            let conn = crate::database::lock_conn!(remote_db.conn);
            conn.execute_batch(
                "INSERT INTO providers (id, app_type, name, settings_config, meta)
                 VALUES ('remote-provider', 'claude', 'Remote Provider', '{}', '{}');
                 INSERT INTO proxy_request_logs (
                     request_id, provider_id, app_type, model,
                     input_tokens, output_tokens, total_cost_usd,
                     latency_ms, status_code, created_at
                 ) VALUES ('remote-request', 'remote-provider', 'claude', 'remote-model', 1, 1, '1', 1, 200, 1);
                 INSERT INTO usage_daily_rollups (
                     date, app_type, provider_id, model, request_count, success_count,
                     input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                     total_cost_usd, avg_latency_ms
                 ) VALUES ('2099-01-01', 'claude', 'remote-provider', 'remote-model', 1, 1, 1, 1, 0, 0, '1', 1);
                 INSERT INTO stream_check_logs (
                     provider_id, provider_name, app_type, status, success, message,
                     response_time_ms, http_status, model_used, retry_count, tested_at
                 ) VALUES ('remote-provider', 'Remote Provider', 'claude', 'failed', 0, 'remote', 1, 500, 'remote-model', 0, 1);
                 INSERT INTO proxy_live_backup (app_type, original_config, backed_up_at)
                 VALUES ('claude', 'remote-live', '2099-01-01');
                 INSERT INTO provider_health (
                     provider_id, app_type, is_healthy, consecutive_failures, updated_at
                 ) VALUES ('remote-provider', 'claude', 0, 9, '2099-01-01');",
            )?;
        }
        let remote_sql = remote_db.export_sql_string_for_sync()?;
        let exported = Connection::open_in_memory()?;
        exported.execute_batch(&remote_sql)?;
        let skipped_counts: (i64, i64, i64, i64, i64) = exported.query_row(
            "SELECT
                (SELECT COUNT(*) FROM proxy_request_logs),
                (SELECT COUNT(*) FROM stream_check_logs),
                (SELECT COUNT(*) FROM provider_health),
                (SELECT COUNT(*) FROM proxy_live_backup),
                (SELECT COUNT(*) FROM usage_daily_rollups)",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )?;
        assert_eq!(skipped_counts, (0, 0, 0, 0, 0));

        let local_db = Database::memory()?;
        {
            let conn = crate::database::lock_conn!(local_db.conn);
            conn.execute_batch(
                "INSERT INTO providers (id, app_type, name, settings_config, meta)
                 VALUES ('local-provider', 'claude', 'Local Provider', '{}', '{}');
                 INSERT INTO proxy_request_logs (
                     request_id, provider_id, app_type, model,
                     input_tokens, output_tokens, total_cost_usd,
                     latency_ms, status_code, created_at
                 ) VALUES ('req-1', 'local-provider', 'claude', 'claude-3', 100, 50, '0.01', 120, 200, 1000);
                 INSERT INTO usage_daily_rollups (
                     date, app_type, provider_id, model, request_count, success_count,
                     input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                     total_cost_usd, avg_latency_ms
                 ) VALUES ('2026-03-01', 'claude', 'local-provider', 'claude-3', 7, 7, 700, 350, 0, 0, '0.07', 120);
                 INSERT INTO stream_check_logs (
                     provider_id, provider_name, app_type, status, success, message,
                     response_time_ms, http_status, model_used, retry_count, tested_at
                 ) VALUES ('local-provider', 'Local Provider', 'claude', 'operational', 1, 'local-ok', 42, 200, 'claude-3', 0, 1000);
                 INSERT INTO proxy_live_backup (app_type, original_config, backed_up_at)
                 VALUES ('claude', '{\"local\":true}', '2026-03-01');
                 INSERT INTO provider_health (
                     provider_id, app_type, is_healthy, consecutive_failures, updated_at
                 ) VALUES ('local-provider', 'claude', 1, 0, '2026-03-01');",
            )?;
        }

        local_db.import_sql_string_for_sync(&remote_sql)?;

        let conn = crate::database::lock_conn!(local_db.conn);
        let providers = conn
            .prepare("SELECT id FROM providers ORDER BY id")?
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(providers, vec!["remote-provider"]);

        let preserved_counts: (i64, i64, i64, i64) = conn.query_row(
            "SELECT
                (SELECT COUNT(*) FROM proxy_request_logs),
                (SELECT COUNT(*) FROM stream_check_logs),
                (SELECT COUNT(*) FROM proxy_live_backup),
                (SELECT COUNT(*) FROM usage_daily_rollups)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;
        assert_eq!(
            preserved_counts,
            (1, 1, 1, 1),
            "同步导入必须替换配置，同时保留本机日志与 Live 备份"
        );

        let preserved_values: (String, String, i64, String, i64, String, i64) = conn.query_row(
            "SELECT
                (SELECT request_id FROM proxy_request_logs),
                (SELECT model FROM proxy_request_logs),
                (SELECT input_tokens FROM proxy_request_logs),
                (SELECT date FROM usage_daily_rollups),
                (SELECT request_count FROM usage_daily_rollups),
                (SELECT message FROM stream_check_logs),
                (SELECT response_time_ms FROM stream_check_logs)",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )?;
        assert_eq!(
            preserved_values,
            (
                "req-1".into(),
                "claude-3".into(),
                100,
                "2026-03-01".into(),
                7,
                "local-ok".into(),
                42,
            )
        );

        let live_backup: (String, String) = conn.query_row(
            "SELECT original_config, backed_up_at FROM proxy_live_backup WHERE app_type = 'claude'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(
            live_backup,
            ("{\"local\":true}".into(), "2026-03-01".into())
        );
        let provider_health_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM provider_health", [], |row| row.get(0))?;
        assert_eq!(
            provider_health_count, 0,
            "同步导入应清除可重建的本地 provider_health 状态"
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn sql_import_holds_main_lock_across_safety_backup_and_replace() {
        let db = Arc::new(Database::memory().expect("create database"));
        let sql = db.export_sql_string().expect("export database");
        let (hook_ready_tx, hook_ready_rx) = mpsc::channel();
        let (continue_tx, continue_rx) = mpsc::channel();
        let database_key = std::ptr::from_ref(&db.conn) as usize;
        *RESTORE_CRITICAL_SECTION_HOOK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some((
            database_key,
            Box::new(move || {
                hook_ready_tx.send(()).expect("signal restore hook");
                continue_rx.recv().expect("resume restore");
            }),
        ));

        let import_db = Arc::clone(&db);
        let import_thread = std::thread::spawn(move || import_db.import_sql_string(&sql));
        // Generous deadline: before the hook fires, the canonical restore replays
        // the full v0→v16 migration chain on the temp DB, which can take well
        // over 5s on slow shared CI runners.
        hook_ready_rx
            .recv_timeout(Duration::from_secs(60))
            .expect("restore should reach critical section hook");

        let (writer_started_tx, writer_started_rx) = mpsc::channel();
        let (writer_done_tx, writer_done_rx) = mpsc::channel();
        let writer_db = Arc::clone(&db);
        let writer_thread = std::thread::spawn(move || {
            writer_started_tx.send(()).expect("signal writer start");
            writer_db
                .set_setting("restore-concurrent-write", "survives")
                .expect("write setting");
            writer_done_tx.send(()).expect("signal writer done");
        });
        writer_started_rx.recv().expect("writer should start");
        assert!(
            writer_done_rx
                .recv_timeout(Duration::from_millis(100))
                .is_err(),
            "writer must remain blocked while safety backup and replacement hold the main lock"
        );

        continue_tx.send(()).expect("resume restore");
        import_thread
            .join()
            .expect("join import thread")
            .expect("import database");
        writer_done_rx
            .recv_timeout(Duration::from_secs(60))
            .expect("writer should finish after restore unlocks");
        writer_thread.join().expect("join writer thread");
        assert_eq!(
            db.get_setting("restore-concurrent-write")
                .expect("read concurrent write")
                .as_deref(),
            Some("survives")
        );
    }

    #[test]
    #[serial]
    fn periodic_maintenance_runs_even_when_auto_backup_disabled() -> Result<(), AppError> {
        let _test_home = TestHomeGuard::new();

        let settings = AppSettings {
            backup_interval_hours: Some(0),
            ..Default::default()
        };
        update_settings(settings).expect("disable auto backup");

        let db = Database::memory()?;
        let now = chrono::Utc::now().timestamp();
        let old_ts = now - 40 * 86400;
        let old_stream_ts = now - 8 * 86400;

        {
            let conn = crate::database::lock_conn!(db.conn);
            conn.execute(
                "INSERT INTO proxy_request_logs (
                    request_id, provider_id, app_type, model,
                    input_tokens, output_tokens, total_cost_usd,
                    latency_ms, status_code, created_at
                ) VALUES ('old-req', 'p1', 'claude', 'claude-3', 100, 50, '0.01', 100, 200, ?1)",
                [old_ts],
            )?;
            conn.execute(
                "INSERT INTO stream_check_logs (
                    provider_id, provider_name, app_type, status, success, message,
                    response_time_ms, http_status, model_used, retry_count, tested_at
                ) VALUES ('p1', 'Provider 1', 'claude', 'operational', 1, 'ok', 42, 200, 'claude-3', 0, ?1)",
                [old_stream_ts],
            )?;
        }

        db.periodic_backup_if_needed()?;

        let (remaining_request_logs, stream_logs, rollups): (i64, i64, i64) = {
            let conn = crate::database::lock_conn!(db.conn);
            let remaining_request_logs =
                conn.query_row("SELECT COUNT(*) FROM proxy_request_logs", [], |row| {
                    row.get(0)
                })?;
            let stream_logs =
                conn.query_row("SELECT COUNT(*) FROM stream_check_logs", [], |row| {
                    row.get(0)
                })?;
            let rollups =
                conn.query_row("SELECT COUNT(*) FROM usage_daily_rollups", [], |row| {
                    row.get(0)
                })?;
            (remaining_request_logs, stream_logs, rollups)
        };

        assert_eq!(
            remaining_request_logs, 0,
            "old request logs should still be pruned when auto backup is disabled"
        );
        assert_eq!(
            stream_logs, 0,
            "old stream check logs should still be pruned when auto backup is disabled"
        );
        assert_eq!(rollups, 1, "old request logs should be rolled up");

        Ok(())
    }

    /// 性能基准（不是回归测试）：用接近重度代理用户的行数测量
    /// 导出 / 本地文件导入 / 同步导入三条路径的耗时与产物大小。
    ///
    /// 手动运行：`cargo test --lib perf_backup -- --ignored --nocapture`
    #[test]
    #[ignore = "perf harness, run explicitly"]
    #[serial]
    fn perf_backup_export_import_paths() -> Result<(), AppError> {
        use std::time::Instant;

        const LOG_ROWS: usize = 20_000;
        const STREAM_ROWS: usize = 5_000;
        const ROLLUP_ROWS: usize = 1_000;

        let _test_home = TestHomeGuard::new();

        fn populate(
            db: &Database,
            log_rows: usize,
            stream_rows: usize,
            rollup_rows: usize,
        ) -> Result<(), AppError> {
            let mut conn = crate::database::lock_conn!(db.conn);
            let tx = conn.transaction()?;
            for i in 0..50 {
                tx.execute(
                    "INSERT INTO providers (id, app_type, name, settings_config, meta)
                     VALUES (?1, 'claude', ?2, '{}', '{}')",
                    rusqlite::params![format!("p{i}"), format!("Provider {i}")],
                )?;
            }
            for i in 0..log_rows {
                tx.execute(
                    "INSERT INTO proxy_request_logs (
                        request_id, provider_id, app_type, model,
                        input_tokens, output_tokens, total_cost_usd,
                        latency_ms, status_code, created_at
                    ) VALUES (?1, 'p1', 'claude', 'claude-3', 100, 50, '0.01', 120, 200, 1000)",
                    [format!("req-{i}")],
                )?;
            }
            for i in 0..stream_rows {
                tx.execute(
                    "INSERT INTO stream_check_logs (
                        provider_id, provider_name, app_type, status, success, message,
                        response_time_ms, http_status, model_used, retry_count, tested_at
                    ) VALUES ('p1', 'Provider 1', 'claude', 'operational', 1, 'ok', 42, 200, 'claude-3', 0, ?1)",
                    [1000i64 + i as i64],
                )?;
            }
            for i in 0..rollup_rows {
                // (date, app_type, provider_id, model, request_model, pricing_model)
                // 上有 UNIQUE 约束，日期必须逐行唯一。
                let date = format!(
                    "{:04}-{:02}-{:02}",
                    2025 + i / 336,
                    i / 28 % 12 + 1,
                    i % 28 + 1
                );
                tx.execute(
                    "INSERT INTO usage_daily_rollups (
                        date, app_type, provider_id, model, request_count, success_count,
                        input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                        total_cost_usd, avg_latency_ms
                    ) VALUES (?1, 'claude', 'p1', 'claude-3', 7, 7, 700, 350, 0, 0, '0.07', 120)",
                    [date],
                )?;
            }
            tx.commit()?;
            Ok(())
        }

        let source = Database::memory()?;
        populate(&source, LOG_ROWS, STREAM_ROWS, ROLLUP_ROWS)?;

        let t = Instant::now();
        let full_sql = source.export_sql_string()?;
        println!(
            "export_sql_string (full): {:?}, {} bytes",
            t.elapsed(),
            full_sql.len()
        );

        let t = Instant::now();
        let import_target = Database::memory()?;
        import_target.import_sql_string(&full_sql)?;
        println!("import_sql_string (local file path): {:?}", t.elapsed());
        {
            let conn = crate::database::lock_conn!(import_target.conn);
            let counts: (i64, i64, i64, i64) = conn.query_row(
                "SELECT
                    (SELECT COUNT(*) FROM providers),
                    (SELECT COUNT(*) FROM proxy_request_logs),
                    (SELECT COUNT(*) FROM stream_check_logs),
                    (SELECT COUNT(*) FROM usage_daily_rollups)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )?;
            assert_eq!(
                counts,
                (50, LOG_ROWS as i64, STREAM_ROWS as i64, ROLLUP_ROWS as i64)
            );
        }

        let sync_sql = source.export_sql_string_for_sync()?;
        println!("sync payload: {} bytes", sync_sql.len());

        // 同步导入的耗时大头在“保留本机日志表”——本机库必须带同样规模的日志行。
        let local = Database::memory()?;
        populate(&local, LOG_ROWS, STREAM_ROWS, ROLLUP_ROWS)?;
        let t = Instant::now();
        local.import_sql_string_for_sync(&sync_sql)?;
        println!(
            "import_sql_string_for_sync ({} preserved log rows): {:?}",
            LOG_ROWS + STREAM_ROWS + ROLLUP_ROWS,
            t.elapsed()
        );
        {
            let conn = crate::database::lock_conn!(local.conn);
            let counts: (i64, i64, i64) = conn.query_row(
                "SELECT
                    (SELECT COUNT(*) FROM proxy_request_logs),
                    (SELECT COUNT(*) FROM stream_check_logs),
                    (SELECT COUNT(*) FROM usage_daily_rollups)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?;
            assert_eq!(
                counts,
                (LOG_ROWS as i64, STREAM_ROWS as i64, ROLLUP_ROWS as i64)
            );
        }
        Ok(())
    }

    /// 分阶段拆解 import_sql_string 的耗时，定位慢在哪一步。
    ///
    /// 手动运行：`cargo test --lib perf_import_phases -- --ignored --nocapture`
    #[test]
    #[ignore = "perf diagnostic, run explicitly"]
    fn perf_import_phases() -> Result<(), AppError> {
        use rusqlite::Connection;
        use std::time::Instant;
        use tempfile::NamedTempFile;

        const LOG_ROWS: usize = 20_000;

        let source = Database::memory()?;
        {
            let mut conn = crate::database::lock_conn!(source.conn);
            let tx = conn.transaction()?;
            for i in 0..50 {
                tx.execute(
                    "INSERT INTO providers (id, app_type, name, settings_config, meta)
                     VALUES (?1, 'claude', ?2, '{}', '{}')",
                    rusqlite::params![format!("p{i}"), format!("Provider {i}")],
                )?;
            }
            for i in 0..LOG_ROWS {
                tx.execute(
                    "INSERT INTO proxy_request_logs (
                        request_id, provider_id, app_type, model,
                        input_tokens, output_tokens, total_cost_usd,
                        latency_ms, status_code, created_at
                    ) VALUES (?1, 'p1', 'claude', 'claude-3', 100, 50, '0.01', 120, 200, 1000)",
                    [format!("req-{i}")],
                )?;
            }
            tx.commit()?;
        }
        let sql = source.export_sql_string()?;
        println!("payload: {} bytes, {LOG_ROWS} log rows", sql.len());

        let temp_file = NamedTempFile::new().expect("temp file");
        let temp_conn = Connection::open(temp_file.path()).expect("open temp conn");

        let t = Instant::now();
        temp_conn
            .execute_batch(&sql)
            .expect("execute_batch should succeed");
        println!("phase execute_batch: {:?}", t.elapsed());

        let t = Instant::now();
        Database::create_tables_on_conn(&temp_conn)?;
        Database::apply_schema_migrations_on_conn(&temp_conn)?;
        println!("phase schema+migrations: {:?}", t.elapsed());

        let t = Instant::now();
        let target = Database::memory()?;
        {
            let mut main_conn = crate::database::lock_conn!(target.conn);
            let backup =
                rusqlite::backup::Backup::new(&temp_conn, &mut main_conn).expect("backup init");
            backup.step(-1).expect("backup step");
        }
        println!("phase backup-to-main: {:?}", t.elapsed());

        // 对照组：同样的语句但临时库关掉 journal / synchronous。
        let temp_file2 = NamedTempFile::new().expect("temp file 2");
        let temp_conn2 = Connection::open(temp_file2.path()).expect("open temp conn 2");
        temp_conn2
            .execute_batch("PRAGMA journal_mode=MEMORY; PRAGMA synchronous=OFF;")
            .expect("pragmas");
        let t = Instant::now();
        temp_conn2
            .execute_batch(&sql)
            .expect("execute_batch should succeed");
        println!(
            "phase execute_batch (journal=MEMORY, sync=OFF): {:?}",
            t.elapsed()
        );

        // 对照组 B：同一份脚本跑在内存库上，区分“纯 CPU/解析”还是“文件 I/O”。
        let mem_conn = Connection::open_in_memory().expect("open mem conn");
        let t = Instant::now();
        mem_conn
            .execute_batch(&sql)
            .expect("execute_batch mem should succeed");
        println!("phase execute_batch (in-memory): {:?}", t.elapsed());

        // 对照组 C：同样的数据改成多行 VALUES（每 200 行一条 INSERT），
        // 验证“每行一条语句”的解析开销占比。
        let mut batched = String::from("PRAGMA foreign_keys=OFF;\nBEGIN TRANSACTION;\n");
        batched.push_str(
            "CREATE TABLE bench_logs (
                request_id TEXT, provider_id TEXT, app_type TEXT, model TEXT,
                input_tokens INTEGER, output_tokens INTEGER, total_cost_usd TEXT,
                latency_ms INTEGER, status_code INTEGER, created_at INTEGER
            );\n",
        );
        const BATCH: usize = 200;
        for chunk_start in (0..LOG_ROWS).step_by(BATCH) {
            batched.push_str("INSERT INTO bench_logs VALUES ");
            for i in chunk_start..(chunk_start + BATCH).min(LOG_ROWS) {
                if i > chunk_start {
                    batched.push(',');
                }
                batched.push_str(&format!(
                    "('req-{i}','p1','claude','claude-3',100,50,'0.01',120,200,1000)"
                ));
            }
            batched.push_str(";\n");
        }
        batched.push_str("COMMIT;\n");
        let mem_conn2 = Connection::open_in_memory().expect("open mem conn 2");
        let t = Instant::now();
        mem_conn2
            .execute_batch(&batched)
            .expect("batched should succeed");
        println!(
            "phase execute_batch (in-memory, multi-row VALUES x{BATCH}): {:?}",
            t.elapsed()
        );

        Ok(())
    }
}
