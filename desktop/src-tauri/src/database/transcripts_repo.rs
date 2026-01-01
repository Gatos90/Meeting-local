// Transcripts repository for Meeting-Local
// Handles CRUD operations for transcript segments

use anyhow::{Context, Result};
use rusqlite::{Connection, params};

use super::models::TranscriptSegment;
use super::DatabaseManager;

impl DatabaseManager {
    /// Save a single transcript segment
    pub fn save_transcript_segment(&self, segment: &TranscriptSegment) -> Result<()> {
        self.with_connection(|conn| {
            save_transcript_segment_impl(conn, segment)
        })
    }

    /// Save multiple transcript segments in a batch
    pub fn save_transcript_segments_batch(&self, segments: &[TranscriptSegment]) -> Result<()> {
        self.with_connection(|conn| {
            save_transcript_segments_batch_impl(conn, segments)
        })
    }

    /// Get all transcript segments for a recording
    pub fn get_transcript_segments(&self, recording_id: &str) -> Result<Vec<TranscriptSegment>> {
        self.with_connection(|conn| {
            get_transcript_segments_impl(conn, recording_id)
        })
    }

    /// Delete all transcript segments for a recording
    pub fn delete_transcript_segments(&self, recording_id: &str) -> Result<()> {
        self.with_connection(|conn| {
            delete_transcript_segments_impl(conn, recording_id)
        })
    }

    /// Get the full transcript text for a recording (all segments concatenated)
    pub fn get_full_transcript(&self, recording_id: &str) -> Result<String> {
        self.with_connection(|conn| {
            get_full_transcript_impl(conn, recording_id)
        })
    }

    /// Replace all transcript segments for a recording with new ones
    /// This is used when retranscription is complete
    pub fn replace_transcripts(&self, recording_id: &str, segments: &[TranscriptSegment]) -> Result<()> {
        self.with_connection(|conn| {
            replace_transcripts_impl(conn, recording_id, segments)
        })
    }

    /// Update speaker label for all segments with a given speaker_id
    /// This is used when renaming a speaker
    pub fn update_speaker_label(&self, speaker_id: &str, new_label: &str) -> Result<usize> {
        self.with_connection(|conn| {
            update_speaker_label_impl(conn, speaker_id, new_label)
        })
    }

    /// Update the text content of a transcript segment
    pub fn update_transcript_text(&self, segment_id: &str, new_text: &str) -> Result<()> {
        self.with_connection(|conn| {
            update_transcript_text_impl(conn, segment_id, new_text)
        })
    }
}

fn save_transcript_segment_impl(conn: &Connection, segment: &TranscriptSegment) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO transcript_segments (
            id, recording_id, text, audio_start_time, audio_end_time,
            duration, display_time, confidence, sequence_id,
            speaker_id, speaker_label, is_registered_speaker
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
        ON CONFLICT(id) DO UPDATE SET
            text = excluded.text,
            audio_start_time = excluded.audio_start_time,
            audio_end_time = excluded.audio_end_time,
            duration = excluded.duration,
            display_time = excluded.display_time,
            confidence = excluded.confidence,
            sequence_id = excluded.sequence_id,
            speaker_id = excluded.speaker_id,
            speaker_label = excluded.speaker_label,
            is_registered_speaker = excluded.is_registered_speaker
        "#,
        params![
            segment.id,
            segment.recording_id,
            segment.text,
            segment.audio_start_time,
            segment.audio_end_time,
            segment.duration,
            segment.display_time,
            segment.confidence,
            segment.sequence_id,
            segment.speaker_id,
            segment.speaker_label,
            segment.is_registered_speaker as i32,
        ],
    ).context("Failed to save transcript segment")?;

    Ok(())
}

