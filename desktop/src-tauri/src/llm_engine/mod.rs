//! LLM Engine module for AI-powered transcript processing
//!
//! Supports multiple backends:
//! - Embedded inference via mistral.rs (GGUF models, no external dependencies)
//! - Ollama API (requires running Ollama server)
//! - OpenAI-compatible APIs
//! - Claude API

pub mod provider;
pub mod engine;
pub mod commands;
pub mod model_manager;
pub mod providers;

pub use provider::{
    LlmProvider, LlmError, LlmModelInfo, ProviderCapabilities,
    CompletionRequest, CompletionResponse, Message, MessageRole,
};
pub use engine::LlmEngine;
