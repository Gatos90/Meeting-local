//! Parakeet engine stub module
//!
//! Provides stub implementations for Parakeet functionality.

use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;
use tauri::{AppHandle, Runtime};

#[derive(Clone)]
pub struct ParakeetEngine;

impl ParakeetEngine {
    pub async fn transcribe(&self, _samples: &[f32], _sample_rate: u32) -> Result<String, String> {
        Err("Parakeet engine not available".to_string())
    }

    pub async fn transcribe_audio(&self, _audio: Vec<f32>) -> Result<String, String> {
        Err("Parakeet engine not available".to_string())
    }

    pub async fn is_model_loaded(&self) -> bool {
        false
    }

    pub async fn get_current_model(&self) -> Option<String> {
        None
    }

    pub async fn unload_model(&self) -> bool {
        // No-op: parakeet not available
        false
    }
}

/// Global Parakeet engine instance
pub static PARAKEET_ENGINE: Lazy<Arc<Mutex<Option<Arc<ParakeetEngine>>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

/// Initialize Parakeet - no-op stub
pub async fn parakeet_init() -> Result<(), String> {
    // No-op: parakeet not available in simplified version
    Ok(())
}

/// Validate Parakeet model ready - returns error stub
pub async fn parakeet_validate_model_ready_with_config<R: Runtime>(
    _app: &AppHandle<R>,
) -> Result<String, String> {
    // No-op: parakeet not available
    Err("Parakeet not available".to_string())
}
