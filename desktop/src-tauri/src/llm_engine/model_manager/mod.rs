//! LLM Model Manager
//!
//! Handles downloading and managing GGUF models for embedded inference.
//!
//! Module structure:
//! - types.rs: DownloadableModel, DownloadProgress, LocalModelInfo
//! - registry.rs: Available models list, HuggingFace repo lookup
//! - downloader.rs: Download logic for curated and custom models
//! - tool_support.rs: Native tool calling detection
//! - manager.rs: LlmModelManager struct

pub mod types;
pub mod registry;
pub mod downloader;
pub mod tool_support;
pub mod manager;

// Re-export for backwards compatibility
pub use types::{DownloadableModel, DownloadProgress, DownloadStatus, LocalModelInfo};
pub use registry::{available_models, get_hf_repo_for_model};
pub use tool_support::{has_native_tool_support, has_native_tool_support_with_override, NATIVE_TOOL_MODELS};
pub use manager::LlmModelManager;
