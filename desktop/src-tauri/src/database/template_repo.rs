// Template repository for Meeting-Local
// Handles CRUD operations for prompt templates

use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use serde::Deserialize;
use std::path::Path;

use super::models::{PromptTemplate, CreatePromptTemplate, UpdatePromptTemplate};
use super::DatabaseManager;

/// JSON template section format (from templates/*.json files)
#[derive(Debug, Deserialize)]
struct TemplateSection {
    title: String,
    instruction: String,
    #[serde(default)]
    format: String,
    #[serde(default)]
    item_format: Option<String>,
}

/// JSON template file format
#[derive(Debug, Deserialize)]
struct JsonTemplate {
    name: String,
    description: String,
    sections: Vec<TemplateSection>,
}

impl DatabaseManager {
    /// Get all prompt templates, ordered by sort_order then created_at
    pub fn list_templates(&self) -> Result<Vec<PromptTemplate>> {
        self.with_connection(|conn| {
            list_templates_impl(conn)
        })
    }

    /// Get a single template by ID
    pub fn get_template(&self, id: &str) -> Result<Option<PromptTemplate>> {
        self.with_connection(|conn| {
            get_template_impl(conn, id)
        })
    }

    /// Create a new custom template
    pub fn create_template(&self, input: &CreatePromptTemplate) -> Result<String> {
        self.with_connection(|conn| {
            create_template_impl(conn, input)
        })
    }

    /// Update an existing template (only custom templates can be updated)
    pub fn update_template(&self, id: &str, input: &UpdatePromptTemplate) -> Result<()> {
        self.with_connection(|conn| {
            update_template_impl(conn, id, input)
        })
    }

    /// Delete a template (only custom templates can be deleted)
    pub fn delete_template(&self, id: &str) -> Result<()> {
        self.with_connection(|conn| {
            delete_template_impl(conn, id)
        })
    }

    /// Duplicate a template (creates a custom copy)
    pub fn duplicate_template(&self, id: &str) -> Result<String> {
        self.with_connection(|conn| {
            duplicate_template_impl(conn, id)
        })
    }

    /// Get the next sort order value
    pub fn get_next_template_sort_order(&self) -> Result<i32> {
        self.with_connection(|conn| {
            get_next_sort_order_impl(conn)
        })
    }

    /// Seed templates from JSON files in the templates directory
    pub fn seed_templates_from_folder(&self, templates_dir: &Path) -> Result<usize> {
        self.with_connection(|conn| {
            seed_templates_from_folder_impl(conn, templates_dir)
        })
    }
}

fn list_templates_impl(conn: &Connection) -> Result<Vec<PromptTemplate>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, name, description, prompt, icon, is_builtin, sort_order, created_at
        FROM prompt_templates
        ORDER BY sort_order ASC, created_at ASC
        "#
    ).context("Failed to prepare list_templates query")?;

    let templates = stmt.query_map([], |row| {
        Ok(PromptTemplate {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            prompt: row.get(3)?,
            icon: row.get(4)?,
            is_builtin: row.get::<_, i32>(5)? != 0,
            sort_order: row.get(6)?,
            created_at: row.get(7)?,
        })
    }).context("Failed to query templates")?;

    templates.collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to collect templates")
}

fn get_template_impl(conn: &Connection, id: &str) -> Result<Option<PromptTemplate>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, name, description, prompt, icon, is_builtin, sort_order, created_at
        FROM prompt_templates
        WHERE id = ?
        "#
    ).context("Failed to prepare get_template query")?;

    let result = stmt.query_row(params![id], |row| {
        Ok(PromptTemplate {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            prompt: row.get(3)?,
            icon: row.get(4)?,
            is_builtin: row.get::<_, i32>(5)? != 0,
            sort_order: row.get(6)?,
            created_at: row.get(7)?,
        })
    });

    match result {
        Ok(template) => Ok(Some(template)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("Failed to get template"),
    }
}

