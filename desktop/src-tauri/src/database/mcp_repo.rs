// MCP Server repository for Meeting-Local
// Handles CRUD operations for MCP (Model Context Protocol) servers

use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use std::collections::HashMap;

use super::models::{McpServer, CreateMcpServer, UpdateMcpServer, McpServerConfig, McpServerStatus, McpServerWithTools, Tool};
use super::DatabaseManager;

impl DatabaseManager {
    /// Get all MCP servers, ordered by name
    pub fn list_mcp_servers(&self) -> Result<Vec<McpServer>> {
        self.with_connection(|conn| {
            list_mcp_servers_impl(conn)
        })
    }

    /// Get all MCP servers with their tool counts
    pub fn list_mcp_servers_with_tools(&self) -> Result<Vec<McpServerWithTools>> {
        self.with_connection(|conn| {
            list_mcp_servers_with_tools_impl(conn)
        })
    }

    /// Get MCP servers that should auto-start
    pub fn list_auto_start_mcp_servers(&self) -> Result<Vec<McpServer>> {
        self.with_connection(|conn| {
            list_auto_start_servers_impl(conn)
        })
    }

    /// Get a single MCP server by ID
    pub fn get_mcp_server(&self, id: &str) -> Result<Option<McpServer>> {
        self.with_connection(|conn| {
            get_mcp_server_impl(conn, id)
        })
    }

    /// Get a single MCP server by name
    pub fn get_mcp_server_by_name(&self, name: &str) -> Result<Option<McpServer>> {
        self.with_connection(|conn| {
            get_mcp_server_by_name_impl(conn, name)
        })
    }

    /// Create a new MCP server
    pub fn create_mcp_server(&self, input: &CreateMcpServer) -> Result<String> {
        self.with_connection(|conn| {
            create_mcp_server_impl(conn, input)
        })
    }

    /// Update an existing MCP server
    pub fn update_mcp_server(&self, id: &str, input: &UpdateMcpServer) -> Result<()> {
        self.with_connection(|conn| {
            update_mcp_server_impl(conn, id, input)
        })
    }

    /// Delete an MCP server and its associated tools
    pub fn delete_mcp_server(&self, id: &str) -> Result<()> {
        self.with_connection(|conn| {
            delete_mcp_server_impl(conn, id)
        })
    }

    /// Update server status (called when starting/stopping servers)
    pub fn update_mcp_server_status(
        &self,
        id: &str,
        status: McpServerStatus,
        error: Option<String>,
    ) -> Result<()> {
        self.with_connection(|conn| {
            update_server_status_impl(conn, id, status, error)
        })
    }

    /// Import MCP servers from standard config JSON format
    /// Format: { "server_name": { "command": "...", "args": [...], "env": {...} } }
    pub fn import_mcp_config(&self, config_json: &str) -> Result<Vec<String>> {
        self.with_connection(|conn| {
            import_mcp_config_impl(conn, config_json)
        })
    }

    /// Get tools discovered from an MCP server
    pub fn get_mcp_server_tools(&self, server_id: &str) -> Result<Vec<Tool>> {
        self.with_connection(|conn| {
            get_mcp_server_tools_impl(conn, server_id)
        })
    }

    /// Create a tool discovered from an MCP server
    pub fn create_mcp_tool(
        &self,
        server_id: &str,
        name: &str,
        description: Option<String>,
        function_schema: &str,
    ) -> Result<String> {
        self.with_connection(|conn| {
            create_mcp_tool_impl(conn, server_id, name, description, function_schema)
        })
    }

    /// Delete all tools for an MCP server (for refresh)
    pub fn delete_mcp_server_tools(&self, server_id: &str) -> Result<()> {
        self.with_connection(|conn| {
            delete_mcp_server_tools_impl(conn, server_id)
        })
    }

    /// Count tools for an MCP server
    pub fn count_mcp_server_tools(&self, server_id: &str) -> Result<i32> {
        self.with_connection(|conn| {
            count_mcp_server_tools_impl(conn, server_id)
        })
    }
}

