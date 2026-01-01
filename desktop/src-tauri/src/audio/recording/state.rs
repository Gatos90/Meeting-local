//! Global recording state management

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tokio::task::JoinHandle;

use super::super::RecordingManager;

// Simple recording state tracking
pub static IS_RECORDING: AtomicBool = AtomicBool::new(false);

// Global recording manager and transcription task to keep them alive during recording
pub static RECORDING_MANAGER: Mutex<Option<RecordingManager>> = Mutex::new(None);
pub static TRANSCRIPTION_TASK: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);

/// Check if recording is currently active
pub fn is_recording() -> bool {
    IS_RECORDING.load(Ordering::SeqCst)
}

/// Set recording state
pub fn set_recording(value: bool) {
    IS_RECORDING.store(value, Ordering::SeqCst);
}

/// Get a reference to the recording manager (takes the lock)
pub fn with_recording_manager<T, F: FnOnce(Option<&RecordingManager>) -> T>(f: F) -> T {
    let guard = RECORDING_MANAGER.lock().unwrap();
    f(guard.as_ref())
}

/// Get a mutable reference to the recording manager (takes the lock)
pub fn with_recording_manager_mut<T, F: FnOnce(Option<&mut RecordingManager>) -> T>(f: F) -> T {
    let mut guard = RECORDING_MANAGER.lock().unwrap();
    f(guard.as_mut())
}

/// Store a recording manager
pub fn set_recording_manager(manager: Option<RecordingManager>) {
    let mut guard = RECORDING_MANAGER.lock().unwrap();
    *guard = manager;
}

/// Take the recording manager (removes it from global state)
pub fn take_recording_manager() -> Option<RecordingManager> {
    let mut guard = RECORDING_MANAGER.lock().unwrap();
    guard.take()
}

/// Store the transcription task handle
pub fn set_transcription_task(task: Option<JoinHandle<()>>) {
    let mut guard = TRANSCRIPTION_TASK.lock().unwrap();
    *guard = task;
}

/// Take the transcription task handle
pub fn take_transcription_task() -> Option<JoinHandle<()>> {
    let mut guard = TRANSCRIPTION_TASK.lock().unwrap();
    guard.take()
}
