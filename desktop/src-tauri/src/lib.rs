// Meeting-Local - Simplified audio recording and transcription app
//
// This is a simplified version that only includes:
// - Audio recording (microphone + system audio)
// - Whisper transcription
// - Basic device management

use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;

// Performance logging macros - exported for use by other modules
#[macro_use]
pub mod macros;

// Global state
pub mod globals;
use globals::{RECORDING_FLAG, LANGUAGE_PREFERENCE};

// Core modules
pub mod audio;
pub mod whisper_engine;
pub mod state;
pub mod database;
pub mod diarization;
pub mod llm_engine;
pub mod chat;
pub mod templates;
pub mod tools;
pub mod mcp;

// Stub modules for removed MeetLocal features
pub mod stubs;

// Re-export stubs for backwards compatibility
pub mod tray {
    pub use crate::stubs::tray::*;
}

pub mod api {
    pub mod api {
        pub use crate::stubs::api::*;
    }
}

pub mod analytics {
    pub mod commands {
        pub use crate::stubs::analytics::*;
    }
}

pub mod parakeet_engine {
    pub use crate::stubs::parakeet::ParakeetEngine;

    pub mod commands {
        pub use crate::stubs::parakeet::{PARAKEET_ENGINE, parakeet_init, parakeet_validate_model_ready_with_config};
    }
}

use audio::{list_audio_devices, AudioDevice};
use log::{error as log_error, info as log_info};
use tauri::{AppHandle, Manager, Runtime};

// Re-export for backwards compatibility
pub use globals::get_language_preference_internal;

#[tauri::command]
fn get_language_preference() -> Option<String> {
    let guard = LANGUAGE_PREFERENCE.lock().ok()?;
    guard.clone()
}

#[tauri::command]
fn set_language_preference(language: String) -> Result<(), String> {
    log_info!("Setting language preference to: {}", language);
    let mut guard = LANGUAGE_PREFERENCE.lock().map_err(|e| format!("Lock error: {}", e))?;
    *guard = if language == "auto" { None } else { Some(language) };
    Ok(())
}

// ============== Audio Processing Commands ==============
// Per-source audio processing controls (mic and system audio)

// --- Microphone Processing ---

#[tauri::command]
fn get_mic_rnnoise_enabled() -> bool {
    audio::ffmpeg_mixer::is_mic_rnnoise_enabled()
}

#[tauri::command]
fn set_mic_rnnoise_enabled(enabled: bool) -> Result<(), String> {
    audio::ffmpeg_mixer::set_mic_rnnoise_enabled(enabled);
    Ok(())
}

#[tauri::command]
fn get_mic_highpass_enabled() -> bool {
    audio::ffmpeg_mixer::is_mic_highpass_enabled()
}

#[tauri::command]
fn set_mic_highpass_enabled(enabled: bool) -> Result<(), String> {
    audio::ffmpeg_mixer::set_mic_highpass_enabled(enabled);
    Ok(())
}

#[tauri::command]
fn get_mic_normalizer_enabled() -> bool {
    audio::ffmpeg_mixer::is_mic_normalizer_enabled()
}

#[tauri::command]
fn set_mic_normalizer_enabled(enabled: bool) -> Result<(), String> {
    audio::ffmpeg_mixer::set_mic_normalizer_enabled(enabled);
    Ok(())
}

// --- System Audio Processing ---

#[tauri::command]
fn get_sys_rnnoise_enabled() -> bool {
    audio::ffmpeg_mixer::is_sys_rnnoise_enabled()
}

#[tauri::command]
fn set_sys_rnnoise_enabled(enabled: bool) -> Result<(), String> {
    audio::ffmpeg_mixer::set_sys_rnnoise_enabled(enabled);
    Ok(())
}

#[tauri::command]
fn get_sys_highpass_enabled() -> bool {
    audio::ffmpeg_mixer::is_sys_highpass_enabled()
}

#[tauri::command]
fn set_sys_highpass_enabled(enabled: bool) -> Result<(), String> {
    audio::ffmpeg_mixer::set_sys_highpass_enabled(enabled);
    Ok(())
}

