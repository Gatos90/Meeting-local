//! Tauri commands for prompt template management

use tauri::State;

use crate::database::{PromptTemplate, CreatePromptTemplate, UpdatePromptTemplate};
use crate::state::AppState;

/// Get all prompt templates
#[tauri::command]
pub async fn template_list(
    state: State<'_, AppState>,
) -> Result<Vec<PromptTemplate>, String> {
    let db = state.db().await;
    db.list_templates()
        .map_err(|e| e.to_string())
}

/// Get a single prompt template by ID
#[tauri::command]
pub async fn template_get(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<PromptTemplate>, String> {
    let db = state.db().await;
    db.get_template(&id)
        .map_err(|e| e.to_string())
}

/// Create a new custom prompt template
#[tauri::command]
pub async fn template_create(
    state: State<'_, AppState>,
    name: String,
    prompt: String,
    description: Option<String>,
    icon: Option<String>,
    sort_order: Option<i32>,
) -> Result<String, String> {
    let db = state.db().await;

    let input = CreatePromptTemplate {
        name,
        prompt,
        description,
        icon,
        sort_order,
    };

    db.create_template(&input)
        .map_err(|e| e.to_string())
}

/// Update an existing custom prompt template
#[tauri::command]
pub async fn template_update(
    state: State<'_, AppState>,
    id: String,
    name: Option<String>,
    prompt: Option<String>,
    description: Option<String>,
    icon: Option<String>,
    sort_order: Option<i32>,
) -> Result<(), String> {
    let db = state.db().await;

    let input = UpdatePromptTemplate {
        name,
        prompt,
        description,
        icon,
        sort_order,
    };

    db.update_template(&id, &input)
        .map_err(|e| e.to_string())
}

/// Delete a custom prompt template
#[tauri::command]
pub async fn template_delete(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let db = state.db().await;
    db.delete_template(&id)
        .map_err(|e| e.to_string())
}

/// Duplicate a prompt template (creates a custom copy)
#[tauri::command]
pub async fn template_duplicate(
    state: State<'_, AppState>,
    id: String,
) -> Result<String, String> {
    let db = state.db().await;
    db.duplicate_template(&id)
        .map_err(|e| e.to_string())
}
