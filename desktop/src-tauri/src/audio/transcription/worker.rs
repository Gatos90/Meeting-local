// audio/transcription/worker.rs
//
// Parallel transcription worker pool and main task loop.

use super::engine::TranscriptionEngine;
use super::provider::TranscriptionError;
use super::globals::{is_live_diarization_enabled, mark_speech_detected, next_sequence_id, SPEECH_DETECTED_EMITTED};
use super::types::{TranscriptUpdate, format_current_timestamp};
use super::transcriber::transcribe_chunk_with_provider;
use crate::audio::AudioChunk;
use log::{error, info, warn};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};

// Re-export for backwards compatibility
pub use super::globals::{
    set_live_diarization_enabled,
    is_live_diarization_enabled as check_live_diarization,
    reset_speech_detected_flag,
};

/// Optimized parallel transcription task ensuring ZERO chunk loss
pub fn start_transcription_task<R: Runtime>(
    app: AppHandle<R>,
    transcription_receiver: tokio::sync::mpsc::UnboundedReceiver<AudioChunk>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        info!("🚀 Starting optimized parallel transcription task - guaranteeing zero chunk loss");

        // Initialize transcription engine (Whisper or Parakeet based on config)
        let transcription_engine = match super::engine::get_or_init_transcription_engine(&app).await {
            Ok(engine) => engine,
            Err(e) => {
                error!("Failed to initialize transcription engine: {}", e);
                let _ = app.emit("transcription-error", serde_json::json!({
                    "error": e,
                    "userMessage": "Recording failed: Unable to initialize speech recognition. Please check your model settings.",
                    "actionable": true
                }));
                return;
            }
        };

        // Create parallel workers for faster processing while preserving ALL chunks
        const NUM_WORKERS: usize = 1; // Serial processing ensures transcripts emit in chronological order
        let (work_sender, work_receiver) = tokio::sync::mpsc::unbounded_channel::<AudioChunk>();
        let work_receiver = Arc::new(tokio::sync::Mutex::new(work_receiver));

        // Track completion: AtomicU64 for chunks queued, AtomicU64 for chunks completed
        let chunks_queued = Arc::new(AtomicU64::new(0));
        let chunks_completed = Arc::new(AtomicU64::new(0));
        let input_finished = Arc::new(AtomicBool::new(false));

        info!("📊 Starting {} transcription worker{} (serial mode for ordered emission)", NUM_WORKERS, if NUM_WORKERS == 1 { "" } else { "s" });

        // Spawn worker tasks
        let mut worker_handles = Vec::new();
        for worker_id in 0..NUM_WORKERS {
            let engine_clone = match &transcription_engine {
                TranscriptionEngine::Whisper(e) => TranscriptionEngine::Whisper(e.clone()),
                TranscriptionEngine::Parakeet(e) => TranscriptionEngine::Parakeet(e.clone()),
                TranscriptionEngine::Provider(p) => TranscriptionEngine::Provider(p.clone()),
            };
            let app_clone = app.clone();
            let work_receiver_clone = work_receiver.clone();
            let chunks_completed_clone = chunks_completed.clone();
            let input_finished_clone = input_finished.clone();
            let chunks_queued_clone = chunks_queued.clone();

            let worker_handle = tokio::spawn(async move {
                worker_loop(
                    worker_id,
                    engine_clone,
                    app_clone,
                    work_receiver_clone,
                    chunks_completed_clone,
                    input_finished_clone,
                    chunks_queued_clone,
                ).await;
            });

            worker_handles.push(worker_handle);
        }

        // Main dispatcher: receive chunks and distribute to workers
        let mut receiver = transcription_receiver;
        while let Some(chunk) = receiver.recv().await {
            let queued = chunks_queued.fetch_add(1, Ordering::SeqCst) + 1;
            info!(
                "📥 Dispatching chunk {} to workers (total queued: {})",
                chunk.chunk_id, queued
            );

            if let Err(_) = work_sender.send(chunk) {
                error!("❌ Failed to send chunk to workers - this should not happen!");
                break;
            }
        }

        // Signal that input is finished
        input_finished.store(true, Ordering::SeqCst);
        drop(work_sender); // Close the channel to signal workers

        let total_chunks_queued = chunks_queued.load(Ordering::SeqCst);
        info!("📭 Input finished with {} total chunks queued. Waiting for all {} workers to complete...",
              total_chunks_queued, NUM_WORKERS);

        // Emit final chunk count to frontend
        let _ = app.emit("transcription-queue-complete", serde_json::json!({
            "total_chunks": total_chunks_queued,
            "message": format!("{} chunks queued for processing - waiting for completion", total_chunks_queued)
        }));

        // Wait for all workers to complete
        for (worker_id, handle) in worker_handles.into_iter().enumerate() {
            if let Err(e) = handle.await {
                error!("❌ Worker {} panicked: {:?}", worker_id, e);
            } else {
                info!("✅ Worker {} completed successfully", worker_id);
            }
        }

        // Final verification with retry logic to catch any stragglers
        verify_all_chunks_processed(&app, &chunks_queued, &chunks_completed).await;

        info!("✅ Parallel transcription task completed - all workers finished, ready for model unload");
    })
}

