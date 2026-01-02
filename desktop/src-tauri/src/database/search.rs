// Search functionality for Meeting-Local
// Full-text search across recordings and transcripts

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use super::models::{Recording, SearchResult, SearchFilters, Category, Tag};
use super::DatabaseManager;

impl DatabaseManager {
    /// Search recordings by query and filters
    pub fn search_recordings(&self, query: &str, filters: &SearchFilters) -> Result<Vec<SearchResult>> {
        self.with_connection(|conn| {
            search_recordings_impl(conn, query, filters)
        })
    }
}

/// Search recordings by title, transcript content, categories, and tags
fn search_recordings_impl(
    conn: &Connection,
    query: &str,
    filters: &SearchFilters,
) -> Result<Vec<SearchResult>> {
    let mut results = Vec::new();

    // If we have a text query, search both title and transcripts
    if !query.is_empty() {
        // Search in recording titles
        let title_results = search_by_title(conn, query, filters)?;
        results.extend(title_results);

        // Search in transcript content using FTS5
        if filters.search_transcripts {
            let transcript_results = search_transcripts_fts(conn, query, filters)?;
            // Merge results, avoiding duplicates
            for result in transcript_results {
                if !results.iter().any(|r| r.recording.id == result.recording.id) {
                    results.push(result);
                }
            }
        }

        // Search by category name
        let category_results = search_by_category_name(conn, query, filters)?;
        for result in category_results {
            if !results.iter().any(|r| r.recording.id == result.recording.id) {
                results.push(result);
            }
        }

        // Search by tag name
        let tag_results = search_by_tag_name(conn, query, filters)?;
        for result in tag_results {
            if !results.iter().any(|r| r.recording.id == result.recording.id) {
                results.push(result);
            }
        }
    } else {
        // No text query, just filter by categories/tags/dates
        let filtered_results = filter_recordings(conn, filters)?;
        results.extend(filtered_results);
    }

    Ok(results)
}

/// Search recordings by title
fn search_by_title(
    conn: &Connection,
    query: &str,
    filters: &SearchFilters,
) -> Result<Vec<SearchResult>> {
    let search_pattern = format!("%{}%", query);

    let mut sql = String::from(
        r#"
        SELECT DISTINCT r.id, r.title, r.created_at, r.completed_at, r.duration_seconds,
               r.status, r.audio_file_path, r.meeting_folder_path, r.microphone_device,
               r.system_audio_device, r.sample_rate, r.transcription_model, r.language, r.diarization_provider
        FROM recordings r
        WHERE r.title LIKE ?1
        "#
    );

    let mut param_count = 1;
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(search_pattern)];

    // Add date filters
    if let Some(ref date_from) = filters.date_from {
        param_count += 1;
        sql.push_str(&format!(" AND r.created_at >= ?{}", param_count));
        params_vec.push(Box::new(date_from.clone()));
    }
    if let Some(ref date_to) = filters.date_to {
        param_count += 1;
        sql.push_str(&format!(" AND r.created_at <= ?{}", param_count));
        params_vec.push(Box::new(date_to.clone()));
    }

    // Add category filter
    if let Some(ref cat_ids) = filters.category_ids {
        if !cat_ids.is_empty() {
            let placeholders: Vec<String> = cat_ids.iter().enumerate()
                .map(|(i, _)| format!("?{}", param_count + i + 1))
                .collect();
            sql.push_str(&format!(
                " AND r.id IN (SELECT recording_id FROM recording_categories WHERE category_id IN ({}))",
                placeholders.join(", ")
            ));
            for id in cat_ids {
                param_count += 1;
                params_vec.push(Box::new(id.clone()));
            }
        }
    }

    // Add tag filter
    if let Some(ref tag_ids) = filters.tag_ids {
        if !tag_ids.is_empty() {
            let placeholders: Vec<String> = tag_ids.iter().enumerate()
                .map(|(i, _)| format!("?{}", param_count + i + 1))
                .collect();
            sql.push_str(&format!(
                " AND r.id IN (SELECT recording_id FROM recording_tags WHERE tag_id IN ({}))",
                placeholders.join(", ")
            ));
            for id in tag_ids {
                params_vec.push(Box::new(id.clone()));
            }
        }
    }

    sql.push_str(" ORDER BY r.created_at DESC");

    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

    let mut stmt = conn.prepare(&sql).context("Failed to prepare search query")?;
    let recordings = stmt.query_map(params_refs.as_slice(), |row| {
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
    }).context("Failed to execute search query")?;

    let mut results = Vec::new();
    for recording in recordings {
        let recording = recording.context("Failed to read recording")?;
        let id = recording.id.clone();
        let title = recording.title.clone();

        let categories = get_recording_categories_internal(conn, &id)?;
        let tags = get_recording_tags_internal(conn, &id)?;

        results.push(SearchResult {
            recording,
            matched_text: format!("Title: {}", title),
            categories,
            tags,
        });
    }

    Ok(results)
}

