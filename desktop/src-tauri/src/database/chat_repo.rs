// Chat repository for Meeting-Local
// Handles CRUD operations for chat messages

use anyhow::{Context, Result};
use rusqlite::{Connection, params};

use super::models::{ChatMessage, ChatRole, ChatMessageStatus, ChatConfig};
use super::DatabaseManager;

impl DatabaseManager {
    /// Save a chat message
    pub fn save_chat_message(&self, message: &ChatMessage) -> Result<()> {
        self.with_connection(|conn| {
            save_chat_message_impl(conn, message)
        })
    }

    /// Get all chat messages for a session
    pub fn get_chat_messages_by_session(&self, session_id: &str) -> Result<Vec<ChatMessage>> {
        self.with_connection(|conn| {
            get_chat_messages_by_session_impl(conn, session_id)
        })
    }

    /// Get all chat messages for a recording (legacy - for backwards compatibility)
    pub fn get_chat_messages(&self, recording_id: &str) -> Result<Vec<ChatMessage>> {
        self.with_connection(|conn| {
            get_chat_messages_impl(conn, recording_id)
        })
    }

    /// Get a single chat message by ID
    pub fn get_chat_message(&self, message_id: &str) -> Result<Option<ChatMessage>> {
        self.with_connection(|conn| {
            get_chat_message_impl(conn, message_id)
        })
    }

    /// Get the next sequence ID for a session's chat
    pub fn get_next_chat_sequence_id_for_session(&self, session_id: &str) -> Result<i64> {
        self.with_connection(|conn| {
            get_next_chat_sequence_id_for_session_impl(conn, session_id)
        })
    }

    /// Get the next sequence ID for a recording's chat (legacy)
    pub fn get_next_chat_sequence_id(&self, recording_id: &str) -> Result<i64> {
        self.with_connection(|conn| {
            get_next_chat_sequence_id_impl(conn, recording_id)
        })
    }

    /// Update the content of a chat message
    pub fn update_chat_message_content(&self, message_id: &str, content: &str) -> Result<()> {
        self.with_connection(|conn| {
            update_chat_message_content_impl(conn, message_id, content)
        })
    }

    /// Update the status of a chat message
    pub fn update_chat_message_status(
        &self,
        message_id: &str,
        status: ChatMessageStatus,
        error_message: Option<&str>,
    ) -> Result<()> {
        self.with_connection(|conn| {
            update_chat_message_status_impl(conn, message_id, status, error_message)
        })
    }

    /// Delete all chat messages for a session
    pub fn delete_chat_messages_by_session(&self, session_id: &str) -> Result<()> {
        self.with_connection(|conn| {
            delete_chat_messages_by_session_impl(conn, session_id)
        })
    }

    /// Delete all chat messages for a recording (legacy)
    pub fn delete_chat_messages(&self, recording_id: &str) -> Result<()> {
        self.with_connection(|conn| {
            delete_chat_messages_impl(conn, recording_id)
        })
    }

    /// Get pending or streaming messages (for resuming on app restart)
    pub fn get_pending_chat_messages(&self) -> Result<Vec<ChatMessage>> {
        self.with_connection(|conn| {
            get_pending_chat_messages_impl(conn)
        })
    }

    /// Get the chat config from a session
    pub fn get_session_chat_config(&self, session_id: &str) -> Result<Option<ChatConfig>> {
        self.with_connection(|conn| {
            get_session_chat_config_impl(conn, session_id)
        })
    }

    /// Get the chat config (provider/model) from the most recent assistant message (legacy)
    pub fn get_chat_config(&self, recording_id: &str) -> Result<Option<ChatConfig>> {
        self.with_connection(|conn| {
            get_chat_config_impl(conn, recording_id)
        })
    }
}

