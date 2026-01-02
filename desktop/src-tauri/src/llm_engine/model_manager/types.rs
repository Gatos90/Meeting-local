//! LLM Model Manager Types

use serde::{Deserialize, Serialize};

/// Information about a downloadable model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadableModel {
    /// Model identifier (used as filename without extension)
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description
    pub description: String,
    /// Size in bytes
    pub size_bytes: u64,
    /// HuggingFace repository ID (e.g., "bartowski/Llama-3.2-3B-Instruct-GGUF")
    /// This is used for both downloading and fetching the tokenizer
    pub hf_repo: String,
    /// GGUF filename in the HuggingFace repo
    pub gguf_file: String,
    /// Download URL (constructed from hf_repo and gguf_file)
    pub url: String,
    /// Expected SHA256 hash
    pub sha256: Option<String>,
    /// Context length
    pub context_length: u32,
    /// Recommended for specific tasks
    pub recommended_for: Vec<String>,
    /// Whether this model has native function calling support
    pub has_native_tool_support: bool,
}

/// Download progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub model_id: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub percent: f32,
    pub status: DownloadStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DownloadStatus {
    Pending,
    Downloading,
    Verifying,
    Complete,
    Failed(String),
}

/// Information about a locally downloaded model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModelInfo {
    pub id: String,
    pub name: String,
    pub size_bytes: u64,
    pub is_curated: bool,
    pub description: Option<String>,
    pub context_length: Option<u32>,
    /// Whether this model has native function calling support
    pub has_native_tool_support: bool,
}
