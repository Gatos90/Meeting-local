// Diarization engine using pyannote-rs
// Wraps segmentation and speaker embedding extraction

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use log::{info, debug, warn};

use pyannote_rs::{EmbeddingExtractor, EmbeddingManager, get_segments};

use super::speaker_db::SpeakerDatabase;

/// Global diarization engine instance
pub static DIARIZATION_ENGINE: Lazy<Arc<RwLock<Option<DiarizationEngine>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

/// Configuration for diarization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiarizationConfig {
    /// Path to segmentation model (segmentation-3.0.onnx)
    pub segmentation_model_path: PathBuf,
    /// Path to speaker embedding model (wespeaker_en_voxceleb_CAM++.onnx)
    pub embedding_model_path: PathBuf,
    /// Maximum number of speakers to track
    pub max_speakers: usize,
    /// Similarity threshold for speaker matching (0.0 to 1.0)
    pub similarity_threshold: f32,
}

impl Default for DiarizationConfig {
    fn default() -> Self {
        Self {
            segmentation_model_path: PathBuf::new(),
            embedding_model_path: PathBuf::new(),
            max_speakers: 10,
            similarity_threshold: 0.85,  // Higher threshold = fewer false speaker splits
        }
    }
}

/// A speaker segment with timing and speaker information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerSegment {
    /// Start time in seconds
    pub start_time: f64,
    /// End time in seconds
    pub end_time: f64,
    /// Internal speaker ID (e.g., "speaker_0", "speaker_1")
    pub speaker_id: String,
    /// Display label (e.g., "Speaker 1" or user-assigned name like "John")
    pub speaker_label: String,
    /// Confidence score for speaker identification (0.0 to 1.0)
    pub confidence: f32,
    /// Whether this speaker matches a registered voice
    pub is_registered: bool,
    /// If registered, the registered speaker's ID
    pub registered_speaker_id: Option<String>,
}

/// Diarization engine that identifies speakers in audio
pub struct DiarizationEngine {
    config: DiarizationConfig,
    embedding_extractor: EmbeddingExtractor,
    embedding_manager: EmbeddingManager,
    speaker_db: SpeakerDatabase,
    /// Maps internal speaker IDs to display labels
    speaker_labels: HashMap<String, String>,
    /// Counter for assigning speaker IDs in a session
    speaker_counter: usize,
}

impl DiarizationEngine {
    /// Create a new diarization engine
    pub fn new(config: DiarizationConfig) -> Result<Self> {
        info!("Initializing diarization engine");
        debug!("Segmentation model: {:?}", config.segmentation_model_path);
        debug!("Embedding model: {:?}", config.embedding_model_path);

        // Verify models exist
        if !config.segmentation_model_path.exists() {
            return Err(anyhow!(
                "Segmentation model not found: {:?}",
                config.segmentation_model_path
            ));
        }
        if !config.embedding_model_path.exists() {
            return Err(anyhow!(
                "Embedding model not found: {:?}",
                config.embedding_model_path
            ));
        }

        // Initialize embedding extractor (pyannote-rs uses eyre, convert to anyhow)
        let embedding_extractor = EmbeddingExtractor::new(&config.embedding_model_path)
            .map_err(|e| anyhow!("Failed to create embedding extractor: {}", e))?;

        // Initialize embedding manager for speaker clustering
        let embedding_manager = EmbeddingManager::new(config.max_speakers);

        // Initialize speaker database (for registered voices)
        let speaker_db = SpeakerDatabase::new()?;

        info!("Diarization engine initialized successfully");

        Ok(Self {
            config,
            embedding_extractor,
            embedding_manager,
            speaker_db,
            speaker_labels: HashMap::new(),
            speaker_counter: 0,
        })
    }

