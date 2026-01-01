// Categories and Tags repository for Meeting-Local
// Handles CRUD operations for categories and tags, and their associations with recordings

use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use uuid::Uuid;

use super::models::{Category, Tag};
use super::DatabaseManager;

impl DatabaseManager {
    // ============ Categories ============

    /// Get all categories
    pub fn get_all_categories(&self) -> Result<Vec<Category>> {
        self.with_connection(|conn| {
            get_all_categories_impl(conn)
        })
    }

    /// Get a category by ID
    pub fn get_category(&self, id: &str) -> Result<Option<Category>> {
        self.with_connection(|conn| {
            get_category_impl(conn, id)
        })
    }

    /// Create a new category
    pub fn create_category(&self, name: &str, color: Option<&str>) -> Result<String> {
        self.with_connection(|conn| {
            create_category_impl(conn, name, color)
        })
    }

    /// Delete a category (only user-created categories can be deleted)
    pub fn delete_category(&self, id: &str) -> Result<()> {
        self.with_connection(|conn| {
            delete_category_impl(conn, id)
        })
    }

    /// Assign a category to a recording
    pub fn assign_category(&self, recording_id: &str, category_id: &str) -> Result<()> {
        self.with_connection(|conn| {
            assign_category_impl(conn, recording_id, category_id)
        })
    }

    /// Remove a category from a recording
    pub fn remove_category(&self, recording_id: &str, category_id: &str) -> Result<()> {
        self.with_connection(|conn| {
            remove_category_impl(conn, recording_id, category_id)
        })
    }

    // ============ Tags ============

    /// Get all tags
    pub fn get_all_tags(&self) -> Result<Vec<Tag>> {
        self.with_connection(|conn| {
            get_all_tags_impl(conn)
        })
    }

    /// Get a tag by ID
    pub fn get_tag(&self, id: &str) -> Result<Option<Tag>> {
        self.with_connection(|conn| {
            get_tag_impl(conn, id)
        })
    }

    /// Create a new tag
    pub fn create_tag(&self, name: &str, color: Option<&str>) -> Result<String> {
        self.with_connection(|conn| {
            create_tag_impl(conn, name, color)
        })
    }

    /// Delete a tag
    pub fn delete_tag(&self, id: &str) -> Result<()> {
        self.with_connection(|conn| {
            delete_tag_impl(conn, id)
        })
    }

    /// Assign a tag to a recording
    pub fn assign_tag(&self, recording_id: &str, tag_id: &str) -> Result<()> {
        self.with_connection(|conn| {
            assign_tag_impl(conn, recording_id, tag_id)
        })
    }

    /// Remove a tag from a recording
    pub fn remove_tag(&self, recording_id: &str, tag_id: &str) -> Result<()> {
        self.with_connection(|conn| {
            remove_tag_impl(conn, recording_id, tag_id)
        })
    }

    /// Get or create a tag by name
    pub fn get_or_create_tag(&self, name: &str, color: Option<&str>) -> Result<String> {
        self.with_connection(|conn| {
            get_or_create_tag_impl(conn, name, color)
        })
    }
}

// ============ Category Implementations ============

fn get_all_categories_impl(conn: &Connection) -> Result<Vec<Category>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, color, is_system FROM categories ORDER BY is_system DESC, name ASC"
    ).context("Failed to prepare get_all_categories query")?;

    let categories = stmt.query_map([], |row| {
        Ok(Category {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
            is_system: row.get::<_, i32>(3)? == 1,
        })
    }).context("Failed to query categories")?;

    categories.collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to collect categories")
}

fn get_category_impl(conn: &Connection, id: &str) -> Result<Option<Category>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, color, is_system FROM categories WHERE id = ?"
    ).context("Failed to prepare get_category query")?;

    let result = stmt.query_row(params![id], |row| {
        Ok(Category {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
            is_system: row.get::<_, i32>(3)? == 1,
        })
    });

    match result {
        Ok(category) => Ok(Some(category)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("Failed to get category"),
    }
}

fn create_category_impl(conn: &Connection, name: &str, color: Option<&str>) -> Result<String> {
    let id = format!("cat_{}", Uuid::new_v4().to_string().replace("-", "")[..12].to_string());

    conn.execute(
        "INSERT INTO categories (id, name, color, is_system) VALUES (?1, ?2, ?3, 0)",
        params![id, name, color],
    ).context("Failed to create category")?;

    Ok(id)
}

fn delete_category_impl(conn: &Connection, id: &str) -> Result<()> {
    // Only delete non-system categories
    conn.execute(
        "DELETE FROM categories WHERE id = ? AND is_system = 0",
        params![id],
    ).context("Failed to delete category")?;

    Ok(())
}

fn assign_category_impl(conn: &Connection, recording_id: &str, category_id: &str) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO recording_categories (recording_id, category_id) VALUES (?1, ?2)",
        params![recording_id, category_id],
    ).context("Failed to assign category")?;

    Ok(())
}

