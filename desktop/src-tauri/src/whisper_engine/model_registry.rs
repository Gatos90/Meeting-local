// Whisper Engine - Model Registry and Discovery
use std::path::PathBuf;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tokio::fs;
use tokio::io::AsyncReadExt;
use anyhow::{Result, anyhow};

use super::types::{ModelStatus, ModelInfo};

/// Model configuration: (name, filename, size_mb, accuracy, speed, description)
pub const MODEL_CONFIGS: &[(&str, &str, u32, &str, &str, &str)] = &[
    // Standard f16 models (full precision, multilingual)
    ("tiny", "ggml-tiny.bin", 78, "Decent", "Very Fast", "Fastest processing, good for real-time use"),
    ("base", "ggml-base.bin", 148, "Good", "Fast", "Good balance of speed and accuracy"),
    ("small", "ggml-small.bin", 488, "Good", "Medium", "Better accuracy, moderate speed"),
    ("medium", "ggml-medium.bin", 1530, "High", "Slow", "High accuracy for professional use"),
    ("large-v3-turbo", "ggml-large-v3-turbo.bin", 1620, "High", "Medium", "Fast large model with great accuracy"),
    ("large-v3", "ggml-large-v3.bin", 3100, "Highest", "Slow", "Best accuracy, latest large model"),

    // English-only models (slightly better for English content)
    ("tiny.en", "ggml-tiny.en.bin", 78, "Decent", "Very Fast", "English-only tiny model"),
    ("base.en", "ggml-base.en.bin", 148, "Good", "Fast", "English-only base model"),
    ("small.en", "ggml-small.en.bin", 488, "Good", "Medium", "English-only small model"),
    ("medium.en", "ggml-medium.en.bin", 1530, "High", "Slow", "English-only medium model"),

    // Q5_1 quantized models (smaller file size, good quality)
    ("tiny-q5_1", "ggml-tiny-q5_1.bin", 32, "Decent", "Very Fast", "Quantized tiny, 60% smaller"),
    ("base-q5_1", "ggml-base-q5_1.bin", 60, "Good", "Fast", "Quantized base, 60% smaller"),
    ("small-q5_1", "ggml-small-q5_1.bin", 190, "Good", "Fast", "Quantized small, 60% smaller"),

    // Q5_0 quantized models (medium/large only)
    ("medium-q5_0", "ggml-medium-q5_0.bin", 539, "High", "Medium", "Quantized medium, 65% smaller"),
    ("large-v3-turbo-q5_0", "ggml-large-v3-turbo-q5_0.bin", 574, "High", "Medium", "Quantized turbo, best balance"),
    ("large-v3-q5_0", "ggml-large-v3-q5_0.bin", 1080, "High", "Medium", "Quantized large-v3, 65% smaller"),

    // Q8_0 quantized models (higher quality quantization)
    ("tiny-q8_0", "ggml-tiny-q8_0.bin", 44, "Decent", "Very Fast", "High-quality quantized tiny"),
    ("base-q8_0", "ggml-base-q8_0.bin", 82, "Good", "Fast", "High-quality quantized base"),
    ("small-q8_0", "ggml-small-q8_0.bin", 264, "Good", "Fast", "High-quality quantized small"),
    ("medium-q8_0", "ggml-medium-q8_0.bin", 823, "High", "Medium", "High-quality quantized medium"),
    ("large-v3-turbo-q8_0", "ggml-large-v3-turbo-q8_0.bin", 874, "High", "Medium", "High-quality quantized turbo"),
];

