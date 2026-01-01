// Recordings repository for Meeting-Local
// Handles CRUD operations for recordings/meetings

use anyhow::{Context, Result};
use rusqlite::{Connection, params};

use super::models::{Recording, RecordingUpdate, RecordingWithMetadata, Category, Tag};
use super::DatabaseManager;

impl DatabaseManager {
    /// Create a new recording
    pub fn create_recording(&self, recording: &Recording) -> Result<String> {
        self.with_connection(|conn| {
            create_recording_impl(conn, recording)
        })
    }

    /// Get a recording by ID
    pub fn get_recording(&self, id: &str) -> Result<Option<Recording>> {
        self.with_connection(|conn| {
            get_recording_impl(conn, id)
        })
    }

    /// Get a recording with its categories, tags, and transcript count
    pub fn get_recording_with_metadata(&self, id: &str) -> Result<Option<RecordingWithMetadata>> {
        self.with_connection(|conn| {
            get_recording_with_metadata_impl(conn, id)
        })
    }

    /// Get all recordings (most recent first)
    pub fn get_all_recordings(&self) -> Result<Vec<RecordingWithMetadata>> {
        self.with_connection(|conn| {
            get_all_recordings_impl(conn, None)
        })
    }

    /// Get recent recordings with a limit
    pub fn get_recent_recordings(&self, limit: i32) -> Result<Vec<RecordingWithMetadata>> {
        self.with_connection(|conn| {
            get_all_recordings_impl(conn, Some(limit))
        })
    }

    /// Update a recording
    pub fn update_recording(&self, id: &str, updates: &RecordingUpdate) -> Result<()> {
        self.with_connection(|conn| {
            update_recording_impl(conn, id, updates)
        })
    }

    /// Delete a recording
    pub fn delete_recording(&self, id: &str) -> Result<()> {
        self.with_connection(|conn| {
            delete_recording_impl(conn, id)
        })
    }

    /// Mark a recording as completed
    pub fn complete_recording(&self, id: &str, duration_seconds: f64) -> Result<()> {
        self.with_connection(|conn| {
            complete_recording_impl(conn, id, duration_seconds)
        })
    }
}

fn create_recording_impl(conn: &Connection, recording: &Recording) -> Result<String> {
    conn.execute(
        r#"
        INSERT INTO recordings (
            id, title, created_at, completed_at, duration_seconds, status,
            audio_file_path, meeting_folder_path, microphone_device, system_audio_device,
            sample_rate, transcription_model, language
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
        "#,
        params![
            recording.id,
            recording.title,
            recording.created_at,
            recording.completed_at,
            recording.duration_seconds,
            recording.status,
            recording.audio_file_path,
            recording.meeting_folder_path,
            recording.microphone_device,
            recording.system_audio_device,
            recording.sample_rate,
            recording.transcription_model,
            recording.language,
        ],
    ).context("Failed to create recording")?;

    Ok(recording.id.clone())
}

fn get_recording_impl(conn: &Connection, id: &str) -> Result<Option<Recording>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, title, created_at, completed_at, duration_seconds, status,
               audio_file_path, meeting_folder_path, microphone_device, system_audio_device,
               sample_rate, transcription_model, language, diarization_provider
        FROM recordings WHERE id = ?
        "#
    ).context("Failed to prepare get_recording query")?;

    let result = stmt.query_row(params![id], |row| {
        Ok(Recording {
            id: row.get(0)?,
            title: row.get(1)?,
            created_at: row.get(2)?,
            completed_at: row.get(3)?,
            duration_seconds: row.get(4)?,
            status: row.get(5)?,
            audio_file_path: row.get(6)?,
            meeting_folder_path: row.get(7)?,
            microphone_device: row.get(8)?,
            system_audio_device: row.get(9)?,
            sample_rate: row.get(10)?,
            transcription_model: row.get(11)?,
            language: row.get(12)?,
            diarization_provider: row.get(13)?,
        })
    });

    match result {
        Ok(recording) => Ok(Some(recording)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("Failed to get recording"),
    }
}

