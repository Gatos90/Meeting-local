// MCP Tauri commands for Meeting-Local
// Frontend API for managing MCP servers

use std::collections::HashMap;
use tauri::State;

use crate::database::models::{CreateMcpServer, McpServer, McpServerWithTools, Tool, UpdateMcpServer};
use crate::state::AppState;

/// List all MCP servers
#[tauri::command]
pub async fn mcp_list_servers(state: State<'_, AppState>) -> Result<Vec<McpServer>, String> {
    let db = state.db().await;
    db.list_mcp_servers()
        .map_err(|e| format!("Failed to list MCP servers: {}", e))
}

/// List all MCP servers with their tool counts
#[tauri::command]
pub async fn mcp_list_servers_with_tools(
    state: State<'_, AppState>,
) -> Result<Vec<McpServerWithTools>, String> {
    let db = state.db().await;
    db.list_mcp_servers_with_tools()
        .map_err(|e| format!("Failed to list MCP servers: {}", e))
}

/// Get a single MCP server by ID
#[tauri::command]
pub async fn mcp_get_server(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<McpServer>, String> {
    let db = state.db().await;
    db.get_mcp_server(&id)
        .map_err(|e| format!("Failed to get MCP server: {}", e))
}

/// Create a new MCP server
#[tauri::command]
pub async fn mcp_create_server(
    state: State<'_, AppState>,
    name: String,
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
    working_directory: Option<String>,
    auto_start: bool,
) -> Result<String, String> {
    let input = CreateMcpServer {
        name,
        command,
        args,
        env,
        working_directory,
        auto_start,
    };

    let db = state.db().await;
    db.create_mcp_server(&input)
        .map_err(|e| format!("Failed to create MCP server: {}", e))
}

/// Import MCP servers from standard config JSON format
/// Format: { "server_name": { "command": "...", "args": [...], "env": {...} } }
#[tauri::command]
pub async fn mcp_import_config(
    state: State<'_, AppState>,
    config_json: String,
) -> Result<Vec<String>, String> {
    let db = state.db().await;
    db.import_mcp_config(&config_json)
        .map_err(|e| format!("Failed to import MCP config: {}", e))
}

/// Update an existing MCP server
#[tauri::command]
pub async fn mcp_update_server(
    state: State<'_, AppState>,
    id: String,
    name: Option<String>,
    command: Option<String>,
    args: Option<Vec<String>>,
    env: Option<HashMap<String, String>>,
    working_directory: Option<String>,
    auto_start: Option<bool>,
    enabled: Option<bool>,
) -> Result<(), String> {
    let input = UpdateMcpServer {
        name,
        command,
        args,
        env,
        working_directory,
        auto_start,
        enabled,
    };

    let db = state.db().await;
    db.update_mcp_server(&id, &input)
        .map_err(|e| format!("Failed to update MCP server: {}", e))
}

/// Delete an MCP server
#[tauri::command]
pub async fn mcp_delete_server(state: State<'_, AppState>, id: String) -> Result<(), String> {
    // Stop if running
    {
        let mcp = state.mcp().await;
        if mcp.is_server_running(&id).await {
            mcp.stop_server(&id)
                .await
                .map_err(|e| format!("Failed to stop MCP server: {}", e))?;
        }
    }

    let db = state.db().await;
    db.delete_mcp_server(&id)
        .map_err(|e| format!("Failed to delete MCP server: {}", e))
}

/// Start an MCP server
#[tauri::command]
pub async fn mcp_start_server(state: State<'_, AppState>, id: String) -> Result<Vec<Tool>, String> {
    let mcp = state.mcp().await;
    mcp.start_server(&id)
        .await
        .map_err(|e| format!("Failed to start MCP server: {}", e))
}

/// Stop an MCP server
#[tauri::command]
pub async fn mcp_stop_server(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let mcp = state.mcp().await;
    mcp.stop_server(&id)
        .await
        .map_err(|e| format!("Failed to stop MCP server: {}", e))
}

/// Restart an MCP server
#[tauri::command]
pub async fn mcp_restart_server(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<Tool>, String> {
    let mcp = state.mcp().await;
    mcp.restart_server(&id)
        .await
        .map_err(|e| format!("Failed to restart MCP server: {}", e))
}

/// Get the status of an MCP server
#[tauri::command]
pub async fn mcp_get_server_status(
    state: State<'_, AppState>,
    id: String,
) -> Result<String, String> {
    let mcp = state.mcp().await;
    let status = mcp
        .get_server_status(&id)
        .await
        .map_err(|e| format!("Failed to get server status: {}", e))?;

    Ok(status.as_str().to_string())
}

/// Check if an MCP server is running
#[tauri::command]
pub async fn mcp_is_server_running(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    let mcp = state.mcp().await;
    Ok(mcp.is_server_running(&id).await)
}

/// Refresh tools from a running MCP server
#[tauri::command]
pub async fn mcp_refresh_tools(state: State<'_, AppState>, id: String) -> Result<Vec<Tool>, String> {
    let mcp = state.mcp().await;
    mcp.refresh_server_tools(&id)
        .await
        .map_err(|e| format!("Failed to refresh tools: {}", e))
}

/// Get tools discovered from an MCP server
#[tauri::command]
pub async fn mcp_get_server_tools(
    state: State<'_, AppState>,
    server_id: String,
) -> Result<Vec<Tool>, String> {
    let db = state.db().await;
    db.get_mcp_server_tools(&server_id)
        .map_err(|e| format!("Failed to get server tools: {}", e))
}

/// Get list of running MCP server IDs
#[tauri::command]
pub async fn mcp_get_running_servers(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let mcp = state.mcp().await;
    Ok(mcp.get_running_servers().await)
}
