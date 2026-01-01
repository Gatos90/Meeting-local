//! Tauri commands for chat functionality
//!
//! Re-exports from focused modules for backwards compatibility.

// Re-export all types
pub use super::types::{SendMessageResponse, ChatMessageStatus2};

// Re-export session commands
pub use super::session_commands::{
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
pub use super::message_commands::{
    chat_send_message,
    chat_get_messages,
    chat_get_status,
    chat_cancel_message,
    chat_clear_session,
    chat_delete_history,
    chat_is_processing,
    chat_get_pending_messages,
};
