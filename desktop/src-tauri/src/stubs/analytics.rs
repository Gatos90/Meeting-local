//! Analytics stub module
//!
//! Provides no-op implementations for analytics tracking.

/// Track meeting ended event - no-op stub
pub async fn track_meeting_ended(
    _transcription_provider: String,
    _transcription_model: String,
    _summary_provider: String,
    _summary_model: String,
    _total_duration: Option<f64>,
    _active_duration: f64,
    _pause_duration: f64,
    _microphone_device_type: String,
    _system_audio_device_type: String,
    _chunks_processed: u64,
    _transcript_segments_count: u64,
    _had_fatal_error: bool,
) -> Result<(), String> {
    // No-op: analytics removed
    Ok(())
}