// Internal helper functions for getting categories and tags
fn get_recording_categories_internal(conn: &Connection, recording_id: &str) -> Result<Vec<Category>> {
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

fn get_recording_tags_internal(conn: &Connection, recording_id: &str) -> Result<Vec<Tag>> {
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

/// Search transcripts using FTS5 full-text search
fn search_transcripts_fts(
    conn: &Connection,
    query: &str,
    filters: &SearchFilters,
) -> Result<Vec<SearchResult>> {
    // FTS5 query - escape special characters
    let fts_query = query.replace("\"", "\"\"");

    let mut sql = String::from(
        r#"
        SELECT DISTINCT r.id, r.title, r.created_at, r.completed_at, r.duration_seconds,
               r.status, r.audio_file_path, r.meeting_folder_path, r.microphone_device,
               r.system_audio_device, r.sample_rate, r.transcription_model, r.language, r.diarization_provider,
               snippet(transcript_fts, 1, '<mark>', '</mark>', '...', 32) as matched_text
        FROM recordings r
        INNER JOIN transcript_fts fts ON r.id = fts.recording_id
        WHERE transcript_fts MATCH ?1
        "#
    );

    let mut param_count = 1;
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(format!("\"{}\"", fts_query))];

    // Add date filters
    if let Some(ref date_from) = filters.date_from {
        param_count += 1;
        sql.push_str(&format!(" AND r.created_at >= ?{}", param_count));
        params_vec.push(Box::new(date_from.clone()));
    }
    if let Some(ref date_to) = filters.date_to {
        param_count += 1;
        sql.push_str(&format!(" AND r.created_at <= ?{}", param_count));
        params_vec.push(Box::new(date_to.clone()));
    }

    // Add category filter
    if let Some(ref cat_ids) = filters.category_ids {
        if !cat_ids.is_empty() {
            let placeholders: Vec<String> = cat_ids.iter().enumerate()
                .map(|(i, _)| format!("?{}", param_count + i + 1))
                .collect();
            sql.push_str(&format!(
                " AND r.id IN (SELECT recording_id FROM recording_categories WHERE category_id IN ({}))",
                placeholders.join(", ")
            ));
            for id in cat_ids {
                param_count += 1;
                params_vec.push(Box::new(id.clone()));
            }
        }
    }

    // Add tag filter
    if let Some(ref tag_ids) = filters.tag_ids {
        if !tag_ids.is_empty() {
            let placeholders: Vec<String> = tag_ids.iter().enumerate()
                .map(|(i, _)| format!("?{}", param_count + i + 1))
                .collect();
            sql.push_str(&format!(
                " AND r.id IN (SELECT recording_id FROM recording_tags WHERE tag_id IN ({}))",
                placeholders.join(", ")
            ));
            for id in tag_ids {
                params_vec.push(Box::new(id.clone()));
            }
        }
    }

    sql.push_str(" ORDER BY r.created_at DESC LIMIT 50");

    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

    let mut stmt = conn.prepare(&sql).context("Failed to prepare FTS query")?;
    let recordings = stmt.query_map(params_refs.as_slice(), |row| {
        Ok((
            Recording {
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
            },
            row.get::<_, String>(14)?,
        ))
    }).context("Failed to execute FTS query")?;

    let mut results = Vec::new();
    for result in recordings {
        let (recording, matched_text) = result.context("Failed to read search result")?;
        let id = recording.id.clone();

        let categories = get_recording_categories_internal(conn, &id)?;
        let tags = get_recording_tags_internal(conn, &id)?;

        results.push(SearchResult {
            recording,
            matched_text,
            categories,
            tags,
        });
    }

    Ok(results)
}

/// Filter recordings without text search
fn filter_recordings(
    conn: &Connection,
    filters: &SearchFilters,
) -> Result<Vec<SearchResult>> {
    let mut sql = String::from(
        r#"
        SELECT r.id, r.title, r.created_at, r.completed_at, r.duration_seconds,
               r.status, r.audio_file_path, r.meeting_folder_path, r.microphone_device,
               r.system_audio_device, r.sample_rate, r.transcription_model, r.language, r.diarization_provider
        FROM recordings r
        WHERE 1=1
        "#
    );

    let mut param_count = 0;
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    // Add date filters
    if let Some(ref date_from) = filters.date_from {
        param_count += 1;
        sql.push_str(&format!(" AND r.created_at >= ?{}", param_count));
        params_vec.push(Box::new(date_from.clone()));
    }
    if let Some(ref date_to) = filters.date_to {
        param_count += 1;
        sql.push_str(&format!(" AND r.created_at <= ?{}", param_count));
        params_vec.push(Box::new(date_to.clone()));
    }

    // Add category filter
    if let Some(ref cat_ids) = filters.category_ids {
        if !cat_ids.is_empty() {
            let placeholders: Vec<String> = cat_ids.iter().enumerate()
                .map(|(i, _)| format!("?{}", param_count + i + 1))
                .collect();
            sql.push_str(&format!(
                " AND r.id IN (SELECT recording_id FROM recording_categories WHERE category_id IN ({}))",
                placeholders.join(", ")
            ));
            for id in cat_ids {
                param_count += 1;
                params_vec.push(Box::new(id.clone()));
            }
        }
    }

    // Add tag filter
    if let Some(ref tag_ids) = filters.tag_ids {
        if !tag_ids.is_empty() {
            let placeholders: Vec<String> = tag_ids.iter().enumerate()
                .map(|(i, _)| format!("?{}", param_count + i + 1))
                .collect();
            sql.push_str(&format!(
                " AND r.id IN (SELECT recording_id FROM recording_tags WHERE tag_id IN ({}))",
                placeholders.join(", ")
            ));
            for id in tag_ids {
                params_vec.push(Box::new(id.clone()));
            }
        }
    }

    sql.push_str(" ORDER BY r.created_at DESC");

    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

    let mut stmt = conn.prepare(&sql).context("Failed to prepare filter query")?;
    let recordings = stmt.query_map(params_refs.as_slice(), |row| {
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
    }).context("Failed to execute filter query")?;

    let mut results = Vec::new();
    for recording in recordings {
        let recording = recording.context("Failed to read recording")?;
        let id = recording.id.clone();

        let categories = get_recording_categories_internal(conn, &id)?;
        let tags = get_recording_tags_internal(conn, &id)?;

        results.push(SearchResult {
            recording,
            matched_text: String::new(),
            categories,
            tags,
        });
    }

    Ok(results)
}

/// Search recordings by category name
fn search_by_category_name(
    conn: &Connection,
    query: &str,
    filters: &SearchFilters,
) -> Result<Vec<SearchResult>> {
    let search_pattern = format!("%{}%", query);

    let mut sql = String::from(
        r#"
        SELECT DISTINCT r.id, r.title, r.created_at, r.completed_at, r.duration_seconds,
               r.status, r.audio_file_path, r.meeting_folder_path, r.microphone_device,
               r.system_audio_device, r.sample_rate, r.transcription_model, r.language, r.diarization_provider,
               c.name as category_name
        FROM recordings r
        INNER JOIN recording_categories rc ON r.id = rc.recording_id
        INNER JOIN categories c ON rc.category_id = c.id
        WHERE c.name LIKE ?1
        "#
    );

    let mut param_count = 1;
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(search_pattern)];

    // Add date filters
    if let Some(ref date_from) = filters.date_from {
        param_count += 1;
        sql.push_str(&format!(" AND r.created_at >= ?{}", param_count));
        params_vec.push(Box::new(date_from.clone()));
    }
    if let Some(ref date_to) = filters.date_to {
        param_count += 1;
        sql.push_str(&format!(" AND r.created_at <= ?{}", param_count));
        params_vec.push(Box::new(date_to.clone()));
    }

    sql.push_str(" ORDER BY r.created_at DESC");

    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

    let mut stmt = conn.prepare(&sql).context("Failed to prepare category name search query")?;
    let recordings = stmt.query_map(params_refs.as_slice(), |row| {
        Ok((
            Recording {
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
            },
            row.get::<_, String>(14)?,
        ))
    }).context("Failed to execute category name search query")?;

    let mut results = Vec::new();
    for result in recordings {
        let (recording, category_name) = result.context("Failed to read search result")?;
        let id = recording.id.clone();

        let categories = get_recording_categories_internal(conn, &id)?;
        let tags = get_recording_tags_internal(conn, &id)?;

        results.push(SearchResult {
            recording,
            matched_text: format!("Category: {}", category_name),
            categories,
            tags,
        });
    }

    Ok(results)
}