fn list_mcp_servers_impl(conn: &Connection) -> Result<Vec<McpServer>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, name, command, args, env, working_directory,
               auto_start, enabled, status, last_error, created_at
        FROM mcp_servers
        ORDER BY name ASC
        "#
    ).context("Failed to prepare list_mcp_servers query")?;

    let servers = stmt.query_map([], |row| {
        Ok(McpServer {
            id: row.get(0)?,
            name: row.get(1)?,
            command: row.get(2)?,
            args: row.get(3)?,
            env: row.get(4)?,
            working_directory: row.get(5)?,
            auto_start: row.get::<_, i32>(6)? != 0,
            enabled: row.get::<_, i32>(7)? != 0,
            status: row.get(8)?,
            last_error: row.get(9)?,
            created_at: row.get(10)?,
        })
    }).context("Failed to query MCP servers")?;

    servers.collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to collect MCP servers")
}

fn list_mcp_servers_with_tools_impl(conn: &Connection) -> Result<Vec<McpServerWithTools>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT s.id, s.name, s.command, s.args, s.env, s.working_directory,
               s.auto_start, s.enabled, s.status, s.last_error, s.created_at,
               COALESCE((SELECT COUNT(*) FROM tools WHERE mcp_server_id = s.id), 0) as tool_count
        FROM mcp_servers s
        ORDER BY s.name ASC
        "#
    ).context("Failed to prepare list_mcp_servers_with_tools query")?;

    let servers = stmt.query_map([], |row| {
        Ok(McpServerWithTools {
            server: McpServer {
                id: row.get(0)?,
                name: row.get(1)?,
                command: row.get(2)?,
                args: row.get(3)?,
                env: row.get(4)?,
                working_directory: row.get(5)?,
                auto_start: row.get::<_, i32>(6)? != 0,
                enabled: row.get::<_, i32>(7)? != 0,
                status: row.get(8)?,
                last_error: row.get(9)?,
                created_at: row.get(10)?,
            },
            tool_count: row.get(11)?,
        })
    }).context("Failed to query MCP servers with tools")?;

    servers.collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to collect MCP servers with tools")
}

fn list_auto_start_servers_impl(conn: &Connection) -> Result<Vec<McpServer>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, name, command, args, env, working_directory,
               auto_start, enabled, status, last_error, created_at
        FROM mcp_servers
        WHERE auto_start = 1 AND enabled = 1
        ORDER BY name ASC
        "#
    ).context("Failed to prepare list_auto_start_servers query")?;

    let servers = stmt.query_map([], |row| {
        Ok(McpServer {
            id: row.get(0)?,
            name: row.get(1)?,
            command: row.get(2)?,
            args: row.get(3)?,
            env: row.get(4)?,
            working_directory: row.get(5)?,
            auto_start: row.get::<_, i32>(6)? != 0,
            enabled: row.get::<_, i32>(7)? != 0,
            status: row.get(8)?,
            last_error: row.get(9)?,
            created_at: row.get(10)?,
        })
    }).context("Failed to query auto-start MCP servers")?;

    servers.collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to collect auto-start MCP servers")
}

fn get_mcp_server_impl(conn: &Connection, id: &str) -> Result<Option<McpServer>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, name, command, args, env, working_directory,
               auto_start, enabled, status, last_error, created_at
        FROM mcp_servers
        WHERE id = ?
        "#
    ).context("Failed to prepare get_mcp_server query")?;

    let result = stmt.query_row(params![id], |row| {
        Ok(McpServer {
            id: row.get(0)?,
            name: row.get(1)?,
            command: row.get(2)?,
            args: row.get(3)?,
            env: row.get(4)?,
            working_directory: row.get(5)?,
            auto_start: row.get::<_, i32>(6)? != 0,
            enabled: row.get::<_, i32>(7)? != 0,
            status: row.get(8)?,
            last_error: row.get(9)?,
            created_at: row.get(10)?,
        })
    });

    match result {
        Ok(server) => Ok(Some(server)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("Failed to get MCP server"),
    }
}