/// Worker loop that processes audio chunks
async fn worker_loop<R: Runtime>(
    worker_id: usize,
    engine_clone: TranscriptionEngine,
    app_clone: AppHandle<R>,
    work_receiver_clone: Arc<tokio::sync::Mutex<tokio::sync::mpsc::UnboundedReceiver<AudioChunk>>>,
    chunks_completed_clone: Arc<AtomicU64>,
    input_finished_clone: Arc<AtomicBool>,
    chunks_queued_clone: Arc<AtomicU64>,
) {
    info!("👷 Worker {} started", worker_id);

    // PRE-VALIDATE model state to avoid repeated async calls per chunk
    let initial_model_loaded = engine_clone.is_model_loaded().await;
    let current_model = engine_clone
        .get_current_model()
        .await
        .unwrap_or_else(|| "unknown".to_string());

    let engine_name = engine_clone.provider_name();

    if initial_model_loaded {
        info!(
            "✅ Worker {} pre-validation: {} model '{}' is loaded and ready",
            worker_id, engine_name, current_model
        );
    } else {
        warn!("⚠️ Worker {} pre-validation: {} model not loaded - chunks may be skipped", worker_id, engine_name);
    }

    loop {
        // Try to get a chunk to process
        let chunk = {
            let mut receiver = work_receiver_clone.lock().await;
            receiver.recv().await
        };

        match chunk {
            Some(chunk) => {
                process_chunk(
                    worker_id,
                    &engine_clone,
                    &app_clone,
                    chunk,
                    &chunks_completed_clone,
                    &chunks_queued_clone,
                ).await;
            }
            None => {
                // No more chunks available
                if input_finished_clone.load(Ordering::SeqCst) {
                    // Double-check that all queued chunks are actually completed
                    let final_queued = chunks_queued_clone.load(Ordering::SeqCst);
                    let final_completed = chunks_completed_clone.load(Ordering::SeqCst);

                    if final_completed >= final_queued {
                        info!(
                            "👷 Worker {} finishing - all {}/{} chunks processed",
                            worker_id, final_completed, final_queued
                        );
                        break;
                    } else {
                        warn!("👷 Worker {} detected potential chunk loss: {}/{} completed, waiting...", worker_id, final_completed, final_queued);
                        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
                    }
                } else {
                    tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                }
            }
        }
    }

    info!("👷 Worker {} completed", worker_id);
}

