//! Sortformer diarization provider and Tauri commands
//!
//! Provides speaker diarization using NVIDIA Sortformer v2 model

use super::sortformer::{Sortformer, DiarizationConfig, SpeakerSegment};
use anyhow::Result;
use log::{info, debug};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use once_cell::sync::Lazy;

/// Global Sortformer engine instance
pub static SORTFORMER_ENGINE: Lazy<Arc<RwLock<Option<SortformerEngine>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

/// Sortformer model file name
pub const SORTFORMER_MODEL_NAME: &str = "diar_streaming_sortformer_4spk-v2.onnx";

/// Sortformer model download URL
pub const SORTFORMER_MODEL_URL: &str =
    "https://huggingface.co/altunenes/parakeet-rs/resolve/main/diar_streaming_sortformer_4spk-v2.1.onnx";

/// Sortformer diarization engine wrapper
pub struct SortformerEngine {
    sortformer: Sortformer,
}

impl SortformerEngine {
    /// Create a new engine from model path
    pub fn new(model_path: PathBuf) -> Result<Self> {
        info!("Initializing Sortformer engine from {:?}", model_path);
        let sortformer = Sortformer::new(&model_path)?;
        info!("Sortformer engine initialized successfully");
        Ok(Self { sortformer })
    }

    /// Create with custom config
    pub fn with_config(model_path: PathBuf, config: DiarizationConfig) -> Result<Self> {
        info!("Initializing Sortformer engine with custom config");
        let sortformer = Sortformer::with_config(&model_path, config)?;
        Ok(Self { sortformer })
    }

    /// Run diarization on audio samples
    pub fn diarize(&mut self, samples: Vec<f32>, sample_rate: u32) -> Result<Vec<SpeakerSegment>> {
        debug!("Running Sortformer diarization on {} samples", samples.len());
        self.sortformer.diarize(samples, sample_rate, 1)
    }

    /// Reset the streaming state
    pub fn reset(&mut self) {
        self.sortformer.reset_state();
    }

    /// Check if model is available
    pub fn is_model_available(models_dir: &std::path::Path) -> bool {
        models_dir.join(SORTFORMER_MODEL_NAME).exists()
    }

    /// Get the model path
    pub fn get_model_path(models_dir: &std::path::Path) -> PathBuf {
        models_dir.join(SORTFORMER_MODEL_NAME)
    }
}

/// Initialize the global Sortformer engine
pub async fn init_sortformer_engine(model_path: PathBuf) -> Result<()> {
    let engine = SortformerEngine::new(model_path)?;
    let mut guard = SORTFORMER_ENGINE.write().await;
    *guard = Some(engine);
    info!("Global Sortformer engine initialized");
    Ok(())
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// Initialize Sortformer engine
#[tauri::command]
pub async fn init_sortformer(model_path: String) -> Result<(), String> {
    init_sortformer_engine(PathBuf::from(model_path))
        .await
        .map_err(|e| e.to_string())
}

/// Check if Sortformer model is available
#[tauri::command]
pub async fn is_sortformer_model_available(app: tauri::AppHandle) -> Result<bool, String> {
    use tauri::Manager;

    let models_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("models");

    Ok(SortformerEngine::is_model_available(&models_dir))
}

/// Download Sortformer model
#[tauri::command]
pub async fn download_sortformer_model(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::{Manager, Emitter};
    use tokio::fs;
    use tokio::io::AsyncWriteExt;
    use futures_util::StreamExt;

    let models_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("models");

    if !models_dir.exists() {
        fs::create_dir_all(&models_dir)
            .await
            .map_err(|e| format!("Failed to create models dir: {}", e))?;
    }

    let model_path = models_dir.join(SORTFORMER_MODEL_NAME);

    if model_path.exists() {
        info!("Sortformer model already exists at {:?}", model_path);
        return Ok(());
    }

    info!("Downloading Sortformer model from {}", SORTFORMER_MODEL_URL);

    let client = reqwest::Client::new();
    let response = client
        .get(SORTFORMER_MODEL_URL)
        .send()
        .await
        .map_err(|e| format!("Failed to start download: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()));
    }

    let total_size = response.content_length().unwrap_or(0);
    info!(
        "Downloading Sortformer model ({:.1} MB)",
        total_size as f64 / (1024.0 * 1024.0)
    );

    let temp_path = model_path.with_extension("tmp");
    let mut file = fs::File::create(&temp_path)
        .await
        .map_err(|e| format!("Failed to create temp file: {}", e))?;

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();
    let mut last_progress: u8 = 0;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| format!("Download error: {}", e))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Failed to write chunk: {}", e))?;

        downloaded += chunk.len() as u64;

        if total_size > 0 {
            let progress = ((downloaded as f64 / total_size as f64) * 100.0) as u8;
            if progress != last_progress && progress % 5 == 0 {
                last_progress = progress;
                let _ = app.emit(
                    "sortformer-download-progress",
                    serde_json::json!({ "progress": progress }),
                );
                debug!("Sortformer download progress: {}%", progress);
            }
        }
    }

    file.flush()
        .await
        .map_err(|e| format!("Failed to flush file: {}", e))?;
    drop(file);

    fs::rename(&temp_path, &model_path)
        .await
        .map_err(|e| format!("Failed to rename temp file: {}", e))?;

    info!("Successfully downloaded Sortformer model to {:?}", model_path);

    let _ = app.emit("sortformer-download-complete", serde_json::json!({}));

    Ok(())
}

/// Diarize audio using Sortformer
#[tauri::command]
pub async fn sortformer_diarize(
    samples: Vec<f32>,
    sample_rate: u32,
) -> Result<Vec<serde_json::Value>, String> {
    let mut guard = SORTFORMER_ENGINE.write().await;
    let engine = guard
        .as_mut()
        .ok_or("Sortformer engine not initialized")?;

    let segments = engine
        .diarize(samples, sample_rate)
        .map_err(|e| e.to_string())?;

    // Convert to JSON-serializable format
    Ok(segments
        .into_iter()
        .map(|seg| {
            serde_json::json!({
                "start": seg.start,
                "end": seg.end,
                "speaker_id": seg.speaker_id,
                "speaker_label": format!("Speaker {}", seg.speaker_id + 1)
            })
        })
        .collect())
}

/// Reset Sortformer state
#[tauri::command]
pub async fn sortformer_reset() -> Result<(), String> {
    let mut guard = SORTFORMER_ENGINE.write().await;
    if let Some(engine) = guard.as_mut() {
        engine.reset();
    }
    Ok(())
}

/// Get Sortformer model info
#[tauri::command]
pub async fn get_sortformer_model_info(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    use tauri::Manager;

    let models_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("models");

    let model_path = models_dir.join(SORTFORMER_MODEL_NAME);
    let available = model_path.exists();

    let size = if available {
        std::fs::metadata(&model_path)
            .map(|m| m.len())
            .unwrap_or(0)
    } else {
        0
    };

    Ok(serde_json::json!({
        "name": SORTFORMER_MODEL_NAME,
        "available": available,
        "size_bytes": size,
        "size_mb": size as f64 / (1024.0 * 1024.0),
        "download_url": SORTFORMER_MODEL_URL,
        "max_speakers": 4,
        "description": "NVIDIA Sortformer v2 - Streaming 4-speaker diarization"
    }))
}