fn create_template_impl(conn: &Connection, input: &CreatePromptTemplate) -> Result<String> {
    let id = format!("custom_{}", uuid::Uuid::new_v4());
    let now = chrono::Utc::now().to_rfc3339();

    // Get sort_order - use provided or get next value
    let sort_order = match input.sort_order {
        Some(order) => order,
        None => get_next_sort_order_impl(conn)?,
    };

    conn.execute(
        r#"
        INSERT INTO prompt_templates (id, name, description, prompt, icon, is_builtin, sort_order, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7)
        "#,
        params![
            id,
            input.name,
            input.description,
            input.prompt,
            input.icon,
            sort_order,
            now,
        ],
    ).context("Failed to create template")?;

    Ok(id)
}

fn update_template_impl(conn: &Connection, id: &str, input: &UpdatePromptTemplate) -> Result<()> {
    // First check if the template exists and is not builtin
    let is_builtin: i32 = conn.query_row(
        "SELECT is_builtin FROM prompt_templates WHERE id = ?",
        params![id],
        |row| row.get(0),
    ).context("Template not found")?;

    if is_builtin != 0 {
        return Err(anyhow::anyhow!("Cannot update built-in templates"));
    }

    // Build dynamic update query
    let mut updates = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(ref name) = input.name {
        updates.push("name = ?");
        values.push(Box::new(name.clone()));
    }
    if let Some(ref description) = input.description {
        updates.push("description = ?");
        values.push(Box::new(description.clone()));
    }
    if let Some(ref prompt) = input.prompt {
        updates.push("prompt = ?");
        values.push(Box::new(prompt.clone()));
    }
    if let Some(ref icon) = input.icon {
        updates.push("icon = ?");
        values.push(Box::new(icon.clone()));
    }
    if let Some(sort_order) = input.sort_order {
        updates.push("sort_order = ?");
        values.push(Box::new(sort_order));
    }

    if updates.is_empty() {
        return Ok(()); // Nothing to update
    }

    let query = format!(
        "UPDATE prompt_templates SET {} WHERE id = ?",
        updates.join(", ")
    );
    values.push(Box::new(id.to_string()));

    let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    conn.execute(&query, params.as_slice()).context("Failed to update template")?;

    Ok(())
}

fn delete_template_impl(conn: &Connection, id: &str) -> Result<()> {
    // First check if the template exists and is not builtin
    let is_builtin: i32 = conn.query_row(
        "SELECT is_builtin FROM prompt_templates WHERE id = ?",
        params![id],
        |row| row.get(0),
    ).context("Template not found")?;

    if is_builtin != 0 {
        return Err(anyhow::anyhow!("Cannot delete built-in templates"));
    }

    conn.execute(
        "DELETE FROM prompt_templates WHERE id = ? AND is_builtin = 0",
        params![id],
    ).context("Failed to delete template")?;

    Ok(())
}

fn duplicate_template_impl(conn: &Connection, id: &str) -> Result<String> {
    // Get the original template
    let original = get_template_impl(conn, id)?
        .ok_or_else(|| anyhow::anyhow!("Template not found"))?;

    // Create a copy with a new ID and name
    let new_id = format!("custom_{}", uuid::Uuid::new_v4());
    let now = chrono::Utc::now().to_rfc3339();
    let new_name = format!("{} (Copy)", original.name);
    let sort_order = get_next_sort_order_impl(conn)?;

    conn.execute(
        r#"
        INSERT INTO prompt_templates (id, name, description, prompt, icon, is_builtin, sort_order, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7)
        "#,
        params![
            new_id,
            new_name,
            original.description,
            original.prompt,
            original.icon,
            sort_order,
            now,
        ],
    ).context("Failed to duplicate template")?;

    Ok(new_id)
}