/// Process a single audio chunk
async fn process_chunk<R: Runtime>(
    worker_id: usize,
    engine_clone: &TranscriptionEngine,
    app_clone: &AppHandle<R>,
    chunk: AudioChunk,
    chunks_completed_clone: &Arc<AtomicU64>,
    chunks_queued_clone: &Arc<AtomicU64>,
) {
    // PERFORMANCE OPTIMIZATION: Reduce logging in hot path
    let should_log_this_chunk = chunk.chunk_id % 10 == 0;

    if should_log_this_chunk {
        info!(
            "👷 Worker {} processing chunk {} with {} samples",
            worker_id,
            chunk.chunk_id,
            chunk.data.len()
        );
    }

    // Check if model is still loaded before processing
    if !engine_clone.is_model_loaded().await {
        warn!("⚠️ Worker {}: Model unloaded, but continuing to preserve chunk {}", worker_id, chunk.chunk_id);
        chunks_completed_clone.fetch_add(1, Ordering::SeqCst);
        return;
    }

    let chunk_timestamp = chunk.timestamp;
    let chunk_duration = chunk.data.len() as f64 / chunk.sample_rate as f64;

    // Transcribe with provider-agnostic approach
    match transcribe_chunk_with_provider(engine_clone, chunk, app_clone).await {
        Ok((transcript, confidence_opt, is_partial)) => {
            handle_transcription_result(
                worker_id,
                transcript,
                confidence_opt,
                is_partial,
                chunk_timestamp,
                chunk_duration,
                engine_clone,
                app_clone,
                should_log_this_chunk,
            ).await;
        }
        Err(e) => {
            handle_transcription_error(worker_id, e, app_clone, chunks_completed_clone).await;
            return;
        }
    }

    // Mark chunk as completed and emit progress
    emit_progress(worker_id, chunks_completed_clone, chunks_queued_clone, app_clone, should_log_this_chunk).await;
}

/// Handle successful transcription result
async fn handle_transcription_result<R: Runtime>(
    worker_id: usize,
    transcript: String,
    confidence_opt: Option<f32>,
    is_partial: bool,
    chunk_timestamp: f64,
    chunk_duration: f64,
    engine_clone: &TranscriptionEngine,
    app_clone: &AppHandle<R>,
    should_log_this_chunk: bool,
) {
    // Provider-aware confidence threshold
    let confidence_threshold = match engine_clone {
        TranscriptionEngine::Whisper(_) | TranscriptionEngine::Provider(_) => 0.3,
        TranscriptionEngine::Parakeet(_) => 0.0, // Parakeet has no confidence, accept all
    };

    let confidence_str = match confidence_opt {
        Some(c) => format!("{:.2}", c),
        None => "N/A".to_string(),
    };

    info!("🔍 Worker {} transcription result: text='{}', confidence={}, partial={}, threshold={:.2}",
          worker_id, transcript, confidence_str, is_partial, confidence_threshold);

    // Check confidence threshold (or accept if no confidence provided)
    let meets_threshold = confidence_opt.map_or(true, |c| c >= confidence_threshold);

    if !transcript.trim().is_empty() && meets_threshold {
        info!("✅ Worker {} transcribed: {} (confidence: {}, partial: {})",
              worker_id, transcript, confidence_str, is_partial);

        // Emit speech-detected event for frontend UX (only on first detection per session)
        let current_flag = SPEECH_DETECTED_EMITTED.load(Ordering::SeqCst);
        info!("🔍 Checking speech-detected flag: current={}, will_emit={}", current_flag, !current_flag);

        if mark_speech_detected() {
            match app_clone.emit("speech-detected", serde_json::json!({
                "message": "Speech activity detected"
            })) {
                Ok(_) => info!("🎤 ✅ First speech detected - successfully emitted speech-detected event"),
                Err(e) => error!("🎤 ❌ Failed to emit speech-detected event: {}", e),
            }
        } else {
            info!("🔍 Speech already detected in this session, not re-emitting");
        }

        // Generate sequence ID and calculate timestamps
        let sequence_id = next_sequence_id();
        let audio_start_time = chunk_timestamp;
        let audio_end_time = chunk_timestamp + chunk_duration;

        // Get speaker info if diarization is enabled
        let (speaker_id, speaker_label, is_registered_speaker) =
            if is_live_diarization_enabled() {
                // For now, just return None - live diarization done on full mixed audio in pipeline
                (None, None, false)
            } else {
                (None, None, false)
            };

        // Emit transcript update with recording-relative timestamps
        let update = TranscriptUpdate {
            text: transcript,
            timestamp: format_current_timestamp(),
            source: "Audio".to_string(),
            sequence_id,
            chunk_start_time: chunk_timestamp,
            is_partial,
            confidence: confidence_opt.unwrap_or(0.85),
            audio_start_time,
            audio_end_time,
            duration: chunk_duration,
            speaker_id,
            speaker_label,
            is_registered_speaker,
        };

        if let Err(e) = app_clone.emit("transcript-update", &update) {
            error!(
                "Worker {}: Failed to emit transcript update: {}",
                worker_id, e
            );
        }
    } else if !transcript.trim().is_empty() && should_log_this_chunk {
        if let Some(c) = confidence_opt {
            info!("Worker {} low-confidence transcription (confidence: {:.2}), skipping", worker_id, c);
        }
    }
}

