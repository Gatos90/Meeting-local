//! LLM Provider implementations
//!
//! Each provider implements the LlmProvider trait for a specific backend

pub mod ollama_provider;
pub mod sidecar_provider;
// pub mod openai_provider;   // TODO: Phase 2 - API providers
// pub mod claude_provider;   // TODO: Phase 2 - API providers

pub use ollama_provider::OllamaProvider;
pub use sidecar_provider::{SidecarProvider, SidecarConfig};
