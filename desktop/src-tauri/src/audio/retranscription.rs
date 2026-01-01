// Retranscription module for batch re-transcribing saved audio files
// This provides higher quality transcription by processing the full audio file

use std::path::Path;
use std::process::{Command, Stdio};
use std::io::Read;
use std::sync::Mutex;
use std::collections::HashSet;
use tauri::{AppHandle, Emitter, Runtime};
use serde::{Deserialize, Serialize};
use log::{info, error, debug, warn};
use anyhow::{Result, anyhow};
use once_cell::sync::Lazy;

use super::ffmpeg::find_ffmpeg_path;
use crate::whisper_engine::parallel_processor::AudioChunk;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// Windows flag to prevent console window from appearing
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Global set of recording IDs that should be cancelled
static CANCELLED_RECORDINGS: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

/// Progress information for retranscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetranscriptionProgress {
    pub recording_id: String,
    pub status: String,  // "loading" | "processing" | "completed" | "failed"
    pub progress_percent: u32,
    pub current_chunk: u32,
    pub total_chunks: u32,
    pub message: String,
}

/// Result of retranscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetranscriptionResult {
    pub recording_id: String,
    pub success: bool,
    pub transcripts: Vec<TranscriptSegment>,
    pub error: Option<String>,
    pub model_used: String,
}

/// A transcript segment from retranscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub text: String,
    pub audio_start_time: f64,
    pub audio_end_time: f64,
    pub confidence: f32,
    pub sequence_id: u32,
    // Speaker diarization fields (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker_label: Option<String>,
    #[serde(default)]
    pub is_registered_speaker: bool,
}

/// Emit retranscription progress to frontend
pub fn emit_progress<R: Runtime>(
    app: &AppHandle<R>,
    recording_id: &str,
    status: &str,
    progress: u32,
    current: u32,
    total: u32,
    message: &str,
) {
    let progress = RetranscriptionProgress {
        recording_id: recording_id.to_string(),
        status: status.to_string(),
        progress_percent: progress,
        current_chunk: current,
        total_chunks: total,
        message: message.to_string(),
    };

    if let Err(e) = app.emit("retranscription-progress", &progress) {
        warn!("Failed to emit retranscription progress: {}", e);
    }
}

/// Emit retranscription completion
pub fn emit_complete<R: Runtime>(
    app: &AppHandle<R>,
    result: &RetranscriptionResult,
) {
    if let Err(e) = app.emit("retranscription-complete", result) {
        warn!("Failed to emit retranscription complete: {}", e);
    }
}

/// Check if a recording's retranscription has been cancelled
fn is_cancelled(recording_id: &str) -> bool {
    CANCELLED_RECORDINGS
        .lock()
        .map(|set| set.contains(recording_id))
        .unwrap_or(false)
}

/// Mark a recording for cancellation
fn mark_cancelled(recording_id: &str) {
    if let Ok(mut set) = CANCELLED_RECORDINGS.lock() {
        set.insert(recording_id.to_string());
    }
}

/// Clear cancellation flag for a recording
fn clear_cancelled(recording_id: &str) {
    if let Ok(mut set) = CANCELLED_RECORDINGS.lock() {
        set.remove(recording_id);
    }
}

/// Tauri command to cancel a retranscription in progress
#[tauri::command]
pub async fn cancel_retranscription<R: Runtime>(
    app: AppHandle<R>,
    recording_id: String,
) -> Result<(), String> {
    info!("Cancelling retranscription for recording: {}", recording_id);

    // Mark for cancellation
    mark_cancelled(&recording_id);

    // Emit cancelled status
    emit_progress(&app, &recording_id, "cancelled", 0, 0, 0, "Retranscription cancelled by user");

    emit_complete(&app, &RetranscriptionResult {
        recording_id: recording_id.clone(),
        success: false,
        transcripts: vec![],
        error: Some("Cancelled by user".to_string()),
        model_used: String::new(),
    });

    Ok(())
}

