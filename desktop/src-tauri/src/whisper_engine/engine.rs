// Whisper Engine - Core Engine
use std::path::PathBuf;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use whisper_rs::{WhisperContext, FullParams, SamplingStrategy};
use anyhow::{Result, anyhow};
use crate::{perf_debug, perf_trace};

use super::types::{ModelStatus, ModelInfo};
use super::text_cleaner::clean_repetitive_text;
use super::model_registry::discover_models;
use super::model_loader::{load_model, unload_model, log_acceleration_capabilities};
use super::downloader::{download_model, cancel_download, delete_model};

pub struct WhisperEngine {
    models_dir: PathBuf,
    current_context: Arc<RwLock<Option<WhisperContext>>>,
    current_model: Arc<RwLock<Option<String>>>,
    available_models: Arc<RwLock<HashMap<String, ModelInfo>>>,
    // State tracking for smart logging
    last_transcription_was_short: Arc<RwLock<bool>>,
    short_audio_warning_logged: Arc<RwLock<bool>>,
    // Performance optimization: reduce logging frequency
    transcription_count: Arc<RwLock<u64>>,
    // Download cancellation tracking
    cancel_download_flag: Arc<RwLock<Option<String>>>,
    // Active downloads tracking
    active_downloads: Arc<RwLock<HashSet<String>>>,
}

impl WhisperEngine {
    pub fn new() -> Result<Self> {
        Self::new_with_models_dir(None)
    }

    pub fn new_with_models_dir(models_dir: Option<PathBuf>) -> Result<Self> {
        // Suppress verbose whisper.cpp logs
        std::env::set_var("GGML_METAL_LOG_LEVEL", "1");
        std::env::set_var("WHISPER_LOG_LEVEL", "1");

        let models_dir = if let Some(dir) = models_dir {
            dir
        } else {
            let current_dir = std::env::current_dir()
                .map_err(|e| anyhow!("Failed to get current directory: {}", e))?;

            if cfg!(debug_assertions) {
                if current_dir.join("models").exists() {
                    current_dir.join("models")
                } else if current_dir.join("../models").exists() {
                    current_dir.join("../models")
                } else if current_dir.join("backend/whisper-server-package/models").exists() {
                    current_dir.join("backend/whisper-server-package/models")
                } else if current_dir.join("../backend/whisper-server-package/models").exists() {
                    current_dir.join("../backend/whisper-server-package/models")
                } else {
                    current_dir.join("models")
                }
            } else {
                log::warn!("WhisperEngine: No models directory provided, using fallback path");
                dirs::data_dir()
                    .or_else(|| dirs::home_dir())
                    .ok_or_else(|| anyhow!("Could not find system data directory"))?
                    .join("MeetLocal")
                    .join("models")
            }
        };

        log::info!("WhisperEngine using models directory: {}", models_dir.display());
        log::info!("Debug mode: {}", cfg!(debug_assertions));

        log_acceleration_capabilities();

        Ok(Self {
            models_dir,
            current_context: Arc::new(RwLock::new(None)),
            current_model: Arc::new(RwLock::new(None)),
            available_models: Arc::new(RwLock::new(HashMap::new())),
            last_transcription_was_short: Arc::new(RwLock::new(false)),
            short_audio_warning_logged: Arc::new(RwLock::new(false)),
            transcription_count: Arc::new(RwLock::new(0)),
            cancel_download_flag: Arc::new(RwLock::new(None)),
            active_downloads: Arc::new(RwLock::new(HashSet::new())),
        })
    }

    pub async fn discover_models(&self) -> Result<Vec<ModelInfo>> {
        discover_models(&self.models_dir, &self.available_models).await
    }

    pub async fn load_model(&self, model_name: &str) -> Result<()> {
        load_model(model_name, &self.available_models, &self.current_context, &self.current_model).await
    }

    pub async fn unload_model(&self) -> bool {
        unload_model(&self.current_context, &self.current_model).await
    }

    pub async fn get_current_model(&self) -> Option<String> {
        self.current_model.read().await.clone()
    }

    pub async fn is_model_loaded(&self) -> bool {
        self.current_context.read().await.is_some()
    }