fn remove_category_impl(conn: &Connection, recording_id: &str, category_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM recording_categories WHERE recording_id = ? AND category_id = ?",
        params![recording_id, category_id],
    ).context("Failed to remove category")?;

    Ok(())
}

// ============ Tag Implementations ============

fn get_all_tags_impl(conn: &Connection) -> Result<Vec<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, color, usage_count FROM tags ORDER BY usage_count DESC, name ASC"
    ).context("Failed to prepare get_all_tags query")?;

    let tags = stmt.query_map([], |row| {
        Ok(Tag {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
            usage_count: row.get(3)?,
        })
    }).context("Failed to query tags")?;

    tags.collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to collect tags")
}

fn get_tag_impl(conn: &Connection, id: &str) -> Result<Option<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, color, usage_count FROM tags WHERE id = ?"
    ).context("Failed to prepare get_tag query")?;

    let result = stmt.query_row(params![id], |row| {
        Ok(Tag {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
            usage_count: row.get(3)?,
        })
    });

    match result {
        Ok(tag) => Ok(Some(tag)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("Failed to get tag"),
    }
}

fn create_tag_impl(conn: &Connection, name: &str, color: Option<&str>) -> Result<String> {
    let id = format!("tag_{}", Uuid::new_v4().to_string().replace("-", "")[..12].to_string());

    conn.execute(
        "INSERT INTO tags (id, name, color, usage_count) VALUES (?1, ?2, ?3, 0)",
        params![id, name, color],
    ).context("Failed to create tag")?;

    Ok(id)
}

fn delete_tag_impl(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM tags WHERE id = ?", params![id])
        .context("Failed to delete tag")?;

    Ok(())
}

fn assign_tag_impl(conn: &Connection, recording_id: &str, tag_id: &str) -> Result<()> {
    // Insert the association
    conn.execute(
        "INSERT OR IGNORE INTO recording_tags (recording_id, tag_id) VALUES (?1, ?2)",
        params![recording_id, tag_id],
    ).context("Failed to assign tag")?;

    // Increment usage count
    conn.execute(
        "UPDATE tags SET usage_count = usage_count + 1 WHERE id = ?",
        params![tag_id],
    ).context("Failed to update tag usage count")?;

    Ok(())
}

fn remove_tag_impl(conn: &Connection, recording_id: &str, tag_id: &str) -> Result<()> {
    let changes = conn.execute(
        "DELETE FROM recording_tags WHERE recording_id = ? AND tag_id = ?",
        params![recording_id, tag_id],
    ).context("Failed to remove tag")?;

    // Decrement usage count if we actually removed something
    if changes > 0 {
        conn.execute(
            "UPDATE tags SET usage_count = MAX(0, usage_count - 1) WHERE id = ?",
            params![tag_id],
        ).context("Failed to update tag usage count")?;
    }

    Ok(())
}

fn get_or_create_tag_impl(conn: &Connection, name: &str, color: Option<&str>) -> Result<String> {
    // Try to find existing tag
    let mut stmt = conn.prepare(
        "SELECT id FROM tags WHERE name = ? COLLATE NOCASE"
    ).context("Failed to prepare get_or_create_tag query")?;

    let result = stmt.query_row(params![name], |row| row.get::<_, String>(0));

    match result {
        Ok(id) => Ok(id),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            // Create new tag
            create_tag_impl(conn, name, color)
        }
        Err(e) => Err(e).context("Failed to get or create tag"),
    }
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
    fn test_predefined_categories_exist() {
        let db = create_test_db();

        let categories = db.get_all_categories().unwrap();
        assert!(categories.len() >= 8);

        let daily = categories.iter().find(|c| c.name == "Daily").unwrap();
        assert!(daily.is_system);
    }

    #[test]
    fn test_create_user_category() {
        let db = create_test_db();

        let id = db.create_category("Custom Category", Some("#FF0000")).unwrap();

        let category = db.get_category(&id).unwrap().unwrap();
        assert_eq!(category.name, "Custom Category");
        assert_eq!(category.color, Some("#FF0000".to_string()));
        assert!(!category.is_system);
    }

    #[test]
    fn test_create_and_assign_tag() {
        let db = create_test_db();

        // Create a recording
        let recording = Recording::new("rec_tag_test".to_string(), "Tag Test".to_string());
        db.create_recording(&recording).unwrap();

        // Create and assign a tag
        let tag_id = db.create_tag("Important", Some("#FF0000")).unwrap();
        db.assign_tag("rec_tag_test", &tag_id).unwrap();

        // Verify tag was assigned
        let tags = db.get_all_tags().unwrap();
        let tag = tags.iter().find(|t| t.id == tag_id).unwrap();
        assert_eq!(tag.usage_count, 1);
    }

    #[test]
    fn test_get_or_create_tag() {
        let db = create_test_db();

        let id1 = db.get_or_create_tag("Test Tag", Some("#00FF00")).unwrap();
        let id2 = db.get_or_create_tag("Test Tag", None).unwrap();

        assert_eq!(id1, id2); // Should return the same tag
    }
}