    /// Run diarization on audio samples
    ///
    /// Takes f32 samples at any sample rate and returns speaker segments.
    /// Internally converts to i16 at 16kHz for pyannote-rs.
    pub fn diarize(&mut self, samples: &[f32], sample_rate: u32) -> Result<Vec<SpeakerSegment>> {
        info!("Running diarization on {} samples at {} Hz", samples.len(), sample_rate);

        // Convert f32 to i16 samples (pyannote-rs uses i16)
        let samples_i16: Vec<i16> = samples
            .iter()
            .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .collect();

        // Get speech segments from the audio (pyannote-rs uses eyre)
        let segments_iter = get_segments(&samples_i16, sample_rate, &self.config.segmentation_model_path)
            .map_err(|e| anyhow!("Failed to run segmentation: {}", e))?;

        let mut speaker_segments = Vec::new();

        // Process each detected speech segment
        for segment_result in segments_iter {
            let segment = match segment_result {
                Ok(seg) => seg,
                Err(e) => {
                    warn!("Failed to process segment: {}", e);
                    continue;
                }
            };

            // Extract speaker embedding for this segment
            let embedding: Vec<f32> = match self.embedding_extractor.compute(&segment.samples) {
                Ok(iter) => iter.collect(),
                Err(e) => {
                    warn!("Failed to compute embedding for segment: {}", e);
                    continue;
                }
            };

            // Find or create speaker for this embedding
            let (speaker_id, speaker_label, confidence, is_registered, registered_id) =
                self.identify_speaker(&embedding)?;

            speaker_segments.push(SpeakerSegment {
                start_time: segment.start,
                end_time: segment.end,
                speaker_id,
                speaker_label,
                confidence,
                is_registered,
                registered_speaker_id: registered_id,
            });
        }

        info!("Diarization complete: {} segments from {} speakers",
              speaker_segments.len(),
              self.speaker_counter);

        Ok(speaker_segments)
    }

    /// Identify speaker from embedding, checking registered voices first
    fn identify_speaker(&mut self, embedding: &[f32]) -> Result<(String, String, f32, bool, Option<String>)> {
        // First, check against registered speakers
        if let Some((registered_id, registered_name, similarity)) =
            self.speaker_db.find_matching_speaker(embedding, self.config.similarity_threshold)?
        {
            debug!("Matched registered speaker '{}' with similarity {:.2}", registered_name, similarity);
            return Ok((
                format!("registered_{}", registered_id),
                registered_name,
                similarity,
                true,
                Some(registered_id),
            ));
        }

        // Not a registered speaker, use session-based clustering
        let embedding_vec: Vec<f32> = embedding.to_vec();

        // search_speaker returns the speaker index if found or creates a new one
        // If threshold is not met and capacity allows, it adds a new speaker
        if let Some(speaker_idx) = self.embedding_manager.search_speaker(
            embedding_vec.clone(),
            self.config.similarity_threshold,
        ) {
            let speaker_id = format!("speaker_{}", speaker_idx);

            // Check if we already have a label for this speaker
            if let Some(label) = self.speaker_labels.get(&speaker_id) {
                return Ok((speaker_id, label.clone(), 0.85, false, None));
            }

            // New speaker detected by embedding manager
            let speaker_label = format!("Speaker {}", speaker_idx + 1);
            self.speaker_labels.insert(speaker_id.clone(), speaker_label.clone());

            // Update counter if this is a new speaker
            if speaker_idx >= self.speaker_counter {
                self.speaker_counter = speaker_idx + 1;
            }

            return Ok((speaker_id, speaker_label, 0.75, false, None));
        }

        // Fallback: max speakers reached, assign to "Unknown"
        warn!("Max speakers ({}) reached, segment assigned to 'Unknown'", self.config.max_speakers);
        Ok(("unknown".to_string(), "Unknown".to_string(), 0.3, false, None))
    }

    /// Register a new voice for future recognition
    pub fn register_voice(&mut self, name: &str, samples: &[f32]) -> Result<String> {
        info!("Registering voice for '{}'", name);

        // Convert samples to i16
        let samples_i16: Vec<i16> = samples
            .iter()
            .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .collect();

        // Extract embedding (pyannote-rs uses eyre)
        let embedding: Vec<f32> = self.embedding_extractor.compute(&samples_i16)
            .map_err(|e| anyhow!("Failed to compute embedding for voice registration: {}", e))?
            .collect();

        // Save to database
        let speaker_id = self.speaker_db.register_speaker(name, &embedding)?;

        info!("Successfully registered voice '{}' with ID {}", name, speaker_id);
        Ok(speaker_id)
    }

    /// Remove a registered voice
    pub fn unregister_voice(&mut self, speaker_id: &str) -> Result<()> {
        info!("Unregistering voice: {}", speaker_id);
        self.speaker_db.unregister_speaker(speaker_id)
    }

    /// Get all registered speakers
    pub fn get_registered_speakers(&self) -> Result<Vec<super::speaker_db::RegisteredSpeaker>> {
        self.speaker_db.get_all_speakers()
    }

    /// Rename a speaker label for a specific session
    pub fn rename_speaker(&mut self, speaker_id: &str, new_label: &str) {
        self.speaker_labels.insert(speaker_id.to_string(), new_label.to_string());
        debug!("Renamed {} to '{}'", speaker_id, new_label);
    }

