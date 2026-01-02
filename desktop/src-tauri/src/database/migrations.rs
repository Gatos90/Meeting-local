// Database migrations for Meeting-Local
// Creates and updates the database schema

use anyhow::{Context, Result};
use rusqlite::Connection;

/// Current schema version
const SCHEMA_VERSION: i32 = 10;

/// Run all necessary migrations to bring the database up to date
pub fn run_migrations(conn: &Connection) -> Result<()> {
    let current_version = get_schema_version(conn)?;

    if current_version < 1 {
        migrate_v1(conn)?;
    }

    if current_version < 2 {
        migrate_v2(conn)?;
    }

    if current_version < 3 {
        migrate_v3(conn)?;
    }

    if current_version < 4 {
        migrate_v4(conn)?;
    }

    if current_version < 5 {
        migrate_v5(conn)?;
    }

    if current_version < 6 {
        migrate_v6(conn)?;
    }

    if current_version < 7 {
        migrate_v7(conn)?;
    }

    if current_version < 8 {
        migrate_v8(conn)?;
    }

    if current_version < 9 {
        migrate_v9(conn)?;
    }

    if current_version < 10 {
        migrate_v10(conn)?;
    }

    Ok(())
}

/// Get the current schema version from the database
fn get_schema_version(conn: &Connection) -> Result<i32> {
    // Check if schema_version table exists
    let table_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='schema_version'",
        [],
        |row| row.get(0),
    ).unwrap_or(false);

    if !table_exists {
        return Ok(0);
    }

    let version: i32 = conn.query_row(
        "SELECT MAX(version) FROM schema_version",
        [],
        |row| row.get(0),
    ).unwrap_or(0);

    Ok(version)
}

