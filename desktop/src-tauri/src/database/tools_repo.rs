// Tools repository for Meeting-Local
// Handles CRUD operations for AI tools (function calling)

use anyhow::{Context, Result};
use rusqlite::{Connection, params};

use super::models::{Tool, CreateTool, UpdateTool, ChatSessionTool};
use super::DatabaseManager;

impl DatabaseManager {
    /// Get all tools, ordered by sort_order then created_at
    pub fn list_tools(&self) -> Result<Vec<Tool>> {
        self.with_connection(|conn| {
            list_tools_impl(conn)
        })
    }

    /// Get enabled tools only
    pub fn list_enabled_tools(&self) -> Result<Vec<Tool>> {
        self.with_connection(|conn| {
            list_enabled_tools_impl(conn)
        })
    }

    /// Get tools marked as default (auto-included in new chats)
    pub fn list_default_tools(&self) -> Result<Vec<Tool>> {
        self.with_connection(|conn| {
            list_default_tools_impl(conn)
        })
    }

    /// Get a single tool by ID
    pub fn get_tool(&self, id: &str) -> Result<Option<Tool>> {
        self.with_connection(|conn| {
            get_tool_impl(conn, id)
        })
    }

    /// Create a new custom tool
    pub fn create_tool(&self, input: &CreateTool) -> Result<String> {
        self.with_connection(|conn| {
            create_tool_impl(conn, input)
        })
    }

    /// Update an existing tool (only custom tools can be fully updated)
    pub fn update_tool(&self, id: &str, input: &UpdateTool) -> Result<()> {
        self.with_connection(|conn| {
            update_tool_impl(conn, id, input)
        })
    }

    /// Delete a tool (only custom tools can be deleted)
    pub fn delete_tool(&self, id: &str) -> Result<()> {
        self.with_connection(|conn| {
            delete_tool_impl(conn, id)
        })
    }

    /// Set the default status of a tool
    pub fn set_tool_default(&self, id: &str, is_default: bool) -> Result<()> {
        self.with_connection(|conn| {
            set_tool_default_impl(conn, id, is_default)
        })
    }

    /// Get tools enabled for a specific chat session
    pub fn get_session_tools(&self, session_id: &str) -> Result<Vec<Tool>> {
        self.with_connection(|conn| {
            get_session_tools_impl(conn, session_id)
        })
    }

    /// Set the tools enabled for a chat session
    pub fn set_session_tools(&self, session_id: &str, tool_ids: &[String]) -> Result<()> {
        self.with_connection(|conn| {
            set_session_tools_impl(conn, session_id, tool_ids)
        })
    }

    /// Initialize default tools for a new session (copies default tools)
    pub fn init_session_tools(&self, session_id: &str) -> Result<()> {
        self.with_connection(|conn| {
            init_session_tools_impl(conn, session_id)
        })
    }

    /// Get the next sort order value for tools
    pub fn get_next_tool_sort_order(&self) -> Result<i32> {
        self.with_connection(|conn| {
            get_next_sort_order_impl(conn)
        })
    }

    /// Get tools by their IDs (for chat completion)
    pub fn get_tools_by_ids(&self, ids: &[String]) -> Result<Vec<Tool>> {
        self.with_connection(|conn| {
            get_tools_by_ids_impl(conn, ids)
        })
    }
}

fn list_tools_impl(conn: &Connection) -> Result<Vec<Tool>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT t.id, t.name, t.description, t.tool_type, t.function_schema, t.execution_location,
               t.enabled, t.is_default, t.icon, t.sort_order, t.created_at,
               t.mcp_server_id, ms.name as mcp_server_name
        FROM tools t
        LEFT JOIN mcp_servers ms ON t.mcp_server_id = ms.id
        ORDER BY t.sort_order ASC, t.created_at ASC
        "#
    ).context("Failed to prepare list_tools query")?;

    let tools = stmt.query_map([], |row| {
        Ok(Tool {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            tool_type: row.get(3)?,
            function_schema: row.get(4)?,
            execution_location: row.get(5)?,
            enabled: row.get::<_, i32>(6)? != 0,
            is_default: row.get::<_, i32>(7)? != 0,
            icon: row.get(8)?,
            sort_order: row.get(9)?,
            created_at: row.get(10)?,
            mcp_server_id: row.get(11)?,
            mcp_server_name: row.get(12)?,
        })
    }).context("Failed to query tools")?;

    tools.collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to collect tools")
}

