// Whisper Engine - Model Downloading
use std::path::PathBuf;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use reqwest::Client;
use anyhow::{Result, anyhow};

use super::types::{ModelStatus, ModelInfo};
use super::model_registry::get_model_url;

/// Download a model from HuggingFace
pub async fn download_model(
    model_name: &str,
    models_dir: &PathBuf,
    available_models: &RwLock<HashMap<String, ModelInfo>>,
    active_downloads: &RwLock<HashSet<String>>,
    cancel_download_flag: &RwLock<Option<String>>,
    progress_callback: Option<Box<dyn Fn(u8) + Send>>,
) -> Result<()> {
    log::info!("Starting download for model: {}", model_name);

    // Check if download is already in progress
    {
        let active = active_downloads.read().await;
        if active.contains(model_name) {
            log::warn!("Download already in progress for model: {}", model_name);
            return Err(anyhow!("Download already in progress for model: {}", model_name));
        }
    }

    // Add to active downloads
    {
        let mut active = active_downloads.write().await;
        active.insert(model_name.to_string());
    }

    // Clear any previous cancellation flag
    {
        let mut cancel_flag = cancel_download_flag.write().await;
        *cancel_flag = None;
    }

    // Get model URL
    let model_url = get_model_url(model_name)
        .ok_or_else(|| anyhow!("Unsupported model: {}", model_name))?;

    log::info!("Model URL for {}: {}", model_name, model_url);

    // Generate filename
    let filename = format!("ggml-{}.bin", model_name);
    let file_path = models_dir.join(&filename);

    log::info!("Downloading to file path: {}", file_path.display());

    // Create models directory if it doesn't exist
    if !models_dir.exists() {
        fs::create_dir_all(models_dir).await
            .map_err(|e| anyhow!("Failed to create models directory: {}", e))?;
    }

    // Update model status to downloading
    {
        let mut models = available_models.write().await;
        if let Some(model_info) = models.get_mut(model_name) {
            model_info.status = ModelStatus::Downloading { progress: 0 };
        }
    }

    log::info!("Creating HTTP client and starting request...");
    let client = Client::new();

    log::info!("Sending GET request to: {}", model_url);
    let response = client.get(model_url).send().await
        .map_err(|e| anyhow!("Failed to start download: {}", e))?;

    log::info!("Received response with status: {}", response.status());
    if !response.status().is_success() {
        let mut active = active_downloads.write().await;
        active.remove(model_name);
        return Err(anyhow!("Download failed with status: {}", response.status()));
    }

    let total_size = response.content_length().unwrap_or(0);
    log::info!("Response successful, content length: {} bytes ({:.1} MB)", total_size, total_size as f64 / (1024.0 * 1024.0));

    let mut file = fs::File::create(&file_path).await
        .map_err(|e| anyhow!("Failed to create file: {}", e))?;

    log::info!("File created successfully at: {}", file_path.display());

    // Stream download with progress reporting
    use futures_util::StreamExt;
    let mut stream = response.bytes_stream();
    let mut downloaded = 0u64;
    let mut last_progress_report = 0u8;
    let mut last_report_time = std::time::Instant::now();

    // Emit initial 0% progress
    if let Some(ref callback) = progress_callback {
        callback(0);
    }

    while let Some(chunk_result) = stream.next().await {
        // Check for cancellation
        {
            let cancel_flag = cancel_download_flag.read().await;
            if cancel_flag.as_ref() == Some(&model_name.to_string()) {
                log::info!("Download cancelled for {}", model_name);
                let mut active = active_downloads.write().await;
                active.remove(model_name);
                return Err(anyhow!("Download cancelled by user"));
            }
        }

        let chunk = chunk_result
            .map_err(|e| anyhow!("Failed to read chunk: {}", e))?;

        file.write_all(&chunk).await
            .map_err(|e| anyhow!("Failed to write chunk to file: {}", e))?;

        downloaded += chunk.len() as u64;

        // Calculate progress
        let progress = if total_size > 0 {
            ((downloaded as f64 / total_size as f64) * 100.0) as u8
        } else {
            0
        };

        // Report progress every 1% or every 2 seconds
        let time_since_last_report = last_report_time.elapsed().as_secs();
        if progress >= last_progress_report + 1 || progress == 100 || time_since_last_report >= 2 {
            log::info!("Download progress: {}% ({:.1} MB / {:.1} MB)",
                     progress,
                     downloaded as f64 / (1024.0 * 1024.0),
                     total_size as f64 / (1024.0 * 1024.0));

            // Update progress in model info
            {
                let mut models = available_models.write().await;
                if let Some(model_info) = models.get_mut(model_name) {
                    model_info.status = ModelStatus::Downloading { progress };
                }
            }

            // Call progress callback
            if let Some(ref callback) = progress_callback {
                callback(progress);
            }

            last_progress_report = progress;
            last_report_time = std::time::Instant::now();
        }
    }

    log::info!("Streaming download completed: {} bytes", downloaded);

    // Ensure 100% progress is reported
    {
        let mut models = available_models.write().await;
        if let Some(model_info) = models.get_mut(model_name) {
            model_info.status = ModelStatus::Downloading { progress: 100 };
        }
    }

    if let Some(ref callback) = progress_callback {
        callback(100);
    }

    file.flush().await
        .map_err(|e| anyhow!("Failed to flush file: {}", e))?;

    log::info!("Download completed for model: {}", model_name);

    // Update model status to available
    {
        let mut models = available_models.write().await;
        if let Some(model_info) = models.get_mut(model_name) {
            model_info.status = ModelStatus::Available;
            model_info.path = file_path.clone();
        }
    }

    // Remove from active downloads
    {
        let mut active = active_downloads.write().await;
        active.remove(model_name);
    }

    Ok(())
}