/// Search recordings by tag name
fn search_by_tag_name(
    conn: &Connection,
    query: &str,
    filters: &SearchFilters,
) -> Result<Vec<SearchResult>> {
    let search_pattern = format!("%{}%", query);

    let mut sql = String::from(
        r#"
        SELECT DISTINCT r.id, r.title, r.created_at, r.completed_at, r.duration_seconds,
               r.status, r.audio_file_path, r.meeting_folder_path, r.microphone_device,
               r.system_audio_device, r.sample_rate, r.transcription_model, r.language, r.diarization_provider,
               t.name as tag_name
        FROM recordings r
        INNER JOIN recording_tags rt ON r.id = rt.recording_id
        INNER JOIN tags t ON rt.tag_id = t.id
        WHERE t.name LIKE ?1
        "#
    );

    let mut param_count = 1;
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(search_pattern)];

    // Add date filters
    if let Some(ref date_from) = filters.date_from {
        param_count += 1;
        sql.push_str(&format!(" AND r.created_at >= ?{}", param_count));
        params_vec.push(Box::new(date_from.clone()));
    }
    if let Some(ref date_to) = filters.date_to {
        param_count += 1;
        sql.push_str(&format!(" AND r.created_at <= ?{}", param_count));
        params_vec.push(Box::new(date_to.clone()));
    }

    sql.push_str(" ORDER BY r.created_at DESC");

    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

    let mut stmt = conn.prepare(&sql).context("Failed to prepare tag name search query")?;
    let recordings = stmt.query_map(params_refs.as_slice(), |row| {
        Ok((
            Recording {
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
            },
            row.get::<_, String>(14)?,
        ))
    }).context("Failed to execute tag name search query")?;

    let mut results = Vec::new();
    for result in recordings {
        let (recording, tag_name) = result.context("Failed to read search result")?;
        let id = recording.id.clone();

        let categories = get_recording_categories_internal(conn, &id)?;
        let tags = get_recording_tags_internal(conn, &id)?;

        results.push(SearchResult {
            recording,
            matched_text: format!("Tag: {}", tag_name),
            categories,
            tags,
        });
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::migrations;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute("PRAGMA foreign_keys = ON", []).unwrap();
        migrations::run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn test_empty_search() {
        let conn = setup_test_db();

        let filters = SearchFilters::default();
        let results = search_recordings_impl(&conn, "", &filters).unwrap();

        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_no_results() {
        let conn = setup_test_db();

        let filters = SearchFilters::default();
        let results = search_recordings_impl(&conn, "nonexistent", &filters).unwrap();

        assert_eq!(results.len(), 0);
    }
}