/// Decode audio file to raw f32 samples using FFmpeg
/// Returns mono 16kHz audio samples suitable for Whisper
pub fn decode_audio_file(audio_path: &str) -> Result<(Vec<f32>, u32)> {
    let path = Path::new(audio_path);

    if !path.exists() {
        return Err(anyhow!("Audio file does not exist: {}", audio_path));
    }

    let ffmpeg_path = find_ffmpeg_path()
        .ok_or_else(|| anyhow!("FFmpeg not found. Please install FFmpeg."))?;

    info!("Decoding audio file: {}", audio_path);
    debug!("Using FFmpeg at: {:?}", ffmpeg_path);

    // Use FFmpeg to decode audio to raw PCM f32le at 16kHz mono (Whisper's expected format)
    let mut command = Command::new(&ffmpeg_path);
    
    #[cfg(target_os = "windows")]
    command.creation_flags(CREATE_NO_WINDOW);

    command
        .arg("-i")
        .arg(audio_path)
        .arg("-f")
        .arg("f32le")           // Output format: 32-bit float little-endian
        .arg("-acodec")
        .arg("pcm_f32le")       // Audio codec
        .arg("-ar")
        .arg("16000")           // Sample rate: 16kHz (Whisper's expected rate)
        .arg("-ac")
        .arg("1")               // Mono
        .arg("-")               // Output to stdout
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    debug!("FFmpeg command: {:?}", command);

    let mut child = command.spawn()
        .map_err(|e| anyhow!("Failed to spawn FFmpeg process: {}", e))?;

    let mut stdout = child.stdout.take()
        .ok_or_else(|| anyhow!("Failed to capture FFmpeg stdout"))?;

    // Read all output
    let mut raw_bytes = Vec::new();
    stdout.read_to_end(&mut raw_bytes)?;

    let output = child.wait_with_output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("FFmpeg decode failed: {}", stderr);
        return Err(anyhow!("FFmpeg failed to decode audio: {}", stderr));
    }

    // Convert bytes to f32 samples
    if raw_bytes.len() % 4 != 0 {
        return Err(anyhow!("Invalid audio data length: {} bytes (not divisible by 4)", raw_bytes.len()));
    }

    let samples: Vec<f32> = raw_bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();

    let duration_seconds = samples.len() as f32 / 16000.0;
    info!("Decoded {} samples ({:.2} seconds) from {}", samples.len(), duration_seconds, audio_path);

    Ok((samples, 16000)) // Return samples and sample rate
}

/// Prepare audio samples into chunks for parallel processing
pub fn prepare_chunks(
    samples: Vec<f32>,
    sample_rate: u32,
    chunk_duration_ms: f64,
) -> Vec<AudioChunk> {
    let samples_per_chunk = ((sample_rate as f64 * chunk_duration_ms) / 1000.0) as usize;
    let mut chunks = Vec::new();
    let mut chunk_id = 0;
    let mut start_sample = 0;

    while start_sample < samples.len() {
        let end_sample = (start_sample + samples_per_chunk).min(samples.len());
        let chunk_data = samples[start_sample..end_sample].to_vec();

        let start_time_ms = (start_sample as f64 / sample_rate as f64) * 1000.0;
        let duration_ms = (chunk_data.len() as f64 / sample_rate as f64) * 1000.0;

        chunks.push(AudioChunk {
            id: chunk_id,
            data: chunk_data,
            sample_rate,
            start_time_ms,
            duration_ms,
        });

        chunk_id += 1;
        start_sample = end_sample;
    }

    info!("Prepared {} chunks of {:.1}s each for retranscription",
          chunks.len(), chunk_duration_ms / 1000.0);

    chunks
}