/// Cancel an ongoing download
pub async fn cancel_download(
    model_name: &str,
    models_dir: &PathBuf,
    available_models: &RwLock<HashMap<String, ModelInfo>>,
    active_downloads: &RwLock<HashSet<String>>,
    cancel_download_flag: &RwLock<Option<String>>,
) -> Result<()> {
    log::info!("Cancelling download for model: {}", model_name);

    // Set cancellation flag
    {
        let mut cancel_flag = cancel_download_flag.write().await;
        *cancel_flag = Some(model_name.to_string());
    }

    // Remove from active downloads
    {
        let mut active = active_downloads.write().await;
        active.remove(model_name);
    }

    // Update model status to Missing
    {
        let mut models = available_models.write().await;
        if let Some(model_info) = models.get_mut(model_name) {
            model_info.status = ModelStatus::Missing;
        }
    }

    // Clean up partially downloaded files
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let filename = format!("ggml-{}.bin", model_name);
    let file_path = models_dir.join(&filename);
    if file_path.exists() {
        if let Err(e) = fs::remove_file(&file_path).await {
            log::warn!("Failed to clean up cancelled download file: {}", e);
        } else {
            log::info!("Cleaned up cancelled download file: {}", file_path.display());
        }
    }

    Ok(())
}

/// Delete a model file
pub async fn delete_model(
    model_name: &str,
    available_models: &RwLock<HashMap<String, ModelInfo>>,
) -> Result<String> {
    log::info!("Attempting to delete model: {}", model_name);

    // Get model info
    let model_info = {
        let models = available_models.read().await;
        models.get(model_name).cloned()
    };

    let model_info = model_info.ok_or_else(|| anyhow!("Model '{}' not found", model_name))?;

    log::info!("Model '{}' has status: {:?}", model_name, model_info.status);

    match &model_info.status {
        ModelStatus::Corrupted { file_size, expected_min_size } => {
            log::info!("Deleting corrupted model '{}' (file size: {} bytes, expected min: {} bytes)",
                      model_name, file_size, expected_min_size);

            if model_info.path.exists() {
                fs::remove_file(&model_info.path).await
                    .map_err(|e| anyhow!("Failed to delete file '{}': {}", model_info.path.display(), e))?;
                log::info!("Successfully deleted corrupted file: {}", model_info.path.display());
            }

            // Update status to Missing
            {
                let mut models = available_models.write().await;
                if let Some(model) = models.get_mut(model_name) {
                    model.status = ModelStatus::Missing;
                }
            }

            Ok(format!("Successfully deleted corrupted model '{}'", model_name))
        }
        ModelStatus::Available => {
            log::info!("Deleting available model '{}' (for cleanup)", model_name);

            if model_info.path.exists() {
                fs::remove_file(&model_info.path).await
                    .map_err(|e| anyhow!("Failed to delete file '{}': {}", model_info.path.display(), e))?;
                log::info!("Successfully deleted model file: {}", model_info.path.display());
            }

            // Update status to Missing
            {
                let mut models = available_models.write().await;
                if let Some(model) = models.get_mut(model_name) {
                    model.status = ModelStatus::Missing;
                }
            }

            Ok(format!("Successfully deleted model '{}'", model_name))
        }
        _ => {
            Err(anyhow!("Can only delete corrupted or available models. Model '{}' has status: {:?}", model_name, model_info.status))
        }
    }
}
