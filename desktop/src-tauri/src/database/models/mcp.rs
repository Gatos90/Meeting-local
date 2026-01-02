// Database models - MCP (Model Context Protocol) Server
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MCP server status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum McpServerStatus {
    Stopped,
    Starting,
    Running,
    Error,
}

impl McpServerStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            McpServerStatus::Stopped => "stopped",
            McpServerStatus::Starting => "starting",
            McpServerStatus::Running => "running",
            McpServerStatus::Error => "error",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "stopped" => McpServerStatus::Stopped,
            "starting" => McpServerStatus::Starting,
            "running" => McpServerStatus::Running,
            "error" => McpServerStatus::Error,
            _ => McpServerStatus::Stopped,
        }
    }
}

/// An MCP (Model Context Protocol) server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub id: String,
    pub name: String,
    pub command: String,
    /// JSON array of command arguments
    pub args: String,
    /// JSON object of environment variables
    pub env: String,
    pub working_directory: Option<String>,
    pub auto_start: bool,
    pub enabled: bool,
    pub status: String,
    pub last_error: Option<String>,
    pub created_at: String,
}

impl McpServer {
    /// Create a new MCP server
    pub fn new(
        name: &str,
        command: &str,
        args: Vec<String>,
        env: HashMap<String, String>,
        working_directory: Option<String>,
        auto_start: bool,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            command: command.to_string(),
            args: serde_json::to_string(&args).unwrap_or_else(|_| "[]".to_string()),
            env: serde_json::to_string(&env).unwrap_or_else(|_| "{}".to_string()),
            working_directory,
            auto_start,
            enabled: true,
            status: McpServerStatus::Stopped.as_str().to_string(),
            last_error: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Get args as a Vec<String>
    pub fn get_args(&self) -> Vec<String> {
        serde_json::from_str(&self.args).unwrap_or_default()
    }

    /// Get env as a HashMap
    pub fn get_env(&self) -> HashMap<String, String> {
        serde_json::from_str(&self.env).unwrap_or_default()
    }
}

/// Input for creating a new MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMcpServer {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub working_directory: Option<String>,
    pub auto_start: bool,
}

/// Input for updating an MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMcpServer {
    pub name: Option<String>,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
    pub working_directory: Option<String>,
    pub auto_start: Option<bool>,
    pub enabled: Option<bool>,
}

/// Standard MCP server config format for import (from claude_desktop_config.json, etc.)
/// Format: { "server_name": { "command": "...", "args": [...], "env": {...} } }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
    #[serde(default)]
    pub working_directory: Option<String>,
}

/// MCP server with its discovered tools count
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerWithTools {
    #[serde(flatten)]
    pub server: McpServer,
    pub tool_count: i32,
}