/// Align speaker segments with transcript segments by time overlap
/// For each transcript segment, find the speaker segment with the most overlap
#[allow(dead_code)]
fn align_speakers_with_transcripts(
    mut transcripts: Vec<TranscriptSegment>,
    speaker_segments: &[crate::diarization::SpeakerSegment],
) -> Vec<TranscriptSegment> {
    for transcript in &mut transcripts {
        // Find the speaker segment with the most overlap with this transcript
        let mut best_match: Option<(&crate::diarization::SpeakerSegment, f64)> = None;

        for speaker_seg in speaker_segments {
            // Calculate overlap between transcript and speaker segment
            let overlap_start = transcript.audio_start_time.max(speaker_seg.start_time);
            let overlap_end = transcript.audio_end_time.min(speaker_seg.end_time);
            let overlap = (overlap_end - overlap_start).max(0.0);

            if overlap > 0.0 {
                // Calculate overlap ratio (what percentage of transcript is covered)
                let transcript_duration = transcript.audio_end_time - transcript.audio_start_time;
                let overlap_ratio = if transcript_duration > 0.0 {
                    overlap / transcript_duration
                } else {
                    0.0
                };

                // Keep track of best match (highest overlap ratio)
                if let Some((_, best_ratio)) = best_match {
                    if overlap_ratio > best_ratio {
                        best_match = Some((speaker_seg, overlap_ratio));
                    }
                } else {
                    best_match = Some((speaker_seg, overlap_ratio));
                }
            }
        }

        // Assign speaker info if we found a match with sufficient overlap
        if let Some((speaker_seg, ratio)) = best_match {
            if ratio >= 0.25 {
                transcript.speaker_id = Some(speaker_seg.speaker_id.clone());
                transcript.speaker_label = Some(speaker_seg.speaker_label.clone());
                transcript.is_registered_speaker = speaker_seg.is_registered;

                debug!("Transcript [{:.1}s-{:.1}s] assigned to {} ({}% overlap)",
                       transcript.audio_start_time, transcript.audio_end_time,
                       speaker_seg.speaker_label, (ratio * 100.0) as u32);
            } else {
                debug!("Transcript [{:.1}s-{:.1}s] best match was {} with only {:.0}% overlap (below 25% threshold)",
                       transcript.audio_start_time, transcript.audio_end_time,
                       speaker_seg.speaker_label, ratio * 100.0);
            }
        } else {
            debug!("Transcript [{:.1}s-{:.1}s] had no overlapping speaker segments",
                   transcript.audio_start_time, transcript.audio_end_time);
        }
    }

    transcripts
}

/// Assign speakers to transcripts and merge consecutive same-speaker segments
/// This preserves all original text while adding speaker labels
fn assign_and_merge_speakers(
    mut transcripts: Vec<TranscriptSegment>,
    speaker_segments: &[crate::diarization::SpeakerSegment],
) -> Vec<TranscriptSegment> {
    // Phase 1: Assign speaker to each transcript based on majority overlap
    for transcript in &mut transcripts {
        let mut best_match: Option<(&crate::diarization::SpeakerSegment, f64)> = None;

        for speaker_seg in speaker_segments {
            // Calculate overlap between transcript and speaker segment
            let overlap_start = transcript.audio_start_time.max(speaker_seg.start_time);
            let overlap_end = transcript.audio_end_time.min(speaker_seg.end_time);
            let overlap = (overlap_end - overlap_start).max(0.0);

            if overlap > 0.0 {
                let transcript_duration = transcript.audio_end_time - transcript.audio_start_time;
                let overlap_ratio = if transcript_duration > 0.0 {
                    overlap / transcript_duration
                } else {
                    0.0
                };

                if let Some((_, best_ratio)) = best_match {
                    if overlap_ratio > best_ratio {
                        best_match = Some((speaker_seg, overlap_ratio));
                    }
                } else {
                    best_match = Some((speaker_seg, overlap_ratio));
                }
            }
        }

        // Assign speaker if we found any overlap
        if let Some((speaker_seg, ratio)) = best_match {
            transcript.speaker_id = Some(speaker_seg.speaker_id.clone());
            transcript.speaker_label = Some(speaker_seg.speaker_label.clone());
            transcript.is_registered_speaker = speaker_seg.is_registered;
            debug!("Transcript [{:.1}s-{:.1}s] assigned to {} ({:.0}% overlap)",
                   transcript.audio_start_time, transcript.audio_end_time,
                   speaker_seg.speaker_label, ratio * 100.0);
        }
    }

    // Phase 2: Merge consecutive segments with same speaker
    let original_count = transcripts.len();
    let mut merged: Vec<TranscriptSegment> = Vec::new();

    for segment in transcripts {
        if let Some(last) = merged.last_mut() {
            // Same speaker and close in time (< 2 seconds gap)? Merge text
            let same_speaker = last.speaker_id == segment.speaker_id;
            let time_gap = segment.audio_start_time - last.audio_end_time;

            if same_speaker && time_gap < 2.0 {
                // Merge: append text with space, extend end time
                last.text.push(' ');
                last.text.push_str(&segment.text);
                last.audio_end_time = segment.audio_end_time;
                continue;
            }
        }
        merged.push(segment);
    }

    // Phase 3: Renumber sequence_ids
    for (i, seg) in merged.iter_mut().enumerate() {
        seg.sequence_id = i as u32;
    }

    info!("Assigned speakers and merged {} segments into {} segments",
          original_count, merged.len());

    merged
}

