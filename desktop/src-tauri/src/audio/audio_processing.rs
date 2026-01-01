// Audio Processing Module - Re-exports from focused submodules
//
// This file provides backwards compatibility by re-exporting from:
// - processing/: DSP operations (normalization, noise suppression, filters, resampling)
// - file_io/: File operations (audio/transcript writing, utilities)

// Re-export processing functions
pub use super::processing::{
    normalize_v2,
    LoudnessNormalizer,
    TruePeakLimiter,
    NoiseSuppressionProcessor,
    HighPassFilter,
    resample,
    resample_audio,
    spectral_subtraction,
    average_noise_spectrum,
    audio_to_mono,
};

// Re-export file I/O functions
pub use super::file_io::{
    sanitize_filename,
    create_meeting_folder,
    write_audio_to_file,
    write_audio_to_file_with_meeting_name,
    write_transcript_to_file,
    write_transcript_json_to_file,
};
