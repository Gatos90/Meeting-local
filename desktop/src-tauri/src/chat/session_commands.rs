//! Chat session CRUD commands

use tauri::State;

use crate::database::{ChatConfig, ChatSession};
use crate::state::AppState;
use super::task_registry::cancel_session_tasks;

/// Create a new chat session for a recording
#[tauri::command]
pub async fn chat_create_session(
    state: State<'_, AppState>,
    recording_id: String,
    title: Option<String>,
    provider_type: Option<String>,
    model_id: Option<String>,
) -> Result<ChatSession, String> {
    let db = state.db().await;

    let session = ChatSession::new_with_config(
        &recording_id,
        &title.unwrap_or_else(|| "New Chat".to_string()),
        provider_type,
        model_id,
    );

    db.create_chat_session(&session)
        .map_err(|e| e.to_string())?;

    // Auto-initialize with default tools
    db.init_session_tools(&session.id)
        .map_err(|e| e.to_string())?;

    log::info!("Created new chat session {} with default tools", session.id);

    Ok(session)
}

/// Get all chat sessions for a recording (newest first)
#[tauri::command]
pub async fn chat_list_sessions(
    state: State<'_, AppState>,
    recording_id: String,
) -> Result<Vec<ChatSession>, String> {
    let db = state.db().await;
    db.get_chat_sessions(&recording_id)
        .map_err(|e| e.to_string())
}

/// Get a single chat session by ID
#[tauri::command]
pub async fn chat_get_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Option<ChatSession>, String> {
    let db = state.db().await;
    db.get_chat_session(&session_id)
        .map_err(|e| e.to_string())
}

/// Get or create a session for a recording (returns latest or creates new)
#[tauri::command]
pub async fn chat_get_or_create_session(
    state: State<'_, AppState>,
    recording_id: String,
) -> Result<ChatSession, String> {
    let db = state.db().await;
    let session = db
        .get_or_create_chat_session(&recording_id)
        .map_err(|e| e.to_string())?;

    // Ensure session has default tools initialized
    // (init_session_tools uses INSERT OR IGNORE, safe to call multiple times)
    db.init_session_tools(&session.id)
        .map_err(|e| e.to_string())?;

    Ok(session)
}

/// Update a chat session's provider/model config
#[tauri::command]
pub async fn chat_update_session_config(
    state: State<'_, AppState>,
    session_id: String,
    provider_type: Option<String>,
    model_id: Option<String>,
) -> Result<(), String> {
    let db = state.db().await;
    db.update_chat_session_config(
        &session_id,
        provider_type.as_deref(),
        model_id.as_deref(),
    )
    .map_err(|e| e.to_string())
}

/// Update a chat session's title
#[tauri::command]
pub async fn chat_update_session_title(
    state: State<'_, AppState>,
    session_id: String,
    title: String,
) -> Result<(), String> {
    let db = state.db().await;
    db.update_chat_session_title(&session_id, &title)
        .map_err(|e| e.to_string())
}

/// Delete a chat session and all its messages
#[tauri::command]
pub async fn chat_delete_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    // Cancel any active tasks for this session
    cancel_session_tasks(&session_id);

    let db = state.db().await;
    db.delete_chat_session(&session_id)
        .map_err(|e| e.to_string())
}

/// Get the chat config (provider/model) for a session
#[tauri::command]
pub async fn chat_get_config(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Option<ChatConfig>, String> {
    let db = state.db().await;
    db.get_session_chat_config(&session_id)
        .map_err(|e| e.to_string())
}
