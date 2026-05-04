use crate::ingest::types::{QuarantineEntry, QuarantineLog};
use rusqlite::{params, Connection};

/// Karantén log mentése az `esg_state` SQLite adatbázisba.
pub fn flush_quarantine(conn: &Connection, log: &QuarantineLog) -> rusqlite::Result<()> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS quarantine_log (
            id               INTEGER PRIMARY KEY AUTOINCREMENT,
            source_file      TEXT    NOT NULL,
            source_line      INTEGER NOT NULL,
            raw_content      TEXT,
            error_type       TEXT    NOT NULL,
            error_detail     TEXT    NOT NULL,
            quarantined_at   TEXT    NOT NULL
        );
    ")?;

    let mut stmt = conn.prepare("
        INSERT INTO quarantine_log
            (source_file, source_line, raw_content, error_type, error_detail, quarantined_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
    ")?;

    for entry in &log.entries {
        stmt.execute(params![
            entry.source_file,
            entry.source_line,
            entry.raw_content,
            format!("{:?}", std::mem::discriminant(&entry.error)),
            entry.error.to_string(),
            entry.quarantined_at.to_rfc3339(),
        ])?;
    }

    Ok(())
}
