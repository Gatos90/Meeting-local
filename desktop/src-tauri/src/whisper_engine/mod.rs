// Whisper Engine Module
//
// Split into focused files:
// - types.rs: ModelStatus, ModelInfo structs
// - text_cleaner.rs: Repetition detection and cleaning
// - model_registry.rs: Model discovery and validation
// - model_loader.rs: Model loading and GPU detection
// - downloader.rs: Model downloading
// - engine.rs: Core WhisperEngine struct and transcription

pub mod types;
pub mod text_cleaner;
pub mod model_registry;
pub mod model_loader;
pub mod downloader;
pub mod engine;
pub mod commands;
pub mod system_monitor;
pub mod parallel_processor;
pub mod parallel_commands;

// Re-export for backwards compatibility
pub use types::{ModelStatus, ModelInfo};
pub use engine::WhisperEngine;
pub use commands::*;
pub use system_monitor::*;
pub use parallel_processor::*;
pub use parallel_commands::*;
