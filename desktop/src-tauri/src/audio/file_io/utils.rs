// File I/O - Utilities
use anyhow::Result;
use chrono::Utc;
use std::path::PathBuf;

/// Sanitize a filename to be safe for filesystem use
pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Create a meeting folder with timestamp and return the path
pub fn create_meeting_folder(
    base_path: &PathBuf,
    meeting_name: &str,
) -> Result<PathBuf> {
    let timestamp = Utc::now().format("%Y-%m-%d_%H-%M").to_string();
    let sanitized_name = sanitize_filename(meeting_name);
    let folder_name = format!("{}_{}", sanitized_name, timestamp);
    let meeting_folder = base_path.join(folder_name);

    std::fs::create_dir_all(&meeting_folder)?;

    let checkpoints_dir = meeting_folder.join(".checkpoints");
    std::fs::create_dir_all(&checkpoints_dir)?;

    log::info!("Created meeting folder: {}", meeting_folder.display());

    Ok(meeting_folder)
}