fn list_enabled_tools_impl(conn: &Connection) -> Result<Vec<Tool>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT t.id, t.name, t.description, t.tool_type, t.function_schema, t.execution_location,
               t.enabled, t.is_default, t.icon, t.sort_order, t.created_at,
               t.mcp_server_id, ms.name as mcp_server_name
        FROM tools t
        LEFT JOIN mcp_servers ms ON t.mcp_server_id = ms.id
        WHERE t.enabled = 1
        ORDER BY t.sort_order ASC, t.created_at ASC
        "#
    ).context("Failed to prepare list_enabled_tools query")?;

    let tools = stmt.query_map([], |row| {
        Ok(Tool {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            tool_type: row.get(3)?,
            function_schema: row.get(4)?,
            execution_location: row.get(5)?,
            enabled: row.get::<_, i32>(6)? != 0,
            is_default: row.get::<_, i32>(7)? != 0,
            icon: row.get(8)?,
            sort_order: row.get(9)?,
            created_at: row.get(10)?,
            mcp_server_id: row.get(11)?,
            mcp_server_name: row.get(12)?,
        })
    }).context("Failed to query enabled tools")?;

    tools.collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to collect enabled tools")
}

fn list_default_tools_impl(conn: &Connection) -> Result<Vec<Tool>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT t.id, t.name, t.description, t.tool_type, t.function_schema, t.execution_location,
               t.enabled, t.is_default, t.icon, t.sort_order, t.created_at,
               t.mcp_server_id, ms.name as mcp_server_name
        FROM tools t
        LEFT JOIN mcp_servers ms ON t.mcp_server_id = ms.id
        WHERE t.is_default = 1 AND t.enabled = 1
        ORDER BY t.sort_order ASC, t.created_at ASC
        "#
    ).context("Failed to prepare list_default_tools query")?;

    let tools = stmt.query_map([], |row| {
        Ok(Tool {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            tool_type: row.get(3)?,
            function_schema: row.get(4)?,
            execution_location: row.get(5)?,
            enabled: row.get::<_, i32>(6)? != 0,
            is_default: row.get::<_, i32>(7)? != 0,
            icon: row.get(8)?,
            sort_order: row.get(9)?,
            created_at: row.get(10)?,
            mcp_server_id: row.get(11)?,
            mcp_server_name: row.get(12)?,
        })
    }).context("Failed to query default tools")?;

    tools.collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to collect default tools")
}

fn get_tool_impl(conn: &Connection, id: &str) -> Result<Option<Tool>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT t.id, t.name, t.description, t.tool_type, t.function_schema, t.execution_location,
               t.enabled, t.is_default, t.icon, t.sort_order, t.created_at,
               t.mcp_server_id, ms.name as mcp_server_name
        FROM tools t
        LEFT JOIN mcp_servers ms ON t.mcp_server_id = ms.id
        WHERE t.id = ?
        "#
    ).context("Failed to prepare get_tool query")?;

    let result = stmt.query_row(params![id], |row| {
        Ok(Tool {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            tool_type: row.get(3)?,
            function_schema: row.get(4)?,
            execution_location: row.get(5)?,
            enabled: row.get::<_, i32>(6)? != 0,
            is_default: row.get::<_, i32>(7)? != 0,
            icon: row.get(8)?,
            sort_order: row.get(9)?,
            created_at: row.get(10)?,
            mcp_server_id: row.get(11)?,
            mcp_server_name: row.get(12)?,
        })
    });

    match result {
        Ok(tool) => Ok(Some(tool)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("Failed to get tool"),
    }
}