    pub async fn get_models_directory(&self) -> PathBuf {
        self.models_dir.clone()
    }

    pub async fn download_model(&self, model_name: &str, progress_callback: Option<Box<dyn Fn(u8) + Send>>) -> Result<()> {
        download_model(
            model_name,
            &self.models_dir,
            &self.available_models,
            &self.active_downloads,
            &self.cancel_download_flag,
            progress_callback,
        ).await
    }

    pub async fn cancel_download(&self, model_name: &str) -> Result<()> {
        cancel_download(
            model_name,
            &self.models_dir,
            &self.available_models,
            &self.active_downloads,
            &self.cancel_download_flag,
        ).await
    }

    pub async fn delete_model(&self, model_name: &str) -> Result<String> {
        delete_model(model_name, &self.available_models).await
    }

    /// Transcribe audio with confidence and partial detection
    pub async fn transcribe_audio_with_confidence(&self, audio_data: Vec<f32>, language: Option<String>) -> Result<(String, f32, bool)> {
        let ctx_lock = self.current_context.read().await;
        let ctx = ctx_lock.as_ref()
            .ok_or_else(|| anyhow!("No model loaded. Please load a model first."))?;

        let hardware_profile = crate::audio::HardwareProfile::detect();
        let adaptive_config = hardware_profile.get_whisper_config();

        let mut params = FullParams::new(SamplingStrategy::BeamSearch {
            beam_size: adaptive_config.beam_size as i32,
            patience: 1.0
        });

        let (language_code, should_translate) = match language.as_deref() {
            Some("auto") | None => (None, false),
            Some("auto-translate") => (None, true),
            Some(lang) => (Some(lang), false),
        };
        params.set_language(language_code);
        params.set_translate(should_translate);
        params.set_no_timestamps(true);
        params.set_token_timestamps(true);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_suppress_blank(true);
        params.set_suppress_non_speech_tokens(true);
        params.set_temperature(adaptive_config.temperature);
        params.set_max_initial_ts(1.0);
        params.set_entropy_thold(2.4);
        params.set_logprob_thold(-1.0);
        params.set_no_speech_thold(0.55);
        params.set_max_len(200);
        params.set_single_segment(false);
        params.set_no_context(true);

        let duration_seconds = audio_data.len() as f64 / 16000.0;
        let is_partial = duration_seconds < 15.0;

        let mut state = ctx.create_state()?;
        state.full(params, &audio_data)?;
        let num_segments = state.full_n_segments()?;

        let mut result = String::new();
        let mut total_confidence = 0.0;
        let mut segment_count = 0;

        for i in 0..num_segments {
            let segment_text = match state.full_get_segment_text_lossy(i) {
                Ok(text) => text,
                Err(_) => continue,
            };

            let segment_length = segment_text.len() as f32;
            let segment_confidence = if segment_length > 0.0 {
                (segment_length / 100.0).min(0.9) + 0.1
            } else {
                0.1
            };
            total_confidence += segment_confidence;
            segment_count += 1;

            let cleaned_text = segment_text.trim();
            if !cleaned_text.is_empty() {
                if !result.is_empty() {
                    result.push(' ');
                }
                result.push_str(cleaned_text);
            }
        }

        let final_result = result.trim().to_string();
        let cleaned_result = clean_repetitive_text(&final_result);

        let avg_confidence = if segment_count > 0 {
            total_confidence / segment_count as f32
        } else {
            0.0
        };

        Ok((cleaned_result, avg_confidence, is_partial))
    }

