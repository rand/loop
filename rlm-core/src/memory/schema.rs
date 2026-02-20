//! SQLite schema and migrations for the memory system.

use rusqlite::{Connection, Result as SqliteResult};

/// Current schema version.
pub const SCHEMA_VERSION: i32 = 1;

/// Initialize the database schema.
pub fn initialize_schema(conn: &Connection) -> SqliteResult<()> {
    // Enable WAL mode for better concurrent access
    conn.pragma_update(None, "journal_mode", "WAL")?;

    // Enable foreign keys
    conn.pragma_update(None, "foreign_keys", "ON")?;

    // Create schema version table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;

    // Check current version
    let current_version: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // Apply migrations
    if current_version < 1 {
        apply_v1_schema(conn)?;
    }

    Ok(())
}

/// Apply version 1 schema.
fn apply_v1_schema(conn: &Connection) -> SqliteResult<()> {
    // Nodes table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS nodes (
            id TEXT PRIMARY KEY,
            node_type TEXT NOT NULL,
            subtype TEXT,
            content TEXT NOT NULL,
            embedding BLOB,
            tier INTEGER NOT NULL DEFAULT 0,
            confidence REAL NOT NULL DEFAULT 1.0,
            provenance_source TEXT,
            provenance_ref TEXT,
            provenance_observed_at TEXT,
            provenance_context TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            last_accessed TEXT NOT NULL DEFAULT (datetime('now')),
            access_count INTEGER NOT NULL DEFAULT 0,
            metadata TEXT
        )",
        [],
    )?;

    // Hyperedges table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS hyperedges (
            id TEXT PRIMARY KEY,
            edge_type TEXT NOT NULL,
            label TEXT,
            weight REAL NOT NULL DEFAULT 1.0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            metadata TEXT
        )",
        [],
    )?;

    // Membership table (connects nodes to hyperedges)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS membership (
            hyperedge_id TEXT NOT NULL,
            node_id TEXT NOT NULL,
            role TEXT NOT NULL,
            position INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (hyperedge_id, node_id, role),
            FOREIGN KEY (hyperedge_id) REFERENCES hyperedges(id) ON DELETE CASCADE,
            FOREIGN KEY (node_id) REFERENCES nodes(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Evolution log table (tracks tier changes)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS evolution_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            node_id TEXT NOT NULL,
            operation TEXT NOT NULL,
            from_tier INTEGER,
            to_tier INTEGER,
            reason TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (node_id) REFERENCES nodes(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Indexes for common queries
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_nodes_type ON nodes(node_type)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_nodes_tier ON nodes(tier)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_nodes_confidence ON nodes(confidence)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_nodes_last_accessed ON nodes(last_accessed)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_membership_node ON membership(node_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_evolution_node ON evolution_log(node_id)",
        [],
    )?;

    // Full-text search on content
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS nodes_fts USING fts5(
            content,
            content='nodes',
            content_rowid='rowid'
        )",
        [],
    )?;

    // Triggers to keep FTS in sync
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS nodes_ai AFTER INSERT ON nodes BEGIN
            INSERT INTO nodes_fts(rowid, content) VALUES (NEW.rowid, NEW.content);
        END",
        [],
    )?;
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS nodes_ad AFTER DELETE ON nodes BEGIN
            INSERT INTO nodes_fts(nodes_fts, rowid, content) VALUES ('delete', OLD.rowid, OLD.content);
        END",
        [],
    )?;
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS nodes_au AFTER UPDATE ON nodes BEGIN
            INSERT INTO nodes_fts(nodes_fts, rowid, content) VALUES ('delete', OLD.rowid, OLD.content);
            INSERT INTO nodes_fts(rowid, content) VALUES (NEW.rowid, NEW.content);
        END",
        [],
    )?;

    // Record migration
    conn.execute("INSERT INTO schema_version (version) VALUES (1)", [])?;

    Ok(())
}

/// Get the current schema version.
pub fn get_schema_version(conn: &Connection) -> SqliteResult<i32> {
    conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )
}

/// Check if the schema is initialized.
pub fn is_initialized(conn: &Connection) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='nodes'",
        [],
        |row| row.get::<_, i32>(0),
    )
    .map(|count| count > 0)
    .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_schema() {
        let conn = Connection::open_in_memory().unwrap();
        initialize_schema(&conn).unwrap();

        assert!(is_initialized(&conn));
        assert_eq!(get_schema_version(&conn).unwrap(), 1);
    }

    #[test]
    fn test_idempotent_initialization() {
        let conn = Connection::open_in_memory().unwrap();

        // Initialize twice
        initialize_schema(&conn).unwrap();
        initialize_schema(&conn).unwrap();

        assert_eq!(get_schema_version(&conn).unwrap(), 1);
    }

    #[test]
    fn test_wal_mode() {
        let conn = Connection::open_in_memory().unwrap();
        initialize_schema(&conn).unwrap();

        let mode: String = conn
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .unwrap();
        // In-memory databases use "memory" mode, file databases would use "wal"
        assert!(mode == "memory" || mode == "wal");
    }
}
