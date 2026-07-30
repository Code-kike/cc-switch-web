use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::provider::{Provider, ProviderMeta};
use indexmap::IndexMap;
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use std::collections::{HashMap, HashSet};

type OmoProviderRow = (
    String,
    String,
    String,
    Option<String>,
    Option<i64>,
    Option<usize>,
    Option<String>,
    String,
);

fn load_custom_endpoints(
    conn: &Connection,
    provider_id: &str,
    app_type: &str,
) -> Result<HashMap<String, crate::settings::CustomEndpoint>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT url, added_at FROM provider_endpoints
             WHERE provider_id = ?1 AND app_type = ?2
             ORDER BY url ASC, added_at IS NULL ASC, added_at ASC, id ASC",
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

    let endpoints = stmt
        .query_map(params![provider_id, app_type], |row| {
            let url: String = row.get(0)?;
            let added_at: Option<i64> = row.get(1)?;
            Ok((
                url.clone(),
                crate::settings::CustomEndpoint {
                    url,
                    added_at: added_at.unwrap_or(0),
                    last_used: None,
                },
            ))
        })
        .map_err(|e| AppError::Database(e.to_string()))?;

    let mut custom_endpoints = HashMap::new();
    for endpoint in endpoints {
        let (url, endpoint) = endpoint.map_err(|e| AppError::Database(e.to_string()))?;
        custom_endpoints.entry(url).or_insert(endpoint);
    }
    Ok(custom_endpoints)
}