/// Initial schema creation (version 1)
fn migrate_v1(conn: &Connection) -> Result<()> {
    log::info!("Running database migration v1");

    conn.execute_batch(r#"
        -- Schema version tracking
        CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Settings table: Key-value store for application settings
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY NOT NULL,
            value TEXT NOT NULL,
            value_type TEXT NOT NULL DEFAULT 'string',
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Recordings table: Core metadata for each meeting recording
        CREATE TABLE IF NOT EXISTS recordings (
            id TEXT PRIMARY KEY NOT NULL,
            title TEXT NOT NULL,
            created_at TEXT NOT NULL,
            completed_at TEXT,
            duration_seconds REAL,
            status TEXT NOT NULL DEFAULT 'recording',
            audio_file_path TEXT,
            meeting_folder_path TEXT,
            microphone_device TEXT,
            system_audio_device TEXT,
            sample_rate INTEGER DEFAULT 48000,
            transcription_model TEXT,
            language TEXT DEFAULT 'auto',
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Transcript segments table: Individual transcription chunks
        CREATE TABLE IF NOT EXISTS transcript_segments (
            id TEXT PRIMARY KEY NOT NULL,
            recording_id TEXT NOT NULL,
            text TEXT NOT NULL,
            audio_start_time REAL NOT NULL,
            audio_end_time REAL NOT NULL,
            duration REAL NOT NULL,
            display_time TEXT NOT NULL,
            confidence REAL DEFAULT 1.0,
            sequence_id INTEGER NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
        );

        -- Create indexes for transcript lookups
        CREATE INDEX IF NOT EXISTS idx_transcript_segments_recording_id
        ON transcript_segments(recording_id);

        CREATE INDEX IF NOT EXISTS idx_transcript_segments_sequence
        ON transcript_segments(recording_id, sequence_id);

        -- Categories table: Predefined and user-created categories
        CREATE TABLE IF NOT EXISTS categories (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL UNIQUE,
            color TEXT,
            is_system INTEGER DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Tags table: User-defined tags
        CREATE TABLE IF NOT EXISTS tags (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL UNIQUE,
            color TEXT,
            usage_count INTEGER DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Recording-to-category junction table
        CREATE TABLE IF NOT EXISTS recording_categories (
            recording_id TEXT NOT NULL,
            category_id TEXT NOT NULL,
            PRIMARY KEY (recording_id, category_id),
            FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE,
            FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE CASCADE
        );

        -- Recording-to-tag junction table
        CREATE TABLE IF NOT EXISTS recording_tags (
            recording_id TEXT NOT NULL,
            tag_id TEXT NOT NULL,
            PRIMARY KEY (recording_id, tag_id),
            FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE,
            FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
        );

        -- Full-text search virtual table for transcript content
        CREATE VIRTUAL TABLE IF NOT EXISTS transcript_fts USING fts5(
            recording_id,
            text,
            content='transcript_segments',
            content_rowid='rowid'
        );

        -- Triggers to keep FTS in sync with transcript_segments
        CREATE TRIGGER IF NOT EXISTS transcript_fts_insert AFTER INSERT ON transcript_segments BEGIN
            INSERT INTO transcript_fts(rowid, recording_id, text)
            VALUES (new.rowid, new.recording_id, new.text);
        END;

        CREATE TRIGGER IF NOT EXISTS transcript_fts_delete AFTER DELETE ON transcript_segments BEGIN
            INSERT INTO transcript_fts(transcript_fts, rowid, recording_id, text)
            VALUES('delete', old.rowid, old.recording_id, old.text);
        END;

        CREATE TRIGGER IF NOT EXISTS transcript_fts_update AFTER UPDATE ON transcript_segments BEGIN
            INSERT INTO transcript_fts(transcript_fts, rowid, recording_id, text)
            VALUES('delete', old.rowid, old.recording_id, old.text);
            INSERT INTO transcript_fts(rowid, recording_id, text)
            VALUES (new.rowid, new.recording_id, new.text);
        END;

        -- Seed predefined categories
        INSERT OR IGNORE INTO categories (id, name, color, is_system) VALUES
            ('cat_daily', 'Daily', '#3B82F6', 1),
            ('cat_sales', 'Sales', '#10B981', 1),
            ('cat_strategy', 'Strategy', '#8B5CF6', 1),
            ('cat_product', 'Product', '#F59E0B', 1),
            ('cat_weekly', 'Weekly', '#EC4899', 1),
            ('cat_hr', 'HR', '#6366F1', 1),
            ('cat_engineering', 'Engineering', '#14B8A6', 1),
            ('cat_design', 'Design', '#F97316', 1);

        -- Record migration
        INSERT INTO schema_version (version) VALUES (1);
    "#).context("Failed to run migration v1")?;

    log::info!("Migration v1 completed successfully");
    Ok(())
}

/// Speaker diarization schema (version 2)
fn migrate_v2(conn: &Connection) -> Result<()> {
    log::info!("Running database migration v2 - Speaker diarization");

    conn.execute_batch(r#"
        -- Add speaker columns to transcript_segments
        ALTER TABLE transcript_segments ADD COLUMN speaker_id TEXT;
        ALTER TABLE transcript_segments ADD COLUMN speaker_label TEXT DEFAULT 'Unknown';
        ALTER TABLE transcript_segments ADD COLUMN is_registered_speaker INTEGER DEFAULT 0;

        -- Registered speakers table for voice profiles
        CREATE TABLE IF NOT EXISTS registered_speakers (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            embedding BLOB NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            sample_count INTEGER DEFAULT 1,
            last_seen TEXT
        );

        -- Speaker label overrides per recording
        -- Allows users to rename "Speaker 1" to "John" for a specific recording
        CREATE TABLE IF NOT EXISTS speaker_labels (
            recording_id TEXT NOT NULL,
            speaker_id TEXT NOT NULL,
            custom_label TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (recording_id, speaker_id),
            FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
        );

        -- Index for faster speaker lookups
        CREATE INDEX IF NOT EXISTS idx_transcript_segments_speaker
        ON transcript_segments(recording_id, speaker_id);

        -- Record migration
        INSERT INTO schema_version (version) VALUES (2);
    "#).context("Failed to run migration v2")?;

    log::info!("Migration v2 completed successfully");
    Ok(())
}

/// Add diarization provider tracking (version 3)
fn migrate_v3(conn: &Connection) -> Result<()> {
    log::info!("Running database migration v3 - Diarization provider tracking");

    conn.execute_batch(r#"
        -- Add diarization_provider column to recordings
        ALTER TABLE recordings ADD COLUMN diarization_provider TEXT;

        -- Record migration
        INSERT INTO schema_version (version) VALUES (3);
    "#).context("Failed to run migration v3")?;

    log::info!("Migration v3 completed successfully");
    Ok(())
}

/// Chat messages schema (version 4)
fn migrate_v4(conn: &Connection) -> Result<()> {
    log::info!("Running database migration v4 - Chat messages");

    conn.execute_batch(r#"
        -- Chat messages table for AI conversations about recordings
        CREATE TABLE IF NOT EXISTS chat_messages (
            id TEXT PRIMARY KEY NOT NULL,
            recording_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            sequence_id INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'complete',
            error_message TEXT,
            FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
        );

        -- Index for fast message lookups by recording
        CREATE INDEX IF NOT EXISTS idx_chat_messages_recording
        ON chat_messages(recording_id, sequence_id);

        -- Index for finding pending/streaming messages
        CREATE INDEX IF NOT EXISTS idx_chat_messages_status
        ON chat_messages(status) WHERE status != 'complete';

        -- Record migration
        INSERT INTO schema_version (version) VALUES (4);
    "#).context("Failed to run migration v4")?;

    log::info!("Migration v4 completed successfully");
    Ok(())
}

/// Add provider/model tracking to chat messages (version 5)
fn migrate_v5(conn: &Connection) -> Result<()> {
    log::info!("Running database migration v5 - Chat provider/model tracking");

    conn.execute_batch(r#"
        -- Add provider and model columns to chat_messages
        ALTER TABLE chat_messages ADD COLUMN provider_type TEXT;
        ALTER TABLE chat_messages ADD COLUMN model_id TEXT;

        -- Record migration
        INSERT INTO schema_version (version) VALUES (5);
    "#).context("Failed to run migration v5")?;

    log::info!("Migration v5 completed successfully");
    Ok(())
}

/// Chat sessions schema (version 6) - Multiple chat conversations per recording
fn migrate_v6(conn: &Connection) -> Result<()> {
    log::info!("Running database migration v6 - Chat sessions");

    conn.execute_batch(r#"
        -- Chat sessions table: Groups chat messages into separate conversations
        CREATE TABLE IF NOT EXISTS chat_sessions (
            id TEXT PRIMARY KEY NOT NULL,
            recording_id TEXT NOT NULL,
            title TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            provider_type TEXT,
            model_id TEXT,
            FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
        );

        -- Add session_id column to chat_messages
        ALTER TABLE chat_messages ADD COLUMN session_id TEXT REFERENCES chat_sessions(id);

        -- Index for fast session lookups by recording (newest first)
        CREATE INDEX IF NOT EXISTS idx_chat_sessions_recording
        ON chat_sessions(recording_id, created_at DESC);

        -- Index for fast message lookups by session
        CREATE INDEX IF NOT EXISTS idx_chat_messages_session
        ON chat_messages(session_id, sequence_id);

        -- Record migration
        INSERT INTO schema_version (version) VALUES (6);
    "#).context("Failed to run migration v6")?;

    // Migrate existing messages: create sessions for each recording that has messages
    log::info!("Migrating existing chat messages to sessions...");

    // Get distinct recording_ids that have chat messages without session_id
    let mut stmt = conn.prepare(
        "SELECT DISTINCT recording_id FROM chat_messages WHERE session_id IS NULL"
    ).context("Failed to prepare migration query")?;

    let recording_ids: Vec<String> = stmt.query_map([], |row| row.get(0))
        .context("Failed to query recording IDs")?
        .filter_map(|r| r.ok())
        .collect();

    for recording_id in recording_ids {
        // Create a session for this recording's existing messages
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        // Get the first user message to use as title preview
        let first_message: Option<String> = conn.query_row(
            "SELECT content FROM chat_messages WHERE recording_id = ? AND role = 'user' ORDER BY sequence_id LIMIT 1",
            [&recording_id],
            |row| row.get(0),
        ).ok();

        let title = match first_message {
            Some(msg) => {
                let preview: String = msg.chars().take(50).collect();
                if msg.len() > 50 {
                    format!("{}...", preview)
                } else {
                    preview
                }
            }
            None => "Imported Chat".to_string(),
        };

        // Get provider/model from the most recent assistant message
        let (provider_type, model_id): (Option<String>, Option<String>) = conn.query_row(
            "SELECT provider_type, model_id FROM chat_messages WHERE recording_id = ? AND role = 'assistant' AND provider_type IS NOT NULL ORDER BY sequence_id DESC LIMIT 1",
            [&recording_id],
            |row| Ok((row.get(0).ok(), row.get(1).ok())),
        ).unwrap_or((None, None));

        // Insert the session
        conn.execute(
            "INSERT INTO chat_sessions (id, recording_id, title, created_at, provider_type, model_id) VALUES (?, ?, ?, ?, ?, ?)",
            rusqlite::params![session_id, recording_id, title, now, provider_type, model_id],
        ).context("Failed to create migration session")?;

        // Update all messages for this recording to use the new session
        conn.execute(
            "UPDATE chat_messages SET session_id = ? WHERE recording_id = ? AND session_id IS NULL",
            rusqlite::params![session_id, recording_id],
        ).context("Failed to update messages with session_id")?;

        log::info!("Migrated messages for recording {} to session {}", recording_id, session_id);
    }

    log::info!("Migration v6 completed successfully");
    Ok(())
}

/// Prompt templates schema (version 7) - Custom AI prompt templates
fn migrate_v7(conn: &Connection) -> Result<()> {
    log::info!("Running database migration v7 - Prompt templates");

    conn.execute_batch(r#"
        -- Prompt templates table for AI chat quick actions
        CREATE TABLE IF NOT EXISTS prompt_templates (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            description TEXT,
            prompt TEXT NOT NULL,
            icon TEXT,
            is_builtin INTEGER DEFAULT 0,
            sort_order INTEGER DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Index for listing templates by sort order
        CREATE INDEX IF NOT EXISTS idx_prompt_templates_sort
        ON prompt_templates(sort_order, created_at);

        -- Seed default built-in templates
        INSERT OR IGNORE INTO prompt_templates (id, name, description, prompt, icon, is_builtin, sort_order) VALUES
            ('builtin_summarize', 'Summarize', 'Get a concise summary of the meeting', 'Please provide a concise summary of this meeting, highlighting the main topics discussed and any conclusions reached.', 'FileText', 1, 1),
            ('builtin_key_points', 'Key Points', 'Extract the most important points', 'What are the key points discussed in this meeting? Please list them as bullet points.', 'List', 1, 2),
            ('builtin_action_items', 'Action Items', 'Find tasks and next steps', 'What action items, tasks, or next steps were mentioned in this meeting? Please list them with any assigned owners if mentioned.', 'CheckSquare', 1, 3);

        -- Record migration
        INSERT INTO schema_version (version) VALUES (7);
    "#).context("Failed to run migration v7")?;

    log::info!("Migration v7 completed successfully");
    Ok(())
}

/// AI Tools schema (version 8) - Function calling tools for AI chat
fn migrate_v8(conn: &Connection) -> Result<()> {
    log::info!("Running database migration v8 - AI Tools");

    conn.execute_batch(r#"
        -- Tools table: Function definitions for AI tool calling
        CREATE TABLE IF NOT EXISTS tools (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            description TEXT,
            tool_type TEXT NOT NULL DEFAULT 'custom',
            function_schema TEXT NOT NULL,
            execution_location TEXT DEFAULT 'backend',
            enabled INTEGER DEFAULT 1,
            is_default INTEGER DEFAULT 0,
            icon TEXT,
            sort_order INTEGER DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Chat session tools: Which tools are enabled per chat session
        CREATE TABLE IF NOT EXISTS chat_session_tools (
            session_id TEXT NOT NULL,
            tool_id TEXT NOT NULL,
            enabled INTEGER DEFAULT 1,
            PRIMARY KEY (session_id, tool_id),
            FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE,
            FOREIGN KEY (tool_id) REFERENCES tools(id) ON DELETE CASCADE
        );

        -- Indexes for tools
        CREATE INDEX IF NOT EXISTS idx_tools_enabled ON tools(enabled, sort_order);
        CREATE INDEX IF NOT EXISTS idx_tools_default ON tools(is_default) WHERE is_default = 1;
        CREATE INDEX IF NOT EXISTS idx_chat_session_tools ON chat_session_tools(session_id);

        -- Record migration
        INSERT INTO schema_version (version) VALUES (8);
    "#).context("Failed to run migration v8")?;

    // Seed built-in tools
    seed_builtin_tools(conn)?;

    log::info!("Migration v8 completed successfully");
    Ok(())
}

/// MCP Servers schema (version 9) - Model Context Protocol server management
fn migrate_v9(conn: &Connection) -> Result<()> {
    log::info!("Running database migration v9 - MCP Servers");

    conn.execute_batch(r#"
        -- MCP Servers table: External tool servers using Model Context Protocol
        CREATE TABLE IF NOT EXISTS mcp_servers (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL UNIQUE,
            command TEXT NOT NULL,
            args TEXT NOT NULL DEFAULT '[]',
            env TEXT DEFAULT '{}',
            working_directory TEXT,
            auto_start INTEGER DEFAULT 0,
            enabled INTEGER DEFAULT 1,
            status TEXT DEFAULT 'stopped',
            last_error TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Add mcp_server_id column to tools table for MCP-discovered tools
        ALTER TABLE tools ADD COLUMN mcp_server_id TEXT REFERENCES mcp_servers(id) ON DELETE CASCADE;

        -- Index for listing MCP servers
        CREATE INDEX IF NOT EXISTS idx_mcp_servers_enabled ON mcp_servers(enabled);
        CREATE INDEX IF NOT EXISTS idx_mcp_servers_auto_start ON mcp_servers(auto_start) WHERE auto_start = 1;

        -- Index for tools by MCP server
        CREATE INDEX IF NOT EXISTS idx_tools_mcp_server ON tools(mcp_server_id) WHERE mcp_server_id IS NOT NULL;

        -- Record migration
        INSERT INTO schema_version (version) VALUES (9);
    "#).context("Failed to run migration v9")?;

    log::info!("Migration v9 completed successfully");
    Ok(())
}

/// Model configuration schema (version 10) - User-defined model settings
fn migrate_v10(conn: &Connection) -> Result<()> {
    log::info!("Running database migration v10 - Model configuration");

    conn.execute_batch(r#"
        -- Model configuration table: Stores user-defined settings per model
        CREATE TABLE IF NOT EXISTS model_config (
            model_id TEXT PRIMARY KEY NOT NULL,
            has_native_tool_support INTEGER DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Index for quick lookups
        CREATE INDEX IF NOT EXISTS idx_model_config_tool_support
        ON model_config(has_native_tool_support) WHERE has_native_tool_support = 1;

        -- Record migration
        INSERT INTO schema_version (version) VALUES (10);
    "#).context("Failed to run migration v10")?;

    log::info!("Migration v10 completed successfully");
    Ok(())
}

/// Seed the built-in tools that come with the app
fn seed_builtin_tools(conn: &Connection) -> Result<()> {
    log::info!("Seeding built-in tools...");

    // get_current_time tool
    conn.execute(
        r#"INSERT OR IGNORE INTO tools (id, name, description, tool_type, function_schema, execution_location, enabled, is_default, icon, sort_order)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        rusqlite::params![
            "builtin_get_current_time",
            "get_current_time",
            "Get the current date and time",
            "builtin",
            r#"{"name":"get_current_time","description":"Get the current date and time","parameters":{"type":"object","properties":{"format":{"type":"string","description":"Optional format string (e.g., 'iso', 'local', 'date', 'time')"}},"required":[]}}"#,
            "backend",
            1,
            1,
            "Clock",
            1
        ],
    ).context("Failed to seed get_current_time tool")?;

    // Ensure is_default is set (for existing databases where tool already exists)
    conn.execute(
        "UPDATE tools SET is_default = 1, enabled = 1 WHERE id = ?",
        rusqlite::params!["builtin_get_current_time"],
    ).context("Failed to update get_current_time defaults")?;

    // search_transcript tool
    conn.execute(
        r#"INSERT OR IGNORE INTO tools (id, name, description, tool_type, function_schema, execution_location, enabled, is_default, icon, sort_order)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        rusqlite::params![
            "builtin_search_transcript",
            "search_transcript",
            "Search within the meeting transcript for specific content",
            "builtin",
            r#"{"name":"search_transcript","description":"Search within the meeting transcript for specific content","parameters":{"type":"object","properties":{"query":{"type":"string","description":"The search query to find in the transcript"},"limit":{"type":"integer","description":"Maximum number of results to return (default: 10)"}},"required":["query"]}}"#,
            "backend",
            1,
            1,
            "Search",
            2
        ],
    ).context("Failed to seed search_transcript tool")?;

    // Ensure is_default is set (for existing databases where tool already exists)
    conn.execute(
        "UPDATE tools SET is_default = 1, enabled = 1 WHERE id = ?",
        rusqlite::params!["builtin_search_transcript"],
    ).context("Failed to update search_transcript defaults")?;

    // list_speakers tool
    conn.execute(
        r#"INSERT OR IGNORE INTO tools (id, name, description, tool_type, function_schema, execution_location, enabled, is_default, icon, sort_order)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        rusqlite::params![
            "builtin_list_speakers",
            "list_speakers",
            "Get a list of all speakers identified in the meeting",
            "builtin",
            r#"{"name":"list_speakers","description":"Get a list of all speakers identified in the meeting","parameters":{"type":"object","properties":{},"required":[]}}"#,
            "backend",
            1,
            1,
            "Users",
            3
        ],
    ).context("Failed to seed list_speakers tool")?;

    // Ensure is_default is set (for existing databases where tool already exists)
    conn.execute(
        "UPDATE tools SET is_default = 1, enabled = 1 WHERE id = ?",
        rusqlite::params!["builtin_list_speakers"],
    ).context("Failed to update list_speakers defaults")?;

    // get_segment tool
    conn.execute(
        r#"INSERT OR IGNORE INTO tools (id, name, description, tool_type, function_schema, execution_location, enabled, is_default, icon, sort_order)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        rusqlite::params![
            "builtin_get_segment",
            "get_segment",
            "Get transcript segments within a specific time range",
            "builtin",
            r#"{"name":"get_segment","description":"Get transcript segments within a specific time range","parameters":{"type":"object","properties":{"start_time":{"type":"string","description":"Start time in format HH:MM:SS or MM:SS"},"end_time":{"type":"string","description":"End time in format HH:MM:SS or MM:SS"}},"required":["start_time","end_time"]}}"#,
            "backend",
            1,
            0,
            "Clock",
            4
        ],
    ).context("Failed to seed get_segment tool")?;

    log::info!("Built-in tools seeded successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_migrations() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let conn = Connection::open(&db_path).unwrap();

        // Run migrations
        run_migrations(&conn).unwrap();

        // Verify schema version
        let version: i32 = conn.query_row(
            "SELECT MAX(version) FROM schema_version",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(version, 1);

        // Verify categories were seeded
        let cat_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM categories WHERE is_system = 1",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(cat_count, 8);
    }
}