/// Split transcript segments at speaker boundaries
/// Takes transcripts and speaker segments, returns finer-grained transcripts
#[allow(dead_code)]
fn split_transcripts_by_speakers(
    transcripts: Vec<TranscriptSegment>,
    speaker_segments: &[crate::diarization::SpeakerSegment],
) -> Vec<TranscriptSegment> {
    let mut result = Vec::new();
    let mut sequence_id: u32 = 0;
    let original_count = transcripts.len();

    for transcript in transcripts {
        // Find all speaker segments that overlap with this transcript
        let mut overlapping: Vec<_> = speaker_segments
            .iter()
            .filter(|s| s.start_time < transcript.audio_end_time && s.end_time > transcript.audio_start_time)
            .collect();

        // Sort by start time to maintain order
        overlapping.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap_or(std::cmp::Ordering::Equal));

        if overlapping.is_empty() {
            // No speaker segments - keep original
            let mut t = transcript;
            t.sequence_id = sequence_id;
            sequence_id += 1;
            result.push(t);
            continue;
        }

        if overlapping.len() == 1 {
            // Single speaker - assign and keep
            let speaker = &overlapping[0];
            let mut t = transcript;
            t.speaker_id = Some(speaker.speaker_id.clone());
            t.speaker_label = Some(speaker.speaker_label.clone());
            t.is_registered_speaker = speaker.is_registered;
            t.sequence_id = sequence_id;
            sequence_id += 1;
            result.push(t);
            continue;
        }

        // Multiple speakers - split the transcript
        let transcript_duration = transcript.audio_end_time - transcript.audio_start_time;
        let text = &transcript.text;
        let text_len = text.len() as f64;

        for speaker in overlapping {
            // Calculate the time range this speaker covers within the transcript
            let seg_start = speaker.start_time.max(transcript.audio_start_time);
            let seg_end = speaker.end_time.min(transcript.audio_end_time);

            // Calculate proportional text positions
            let start_ratio = (seg_start - transcript.audio_start_time) / transcript_duration;
            let end_ratio = (seg_end - transcript.audio_start_time) / transcript_duration;

            let char_start = (start_ratio * text_len) as usize;
            let char_end = (end_ratio * text_len) as usize;

            // Extract text portion, trying to break at word boundaries
            let segment_text = extract_text_portion(text, char_start, char_end);

            if segment_text.trim().is_empty() {
                continue;
            }

            debug!("Split segment [{:.1}s-{:.1}s] -> {} (chars {}-{})",
                   seg_start, seg_end, speaker.speaker_label, char_start, char_end);

            result.push(TranscriptSegment {
                text: segment_text.trim().to_string(),
                audio_start_time: seg_start,
                audio_end_time: seg_end,
                confidence: transcript.confidence,
                sequence_id,
                speaker_id: Some(speaker.speaker_id.clone()),
                speaker_label: Some(speaker.speaker_label.clone()),
                is_registered_speaker: speaker.is_registered,
            });
            sequence_id += 1;
        }
    }

    info!("Split {} original segments into {} speaker-aligned segments",
          original_count, result.len());

    result
}

