//! Tauri commands for LLM functionality

use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

use crate::llm_engine::engine::LlmEngine;
use crate::llm_engine::model_manager::{DownloadableModel, LocalModelInfo, LlmModelManager};
use crate::llm_engine::provider::{
    CompletionRequest, CompletionResponse, LlmError, LlmModelInfo, Message,
    ProviderCapabilities, ProviderType,
};
use crate::state::AppState;

/// Provider info for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub provider_type: ProviderType,
    pub name: String,
    pub capabilities: ProviderCapabilities,
    pub is_available: bool,
}

// === Provider Management Commands ===

/// Get list of available LLM providers
#[tauri::command]
pub async fn llm_get_providers(state: State<'_, AppState>) -> Result<Vec<ProviderInfo>, String> {
    let engine = state.llm_engine.read().await;

    let mut providers = Vec::new();
    for provider_type in engine.available_providers() {
        let capabilities = engine.provider_capabilities(&provider_type).unwrap_or_default();

        let is_available = match &provider_type {
            ProviderType::Ollama => {
                // Check if Ollama is running
                if let Some(provider) = engine.get_provider(&provider_type) {
                    provider.is_ready().await
                } else {
                    false
                }
            }
            ProviderType::Embedded => {
                // TODO: Check if any models are downloaded
                false
            }
            _ => false,
        };

        providers.push(ProviderInfo {
            provider_type: provider_type.clone(),
            name: provider_type.to_string(),
            capabilities,
            is_available,
        });
    }

    Ok(providers)
}

/// Get the active LLM provider
#[tauri::command]
pub async fn llm_get_active_provider(state: State<'_, AppState>) -> Result<Option<ProviderType>, String> {
    let engine = state.llm_engine.read().await;
    Ok(engine.active_provider_type().await)
}

/// Set the active LLM provider
#[tauri::command]
pub async fn llm_set_active_provider(
    state: State<'_, AppState>,
    provider_type: ProviderType,
) -> Result<(), String> {
    let engine = state.llm_engine.read().await;
    engine
        .set_active_provider(provider_type)
        .await
        .map_err(|e| e.to_string())
}

// === Model Management Commands ===

/// List models for the active provider
#[tauri::command]
pub async fn llm_list_models(state: State<'_, AppState>) -> Result<Vec<LlmModelInfo>, String> {
    let engine = state.llm_engine.read().await;
    engine.list_models().await.map_err(|e| e.to_string())
}

/// List models for a specific provider
#[tauri::command]
pub async fn llm_list_models_for_provider(
    state: State<'_, AppState>,
    provider_type: ProviderType,
) -> Result<Vec<LlmModelInfo>, String> {
    let engine = state.llm_engine.read().await;
    engine
        .list_models_for_provider(&provider_type)
        .await
        .map_err(|e| e.to_string())
}

/// Initialize the active provider with a model
#[tauri::command]
pub async fn llm_initialize(state: State<'_, AppState>, model_id: String) -> Result<(), String> {
    let engine = state.llm_engine.read().await;
    engine.initialize(&model_id).await.map_err(|e| e.to_string())
}

/// Get the currently loaded model
#[tauri::command]
pub async fn llm_current_model(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let engine = state.llm_engine.read().await;
    Ok(engine.current_model().await)
}

/// Check if the LLM engine is ready
#[tauri::command]
pub async fn llm_is_ready(state: State<'_, AppState>) -> Result<bool, String> {
    let engine = state.llm_engine.read().await;
    Ok(engine.is_ready().await)
}

// === Ollama-specific Commands ===

/// Check Ollama connection and get version
#[tauri::command]
pub async fn llm_ollama_check_connection(state: State<'_, AppState>) -> Result<String, String> {
    let engine = state.llm_engine.read().await;
    engine
        .ollama_check_connection()
        .await
        .map_err(|e| e.to_string())
}

// === Completion Commands ===

/// Request for completion from frontend
#[derive(Debug, Clone, Deserialize)]
pub struct CompletionRequestInput {
    pub messages: Vec<Message>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stream: Option<bool>,
}

/// Run a completion (non-streaming)
#[tauri::command]
pub async fn llm_complete(
    state: State<'_, AppState>,
    request: CompletionRequestInput,
) -> Result<CompletionResponse, String> {
    let engine = state.llm_engine.read().await;

    let completion_request = CompletionRequest {
        messages: request.messages,
        max_tokens: request.max_tokens,
        temperature: request.temperature,
        stream: false,
        ..Default::default()
    };

    engine.complete(completion_request).await.map_err(|e| e.to_string())
}

