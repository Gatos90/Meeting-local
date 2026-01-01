// Database models - Transcript
use serde::{Deserialize, Serialize};

/// A transcript segment (a piece of transcribed audio)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub id: String,
    pub recording_id: String,
    pub text: String,
    pub audio_start_time: f64,
    pub audio_end_time: f64,
    pub duration: f64,
    pub display_time: String,
    pub confidence: f32,
    pub sequence_id: i64,
    // Speaker diarization fields (optional for backward compatibility)
    #[serde(default)]
    pub speaker_id: Option<String>,
    #[serde(default)]
    pub speaker_label: Option<String>,
    #[serde(default)]
    pub is_registered_speaker: bool,
}

/// A registered speaker with voice profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredSpeakerDb {
    pub id: String,
    pub name: String,
    // embedding stored as BLOB, not exposed to frontend
    pub created_at: String,
    pub sample_count: i32,
    pub last_seen: Option<String>,
}

/// Speaker label override for a specific recording
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerLabel {
    pub recording_id: String,
    pub speaker_id: String,
    pub custom_label: String,
}