#[tauri::command]
fn get_sys_normalizer_enabled() -> bool {
    audio::ffmpeg_mixer::is_sys_normalizer_enabled()
}

#[tauri::command]
fn set_sys_normalizer_enabled(enabled: bool) -> Result<(), String> {
    audio::ffmpeg_mixer::set_sys_normalizer_enabled(enabled);
    Ok(())
}

// --- Legacy commands (backward compatibility) ---

#[tauri::command]
fn get_noise_suppression_enabled() -> bool {
    audio::ffmpeg_mixer::is_rnnoise_enabled()
}

#[tauri::command]
fn set_noise_suppression_enabled(enabled: bool) -> Result<(), String> {
    log_info!("Setting noise suppression to: {}", enabled);
    audio::ffmpeg_mixer::set_rnnoise_enabled(enabled);
    Ok(())
}

// ============== Database Commands ==============

use database::{
    AllSettings, Recording, RecordingUpdate, RecordingWithMetadata,
    TranscriptSegment, Category, Tag, SearchResult, SearchFilters,
};

#[tauri::command]
async fn db_get_setting(
    key: String,
    state: tauri::State<'_, state::AppState>,
) -> Result<Option<String>, String> {
    let db = state.db().await;
    db.get_setting(&key).map_err(|e| e.to_string())
}

#[tauri::command]
async fn db_set_setting(
    key: String,
    value: String,
    value_type: String,
    state: tauri::State<'_, state::AppState>,
) -> Result<(), String> {
    let db = state.db().await;
    db.set_setting(&key, &value, &value_type).map_err(|e| e.to_string())
}

#[tauri::command]
async fn db_get_all_settings(
    state: tauri::State<'_, state::AppState>,
) -> Result<Vec<database::Setting>, String> {
    let db = state.db().await;
    db.get_all_settings_list().map_err(|e| e.to_string())
}

#[tauri::command]
async fn db_load_settings_on_startup(
    state: tauri::State<'_, state::AppState>,
) -> Result<AllSettings, String> {
    let db = state.db().await;
    db.load_all_settings().map_err(|e| e.to_string())
}

// Recording commands
#[tauri::command]
async fn db_create_recording(
    recording: Recording,
    state: tauri::State<'_, state::AppState>,
) -> Result<String, String> {
    let db = state.db().await;
    db.create_recording(&recording).map_err(|e| e.to_string())
}