fn get_recording_with_metadata_impl(conn: &Connection, id: &str) -> Result<Option<RecordingWithMetadata>> {
    let recording = match get_recording_impl(conn, id)? {
        Some(r) => r,
        None => return Ok(None),
    };

    let categories = get_recording_categories(conn, id)?;
    let tags = get_recording_tags(conn, id)?;
    let transcript_count = get_transcript_count(conn, id)?;

    Ok(Some(RecordingWithMetadata {
        recording,
        categories,
        tags,
        transcript_count,
    }))
}

fn get_all_recordings_impl(conn: &Connection, limit: Option<i32>) -> Result<Vec<RecordingWithMetadata>> {
    let query = match limit {
        Some(l) => format!(
            r#"
            SELECT id, title, created_at, completed_at, duration_seconds, status,
                   audio_file_path, meeting_folder_path, microphone_device, system_audio_device,
                   sample_rate, transcription_model, language, diarization_provider
            FROM recordings
            ORDER BY created_at DESC
            LIMIT {}
            "#, l
        ),
        None => r#"
            SELECT id, title, created_at, completed_at, duration_seconds, status,
                   audio_file_path, meeting_folder_path, microphone_device, system_audio_device,
                   sample_rate, transcription_model, language, diarization_provider
            FROM recordings
            ORDER BY created_at DESC
            "#.to_string(),
    };

    let mut stmt = conn.prepare(&query).context("Failed to prepare get_all_recordings query")?;

    let recordings = stmt.query_map([], |row| {
        Ok(Recording {
            id: row.get(0)?,
            title: row.get(1)?,
            created_at: row.get(2)?,
            completed_at: row.get(3)?,
            duration_seconds: row.get(4)?,
            status: row.get(5)?,
            audio_file_path: row.get(6)?,
            meeting_folder_path: row.get(7)?,
            microphone_device: row.get(8)?,
            system_audio_device: row.get(9)?,
            sample_rate: row.get(10)?,
            transcription_model: row.get(11)?,
            language: row.get(12)?,
            diarization_provider: row.get(13)?,
        })
    }).context("Failed to query recordings")?;

    let mut results = Vec::new();
    for recording_result in recordings {
        let recording = recording_result.context("Failed to read recording row")?;
        let id = recording.id.clone();

        let categories = get_recording_categories(conn, &id)?;
        let tags = get_recording_tags(conn, &id)?;
        let transcript_count = get_transcript_count(conn, &id)?;

        results.push(RecordingWithMetadata {
            recording,
            categories,
            tags,
            transcript_count,
        });
    }

    Ok(results)
}

fn update_recording_impl(conn: &Connection, id: &str, updates: &RecordingUpdate) -> Result<()> {
    let mut set_clauses = Vec::new();
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(ref title) = updates.title {
        set_clauses.push("title = ?");
        params_vec.push(Box::new(title.clone()));
    }
    if let Some(ref completed_at) = updates.completed_at {
        set_clauses.push("completed_at = ?");
        params_vec.push(Box::new(completed_at.clone()));
    }
    if let Some(duration) = updates.duration_seconds {
        set_clauses.push("duration_seconds = ?");
        params_vec.push(Box::new(duration));
    }
    if let Some(ref status) = updates.status {
        set_clauses.push("status = ?");
        params_vec.push(Box::new(status.clone()));
    }
    if let Some(ref audio_file_path) = updates.audio_file_path {
        set_clauses.push("audio_file_path = ?");
        params_vec.push(Box::new(audio_file_path.clone()));
    }
    if let Some(ref meeting_folder_path) = updates.meeting_folder_path {
        set_clauses.push("meeting_folder_path = ?");
        params_vec.push(Box::new(meeting_folder_path.clone()));
    }
    if let Some(ref transcription_model) = updates.transcription_model {
        set_clauses.push("transcription_model = ?");
        params_vec.push(Box::new(transcription_model.clone()));
    }
    if let Some(ref diarization_provider) = updates.diarization_provider {
        set_clauses.push("diarization_provider = ?");
        // Empty string means "clear the field" (set to NULL)
        if diarization_provider.is_empty() {
            params_vec.push(Box::new(None::<String>));
        } else {
            params_vec.push(Box::new(diarization_provider.clone()));
        }
    }

    if set_clauses.is_empty() {
        return Ok(());
    }

    set_clauses.push("updated_at = datetime('now')");
    params_vec.push(Box::new(id.to_string()));

    let query = format!(
        "UPDATE recordings SET {} WHERE id = ?",
        set_clauses.join(", ")
    );

    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();

    conn.execute(&query, params_refs.as_slice())
        .context("Failed to update recording")?;

    Ok(())
}

