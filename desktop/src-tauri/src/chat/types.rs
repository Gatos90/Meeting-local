//! Chat types and structures

use serde::{Deserialize, Serialize};

/// Response when sending a chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageResponse {
    pub user_message_id: String,
    pub assistant_message_id: String,
}

/// Chat message status for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageStatus2 {
    pub message_id: String,
    pub status: String,
    pub content: String,
    pub error_message: Option<String>,
}