fn save_transcript_segments_batch_impl(conn: &Connection, segments: &[TranscriptSegment]) -> Result<()> {
    let tx = conn.unchecked_transaction()
        .context("Failed to start transaction")?;

    for segment in segments {
        tx.execute(
            r#"
            INSERT INTO transcript_segments (
                id, recording_id, text, audio_start_time, audio_end_time,
                duration, display_time, confidence, sequence_id,
                speaker_id, speaker_label, is_registered_speaker
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ON CONFLICT(id) DO UPDATE SET
                text = excluded.text,
                audio_start_time = excluded.audio_start_time,
                audio_end_time = excluded.audio_end_time,
                duration = excluded.duration,
                display_time = excluded.display_time,
                confidence = excluded.confidence,
                sequence_id = excluded.sequence_id,
                speaker_id = excluded.speaker_id,
                speaker_label = excluded.speaker_label,
                is_registered_speaker = excluded.is_registered_speaker
            "#,
            params![
                segment.id,
                segment.recording_id,
                segment.text,
                segment.audio_start_time,
                segment.audio_end_time,
                segment.duration,
                segment.display_time,
                segment.confidence,
                segment.sequence_id,
                segment.speaker_id,
                segment.speaker_label,
                segment.is_registered_speaker as i32,
            ],
        ).context("Failed to save transcript segment in batch")?;
    }

    tx.commit().context("Failed to commit transcript batch")?;
    Ok(())
}

fn get_transcript_segments_impl(conn: &Connection, recording_id: &str) -> Result<Vec<TranscriptSegment>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, recording_id, text, audio_start_time, audio_end_time,
               duration, display_time, confidence, sequence_id,
               speaker_id, speaker_label, is_registered_speaker
        FROM transcript_segments
        WHERE recording_id = ?
        ORDER BY sequence_id ASC
        "#
    ).context("Failed to prepare get_transcript_segments query")?;

    let segments = stmt.query_map(params![recording_id], |row| {
        Ok(TranscriptSegment {
            id: row.get(0)?,
            recording_id: row.get(1)?,
            text: row.get(2)?,
            audio_start_time: row.get(3)?,
            audio_end_time: row.get(4)?,
            duration: row.get(5)?,
            display_time: row.get(6)?,
            confidence: row.get(7)?,
            sequence_id: row.get(8)?,
            speaker_id: row.get(9)?,
            speaker_label: row.get(10)?,
            is_registered_speaker: row.get::<_, Option<i32>>(11)?.map_or(false, |v| v != 0),
        })
    }).context("Failed to query transcript segments")?;

    segments.collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to collect transcript segments")
}

fn delete_transcript_segments_impl(conn: &Connection, recording_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM transcript_segments WHERE recording_id = ?",
        params![recording_id],
    ).context("Failed to delete transcript segments")?;

    Ok(())
}

fn get_full_transcript_impl(conn: &Connection, recording_id: &str) -> Result<String> {
    let segments = get_transcript_segments_impl(conn, recording_id)?;
    let texts: Vec<&str> = segments.iter().map(|s| s.text.as_str()).collect();
    Ok(texts.join(" "))
}

