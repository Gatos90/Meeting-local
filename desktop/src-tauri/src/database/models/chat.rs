// Database models - Chat
use serde::{Deserialize, Serialize};

/// Chat message role (compatible with OpenAI, Ollama, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

impl ChatRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChatRole::System => "system",
            ChatRole::User => "user",
            ChatRole::Assistant => "assistant",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "system" => ChatRole::System,
            "user" => ChatRole::User,
            "assistant" => ChatRole::Assistant,
            _ => ChatRole::User,
        }
    }
}

/// Chat message status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChatMessageStatus {
    Pending,
    Streaming,
    Complete,
    Cancelled,
    Error,
}

impl ChatMessageStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChatMessageStatus::Pending => "pending",
            ChatMessageStatus::Streaming => "streaming",
            ChatMessageStatus::Complete => "complete",
            ChatMessageStatus::Cancelled => "cancelled",
            ChatMessageStatus::Error => "error",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pending" => ChatMessageStatus::Pending,
            "streaming" => ChatMessageStatus::Streaming,
            "complete" => ChatMessageStatus::Complete,
            "cancelled" => ChatMessageStatus::Cancelled,
            "error" => ChatMessageStatus::Error,
            _ => ChatMessageStatus::Complete,
        }
    }
}

/// A chat message in a conversation about a recording
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub recording_id: String,
    /// The session this message belongs to
    #[serde(default)]
    pub session_id: Option<String>,
    pub role: ChatRole,
    pub content: String,
    pub created_at: String,
    pub sequence_id: i64,
    pub status: ChatMessageStatus,
    pub error_message: Option<String>,
    /// The LLM provider used (e.g., "ollama", "embedded")
    #[serde(default)]
    pub provider_type: Option<String>,
    /// The model ID used (e.g., "llama3.2", "mistral-7b")
    #[serde(default)]
    pub model_id: Option<String>,
}

impl ChatMessage {
    /// Create a new user message for a session
    pub fn user(session_id: &str, recording_id: &str, content: &str, sequence_id: i64) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            recording_id: recording_id.to_string(),
            session_id: Some(session_id.to_string()),
            role: ChatRole::User,
            content: content.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            sequence_id,
            status: ChatMessageStatus::Complete,
            error_message: None,
            provider_type: None,
            model_id: None,
        }
    }

    /// Create a new assistant message (pending response) with provider/model info
    pub fn assistant_pending(
        session_id: &str,
        recording_id: &str,
        sequence_id: i64,
        provider_type: Option<String>,
        model_id: Option<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            recording_id: recording_id.to_string(),
            session_id: Some(session_id.to_string()),
            role: ChatRole::Assistant,
            content: String::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            sequence_id,
            status: ChatMessageStatus::Pending,
            error_message: None,
            provider_type,
            model_id,
        }
    }

    /// Create a system message (for transcript context)
    pub fn system(session_id: &str, recording_id: &str, content: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            recording_id: recording_id.to_string(),
            session_id: Some(session_id.to_string()),
            role: ChatRole::System,
            content: content.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            sequence_id: 0,
            status: ChatMessageStatus::Complete,
            error_message: None,
            provider_type: None,
            model_id: None,
        }
    }
}

/// Chat configuration for a recording (provider/model used)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConfig {
    pub provider_type: Option<String>,
    pub model_id: Option<String>,
}

/// A chat session - groups chat messages into separate conversations per recording
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: String,
    pub recording_id: String,
    pub title: String,
    pub created_at: String,
    pub provider_type: Option<String>,
    pub model_id: Option<String>,
}

impl ChatSession {
    /// Create a new chat session with a default title
    pub fn new(recording_id: &str, title: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            recording_id: recording_id.to_string(),
            title: title.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            provider_type: None,
            model_id: None,
        }
    }

    /// Create a new session with provider/model config
    pub fn new_with_config(
        recording_id: &str,
        title: &str,
        provider_type: Option<String>,
        model_id: Option<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            recording_id: recording_id.to_string(),
            title: title.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            provider_type,
            model_id,
        }
    }
}

/// Default LLM model configuration (stored in settings)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultLlmConfig {
    pub provider_type: Option<String>,
    pub model_id: Option<String>,
}
