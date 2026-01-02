// Database models - Re-exports all domain-specific models
//
// This module is split into focused files by domain:
// - settings.rs: Application settings
// - recording.rs: Recording/meeting data
// - transcript.rs: Transcript segments and speaker data
// - category_tag.rs: Categories and tags for organization
// - chat.rs: Chat sessions and messages
// - template.rs: Prompt templates
// - tool.rs: AI tools
// - mcp.rs: MCP server configuration

mod settings;
mod recording;
mod transcript;
mod category_tag;
mod chat;
mod template;
mod tool;
mod mcp;
mod model_config;

// Re-export all public types for backwards compatibility
pub use settings::{Setting, AllSettings};
pub use recording::{Recording, RecordingUpdate, RecordingWithMetadata};
pub use transcript::{TranscriptSegment, RegisteredSpeakerDb, SpeakerLabel};
pub use category_tag::{Category, Tag, SearchResult, SearchFilters};
pub use chat::{
    ChatRole, ChatMessageStatus, ChatMessage, ChatConfig, ChatSession, DefaultLlmConfig,
};
pub use template::{PromptTemplate, CreatePromptTemplate, UpdatePromptTemplate};
pub use tool::{
    ToolType, ToolExecutionLocation, Tool, CreateTool, UpdateTool,
    ChatSessionTool, ToolCall, ToolResult,
};
pub use mcp::{
    McpServerStatus, McpServer, CreateMcpServer, UpdateMcpServer,
    McpServerConfig, McpServerWithTools,
};
pub use model_config::{ModelConfig, UpsertModelConfig};