/// Extract a portion of text, trying to break at word boundaries
fn extract_text_portion(text: &str, start: usize, end: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    if start >= len {
        return String::new();
    }

    let end = end.min(len);

    // Adjust start to next word boundary (after space)
    let adjusted_start = if start > 0 {
        (start..end.min(start + 20))
            .find(|&i| i < len && chars.get(i.saturating_sub(1)) == Some(&' '))
            .unwrap_or(start)
    } else {
        start
    };

    // Adjust end to previous word boundary (before space)
    let adjusted_end = if end < len {
        (adjusted_start.max(end.saturating_sub(20))..end)
            .rev()
            .find(|&i| chars.get(i) == Some(&' '))
            .unwrap_or(end)
    } else {
        end
    };

    chars[adjusted_start..adjusted_end].iter().collect()
}

/// Get audio duration in seconds from an audio file
pub fn get_audio_duration(audio_path: &str) -> Result<f64> {
    let ffmpeg_path = find_ffmpeg_path()
        .ok_or_else(|| anyhow!("FFmpeg not found"))?;

    // Use ffprobe-style query with ffmpeg
    let mut cmd = Command::new(&ffmpeg_path);
    
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    
    let output = cmd
        .arg("-i")
        .arg(audio_path)
        .arg("-f")
        .arg("null")
        .arg("-")
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| anyhow!("Failed to run FFmpeg: {}", e))?;

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Parse duration from FFmpeg output (format: "Duration: HH:MM:SS.ms")
    for line in stderr.lines() {
        if line.contains("Duration:") {
            if let Some(duration_str) = line.split("Duration:").nth(1) {
                if let Some(time_str) = duration_str.split(',').next() {
                    let time_str = time_str.trim();
                    let parts: Vec<&str> = time_str.split(':').collect();
                    if parts.len() == 3 {
                        let hours: f64 = parts[0].parse().unwrap_or(0.0);
                        let minutes: f64 = parts[1].parse().unwrap_or(0.0);
                        let seconds: f64 = parts[2].parse().unwrap_or(0.0);
                        return Ok(hours * 3600.0 + minutes * 60.0 + seconds);
                    }
                }
            }
        }
    }

    Err(anyhow!("Could not determine audio duration"))
}

