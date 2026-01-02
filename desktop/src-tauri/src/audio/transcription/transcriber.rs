// audio/transcription/transcriber.rs
//
// Provider-agnostic chunk transcription logic.

use super::engine::TranscriptionEngine;
use super::provider::TranscriptionError;
use crate::audio::AudioChunk;
use log::{error, info, warn};
use tauri::{AppHandle, Emitter, Runtime};

/// Transcribe audio chunk using the appropriate provider (Whisper, Parakeet, or trait-based)
/// Returns: (text, confidence Option, is_partial)
pub async fn transcribe_chunk_with_provider<R: Runtime>(
    engine: &TranscriptionEngine,
    chunk: AudioChunk,
    app: &AppHandle<R>,
) -> std::result::Result<(String, Option<f32>, bool), TranscriptionError> {
    // Convert to 16kHz mono for transcription
    let transcription_data = if chunk.sample_rate != 16000 {
        crate::audio::audio_processing::resample_audio(&chunk.data, chunk.sample_rate, 16000)
    } else {
        chunk.data
    };

    // Skip VAD processing here since the pipeline already extracted speech using VAD
    let speech_samples = transcription_data;

    // Check for empty samples - improved error handling
    if speech_samples.is_empty() {
        warn!(
            "Audio chunk {} is empty, skipping transcription",
            chunk.chunk_id
        );
        return Err(TranscriptionError::AudioTooShort {
            samples: 0,
            minimum: 1600, // 100ms at 16kHz
        });
    }

    // Calculate energy for logging/monitoring only
    let energy: f32 =
        speech_samples.iter().map(|&x| x * x).sum::<f32>() / speech_samples.len() as f32;
    info!(
        "Processing speech audio chunk {} with {} samples (energy: {:.6})",
        chunk.chunk_id,
        speech_samples.len(),
        energy
    );

    // Transcribe using the appropriate engine (with improved error handling)
    match engine {
        TranscriptionEngine::Whisper(whisper_engine) => {
            transcribe_with_whisper(whisper_engine, speech_samples, chunk.chunk_id, app).await
        }
        TranscriptionEngine::Parakeet(parakeet_engine) => {
            transcribe_with_parakeet(parakeet_engine, speech_samples, chunk.chunk_id, app).await
        }
        TranscriptionEngine::Provider(provider) => {
            transcribe_with_provider(provider, speech_samples, chunk.chunk_id, app).await
        }
    }
}

async fn transcribe_with_whisper<R: Runtime>(
    whisper_engine: &std::sync::Arc<crate::whisper_engine::WhisperEngine>,
    speech_samples: Vec<f32>,
    chunk_id: u64,
    app: &AppHandle<R>,
) -> std::result::Result<(String, Option<f32>, bool), TranscriptionError> {
    // Get language preference from global state
    let language = crate::get_language_preference_internal();

    match whisper_engine
        .transcribe_audio_with_confidence(speech_samples, language)
        .await
    {
        Ok((text, confidence, is_partial)) => {
            let cleaned_text = text.trim().to_string();
            if cleaned_text.is_empty() {
                return Ok((String::new(), Some(confidence), is_partial));
            }

            info!(
                "Whisper transcription complete for chunk {}: '{}' (confidence: {:.2}, partial: {})",
                chunk_id, cleaned_text, confidence, is_partial
            );

            Ok((cleaned_text, Some(confidence), is_partial))
        }
        Err(e) => {
            error!(
                "Whisper transcription failed for chunk {}: {}",
                chunk_id, e
            );

            let transcription_error = TranscriptionError::EngineFailed(e.to_string());
            let _ = app.emit(
                "transcription-error",
                &serde_json::json!({
                    "error": transcription_error.to_string(),
                    "userMessage": format!("Transcription failed: {}", transcription_error),
                    "actionable": false
                }),
            );

            Err(transcription_error)
        }
    }
}

async fn transcribe_with_parakeet<R: Runtime>(
    parakeet_engine: &std::sync::Arc<crate::parakeet_engine::ParakeetEngine>,
    speech_samples: Vec<f32>,
    chunk_id: u64,
    app: &AppHandle<R>,
) -> std::result::Result<(String, Option<f32>, bool), TranscriptionError> {
    match parakeet_engine.transcribe_audio(speech_samples).await {
        Ok(text) => {
            let cleaned_text = text.trim().to_string();
            if cleaned_text.is_empty() {
                return Ok((String::new(), None, false));
            }

            info!(
                "Parakeet transcription complete for chunk {}: '{}'",
                chunk_id, cleaned_text
            );

            // Parakeet doesn't provide confidence or partial results
            Ok((cleaned_text, None, false))
        }
        Err(e) => {
            error!(
                "Parakeet transcription failed for chunk {}: {}",
                chunk_id, e
            );

            let transcription_error = TranscriptionError::EngineFailed(e.to_string());
            let _ = app.emit(
                "transcription-error",
                &serde_json::json!({
                    "error": transcription_error.to_string(),
                    "userMessage": format!("Transcription failed: {}", transcription_error),
                    "actionable": false
                }),
            );

            Err(transcription_error)
        }
    }
}

async fn transcribe_with_provider<R: Runtime>(
    provider: &std::sync::Arc<dyn super::provider::TranscriptionProvider>,
    speech_samples: Vec<f32>,
    chunk_id: u64,
    app: &AppHandle<R>,
) -> std::result::Result<(String, Option<f32>, bool), TranscriptionError> {
    // Trait-based provider (clean, unified interface)
    let language = crate::get_language_preference_internal();

    match provider.transcribe(speech_samples, language).await {
        Ok(result) => {
            let cleaned_text = result.text.trim().to_string();
            if cleaned_text.is_empty() {
                return Ok((String::new(), result.confidence, result.is_partial));
            }

            let confidence_str = match result.confidence {
                Some(c) => format!("confidence: {:.2}", c),
                None => "no confidence".to_string(),
            };

            info!(
                "{} transcription complete for chunk {}: '{}' ({}, partial: {})",
                provider.provider_name(),
                chunk_id,
                cleaned_text,
                confidence_str,
                result.is_partial
            );

            Ok((cleaned_text, result.confidence, result.is_partial))
        }
        Err(e) => {
            error!(
                "{} transcription failed for chunk {}: {}",
                provider.provider_name(),
                chunk_id,
                e
            );

            let _ = app.emit(
                "transcription-error",
                &serde_json::json!({
                    "error": e.to_string(),
                    "userMessage": format!("Transcription failed: {}", e),
                    "actionable": false
                }),
            );

            Err(e)
        }
    }
}
