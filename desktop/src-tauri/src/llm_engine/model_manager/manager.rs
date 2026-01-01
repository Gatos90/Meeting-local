//! LLM Model Manager - Core struct and local model operations

use std::path::PathBuf;

use crate::llm_engine::provider::LlmError;
use super::types::{DownloadProgress, DownloadableModel, LocalModelInfo};
use super::registry::{available_models, get_hf_repo_for_model};
use super::tool_support::has_native_tool_support;
use super::downloader::{download_model, download_custom_model, cancel_download};

/// Manages GGUF model downloads and local storage
pub struct LlmModelManager {
    /// Directory where models are stored
    models_dir: PathBuf,
}

impl LlmModelManager {
    /// Create a new model manager
    pub fn new(app_data_dir: PathBuf) -> Self {
        let models_dir = app_data_dir.join("llm_models");

        // Ensure directory exists
        if !models_dir.exists() {
            std::fs::create_dir_all(&models_dir).ok();
        }

        Self { models_dir }
    }

    /// Get the models directory path
    pub fn models_dir(&self) -> &PathBuf {
        &self.models_dir
    }

    /// Get list of available models for download
    pub fn available_models() -> Vec<DownloadableModel> {
        available_models()
    }

    /// Get the HuggingFace repo ID for a model
    pub fn get_hf_repo_for_model(model_id: &str) -> Option<String> {
        get_hf_repo_for_model(model_id)
    }

    /// Get list of locally downloaded models
    pub fn local_models(&self) -> Result<Vec<String>, LlmError> {
        let mut models = Vec::new();

        if !self.models_dir.exists() {
            return Ok(models);
        }

        let entries = std::fs::read_dir(&self.models_dir)
            .map_err(|e| LlmError::Other(format!("Failed to read models directory: {}", e)))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "gguf").unwrap_or(false) {
                if let Some(stem) = path.file_stem() {
                    models.push(stem.to_string_lossy().to_string());
                }
            }
        }

        Ok(models)
    }

    /// Get path to a downloaded model
    pub fn model_path(&self, model_id: &str) -> PathBuf {
        self.models_dir.join(format!("{}.gguf", model_id))
    }

    /// Check if a model is downloaded
    pub fn is_downloaded(&self, model_id: &str) -> bool {
        self.model_path(model_id).exists()
    }

    /// Delete a downloaded model
    pub fn delete_model(&self, model_id: &str) -> Result<(), LlmError> {
        let path = self.model_path(model_id);
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| LlmError::Other(format!("Failed to delete model: {}", e)))?;
        }
        Ok(())
    }

    /// Download a model with progress callback
    /// Returns the path to the downloaded model
    pub async fn download_model<F>(
        &self,
        model_id: &str,
        on_progress: F,
    ) -> Result<PathBuf, LlmError>
    where
        F: Fn(DownloadProgress) + Send + 'static,
    {
        download_model(&self.models_dir, model_id, on_progress).await
    }

    /// Cancel an in-progress download
    pub fn cancel_download(&self, model_id: &str) -> Result<(), LlmError> {
        cancel_download(&self.models_dir, model_id)
    }

    /// Download a custom model from a URL
    /// Returns the path to the downloaded model
    pub async fn download_custom_model<F>(
        &self,
        name: &str,
        url: &str,
        on_progress: F,
    ) -> Result<PathBuf, LlmError>
    where
        F: Fn(DownloadProgress) + Send + 'static,
    {
        download_custom_model(&self.models_dir, name, url, on_progress).await
    }

    /// Get detailed info about all local models
    pub fn local_models_info(&self) -> Result<Vec<LocalModelInfo>, LlmError> {
        let mut models = Vec::new();

        if !self.models_dir.exists() {
            return Ok(models);
        }

        let entries = std::fs::read_dir(&self.models_dir)
            .map_err(|e| LlmError::Other(format!("Failed to read models directory: {}", e)))?;

        let available = Self::available_models();

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "gguf").unwrap_or(false) {
                if let Some(stem) = path.file_stem() {
                    let id = stem.to_string_lossy().to_string();
                    let metadata = std::fs::metadata(&path).ok();
                    let size_bytes = metadata.map(|m| m.len()).unwrap_or(0);

                    // Check if this is a curated model
                    let curated = available.iter().find(|m| m.id == id);

                    models.push(LocalModelInfo {
                        id: id.clone(),
                        name: curated.map(|m| m.name.clone()).unwrap_or(id.clone()),
                        size_bytes,
                        is_curated: curated.is_some(),
                        description: curated.map(|m| m.description.clone()),
                        context_length: curated.map(|m| m.context_length),
                        has_native_tool_support: has_native_tool_support(&id),
                    });
                }
            }
        }

        Ok(models)
    }
}