/// Tauri command to start retranscription of a recording
/// This runs in the background and emits progress events
#[tauri::command]
pub async fn retranscribe_recording<R: Runtime>(
    app: AppHandle<R>,
    recording_id: String,
    audio_file_path: String,
    model_name: Option<String>,
    language: Option<String>,
    enable_diarization: Option<bool>,
    diarization_provider: Option<String>,
    max_speakers: Option<usize>,
    similarity_threshold: Option<f32>,
) -> Result<(), String> {
    use crate::whisper_engine::commands::WHISPER_ENGINE;
    use crate::diarization::DIARIZATION_ENGINE;
    use crate::diarization::sortformer_provider::SORTFORMER_ENGINE;

    let diarization_enabled = enable_diarization.unwrap_or(false);
    let provider = diarization_provider.as_deref().unwrap_or("pyannote");

    // Use provided values or defaults for pyannote settings
    let max_spk = max_speakers.unwrap_or(10);
    let sim_threshold = similarity_threshold.unwrap_or(0.4);

    info!("Starting retranscription for recording: {}", recording_id);
    info!("Audio file: {}", audio_file_path);
    info!("Model: {:?}, Language: {:?}, Diarization: {} (provider: {}, max_speakers: {}, threshold: {:.2})",
          model_name, language, diarization_enabled, provider, max_spk, sim_threshold);

    // Clear any previous cancellation flag for this recording
    clear_cancelled(&recording_id);

    // Emit initial progress
    emit_progress(&app, &recording_id, "loading", 0, 0, 0, "Loading audio file...");

    // Decode the audio file
    let (samples, sample_rate) = match decode_audio_file(&audio_file_path) {
        Ok(result) => result,
        Err(e) => {
            let error_msg = format!("Failed to decode audio: {}", e);
            error!("{}", error_msg);
            emit_complete(&app, &RetranscriptionResult {
                recording_id: recording_id.clone(),
                success: false,
                transcripts: vec![],
                error: Some(error_msg.clone()),
                model_used: model_name.clone().unwrap_or_default(),
            });
            return Err(error_msg);
        }
    };

    let duration_seconds = samples.len() as f64 / sample_rate as f64;
    info!("Audio duration: {:.2} seconds", duration_seconds);

    // Prepare chunks (30 second chunks for better accuracy)
    let chunk_duration_ms = 30000.0; // 30 seconds per chunk
    let chunks = prepare_chunks(samples, sample_rate, chunk_duration_ms);
    let total_chunks = chunks.len() as u32;

    emit_progress(&app, &recording_id, "processing", 5, 0, total_chunks,
                  &format!("Processing {} chunks...", total_chunks));

    // Get Whisper engine
    let engine = {
        let guard = WHISPER_ENGINE.lock().unwrap();
        match guard.as_ref() {
            Some(e) => e.clone(),
            None => {
                let error_msg = "Whisper engine not initialized".to_string();
                error!("{}", error_msg);
                emit_complete(&app, &RetranscriptionResult {
                    recording_id: recording_id.clone(),
                    success: false,
                    transcripts: vec![],
                    error: Some(error_msg.clone()),
                    model_used: model_name.clone().unwrap_or_default(),
                });
                return Err(error_msg);
            }
        }
    };

    // Load the requested model if specified and different from current
    let model = model_name.clone().unwrap_or_else(|| "current".to_string());
    if model != "current" {
        // Check if we need to load a different model
        let current_model = engine.get_current_model().await;
        if current_model.as_deref() != Some(model.as_str()) {
            info!("Loading model '{}' for retranscription (current: {:?})", model, current_model);
            emit_progress(&app, &recording_id, "loading", 2, 0, 0,
                          &format!("Loading model '{}'...", model));

            if let Err(e) = engine.load_model(&model).await {
                let error_msg = format!("Failed to load model '{}': {}", model, e);
                error!("{}", error_msg);
                emit_complete(&app, &RetranscriptionResult {
                    recording_id: recording_id.clone(),
                    success: false,
                    transcripts: vec![],
                    error: Some(error_msg.clone()),
                    model_used: model.clone(),
                });
                return Err(error_msg);
            }
            info!("Model '{}' loaded successfully for retranscription", model);
        } else {
            debug!("Model '{}' already loaded, using it for retranscription", model);
        }
    } else {
        debug!("Using currently loaded model for retranscription");
    }

    // Process each chunk
    let mut transcripts: Vec<TranscriptSegment> = Vec::new();

    for (idx, chunk) in chunks.iter().enumerate() {
        // Check for cancellation before processing each chunk
        if is_cancelled(&recording_id) {
            info!("Retranscription cancelled for recording: {}", recording_id);
            clear_cancelled(&recording_id);
            return Ok(()); // Exit gracefully - cancellation event already emitted
        }

        let progress_percent = ((idx as f64 / total_chunks as f64) * 90.0 + 5.0) as u32;
        emit_progress(&app, &recording_id, "processing", progress_percent,
                      idx as u32 + 1, total_chunks,
                      &format!("Transcribing chunk {} of {}...", idx + 1, total_chunks));

        // Transcribe the chunk
        match engine.transcribe_audio(chunk.data.clone(), language.clone()).await {
            Ok(text) => {
                if !text.trim().is_empty() {
                    transcripts.push(TranscriptSegment {
                        text: text.trim().to_string(),
                        audio_start_time: chunk.start_time_ms / 1000.0, // Convert to seconds
                        audio_end_time: (chunk.start_time_ms + chunk.duration_ms) / 1000.0,
                        confidence: 0.95, // Placeholder - could be extracted from Whisper
                        sequence_id: idx as u32,
                        // Speaker info will be added after diarization if enabled
                        speaker_id: None,
                        speaker_label: None,
                        is_registered_speaker: false,
                    });
                }
            }
            Err(e) => {
                warn!("Failed to transcribe chunk {}: {}", idx, e);
                // Continue with other chunks even if one fails
            }
        }

        // Check for cancellation after processing each chunk as well
        if is_cancelled(&recording_id) {
            info!("Retranscription cancelled after chunk {} for recording: {}", idx, recording_id);
            clear_cancelled(&recording_id);
            return Ok(()); // Exit gracefully - cancellation event already emitted
        }
    }

    info!("Transcription complete: {} segments", transcripts.len());

    // Run diarization if enabled
    if diarization_enabled && !transcripts.is_empty() {
        let provider_name = if provider == "sortformer" { "Sortformer" } else { "PyAnnote" };

        emit_progress(&app, &recording_id, "diarizing", 95, total_chunks, total_chunks,
                      &format!("Loading {} diarization model...", provider_name));

        // Re-decode audio for diarization (need fresh samples)
        match decode_audio_file(&audio_file_path) {
            Ok((diarization_samples, diarization_rate)) => {
                let speaker_segments: Option<Vec<crate::diarization::SpeakerSegment>> = if provider == "sortformer" {
                    // Use Sortformer for diarization
                    info!("Using Sortformer for diarization");

                    let mut guard = SORTFORMER_ENGINE.write().await;

                    // Auto-initialize if not already initialized
                    if guard.is_none() {
                        info!("Sortformer engine not initialized, attempting auto-initialization...");
                        use tauri::Manager;
                        if let Ok(app_data_dir) = app.path().app_data_dir() {
                            let models_dir = app_data_dir.join("models");
                            let model_path = models_dir.join(crate::diarization::SORTFORMER_MODEL_NAME);

                            if model_path.exists() {
                                info!("Found Sortformer model, initializing engine...");
                                match crate::diarization::SortformerEngine::new(model_path) {
                                    Ok(engine) => {
                                        *guard = Some(engine);
                                        info!("Sortformer engine initialized successfully");
                                    }
                                    Err(e) => {
                                        warn!("Failed to initialize Sortformer engine: {}", e);
                                    }
                                }
                            } else {
                                warn!("Sortformer model not found at {:?}", model_path);
                            }
                        }
                    }

                    if let Some(sortformer_engine) = guard.as_mut() {
                        sortformer_engine.reset();

                        emit_progress(&app, &recording_id, "diarizing", 96, total_chunks, total_chunks,
                                      "Detecting speakers in audio...");

                        match sortformer_engine.diarize(diarization_samples, diarization_rate) {
                            Ok(segments) => {
                                info!("Sortformer diarization found {} speaker segments", segments.len());
                                // Convert Sortformer segments to our format
                                Some(segments.into_iter().map(|s| crate::diarization::SpeakerSegment {
                                    start_time: s.start as f64,
                                    end_time: s.end as f64,
                                    speaker_id: format!("speaker_{}", s.speaker_id),
                                    speaker_label: format!("Speaker {}", s.speaker_id + 1),
                                    confidence: 0.9, // Sortformer doesn't provide confidence
                                    is_registered: false,
                                    registered_speaker_id: None,
                                }).collect())
                            }
                            Err(e) => {
                                warn!("Sortformer diarization failed: {}", e);
                                None
                            }
                        }
                    } else {
                        warn!("Sortformer engine not initialized, skipping speaker identification");
                        None
                    }
                } else {
                    // Use PyAnnote for diarization (default)
                    info!("Using PyAnnote for diarization");

                    let mut guard = DIARIZATION_ENGINE.write().await;

                    // Auto-initialize if not already initialized
                    if guard.is_none() {
                        info!("Diarization engine not initialized, attempting auto-initialization...");

                        // Get models directory from app handle
                        use tauri::Manager;
                        if let Ok(app_data_dir) = app.path().app_data_dir() {
                            let models_dir = app_data_dir.join("models");
                            let seg_path = models_dir.join(crate::diarization::SEGMENTATION_MODEL_NAME);
                            let emb_path = models_dir.join(crate::diarization::EMBEDDING_MODEL_NAME);

                            if seg_path.exists() && emb_path.exists() {
                                info!("Found diarization models, initializing engine...");
                                match crate::diarization::DiarizationEngine::new(
                                    crate::diarization::DiarizationConfig {
                                        segmentation_model_path: seg_path,
                                        embedding_model_path: emb_path,
                                        max_speakers: max_spk,
                                        similarity_threshold: sim_threshold,
                                    }
                                ) {
                                    Ok(engine) => {
                                        *guard = Some(engine);
                                        info!("Diarization engine initialized successfully");
                                    }
                                    Err(e) => {
                                        warn!("Failed to initialize diarization engine: {}", e);
                                    }
                                }
                            } else {
                                warn!("Diarization models not found at {:?}", models_dir);
                            }
                        }
                    }

                    if let Some(diarization_engine) = guard.as_mut() {
                        // Update configuration with user-specified values
                        diarization_engine.update_config(Some(max_spk), Some(sim_threshold));

                        emit_progress(&app, &recording_id, "diarizing", 96, total_chunks, total_chunks,
                                      "Detecting speakers in audio...");

                        // Run diarization on the full audio
                        match diarization_engine.diarize(&diarization_samples, diarization_rate) {
                            Ok(segments) => {
                                info!("PyAnnote diarization found {} speaker segments", segments.len());
                                Some(segments)
                            }
                            Err(e) => {
                                warn!("PyAnnote diarization failed: {}", e);
                                None
                            }
                        }
                    } else {
                        warn!("Diarization engine not initialized, skipping speaker identification");
                        None
                    }
                };

                // Apply speaker segments to transcripts if diarization succeeded
                if let Some(segments) = speaker_segments {
                    emit_progress(&app, &recording_id, "diarizing", 98, total_chunks, total_chunks,
                                  "Assigning speakers to transcript...");

                    transcripts = assign_and_merge_speakers(transcripts, &segments);
                }
            }
            Err(e) => {
                warn!("Failed to decode audio for diarization: {}", e);
            }
        }
    }

    info!("Retranscription complete: {} segments", transcripts.len());

    // Emit completion
    emit_progress(&app, &recording_id, "completed", 100, total_chunks, total_chunks,
                  "Retranscription complete!");

    let result = RetranscriptionResult {
        recording_id: recording_id.clone(),
        success: true,
        transcripts,
        error: None,
        model_used: model,
    };

    emit_complete(&app, &result);

    Ok(())
}

/// Get status of a retranscription job (placeholder for future job tracking)
#[tauri::command]
pub async fn get_retranscription_status(
    recording_id: String,
) -> Result<serde_json::Value, String> {
    // For now, return a simple status
    // In the future, we could track active jobs in a HashMap
    Ok(serde_json::json!({
        "recording_id": recording_id,
        "status": "unknown",
        "message": "Job tracking not yet implemented"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prepare_chunks() {
        // Create 5 seconds of dummy audio at 16kHz
        let sample_rate = 16000;
        let samples: Vec<f32> = vec![0.0; 16000 * 5]; // 5 seconds

        let chunks = prepare_chunks(samples, sample_rate, 1000.0); // 1 second chunks

        assert_eq!(chunks.len(), 5);
        assert_eq!(chunks[0].id, 0);
        assert_eq!(chunks[0].duration_ms, 1000.0);
        assert_eq!(chunks[0].start_time_ms, 0.0);
        assert_eq!(chunks[4].start_time_ms, 4000.0);
    }
}
