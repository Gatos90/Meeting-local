// Whisper Engine - Model Loading and GPU Detection
use std::sync::Arc;
use tokio::sync::RwLock;
use whisper_rs::{WhisperContext, WhisperContextParameters};
use anyhow::{Result, anyhow};

use super::types::{ModelStatus, ModelInfo};
use std::collections::HashMap;

/// Detect available GPU acceleration capabilities
pub fn detect_gpu_acceleration() -> bool {
    // On macOS, prefer Metal GPU acceleration
    if cfg!(target_os = "macos") {
        log::info!("macOS detected - attempting to enable Metal GPU acceleration");
        return true;
    }

    // Check for CUDA support on other platforms
    if cfg!(feature = "cuda") {
        log::info!("CUDA feature enabled - attempting GPU acceleration");
        return true;
    }

    // Check for Vulkan support on other platforms
    if cfg!(feature = "vulkan") {
        log::info!("Vulkan feature enabled - attempting GPU acceleration");
        return true;
    }

    // Fall back to CPU
    log::info!("No GPU acceleration features detected - using CPU processing");
    false
}

/// Log hardware acceleration capabilities
pub fn log_acceleration_capabilities() {
    let gpu_support = detect_gpu_acceleration();
    log::info!("Hardware acceleration support: {}", if gpu_support { "enabled" } else { "disabled" });

    #[cfg(feature = "metal")]
    log::info!("Apple Metal GPU support: enabled");

    #[cfg(feature = "openblas")]
    log::info!("OpenBLAS CPU optimization: enabled");

    #[cfg(feature = "coreml")]
    log::info!("Apple CoreML support: enabled");

    #[cfg(feature = "cuda")]
    log::info!("NVIDIA CUDA support: enabled");

    #[cfg(feature = "vulkan")]
    log::info!("Vulkan GPU support: enabled");

    #[cfg(feature = "openmp")]
    log::info!("OpenMP parallel processing: enabled");
}

/// Load a whisper model
pub async fn load_model(
    model_name: &str,
    available_models: &RwLock<HashMap<String, ModelInfo>>,
    current_context: &RwLock<Option<WhisperContext>>,
    current_model: &RwLock<Option<String>>,
) -> Result<()> {
    let models = available_models.read().await;
    let model_info = models.get(model_name)
        .ok_or_else(|| anyhow!("Model {} not found", model_name))?;

    match model_info.status {
        ModelStatus::Available => {
            // Check if already loaded
            let should_unload = {
                let current_model_guard = current_model.read().await;
                if let Some(current) = current_model_guard.as_ref() {
                    if current == model_name {
                        log::info!("Model {} is already loaded, skipping reload", model_name);
                        return Ok(());
                    }
                    Some(current.clone())
                } else {
                    None
                }
            };

            // Unload current model if needed
            if let Some(old_model) = should_unload {
                log::info!("Unloading current model '{}' before loading '{}'", old_model, model_name);
                unload_model(current_context, current_model).await;
            }

            log::info!("Loading model: {}", model_name);

            // Get adaptive configuration based on hardware
            let hardware_profile = crate::audio::HardwareProfile::detect();
            let adaptive_config = hardware_profile.get_whisper_config();

            // Enable flash attention for high-end GPUs
            let flash_attn_enabled = match (&hardware_profile.gpu_type, &hardware_profile.performance_tier) {
                (crate::audio::GpuType::Metal, crate::audio::PerformanceTier::Ultra | crate::audio::PerformanceTier::High) => true,
                (crate::audio::GpuType::Cuda, crate::audio::PerformanceTier::Ultra | crate::audio::PerformanceTier::High) => true,
                _ => false,
            };

            let context_param = WhisperContextParameters {
                use_gpu: adaptive_config.use_gpu,
                gpu_device: 0,
                flash_attn: flash_attn_enabled,
                ..Default::default()
            };

            let ctx = WhisperContext::new_with_params(&model_info.path.to_string_lossy(), context_param)
                .map_err(|e| anyhow!("Failed to load model {}: {}", model_name, e))?;

            // Update current context and model
            *current_context.write().await = Some(ctx);
            *current_model.write().await = Some(model_name.to_string());

            // Log acceleration status
            let acceleration_status = match (&hardware_profile.gpu_type, flash_attn_enabled) {
                (crate::audio::GpuType::Metal, true) => "Metal GPU with Flash Attention (Ultra-Fast)",
                (crate::audio::GpuType::Metal, false) => "Metal GPU acceleration",
                (crate::audio::GpuType::Cuda, true) => "CUDA GPU with Flash Attention (Ultra-Fast)",
                (crate::audio::GpuType::Cuda, false) => "CUDA GPU acceleration",
                (crate::audio::GpuType::Vulkan, _) => "Vulkan GPU acceleration",
                (crate::audio::GpuType::OpenCL, _) => "OpenCL GPU acceleration",
                (crate::audio::GpuType::None, _) => "CPU processing only",
            };

            log::info!("Successfully loaded model: {} with {} (Performance Tier: {:?}, Beam Size: {}, Threads: {:?})",
                      model_name, acceleration_status, hardware_profile.performance_tier,
                      adaptive_config.beam_size, adaptive_config.max_threads);
            Ok(())
        },
        ModelStatus::Missing => Err(anyhow!("Model {} is not downloaded", model_name)),
        ModelStatus::Downloading { .. } => Err(anyhow!("Model {} is currently downloading", model_name)),
        ModelStatus::Error(ref err) => Err(anyhow!("Model {} has error: {}", model_name, err)),
        ModelStatus::Corrupted { .. } => Err(anyhow!("Model {} is corrupted and cannot be loaded", model_name)),
    }
}

/// Unload the current model
pub async fn unload_model(
    current_context: &RwLock<Option<WhisperContext>>,
    current_model: &RwLock<Option<String>>,
) -> bool {
    let mut ctx_guard = current_context.write().await;
    let unloaded = ctx_guard.take().is_some();
    if unloaded {
        log::info!("Whisper model unloaded");
    }

    let mut model_name_guard = current_model.write().await;
    model_name_guard.take();

    unloaded
}
