// Database models - Category and Tag
use serde::{Deserialize, Serialize};
use super::Recording;

/// A category (predefined or user-created)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub is_system: bool,
}

/// A user-defined tag
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub usage_count: i32,
}

/// Search result from full-text search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub recording: Recording,
    pub matched_text: String,
    pub categories: Vec<Category>,
    pub tags: Vec<Tag>,
}

/// Search filters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchFilters {
    pub category_ids: Option<Vec<String>>,
    pub tag_ids: Option<Vec<String>>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub search_transcripts: bool,
}
