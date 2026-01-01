//! Pause/resume and state query commands for recording

use log::info;
use tauri::{AppHandle, Emitter, Runtime};

use super::state::{is_recording, with_recording_manager, RECORDING_MANAGER};

/// Pause the current recording
#[tauri::command]
pub async fn pause_recording<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    info!("Pausing recording");

    // Check if currently recording
    if !is_recording() {
        return Err("No recording is currently active".to_string());
    }

    // Access the recording manager and pause it
    let manager_guard = RECORDING_MANAGER.lock().unwrap();
    if let Some(manager) = manager_guard.as_ref() {
        manager.pause_recording().map_err(|e| e.to_string())?;

        // Emit pause event to frontend
        app.emit(
            "recording-paused",
            serde_json::json!({
                "message": "Recording paused"
            }),
        )
        .map_err(|e| e.to_string())?;

        // Update tray menu to reflect paused state
        crate::tray::update_tray_menu(&app);

        info!("Recording paused successfully");
        Ok(())
    } else {
        Err("No recording manager found".to_string())
    }
}

/// Resume the current recording
#[tauri::command]
pub async fn resume_recording<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    info!("Resuming recording");

    // Check if currently recording
    if !is_recording() {
        return Err("No recording is currently active".to_string());
    }

    // Access the recording manager and resume it
    let manager_guard = RECORDING_MANAGER.lock().unwrap();
    if let Some(manager) = manager_guard.as_ref() {
        manager.resume_recording().map_err(|e| e.to_string())?;

        // Emit resume event to frontend
        app.emit(
            "recording-resumed",
            serde_json::json!({
                "message": "Recording resumed"
            }),
        )
        .map_err(|e| e.to_string())?;

        // Update tray menu to reflect resumed state
        crate::tray::update_tray_menu(&app);

        info!("Recording resumed successfully");
        Ok(())
    } else {
        Err("No recording manager found".to_string())
    }
}

/// Check if recording is currently paused
#[tauri::command]
pub async fn is_recording_paused() -> bool {
    with_recording_manager(|manager| {
        manager.map(|m| m.is_paused()).unwrap_or(false)
    })
}

/// Get detailed recording state
#[tauri::command]
pub async fn get_recording_state() -> serde_json::Value {
    let is_recording_now = is_recording();

    with_recording_manager(|manager| {
        if let Some(manager) = manager {
            serde_json::json!({
                "is_recording": is_recording_now,
                "is_paused": manager.is_paused(),
                "is_active": manager.is_active(),
                "recording_duration": manager.get_recording_duration(),
                "active_duration": manager.get_active_recording_duration(),
                "total_pause_duration": manager.get_total_pause_duration(),
                "current_pause_duration": manager.get_current_pause_duration()
            })
        } else {
            serde_json::json!({
                "is_recording": is_recording_now,
                "is_paused": false,
                "is_active": false,
                "recording_duration": null,
                "active_duration": null,
                "total_pause_duration": 0.0,
                "current_pause_duration": null
            })
        }
    })
}

/// Get the meeting folder path for the current recording
/// Returns the path if a meeting name was set and folder structure initialized
#[tauri::command]
pub async fn get_meeting_folder_path() -> Result<Option<String>, String> {
    Ok(with_recording_manager(|manager| {
        manager.and_then(|m| m.get_meeting_folder().map(|p| p.to_string_lossy().to_string()))
    }))
}

/// Get accumulated transcript segments from current recording session
/// Used for syncing frontend state after page reload during active recording
#[tauri::command]
pub async fn get_transcript_history() -> Result<Vec<crate::audio::recording_saver::TranscriptSegment>, String> {
    Ok(with_recording_manager(|manager| {
        manager.map(|m| m.get_transcript_segments()).unwrap_or_default()
    }))
}

/// Get meeting name from current recording session
/// Used for syncing frontend state after page reload during active recording
#[tauri::command]
pub async fn get_recording_meeting_name() -> Result<Option<String>, String> {
    Ok(with_recording_manager(|manager| {
        manager.and_then(|m| m.get_meeting_name())
    }))
}
