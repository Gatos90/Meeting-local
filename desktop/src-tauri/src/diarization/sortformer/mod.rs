//! NVIDIA Sortformer v2 Streaming Speaker Diarization
//!
//! Standalone implementation adapted from parakeet-rs.
//! Supports streaming 4-speaker diarization via ONNX model.
//!
//! Module structure:
//! - config.rs: Constants and DiarizationConfig
//! - types.rs: SpeakerSegment output type
//! - features.rs: Mel spectrogram extraction, STFT, preemphasis
//! - streaming.rs: Streaming cache management and update logic
//! - postprocess.rs: Binarization and median filtering
//! - engine.rs: Main Sortformer struct

pub mod config;
pub mod types;
pub mod features;
pub mod streaming;
pub mod postprocess;
pub mod engine;

// Re-export main types for backwards compatibility
pub use config::DiarizationConfig;
pub use types::SpeakerSegment;
pub use engine::Sortformer;
