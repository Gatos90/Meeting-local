//! Sortformer Configuration and Constants

// Model constants
pub const N_FFT: usize = 512;
pub const WIN_LENGTH: usize = 400;
pub const HOP_LENGTH: usize = 160;
pub const N_MELS: usize = 128;
pub const PREEMPH: f32 = 0.97;
pub const LOG_ZERO_GUARD: f32 = 5.960464478e-8;
pub const SAMPLE_RATE: usize = 16000;
pub const FMIN: f32 = 0.0;
pub const FMAX: f32 = 8000.0;

// Streaming constants
pub const CHUNK_LEN: usize = 124;
pub const FIFO_LEN: usize = 124;
pub const SPKCACHE_LEN: usize = 188;
pub const SPKCACHE_UPDATE_PERIOD: usize = 124;
pub const SUBSAMPLING: usize = 8;
pub const EMB_DIM: usize = 512;
pub const NUM_SPEAKERS: usize = 4;
pub const FRAME_DURATION: f32 = 0.08;

// Cache compression params
pub const SPKCACHE_SIL_FRAMES_PER_SPK: usize = 3;
pub const PRED_SCORE_THRESHOLD: f32 = 0.25;
pub const STRONG_BOOST_RATE: f32 = 0.75;
pub const WEAK_BOOST_RATE: f32 = 1.5;
pub const MIN_POS_SCORES_RATE: f32 = 0.5;
pub const SIL_THRESHOLD: f32 = 0.2;
pub const MAX_INDEX: usize = 99999;

/// Post-processing configuration for speaker diarization
#[derive(Debug, Clone)]
pub struct DiarizationConfig {
    pub onset: f32,
    pub offset: f32,
    pub pad_onset: f32,
    pub pad_offset: f32,
    pub min_duration_on: f32,
    pub min_duration_off: f32,
    pub median_window: usize,
}

impl Default for DiarizationConfig {
    fn default() -> Self {
        Self::callhome()
    }
}

impl DiarizationConfig {
    /// CallHome dataset config (default)
    pub fn callhome() -> Self {
        Self {
            onset: 0.641,
            offset: 0.561,
            pad_onset: 0.229,
            pad_offset: 0.079,
            min_duration_on: 0.511,
            min_duration_off: 0.296,
            median_window: 11,
        }
    }

    /// DIHARD3 dataset config
    pub fn dihard3() -> Self {
        Self {
            onset: 0.56,
            offset: 1.0,
            pad_onset: 0.063,
            pad_offset: 0.002,
            min_duration_on: 0.007,
            min_duration_off: 0.151,
            median_window: 11,
        }
    }
}
