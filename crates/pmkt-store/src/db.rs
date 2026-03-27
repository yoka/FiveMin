use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;

/// Open (or create) the SQLite database and run migrations.
pub fn open_database(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)
        .with_context(|| format!("opening database at {}", path.display()))?;
    crate::schema::run_migrations(&conn)?;
    Ok(conn)
}

/// Open an in-memory database (for testing).
pub fn open_in_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory()
        .with_context(|| "opening in-memory database")?;
    crate::schema::run_migrations(&conn)?;
    Ok(conn)
}
