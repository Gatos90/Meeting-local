// Database module for Meeting-Local
// Provides SQLite persistence for settings, recordings, transcripts, categories, and tags

pub mod manager;
pub mod migrations;
pub mod models;
pub mod settings_repo;
pub mod recordings_repo;
pub mod transcripts_repo;
pub mod categories_repo;
pub mod search;
pub mod chat_repo;
pub mod chat_session_repo;
pub mod template_repo;
pub mod tools_repo;
pub mod mcp_repo;
pub mod model_config_repo;

pub use manager::DatabaseManager;
pub use models::*;