fn get_next_sort_order_impl(conn: &Connection) -> Result<i32> {
    let max_order: Option<i32> = conn.query_row(
        "SELECT MAX(sort_order) FROM prompt_templates",
        [],
        |row| row.get(0),
    ).context("Failed to get max sort_order")?;

    Ok(max_order.unwrap_or(0) + 1)
}

/// Convert a JSON template to a prompt string
fn convert_template_to_prompt(template: &JsonTemplate) -> String {
    let mut prompt_parts = vec![
        format!("Please analyze this meeting transcript and create {} notes.", template.name),
        String::new(),
        "Structure your response with the following sections:".to_string(),
        String::new(),
    ];

    for (i, section) in template.sections.iter().enumerate() {
        prompt_parts.push(format!("## {} {}", i + 1, section.title));
        prompt_parts.push(section.instruction.clone());

        if let Some(ref item_format) = section.item_format {
            prompt_parts.push(format!("Use this format:\n{}", item_format));
        }
        prompt_parts.push(String::new());
    }

    prompt_parts.push("Please be thorough but concise, focusing on actionable information.".to_string());

    prompt_parts.join("\n")
}

/// Get an icon name based on template name
fn get_icon_for_template(name: &str) -> &'static str {
    let name_lower = name.to_lowercase();
    if name_lower.contains("standup") || name_lower.contains("daily") {
        "Users"
    } else if name_lower.contains("1:1") || name_lower.contains("one_on_one") || name_lower.contains("one-on-one") {
        "UserCheck"
    } else if name_lower.contains("interview") || name_lower.contains("debrief") {
        "UserPlus"
    } else if name_lower.contains("brainstorm") || name_lower.contains("ideation") {
        "Lightbulb"
    } else if name_lower.contains("retro") {
        "Target"
    } else if name_lower.contains("sync") || name_lower.contains("status") {
        "Calendar"
    } else if name_lower.contains("sales") || name_lower.contains("client") {
        "MessageSquare"
    } else if name_lower.contains("psych") || name_lower.contains("session") {
        "ClipboardList"
    } else {
        "FileText"
    }
}

fn seed_templates_from_folder_impl(conn: &Connection, templates_dir: &Path) -> Result<usize> {
    if !templates_dir.exists() {
        log::warn!("Templates directory does not exist: {:?}", templates_dir);
        return Ok(0);
    }

    let mut seeded_count = 0;
    let mut sort_order = get_next_sort_order_impl(conn)?;

    // Read all JSON files in the templates directory
    let entries = std::fs::read_dir(templates_dir)
        .context("Failed to read templates directory")?;

    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        // Skip non-JSON files
        if path.extension().map_or(true, |ext| ext != "json") {
            continue;
        }

        // Read and parse the JSON file
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Failed to read template file {:?}: {}", path, e);
                continue;
            }
        };

        let template: JsonTemplate = match serde_json::from_str(&content) {
            Ok(t) => t,
            Err(e) => {
                log::warn!("Failed to parse template file {:?}: {}", path, e);
                continue;
            }
        };

        // Generate a stable ID from the file name
        let file_stem = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let template_id = format!("builtin_{}", file_stem);

        // Check if this template already exists
        let exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM prompt_templates WHERE id = ?",
            params![&template_id],
            |row| row.get(0),
        ).unwrap_or(false);

        if exists {
            log::debug!("Template {} already exists, skipping", template_id);
            continue;
        }

        // Convert to prompt and insert
        let prompt = convert_template_to_prompt(&template);
        let icon = get_icon_for_template(&template.name);
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            r#"
            INSERT INTO prompt_templates (id, name, description, prompt, icon, is_builtin, sort_order, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?7)
            "#,
            params![
                template_id,
                template.name,
                template.description,
                prompt,
                icon,
                sort_order,
                now,
            ],
        ).context(format!("Failed to insert template {}", template_id))?;

        log::info!("Seeded template: {} ({})", template.name, template_id);
        seeded_count += 1;
        sort_order += 1;
    }

    Ok(seeded_count)
}