#[tauri::command]
async fn db_get_recording(
    id: String,
    state: tauri::State<'_, state::AppState>,
) -> Result<Option<RecordingWithMetadata>, String> {
    let db = state.db().await;
    db.get_recording_with_metadata(&id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn db_get_all_recordings(
    state: tauri::State<'_, state::AppState>,
) -> Result<Vec<RecordingWithMetadata>, String> {
    let db = state.db().await;
    db.get_all_recordings().map_err(|e| e.to_string())
}

#[tauri::command]
async fn db_get_recent_recordings(
    limit: i32,
    state: tauri::State<'_, state::AppState>,
) -> Result<Vec<RecordingWithMetadata>, String> {
    let db = state.db().await;
    db.get_recent_recordings(limit).map_err(|e| e.to_string())
}

#[tauri::command]
async fn db_update_recording(
    id: String,
    updates: RecordingUpdate,
    state: tauri::State<'_, state::AppState>,
) -> Result<(), String> {
    let db = state.db().await;
    db.update_recording(&id, &updates).map_err(|e| e.to_string())
}

#[tauri::command]
async fn db_delete_recording(
    id: String,
    state: tauri::State<'_, state::AppState>,
) -> Result<(), String> {
    let db = state.db().await;

    // Get the recording first to find file paths
    let recording = db.get_recording(&id).map_err(|e| e.to_string())?;

    // Delete from database first (cascades to transcripts, categories, tags, chat messages, etc.)
    db.delete_recording(&id).map_err(|e| e.to_string())?;

    // Then delete files from disk
    if let Some(recording) = recording {
        // Delete the meeting folder (contains audio and other files)
        if let Some(folder_path) = recording.meeting_folder_path {
            let folder = std::path::Path::new(&folder_path);
            if folder.exists() && folder.is_dir() {
                if let Err(e) = std::fs::remove_dir_all(&folder) {
                    log::warn!("Failed to delete meeting folder {}: {}", folder_path, e);
                } else {
                    log::info!("Deleted meeting folder: {}", folder_path);
                }
            }
        } else if let Some(audio_path) = recording.audio_file_path {
            // Fallback: if no folder path, just delete the audio file
            let audio_file = std::path::Path::new(&audio_path);
            if audio_file.exists() {
                if let Err(e) = std::fs::remove_file(&audio_file) {
                    log::warn!("Failed to delete audio file {}: {}", audio_path, e);
                } else {
                    log::info!("Deleted audio file: {}", audio_path);
                }
            }
        }
    }

    log::info!("Successfully deleted recording: {}", id);
    Ok(())
}

#[tauri::command]
async fn db_complete_recording(
    id: String,
    duration: f64,
    state: tauri::State<'_, state::AppState>,
) -> Result<(), String> {
    let db = state.db().await;
    db.complete_recording(&id, duration).map_err(|e| e.to_string())
}

// Transcript commands
#[tauri::command]
async fn db_save_transcript_segment(
    segment: TranscriptSegment,
    state: tauri::State<'_, state::AppState>,
) -> Result<(), String> {
    let db = state.db().await;
    db.save_transcript_segment(&segment).map_err(|e| e.to_string())
}

#[tauri::command]
async fn db_save_transcript_segments_batch(
    segments: Vec<TranscriptSegment>,
    state: tauri::State<'_, state::AppState>,
) -> Result<(), String> {
    let db = state.db().await;
    db.save_transcript_segments_batch(&segments).map_err(|e| e.to_string())
}

#[tauri::command]
async fn db_get_transcript_segments(
    recording_id: String,
    state: tauri::State<'_, state::AppState>,
) -> Result<Vec<TranscriptSegment>, String> {
    let db = state.db().await;
    db.get_transcript_segments(&recording_id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn db_replace_transcripts(
    recording_id: String,
    segments: Vec<TranscriptSegment>,
    state: tauri::State<'_, state::AppState>,
) -> Result<(), String> {
    let db = state.db().await;
    db.replace_transcripts(&recording_id, &segments).map_err(|e| e.to_string())
}

#[tauri::command]
async fn db_update_speaker_label(
    speaker_id: String,
    new_label: String,
    state: tauri::State<'_, state::AppState>,
) -> Result<usize, String> {
    let db = state.db().await;
    db.update_speaker_label(&speaker_id, &new_label).map_err(|e| e.to_string())
}

#[tauri::command]
async fn db_update_transcript_text(
    segment_id: String,
    new_text: String,
    state: tauri::State<'_, state::AppState>,
) -> Result<(), String> {
    let db = state.db().await;
    db.update_transcript_text(&segment_id, &new_text).map_err(|e| e.to_string())
}

// Category commands
#[tauri::command]
async fn db_get_all_categories(
    state: tauri::State<'_, state::AppState>,
) -> Result<Vec<Category>, String> {
    let db = state.db().await;
    db.get_all_categories().map_err(|e| e.to_string())
}

#[tauri::command]
async fn db_create_category(
    name: String,
    color: Option<String>,
    state: tauri::State<'_, state::AppState>,
) -> Result<String, String> {
    let db = state.db().await;
    db.create_category(&name, color.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn db_assign_category(
    recording_id: String,
    category_id: String,
    state: tauri::State<'_, state::AppState>,
) -> Result<(), String> {
    let db = state.db().await;
    db.assign_category(&recording_id, &category_id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn db_remove_category(
    recording_id: String,
    category_id: String,
    state: tauri::State<'_, state::AppState>,
) -> Result<(), String> {
    let db = state.db().await;
    db.remove_category(&recording_id, &category_id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn db_delete_category(
    category_id: String,
    state: tauri::State<'_, state::AppState>,
) -> Result<(), String> {
    let db = state.db().await;
    db.delete_category(&category_id).map_err(|e| e.to_string())
}

// Tag commands
#[tauri::command]
async fn db_get_all_tags(
    state: tauri::State<'_, state::AppState>,
) -> Result<Vec<Tag>, String> {
    let db = state.db().await;
    db.get_all_tags().map_err(|e| e.to_string())
}

#[tauri::command]
async fn db_create_tag(
    name: String,
    color: Option<String>,
    state: tauri::State<'_, state::AppState>,
) -> Result<String, String> {
    let db = state.db().await;
    db.create_tag(&name, color.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn db_assign_tag(
    recording_id: String,
    tag_id: String,
    state: tauri::State<'_, state::AppState>,
) -> Result<(), String> {
    let db = state.db().await;
    db.assign_tag(&recording_id, &tag_id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn db_remove_tag(
    recording_id: String,
    tag_id: String,
    state: tauri::State<'_, state::AppState>,
) -> Result<(), String> {
    let db = state.db().await;
    db.remove_tag(&recording_id, &tag_id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn db_delete_tag(
    tag_id: String,
    state: tauri::State<'_, state::AppState>,
) -> Result<(), String> {
    let db = state.db().await;
    db.delete_tag(&tag_id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn db_get_or_create_tag(
    name: String,
    color: Option<String>,
    state: tauri::State<'_, state::AppState>,
) -> Result<String, String> {
    let db = state.db().await;
    db.get_or_create_tag(&name, color.as_deref()).map_err(|e| e.to_string())
}

// Search command
#[tauri::command]
async fn db_search_recordings(
    query: String,
    filters: SearchFilters,
    state: tauri::State<'_, state::AppState>,
) -> Result<Vec<SearchResult>, String> {
    let db = state.db().await;
    db.search_recordings(&query, &filters).map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
struct RecordingArgs {
    save_path: String,
}

#[derive(Debug, Deserialize, Default)]
struct StartRecordingArgs {
    #[serde(default)]
    mic_device_name: Option<String>,
    #[serde(default)]
    system_device_name: Option<String>,
    #[serde(default)]
    meeting_name: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct TranscriptionStatus {
    chunks_in_queue: usize,
    is_processing: bool,
    last_activity_ms: u64,
}

// ============== Hardware Recommendations ==============

#[tauri::command]
fn get_hardware_recommendations() -> audio::HardwareRecommendations {
    let profile = audio::HardwareProfile::detect();
    profile.get_model_recommendations()
}

// ============== Recording Commands ==============

#[tauri::command]
async fn start_recording<R: Runtime>(
    app: AppHandle<R>,
    args: StartRecordingArgs,
) -> Result<(), String> {
    log_info!("Starting recording with args: {:?}", args);

    if is_recording().await {
        return Err("Recording already in progress".to_string());
    }

    match audio::recording::lifecycle::start_recording_with_devices_and_meeting(
        app.clone(),
        args.mic_device_name,
        args.system_device_name,
        args.meeting_name.clone(),
    )
    .await
    {
        Ok(_) => {
            RECORDING_FLAG.store(true, Ordering::SeqCst);
            log_info!("Recording started successfully");
            Ok(())
        }
        Err(e) => {
            log_error!("Failed to start audio recording: {}", e);
            Err(format!("Failed to start recording: {}", e))
        }
    }
}

#[tauri::command]
async fn stop_recording<R: Runtime>(app: AppHandle<R>, args: RecordingArgs) -> Result<(), String> {
    log_info!("Attempting to stop recording...");

    if !audio::recording::lifecycle::is_recording_async().await {
        log_info!("Recording is already stopped");
        return Ok(());
    }

    match audio::recording::lifecycle::stop_recording(
        app.clone(),
        audio::recording::types::RecordingArgs {
            save_path: args.save_path.clone(),
        },
    )
    .await
    {
        Ok(_) => {
            RECORDING_FLAG.store(false, Ordering::SeqCst);
            // Note: Recording is saved to the default recordings folder by the backend
            // (typically ~/Movies/meetlocal-recordings on macOS)
            Ok(())
        }
        Err(e) => {
            log_error!("Failed to stop audio recording: {}", e);
            RECORDING_FLAG.store(false, Ordering::SeqCst);
            Err(format!("Failed to stop recording: {}", e))
        }
    }
}

#[tauri::command]
async fn is_recording() -> bool {
    audio::recording::lifecycle::is_recording_async().await
}

#[tauri::command]
fn get_transcription_status() -> TranscriptionStatus {
    TranscriptionStatus {
        chunks_in_queue: 0,
        is_processing: false,
        last_activity_ms: 0,
    }
}

// ============== Live Diarization Commands ==============

#[tauri::command]
fn set_live_diarization_enabled(enabled: bool) {
    audio::transcription::set_live_diarization_enabled(enabled);
}

#[tauri::command]
fn get_live_diarization_enabled() -> bool {
    audio::transcription::is_live_diarization_enabled()
}

#[tauri::command]
fn read_audio_file(file_path: String) -> Result<Vec<u8>, String> {
    std::fs::read(&file_path).map_err(|e| format!("Failed to read audio file: {}", e))
}

#[tauri::command]
async fn save_transcript(file_path: String, content: String) -> Result<(), String> {
    log_info!("Saving transcript to: {}", file_path);

    if let Some(parent) = std::path::Path::new(&file_path).parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }
    }

    std::fs::write(&file_path, content)
        .map_err(|e| format!("Failed to write transcript: {}", e))?;

    log_info!("Transcript saved successfully");
    Ok(())
}

// ============== Device Commands ==============

#[tauri::command]
async fn get_audio_devices() -> Result<Vec<AudioDevice>, String> {
    list_audio_devices()
        .await
        .map_err(|e| format!("Failed to list audio devices: {}", e))
}

#[tauri::command]
async fn start_recording_with_devices<R: Runtime>(
    app: AppHandle<R>,
    mic_device_name: Option<String>,
    system_device_name: Option<String>,
) -> Result<(), String> {
    start_recording(app, StartRecordingArgs {
        mic_device_name,
        system_device_name,
        meeting_name: None,
    }).await
}

// ============== Audio Level Monitoring ==============

#[tauri::command]
async fn start_audio_level_monitoring<R: Runtime>(
    app: AppHandle<R>,
    device_names: Vec<String>,
) -> Result<(), String> {
    log_info!("Starting audio level monitoring for devices: {:?}", device_names);
    audio::simple_level_monitor::start_monitoring(app, device_names)
        .await
        .map_err(|e| format!("Failed to start audio level monitoring: {}", e))
}

#[tauri::command]
async fn stop_audio_level_monitoring() -> Result<(), String> {
    log_info!("Stopping audio level monitoring");
    audio::simple_level_monitor::stop_monitoring()
        .await
        .map_err(|e| format!("Failed to stop audio level monitoring: {}", e))
}

#[tauri::command]
async fn is_audio_level_monitoring() -> bool {
    audio::simple_level_monitor::is_monitoring()
}

// ============== Main App Entry ==============

pub fn run() {
    // Initialize env_logger to output to stderr (reads RUST_LOG env var)
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(whisper_engine::parallel_commands::ParallelProcessorState::new())
        .manage(audio::init_system_audio_state())
        .manage(state::AppState::new())
        .setup(|app| {
            log::info!("Meeting-Local application setup starting...");

            // Initialize database
            let db = match database::DatabaseManager::init_with_app_handle(&app.handle()) {
                Ok(db) => {
                    log::info!("Database initialized successfully");
                    db
                }
                Err(e) => {
                    log::error!("Failed to initialize database: {}", e);
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Database initialization failed: {}", e),
                    )));
                }
            };

            // Load settings from database and apply to audio processing flags
            if let Ok(settings) = db.load_all_settings() {
                log::info!("Applying settings from database...");

                // Apply audio processing settings
                audio::ffmpeg_mixer::set_mic_rnnoise_enabled(settings.mic_rnnoise);
                audio::ffmpeg_mixer::set_mic_highpass_enabled(settings.mic_highpass);
                audio::ffmpeg_mixer::set_mic_normalizer_enabled(settings.mic_normalizer);
                audio::ffmpeg_mixer::set_sys_rnnoise_enabled(settings.sys_rnnoise);
                audio::ffmpeg_mixer::set_sys_highpass_enabled(settings.sys_highpass);
                audio::ffmpeg_mixer::set_sys_normalizer_enabled(settings.sys_normalizer);

                // Apply language preference
                if let Some(lang) = settings.language {
                    if let Ok(mut guard) = LANGUAGE_PREFERENCE.lock() {
                        *guard = Some(lang);
                    }
                }

                log::info!("Settings applied successfully");
            }

            // Seed templates from JSON files
            let templates_dir = app.path().resource_dir()
                .map(|p| p.join("templates"))
                .unwrap_or_else(|_| std::path::PathBuf::from("templates"));

            // Also check relative to executable for development
            let dev_templates_dir = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .map(|p| p.join("../templates"))
                .unwrap_or_else(|| std::path::PathBuf::from("templates"));

            // Try resource dir first, then dev dir
            let templates_path = if templates_dir.exists() {
                templates_dir
            } else if dev_templates_dir.exists() {
                dev_templates_dir
            } else {
                // Try from src-tauri directory (development mode)
                std::path::PathBuf::from("templates")
            };

            log::info!("Looking for templates in: {:?}", templates_path);
            match db.seed_templates_from_folder(&templates_path) {
                Ok(count) => {
                    if count > 0 {
                        log::info!("Seeded {} templates from {:?}", count, templates_path);
                    }
                }
                Err(e) => {
                    log::warn!("Failed to seed templates: {}", e);
                }
            }

            // Store database in app state
            let app_state: tauri::State<state::AppState> = app.state();
            let db_clone = db;
            tauri::async_runtime::block_on(async {
                app_state.init_database(db_clone).await;
            });

            // Set models directory
            whisper_engine::commands::set_models_directory(&app.handle());

            // Initialize Whisper engine on startup
            tauri::async_runtime::spawn(async {
                if let Err(e) = whisper_engine::commands::whisper_init().await {
                    log::error!("Failed to initialize Whisper engine: {}", e);
                }
            });

            log::info!("Meeting-Local application setup complete");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Recording commands
            start_recording,
            stop_recording,
            is_recording,
            get_transcription_status,
            read_audio_file,
            save_transcript,
            // Device commands
            get_audio_devices,
            start_recording_with_devices,
            // Audio level monitoring
            start_audio_level_monitoring,
            stop_audio_level_monitoring,
            is_audio_level_monitoring,
            // Recording control - pause/resume
            audio::recording::pause_resume::pause_recording,
            audio::recording::pause_resume::resume_recording,
            audio::recording::pause_resume::is_recording_paused,
            audio::recording::pause_resume::get_recording_state,
            audio::recording::pause_resume::get_meeting_folder_path,
            audio::recording::pause_resume::get_transcript_history,
            audio::recording::pause_resume::get_recording_meeting_name,
            // Recording control - device events
            audio::recording::device_events::poll_audio_device_events,
            audio::recording::device_events::get_reconnection_status,
            audio::recording::device_events::attempt_device_reconnect,
            audio::recording::device_events::get_active_audio_output,
            // Recording preferences
            audio::recording_preferences::get_recording_preferences,
            audio::recording_preferences::set_recording_preferences,
            audio::recording_preferences::get_default_recordings_folder_path,
            audio::recording_preferences::open_recordings_folder,
            audio::recording_preferences::open_folder,
            audio::recording_preferences::select_recording_folder,
            // Retranscription commands
            audio::retranscription::retranscribe_recording,
            audio::retranscription::cancel_retranscription,
            audio::retranscription::get_retranscription_status,
            audio::recording_preferences::get_available_audio_backends,
            audio::recording_preferences::get_current_audio_backend,
            audio::recording_preferences::set_audio_backend,
            audio::recording_preferences::get_audio_backend_info,
            // Whisper commands
            whisper_engine::commands::whisper_init,
            whisper_engine::commands::whisper_get_available_models,
            whisper_engine::commands::whisper_load_model,
            whisper_engine::commands::whisper_get_current_model,
            whisper_engine::commands::whisper_is_model_loaded,
            whisper_engine::commands::whisper_has_available_models,
            whisper_engine::commands::whisper_validate_model_ready,
            whisper_engine::commands::whisper_transcribe_audio,
            whisper_engine::commands::whisper_get_models_directory,
            whisper_engine::commands::whisper_download_model,
            whisper_engine::commands::whisper_cancel_download,
            whisper_engine::commands::whisper_delete_model,
            whisper_engine::commands::open_models_folder,
            // Parallel processing
            whisper_engine::parallel_commands::initialize_parallel_processor,
            whisper_engine::parallel_commands::start_parallel_processing,
            whisper_engine::parallel_commands::pause_parallel_processing,
            whisper_engine::parallel_commands::resume_parallel_processing,
            whisper_engine::parallel_commands::stop_parallel_processing,
            whisper_engine::parallel_commands::get_parallel_processing_status,
            whisper_engine::parallel_commands::get_system_resources,
            whisper_engine::parallel_commands::check_resource_constraints,
            whisper_engine::parallel_commands::calculate_optimal_workers,
            whisper_engine::parallel_commands::prepare_audio_chunks,
            whisper_engine::parallel_commands::test_parallel_processing_setup,
            // System audio
            audio::system_audio_commands::start_system_audio_capture_command,
            audio::system_audio_commands::list_system_audio_devices_command,
            audio::system_audio_commands::check_system_audio_permissions_command,
            audio::system_audio_commands::start_system_audio_monitoring,
            audio::system_audio_commands::stop_system_audio_monitoring,
            audio::system_audio_commands::get_system_audio_monitoring_status,
            // Permissions
            audio::permissions::check_screen_recording_permission_command,
            audio::permissions::request_screen_recording_permission_command,
            // Language preference
            get_language_preference,
            set_language_preference,
            // Hardware recommendations
            get_hardware_recommendations,
            // Audio processing controls (per-source)
            get_mic_rnnoise_enabled,
            set_mic_rnnoise_enabled,
            get_mic_highpass_enabled,
            set_mic_highpass_enabled,
            get_mic_normalizer_enabled,
            set_mic_normalizer_enabled,
            get_sys_rnnoise_enabled,
            set_sys_rnnoise_enabled,
            get_sys_highpass_enabled,
            set_sys_highpass_enabled,
            get_sys_normalizer_enabled,
            set_sys_normalizer_enabled,
            // Legacy noise suppression (backward compat)
            get_noise_suppression_enabled,
            set_noise_suppression_enabled,
            // Database commands - Settings
            db_get_setting,
            db_set_setting,
            db_get_all_settings,
            db_load_settings_on_startup,
            // Database commands - Recordings
            db_create_recording,
            db_get_recording,
            db_get_all_recordings,
            db_get_recent_recordings,
            db_update_recording,
            db_delete_recording,
            db_complete_recording,
            // Database commands - Transcripts
            db_save_transcript_segment,
            db_save_transcript_segments_batch,
            db_get_transcript_segments,
            db_replace_transcripts,
            db_update_speaker_label,
            db_update_transcript_text,
            // Database commands - Categories
            db_get_all_categories,
            db_create_category,
            db_assign_category,
            db_remove_category,
            db_delete_category,
            // Database commands - Tags
            db_get_all_tags,
            db_create_tag,
            db_assign_tag,
            db_remove_tag,
            db_delete_tag,
            db_get_or_create_tag,
            // Database commands - Search
            db_search_recordings,
            // Diarization commands
            diarization::engine::init_diarization,
            diarization::engine::diarize_audio,
            diarization::engine::register_speaker_voice,
            diarization::engine::get_registered_speakers,
            diarization::engine::delete_registered_speaker,
            diarization::engine::rename_speaker,
            // Diarization model management
            diarization::model_manager::download_diarization_models,
            diarization::model_manager::check_diarization_models,
            diarization::model_manager::are_diarization_models_ready,
            // Live diarization control
            set_live_diarization_enabled,
            get_live_diarization_enabled,
            // Sortformer diarization
            diarization::sortformer_provider::init_sortformer,
            diarization::sortformer_provider::is_sortformer_model_available,
            diarization::sortformer_provider::download_sortformer_model,
            diarization::sortformer_provider::sortformer_diarize,
            diarization::sortformer_provider::sortformer_reset,
            diarization::sortformer_provider::get_sortformer_model_info,
            // LLM commands - Provider management
            llm_engine::commands::llm_get_providers,
            llm_engine::commands::llm_get_active_provider,
            llm_engine::commands::llm_set_active_provider,
            // LLM commands - Model management
            llm_engine::commands::llm_list_models,
            llm_engine::commands::llm_list_models_for_provider,
            llm_engine::commands::llm_initialize,
            llm_engine::commands::llm_current_model,
            llm_engine::commands::llm_is_ready,
            // LLM commands - Ollama specific
            llm_engine::commands::llm_ollama_check_connection,
            // LLM commands - Completion
            llm_engine::commands::llm_complete,
            llm_engine::commands::llm_complete_streaming,
            // LLM commands - Model downloads (for embedded)
            llm_engine::commands::llm_get_downloadable_models,
            llm_engine::commands::llm_get_local_models,
            llm_engine::commands::llm_is_model_downloaded,
            llm_engine::commands::llm_delete_model,
            llm_engine::commands::llm_download_model,
            llm_engine::commands::llm_cancel_download,
            llm_engine::commands::llm_download_custom_model,
            llm_engine::commands::llm_get_local_models_info,
            // LLM default model commands
            llm_engine::commands::llm_get_default_model,
            llm_engine::commands::llm_set_default_model,
            llm_engine::commands::llm_clear_default_model,
            // LLM model tool support commands
            llm_engine::commands::llm_get_model_tool_support,
            llm_engine::commands::llm_set_model_tool_support,
            llm_engine::commands::llm_delete_model_tool_support,
            llm_engine::commands::llm_get_all_model_configs,
            llm_engine::commands::llm_get_effective_tool_support,
            // Chat session commands
            chat::session_commands::chat_create_session,
            chat::session_commands::chat_list_sessions,
            chat::session_commands::chat_get_session,
            chat::session_commands::chat_get_or_create_session,
            chat::session_commands::chat_update_session_config,
            chat::session_commands::chat_update_session_title,
            chat::session_commands::chat_delete_session,
            chat::session_commands::chat_get_config,
            // Chat message commands
            chat::message_commands::chat_send_message,
            chat::message_commands::chat_get_messages,
            chat::message_commands::chat_get_status,
            chat::message_commands::chat_cancel_message,
            chat::message_commands::chat_clear_session,
            chat::message_commands::chat_delete_history,
            chat::message_commands::chat_is_processing,
            chat::message_commands::chat_get_pending_messages,
            // Template commands
            templates::commands::template_list,
            templates::commands::template_get,
            templates::commands::template_create,
            templates::commands::template_update,
            templates::commands::template_delete,
            templates::commands::template_duplicate,
            // Tools commands
            tools::commands::tools_list,
            tools::commands::tools_list_enabled,
            tools::commands::tools_list_defaults,
            tools::commands::tools_get,
            tools::commands::tools_create,
            tools::commands::tools_update,
            tools::commands::tools_delete,
            tools::commands::tools_set_default,
            tools::commands::tools_get_for_session,
            tools::commands::tools_set_for_session,
            tools::commands::tools_init_for_session,
            // MCP commands
            mcp::commands::mcp_list_servers,
            mcp::commands::mcp_list_servers_with_tools,
            mcp::commands::mcp_get_server,
            mcp::commands::mcp_create_server,
            mcp::commands::mcp_import_config,
            mcp::commands::mcp_update_server,
            mcp::commands::mcp_delete_server,
            mcp::commands::mcp_start_server,
            mcp::commands::mcp_stop_server,
            mcp::commands::mcp_restart_server,
            mcp::commands::mcp_get_server_status,
            mcp::commands::mcp_is_server_running,
            mcp::commands::mcp_refresh_tools,
            mcp::commands::mcp_get_server_tools,
            mcp::commands::mcp_get_running_servers,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