fn get_mcp_server_by_name_impl(conn: &Connection, name: &str) -> Result<Option<McpServer>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, name, command, args, env, working_directory,
               auto_start, enabled, status, last_error, created_at
        FROM mcp_servers
        WHERE name = ?
        "#
    ).context("Failed to prepare get_mcp_server_by_name query")?;

    let result = stmt.query_row(params![name], |row| {
        Ok(McpServer {
            id: row.get(0)?,
            name: row.get(1)?,
            command: row.get(2)?,
            args: row.get(3)?,
            env: row.get(4)?,
            working_directory: row.get(5)?,
            auto_start: row.get::<_, i32>(6)? != 0,
            enabled: row.get::<_, i32>(7)? != 0,
            status: row.get(8)?,
            last_error: row.get(9)?,
            created_at: row.get(10)?,
        })
    });

    match result {
        Ok(server) => Ok(Some(server)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("Failed to get MCP server by name"),
    }
}

fn create_mcp_server_impl(conn: &Connection, input: &CreateMcpServer) -> Result<String> {
    // Check for duplicate name
    if get_mcp_server_by_name_impl(conn, &input.name)?.is_some() {
        return Err(anyhow::anyhow!("MCP server with name '{}' already exists", input.name));
    }

    let id = format!("mcp_{}", uuid::Uuid::new_v4());
    let now = chrono::Utc::now().to_rfc3339();
    let args_json = serde_json::to_string(&input.args).unwrap_or_else(|_| "[]".to_string());
    let env_json = serde_json::to_string(&input.env).unwrap_or_else(|_| "{}".to_string());

    conn.execute(
        r#"
        INSERT INTO mcp_servers (id, name, command, args, env, working_directory,
                                 auto_start, enabled, status, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, 'stopped', ?8)
        "#,
        params![
            id,
            input.name,
            input.command,
            args_json,
            env_json,
            input.working_directory,
            if input.auto_start { 1 } else { 0 },
            now,
        ],
    ).context("Failed to create MCP server")?;

    Ok(id)
}

fn update_mcp_server_impl(conn: &Connection, id: &str, input: &UpdateMcpServer) -> Result<()> {
    // First check if the server exists
    let existing = get_mcp_server_impl(conn, id)?;
    if existing.is_none() {
        return Err(anyhow::anyhow!("MCP server not found"));
    }

    // Check for duplicate name if name is being updated
    if let Some(ref new_name) = input.name {
        if let Some(other) = get_mcp_server_by_name_impl(conn, new_name)? {
            if other.id != id {
                return Err(anyhow::anyhow!("MCP server with name '{}' already exists", new_name));
            }
        }
    }

    // Build dynamic update query
    let mut updates = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(ref name) = input.name {
        updates.push("name = ?");
        values.push(Box::new(name.clone()));
    }
    if let Some(ref command) = input.command {
        updates.push("command = ?");
        values.push(Box::new(command.clone()));
    }
    if let Some(ref args) = input.args {
        updates.push("args = ?");
        values.push(Box::new(serde_json::to_string(args).unwrap_or_else(|_| "[]".to_string())));
    }
    if let Some(ref env) = input.env {
        updates.push("env = ?");
        values.push(Box::new(serde_json::to_string(env).unwrap_or_else(|_| "{}".to_string())));
    }
    if input.working_directory.is_some() {
        updates.push("working_directory = ?");
        values.push(Box::new(input.working_directory.clone()));
    }
    if let Some(auto_start) = input.auto_start {
        updates.push("auto_start = ?");
        values.push(Box::new(if auto_start { 1 } else { 0 }));
    }
    if let Some(enabled) = input.enabled {
        updates.push("enabled = ?");
        values.push(Box::new(if enabled { 1 } else { 0 }));
    }

    if updates.is_empty() {
        return Ok(()); // Nothing to update
    }

    let query = format!(
        "UPDATE mcp_servers SET {} WHERE id = ?",
        updates.join(", ")
    );
    values.push(Box::new(id.to_string()));

    let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    conn.execute(&query, params.as_slice()).context("Failed to update MCP server")?;

    Ok(())
}

