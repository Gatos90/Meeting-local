//! Global state for recording flag and language preference

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use once_cell::sync::Lazy;

/// Flag indicating whether recording is active
pub static RECORDING_FLAG: AtomicBool = AtomicBool::new(false);

/// Language preference storage
pub static LANGUAGE_PREFERENCE: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

/// Get the current language preference
pub fn get_language_preference_internal() -> Option<String> {
    let guard = LANGUAGE_PREFERENCE.lock().ok()?;
    guard.clone()
}

/// Set the language preference
pub fn set_language_preference_internal(language: &str) -> Result<(), String> {
    let mut guard = LANGUAGE_PREFERENCE.lock().map_err(|e| format!("Lock error: {}", e))?;
    *guard = if language == "auto" { None } else { Some(language.to_string()) };
    Ok(())
}

/// Check if recording is active
pub fn is_recording_flag() -> bool {
    RECORDING_FLAG.load(Ordering::SeqCst)
}

/// Set the recording flag
pub fn set_recording_flag(value: bool) {
    RECORDING_FLAG.store(value, Ordering::SeqCst);
}
