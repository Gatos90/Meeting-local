// Chat session repository for Meeting-Local
// Handles CRUD operations for chat sessions

use anyhow::{Context, Result};
use rusqlite::{Connection, params};

use super::models::ChatSession;
use super::DatabaseManager;

impl DatabaseManager {
    /// Create a new chat session
    pub fn create_chat_session(&self, session: &ChatSession) -> Result<String> {
        self.with_connection(|conn| {
            create_chat_session_impl(conn, session)
        })
    }

    /// Get all chat sessions for a recording (newest first)
    pub fn get_chat_sessions(&self, recording_id: &str) -> Result<Vec<ChatSession>> {
        self.with_connection(|conn| {
            get_chat_sessions_impl(conn, recording_id)
        })
    }

    /// Get a single chat session by ID
    pub fn get_chat_session(&self, session_id: &str) -> Result<Option<ChatSession>> {
        self.with_connection(|conn| {
            get_chat_session_impl(conn, session_id)
        })
    }

    /// Get the most recent chat session for a recording
    pub fn get_latest_chat_session(&self, recording_id: &str) -> Result<Option<ChatSession>> {
        self.with_connection(|conn| {
            get_latest_chat_session_impl(conn, recording_id)
        })
    }

    /// Update a chat session's provider/model config
    pub fn update_chat_session_config(
        &self,
        session_id: &str,
        provider_type: Option<&str>,
        model_id: Option<&str>,
    ) -> Result<()> {
        self.with_connection(|conn| {
            update_chat_session_config_impl(conn, session_id, provider_type, model_id)
        })
    }

    /// Update a chat session's title
    pub fn update_chat_session_title(&self, session_id: &str, title: &str) -> Result<()> {
        self.with_connection(|conn| {
            update_chat_session_title_impl(conn, session_id, title)
        })
    }

    /// Delete a chat session and all its messages
    pub fn delete_chat_session(&self, session_id: &str) -> Result<()> {
        self.with_connection(|conn| {
            delete_chat_session_impl(conn, session_id)
        })
    }

    /// Get or create a session for a recording (creates if none exist)
    pub fn get_or_create_chat_session(&self, recording_id: &str) -> Result<ChatSession> {
        self.with_connection(|conn| {
            // First try to get the latest session
            if let Some(session) = get_latest_chat_session_impl(conn, recording_id)? {
                return Ok(session);
            }

            // No sessions exist, create one
            let session = ChatSession::new(recording_id, "New Chat");
            create_chat_session_impl(conn, &session)?;
            Ok(session)
        })
    }
}

fn create_chat_session_impl(conn: &Connection, session: &ChatSession) -> Result<String> {
    conn.execute(
        r#"
        INSERT INTO chat_sessions (id, recording_id, title, created_at, provider_type, model_id)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        params![
            session.id,
            session.recording_id,
            session.title,
            session.created_at,
            session.provider_type,
            session.model_id,
        ],
    ).context("Failed to create chat session")?;

    Ok(session.id.clone())
}

fn get_chat_sessions_impl(conn: &Connection, recording_id: &str) -> Result<Vec<ChatSession>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, recording_id, title, created_at, provider_type, model_id
        FROM chat_sessions
        WHERE recording_id = ?
        ORDER BY created_at DESC
        "#
    ).context("Failed to prepare get_chat_sessions query")?;

    let sessions = stmt.query_map(params![recording_id], |row| {
        Ok(ChatSession {
            id: row.get(0)?,
            recording_id: row.get(1)?,
            title: row.get(2)?,
            created_at: row.get(3)?,
            provider_type: row.get(4)?,
            model_id: row.get(5)?,
        })
    }).context("Failed to query chat sessions")?;

    sessions.collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to collect chat sessions")
}

fn get_chat_session_impl(conn: &Connection, session_id: &str) -> Result<Option<ChatSession>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, recording_id, title, created_at, provider_type, model_id
        FROM chat_sessions
        WHERE id = ?
        "#
    ).context("Failed to prepare get_chat_session query")?;

    let result = stmt.query_row(params![session_id], |row| {
        Ok(ChatSession {
            id: row.get(0)?,
            recording_id: row.get(1)?,
            title: row.get(2)?,
            created_at: row.get(3)?,
            provider_type: row.get(4)?,
            model_id: row.get(5)?,
        })
    });

    match result {
        Ok(session) => Ok(Some(session)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("Failed to get chat session"),
    }
}

fn get_latest_chat_session_impl(conn: &Connection, recording_id: &str) -> Result<Option<ChatSession>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, recording_id, title, created_at, provider_type, model_id
        FROM chat_sessions
        WHERE recording_id = ?
        ORDER BY created_at DESC
        LIMIT 1
        "#
    ).context("Failed to prepare get_latest_chat_session query")?;

    let result = stmt.query_row(params![recording_id], |row| {
        Ok(ChatSession {
            id: row.get(0)?,
            recording_id: row.get(1)?,
            title: row.get(2)?,
            created_at: row.get(3)?,
            provider_type: row.get(4)?,
            model_id: row.get(5)?,
        })
    });

    match result {
        Ok(session) => Ok(Some(session)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("Failed to get latest chat session"),
    }
}

fn update_chat_session_config_impl(
    conn: &Connection,
    session_id: &str,
    provider_type: Option<&str>,
    model_id: Option<&str>,
) -> Result<()> {
    conn.execute(
        "UPDATE chat_sessions SET provider_type = ?, model_id = ? WHERE id = ?",
        params![provider_type, model_id, session_id],
    ).context("Failed to update chat session config")?;

    Ok(())
}

fn update_chat_session_title_impl(conn: &Connection, session_id: &str, title: &str) -> Result<()> {
    conn.execute(
        "UPDATE chat_sessions SET title = ? WHERE id = ?",
        params![title, session_id],
    ).context("Failed to update chat session title")?;

    Ok(())
}

fn delete_chat_session_impl(conn: &Connection, session_id: &str) -> Result<()> {
    // Delete all messages in this session first
    conn.execute(
        "DELETE FROM chat_messages WHERE session_id = ?",
        params![session_id],
    ).context("Failed to delete chat messages for session")?;

    // Then delete the session
    conn.execute(
        "DELETE FROM chat_sessions WHERE id = ?",
        params![session_id],
    ).context("Failed to delete chat session")?;

    Ok(())
}
