// Model configuration repository for Meeting-Local
// Handles CRUD operations for user-defined model settings

use anyhow::{Context, Result};
use rusqlite::{Connection, params};

use super::models::{ModelConfig, UpsertModelConfig};
use super::DatabaseManager;

impl DatabaseManager {
    /// Get the tool support setting for a specific model
    /// Returns None if no custom config exists for this model
    pub fn get_model_tool_support(&self, model_id: &str) -> Result<Option<bool>> {
        self.with_connection(|conn| {
            get_model_tool_support_impl(conn, model_id)
        })
    }

    /// Set whether a model has native tool support
    pub fn set_model_tool_support(&self, model_id: &str, has_support: bool) -> Result<()> {
        self.with_connection(|conn| {
            set_model_tool_support_impl(conn, model_id, has_support)
        })
    }

    /// Get full model config for a specific model
    pub fn get_model_config(&self, model_id: &str) -> Result<Option<ModelConfig>> {
        self.with_connection(|conn| {
            get_model_config_impl(conn, model_id)
        })
    }

    /// Get all model configs
    pub fn get_all_model_configs(&self) -> Result<Vec<ModelConfig>> {
        self.with_connection(|conn| {
            get_all_model_configs_impl(conn)
        })
    }

    /// Upsert a model config (create or update)
    pub fn upsert_model_config(&self, config: UpsertModelConfig) -> Result<()> {
        self.with_connection(|conn| {
            upsert_model_config_impl(conn, &config)
        })
    }

    /// Delete a model config
    pub fn delete_model_config(&self, model_id: &str) -> Result<()> {
        self.with_connection(|conn| {
            delete_model_config_impl(conn, model_id)
        })
    }
}

fn get_model_tool_support_impl(conn: &Connection, model_id: &str) -> Result<Option<bool>> {
    let mut stmt = conn.prepare(
        "SELECT has_native_tool_support FROM model_config WHERE model_id = ?"
    ).context("Failed to prepare get_model_tool_support query")?;

    let result = stmt.query_row(params![model_id], |row| {
        let value: i32 = row.get(0)?;
        Ok(value != 0)
    });

    match result {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("Failed to get model tool support"),
    }
}

fn set_model_tool_support_impl(conn: &Connection, model_id: &str, has_support: bool) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO model_config (model_id, has_native_tool_support, created_at, updated_at)
        VALUES (?1, ?2, datetime('now'), datetime('now'))
        ON CONFLICT(model_id) DO UPDATE SET
            has_native_tool_support = excluded.has_native_tool_support,
            updated_at = datetime('now')
        "#,
        params![model_id, if has_support { 1 } else { 0 }],
    ).context("Failed to set model tool support")?;

    Ok(())
}

fn get_model_config_impl(conn: &Connection, model_id: &str) -> Result<Option<ModelConfig>> {
    let mut stmt = conn.prepare(
        "SELECT model_id, has_native_tool_support, created_at, updated_at FROM model_config WHERE model_id = ?"
    ).context("Failed to prepare get_model_config query")?;

    let result = stmt.query_row(params![model_id], |row| {
        Ok(ModelConfig {
            model_id: row.get(0)?,
            has_native_tool_support: row.get::<_, i32>(1)? != 0,
            created_at: row.get(2)?,
            updated_at: row.get(3)?,
        })
    });

    match result {
        Ok(config) => Ok(Some(config)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("Failed to get model config"),
    }
}

fn get_all_model_configs_impl(conn: &Connection) -> Result<Vec<ModelConfig>> {
    let mut stmt = conn.prepare(
        "SELECT model_id, has_native_tool_support, created_at, updated_at FROM model_config ORDER BY model_id"
    ).context("Failed to prepare get_all_model_configs query")?;

    let configs = stmt.query_map([], |row| {
        Ok(ModelConfig {
            model_id: row.get(0)?,
            has_native_tool_support: row.get::<_, i32>(1)? != 0,
            created_at: row.get(2)?,
            updated_at: row.get(3)?,
        })
    }).context("Failed to query model configs")?;

    configs.collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to collect model configs")
}

fn upsert_model_config_impl(conn: &Connection, config: &UpsertModelConfig) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO model_config (model_id, has_native_tool_support, created_at, updated_at)
        VALUES (?1, ?2, datetime('now'), datetime('now'))
        ON CONFLICT(model_id) DO UPDATE SET
            has_native_tool_support = excluded.has_native_tool_support,
            updated_at = datetime('now')
        "#,
        params![config.model_id, if config.has_native_tool_support { 1 } else { 0 }],
    ).context("Failed to upsert model config")?;

    Ok(())
}

fn delete_model_config_impl(conn: &Connection, model_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM model_config WHERE model_id = ?",
        params![model_id],
    ).context("Failed to delete model config")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_db() -> DatabaseManager {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        DatabaseManager::new(db_path).unwrap()
    }

    #[test]
    fn test_set_and_get_tool_support() {
        let db = create_test_db();

        // Initially should be None
        let support = db.get_model_tool_support("test-model").unwrap();
        assert!(support.is_none());

        // Set to true
        db.set_model_tool_support("test-model", true).unwrap();
        let support = db.get_model_tool_support("test-model").unwrap();
        assert_eq!(support, Some(true));

        // Update to false
        db.set_model_tool_support("test-model", false).unwrap();
        let support = db.get_model_tool_support("test-model").unwrap();
        assert_eq!(support, Some(false));
    }

    #[test]
    fn test_get_all_model_configs() {
        let db = create_test_db();

        db.set_model_tool_support("model-a", true).unwrap();
        db.set_model_tool_support("model-b", false).unwrap();

        let configs = db.get_all_model_configs().unwrap();
        assert_eq!(configs.len(), 2);
    }

    #[test]
    fn test_delete_model_config() {
        let db = create_test_db();

        db.set_model_tool_support("test-model", true).unwrap();
        assert!(db.get_model_tool_support("test-model").unwrap().is_some());

        db.delete_model_config("test-model").unwrap();
        assert!(db.get_model_tool_support("test-model").unwrap().is_none());
    }
}