fn replace_transcripts_impl(conn: &Connection, recording_id: &str, segments: &[TranscriptSegment]) -> Result<()> {
    let tx = conn.unchecked_transaction()
        .context("Failed to start transaction for replace_transcripts")?;

    // First, delete all existing segments for this recording
    tx.execute(
        "DELETE FROM transcript_segments WHERE recording_id = ?",
        params![recording_id],
    ).context("Failed to delete old transcript segments")?;

    // Then insert all new segments
    for segment in segments {
        tx.execute(
            r#"
            INSERT INTO transcript_segments (
                id, recording_id, text, audio_start_time, audio_end_time,
                duration, display_time, confidence, sequence_id,
                speaker_id, speaker_label, is_registered_speaker
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            params![
                segment.id,
                segment.recording_id,
                segment.text,
                segment.audio_start_time,
                segment.audio_end_time,
                segment.duration,
                segment.display_time,
                segment.confidence,
                segment.sequence_id,
                segment.speaker_id,
                segment.speaker_label,
                segment.is_registered_speaker as i32,
            ],
        ).context("Failed to insert new transcript segment")?;
    }

    tx.commit().context("Failed to commit replace_transcripts")?;
    Ok(())
}

fn update_speaker_label_impl(conn: &Connection, speaker_id: &str, new_label: &str) -> Result<usize> {
    let rows_updated = conn.execute(
        "UPDATE transcript_segments SET speaker_label = ? WHERE speaker_id = ?",
        params![new_label, speaker_id],
    ).context("Failed to update speaker label")?;

    Ok(rows_updated)
}

fn update_transcript_text_impl(conn: &Connection, segment_id: &str, new_text: &str) -> Result<()> {
    conn.execute(
        "UPDATE transcript_segments SET text = ? WHERE id = ?",
        params![new_text, segment_id],
    ).context("Failed to update transcript text")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crate::database::models::Recording;

    fn create_test_db() -> DatabaseManager {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        DatabaseManager::new(db_path).unwrap()
    }

    #[test]
    fn test_save_and_get_transcript_segments() {
        let db = create_test_db();

        // Create a recording first
        let recording = Recording::new("rec_test".to_string(), "Test".to_string());
        db.create_recording(&recording).unwrap();

        // Create segments
        let segments = vec![
            TranscriptSegment {
                id: "seg_1".to_string(),
                recording_id: "rec_test".to_string(),
                text: "Hello world".to_string(),
                audio_start_time: 0.0,
                audio_end_time: 1.5,
                duration: 1.5,
                display_time: "[00:00]".to_string(),
                confidence: 0.95,
                sequence_id: 1,
                speaker_id: Some("speaker_0".to_string()),
                speaker_label: Some("Speaker 1".to_string()),
                is_registered_speaker: false,
            },
            TranscriptSegment {
                id: "seg_2".to_string(),
                recording_id: "rec_test".to_string(),
                text: "This is a test".to_string(),
                audio_start_time: 1.5,
                audio_end_time: 3.0,
                duration: 1.5,
                display_time: "[00:01]".to_string(),
                confidence: 0.92,
                sequence_id: 2,
                speaker_id: Some("speaker_1".to_string()),
                speaker_label: Some("Speaker 2".to_string()),
                is_registered_speaker: false,
            },
        ];

        db.save_transcript_segments_batch(&segments).unwrap();

        let retrieved = db.get_transcript_segments("rec_test").unwrap();
        assert_eq!(retrieved.len(), 2);
        assert_eq!(retrieved[0].text, "Hello world");
        assert_eq!(retrieved[1].text, "This is a test");
    }

    #[test]
    fn test_get_full_transcript() {
        let db = create_test_db();

        let recording = Recording::new("rec_full".to_string(), "Full Test".to_string());
        db.create_recording(&recording).unwrap();

        let segments = vec![
            TranscriptSegment {
                id: "seg_a".to_string(),
                recording_id: "rec_full".to_string(),
                text: "First".to_string(),
                audio_start_time: 0.0,
                audio_end_time: 1.0,
                duration: 1.0,
                display_time: "[00:00]".to_string(),
                confidence: 1.0,
                sequence_id: 1,
                speaker_id: None,
                speaker_label: None,
                is_registered_speaker: false,
            },
            TranscriptSegment {
                id: "seg_b".to_string(),
                recording_id: "rec_full".to_string(),
                text: "Second".to_string(),
                audio_start_time: 1.0,
                audio_end_time: 2.0,
                duration: 1.0,
                display_time: "[00:01]".to_string(),
                confidence: 1.0,
                sequence_id: 2,
                speaker_id: None,
                speaker_label: None,
                is_registered_speaker: false,
            },
        ];

        db.save_transcript_segments_batch(&segments).unwrap();

        let full = db.get_full_transcript("rec_full").unwrap();
        assert_eq!(full, "First Second");
    }
}
