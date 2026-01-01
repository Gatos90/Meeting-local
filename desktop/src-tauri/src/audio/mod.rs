// src/audio/mod.rs
pub mod audio_processing;
pub mod encode;
pub mod ffmpeg;
pub mod vad;

// Modularized audio processing (split from audio_processing.rs)
pub mod processing;
pub mod file_io;

// Modularized device management
pub mod devices;
pub mod capture;
pub mod permissions;

// NEW: Device detection and diagnostics for adaptive buffering
pub mod device_detection;
pub mod diagnostics;
pub mod ffmpeg_mixer;  // NEW: FFmpeg-style adaptive audio mixer

// New simplified audio system
pub mod recording_state;
pub mod pipeline;
pub mod stream;
pub mod recording_manager;
pub mod recording;
pub mod recording_preferences;
pub mod recording_saver;
pub mod incremental_saver;  // NEW: Incremental audio saving with checkpoints
pub mod level_monitor;
pub mod simple_level_monitor;
pub mod buffer_pool;
pub mod post_processor;
pub mod hardware_detector;
pub mod model_recommendations;
pub mod async_logger;
pub mod batch_processor;
pub mod system_detector;
pub mod system_audio_commands;
pub mod device_monitor;  // NEW: Device disconnect/reconnect monitoring
pub mod playback_monitor; // NEW: Playback device detection for BT warnings
pub mod retranscription;  // NEW: Batch retranscription of audio files

// Transcription module (provider abstraction, engine management, worker pool)
pub mod transcription;

pub use devices::{
    default_input_device, default_output_device, get_device_and_config, list_audio_devices,
    parse_audio_device, trigger_audio_permission,
    AudioDevice, AudioTranscriptionEngine, DeviceControl, DeviceType,
    LAST_AUDIO_CAPTURE,
};

// Export system audio capture functionality
pub use capture::{
    SystemAudioCapture, SystemAudioStream,
    start_system_audio_capture, list_system_audio_devices,
    check_system_audio_permissions
};

// Export system audio detection functionality
pub use system_detector::{
    SystemAudioDetector, SystemAudioEvent, SystemAudioCallback,
    new_system_audio_callback
};

// Export system audio commands
pub use system_audio_commands::{
    start_system_audio_capture_command, list_system_audio_devices_command,
    check_system_audio_permissions_command, start_system_audio_monitoring,
    stop_system_audio_monitoring, get_system_audio_monitoring_status,
    init_system_audio_state
};

// Export new simplified components
pub use recording_state::{RecordingState, AudioChunk, ProcessedAudioChunk, AudioError, DeviceType as RecordingDeviceType};
pub use pipeline::{AudioPipelineManager, AudioCapture};
pub use stream::{AudioStreamManager};
pub use recording_manager::{RecordingManager};
// Re-export from new recording module
pub use recording::{
    start_recording, start_recording_with_devices, stop_recording,
    is_recording, get_transcription_status, RecordingArgs, TranscriptionStatus, TranscriptUpdate,
    // Pause/resume
    pause_recording, resume_recording, is_recording_paused, get_recording_state,
    get_meeting_folder_path, get_transcript_history, get_recording_meeting_name,
    // Device events
    poll_audio_device_events, get_reconnection_status, get_active_audio_output, attempt_device_reconnect,
    DeviceEventResponse, ReconnectionStatus, DisconnectedDeviceInfo,
};
pub use recording_preferences::{
    RecordingPreferences, get_default_recordings_folder
};
pub use recording_saver::RecordingSaver;
pub use level_monitor::{AudioLevelMonitor, AudioLevelData, AudioLevelUpdate};
pub use buffer_pool::{AudioBufferPool, PooledBuffer};
pub use post_processor::{PostProcessor, PostProcessRequest, PostProcessResponse};
pub use hardware_detector::{HardwareProfile, AdaptiveWhisperConfig, PerformanceTier, GpuType};
pub use model_recommendations::{HardwareRecommendations, ModelRecommendation, RecommendationLevel, HardwareProfileInfo};
pub use encode::{
    encode_single_audio, AudioInput
};
pub use device_monitor::{AudioDeviceMonitor, DeviceEvent, DeviceMonitorType};

// Export device detection and diagnostics
pub use device_detection::{InputDeviceKind, calculate_buffer_timeout};
pub use diagnostics::{
    log_device_capabilities, log_detection_summary, log_buffer_health,
    log_mixer_status, log_performance_summary
};

// Export FFmpeg mixer
pub use ffmpeg_mixer::{
    FFmpegAudioMixer, BufferStats,
    // Legacy (backward compat)
    is_rnnoise_enabled, set_rnnoise_enabled,
    // Microphone processing controls
    is_mic_rnnoise_enabled, set_mic_rnnoise_enabled,
    is_mic_highpass_enabled, set_mic_highpass_enabled,
    is_mic_normalizer_enabled, set_mic_normalizer_enabled,
    // System audio processing controls
    is_sys_rnnoise_enabled, set_sys_rnnoise_enabled,
    is_sys_highpass_enabled, set_sys_highpass_enabled,
    is_sys_normalizer_enabled, set_sys_normalizer_enabled,
};

pub use vad::{extract_speech_16k};

