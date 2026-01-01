//! Chat module for AI conversations about recordings
//!
//! This module provides:
//! - Persistent chat history stored in SQLite
//! - Background processing of LLM requests
//! - Cancellable requests
//! - Provider-agnostic message format
//!
//! Module structure:
//! - types.rs: SendMessageResponse, ChatMessageStatus2
//! - task_registry.rs: ACTIVE_CHAT_TASKS, task management
//! - session_commands.rs: Session CRUD Tauri commands
//! - message_commands.rs: Message operation Tauri commands
//! - completion.rs: run_chat_completion with tool loop

pub mod types;
pub mod task_registry;
pub mod session_commands;
pub mod message_commands;
pub mod completion;
pub mod commands;
pub mod tool_orchestration;

// Re-export types
pub use types::{SendMessageResponse, ChatMessageStatus2};

// Re-export session commands
pub use session_commands::{
    chat_create_session,
    chat_list_sessions,
    chat_get_session,
    chat_get_or_create_session,
    chat_update_session_config,
    chat_update_session_title,
    chat_delete_session,
    chat_get_config,
};

// Re-export message commands
pub use message_commands::{
    chat_send_message,
    chat_get_messages,
    chat_get_status,
    chat_cancel_message,
    chat_clear_session,
    chat_delete_history,
    chat_is_processing,
    chat_get_pending_messages,
};
