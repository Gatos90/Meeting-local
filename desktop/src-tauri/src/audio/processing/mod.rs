// Audio Processing Module
//
// Split into focused files:
// - normalizer.rs: Volume normalization, EBU R128, peak limiting
// - noise_suppression.rs: RNNoise-based noise reduction
// - filters.rs: High-pass and other filters
// - resampling.rs: Sample rate conversion
// - spectral.rs: Spectral operations

pub mod normalizer;
pub mod noise_suppression;
pub mod filters;
pub mod resampling;
pub mod spectral;

// Re-export for backwards compatibility
pub use normalizer::{normalize_v2, LoudnessNormalizer, TruePeakLimiter};
pub use noise_suppression::NoiseSuppressionProcessor;
pub use filters::HighPassFilter;
pub use resampling::{resample, resample_audio};
pub use spectral::{spectral_subtraction, average_noise_spectrum, audio_to_mono};
