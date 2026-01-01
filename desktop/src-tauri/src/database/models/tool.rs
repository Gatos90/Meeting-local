// Database models - AI Tools
use serde::{Deserialize, Serialize};

/// Tool type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
    Builtin,
    Custom,
    Mcp,
}

impl ToolType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ToolType::Builtin => "builtin",
            ToolType::Custom => "custom",
            ToolType::Mcp => "mcp",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "builtin" => ToolType::Builtin,
            "custom" => ToolType::Custom,
            "mcp" => ToolType::Mcp,
            _ => ToolType::Custom,
        }
    }
}

/// Tool execution location
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ToolExecutionLocation {
    Backend,
    Frontend,
}

impl ToolExecutionLocation {
    pub fn as_str(&self) -> &'static str {
        match self {
            ToolExecutionLocation::Backend => "backend",
            ToolExecutionLocation::Frontend => "frontend",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "backend" => ToolExecutionLocation::Backend,
            "frontend" => ToolExecutionLocation::Frontend,
            _ => ToolExecutionLocation::Backend,
        }
    }
}

/// An AI tool that can be called during chat conversations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub tool_type: String,
    /// JSON string containing the function schema
    pub function_schema: String,
    pub execution_location: String,
    pub enabled: bool,
    pub is_default: bool,
    pub icon: Option<String>,
    pub sort_order: i32,
    pub created_at: String,
    /// MCP server ID (for MCP tools only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_server_id: Option<String>,
    /// MCP server name (for MCP tools, joined from mcp_servers table)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_server_name: Option<String>,
}

impl Tool {
    /// Create a new custom tool
    pub fn new_custom(
        name: &str,
        description: Option<String>,
        function_schema: &str,
        icon: Option<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            description,
            tool_type: ToolType::Custom.as_str().to_string(),
            function_schema: function_schema.to_string(),
            execution_location: ToolExecutionLocation::Backend.as_str().to_string(),
            enabled: true,
            is_default: false,
            icon,
            sort_order: 0,
            created_at: chrono::Utc::now().to_rfc3339(),
            mcp_server_id: None,
            mcp_server_name: None,
        }
    }
}

/// Input for creating a new tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTool {
    pub name: String,
    pub description: Option<String>,
    pub function_schema: String,
    pub execution_location: Option<String>,
    pub icon: Option<String>,
}

/// Input for updating a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTool {
    pub name: Option<String>,
    pub description: Option<String>,
    pub function_schema: Option<String>,
    pub execution_location: Option<String>,
    pub enabled: Option<bool>,
    pub is_default: Option<bool>,
    pub icon: Option<String>,
    pub sort_order: Option<i32>,
}

/// Association between a chat session and a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSessionTool {
    pub session_id: String,
    pub tool_id: String,
    pub enabled: bool,
}

/// Tool call request from the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String, // JSON string
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub content: String,
    pub success: bool,
    pub error: Option<String>,
}
