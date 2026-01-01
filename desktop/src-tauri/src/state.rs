// State management for Meeting-Local

use std::sync::Arc;
use tokio::sync::RwLock;
use crate::database::DatabaseManager;
use crate::llm_engine::engine::LlmEngine;
use crate::llm_engine::model_manager::LlmModelManager;
use crate::mcp::McpManager;

/// Wrapper around DatabaseManager for shared access
pub struct DbWrapper {
    inner: Arc<DatabaseManager>,
}

impl DbWrapper {
    pub fn new(db: DatabaseManager) -> Self {
        Self {
            inner: Arc::new(db),
        }
    }

    pub fn inner(&self) -> &DatabaseManager {
        &self.inner
    }

    pub fn arc(&self) -> Arc<DatabaseManager> {
        self.inner.clone()
    }
}

impl std::ops::Deref for DbWrapper {
    type Target = DatabaseManager;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct AppState {
    /// Database manager for SQLite persistence
    database: Arc<RwLock<Option<DbWrapper>>>,
    /// LLM engine for AI-powered transcript processing
    pub llm_engine: Arc<RwLock<LlmEngine>>,
    /// LLM model manager for GGUF model downloads
    pub llm_model_manager: Arc<RwLock<LlmModelManager>>,
    /// MCP server manager
    mcp_manager: Arc<RwLock<Option<McpManager>>>,
}

impl AppState {
    pub fn new() -> Self {
        // Use a default path for model manager - will be updated in app setup
        let default_models_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("meeting-local");

        Self {
            database: Arc::new(RwLock::new(None)),
            llm_engine: Arc::new(RwLock::new(LlmEngine::new())),
            llm_model_manager: Arc::new(RwLock::new(LlmModelManager::new(default_models_dir))),
            mcp_manager: Arc::new(RwLock::new(None)),
        }
    }

    /// Update the LLM model manager with the correct app data directory
    pub async fn init_llm_model_manager(&self, app_data_dir: std::path::PathBuf) {
        let mut manager = self.llm_model_manager.write().await;
        *manager = LlmModelManager::new(app_data_dir);
    }

    /// Initialize the database manager and MCP manager
    pub async fn init_database(&self, db: DatabaseManager) {
        let wrapper = DbWrapper::new(db);
        let db_arc = wrapper.arc();

        // Initialize MCP manager with database reference
        let mcp = McpManager::new(db_arc);
        {
            let mut mcp_guard = self.mcp_manager.write().await;
            *mcp_guard = Some(mcp);
        }

        let mut guard = self.database.write().await;
        *guard = Some(wrapper);
    }

    /// Get the database Arc for cloning (used by background tasks)
    pub fn database_arc(&self) -> Arc<RwLock<Option<DbWrapper>>> {
        self.database.clone()
    }

    /// Get the database manager, panicking if not initialized
    /// Use this only when you're sure the database is initialized
    pub async fn db(&self) -> impl std::ops::Deref<Target = DatabaseManager> + '_ {
        let guard = self.database.read().await;
        tokio::sync::RwLockReadGuard::map(guard, |opt| {
            opt.as_ref().expect("Database not initialized").inner()
        })
    }

    /// Get the MCP manager
    pub async fn mcp(&self) -> impl std::ops::Deref<Target = McpManager> + '_ {
        let guard = self.mcp_manager.read().await;
        tokio::sync::RwLockReadGuard::map(guard, |opt| {
            opt.as_ref().expect("MCP manager not initialized")
        })
    }

    /// Get the MCP manager Arc for async operations
    pub fn mcp_manager_arc(&self) -> Arc<RwLock<Option<McpManager>>> {
        self.mcp_manager.clone()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
