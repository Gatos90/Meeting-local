// File I/O Module
//
// Split into focused files:
// - utils.rs: Filename sanitization, folder creation
// - audio_writer.rs: Audio file writing
// - transcript_writer.rs: Transcript file writing

pub mod utils;
pub mod audio_writer;
pub mod transcript_writer;

// Re-export for backwards compatibility
pub use utils::{sanitize_filename, create_meeting_folder};
pub use audio_writer::{write_audio_to_file, write_audio_to_file_with_meeting_name};
pub use transcript_writer::{write_transcript_to_file, write_transcript_json_to_file};
