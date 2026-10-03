pub mod daily;
pub mod sessions;

use std::path::PathBuf;

use anyhow::{Context, Result};
use rusqlite::Connection;

/// Open or create the database, enable WAL mode, and run migrations.
pub fn init(db_path: &std::path::Path) -> Result<Connection> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).context("failed to create database directory")?;
    }
    let conn = Connection::open(db_path).context("failed to open database")?;
    conn.execute_batch("PRAGMA journal_mode = WAL;")
        .context("failed to enable WAL mode")?;
    migrate(&conn).context("failed to run migrations")?;
    Ok(conn)
}

/// Create an in-memory database for testing (runs migrations).
pub fn create_in_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch("PRAGMA journal_mode = WAL;")?;
    migrate(&conn)?;
    Ok(conn)
}

/// Run migrations to create tables if they don't exist.
pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS sessions (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            date            TEXT NOT NULL,
            start_time      TEXT NOT NULL,
            end_time        TEXT,
            duration_seconds INTEGER,
            session_type    TEXT NOT NULL CHECK(session_type IN ('study', 'short_break', 'endless_break'))
        );

        CREATE INDEX IF NOT EXISTS idx_sessions_date ON sessions(date);
        CREATE INDEX IF NOT EXISTS idx_sessions_type ON sessions(session_type);

        CREATE TABLE IF NOT EXISTS daily_sessions (
            date            TEXT PRIMARY KEY,
            total_seconds   INTEGER NOT NULL DEFAULT 0,
            created_at      TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_daily_sessions_date ON daily_sessions(date);",
    )?;
    Ok(())
}

/// Return the path to the database file.
pub fn db_path() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("pausa").join("pausa.db")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_creates_db() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("test.db");
        let conn = init(&path).unwrap();
        // Verify tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(tables.contains(&"daily_sessions".to_string()));
        assert!(tables.contains(&"sessions".to_string()));
    }

    #[test]
    fn test_create_in_memory_has_tables() {
        let conn = create_in_memory().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table'", [], |row| {
                row.get(0)
            })
            .unwrap();
        // sessions + daily_sessions (sqlite_sequence may also exist due to AUTOINCREMENT)
        assert!(count >= 2, "expected at least 2 user tables, found {count}");
    }

    #[test]
    fn test_db_path_ends_correctly() {
        let path = db_path();
        let filename = path.file_name().unwrap().to_str().unwrap();
        assert_eq!(filename, "pausa.db");
    }
}
