// Database Manager for Meeting-Local
// Handles SQLite connection and provides access to repositories

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;

use super::migrations;

/// Database manager that owns the SQLite connection
pub struct DatabaseManager {
    conn: Mutex<Connection>,
    db_path: PathBuf,
}

impl DatabaseManager {
    /// Create a new DatabaseManager with the database at the specified path
    pub fn new(db_path: PathBuf) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create database directory")?;
        }

        let conn = Connection::open(&db_path)
            .context("Failed to open database")?;

        // Enable foreign keys
        conn.execute("PRAGMA foreign_keys = ON", [])
            .context("Failed to enable foreign keys")?;

        // Run migrations
        migrations::run_migrations(&conn)
            .context("Failed to run database migrations")?;

        log::info!("Database initialized at: {:?}", db_path);

        Ok(Self {
            conn: Mutex::new(conn),
            db_path,
        })
    }

    /// Initialize the database manager using Tauri's app data directory
    pub fn init_with_app_handle<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<Self> {
        use tauri::path::PathResolver;

        let app_data_dir = app.path()
            .app_data_dir()
            .context("Failed to get app data directory")?;

        let db_path = app_data_dir.join("meetlocal.db");
        Self::new(db_path)
    }

    /// Execute a function with access to the database connection
    pub fn with_connection<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> Result<T>,
    {
        let conn = self.conn.lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock database connection: {}", e))?;
        f(&conn)
    }

    /// Get the database path
    pub fn db_path(&self) -> &PathBuf {
        &self.db_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_database_creation() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let manager = DatabaseManager::new(db_path.clone()).unwrap();
        assert!(db_path.exists());

        // Test that we can access the connection
        manager.with_connection(|conn| {
            let count: i32 = conn.query_row(
                "SELECT COUNT(*) FROM settings",
                [],
                |row| row.get(0),
            )?;
            assert_eq!(count, 0);
            Ok(())
        }).unwrap();
    }
}
