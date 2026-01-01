//! Chat task registry - tracks active chat completion tasks

use dashmap::DashMap;
use once_cell::sync::Lazy;
use tokio_util::sync::CancellationToken;

/// Active chat completion task
pub struct ChatTask {
    pub session_id: String,
    pub message_id: String,
    pub cancel_token: CancellationToken,
}

/// Global registry of active chat tasks (keyed by message_id)
pub static ACTIVE_CHAT_TASKS: Lazy<DashMap<String, ChatTask>> = Lazy::new(DashMap::new);

/// Register a new chat task
pub fn register_task(message_id: String, session_id: String, cancel_token: CancellationToken) {
    ACTIVE_CHAT_TASKS.insert(
        message_id.clone(),
        ChatTask {
            session_id,
            message_id,
            cancel_token,
        },
    );
}

/// Remove a task from the registry
pub fn remove_task(message_id: &str) {
    ACTIVE_CHAT_TASKS.remove(message_id);
}

/// Cancel and remove tasks for a session
pub fn cancel_session_tasks(session_id: &str) {
    let tasks_to_cancel: Vec<String> = ACTIVE_CHAT_TASKS
        .iter()
        .filter(|entry| entry.session_id == session_id)
        .map(|entry| entry.message_id.clone())
        .collect();

    for message_id in tasks_to_cancel {
        if let Some((_, task)) = ACTIVE_CHAT_TASKS.remove(&message_id) {
            task.cancel_token.cancel();
        }
    }
}

/// Check if there's an active task for a session
pub fn is_session_processing(session_id: &str) -> bool {
    ACTIVE_CHAT_TASKS
        .iter()
        .any(|entry| entry.session_id == session_id)
}

/// Cancel a specific task by message_id
pub fn cancel_task(message_id: &str) -> Option<CancellationToken> {
    ACTIVE_CHAT_TASKS
        .remove(message_id)
        .map(|(_, task)| {
            task.cancel_token.cancel();
            task.cancel_token
        })
}
