//! Performance logging macros
//!
//! These macros are used to log performance-related information in debug builds
//! but compile to no-ops in release builds to avoid overhead.

/// Performance debug logging - only active in debug builds
#[cfg(debug_assertions)]
#[macro_export]
macro_rules! perf_debug {
    ($($arg:tt)*) => { log::debug!($($arg)*) };
}

/// Performance debug logging - no-op in release builds
#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! perf_debug {
    ($($arg:tt)*) => {};
}

/// Performance trace logging - only active in debug builds
#[cfg(debug_assertions)]
#[macro_export]
macro_rules! perf_trace {
    ($($arg:tt)*) => { log::trace!($($arg)*) };
}

/// Performance trace logging - no-op in release builds
#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! perf_trace {
    ($($arg:tt)*) => {};
}

// Note: batch_audio_metric! macro is defined in audio/batch_processor.rs
