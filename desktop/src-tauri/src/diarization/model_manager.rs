// Diarization model manager - handles automatic downloading of pyannote models

use std::path::{Path, PathBuf};
use std::sync::Arc;
use anyhow::{Result, anyhow};
use log::{info, error, debug};
use reqwest::Client;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use futures_util::StreamExt;

/// Model URLs for pyannote diarization
/// These models are the official pyannote-rs releases and are compatible with pyannote-rs 0.3.x
const SEGMENTATION_MODEL_URL: &str =
    "https://github.com/thewh1teagle/pyannote-rs/releases/download/v0.1.0/segmentation-3.0.onnx";
const EMBEDDING_MODEL_URL: &str =
    "https://github.com/thewh1teagle/pyannote-rs/releases/download/v0.1.0/wespeaker_en_voxceleb_CAM++.onnx";

/// Expected file names for the models
pub const SEGMENTATION_MODEL_NAME: &str = "segmentation-3.0.onnx";
pub const EMBEDDING_MODEL_NAME: &str = "wespeaker_en_voxceleb_CAM++.onnx";

/// Model info with status
#[derive(Debug, Clone, serde::Serialize)]
pub struct DiarizationModelInfo {
    pub name: String,
    pub size_mb: f64,
    pub is_downloaded: bool,
    pub path: Option<String>,
}

/// Check if diarization models are available
pub fn are_models_available(models_dir: &Path) -> bool {
    let seg_path = models_dir.join(SEGMENTATION_MODEL_NAME);
    let emb_path = models_dir.join(EMBEDDING_MODEL_NAME);

    seg_path.exists() && emb_path.exists()
}

/// Get the paths for diarization models
pub fn get_model_paths(models_dir: &Path) -> (PathBuf, PathBuf) {
    (
        models_dir.join(SEGMENTATION_MODEL_NAME),
        models_dir.join(EMBEDDING_MODEL_NAME),
    )
}

/// Get info about diarization models
pub fn get_models_info(models_dir: &Path) -> Vec<DiarizationModelInfo> {
    let seg_path = models_dir.join(SEGMENTATION_MODEL_NAME);
    let emb_path = models_dir.join(EMBEDDING_MODEL_NAME);

    vec![
        DiarizationModelInfo {
            name: "Segmentation 3.0".to_string(),
            size_mb: 5.9,
            is_downloaded: seg_path.exists(),
            path: if seg_path.exists() { Some(seg_path.to_string_lossy().to_string()) } else { None },
        },
        DiarizationModelInfo {
            name: "WeSpeaker Embedding".to_string(),
            size_mb: 26.5,
            is_downloaded: emb_path.exists(),
            path: if emb_path.exists() { Some(emb_path.to_string_lossy().to_string()) } else { None },
        },
    ]
}

/// Progress callback type that is Send + Sync
pub type ProgressCallback = Arc<dyn Fn(u8, &str) + Send + Sync>;

/// Download a file with progress reporting
async fn download_file(
    url: &str,
    dest_path: &Path,
    progress_callback: Option<ProgressCallback>,
    model_name: &str,
) -> Result<()> {
    info!("Downloading {} from {}", model_name, url);

    // Create parent directory if needed
    if let Some(parent) = dest_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).await?;
        }
    }

    let client = Client::new();
    let response = client.get(url).send().await
        .map_err(|e| anyhow!("Failed to start download: {}", e))?;

    if !response.status().is_success() {
        return Err(anyhow!("Download failed with status: {}", response.status()));
    }

    let total_size = response.content_length().unwrap_or(0);
    info!("Downloading {} ({:.1} MB)", model_name, total_size as f64 / (1024.0 * 1024.0));

    // Create temp file first
    let temp_path = dest_path.with_extension("tmp");
    let mut file = fs::File::create(&temp_path).await
        .map_err(|e| anyhow!("Failed to create temp file: {}", e))?;

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();
    let mut last_progress: u8 = 0;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| anyhow!("Download error: {}", e))?;

        file.write_all(&chunk).await
            .map_err(|e| anyhow!("Failed to write chunk: {}", e))?;

        downloaded += chunk.len() as u64;

        if total_size > 0 {
            let progress = ((downloaded as f64 / total_size as f64) * 100.0) as u8;
            if progress != last_progress {
                last_progress = progress;
                if let Some(ref cb) = progress_callback {
                    cb(progress, model_name);
                }
                debug!("Download progress for {}: {}%", model_name, progress);
            }
        }
    }

    file.flush().await?;
    drop(file);

    // Rename temp file to final name
    fs::rename(&temp_path, dest_path).await
        .map_err(|e| anyhow!("Failed to rename temp file: {}", e))?;

    info!("Successfully downloaded {} to {:?}", model_name, dest_path);

    Ok(())
}

