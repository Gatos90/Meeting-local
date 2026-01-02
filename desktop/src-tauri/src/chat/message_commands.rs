//! Chat message commands - send, query, cancel messages

use std::sync::Arc;
use tauri::{Emitter, State};
use tokio_util::sync::CancellationToken;

use crate::database::{ChatMessage, ChatMessageStatus};
use crate::state::AppState;
use super::types::{SendMessageResponse, ChatMessageStatus2};
use super::task_registry::{
    register_task, remove_task, cancel_task, cancel_session_tasks, is_session_processing,
};
use super::completion::run_chat_completion;

/// Send a chat message and start background completion
#[tauri::command]
pub async fn chat_send_message(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    content: String,
    provider_type: Option<String>,
    model_id: Option<String>,
    tool_ids: Option<Vec<String>>,
) -> Result<SendMessageResponse, String> {
    let db = state.db().await;

    // Get the session to get recording_id
    let session = db
        .get_chat_session(&session_id)
        .map_err(|e| e.to_string())?
        .ok_or("Session not found")?;

    let recording_id = session.recording_id.clone();

    // Get next sequence ID for this session
    let user_seq = db
        .get_next_chat_sequence_id_for_session(&session_id)
        .map_err(|e| e.to_string())?;

    // Create and save user message
    let user_message = ChatMessage::user(&session_id, &recording_id, &content, user_seq);
    db.save_chat_message(&user_message)
        .map_err(|e| e.to_string())?;

    // Create assistant message placeholder (pending) with provider/model info
    let assistant_seq = user_seq + 1;
    let assistant_message = ChatMessage::assistant_pending(
        &session_id,
        &recording_id,
        assistant_seq,
        provider_type.clone(),
        model_id.clone(),
    );
    db.save_chat_message(&assistant_message)
        .map_err(|e| e.to_string())?;

    // Update session config if provider/model provided
    if provider_type.is_some() || model_id.is_some() {
        let _ = db.update_chat_session_config(
            &session_id,
            provider_type.as_deref(),
            model_id.as_deref(),
        );
    }

    let user_message_id = user_message.id.clone();
    let assistant_message_id = assistant_message.id.clone();

    // Create cancellation token
    let cancel_token = CancellationToken::new();

    // Register the task
    register_task(
        assistant_message_id.clone(),
        session_id.clone(),
        cancel_token.clone(),
    );

    // Clone what we need for the spawned task
    let app_handle_clone = app_handle.clone();
    let state_llm_engine = state.llm_engine.clone();
    let state_db = state.database_arc();
    let state_mcp = state.mcp_manager_arc();
    let session_id_clone = session_id.clone();
    let recording_id_clone = recording_id.clone();
    let assistant_message_id_clone = assistant_message_id.clone();
    let tool_ids_clone = tool_ids.clone();

    // Spawn background task
    tokio::spawn(async move {
        let result = run_chat_completion(
            app_handle_clone.clone(),
            state_llm_engine,
            state_db,
            state_mcp,
            session_id_clone.clone(),
            recording_id_clone.clone(),
            assistant_message_id_clone.clone(),
            cancel_token,
            tool_ids_clone,
        )
        .await;

        // Remove from active tasks
        remove_task(&assistant_message_id_clone);

        // Emit completion event
        match result {
            Ok(_) => {
                let _ = app_handle_clone.emit(
                    &format!("chat-complete-{}", session_id_clone),
                    serde_json::json!({
                        "message_id": assistant_message_id_clone,
                        "status": "complete"
                    }),
                );
            }
            Err(e) => {
                let _ = app_handle_clone.emit(
                    &format!("chat-complete-{}", session_id_clone),
                    serde_json::json!({
                        "message_id": assistant_message_id_clone,
                        "status": "error",
                        "error": e
                    }),
                );
            }
        }
    });

    Ok(SendMessageResponse {
        user_message_id,
        assistant_message_id,
    })
}

/// Get all chat messages for a session
#[tauri::command]
pub async fn chat_get_messages(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<ChatMessage>, String> {
    let db = state.db().await;
    db.get_chat_messages_by_session(&session_id)
        .map_err(|e| e.to_string())
}

/// Get the status of a specific message (for polling)
#[tauri::command]
pub async fn chat_get_status(
    state: State<'_, AppState>,
    message_id: String,
) -> Result<Option<ChatMessageStatus2>, String> {
    let db = state.db().await;
    let message = db
        .get_chat_message(&message_id)
        .map_err(|e| e.to_string())?;

    Ok(message.map(|m| ChatMessageStatus2 {
        message_id: m.id,
        status: m.status.as_str().to_string(),
        content: m.content,
        error_message: m.error_message,
    }))
}

/// Cancel an in-progress chat message
#[tauri::command]
pub async fn chat_cancel_message(
    state: State<'_, AppState>,
    message_id: String,
) -> Result<(), String> {
    // Find and cancel the task
    if cancel_task(&message_id).is_some() {
        // Update status in database
        let db = state.db().await;
        db.update_chat_message_status(&message_id, ChatMessageStatus::Cancelled, None)
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Delete all chat messages for a session (but keep the session)
#[tauri::command]
pub async fn chat_clear_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    // Cancel any active tasks for this session
    cancel_session_tasks(&session_id);

    // Delete messages from database
    let db = state.db().await;
    db.delete_chat_messages_by_session(&session_id)
        .map_err(|e| e.to_string())
}

/// Delete all chat history for a recording (legacy - deletes all sessions)
#[tauri::command]
pub async fn chat_delete_history(
    state: State<'_, AppState>,
    recording_id: String,
) -> Result<(), String> {
    let db = state.db().await;

    // Get all sessions for this recording
    let sessions = db
        .get_chat_sessions(&recording_id)
        .map_err(|e| e.to_string())?;

    // Cancel any active tasks and delete each session
    for session in sessions {
        cancel_session_tasks(&session.id);
        db.delete_chat_session(&session.id)
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Check if there's an active chat task for a session
#[tauri::command]
pub async fn chat_is_processing(session_id: String) -> Result<bool, String> {
    Ok(is_session_processing(&session_id))
}

/// Get pending messages (for resuming on app restart)
#[tauri::command]
pub async fn chat_get_pending_messages(
    state: State<'_, AppState>,
) -> Result<Vec<ChatMessage>, String> {
    let db = state.db().await;
    db.get_pending_chat_messages().map_err(|e| e.to_string())
}
