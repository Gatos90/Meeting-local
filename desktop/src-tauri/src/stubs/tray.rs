//! Tray stub - no-op implementation

use tauri::{AppHandle, Runtime};

/// Update tray menu - no-op in simplified version
pub fn update_tray_menu<R: Runtime>(_app: &AppHandle<R>) {
    // No-op: tray functionality removed
}
