// Database models - Recording
use serde::{Deserialize, Serialize};
use super::{Category, Tag};

/// A recording (meeting) entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recording {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub duration_seconds: Option<f64>,
    pub status: String,
    pub audio_file_path: Option<String>,
    pub meeting_folder_path: Option<String>,
    pub microphone_device: Option<String>,
    pub system_audio_device: Option<String>,
    pub sample_rate: i32,
    pub transcription_model: Option<String>,
    pub language: Option<String>,
    pub diarization_provider: Option<String>,
}

impl Recording {
    pub fn new(id: String, title: String) -> Self {
        Self {
            id,
            title,
            created_at: chrono::Utc::now().to_rfc3339(),
            completed_at: None,
            duration_seconds: None,
            status: "recording".to_string(),
            audio_file_path: None,
            meeting_folder_path: None,
            microphone_device: None,
            system_audio_device: None,
            sample_rate: 48000,
            transcription_model: None,
            language: None,
            diarization_provider: None,
        }
    }
}

/// Updates that can be applied to a recording
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecordingUpdate {
    pub title: Option<String>,
    pub completed_at: Option<String>,
    pub duration_seconds: Option<f64>,
    pub status: Option<String>,
    pub audio_file_path: Option<String>,
    pub meeting_folder_path: Option<String>,
    pub transcription_model: Option<String>,
    pub diarization_provider: Option<String>,
}

/// A recording with its associated categories and tags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingWithMetadata {
    pub recording: Recording,
    pub categories: Vec<Category>,
    pub tags: Vec<Tag>,
    pub transcript_count: i32,
}