    pub async fn transcribe_audio(&self, audio_data: Vec<f32>, language: Option<String>) -> Result<String> {
        let ctx_lock = self.current_context.read().await;
        let ctx = ctx_lock.as_ref()
            .ok_or_else(|| anyhow!("No model loaded. Please load a model first."))?;

        let hardware_profile = crate::audio::HardwareProfile::detect();
        let adaptive_config = hardware_profile.get_whisper_config();

        let mut params = FullParams::new(SamplingStrategy::BeamSearch {
            beam_size: adaptive_config.beam_size as i32,
            patience: 1.0
        });

        let (language_code, should_translate) = match language.as_deref() {
            Some("auto") | None => (None, false),
            Some("auto-translate") => (None, true),
            Some(lang) => (Some(lang), false),
        };
        params.set_language(language_code);
        params.set_translate(should_translate);
        params.set_no_timestamps(true);
        params.set_token_timestamps(true);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_suppress_blank(true);
        params.set_suppress_non_speech_tokens(true);
        params.set_temperature(0.3);
        params.set_max_initial_ts(1.0);
        params.set_entropy_thold(2.4);
        params.set_logprob_thold(-1.0);
        params.set_no_speech_thold(0.55);
        params.set_max_len(200);
        params.set_single_segment(false);
        params.set_no_context(true);

        let duration_seconds = audio_data.len() as f64 / 16000.0;
        let is_short_audio = duration_seconds < 1.0;

        // Smart logging based on audio duration
        let mut should_log_transcription = true;
        let mut should_log_short_warning = false;

        if is_short_audio {
            let last_was_short = *self.last_transcription_was_short.read().await;
            let warning_logged = *self.short_audio_warning_logged.read().await;

            if !warning_logged {
                should_log_short_warning = true;
                *self.short_audio_warning_logged.write().await = true;
            }

            should_log_transcription = !last_was_short;
            *self.last_transcription_was_short.write().await = true;
        } else {
            let last_was_short = *self.last_transcription_was_short.read().await;

            if last_was_short {
                log::info!("Audio duration normalized, resuming transcription");
                *self.short_audio_warning_logged.write().await = false;
            }

            *self.last_transcription_was_short.write().await = false;
        }

        if should_log_short_warning {
            log::warn!("Audio duration is short ({:.1}s < 1.0s). Consider padding the input audio with silence.", duration_seconds);
        }

        let transcription_count = {
            let mut count = self.transcription_count.write().await;
            *count += 1;
            *count
        };

        if should_log_transcription && (transcription_count % 10 == 0 || duration_seconds > 10.0) {
            log::info!("Starting transcription #{} of {} samples ({:.1}s duration)",
                      transcription_count, audio_data.len(), duration_seconds);
        }

        let mut state = ctx.create_state()?;
        state.full(params, &audio_data)?;

        let num_segments = state.full_n_segments()?;

        if (should_log_transcription || num_segments > 0) && (num_segments > 3 || duration_seconds > 5.0) {
            perf_debug!("Transcription #{} completed with {} segments ({:.1}s)", transcription_count, num_segments, duration_seconds);
        }

        let mut result = String::new();

        for i in 0..num_segments {
            let segment_text = match state.full_get_segment_text_lossy(i) {
                Ok(text) => text,
                Err(_) => continue,
            };

            let _start_time = state.full_get_segment_t0(i).unwrap_or(0);
            let _end_time = state.full_get_segment_t1(i).unwrap_or(0);

            if duration_seconds > 30.0 {
                perf_trace!("Segment {} ({:.2}s-{:.2}s): '{}'",
                           i, _start_time as f64 / 100.0, _end_time as f64 / 100.0, segment_text);
            }

            let cleaned_text = segment_text.trim();
            if !cleaned_text.is_empty() {
                if !result.is_empty() {
                    result.push(' ');
                }
                result.push_str(cleaned_text);
            }
        }

        let final_result = result.trim().to_string();
        let cleaned_result = clean_repetitive_text(&final_result);

        // Smart logging for results
        if cleaned_result.is_empty() {
            if should_log_transcription && transcription_count % 20 == 0 {
                perf_debug!("Transcription #{} result is empty - no speech detected", transcription_count);
            }
        } else {
            if cleaned_result != final_result {
                log::info!("Cleaned repetitive transcription #{}: '{}' -> '{}'", transcription_count, final_result, cleaned_result);
            }
            if transcription_count % 5 == 0 || cleaned_result.len() > 50 || duration_seconds > 10.0 {
                log::info!("Transcription #{} result: '{}'", transcription_count, cleaned_result);
            } else {
                perf_debug!("Transcription #{} result: '{}'", transcription_count, cleaned_result);
            }
        }

        Ok(cleaned_result)
    }
}
