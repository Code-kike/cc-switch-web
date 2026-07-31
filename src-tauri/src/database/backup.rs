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

    fn install_sql_restore_authorizer(conn: &Connection) {
        conn.authorizer(Some(|ctx: AuthContext<'_>| {
            if ctx.accessor.is_some()
                || matches!(ctx.database_name, Some(database) if database != "main")
            {
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
        for table in tables {
            if !Self::table_exists(source_conn, table)? || !Self::table_exists(target_conn, table)?
            {
                continue;
            }

            let columns = Self::get_table_columns(source_conn, table)?;
            if columns.is_empty() {
                continue;
            }

            target_conn
                .execute(&format!("DELETE FROM \"{table}\""), [])
                .map_err(|e| AppError::Database(format!("清空表 {table} 失败: {e}")))?;

            let placeholders = (1..=columns.len())
                .map(|idx| format!("?{idx}"))
                .collect::<Vec<_>>()
                .join(", ");
            let cols = columns
                .iter()
                .map(|column| format!("\"{column}\""))
                .collect::<Vec<_>>()
                .join(", ");
            let insert_sql = format!("INSERT INTO \"{table}\" ({cols}) VALUES ({placeholders})");

            let mut stmt = source_conn
                .prepare(&format!("SELECT * FROM \"{table}\""))
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

                target_conn
                    .execute(&insert_sql, rusqlite::params_from_iter(values.iter()))
                    .map_err(|e| AppError::Database(format!("恢复表 {table} 数据失败: {e}")))?;
            }
        }

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

            output.push_str(&sql);
            output.push_str(";\n");

            if obj_type == "table" && !name.starts_with("sqlite_") {
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

            let mut stmt = conn
                .prepare(&format!("SELECT * FROM \"{table}\""))
                .map_err(|e| AppError::Database(e.to_string()))?;
            let mut rows = stmt
                .query([])
                .map_err(|e| AppError::Database(e.to_string()))?;

            while let Some(row) = rows.next().map_err(|e| AppError::Database(e.to_string()))? {
                let mut values = Vec::with_capacity(columns.len());
                for idx in 0..columns.len() {
                    let value = row
                        .get_ref(idx)
                        .map_err(|e| AppError::Database(e.to_string()))?;
                    values.push(Self::format_sql_value(value)?);
                }

                let cols = columns
                    .iter()
                    .map(|c| format!("\"{c}\""))
                    .collect::<Vec<_>>()
                    .join(", ");
                output.push_str(&format!(
                    "INSERT INTO \"{table}\" ({cols}) VALUES ({});\n",
                    values.join(", ")
                ));
            }
        }

        output.push_str("COMMIT;\nPRAGMA foreign_keys=ON;\n");
        Ok(output)
    }

    /// 获取表的列名列表
    fn get_table_columns(conn: &Connection, table: &str) -> Result<Vec<String>, AppError> {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info(\"{table}\")"))
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
    use serial_test::serial;
    use std::sync::{mpsc, Arc};
    use std::time::Duration;

    #[test]
    fn sync_import_preserves_local_only_tables() -> Result<(), AppError> {
        let remote_db = Database::memory()?;
        {
            let conn = crate::database::lock_conn!(remote_db.conn);
            conn.execute(
                "INSERT INTO providers (id, app_type, name, settings_config, meta)
                 VALUES ('remote-provider', 'claude', 'Remote Provider', '{}', '{}')",
                [],
            )?;
        }
        let remote_sql = remote_db.export_sql_string_for_sync()?;

        let local_db = Database::memory()?;
        {
            let conn = crate::database::lock_conn!(local_db.conn);
            conn.execute(
                "INSERT INTO providers (id, app_type, name, settings_config, meta)
                 VALUES ('local-provider', 'claude', 'Local Provider', '{}', '{}')",
                [],
            )?;
            conn.execute(
                "INSERT INTO proxy_request_logs (
                    request_id, provider_id, app_type, model,
                    input_tokens, output_tokens, total_cost_usd,
                    latency_ms, status_code, created_at
                ) VALUES ('req-1', 'local-provider', 'claude', 'claude-3', 100, 50, '0.01', 120, 200, 1000)",
                [],
            )?;
            conn.execute(
                "INSERT INTO usage_daily_rollups (
                    date, app_type, provider_id, model, request_count, success_count,
                    input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                    total_cost_usd, avg_latency_ms
                ) VALUES ('2026-03-01', 'claude', 'local-provider', 'claude-3', 7, 7, 700, 350, 0, 0, '0.07', 120)",
                [],
            )?;
            conn.execute(
                "INSERT INTO stream_check_logs (
                    provider_id, provider_name, app_type, status, success, message,
                    response_time_ms, http_status, model_used, retry_count, tested_at
                ) VALUES ('local-provider', 'Local Provider', 'claude', 'operational', 1, 'ok', 42, 200, 'claude-3', 0, 1000)",
                [],
            )?;
        }

        local_db.import_sql_string_for_sync(&remote_sql)?;

        let remote_provider_exists: i64 = {
            let conn = crate::database::lock_conn!(local_db.conn);
            conn.query_row(
                "SELECT COUNT(*) FROM providers WHERE id = 'remote-provider' AND app_type = 'claude'",
                [],
                |row| row.get(0),
            )?
        };
        assert_eq!(
            remote_provider_exists, 1,
            "remote config should be imported"
        );

        let (request_logs, rollups, stream_logs): (i64, i64, i64) = {
            let conn = crate::database::lock_conn!(local_db.conn);
            let request_logs =
                conn.query_row("SELECT COUNT(*) FROM proxy_request_logs", [], |row| {
                    row.get(0)
                })?;
            let rollups =
                conn.query_row("SELECT COUNT(*) FROM usage_daily_rollups", [], |row| {
                    row.get(0)
                })?;
            let stream_logs =
                conn.query_row("SELECT COUNT(*) FROM stream_check_logs", [], |row| {
                    row.get(0)
                })?;
            (request_logs, rollups, stream_logs)
        };
        assert_eq!(request_logs, 1, "local request logs should be preserved");
        assert_eq!(rollups, 1, "local rollups should be preserved");
        assert_eq!(
            stream_logs, 1,
            "local stream check logs should be preserved"
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
        let old_test_home = std::env::var_os("CC_SWITCH_TEST_HOME");
        let test_home =
            std::env::temp_dir().join("cc-switch-periodic-maintenance-backup-disabled-test");
        let _ = std::fs::remove_dir_all(&test_home);
        std::fs::create_dir_all(&test_home).expect("create test home");
        std::env::set_var("CC_SWITCH_TEST_HOME", &test_home);

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

        match old_test_home {
            Some(value) => std::env::set_var("CC_SWITCH_TEST_HOME", value),
            None => std::env::remove_var("CC_SWITCH_TEST_HOME"),
        }

        Ok(())
    }
}