/// Run a streaming completion
/// This emits events to the frontend as tokens arrive
#[tauri::command]
pub async fn llm_complete_streaming(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    request: CompletionRequestInput,
    event_id: String,
) -> Result<CompletionResponse, String> {
    let engine = state.llm_engine.read().await;

    let completion_request = CompletionRequest {
        messages: request.messages,
        max_tokens: request.max_tokens,
        temperature: request.temperature,
        stream: true,
        ..Default::default()
    };

    let app_handle_clone = app_handle.clone();
    let event_id_clone = event_id.clone();

    let callback = Box::new(move |token: String| {
        let _ = app_handle_clone.emit(&format!("llm-stream-{}", event_id_clone), token);
    });

    let response = engine
        .complete_streaming(completion_request, callback, None)
        .await
        .map_err(|e| e.to_string())?;

    // Emit completion event
    let _ = app_handle.emit(&format!("llm-stream-{}-complete", event_id), &response);

    Ok(response)
}

// === Model Download Commands (for embedded provider) ===

/// Get list of models available for download
#[tauri::command]
pub async fn llm_get_downloadable_models() -> Result<Vec<DownloadableModel>, String> {
    Ok(LlmModelManager::available_models())
}

/// Get list of locally downloaded models
#[tauri::command]
pub async fn llm_get_local_models(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let manager = state.llm_model_manager.read().await;
    manager.local_models().map_err(|e| e.to_string())
}

/// Check if a model is downloaded
#[tauri::command]
pub async fn llm_is_model_downloaded(
    state: State<'_, AppState>,
    model_id: String,
) -> Result<bool, String> {
    let manager = state.llm_model_manager.read().await;
    Ok(manager.is_downloaded(&model_id))
}

/// Delete a downloaded model
#[tauri::command]
pub async fn llm_delete_model(state: State<'_, AppState>, model_id: String) -> Result<(), String> {
    let manager = state.llm_model_manager.read().await;
    manager.delete_model(&model_id).map_err(|e| e.to_string())
}

/// Download a model with progress events
#[tauri::command]
pub async fn llm_download_model(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    model_id: String,
) -> Result<(), String> {
    use crate::llm_engine::model_manager::DownloadProgress;

    // Clone the Arc for the spawned task
    let manager_arc = state.llm_model_manager.clone();

    // Spawn download task
    tokio::spawn(async move {
        let manager = manager_arc.read().await;
        let app_handle_for_progress = app_handle.clone();
        let model_id_for_progress = model_id.clone();

        let result = manager
            .download_model(&model_id, move |progress: DownloadProgress| {
                // Emit progress event
                let _ = app_handle_for_progress.emit("llm-download-progress", &progress);
            })
            .await;

        match result {
            Ok(_path) => {
                let _ = app_handle.emit(
                    "llm-download-complete",
                    serde_json::json!({ "model_id": model_id_for_progress }),
                );
            }
            Err(e) => {
                let _ = app_handle.emit(
                    "llm-download-error",
                    serde_json::json!({
                        "model_id": model_id_for_progress,
                        "error": e.to_string()
                    }),
                );
            }
        }
    });

    Ok(())
}

/// Cancel a model download
#[tauri::command]
pub async fn llm_cancel_download(
    state: State<'_, AppState>,
    model_id: String,
) -> Result<(), String> {
    let manager = state.llm_model_manager.read().await;
    manager.cancel_download(&model_id).map_err(|e| e.to_string())
}

/// Download a custom model from a URL
#[tauri::command]
pub async fn llm_download_custom_model(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    name: String,
    url: String,
) -> Result<(), String> {
    use crate::llm_engine::model_manager::DownloadProgress;

    // Validate inputs
    if name.trim().is_empty() {
        return Err("Model name is required".to_string());
    }
    if url.trim().is_empty() {
        return Err("URL is required".to_string());
    }

    // Clone the Arc for the spawned task
    let manager_arc = state.llm_model_manager.clone();

    // Spawn download task
    tokio::spawn(async move {
        let manager = manager_arc.read().await;
        let app_handle_for_progress = app_handle.clone();
        let name_clone = name.clone();

        let result = manager
            .download_custom_model(&name, &url, move |progress: DownloadProgress| {
                // Emit progress event
                let _ = app_handle_for_progress.emit("llm-download-progress", &progress);
            })
            .await;

        match result {
            Ok(_path) => {
                // Sanitize name to get model_id (same logic as in download_custom_model)
                let model_id: String = name_clone
                    .chars()
                    .map(|c| {
                        if c.is_alphanumeric() || c == '-' || c == '_' {
                            c
                        } else {
                            '-'
                        }
                    })
                    .collect::<String>()
                    .to_lowercase();

                let _ = app_handle.emit(
                    "llm-download-complete",
                    serde_json::json!({ "model_id": model_id }),
                );
            }
            Err(e) => {
                let _ = app_handle.emit(
                    "llm-download-error",
                    serde_json::json!({
                        "model_id": name_clone,
                        "error": e.to_string()
                    }),
                );
            }
        }
    });

    Ok(())
}