fn create_tool_impl(conn: &Connection, input: &CreateTool) -> Result<String> {
    let id = format!("custom_{}", uuid::Uuid::new_v4());
    let now = chrono::Utc::now().to_rfc3339();
    let execution_location = input.execution_location.as_deref().unwrap_or("backend");
    let sort_order = get_next_sort_order_impl(conn)?;

    conn.execute(
        r#"
        INSERT INTO tools (id, name, description, tool_type, function_schema, execution_location,
                          enabled, is_default, icon, sort_order, created_at)
        VALUES (?1, ?2, ?3, 'custom', ?4, ?5, 1, 0, ?6, ?7, ?8)
        "#,
        params![
            id,
            input.name,
            input.description,
            input.function_schema,
            execution_location,
            input.icon,
            sort_order,
            now,
        ],
    ).context("Failed to create tool")?;

    Ok(id)
}

fn update_tool_impl(conn: &Connection, id: &str, input: &UpdateTool) -> Result<()> {
    // First check if the tool exists and get its type
    let tool_type: String = conn.query_row(
        "SELECT tool_type FROM tools WHERE id = ?",
        params![id],
        |row| row.get(0),
    ).context("Tool not found")?;

    // Built-in tools can only have enabled and is_default updated
    let is_builtin = tool_type == "builtin";

    // Build dynamic update query
    let mut updates = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    // Only allow full updates for non-builtin tools
    if !is_builtin {
        if let Some(ref name) = input.name {
            updates.push("name = ?");
            values.push(Box::new(name.clone()));
        }
        if let Some(ref description) = input.description {
            updates.push("description = ?");
            values.push(Box::new(description.clone()));
        }
        if let Some(ref function_schema) = input.function_schema {
            updates.push("function_schema = ?");
            values.push(Box::new(function_schema.clone()));
        }
        if let Some(ref execution_location) = input.execution_location {
            updates.push("execution_location = ?");
            values.push(Box::new(execution_location.clone()));
        }
        if let Some(ref icon) = input.icon {
            updates.push("icon = ?");
            values.push(Box::new(icon.clone()));
        }
        if let Some(sort_order) = input.sort_order {
            updates.push("sort_order = ?");
            values.push(Box::new(sort_order));
        }
    }

    // These can be updated for any tool type
    if let Some(enabled) = input.enabled {
        updates.push("enabled = ?");
        values.push(Box::new(if enabled { 1 } else { 0 }));
    }
    if let Some(is_default) = input.is_default {
        updates.push("is_default = ?");
        values.push(Box::new(if is_default { 1 } else { 0 }));
    }

    if updates.is_empty() {
        return Ok(()); // Nothing to update
    }

    let query = format!(
        "UPDATE tools SET {} WHERE id = ?",
        updates.join(", ")
    );
    values.push(Box::new(id.to_string()));

    let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    conn.execute(&query, params.as_slice()).context("Failed to update tool")?;

    Ok(())
}

fn delete_tool_impl(conn: &Connection, id: &str) -> Result<()> {
    // First check if the tool exists and is custom
    let tool_type: String = conn.query_row(
        "SELECT tool_type FROM tools WHERE id = ?",
        params![id],
        |row| row.get(0),
    ).context("Tool not found")?;

    if tool_type != "custom" {
        return Err(anyhow::anyhow!("Cannot delete built-in or MCP tools"));
    }

    // Delete from chat_session_tools first (cascading)
    conn.execute(
        "DELETE FROM chat_session_tools WHERE tool_id = ?",
        params![id],
    ).context("Failed to delete tool associations")?;

    // Delete the tool
    conn.execute(
        "DELETE FROM tools WHERE id = ? AND tool_type = 'custom'",
        params![id],
    ).context("Failed to delete tool")?;

    Ok(())
}

fn set_tool_default_impl(conn: &Connection, id: &str, is_default: bool) -> Result<()> {
    conn.execute(
        "UPDATE tools SET is_default = ? WHERE id = ?",
        params![if is_default { 1 } else { 0 }, id],
    ).context("Failed to update tool default status")?;

    Ok(())
}

