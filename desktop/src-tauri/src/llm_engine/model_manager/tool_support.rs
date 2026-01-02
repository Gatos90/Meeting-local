//! LLM Model Tool Support Detection

use super::registry::available_models;

/// Models known to have native function calling support in their chat templates
pub const NATIVE_TOOL_MODELS: &[&str] = &[
    "qwen2.5",
    "qwen2",
    "qwen",
    "hermes",
    "mistral",
    "mixtral",
    "command-r",
    "functionary",
    "gorilla",
    "nexusraven",
    "firefunction",
];

/// Check if a model has native tool calling support
///
/// This function checks:
/// 1. User-defined override (if provided)
/// 2. Curated model registry definitions
/// 3. Falls back to pattern matching on model name
pub fn has_native_tool_support(model_id: &str) -> bool {
    has_native_tool_support_with_override(model_id, None)
}

/// Check if a model has native tool calling support with optional user override
///
/// Priority:
/// 1. User-defined override from database (passed as parameter)
/// 2. Curated model registry definitions
/// 3. Pattern matching on model name
pub fn has_native_tool_support_with_override(model_id: &str, user_override: Option<bool>) -> bool {
    // 1. User override takes precedence
    if let Some(has_support) = user_override {
        return has_support;
    }

    // 2. Check curated model registry
    if let Some(curated_model) = available_models().iter().find(|m| m.id == model_id) {
        return curated_model.has_native_tool_support;
    }

    // 3. Fall back to pattern matching
    let name_lower = model_id.to_lowercase();
    NATIVE_TOOL_MODELS.iter().any(|m| name_lower.contains(m))
}

/// Check only the curated registry for tool support (no pattern matching fallback)
pub fn get_curated_tool_support(model_id: &str) -> Option<bool> {
    available_models()
        .iter()
        .find(|m| m.id == model_id)
        .map(|m| m.has_native_tool_support)
}