fn save_chat_message_impl(conn: &Connection, message: &ChatMessage) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO chat_messages (
            id, recording_id, session_id, role, content, created_at,
            sequence_id, status, error_message, provider_type, model_id
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        ON CONFLICT(id) DO UPDATE SET
            content = excluded.content,
            status = excluded.status,
            error_message = excluded.error_message,
            provider_type = excluded.provider_type,
            model_id = excluded.model_id
        "#,
        params![
            message.id,
            message.recording_id,
            message.session_id,
            message.role.as_str(),
            message.content,
            message.created_at,
            message.sequence_id,
            message.status.as_str(),
            message.error_message,
            message.provider_type,
            message.model_id,
        ],
    ).context("Failed to save chat message")?;

    Ok(())
}

fn get_chat_messages_by_session_impl(conn: &Connection, session_id: &str) -> Result<Vec<ChatMessage>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, recording_id, session_id, role, content, created_at,
               sequence_id, status, error_message, provider_type, model_id
        FROM chat_messages
        WHERE session_id = ?
        ORDER BY sequence_id ASC
        "#
    ).context("Failed to prepare get_chat_messages_by_session query")?;

    let messages = stmt.query_map(params![session_id], |row| {
        Ok(ChatMessage {
            id: row.get(0)?,
            recording_id: row.get(1)?,
            session_id: row.get(2)?,
            role: ChatRole::from_str(&row.get::<_, String>(3)?),
            content: row.get(4)?,
            created_at: row.get(5)?,
            sequence_id: row.get(6)?,
            status: ChatMessageStatus::from_str(&row.get::<_, String>(7)?),
            error_message: row.get(8)?,
            provider_type: row.get(9)?,
            model_id: row.get(10)?,
        })
    }).context("Failed to query chat messages")?;

    messages.collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to collect chat messages")
}

fn get_chat_messages_impl(conn: &Connection, recording_id: &str) -> Result<Vec<ChatMessage>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, recording_id, session_id, role, content, created_at,
               sequence_id, status, error_message, provider_type, model_id
        FROM chat_messages
        WHERE recording_id = ?
        ORDER BY sequence_id ASC
        "#
    ).context("Failed to prepare get_chat_messages query")?;

    let messages = stmt.query_map(params![recording_id], |row| {
        Ok(ChatMessage {
            id: row.get(0)?,
            recording_id: row.get(1)?,
            session_id: row.get(2)?,
            role: ChatRole::from_str(&row.get::<_, String>(3)?),
            content: row.get(4)?,
            created_at: row.get(5)?,
            sequence_id: row.get(6)?,
            status: ChatMessageStatus::from_str(&row.get::<_, String>(7)?),
            error_message: row.get(8)?,
            provider_type: row.get(9)?,
            model_id: row.get(10)?,
        })
    }).context("Failed to query chat messages")?;

    messages.collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to collect chat messages")
}

fn get_chat_message_impl(conn: &Connection, message_id: &str) -> Result<Option<ChatMessage>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, recording_id, session_id, role, content, created_at,
               sequence_id, status, error_message, provider_type, model_id
        FROM chat_messages
        WHERE id = ?
        "#
    ).context("Failed to prepare get_chat_message query")?;

    let result = stmt.query_row(params![message_id], |row| {
        Ok(ChatMessage {
            id: row.get(0)?,
            recording_id: row.get(1)?,
            session_id: row.get(2)?,
            role: ChatRole::from_str(&row.get::<_, String>(3)?),
            content: row.get(4)?,
            created_at: row.get(5)?,
            sequence_id: row.get(6)?,
            status: ChatMessageStatus::from_str(&row.get::<_, String>(7)?),
            error_message: row.get(8)?,
            provider_type: row.get(9)?,
            model_id: row.get(10)?,
        })
    });

    match result {
        Ok(msg) => Ok(Some(msg)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("Failed to get chat message"),
    }
}

fn get_next_chat_sequence_id_for_session_impl(conn: &Connection, session_id: &str) -> Result<i64> {
    let max_seq: Option<i64> = conn.query_row(
        "SELECT MAX(sequence_id) FROM chat_messages WHERE session_id = ?",
        params![session_id],
        |row| row.get(0),
    ).context("Failed to get max sequence_id for session")?;

    Ok(max_seq.unwrap_or(0) + 1)
}

