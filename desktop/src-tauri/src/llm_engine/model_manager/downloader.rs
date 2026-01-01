//! LLM Model Download Logic

use std::path::PathBuf;

use crate::llm_engine::provider::LlmError;
use super::types::{DownloadProgress, DownloadStatus};
use super::registry::available_models;

/// Download a model with progress callback
/// Returns the path to the downloaded model
pub async fn download_model<F>(
    models_dir: &PathBuf,
    model_id: &str,
    on_progress: F,
) -> Result<PathBuf, LlmError>
where
    F: Fn(DownloadProgress) + Send + 'static,
{
    // Find the model in available models
    let model = available_models()
        .into_iter()
        .find(|m| m.id == model_id)
        .ok_or_else(|| LlmError::ModelNotFound(model_id.to_string()))?;

    let dest_path = models_dir.join(format!("{}.gguf", model_id));

    // Report starting
    on_progress(DownloadProgress {
        model_id: model_id.to_string(),
        downloaded_bytes: 0,
        total_bytes: model.size_bytes,
        percent: 0.0,
        status: DownloadStatus::Downloading,
    });

    // Create HTTP client with timeout
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3600)) // 1 hour timeout for large models
        .build()
        .map_err(|e| LlmError::Other(format!("Failed to create HTTP client: {}", e)))?;

    // Start download
    let response = client
        .get(&model.url)
        .send()
        .await
        .map_err(|e| LlmError::Other(format!("Failed to start download: {}", e)))?;

    if !response.status().is_success() {
        return Err(LlmError::Other(format!(
            "Download failed with status: {}",
            response.status()
        )));
    }

    // Get content length
    let total_size = response
        .content_length()
        .unwrap_or(model.size_bytes);

    // Create temp file for download
    let temp_path = dest_path.with_extension("gguf.tmp");
    let mut file = tokio::fs::File::create(&temp_path)
        .await
        .map_err(|e| LlmError::Other(format!("Failed to create temp file: {}", e)))?;

    // Stream download with progress
    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();
    let model_id_owned = model_id.to_string();

    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result
            .map_err(|e| LlmError::Other(format!("Download error: {}", e)))?;

        file.write_all(&chunk)
            .await
            .map_err(|e| LlmError::Other(format!("Failed to write chunk: {}", e)))?;

        downloaded += chunk.len() as u64;
        let percent = (downloaded as f32 / total_size as f32) * 100.0;

        on_progress(DownloadProgress {
            model_id: model_id_owned.clone(),
            downloaded_bytes: downloaded,
            total_bytes: total_size,
            percent,
            status: DownloadStatus::Downloading,
        });
    }

    // Flush and close file
    file.flush()
        .await
        .map_err(|e| LlmError::Other(format!("Failed to flush file: {}", e)))?;
    drop(file);

    // Verify download (basic size check)
    on_progress(DownloadProgress {
        model_id: model_id.to_string(),
        downloaded_bytes: downloaded,
        total_bytes: total_size,
        percent: 100.0,
        status: DownloadStatus::Verifying,
    });

    let metadata = tokio::fs::metadata(&temp_path)
        .await
        .map_err(|e| LlmError::Other(format!("Failed to read file metadata: {}", e)))?;

    // Basic sanity check - file should be reasonably close to expected size
    if metadata.len() < total_size / 2 {
        tokio::fs::remove_file(&temp_path).await.ok();
        return Err(LlmError::Other(format!(
            "Downloaded file too small: {} bytes (expected ~{})",
            metadata.len(),
            total_size
        )));
    }

    // Move temp file to final location
    tokio::fs::rename(&temp_path, &dest_path)
        .await
        .map_err(|e| LlmError::Other(format!("Failed to rename temp file: {}", e)))?;

    // Report completion
    on_progress(DownloadProgress {
        model_id: model_id.to_string(),
        downloaded_bytes: downloaded,
        total_bytes: total_size,
        percent: 100.0,
        status: DownloadStatus::Complete,
    });

    Ok(dest_path)
}

