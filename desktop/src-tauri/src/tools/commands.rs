//! Tauri commands for AI tools management

use tauri::State;

use crate::database::{Tool, CreateTool, UpdateTool};
use crate::state::AppState;

/// Get all tools
#[tauri::command]
pub async fn tools_list(
    state: State<'_, AppState>,
) -> Result<Vec<Tool>, String> {
    let db = state.db().await;
    db.list_tools()
        .map_err(|e| e.to_string())
}

/// Get enabled tools only
#[tauri::command]
pub async fn tools_list_enabled(
    state: State<'_, AppState>,
) -> Result<Vec<Tool>, String> {
    let db = state.db().await;
    db.list_enabled_tools()
        .map_err(|e| e.to_string())
}

/// Get default tools (auto-included in chats)
#[tauri::command]
pub async fn tools_list_defaults(
    state: State<'_, AppState>,
) -> Result<Vec<Tool>, String> {
    let db = state.db().await;
    db.list_default_tools()
        .map_err(|e| e.to_string())
}

/// Get a single tool by ID
#[tauri::command]
pub async fn tools_get(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<Tool>, String> {
    let db = state.db().await;
    db.get_tool(&id)
        .map_err(|e| e.to_string())
}

/// Create a new custom tool
#[tauri::command]
pub async fn tools_create(
    state: State<'_, AppState>,
    name: String,
    function_schema: String,
    description: Option<String>,
    execution_location: Option<String>,
    icon: Option<String>,
) -> Result<String, String> {
    let db = state.db().await;

    let input = CreateTool {
        name,
        description,
        function_schema,
        execution_location,
        icon,
    };

    db.create_tool(&input)
        .map_err(|e| e.to_string())
}

/// Update an existing tool
#[tauri::command]
pub async fn tools_update(
    state: State<'_, AppState>,
    id: String,
    name: Option<String>,
    description: Option<String>,
    function_schema: Option<String>,
    execution_location: Option<String>,
    enabled: Option<bool>,
    is_default: Option<bool>,
    icon: Option<String>,
    sort_order: Option<i32>,
) -> Result<(), String> {
    let db = state.db().await;

    let input = UpdateTool {
        name,
        description,
        function_schema,
        execution_location,
        enabled,
        is_default,
        icon,
        sort_order,
    };

    db.update_tool(&id, &input)
        .map_err(|e| e.to_string())
}

/// Delete a custom tool
#[tauri::command]
pub async fn tools_delete(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let db = state.db().await;
    db.delete_tool(&id)
        .map_err(|e| e.to_string())
}

/// Set a tool's default status
#[tauri::command]
pub async fn tools_set_default(
    state: State<'_, AppState>,
    id: String,
    is_default: bool,
) -> Result<(), String> {
    let db = state.db().await;
    db.set_tool_default(&id, is_default)
        .map_err(|e| e.to_string())
}

/// Get tools enabled for a specific chat session
#[tauri::command]
pub async fn tools_get_for_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<Tool>, String> {
    let db = state.db().await;
    db.get_session_tools(&session_id)
        .map_err(|e| e.to_string())
}

/// Set tools enabled for a chat session
#[tauri::command]
pub async fn tools_set_for_session(
    state: State<'_, AppState>,
    session_id: String,
    tool_ids: Vec<String>,
) -> Result<(), String> {
    let db = state.db().await;
    db.set_session_tools(&session_id, &tool_ids)
        .map_err(|e| e.to_string())
}

/// Initialize default tools for a new chat session
#[tauri::command]
pub async fn tools_init_for_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    let db = state.db().await;
    db.init_session_tools(&session_id)
        .map_err(|e| e.to_string())
}