fn get_next_chat_sequence_id_impl(conn: &Connection, recording_id: &str) -> Result<i64> {
    let max_seq: Option<i64> = conn.query_row(
        "SELECT MAX(sequence_id) FROM chat_messages WHERE recording_id = ?",
        params![recording_id],
        |row| row.get(0),
    ).context("Failed to get max sequence_id")?;

    Ok(max_seq.unwrap_or(0) + 1)
}

fn update_chat_message_content_impl(conn: &Connection, message_id: &str, content: &str) -> Result<()> {
    conn.execute(
        "UPDATE chat_messages SET content = ? WHERE id = ?",
        params![content, message_id],
    ).context("Failed to update chat message content")?;

    Ok(())
}

fn update_chat_message_status_impl(
    conn: &Connection,
    message_id: &str,
    status: ChatMessageStatus,
    error_message: Option<&str>,
) -> Result<()> {
    conn.execute(
        "UPDATE chat_messages SET status = ?, error_message = ? WHERE id = ?",
        params![status.as_str(), error_message, message_id],
    ).context("Failed to update chat message status")?;

    Ok(())
}

fn delete_chat_messages_by_session_impl(conn: &Connection, session_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM chat_messages WHERE session_id = ?",
        params![session_id],
    ).context("Failed to delete chat messages for session")?;

    Ok(())
}

fn delete_chat_messages_impl(conn: &Connection, recording_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM chat_messages WHERE recording_id = ?",
        params![recording_id],
    ).context("Failed to delete chat messages")?;

    Ok(())
}

fn get_pending_chat_messages_impl(conn: &Connection) -> Result<Vec<ChatMessage>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, recording_id, session_id, role, content, created_at,
               sequence_id, status, error_message, provider_type, model_id
        FROM chat_messages
        WHERE status IN ('pending', 'streaming')
        ORDER BY created_at ASC
        "#
    ).context("Failed to prepare get_pending_chat_messages query")?;

    let messages = stmt.query_map([], |row| {
        Ok(ChatMessage {
            id: row.get(0)?,
            recording_id: row.get(1)?,
            session_id: row.get(2)?,
            role: ChatRole::from_str(&row.get::<_, String>(3)?),
            content: row.get(4)?,
            created_at: row.get(5)?,
            sequence_id: row.get(6)?,
            status: ChatMessageStatus::from_str(&row.get::<_, String>(7)?),
            error_message: row.get(8)?,
            provider_type: row.get(9)?,
            model_id: row.get(10)?,
        })
    }).context("Failed to query pending chat messages")?;

    messages.collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to collect pending chat messages")
}

fn get_session_chat_config_impl(conn: &Connection, session_id: &str) -> Result<Option<ChatConfig>> {
    // Get config from the session itself (not from messages)
    let result = conn.query_row(
        r#"
        SELECT provider_type, model_id
        FROM chat_sessions
        WHERE id = ?
        "#,
        params![session_id],
        |row| {
            Ok(ChatConfig {
                provider_type: row.get(0)?,
                model_id: row.get(1)?,
            })
        },
    );

    match result {
        Ok(config) => {
            // Only return if at least one field is set
            if config.provider_type.is_some() || config.model_id.is_some() {
                Ok(Some(config))
            } else {
                Ok(None)
            }
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("Failed to get session chat config"),
    }
}

fn get_chat_config_impl(conn: &Connection, recording_id: &str) -> Result<Option<ChatConfig>> {
    // Get the most recent assistant message with provider/model info
    let result = conn.query_row(
        r#"
        SELECT provider_type, model_id
        FROM chat_messages
        WHERE recording_id = ?
          AND role = 'assistant'
          AND provider_type IS NOT NULL
        ORDER BY sequence_id DESC
        LIMIT 1
        "#,
        params![recording_id],
        |row| {
            Ok(ChatConfig {
                provider_type: row.get(0)?,
                model_id: row.get(1)?,
            })
        },
    );

    match result {
        Ok(config) => Ok(Some(config)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("Failed to get chat config"),
    }
}