    /// Reset session state (speaker counter and labels)
    /// Call this when starting a new recording session
    pub fn reset_session(&mut self) {
        self.speaker_counter = 0;
        self.speaker_labels.clear();
        self.embedding_manager = EmbeddingManager::new(self.config.max_speakers);
        info!("Diarization session reset");
    }

    /// Update configuration dynamically (for retranscription with different settings)
    pub fn update_config(&mut self, max_speakers: Option<usize>, threshold: Option<f32>) {
        if let Some(max) = max_speakers {
            self.config.max_speakers = max;
            // Recreate embedding manager with new capacity
            self.embedding_manager = EmbeddingManager::new(max);
            info!("Updated max_speakers to {}", max);
        }
        if let Some(t) = threshold {
            self.config.similarity_threshold = t;
            info!("Updated similarity_threshold to {:.2}", t);
        }
        // Reset session state when config changes
        self.speaker_counter = 0;
        self.speaker_labels.clear();
    }

    /// Check if the engine is ready
    pub fn is_ready(&self) -> bool {
        true // If we got here, the engine is initialized
    }
}

/// Initialize the global diarization engine
pub async fn init_diarization_engine(config: DiarizationConfig) -> Result<()> {
    let engine = DiarizationEngine::new(config)?;

    let mut guard = DIARIZATION_ENGINE.write().await;
    *guard = Some(engine);

    info!("Global diarization engine initialized");
    Ok(())
}

/// Get a reference to the global diarization engine
pub async fn get_diarization_engine() -> Option<Arc<RwLock<Option<DiarizationEngine>>>> {
    Some(DIARIZATION_ENGINE.clone())
}

/// Tauri command to initialize diarization
#[tauri::command]
pub async fn init_diarization(
    segmentation_model_path: String,
    embedding_model_path: String,
) -> Result<(), String> {
    let config = DiarizationConfig {
        segmentation_model_path: PathBuf::from(segmentation_model_path),
        embedding_model_path: PathBuf::from(embedding_model_path),
        max_speakers: 10,
        similarity_threshold: 0.5,
    };

    init_diarization_engine(config)
        .await
        .map_err(|e| e.to_string())
}

/// Tauri command to diarize an audio file
#[tauri::command]
pub async fn diarize_audio(
    samples: Vec<f32>,
    sample_rate: u32,
) -> Result<Vec<SpeakerSegment>, String> {
    let mut guard = DIARIZATION_ENGINE.write().await;
    let engine = guard.as_mut().ok_or("Diarization engine not initialized")?;

    engine.diarize(&samples, sample_rate).map_err(|e| e.to_string())
}

/// Tauri command to register a speaker voice
#[tauri::command]
pub async fn register_speaker_voice(
    name: String,
    audio_samples: Vec<f32>,
) -> Result<String, String> {
    let mut guard = DIARIZATION_ENGINE.write().await;
    let engine = guard.as_mut().ok_or("Diarization engine not initialized")?;

    engine.register_voice(&name, &audio_samples).map_err(|e| e.to_string())
}

/// Tauri command to get all registered speakers
#[tauri::command]
pub async fn get_registered_speakers() -> Result<Vec<super::speaker_db::RegisteredSpeaker>, String> {
    let guard = DIARIZATION_ENGINE.read().await;
    let engine = guard.as_ref().ok_or("Diarization engine not initialized")?;

    engine.get_registered_speakers().map_err(|e| e.to_string())
}

/// Tauri command to delete a registered speaker
#[tauri::command]
pub async fn delete_registered_speaker(speaker_id: String) -> Result<(), String> {
    let mut guard = DIARIZATION_ENGINE.write().await;
    let engine = guard.as_mut().ok_or("Diarization engine not initialized")?;

    engine.unregister_voice(&speaker_id).map_err(|e| e.to_string())
}

/// Tauri command to rename a speaker in the current session
#[tauri::command]
pub async fn rename_speaker(
    speaker_id: String,
    new_label: String,
) -> Result<(), String> {
    let mut guard = DIARIZATION_ENGINE.write().await;
    let engine = guard.as_mut().ok_or("Diarization engine not initialized")?;

    engine.rename_speaker(&speaker_id, &new_label);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DiarizationConfig::default();
        assert_eq!(config.max_speakers, 10);
        assert_eq!(config.similarity_threshold, 0.5);
    }
}
