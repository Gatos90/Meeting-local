// Database models - Settings
use serde::{Deserialize, Serialize};

/// A single setting stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    pub key: String,
    pub value: String,
    pub value_type: String,
    pub updated_at: String,
}

/// All settings loaded at startup
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AllSettings {
    pub language: Option<String>,
    pub mic_rnnoise: bool,
    pub mic_highpass: bool,
    pub mic_normalizer: bool,
    pub sys_rnnoise: bool,
    pub sys_highpass: bool,
    pub sys_normalizer: bool,
    pub last_microphone: Option<String>,
    pub last_system_audio: Option<String>,
    pub recordings_folder: Option<String>,
    pub current_model: Option<String>,
}