fn get_session_tools_impl(conn: &Connection, session_id: &str) -> Result<Vec<Tool>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT t.id, t.name, t.description, t.tool_type, t.function_schema, t.execution_location,
               t.enabled, t.is_default, t.icon, t.sort_order, t.created_at,
               t.mcp_server_id, ms.name as mcp_server_name
        FROM tools t
        INNER JOIN chat_session_tools cst ON t.id = cst.tool_id
        LEFT JOIN mcp_servers ms ON t.mcp_server_id = ms.id
        WHERE cst.session_id = ? AND cst.enabled = 1 AND t.enabled = 1
        ORDER BY t.sort_order ASC, t.created_at ASC
        "#
    ).context("Failed to prepare get_session_tools query")?;

    let tools = stmt.query_map(params![session_id], |row| {
        Ok(Tool {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            tool_type: row.get(3)?,
            function_schema: row.get(4)?,
            execution_location: row.get(5)?,
            enabled: row.get::<_, i32>(6)? != 0,
            is_default: row.get::<_, i32>(7)? != 0,
            icon: row.get(8)?,
            sort_order: row.get(9)?,
            created_at: row.get(10)?,
            mcp_server_id: row.get(11)?,
            mcp_server_name: row.get(12)?,
        })
    }).context("Failed to query session tools")?;

    tools.collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to collect session tools")
}

fn set_session_tools_impl(conn: &Connection, session_id: &str, tool_ids: &[String]) -> Result<()> {
    // Delete existing associations
    conn.execute(
        "DELETE FROM chat_session_tools WHERE session_id = ?",
        params![session_id],
    ).context("Failed to clear session tools")?;

    // Insert new associations
    for tool_id in tool_ids {
        conn.execute(
            "INSERT INTO chat_session_tools (session_id, tool_id, enabled) VALUES (?, ?, 1)",
            params![session_id, tool_id],
        ).context("Failed to add session tool")?;
    }

    Ok(())
}

fn init_session_tools_impl(conn: &Connection, session_id: &str) -> Result<()> {
    // Get default tools and add them to the session
    let default_tools = list_default_tools_impl(conn)?;

    for tool in default_tools {
        conn.execute(
            "INSERT OR IGNORE INTO chat_session_tools (session_id, tool_id, enabled) VALUES (?, ?, 1)",
            params![session_id, tool.id],
        ).context("Failed to init session tool")?;
    }

    Ok(())
}

fn get_next_sort_order_impl(conn: &Connection) -> Result<i32> {
    let max_order: Option<i32> = conn.query_row(
        "SELECT MAX(sort_order) FROM tools",
        [],
        |row| row.get(0),
    ).context("Failed to get max sort_order")?;

    Ok(max_order.unwrap_or(0) + 1)
}

fn get_tools_by_ids_impl(conn: &Connection, ids: &[String]) -> Result<Vec<Tool>> {
    if ids.is_empty() {
        return Ok(vec![]);
    }

    // Build parameterized query with placeholders
    let placeholders: Vec<&str> = ids.iter().map(|_| "?").collect();
    let query = format!(
        r#"
        SELECT t.id, t.name, t.description, t.tool_type, t.function_schema, t.execution_location,
               t.enabled, t.is_default, t.icon, t.sort_order, t.created_at,
               t.mcp_server_id, ms.name as mcp_server_name
        FROM tools t
        LEFT JOIN mcp_servers ms ON t.mcp_server_id = ms.id
        WHERE t.id IN ({}) AND t.enabled = 1
        ORDER BY t.sort_order ASC, t.created_at ASC
        "#,
        placeholders.join(", ")
    );

    let mut stmt = conn.prepare(&query).context("Failed to prepare get_tools_by_ids query")?;

    let params: Vec<&dyn rusqlite::ToSql> = ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();

    let tools = stmt.query_map(params.as_slice(), |row| {
        Ok(Tool {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            tool_type: row.get(3)?,
            function_schema: row.get(4)?,
            execution_location: row.get(5)?,
            enabled: row.get::<_, i32>(6)? != 0,
            is_default: row.get::<_, i32>(7)? != 0,
            icon: row.get(8)?,
            sort_order: row.get(9)?,
            created_at: row.get(10)?,
            mcp_server_id: row.get(11)?,
            mcp_server_name: row.get(12)?,
        })
    }).context("Failed to query tools by ids")?;

    tools.collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to collect tools by ids")
}