/// Download a custom model from a URL
/// Returns the path to the downloaded model
pub async fn download_custom_model<F>(
    models_dir: &PathBuf,
    name: &str,
    url: &str,
    on_progress: F,
) -> Result<PathBuf, LlmError>
where
    F: Fn(DownloadProgress) + Send + 'static,
{
    // Validate URL
    if !url.to_lowercase().contains(".gguf") {
        return Err(LlmError::Other(
            "URL must point to a .gguf file".to_string(),
        ));
    }

    // Sanitize the model name for use as filename
    let safe_name: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();

    let model_id = safe_name.to_lowercase();
    let dest_path = models_dir.join(format!("{}.gguf", model_id));

    // Check if already exists
    if dest_path.exists() {
        return Err(LlmError::Other(format!(
            "Model '{}' already exists. Delete it first to re-download.",
            model_id
        )));
    }

    // Report starting
    on_progress(DownloadProgress {
        model_id: model_id.clone(),
        downloaded_bytes: 0,
        total_bytes: 0,
        percent: 0.0,
        status: DownloadStatus::Downloading,
    });

    // Create HTTP client with timeout
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(7200)) // 2 hour timeout for large models
        .build()
        .map_err(|e| LlmError::Other(format!("Failed to create HTTP client: {}", e)))?;

    // Start download
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| LlmError::Other(format!("Failed to start download: {}", e)))?;

    if !response.status().is_success() {
        return Err(LlmError::Other(format!(
            "Download failed with status: {}",
            response.status()
        )));
    }

    // Get content length (may not always be available)
    let total_size = response.content_length().unwrap_or(0);

    // Create temp file for download
    let temp_path = dest_path.with_extension("gguf.tmp");
    let mut file = tokio::fs::File::create(&temp_path)
        .await
        .map_err(|e| LlmError::Other(format!("Failed to create temp file: {}", e)))?;

    // Stream download with progress
    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result
            .map_err(|e| LlmError::Other(format!("Download error: {}", e)))?;

        file.write_all(&chunk)
            .await
            .map_err(|e| LlmError::Other(format!("Failed to write chunk: {}", e)))?;

        downloaded += chunk.len() as u64;
        let percent = if total_size > 0 {
            (downloaded as f32 / total_size as f32) * 100.0
        } else {
            0.0 // Unknown total size
        };

        on_progress(DownloadProgress {
            model_id: model_id.clone(),
            downloaded_bytes: downloaded,
            total_bytes: total_size,
            percent,
            status: DownloadStatus::Downloading,
        });
    }

    // Flush and close file
    file.flush()
        .await
        .map_err(|e| LlmError::Other(format!("Failed to flush file: {}", e)))?;
    drop(file);

    // Verify download (basic size check)
    on_progress(DownloadProgress {
        model_id: model_id.clone(),
        downloaded_bytes: downloaded,
        total_bytes: downloaded,
        percent: 100.0,
        status: DownloadStatus::Verifying,
    });

    let metadata = tokio::fs::metadata(&temp_path)
        .await
        .map_err(|e| LlmError::Other(format!("Failed to read file metadata: {}", e)))?;

    // Basic sanity check - file should be at least 10MB for a GGUF model
    if metadata.len() < 10_000_000 {
        tokio::fs::remove_file(&temp_path).await.ok();
        return Err(LlmError::Other(format!(
            "Downloaded file too small ({} bytes). This doesn't appear to be a valid GGUF model.",
            metadata.len()
        )));
    }

    // Move temp file to final location
    tokio::fs::rename(&temp_path, &dest_path)
        .await
        .map_err(|e| LlmError::Other(format!("Failed to rename temp file: {}", e)))?;

    // Report completion
    on_progress(DownloadProgress {
        model_id: model_id.clone(),
        downloaded_bytes: downloaded,
        total_bytes: downloaded,
        percent: 100.0,
        status: DownloadStatus::Complete,
    });

    log::info!("Downloaded custom model '{}' to {:?}", model_id, dest_path);
    Ok(dest_path)
}

/// Cancel an in-progress download
pub fn cancel_download(models_dir: &PathBuf, model_id: &str) -> Result<(), LlmError> {
    // Remove any temp file
    let temp_path = models_dir.join(format!("{}.gguf.tmp", model_id));
    if temp_path.exists() {
        std::fs::remove_file(&temp_path)
            .map_err(|e| LlmError::Other(format!("Failed to remove temp file: {}", e)))?;
    }
    Ok(())
}
