// audio/transcription/globals.rs
//
// Global state for transcription: counters, flags, and settings.

use log::info;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Sequence counter for transcript updates (monotonically increasing)
pub static SEQUENCE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Speech detection flag - reset per recording session
pub static SPEECH_DETECTED_EMITTED: AtomicBool = AtomicBool::new(false);

/// Live diarization enabled flag - controlled via settings
pub static LIVE_DIARIZATION_ENABLED: AtomicBool = AtomicBool::new(false);

/// Enable or disable live speaker diarization
pub fn set_live_diarization_enabled(enabled: bool) {
    LIVE_DIARIZATION_ENABLED.store(enabled, Ordering::SeqCst);
    info!("Live diarization {}", if enabled { "enabled" } else { "disabled" });
}

/// Check if live diarization is enabled
pub fn is_live_diarization_enabled() -> bool {
    LIVE_DIARIZATION_ENABLED.load(Ordering::SeqCst)
}

/// Reset the speech detected flag for a new recording session
pub fn reset_speech_detected_flag() {
    SPEECH_DETECTED_EMITTED.store(false, Ordering::SeqCst);
    info!("🔍 SPEECH_DETECTED_EMITTED reset to: {}", SPEECH_DETECTED_EMITTED.load(Ordering::SeqCst));
}

/// Get the next sequence ID for transcript updates
pub fn next_sequence_id() -> u64 {
    SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Check if speech has been detected in the current session
pub fn was_speech_detected() -> bool {
    SPEECH_DETECTED_EMITTED.load(Ordering::SeqCst)
}

/// Mark that speech has been detected
pub fn mark_speech_detected() -> bool {
    // Returns true if this is the first detection (flag was previously false)
    !SPEECH_DETECTED_EMITTED.swap(true, Ordering::SeqCst)
}