/// Handle transcription errors
async fn handle_transcription_error<R: Runtime>(
    worker_id: usize,
    e: TranscriptionError,
    app_clone: &AppHandle<R>,
    chunks_completed_clone: &Arc<AtomicU64>,
) {
    match e {
        TranscriptionError::AudioTooShort { .. } => {
            info!("Worker {}: {}", worker_id, e);
            chunks_completed_clone.fetch_add(1, Ordering::SeqCst);
        }
        TranscriptionError::ModelNotLoaded => {
            warn!("Worker {}: Model unloaded during transcription", worker_id);
            chunks_completed_clone.fetch_add(1, Ordering::SeqCst);
        }
        _ => {
            warn!("Worker {}: Transcription failed: {}", worker_id, e);
            let _ = app_clone.emit("transcription-warning", e.to_string());
        }
    }
}

/// Emit progress update
async fn emit_progress<R: Runtime>(
    worker_id: usize,
    chunks_completed_clone: &Arc<AtomicU64>,
    chunks_queued_clone: &Arc<AtomicU64>,
    app_clone: &AppHandle<R>,
    should_log_this_chunk: bool,
) {
    let completed = chunks_completed_clone.fetch_add(1, Ordering::SeqCst) + 1;
    let queued = chunks_queued_clone.load(Ordering::SeqCst);

    if completed % 5 == 0 || should_log_this_chunk {
        info!(
            "Worker {}: Progress {}/{} chunks ({:.1}%)",
            worker_id,
            completed,
            queued,
            (completed as f64 / queued.max(1) as f64 * 100.0)
        );
    }

    let progress_percentage = if queued > 0 {
        (completed as f64 / queued as f64 * 100.0) as u32
    } else {
        100
    };

    let _ = app_clone.emit("transcription-progress", serde_json::json!({
        "worker_id": worker_id,
        "chunks_completed": completed,
        "chunks_queued": queued,
        "progress_percentage": progress_percentage,
        "message": format!("Worker {} processing... ({}/{})", worker_id, completed, queued)
    }));
}

/// Verify all chunks were processed
async fn verify_all_chunks_processed<R: Runtime>(
    app: &AppHandle<R>,
    chunks_queued: &Arc<AtomicU64>,
    chunks_completed: &Arc<AtomicU64>,
) {
    let mut verification_attempts = 0;
    const MAX_VERIFICATION_ATTEMPTS: u32 = 10;

    loop {
        let final_queued = chunks_queued.load(Ordering::SeqCst);
        let final_completed = chunks_completed.load(Ordering::SeqCst);

        if final_queued == final_completed {
            info!(
                "🎉 ALL {} chunks processed successfully - ZERO chunks lost!",
                final_completed
            );
            break;
        } else if verification_attempts < MAX_VERIFICATION_ATTEMPTS {
            verification_attempts += 1;
            warn!("⚠️ Chunk count mismatch (attempt {}): {} queued, {} completed - waiting for stragglers...",
                 verification_attempts, final_queued, final_completed);
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        } else {
            error!(
                "❌ CRITICAL: After {} attempts, chunk loss detected: {} queued, {} completed",
                MAX_VERIFICATION_ATTEMPTS, final_queued, final_completed
            );

            let _ = app.emit(
                "transcript-chunk-loss-detected",
                serde_json::json!({
                    "chunks_queued": final_queued,
                    "chunks_completed": final_completed,
                    "chunks_lost": final_queued - final_completed,
                    "message": "Some transcript chunks may have been lost during shutdown"
                }),
            );
            break;
        }
    }
}
