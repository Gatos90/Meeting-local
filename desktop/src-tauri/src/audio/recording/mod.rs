//! Recording commands module
//!
//! This module provides:
//! - Recording lifecycle management (start/stop)
//! - Pause/resume functionality
//! - Device monitoring and reconnection
//! - Global recording state management

pub mod types;
pub mod state;
pub mod lifecycle;
pub mod pause_resume;
pub mod device_events;

// Re-export types
pub use types::{
    RecordingArgs, TranscriptionStatus,
    DeviceEventResponse, ReconnectionStatus, DisconnectedDeviceInfo,
};

// Re-export lifecycle functions
pub use lifecycle::{
    start_recording, start_recording_with_meeting_name,
    start_recording_with_devices, start_recording_with_devices_and_meeting,
    stop_recording,
    is_recording_async as is_recording,
    get_transcription_status,
    TranscriptUpdate,
};

// Re-export pause/resume commands
pub use pause_resume::{
    pause_recording, resume_recording, is_recording_paused,
    get_recording_state, get_meeting_folder_path,
    get_transcript_history, get_recording_meeting_name,
};

// Re-export device event commands
pub use device_events::{
    poll_audio_device_events, get_reconnection_status,
    get_active_audio_output, attempt_device_reconnect,
};
