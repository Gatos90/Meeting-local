//! LLM Provider trait and types
//!
//! Defines the common interface for all LLM backends (embedded, Ollama, OpenAI, Claude)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Error types for LLM operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmError {
    /// Model not found or not downloaded
    ModelNotFound(String),
    /// Model failed to load
    ModelLoadFailed(String),
    /// Provider not available (e.g., Ollama not running)
    ProviderUnavailable(String),
    /// API key missing or invalid
    AuthenticationFailed(String),
    /// Request failed (network, timeout, etc.)
    RequestFailed(String),
    /// Invalid request parameters
    InvalidRequest(String),
    /// Model download failed
    DownloadFailed(String),
    /// Inference/completion failed
    InferenceFailed(String),
    /// Provider not initialized
    NotInitialized,
    /// Generic error
    Other(String),
}

impl fmt::Display for LlmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmError::ModelNotFound(msg) => write!(f, "Model not found: {}", msg),
            LlmError::ModelLoadFailed(msg) => write!(f, "Failed to load model: {}", msg),
            LlmError::ProviderUnavailable(msg) => write!(f, "Provider unavailable: {}", msg),
            LlmError::AuthenticationFailed(msg) => write!(f, "Authentication failed: {}", msg),
            LlmError::RequestFailed(msg) => write!(f, "Request failed: {}", msg),
            LlmError::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            LlmError::DownloadFailed(msg) => write!(f, "Download failed: {}", msg),
            LlmError::InferenceFailed(msg) => write!(f, "Inference failed: {}", msg),
            LlmError::NotInitialized => write!(f, "Provider not initialized"),
            LlmError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for LlmError {}

/// Role of a message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    /// Tool calls made by the assistant (when role is Assistant)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Tool call ID (when role is Tool - the result of a tool call)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// A tool call requested by the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub function: FunctionCall,
}

/// Function call details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String, // JSON string
}

/// Tool definition to pass to the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value, // JSON Schema
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant_with_tool_calls(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }

    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Tool,
            content: content.into(),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

/// Request for text completion/generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// Conversation messages
    pub messages: Vec<Message>,
    /// Maximum tokens to generate (None = model default)
    pub max_tokens: Option<u32>,
    /// Temperature for sampling (0.0 = deterministic, 1.0+ = creative)
    pub temperature: Option<f32>,
    /// Top-p nucleus sampling
    pub top_p: Option<f32>,
    /// Stop sequences
    pub stop: Option<Vec<String>>,
    /// Whether to stream the response
    pub stream: bool,
    /// Tools available for the LLM to call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    /// Tool choice: "auto", "none", or "required"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,
}

impl Default for CompletionRequest {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            max_tokens: None,
            temperature: Some(0.7),
            top_p: None,
            stop: None,
            stream: false,
            tools: None,
            tool_choice: None,
        }
    }
}

impl CompletionRequest {
    pub fn new(messages: Vec<Message>) -> Self {
        Self {
            messages,
            ..Default::default()
        }
    }

    pub fn with_system_and_user(system: impl Into<String>, user: impl Into<String>) -> Self {
        Self::new(vec![Message::system(system), Message::user(user)])
    }
}

/// Response from a completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// Generated text content
    pub content: String,
    /// Model that generated the response
    pub model: String,
    /// Number of tokens in the prompt
    pub prompt_tokens: Option<u32>,
    /// Number of tokens generated
    pub completion_tokens: Option<u32>,
    /// Whether the response was truncated (hit max_tokens)
    pub truncated: bool,
    /// Finish reason (stop, length, tool_calls, etc.)
    pub finish_reason: Option<String>,
    /// Tool calls requested by the LLM (when finish_reason is "tool_calls")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Information about an available model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmModelInfo {
    /// Unique identifier for the model
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Model description
    pub description: Option<String>,
    /// Model size in bytes (for local models)
    pub size_bytes: Option<u64>,
    /// Whether the model is downloaded/available locally
    pub is_local: bool,
    /// Whether the model is currently loaded in memory
    pub is_loaded: bool,
    /// Context window size (max tokens)
    pub context_length: Option<u32>,
    /// Provider this model belongs to
    pub provider: String,
}

/// Capabilities of an LLM provider
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderCapabilities {
    /// Supports streaming responses
    pub streaming: bool,
    /// Supports chat/conversation format
    pub chat: bool,
    /// Supports function/tool calling
    pub function_calling: bool,
    /// Supports vision/image input
    pub vision: bool,
    /// Is an embedded/local provider (no network required)
    pub embedded: bool,
    /// Requires API key
    pub requires_api_key: bool,
    /// Supports model downloading
    pub supports_download: bool,
}

/// Callback for streaming responses
pub type StreamCallback = Box<dyn Fn(String) + Send + Sync>;

/// The main trait that all LLM providers must implement
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Get the provider name (e.g., "mistral.rs", "ollama", "openai")
    fn provider_name(&self) -> &'static str;

    /// Get provider capabilities
    fn capabilities(&self) -> ProviderCapabilities;

    /// List available models for this provider
    async fn list_models(&self) -> Result<Vec<LlmModelInfo>, LlmError>;

    /// Check if the provider is ready (server running, model loaded, etc.)
    async fn is_ready(&self) -> bool;

    /// Initialize the provider with a specific model
    async fn initialize(&self, model_id: &str) -> Result<(), LlmError>;

    /// Get the currently loaded model ID
    async fn current_model(&self) -> Option<String>;

    /// Run a completion request (non-streaming)
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError>;

    /// Run a completion request with streaming
    /// The callback is called for each token/chunk received
    /// Optional cancel_token allows cancelling the stream mid-generation
    async fn complete_streaming(
        &self,
        request: CompletionRequest,
        callback: StreamCallback,
        cancel_token: Option<tokio_util::sync::CancellationToken>,
    ) -> Result<CompletionResponse, LlmError>;

    /// Shutdown the provider and release resources
    async fn shutdown(&self) -> Result<(), LlmError>;
}

/// Provider type enum for serialization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    Embedded,
    Ollama,
    OpenAi,
    Claude,
}

impl fmt::Display for ProviderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderType::Embedded => write!(f, "Embedded (mistral.rs)"),
            ProviderType::Ollama => write!(f, "Ollama"),
            ProviderType::OpenAi => write!(f, "OpenAI"),
            ProviderType::Claude => write!(f, "Claude"),
        }
    }
}