fn delete_recording_impl(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM recordings WHERE id = ?", params![id])
        .context("Failed to delete recording")?;
    Ok(())
}

fn complete_recording_impl(conn: &Connection, id: &str, duration_seconds: f64) -> Result<()> {
    conn.execute(
        r#"
        UPDATE recordings
        SET completed_at = datetime('now'),
            duration_seconds = ?,
            status = 'completed',
            updated_at = datetime('now')
        WHERE id = ?
        "#,
        params![duration_seconds, id],
    ).context("Failed to complete recording")?;

    Ok(())
}

fn get_recording_categories(conn: &Connection, recording_id: &str) -> Result<Vec<Category>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT c.id, c.name, c.color, c.is_system
        FROM categories c
        JOIN recording_categories rc ON c.id = rc.category_id
        WHERE rc.recording_id = ?
        "#
    ).context("Failed to prepare get_recording_categories query")?;

    let categories = stmt.query_map(params![recording_id], |row| {
        Ok(Category {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
            is_system: row.get::<_, i32>(3)? == 1,
        })
    }).context("Failed to query recording categories")?;

    categories.collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to collect recording categories")
}

fn get_recording_tags(conn: &Connection, recording_id: &str) -> Result<Vec<Tag>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT t.id, t.name, t.color, t.usage_count
        FROM tags t
        JOIN recording_tags rt ON t.id = rt.tag_id
        WHERE rt.recording_id = ?
        "#
    ).context("Failed to prepare get_recording_tags query")?;

    let tags = stmt.query_map(params![recording_id], |row| {
        Ok(Tag {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
            usage_count: row.get(3)?,
        })
    }).context("Failed to query recording tags")?;

    tags.collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to collect recording tags")
}

fn get_transcript_count(conn: &Connection, recording_id: &str) -> Result<i32> {
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM transcript_segments WHERE recording_id = ?",
        params![recording_id],
        |row| row.get(0),
    ).context("Failed to get transcript count")?;

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_db() -> DatabaseManager {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        DatabaseManager::new(db_path).unwrap()
    }

    #[test]
    fn test_create_and_get_recording() {
        let db = create_test_db();

        let recording = Recording::new("rec_123".to_string(), "Test Meeting".to_string());
        db.create_recording(&recording).unwrap();

        let retrieved = db.get_recording("rec_123").unwrap().unwrap();
        assert_eq!(retrieved.title, "Test Meeting");
        assert_eq!(retrieved.status, "recording");
    }

    #[test]
    fn test_complete_recording() {
        let db = create_test_db();

        let recording = Recording::new("rec_456".to_string(), "Meeting to Complete".to_string());
        db.create_recording(&recording).unwrap();

        db.complete_recording("rec_456", 120.5).unwrap();

        let retrieved = db.get_recording("rec_456").unwrap().unwrap();
        assert_eq!(retrieved.status, "completed");
        assert_eq!(retrieved.duration_seconds, Some(120.5));
    }
}
