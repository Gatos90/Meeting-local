//! API stub module
//!
//! Provides stub implementations for API configuration functions.

use tauri::{AppHandle, Runtime, State};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TranscriptConfig {
    pub model: String,
    pub provider: String,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelConfig {
    pub model: String,
    pub provider: String,
}

pub async fn api_get_transcript_config<R: Runtime>(
    _app: AppHandle<R>,
    _state: State<'_, crate::state::AppState>,
    _meeting_id: Option<String>,
) -> Result<Option<TranscriptConfig>, String> {
    Ok(Some(TranscriptConfig {
        model: "base".to_string(),
        provider: "localWhisper".to_string(),
        api_key: None,
    }))
}

pub async fn api_get_model_config<R: Runtime>(
    _app: AppHandle<R>,
    _state: State<'_, crate::state::AppState>,
    _meeting_id: Option<String>,
) -> Result<Option<ModelConfig>, String> {
    Ok(Some(ModelConfig {
        model: "base".to_string(),
        provider: "localWhisper".to_string(),
    }))
}
