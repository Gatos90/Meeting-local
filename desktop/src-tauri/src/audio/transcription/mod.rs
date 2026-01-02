// audio/transcription/mod.rs
//
// Transcription module: Provider abstraction, engine management, and worker pool.
//
// Module structure:
// - provider.rs: TranscriptionProvider trait, error types
// - whisper_provider.rs: Whisper-based implementation
// - parakeet_provider.rs: Parakeet-based implementation
// - engine.rs: TranscriptionEngine enum, initialization
// - globals.rs: Sequence counter, speech detection flag, diarization settings
// - types.rs: TranscriptUpdate struct, formatting utilities
// - diarization_integration.rs: Live speaker diarization support
// - transcriber.rs: Provider-agnostic chunk transcription
// - worker.rs: Parallel worker pool and main task loop

pub mod provider;
pub mod whisper_provider;
pub mod parakeet_provider;
pub mod engine;
pub mod globals;
pub mod types;
pub mod diarization_integration;
pub mod transcriber;
pub mod worker;

// Re-export commonly used types
pub use provider::{TranscriptionError, TranscriptionProvider, TranscriptResult};
pub use whisper_provider::WhisperProvider;
pub use parakeet_provider::ParakeetProvider;
pub use engine::{
    TranscriptionEngine,
    validate_transcription_model_ready,
    get_or_init_transcription_engine,
    get_or_init_whisper
};

// Re-export worker functions and types (main public API)
pub use worker::{
    start_transcription_task,
    reset_speech_detected_flag,
    set_live_diarization_enabled,
};

// Re-export types
pub use types::TranscriptUpdate;

// Re-export diarization check (for backwards compatibility)
pub use globals::is_live_diarization_enabled;
