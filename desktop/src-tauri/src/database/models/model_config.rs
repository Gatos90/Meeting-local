// Model configuration models

use serde::{Deserialize, Serialize};

/// User-defined configuration for a specific LLM model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// The model ID (e.g., "mistral-7b-instruct-v0.3.Q4_K_M.gguf")
    pub model_id: String,
    /// Whether this model has native function calling support
    pub has_native_tool_support: bool,
    /// When this config was created
    pub created_at: String,
    /// When this config was last updated
    pub updated_at: String,
}

/// Input for creating/updating a model config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertModelConfig {
    pub model_id: String,
    pub has_native_tool_support: bool,
}