fn reconcile_provider_endpoints(
    tx: &Transaction<'_>,
    provider_id: &str,
    app_type: &str,
    endpoints: &HashMap<String, crate::settings::CustomEndpoint>,
) -> Result<(), AppError> {
    let existing_rows = {
        let mut stmt = tx
            .prepare(
                "SELECT id, url FROM provider_endpoints
                 WHERE provider_id = ?1 AND app_type = ?2
                 ORDER BY url ASC, added_at IS NULL ASC, added_at ASC, id ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        let rows = stmt
            .query_map(params![provider_id, app_type], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| AppError::Database(e.to_string()))?;
        let mut endpoints = Vec::new();
        for row in rows {
            endpoints.push(row.map_err(|e| AppError::Database(e.to_string()))?);
        }
        endpoints
    };
    let mut existing_endpoints = HashMap::new();
    for (id, url) in existing_rows {
        match existing_endpoints.entry(url) {
            std::collections::hash_map::Entry::Occupied(_) => {
                tx.execute("DELETE FROM provider_endpoints WHERE id = ?1", params![id])
                    .map_err(|e| AppError::Database(e.to_string()))?;
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(id);
            }
        }
    }

    for url in existing_endpoints
        .keys()
        .filter(|url| !endpoints.contains_key(*url))
    {
        tx.execute(
            "DELETE FROM provider_endpoints
             WHERE provider_id = ?1 AND app_type = ?2 AND url = ?3",
            params![provider_id, app_type, url],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
    }

    for (url, endpoint) in endpoints {
        if let Some(id) = existing_endpoints.get(url) {
            tx.execute(
                "UPDATE provider_endpoints
                 SET added_at = COALESCE(added_at, ?2)
                 WHERE id = ?1",
                params![id, endpoint.added_at],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        } else {
            tx.execute(
                "INSERT INTO provider_endpoints (provider_id, app_type, url, added_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![provider_id, app_type, url, endpoint.added_at],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        }
    }

    Ok(())
}

impl Database {
    pub fn get_all_providers(
        &self,
        app_type: &str,
    ) -> Result<IndexMap<String, Provider>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn.prepare(
            "SELECT id, name, settings_config, website_url, category, created_at, sort_index, notes, icon, icon_color, meta, in_failover_queue
             FROM providers WHERE app_type = ?1
             ORDER BY COALESCE(sort_index, 999999), created_at ASC, id ASC"
        ).map_err(|e| AppError::Database(e.to_string()))?;

        let provider_iter = stmt
            .query_map(params![app_type], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let settings_config_str: String = row.get(2)?;
                let website_url: Option<String> = row.get(3)?;
                let category: Option<String> = row.get(4)?;
                let created_at: Option<i64> = row.get(5)?;
                let sort_index: Option<usize> = row.get(6)?;
                let notes: Option<String> = row.get(7)?;
                let icon: Option<String> = row.get(8)?;
                let icon_color: Option<String> = row.get(9)?;
                let meta_str: String = row.get(10)?;
                let in_failover_queue: bool = row.get(11)?;

                let settings_config =
                    serde_json::from_str(&settings_config_str).unwrap_or(serde_json::Value::Null);
                let meta: ProviderMeta = serde_json::from_str(&meta_str).unwrap_or_default();

                Ok((
                    id,
                    Provider {
                        id: "".to_string(), // Placeholder, set below
                        name,
                        settings_config,
                        website_url,
                        category,
                        created_at,
                        sort_index,
                        notes,
                        meta: Some(meta),
                        icon,
                        icon_color,
                        in_failover_queue,
                    },
                ))
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut providers = IndexMap::new();
        for provider_res in provider_iter {
            let (id, mut provider) = provider_res.map_err(|e| AppError::Database(e.to_string()))?;
            provider.id = id.clone();

            let custom_endpoints = load_custom_endpoints(&conn, &id, app_type)?;

            if let Some(meta) = &mut provider.meta {
                meta.custom_endpoints = custom_endpoints;
            }

            providers.insert(id, provider);
        }

        Ok(providers)
    }

    pub fn get_current_provider(&self, app_type: &str) -> Result<Option<String>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare("SELECT id FROM providers WHERE app_type = ?1 AND is_current = 1 LIMIT 1")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut rows = stmt
            .query(params![app_type])
            .map_err(|e| AppError::Database(e.to_string()))?;

        if let Some(row) = rows.next().map_err(|e| AppError::Database(e.to_string()))? {
            Ok(Some(
                row.get(0).map_err(|e| AppError::Database(e.to_string()))?,
            ))
        } else {
            Ok(None)
        }
    }

    pub fn get_provider_by_id(
        &self,
        id: &str,
        app_type: &str,
    ) -> Result<Option<Provider>, AppError> {
        let conn = lock_conn!(self.conn);
        let result = conn.query_row(
            "SELECT name, settings_config, website_url, category, created_at, sort_index, notes, icon, icon_color, meta, in_failover_queue
             FROM providers WHERE id = ?1 AND app_type = ?2",
            params![id, app_type],
            |row| {
                let name: String = row.get(0)?;
                let settings_config_str: String = row.get(1)?;
                let website_url: Option<String> = row.get(2)?;
                let category: Option<String> = row.get(3)?;
                let created_at: Option<i64> = row.get(4)?;
                let sort_index: Option<usize> = row.get(5)?;
                let notes: Option<String> = row.get(6)?;
                let icon: Option<String> = row.get(7)?;
                let icon_color: Option<String> = row.get(8)?;
                let meta_str: String = row.get(9)?;
                let in_failover_queue: bool = row.get(10)?;

                let settings_config = serde_json::from_str(&settings_config_str).unwrap_or(serde_json::Value::Null);
                let meta: ProviderMeta = serde_json::from_str(&meta_str).unwrap_or_default();

                Ok(Provider {
                    id: id.to_string(),
                    name,
                    settings_config,
                    website_url,
                    category,
                    created_at,
                    sort_index,
                    notes,
                    meta: Some(meta),
                    icon,
                    icon_color,
                    in_failover_queue,
                })
            },
        );

        match result {
            Ok(mut provider) => {
                let custom_endpoints = load_custom_endpoints(&conn, id, app_type)?;
                provider
                    .meta
                    .get_or_insert_with(ProviderMeta::default)
                    .custom_endpoints = custom_endpoints;
                Ok(Some(provider))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    pub fn save_provider(&self, app_type: &str, provider: &Provider) -> Result<(), AppError> {
        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction()
            .map_err(|e| AppError::Database(e.to_string()))?;

        let endpoints = provider
            .meta
            .as_ref()
            .map(|meta| meta.custom_endpoints.clone());
        let mut meta_clone = provider.meta.clone().unwrap_or_default();
        meta_clone.custom_endpoints.clear();

        let existing: Option<(bool, bool)> = tx
            .query_row(
                "SELECT is_current, in_failover_queue FROM providers WHERE id = ?1 AND app_type = ?2",
                params![provider.id, app_type],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|e| AppError::Database(e.to_string()))?;

        let is_update = existing.is_some();
        let (is_current, in_failover_queue) =
            existing.unwrap_or((false, provider.in_failover_queue));

        if is_update {
            tx.execute(
                "UPDATE providers SET
                    name = ?1,
                    settings_config = ?2,
                    website_url = ?3,
                    category = ?4,
                    created_at = ?5,
                    sort_index = ?6,
                    notes = ?7,
                    icon = ?8,
                    icon_color = ?9,
                    meta = ?10,
                    is_current = ?11,
                    in_failover_queue = ?12
                WHERE id = ?13 AND app_type = ?14",
                params![
                    provider.name,
                    serde_json::to_string(&provider.settings_config).map_err(|e| {
                        AppError::Database(format!("Failed to serialize settings_config: {e}"))
                    })?,
                    provider.website_url,
                    provider.category,
                    provider.created_at,
                    provider.sort_index,
                    provider.notes,
                    provider.icon,
                    provider.icon_color,
                    serde_json::to_string(&meta_clone).map_err(|e| AppError::Database(format!(
                        "Failed to serialize meta: {e}"
                    )))?,
                    is_current,
                    in_failover_queue,
                    provider.id,
                    app_type,
                ],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        } else {
            tx.execute(
                "INSERT INTO providers (
                    id, app_type, name, settings_config, website_url, category,
                    created_at, sort_index, notes, icon, icon_color, meta, is_current, in_failover_queue
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    provider.id,
                    app_type,
                    provider.name,
                    serde_json::to_string(&provider.settings_config)
                        .map_err(|e| AppError::Database(format!("Failed to serialize settings_config: {e}")))?,
                    provider.website_url,
                    provider.category,
                    provider.created_at,
                    provider.sort_index,
                    provider.notes,
                    provider.icon,
                    provider.icon_color,
                    serde_json::to_string(&meta_clone)
                        .map_err(|e| AppError::Database(format!("Failed to serialize meta: {e}")))?,
                    is_current,
                    in_failover_queue,
                ],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        }

        if let Some(endpoints) = endpoints.as_ref() {
            reconcile_provider_endpoints(&tx, &provider.id, app_type, endpoints)?;
        }

        tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn delete_provider(&self, app_type: &str, id: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "DELETE FROM providers WHERE id = ?1 AND app_type = ?2",
            params![id, app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn set_current_provider(&self, app_type: &str, id: &str) -> Result<(), AppError> {
        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction()
            .map_err(|e| AppError::Database(e.to_string()))?;

        tx.execute(
            "UPDATE providers SET is_current = 0 WHERE app_type = ?1",
            params![app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        tx.execute(
            "UPDATE providers SET is_current = 1 WHERE id = ?1 AND app_type = ?2",
            params![id, app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn clear_current_provider(&self, app_type: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "UPDATE providers SET is_current = 0 WHERE app_type = ?1",
            params![app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn update_provider_settings_config(
        &self,
        app_type: &str,
        provider_id: &str,
        settings_config: &serde_json::Value,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "UPDATE providers SET settings_config = ?1 WHERE id = ?2 AND app_type = ?3",
            params![
                serde_json::to_string(settings_config).map_err(|e| AppError::Database(format!(
                    "Failed to serialize settings_config: {e}"
                )))?,
                provider_id,
                app_type
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn add_custom_endpoint(
        &self,
        app_type: &str,
        provider_id: &str,
        url: &str,
    ) -> Result<(), AppError> {
        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction()
            .map_err(|e| AppError::Database(e.to_string()))?;
        let added_at = chrono::Utc::now().timestamp_millis();
        let existing_rows = {
            let mut stmt = tx
                .prepare(
                    "SELECT id FROM provider_endpoints
                     WHERE provider_id = ?1 AND app_type = ?2 AND url = ?3
                     ORDER BY added_at IS NULL ASC, added_at ASC, id ASC",
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            let rows = stmt
                .query_map(params![provider_id, app_type, url], |row| {
                    row.get::<_, i64>(0)
                })
                .map_err(|e| AppError::Database(e.to_string()))?;
            let mut ids = Vec::new();
            for row in rows {
                ids.push(row.map_err(|e| AppError::Database(e.to_string()))?);
            }
            ids
        };

        if let Some((keeper_id, duplicate_ids)) = existing_rows.split_first() {
            tx.execute(
                "UPDATE provider_endpoints
                 SET added_at = COALESCE(added_at, ?2)
                 WHERE id = ?1",
                params![keeper_id, added_at],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            for duplicate_id in duplicate_ids {
                tx.execute(
                    "DELETE FROM provider_endpoints WHERE id = ?1",
                    params![duplicate_id],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            }
        } else {
            tx.execute(
                "INSERT INTO provider_endpoints (provider_id, app_type, url, added_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![provider_id, app_type, url, added_at],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        }

        tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn remove_custom_endpoint(
        &self,
        app_type: &str,
        provider_id: &str,
        url: &str,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "DELETE FROM provider_endpoints WHERE provider_id = ?1 AND app_type = ?2 AND url = ?3",
            params![provider_id, app_type, url],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn set_omo_provider_current(
        &self,
        app_type: &str,
        provider_id: &str,
        category: &str,
    ) -> Result<(), AppError> {
        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction()
            .map_err(|e| AppError::Database(e.to_string()))?;
        tx.execute(
            "UPDATE providers SET is_current = 0 WHERE app_type = ?1 AND category = ?2",
            params![app_type, category],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        // OMO ↔ OMO Slim mutually exclusive: deactivate the opposite category
        let opposite = match category {
            "omo" => Some("omo-slim"),
            "omo-slim" => Some("omo"),
            _ => None,
        };
        if let Some(opp) = opposite {
            tx.execute(
                "UPDATE providers SET is_current = 0 WHERE app_type = ?1 AND category = ?2",
                params![app_type, opp],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        }
        let updated = tx
            .execute(
                "UPDATE providers SET is_current = 1 WHERE id = ?1 AND app_type = ?2 AND category = ?3",
                params![provider_id, app_type, category],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        if updated != 1 {
            return Err(AppError::Database(format!(
                "Failed to set {category} provider current: provider '{provider_id}' not found in app '{app_type}'"
            )));
        }
        tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn is_omo_provider_current(
        &self,
        app_type: &str,
        provider_id: &str,
        category: &str,
    ) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);
        match conn.query_row(
            "SELECT is_current FROM providers
             WHERE id = ?1 AND app_type = ?2 AND category = ?3",
            params![provider_id, app_type, category],
            |row| row.get(0),
        ) {
            Ok(is_current) => Ok(is_current),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    pub fn clear_omo_provider_current(
        &self,
        app_type: &str,
        provider_id: &str,
        category: &str,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "UPDATE providers SET is_current = 0
             WHERE id = ?1 AND app_type = ?2 AND category = ?3",
            params![provider_id, app_type, category],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn get_current_omo_provider(
        &self,
        app_type: &str,
        category: &str,
    ) -> Result<Option<Provider>, AppError> {
        let conn = lock_conn!(self.conn);
        let row_data: Result<OmoProviderRow, rusqlite::Error> = conn.query_row(
            "SELECT id, name, settings_config, category, created_at, sort_index, notes, meta
             FROM providers
             WHERE app_type = ?1 AND category = ?2 AND is_current = 1
             LIMIT 1",
            params![app_type, category],
            |row| {
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
            },
        );

        let (id, name, settings_config_str, _row_category, created_at, sort_index, notes, meta_str) =
            match row_data {
                Ok(v) => v,
                Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
                Err(e) => return Err(AppError::Database(e.to_string())),
            };

        let settings_config = serde_json::from_str(&settings_config_str).map_err(|e| {
            AppError::Database(format!(
                "Failed to parse {category} provider settings_config (provider_id={id}): {e}"
            ))
        })?;
        let meta: crate::provider::ProviderMeta = if meta_str.trim().is_empty() {
            crate::provider::ProviderMeta::default()
        } else {
            serde_json::from_str(&meta_str).map_err(|e| {
                AppError::Database(format!(
                    "Failed to parse {category} provider meta (provider_id={id}): {e}"
                ))
            })?
        };

        Ok(Some(Provider {
            id,
            name,
            settings_config,
            website_url: None,
            category: Some(category.to_string()),
            created_at,
            sort_index,
            notes,
            meta: Some(meta),
            icon: None,
            icon_color: None,
            in_failover_queue: false,
        }))
    }

    /// 判断 providers 表是否为空（全 app_type 一起算）。
    ///
    /// 用于区分"全新安装"和"升级用户"：在启动流程 import/seed 之前调用。
    /// 使用 `EXISTS` 短路查询，比 `COUNT(*)` 在将来表变大时更高效。
    pub fn is_providers_empty(&self) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);
        let exists: bool = conn
            .query_row("SELECT EXISTS(SELECT 1 FROM providers)", [], |row| {
                row.get(0)
            })
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(!exists)
    }

    /// 判断指定 app 下是否已存在任意 provider。
    ///
    /// 启动阶段的 live import 使用这个更严格的判断：
    /// 只要该 app 已经有任何 provider（包括官方 seed），就不应再自动导入 `default`。
    pub fn has_any_provider_for_app(&self, app_type: &str) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM providers WHERE app_type = ?1)",
                params![app_type],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(exists)
    }

    /// 仅获取指定 app 下所有 provider 的 id 集合。
    ///
    /// 比 `get_all_providers` 轻量得多：只读 id 列、无 endpoint 子查询。
    /// 用于只需要做存在性检查的场景（如 additive 模式的 live 同步去重）。
    pub fn get_provider_ids(&self, app_type: &str) -> Result<HashSet<String>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare("SELECT id FROM providers WHERE app_type = ?1")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let rows = stmt
            .query_map(params![app_type], |row| row.get::<_, String>(0))
            .map_err(|e| AppError::Database(e.to_string()))?;
        let mut ids = HashSet::new();
        for row in rows {
            ids.insert(row.map_err(|e| AppError::Database(e.to_string()))?);
        }
        Ok(ids)
    }

    /// 判断指定 app 下是否存在非官方种子的供应商。
    ///
    /// 比 `get_all_providers` 轻量得多：只读 id 列、无 endpoint 子查询、首条命中即返回。
    /// 用于 `import_default_config` 决定是否跳过 live 导入。
    pub fn has_non_official_seed_provider(&self, app_type: &str) -> Result<bool, AppError> {
        use crate::database::dao::providers_seed::is_official_seed_id;
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare("SELECT id FROM providers WHERE app_type = ?1")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let mut rows = stmt
            .query(params![app_type])
            .map_err(|e| AppError::Database(e.to_string()))?;
        while let Some(row) = rows.next().map_err(|e| AppError::Database(e.to_string()))? {
            let id: String = row.get(0).map_err(|e| AppError::Database(e.to_string()))?;
            if !is_official_seed_id(&id) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// 计算指定 app 下一个可用的 sort_index（追加到末尾）。
    fn next_sort_index_for_app(&self, app_type: &str) -> Result<usize, AppError> {
        let conn = lock_conn!(self.conn);
        let max: Option<i64> = conn
            .query_row(
                "SELECT MAX(sort_index) FROM providers WHERE app_type = ?1",
                params![app_type],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(max.map(|v| (v + 1) as usize).unwrap_or(0))
    }

    /// 启动时调用：补齐缺失的官方预设供应商（Claude / Codex / Gemini / Grok Build）。
    ///
    /// 使用 settings flag `official_providers_seeded` 保证每个数据库只执行一次：
    /// - 全新用户：seed 四条官方预设
    /// - 老用户升级：同样会触发一次（flag 不存在），追加到末尾，不影响已有排序
    /// - 用户删除 seed 后：不再重建（flag 已为 true），尊重用户意图
    ///
    /// 与 `Database::save_provider` 的 UPSERT 语义配合，即使被意外重复调用
    /// 也不会覆盖用户当前激活的供应商（is_current 字段会被保留）。
    pub fn init_default_official_providers(&self) -> Result<usize, AppError> {
        use crate::database::dao::providers_seed::OFFICIAL_SEEDS;

        if self
            .get_bool_flag("official_providers_seeded")
            .unwrap_or(false)
        {
            return Ok(0);
        }

        let mut inserted = 0_usize;
        let now_ms = chrono::Utc::now().timestamp_millis();

        for seed in OFFICIAL_SEEDS {
            let app_type_str = seed.app_type.as_str();

            // 若该 id 已存在（极端情况：用户曾手动用过同 id），跳过
            if self.get_provider_by_id(seed.id, app_type_str)?.is_some() {
                continue;
            }

            let next_sort_index = self.next_sort_index_for_app(app_type_str)?;

            let settings_config: serde_json::Value =
                serde_json::from_str(seed.settings_config_json).map_err(|e| {
                    AppError::Database(format!("Seed JSON parse failed for {}: {e}", seed.id))
                })?;

            let mut provider = Provider::with_id(
                seed.id.to_string(),
                seed.name.to_string(),
                settings_config,
                Some(seed.website_url.to_string()),
            );
            provider.category = Some("official".to_string());
            provider.icon = Some(seed.icon.to_string());
            provider.icon_color = Some(seed.icon_color.to_string());
            provider.sort_index = Some(next_sort_index);
            provider.created_at = Some(now_ms);

            self.save_provider(app_type_str, &provider)?;
            inserted += 1;
            log::info!(
                "✓ Seeded official provider: {} ({})",
                seed.name,
                app_type_str
            );
        }

        // 即使 inserted=0（例如用户手动创建过同 id）也设置 flag 防止反复检查
        self.set_setting("official_providers_seeded", "true")?;

        Ok(inserted)
    }

    /// 按 id 兜底插入单条 official seed（仅当目标表中该 id 不存在时插入）。
    ///
    /// 与 `init_default_official_providers` 不同，这个按需修复入口不触碰
    /// `official_providers_seeded` 全局 flag，并且不会覆盖同 id 的用户数据。
    /// 返回 `Ok(true)` 表示新插入，`Ok(false)` 表示目标已存在。
    pub fn ensure_official_seed_by_id(
        &self,
        seed_id: &str,
        app_type: crate::app_config::AppType,
    ) -> Result<bool, AppError> {
        use crate::database::dao::providers_seed::OFFICIAL_SEEDS;

        let seed = OFFICIAL_SEEDS
            .iter()
            .find(|seed| seed.id == seed_id && seed.app_type == app_type)
            .ok_or_else(|| {
                AppError::Database(format!(
                    "unknown official seed: id={seed_id}, app_type={}",
                    app_type.as_str()
                ))
            })?;

        let app_type_str = seed.app_type.as_str();
        if self.get_provider_by_id(seed_id, app_type_str)?.is_some() {
            return Ok(false);
        }

        let settings_config: serde_json::Value = serde_json::from_str(seed.settings_config_json)
            .map_err(|e| {
                AppError::Database(format!("Seed JSON parse failed for {}: {e}", seed.id))
            })?;
        let next_sort_index = self.next_sort_index_for_app(app_type_str)?;

        let mut provider = Provider::with_id(
            seed.id.to_string(),
            seed.name.to_string(),
            settings_config,
            Some(seed.website_url.to_string()),
        );
        provider.category = Some("official".to_string());
        provider.icon = Some(seed.icon.to_string());
        provider.icon_color = Some(seed.icon_color.to_string());
        provider.sort_index = Some(next_sort_index);
        provider.created_at = Some(chrono::Utc::now().timestamp_millis());

        self.save_provider(app_type_str, &provider)?;
        Ok(true)
    }
}

#[cfg(test)]
mod ensure_official_seed_tests {
    use crate::app_config::AppType;
    use crate::database::{Database, GROKBUILD_OFFICIAL_PROVIDER_ID};

    #[test]
    fn ensure_recreates_grokbuild_official_seed_after_deletion() {
        let db = Database::memory().expect("memory db");
        db.init_default_official_providers().expect("seed");
        db.delete_provider(AppType::GrokBuild.as_str(), GROKBUILD_OFFICIAL_PROVIDER_ID)
            .expect("delete Grok Build official");

        let inserted = db
            .ensure_official_seed_by_id(GROKBUILD_OFFICIAL_PROVIDER_ID, AppType::GrokBuild)
            .expect("ensure Grok Build official");
        assert!(inserted);

        let provider = db
            .get_provider_by_id(GROKBUILD_OFFICIAL_PROVIDER_ID, AppType::GrokBuild.as_str())
            .expect("query")
            .expect("Grok Build official restored");
        assert_eq!(provider.category.as_deref(), Some("official"));
        assert_eq!(provider.settings_config["config"], serde_json::json!(""));
    }

    #[test]
    fn ensure_preserves_existing_grokbuild_official_customization() {
        let db = Database::memory().expect("memory db");
        db.init_default_official_providers().expect("seed");

        let mut provider = db
            .get_provider_by_id(GROKBUILD_OFFICIAL_PROVIDER_ID, AppType::GrokBuild.as_str())
            .expect("query")
            .expect("seed exists");
        provider.name = "Renamed Grok Official".to_string();
        db.save_provider(AppType::GrokBuild.as_str(), &provider)
            .expect("rename seed");

        assert!(!db
            .ensure_official_seed_by_id(GROKBUILD_OFFICIAL_PROVIDER_ID, AppType::GrokBuild)
            .expect("ensure existing seed"));
        assert_eq!(
            db.get_provider_by_id(GROKBUILD_OFFICIAL_PROVIDER_ID, AppType::GrokBuild.as_str())
                .expect("query")
                .expect("seed exists")
                .name,
            "Renamed Grok Official"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::CustomEndpoint;
    use serde_json::json;

    fn endpoint(url: &str, added_at: i64) -> CustomEndpoint {
        CustomEndpoint {
            url: url.to_string(),
            added_at,
            last_used: None,
        }
    }

    fn provider_with_endpoints(id: &str, name: &str, endpoints: &[(&str, i64)]) -> Provider {
        let mut provider =
            Provider::with_id(id.to_string(), name.to_string(), json!({"env": {}}), None);
        provider.meta = Some(ProviderMeta {
            custom_endpoints: endpoints
                .iter()
                .map(|(url, added_at)| (url.to_string(), endpoint(url, *added_at)))
                .collect(),
            ..ProviderMeta::default()
        });
        provider
    }

    #[test]
    fn save_provider_reconciles_endpoint_snapshot_on_update() {
        let db = Database::memory().expect("create database");
        let original = provider_with_endpoints(
            "provider-1",
            "Original",
            &[
                ("https://keep.example", 100),
                ("https://remove.example", 200),
            ],
        );
        db.save_provider("claude", &original)
            .expect("save original provider");
        {
            let conn = db.conn.lock().expect("lock database");
            conn.execute(
                "INSERT INTO provider_endpoints (provider_id, app_type, url, added_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params!["provider-1", "claude", "https://keep.example", 500],
            )
            .expect("seed duplicate endpoint row");
        }

        let updated = provider_with_endpoints(
            "provider-1",
            "Updated",
            &[("https://keep.example", 999), ("https://new.example", 300)],
        );
        db.save_provider("claude", &updated)
            .expect("update provider");

        let fresh = db
            .get_provider_by_id("provider-1", "claude")
            .expect("read updated provider")
            .expect("provider exists");
        let endpoints = &fresh.meta.as_ref().expect("provider meta").custom_endpoints;

        assert_eq!(fresh.name, "Updated");
        assert_eq!(endpoints.len(), 2);
        assert_eq!(
            endpoints
                .get("https://keep.example")
                .expect("kept endpoint")
                .added_at,
            100,
            "an existing URL keeps its original added_at"
        );
        assert_eq!(
            endpoints
                .get("https://new.example")
                .expect("new endpoint")
                .added_at,
            300
        );
        assert!(!endpoints.contains_key("https://remove.example"));

        let kept_row_count: i64 = db
            .conn
            .lock()
            .expect("lock database")
            .query_row(
                "SELECT COUNT(*) FROM provider_endpoints
                 WHERE provider_id = ?1 AND app_type = ?2 AND url = ?3",
                params!["provider-1", "claude", "https://keep.example"],
                |row| row.get(0),
            )
            .expect("count kept endpoint rows");
        assert_eq!(kept_row_count, 1, "duplicate URL rows are reconciled");
    }

    #[test]
    fn save_provider_rolls_back_provider_when_endpoint_reconcile_fails() {
        let db = Database::memory().expect("create database");
        let original =
            provider_with_endpoints("provider-1", "Original", &[("https://old.example", 100)]);
        db.save_provider("claude", &original)
            .expect("save original provider");

        {
            let conn = db.conn.lock().expect("lock database");
            conn.execute_batch(
                "CREATE TRIGGER fail_provider_endpoint_insert
                 BEFORE INSERT ON provider_endpoints
                 BEGIN
                     SELECT RAISE(ABORT, 'forced endpoint failure');
                 END;",
            )
            .expect("create failure trigger");
        }

        let updated = provider_with_endpoints(
            "provider-1",
            "Partially updated",
            &[("https://new.example", 200)],
        );
        assert!(db.save_provider("claude", &updated).is_err());

        let fresh = db
            .get_provider_by_id("provider-1", "claude")
            .expect("read provider after rollback")
            .expect("provider exists");
        let endpoints = &fresh.meta.as_ref().expect("provider meta").custom_endpoints;

        assert_eq!(fresh.name, "Original");
        assert_eq!(endpoints.len(), 1);
        assert!(endpoints.contains_key("https://old.example"));
        assert!(!endpoints.contains_key("https://new.example"));
    }

    #[test]
    fn save_provider_without_meta_preserves_existing_endpoints() {
        let db = Database::memory().expect("create database");
        let original = provider_with_endpoints(
            "provider-1",
            "Original",
            &[("https://existing.example", 100)],
        );
        db.save_provider("claude", &original)
            .expect("save original provider");

        let update_without_meta = Provider::with_id(
            "provider-1".to_string(),
            "Updated without meta".to_string(),
            json!({"env": {"ANTHROPIC_API_KEY": "updated"}}),
            None,
        );
        db.save_provider("claude", &update_without_meta)
            .expect("update provider without meta");

        let fresh = db
            .get_provider_by_id("provider-1", "claude")
            .expect("read updated provider")
            .expect("provider exists");
        let endpoints = &fresh.meta.as_ref().expect("provider meta").custom_endpoints;

        assert_eq!(fresh.name, "Updated without meta");
        assert_eq!(endpoints.len(), 1);
        assert_eq!(
            endpoints
                .get("https://existing.example")
                .expect("existing endpoint")
                .added_at,
            100
        );
    }

    #[test]
    fn save_provider_with_empty_endpoint_snapshot_clears_existing_endpoints() {
        let db = Database::memory().expect("create database");
        let original = provider_with_endpoints(
            "provider-1",
            "Original",
            &[("https://existing.example", 100)],
        );
        db.save_provider("claude", &original)
            .expect("save original provider");

        let cleared = provider_with_endpoints("provider-1", "Cleared", &[]);
        db.save_provider("claude", &cleared)
            .expect("clear provider endpoints");

        let fresh = db
            .get_provider_by_id("provider-1", "claude")
            .expect("read updated provider")
            .expect("provider exists");
        assert!(
            fresh
                .meta
                .expect("provider meta")
                .custom_endpoints
                .is_empty(),
            "Some(meta) with an empty endpoint snapshot explicitly clears endpoints"
        );
    }

    #[test]
    fn add_custom_endpoint_is_idempotent_and_collapses_historical_duplicates() {
        let db = Database::memory().expect("create database");
        let provider = Provider::with_id(
            "provider-1".to_string(),
            "Provider".to_string(),
            json!({"env": {}}),
            None,
        );
        db.save_provider("claude", &provider)
            .expect("save provider");

        db.add_custom_endpoint("claude", "provider-1", "https://api.example")
            .expect("add endpoint");
        let original_added_at: i64 = db
            .conn
            .lock()
            .expect("lock database")
            .query_row(
                "SELECT added_at FROM provider_endpoints
                 WHERE provider_id = ?1 AND app_type = ?2 AND url = ?3",
                params!["provider-1", "claude", "https://api.example"],
                |row| row.get(0),
            )
            .expect("read original added_at");

        db.add_custom_endpoint("claude", "provider-1", "https://api.example")
            .expect("add endpoint again");

        {
            let conn = db.conn.lock().expect("lock database");
            let (count, added_at): (i64, i64) = conn
                .query_row(
                    "SELECT COUNT(*), MIN(added_at) FROM provider_endpoints
                     WHERE provider_id = ?1 AND app_type = ?2 AND url = ?3",
                    params!["provider-1", "claude", "https://api.example"],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .expect("read idempotent endpoint state");
            assert_eq!(count, 1);
            assert_eq!(added_at, original_added_at);

            conn.execute(
                "INSERT INTO provider_endpoints (provider_id, app_type, url, added_at)
                 VALUES (?1, ?2, ?3, ?4), (?1, ?2, ?3, NULL)",
                params![
                    "provider-1",
                    "claude",
                    "https://api.example",
                    original_added_at + 1_000
                ],
            )
            .expect("seed historical duplicate rows");
        }

        db.add_custom_endpoint("claude", "provider-1", "https://api.example")
            .expect("reconcile historical duplicates");

        let (count, added_at): (i64, i64) = db
            .conn
            .lock()
            .expect("lock database")
            .query_row(
                "SELECT COUNT(*), MIN(added_at) FROM provider_endpoints
                 WHERE provider_id = ?1 AND app_type = ?2 AND url = ?3",
                params!["provider-1", "claude", "https://api.example"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read reconciled endpoint state");
        assert_eq!(count, 1);
        assert_eq!(added_at, original_added_at);
    }
}
