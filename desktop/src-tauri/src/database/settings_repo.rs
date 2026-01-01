// Settings repository for Meeting-Local
// Handles CRUD operations for application settings

use anyhow::{Context, Result};
use rusqlite::{Connection, params};

use super::models::{AllSettings, Setting};
use super::DatabaseManager;

impl DatabaseManager {
    /// Get a single setting by key
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        self.with_connection(|conn| {
            get_setting_impl(conn, key)
        })
    }

    /// Set a single setting
    pub fn set_setting(&self, key: &str, value: &str, value_type: &str) -> Result<()> {
        self.with_connection(|conn| {
            set_setting_impl(conn, key, value, value_type)
        })
    }

    /// Get all settings
    pub fn get_all_settings_list(&self) -> Result<Vec<Setting>> {
        self.with_connection(|conn| {
            get_all_settings_impl(conn)
        })
    }

    /// Load all settings as a structured object
    pub fn load_all_settings(&self) -> Result<AllSettings> {
        self.with_connection(|conn| {
            load_all_settings_impl(conn)
        })
    }

    /// Set a boolean setting
    pub fn set_bool_setting(&self, key: &str, value: bool) -> Result<()> {
        self.set_setting(key, if value { "true" } else { "false" }, "boolean")
    }

    /// Get a boolean setting
    pub fn get_bool_setting(&self, key: &str, default: bool) -> Result<bool> {
        match self.get_setting(key)? {
            Some(v) => Ok(v == "true"),
            None => Ok(default),
        }
    }

    /// Delete a setting by key
    pub fn delete_setting(&self, key: &str) -> Result<()> {
        self.with_connection(|conn| {
            delete_setting_impl(conn, key)
        })
    }
}

fn get_setting_impl(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare(
        "SELECT value FROM settings WHERE key = ?"
    ).context("Failed to prepare get_setting query")?;

    let result = stmt.query_row(params![key], |row| row.get(0));

    match result {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("Failed to get setting"),
    }
}

fn set_setting_impl(conn: &Connection, key: &str, value: &str, value_type: &str) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO settings (key, value, value_type, updated_at)
        VALUES (?1, ?2, ?3, datetime('now'))
        ON CONFLICT(key) DO UPDATE SET
            value = excluded.value,
            value_type = excluded.value_type,
            updated_at = datetime('now')
        "#,
        params![key, value, value_type],
    ).context("Failed to set setting")?;

    Ok(())
}

fn get_all_settings_impl(conn: &Connection) -> Result<Vec<Setting>> {
    let mut stmt = conn.prepare(
        "SELECT key, value, value_type, updated_at FROM settings"
    ).context("Failed to prepare get_all_settings query")?;

    let settings = stmt.query_map([], |row| {
        Ok(Setting {
            key: row.get(0)?,
            value: row.get(1)?,
            value_type: row.get(2)?,
            updated_at: row.get(3)?,
        })
    }).context("Failed to query settings")?;

    settings.collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to collect settings")
}

fn load_all_settings_impl(conn: &Connection) -> Result<AllSettings> {
    let mut settings = AllSettings::default();

    let mut stmt = conn.prepare(
        "SELECT key, value, value_type FROM settings"
    ).context("Failed to prepare load_all_settings query")?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    }).context("Failed to query settings")?;

    for row in rows {
        let (key, value, value_type) = row.context("Failed to read setting row")?;

        match key.as_str() {
            "language" => settings.language = Some(value),
            "mic_rnnoise" => settings.mic_rnnoise = value == "true",
            "mic_highpass" => settings.mic_highpass = value == "true",
            "mic_normalizer" => settings.mic_normalizer = value == "true",
            "sys_rnnoise" => settings.sys_rnnoise = value == "true",
            "sys_highpass" => settings.sys_highpass = value == "true",
            "sys_normalizer" => settings.sys_normalizer = value == "true",
            "last_microphone" => settings.last_microphone = Some(value),
            "last_system_audio" => settings.last_system_audio = Some(value),
            "recordings_folder" => settings.recordings_folder = Some(value),
            "current_model" => settings.current_model = Some(value),
            _ => {
                log::debug!("Unknown setting key: {}", key);
            }
        }
    }

    // Set defaults for boolean settings if not in database
    // (mic_highpass, mic_normalizer, sys_highpass, sys_normalizer default to true)
    if get_setting_impl(conn, "mic_highpass")?.is_none() {
        settings.mic_highpass = true;
    }
    if get_setting_impl(conn, "mic_normalizer")?.is_none() {
        settings.mic_normalizer = true;
    }
    if get_setting_impl(conn, "sys_highpass")?.is_none() {
        settings.sys_highpass = true;
    }
    if get_setting_impl(conn, "sys_normalizer")?.is_none() {
        settings.sys_normalizer = true;
    }

    Ok(settings)
}

fn delete_setting_impl(conn: &Connection, key: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM settings WHERE key = ?",
        params![key],
    ).context("Failed to delete setting")?;

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
    fn test_set_and_get_setting() {
        let db = create_test_db();

        db.set_setting("test_key", "test_value", "string").unwrap();
        let value = db.get_setting("test_key").unwrap();
        assert_eq!(value, Some("test_value".to_string()));
    }

    #[test]
    fn test_bool_setting() {
        let db = create_test_db();

        db.set_bool_setting("test_bool", true).unwrap();
        assert_eq!(db.get_bool_setting("test_bool", false).unwrap(), true);

        db.set_bool_setting("test_bool", false).unwrap();
        assert_eq!(db.get_bool_setting("test_bool", true).unwrap(), false);
    }

    #[test]
    fn test_load_all_settings() {
        let db = create_test_db();

        db.set_setting("language", "en", "string").unwrap();
        db.set_bool_setting("mic_rnnoise", true).unwrap();

        let settings = db.load_all_settings().unwrap();
        assert_eq!(settings.language, Some("en".to_string()));
        assert_eq!(settings.mic_rnnoise, true);
        // Defaults
        assert_eq!(settings.mic_highpass, true);
        assert_eq!(settings.mic_normalizer, true);
    }
}
