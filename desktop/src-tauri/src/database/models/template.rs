// Database models - Prompt Templates
use serde::{Deserialize, Serialize};

/// A prompt template for quick chat actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub prompt: String,
    pub icon: Option<String>,
    pub is_builtin: bool,
    pub sort_order: i32,
    pub created_at: String,
}

/// Input for creating a new prompt template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePromptTemplate {
    pub name: String,
    pub description: Option<String>,
    pub prompt: String,
    pub icon: Option<String>,
    pub sort_order: Option<i32>,
}

/// Input for updating a prompt template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePromptTemplate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub prompt: Option<String>,
    pub icon: Option<String>,
    pub sort_order: Option<i32>,
}
