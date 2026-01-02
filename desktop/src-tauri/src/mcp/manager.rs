// MCP Manager - Handles MCP server lifecycle and tool discovery
//
// Responsibilities:
// - Start/stop MCP servers
// - Manage client connections
// - Discover and register tools
// - Route tool calls to appropriate servers

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::database::models::{McpServer, McpServerStatus, Tool};
use crate::database::DatabaseManager;

use super::client::McpClient;

/// MCP Manager handles lifecycle and communication with MCP servers
pub struct McpManager {
    /// Active MCP clients by server ID
    clients: Arc<RwLock<HashMap<String, McpClient>>>,
    /// Database manager for persistence
    db: Arc<DatabaseManager>,
}

impl McpManager {
    /// Create a new MCP manager
    pub fn new(db: Arc<DatabaseManager>) -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            db,
        }
    }

    /// Start an MCP server and discover its tools
    pub async fn start_server(&self, server_id: &str) -> Result<Vec<Tool>> {
        // Get server config from database
        let server = self
            .db
            .get_mcp_server(server_id)?
            .ok_or_else(|| anyhow!("MCP server not found: {}", server_id))?;

        // Check if already running
        {
            let clients = self.clients.read().await;
            if clients.contains_key(server_id) {
                return Err(anyhow!("MCP server '{}' is already running", server.name));
            }
        }

        // Update status to starting
        self.db
            .update_mcp_server_status(server_id, McpServerStatus::Starting, None)?;

        // Spawn the server
        let mut client = match McpClient::spawn(&server).await {
            Ok(c) => c,
            Err(e) => {
                let error_msg = format!("Failed to spawn: {}", e);
                self.db.update_mcp_server_status(
                    server_id,
                    McpServerStatus::Error,
                    Some(error_msg.clone()),
                )?;
                return Err(anyhow!(error_msg));
            }
        };

        // Initialize the connection
        if let Err(e) = client.initialize().await {
            let error_msg = format!("Failed to initialize: {}", e);
            let _ = client.shutdown().await;
            self.db.update_mcp_server_status(
                server_id,
                McpServerStatus::Error,
                Some(error_msg.clone()),
            )?;
            return Err(anyhow!(error_msg));
        }

        // Discover tools
        let mcp_tools = match client.list_tools().await {
            Ok(tools) => tools,
            Err(e) => {
                let error_msg = format!("Failed to list tools: {}", e);
                let _ = client.shutdown().await;
                self.db.update_mcp_server_status(
                    server_id,
                    McpServerStatus::Error,
                    Some(error_msg.clone()),
                )?;
                return Err(anyhow!(error_msg));
            }
        };

        // Delete existing tools for this server (for refresh)
        self.db.delete_mcp_server_tools(server_id)?;

        // Register discovered tools
        let mut registered_tools = Vec::new();
        for mcp_tool in &mcp_tools {
            // Convert input_schema to function_schema format
            let function_schema = serde_json::json!({
                "name": mcp_tool.name,
                "description": mcp_tool.description.clone().unwrap_or_default(),
                "parameters": mcp_tool.input_schema.clone().unwrap_or(serde_json::json!({
                    "type": "object",
                    "properties": {}
                }))
            });

            let tool_id = self.db.create_mcp_tool(
                server_id,
                &mcp_tool.name,
                mcp_tool.description.clone(),
                &function_schema.to_string(),
            )?;

            // Get the created tool
            if let Some(tool) = self.db.get_tool(&tool_id)? {
                registered_tools.push(tool);
            }
        }

        // Store the client
        {
            let mut clients = self.clients.write().await;
            clients.insert(server_id.to_string(), client);
        }

        // Update status to running
        self.db
            .update_mcp_server_status(server_id, McpServerStatus::Running, None)?;

        log::info!(
            "MCP server '{}' started with {} tools",
            server.name,
            registered_tools.len()
        );

        Ok(registered_tools)
    }

    /// Stop a running MCP server
    pub async fn stop_server(&self, server_id: &str) -> Result<()> {
        let mut client = {
            let mut clients = self.clients.write().await;
            clients
                .remove(server_id)
                .ok_or_else(|| anyhow!("MCP server is not running"))?
        };

        // Shutdown the client
        client.shutdown().await?;

        // Update status
        self.db
            .update_mcp_server_status(server_id, McpServerStatus::Stopped, None)?;

        log::info!("MCP server stopped: {}", server_id);

        Ok(())
    }

    /// Restart an MCP server
    pub async fn restart_server(&self, server_id: &str) -> Result<Vec<Tool>> {
        // Stop if running
        {
            let clients = self.clients.read().await;
            if clients.contains_key(server_id) {
                drop(clients);
                self.stop_server(server_id).await?;
            }
        }

        // Start again
        self.start_server(server_id).await
    }

    /// Start all servers marked for auto-start
    pub async fn start_auto_start_servers(&self) -> Vec<(String, Result<Vec<Tool>>)> {
        let servers = match self.db.list_auto_start_mcp_servers() {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to list auto-start MCP servers: {}", e);
                return vec![];
            }
        };

        let mut results = Vec::new();

        for server in servers {
            log::info!("Auto-starting MCP server: {}", server.name);
            let result = self.start_server(&server.id).await;
            if let Err(ref e) = result {
                log::error!("Failed to auto-start MCP server '{}': {}", server.name, e);
            }
            results.push((server.id, result));
        }

        results
    }

    /// Check if a server is running
    pub async fn is_server_running(&self, server_id: &str) -> bool {
        let clients = self.clients.read().await;
        clients.contains_key(server_id)
    }

    /// Get the status of a server
    pub async fn get_server_status(&self, server_id: &str) -> Result<McpServerStatus> {
        let server = self
            .db
            .get_mcp_server(server_id)?
            .ok_or_else(|| anyhow!("MCP server not found"))?;

        Ok(McpServerStatus::from_str(&server.status))
    }

    /// Call a tool on an MCP server
    pub async fn call_tool(
        &self,
        tool_id: &str,
        arguments: serde_json::Value,
    ) -> Result<String> {
        // Get the tool to find its server and name
        let tool = self
            .db
            .get_tool(tool_id)?
            .ok_or_else(|| anyhow!("Tool not found: {}", tool_id))?;

        // Verify it's an MCP tool
        if tool.tool_type != "mcp" {
            return Err(anyhow!("Tool '{}' is not an MCP tool", tool.name));
        }

        // Parse function schema to get the tool name
        let schema: serde_json::Value = serde_json::from_str(&tool.function_schema)
            .context("Failed to parse tool function_schema")?;
        let tool_name = schema["name"]
            .as_str()
            .ok_or_else(|| anyhow!("Tool schema missing name"))?;

        // Get the server ID directly from the tool
        let server_id = tool
            .mcp_server_id
            .as_ref()
            .ok_or_else(|| anyhow!("Tool '{}' has no associated MCP server", tool.name))?;

        // Check if server is running and get client
        let clients = self.clients.read().await;
        let client = clients.get(server_id).ok_or_else(|| {
            anyhow!(
                "MCP server for tool '{}' is not running. Start the server first.",
                tool.name
            )
        })?;

        // Call the tool
        client.call_tool(tool_name, arguments).await
    }

    /// Refresh tools from a running server
    pub async fn refresh_server_tools(&self, server_id: &str) -> Result<Vec<Tool>> {
        let clients = self.clients.read().await;
        let client = clients
            .get(server_id)
            .ok_or_else(|| anyhow!("MCP server is not running"))?;

        // List tools from server
        let mcp_tools = client.list_tools().await?;
        drop(clients);

        // Delete existing tools
        self.db.delete_mcp_server_tools(server_id)?;

        // Register discovered tools
        let mut registered_tools = Vec::new();
        for mcp_tool in &mcp_tools {
            let function_schema = serde_json::json!({
                "name": mcp_tool.name,
                "description": mcp_tool.description.clone().unwrap_or_default(),
                "parameters": mcp_tool.input_schema.clone().unwrap_or(serde_json::json!({
                    "type": "object",
                    "properties": {}
                }))
            });

            let tool_id = self.db.create_mcp_tool(
                server_id,
                &mcp_tool.name,
                mcp_tool.description.clone(),
                &function_schema.to_string(),
            )?;

            if let Some(tool) = self.db.get_tool(&tool_id)? {
                registered_tools.push(tool);
            }
        }

        log::info!(
            "Refreshed MCP server tools: {} tools discovered",
            registered_tools.len()
        );

        Ok(registered_tools)
    }

    /// Stop all running servers (for shutdown)
    pub async fn stop_all_servers(&self) {
        let server_ids: Vec<String> = {
            let clients = self.clients.read().await;
            clients.keys().cloned().collect()
        };

        for server_id in server_ids {
            if let Err(e) = self.stop_server(&server_id).await {
                log::error!("Failed to stop MCP server {}: {}", server_id, e);
            }
        }
    }

    /// Get list of running server IDs
    pub async fn get_running_servers(&self) -> Vec<String> {
        let clients = self.clients.read().await;
        clients.keys().cloned().collect()
    }
}

use anyhow::Context;
