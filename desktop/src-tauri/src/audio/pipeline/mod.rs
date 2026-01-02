//! Audio pipeline module
//!
//! This module provides:
//! - Ring buffer for synchronized audio mixing
//! - Professional audio mixer
//! - Audio capture from devices
//! - VAD-driven audio processing pipeline
//! - Pipeline manager for lifecycle control

pub mod ring_buffer;
pub mod mixer;
pub mod capture;
pub mod processor;
pub mod manager;

// Re-export main types
pub use capture::AudioCapture;
pub use processor::AudioPipeline;
pub use manager::AudioPipelineManager;