/// Download diarization models if they don't exist
pub async fn ensure_models_downloaded(
    models_dir: &Path,
    progress_callback: Option<ProgressCallback>,
) -> Result<(PathBuf, PathBuf)> {
    // Create models directory if needed
    if !models_dir.exists() {
        fs::create_dir_all(models_dir).await?;
    }

    let seg_path = models_dir.join(SEGMENTATION_MODEL_NAME);
    let emb_path = models_dir.join(EMBEDDING_MODEL_NAME);

    // Download segmentation model if needed
    if !seg_path.exists() {
        info!("Segmentation model not found, downloading...");
        download_file(
            SEGMENTATION_MODEL_URL,
            &seg_path,
            progress_callback.clone(),
            "Segmentation Model",
        ).await?;
    } else {
        info!("Segmentation model already exists at {:?}", seg_path);
    }

    // Download embedding model if needed
    if !emb_path.exists() {
        info!("Embedding model not found, downloading...");
        download_file(
            EMBEDDING_MODEL_URL,
            &emb_path,
            progress_callback,
            "Embedding Model",
        ).await?;
    } else {
        info!("Embedding model already exists at {:?}", emb_path);
    }

    Ok((seg_path, emb_path))
}

/// Tauri command to download diarization models
#[tauri::command]
pub async fn download_diarization_models(
    app: tauri::AppHandle,
) -> Result<(), String> {
    use tauri::Emitter;

    // Get models directory (same as Whisper models)
    let models_dir = get_models_directory(&app)?;

    info!("Downloading diarization models to {:?}", models_dir);

    let app_clone = app.clone();
    let progress_callback: ProgressCallback = Arc::new(move |progress: u8, model_name: &str| {
        let _ = app_clone.emit("diarization-model-download-progress", serde_json::json!({
            "progress": progress,
            "model": model_name,
        }));
    });

    match ensure_models_downloaded(&models_dir, Some(progress_callback)).await {
        Ok((seg_path, emb_path)) => {
            info!("Diarization models downloaded successfully");
            let _ = app.emit("diarization-models-ready", serde_json::json!({
                "segmentation_path": seg_path.to_string_lossy(),
                "embedding_path": emb_path.to_string_lossy(),
            }));
            Ok(())
        }
        Err(e) => {
            error!("Failed to download diarization models: {}", e);
            let _ = app.emit("diarization-model-download-error", serde_json::json!({
                "error": e.to_string(),
            }));
            Err(e.to_string())
        }
    }
}

/// Tauri command to check if diarization models are available
#[tauri::command]
pub async fn check_diarization_models(
    app: tauri::AppHandle,
) -> Result<Vec<DiarizationModelInfo>, String> {
    let models_dir = get_models_directory(&app)?;
    Ok(get_models_info(&models_dir))
}

/// Tauri command to get diarization models status
#[tauri::command]
pub async fn are_diarization_models_ready(
    app: tauri::AppHandle,
) -> Result<bool, String> {
    let models_dir = get_models_directory(&app)?;
    Ok(are_models_available(&models_dir))
}

/// Get the models directory path
fn get_models_directory(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    use tauri::Manager;

    // Use the same directory as Whisper models
    let app_data_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    Ok(app_data_dir.join("models"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_model_paths() {
        let dir = tempdir().unwrap();
        let (seg, emb) = get_model_paths(dir.path());

        assert!(seg.to_string_lossy().contains(SEGMENTATION_MODEL_NAME));
        assert!(emb.to_string_lossy().contains(EMBEDDING_MODEL_NAME));
    }

    #[test]
    fn test_models_not_available() {
        let dir = tempdir().unwrap();
        assert!(!are_models_available(dir.path()));
    }
}