/// Discover available models in the models directory
pub async fn discover_models(
    models_dir: &PathBuf,
    available_models: &RwLock<HashMap<String, ModelInfo>>,
) -> Result<Vec<ModelInfo>> {
    let mut models = Vec::new();

    for (name, filename, size_mb, accuracy, speed, description) in MODEL_CONFIGS {
        let model_path = models_dir.join(filename);
        let status = if model_path.exists() {
            // Check if file size is reasonable (at least 1MB for a valid model)
            match std::fs::metadata(&model_path) {
                Ok(metadata) => {
                    let file_size_bytes = metadata.len();
                    let file_size_mb = file_size_bytes / (1024 * 1024);
                    let expected_min_size_mb = (*size_mb as f64 * 0.9) as u64;

                    if file_size_mb >= expected_min_size_mb && file_size_mb > 1 {
                        // File size looks good, validate GGML header
                        match validate_model_file(&model_path).await {
                            Ok(_) => ModelStatus::Available,
                            Err(_) => {
                                log::warn!("Model file {} has correct size but appears corrupted", filename);
                                ModelStatus::Corrupted {
                                    file_size: file_size_bytes,
                                    expected_min_size: expected_min_size_mb * 1024 * 1024,
                                }
                            }
                        }
                    } else if file_size_mb > 0 {
                        // File exists but is smaller than expected - check if downloading
                        let models_guard = available_models.read().await;
                        if let Some(existing_model) = models_guard.get(*name) {
                            if let ModelStatus::Downloading { progress } = &existing_model.status {
                                log::debug!("Model {} appears to be downloading ({}%)", filename, progress);
                                ModelStatus::Downloading { progress: *progress }
                            } else {
                                log::warn!("Model file {} is corrupted ({} MB, expected ~{} MB)",
                                         filename, file_size_mb, size_mb);
                                ModelStatus::Corrupted {
                                    file_size: file_size_bytes,
                                    expected_min_size: expected_min_size_mb * 1024 * 1024,
                                }
                            }
                        } else {
                            log::warn!("Model file {} is corrupted ({} MB, expected ~{} MB)",
                                     filename, file_size_mb, size_mb);
                            ModelStatus::Corrupted {
                                file_size: file_size_bytes,
                                expected_min_size: expected_min_size_mb * 1024 * 1024,
                            }
                        }
                    } else {
                        ModelStatus::Missing
                    }
                }
                Err(_) => ModelStatus::Missing
            }
        } else {
            ModelStatus::Missing
        };

        let model_info = ModelInfo {
            name: name.to_string(),
            path: model_path,
            size_mb: *size_mb,
            accuracy: accuracy.to_string(),
            speed: speed.to_string(),
            status,
            description: description.to_string(),
        };

        models.push(model_info);
    }

    // Update internal cache
    let mut cache = available_models.write().await;
    cache.clear();
    for model in &models {
        cache.insert(model.name.clone(), model.clone());
    }

    Ok(models)
}

/// Validate if a model file is a valid GGML file by checking its header
pub async fn validate_model_file(model_path: &PathBuf) -> Result<()> {
    let mut file = fs::File::open(model_path).await
        .map_err(|e| anyhow!("Failed to open model file: {}", e))?;

    // Read the first 8 bytes to check for GGML magic number
    let mut buffer = [0u8; 8];
    file.read_exact(&mut buffer).await
        .map_err(|e| anyhow!("Failed to read model file header: {}", e))?;

    // Check for GGML magic number (various versions and endianness)
    if buffer.starts_with(b"ggml") || buffer.starts_with(b"GGUF") || buffer.starts_with(b"ggmf") ||
       buffer.starts_with(b"lmgg") || buffer.starts_with(b"FUGU") || buffer.starts_with(b"fmgg") {
        Ok(())
    } else {
        Err(anyhow!("Invalid model file: missing GGML/GGUF magic number. Found: {:?}",
                   String::from_utf8_lossy(&buffer[..4])))
    }
}

/// Get model URL for downloading
pub fn get_model_url(model_name: &str) -> Option<&'static str> {
    match model_name {
        // Standard f16 models (multilingual)
        "tiny" => Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin"),
        "base" => Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin"),
        "small" => Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin"),
        "medium" => Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin"),
        "large-v3-turbo" => Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin"),
        "large-v3" => Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin"),

        // English-only models
        "tiny.en" => Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin"),
        "base.en" => Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin"),
        "small.en" => Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin"),
        "medium.en" => Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.en.bin"),

        // Q5_1 quantized models
        "tiny-q5_1" => Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny-q5_1.bin"),
        "base-q5_1" => Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base-q5_1.bin"),
        "small-q5_1" => Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small-q5_1.bin"),

        // Q5_0 quantized models
        "medium-q5_0" => Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium-q5_0.bin"),
        "large-v3-turbo-q5_0" => Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin"),
        "large-v3-q5_0" => Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-q5_0.bin"),

        // Q8_0 quantized models
        "tiny-q8_0" => Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny-q8_0.bin"),
        "base-q8_0" => Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base-q8_0.bin"),
        "small-q8_0" => Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small-q8_0.bin"),
        "medium-q8_0" => Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium-q8_0.bin"),
        "large-v3-turbo-q8_0" => Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q8_0.bin"),

        _ => None
    }
}