fn delete_mcp_server_impl(conn: &Connection, id: &str) -> Result<()> {
    // Delete associated tools first (should cascade, but be explicit)
    delete_mcp_server_tools_impl(conn, id)?;

    // Delete the server
    let rows = conn.execute(
        "DELETE FROM mcp_servers WHERE id = ?",
        params![id],
    ).context("Failed to delete MCP server")?;

    if rows == 0 {
        return Err(anyhow::anyhow!("MCP server not found"));
    }

    Ok(())
}

fn update_server_status_impl(
    conn: &Connection,
    id: &str,
    status: McpServerStatus,
    error: Option<String>,
) -> Result<()> {
    conn.execute(
        "UPDATE mcp_servers SET status = ?, last_error = ? WHERE id = ?",
        params![status.as_str(), error, id],
    ).context("Failed to update MCP server status")?;

    Ok(())
}

fn import_mcp_config_impl(conn: &Connection, config_json: &str) -> Result<Vec<String>> {
    // Parse the config JSON
    let configs: HashMap<String, McpServerConfig> = serde_json::from_str(config_json)
        .context("Invalid MCP config JSON format")?;

    let mut created_ids = Vec::new();

    for (name, config) in configs {
        // Skip if server with this name already exists
        if get_mcp_server_by_name_impl(conn, &name)?.is_some() {
            log::info!("Skipping MCP server '{}' - already exists", name);
            continue;
        }

        let input = CreateMcpServer {
            name: name.clone(),
            command: config.command,
            args: config.args.unwrap_or_default(),
            env: config.env.unwrap_or_default(),
            working_directory: config.working_directory,
            auto_start: false, // Default to not auto-starting imported servers
        };

        match create_mcp_server_impl(conn, &input) {
            Ok(id) => {
                log::info!("Imported MCP server '{}' with id {}", name, id);
                created_ids.push(id);
            }
            Err(e) => {
                log::error!("Failed to import MCP server '{}': {}", name, e);
            }
        }
    }

    Ok(created_ids)
}

fn get_mcp_server_tools_impl(conn: &Connection, server_id: &str) -> Result<Vec<Tool>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT t.id, t.name, t.description, t.tool_type, t.function_schema, t.execution_location,
               t.enabled, t.is_default, t.icon, t.sort_order, t.created_at,
               t.mcp_server_id, ms.name as mcp_server_name
        FROM tools t
        LEFT JOIN mcp_servers ms ON t.mcp_server_id = ms.id
        WHERE t.mcp_server_id = ?
        ORDER BY t.name ASC
        "#
    ).context("Failed to prepare get_mcp_server_tools query")?;

    let tools = stmt.query_map(params![server_id], |row| {
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
    }).context("Failed to query MCP server tools")?;

    tools.collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to collect MCP server tools")
}

fn create_mcp_tool_impl(
    conn: &Connection,
    server_id: &str,
    name: &str,
    description: Option<String>,
    function_schema: &str,
) -> Result<String> {
    let id = format!("mcp_tool_{}", uuid::Uuid::new_v4());
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        r#"
        INSERT INTO tools (id, name, description, tool_type, function_schema, execution_location,
                          enabled, is_default, icon, sort_order, created_at, mcp_server_id)
        VALUES (?1, ?2, ?3, 'mcp', ?4, 'backend', 1, 0, 'Server', 0, ?5, ?6)
        "#,
        params![id, name, description, function_schema, now, server_id],
    ).context("Failed to create MCP tool")?;

    Ok(id)
}

fn delete_mcp_server_tools_impl(conn: &Connection, server_id: &str) -> Result<()> {
    // First delete from chat_session_tools
    conn.execute(
        r#"
        DELETE FROM chat_session_tools
        WHERE tool_id IN (SELECT id FROM tools WHERE mcp_server_id = ?)
        "#,
        params![server_id],
    ).context("Failed to delete MCP tool associations")?;

    // Then delete the tools
    conn.execute(
        "DELETE FROM tools WHERE mcp_server_id = ?",
        params![server_id],
    ).context("Failed to delete MCP server tools")?;

    Ok(())
}

fn count_mcp_server_tools_impl(conn: &Connection, server_id: &str) -> Result<i32> {
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM tools WHERE mcp_server_id = ?",
        params![server_id],
        |row| row.get(0),
    ).context("Failed to count MCP server tools")?;

    Ok(count)
}