/// Get detailed info about all locally downloaded models
/// Includes user-defined tool support overrides from the database
#[tauri::command]
pub async fn llm_get_local_models_info(
    state: State<'_, AppState>,
) -> Result<Vec<LocalModelInfo>, String> {
    let manager = state.llm_model_manager.read().await;
    let mut models = manager.local_models_info().map_err(|e| e.to_string())?;

    // Check database for user-defined tool support overrides
    let db = state.db().await;
    for model in &mut models {
        if let Ok(Some(user_override)) = db.get_model_tool_support(&model.id) {
            model.has_native_tool_support = user_override;
        }
    }

    Ok(models)
}

// === Default Model Settings ===

/// Default LLM model configuration response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultLlmConfigResponse {
    pub provider_type: Option<String>,
    pub model_id: Option<String>,
}

/// Get the default LLM model from settings
#[tauri::command]
pub async fn llm_get_default_model(
    state: State<'_, AppState>,
) -> Result<Option<DefaultLlmConfigResponse>, String> {
    let db = state.db().await;

    let provider = db
        .get_setting("default_llm_provider")
        .map_err(|e| e.to_string())?;
    let model = db
        .get_setting("default_llm_model")
        .map_err(|e| e.to_string())?;

    // Only return if at least one is set
    if provider.is_none() && model.is_none() {
        return Ok(None);
    }

    Ok(Some(DefaultLlmConfigResponse {
        provider_type: provider,
        model_id: model,
    }))
}

/// Set the default LLM model in settings
#[tauri::command]
pub async fn llm_set_default_model(
    state: State<'_, AppState>,
    provider_type: Option<String>,
    model_id: Option<String>,
) -> Result<(), String> {
    let db = state.db().await;

    // Save provider type (or clear if None)
    match provider_type {
        Some(p) => db.set_setting("default_llm_provider", &p, "string"),
        None => db.delete_setting("default_llm_provider"),
    }
    .map_err(|e| e.to_string())?;

    // Save model id (or clear if None)
    match model_id {
        Some(m) => db.set_setting("default_llm_model", &m, "string"),
        None => db.delete_setting("default_llm_model"),
    }
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Clear the default LLM model settings
#[tauri::command]
pub async fn llm_clear_default_model(state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db().await;

    let _ = db.delete_setting("default_llm_provider");
    let _ = db.delete_setting("default_llm_model");

    Ok(())
}

// === Model Tool Support Commands ===

/// Get whether a model has native tool support
/// Returns: Option<bool> - None if no user override, Some(bool) if user has set a preference
#[tauri::command]
pub async fn llm_get_model_tool_support(
    state: State<'_, AppState>,
    model_id: String,
) -> Result<Option<bool>, String> {
    let db = state.db().await;
    db.get_model_tool_support(&model_id).map_err(|e| e.to_string())
}

/// Set whether a model has native tool support
#[tauri::command]
pub async fn llm_set_model_tool_support(
    state: State<'_, AppState>,
    model_id: String,
    has_native_tool_support: bool,
) -> Result<(), String> {
    let db = state.db().await;
    db.set_model_tool_support(&model_id, has_native_tool_support)
        .map_err(|e| e.to_string())
}

/// Delete a model's tool support configuration (revert to default)
#[tauri::command]
pub async fn llm_delete_model_tool_support(
    state: State<'_, AppState>,
    model_id: String,
) -> Result<(), String> {
    let db = state.db().await;
    db.delete_model_config(&model_id).map_err(|e| e.to_string())
}

/// Get all model tool support configurations
#[tauri::command]
pub async fn llm_get_all_model_configs(
    state: State<'_, AppState>,
) -> Result<Vec<crate::database::ModelConfig>, String> {
    let db = state.db().await;
    db.get_all_model_configs().map_err(|e| e.to_string())
}

/// Get effective tool support for a model (checking user config, registry, and fallback)
#[tauri::command]
pub async fn llm_get_effective_tool_support(
    state: State<'_, AppState>,
    model_id: String,
) -> Result<bool, String> {
    use crate::llm_engine::model_manager::has_native_tool_support_with_override;

    let db = state.db().await;
    let user_override = db.get_model_tool_support(&model_id).ok().flatten();

    Ok(has_native_tool_support_with_override(&model_id, user_override))
}
