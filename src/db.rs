use crate::models::{GhgScope, LedgerRow, QuarantineRow, Scope3Extension};
use anyhow::{anyhow, Result};
use chrono::Utc;
use rusqlite::{params, Connection, Transaction};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub type DbPool = Arc<Mutex<Connection>>;

/// Initialize the SQLite database connection and schema
pub fn init_db() -> Result<DbPool> {
    let conn = Connection::open("./targoo_v2.db")?;
    
    // Enable foreign keys and WAL mode for better concurrency
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;",
    )?;

    // Create runs table (metadata for each processing run)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS runs (
            run_id TEXT PRIMARY KEY,
            created_at TEXT NOT NULL,
            jurisdiction TEXT NOT NULL,
            language TEXT NOT NULL,
            industry TEXT NOT NULL,
            status TEXT NOT NULL
        )",
        [],
    )?;

    // Create ledger table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ledger (
            row_id TEXT PRIMARY KEY,
            run_id TEXT NOT NULL,
            source_file TEXT NOT NULL,
            raw_row_index INTEGER NOT NULL,
            raw_header TEXT NOT NULL,
            raw_value REAL NOT NULL,
            raw_unit TEXT NOT NULL,
            converted_value REAL NOT NULL,
            converted_unit TEXT NOT NULL,
            assumed_unit TEXT,
            ghg_scope TEXT NOT NULL,
            ghg_category TEXT NOT NULL,
            ghg_subcategory TEXT NOT NULL,
            emission_factor REAL NOT NULL,
            ef_source TEXT NOT NULL,
            ef_jurisdiction TEXT NOT NULL,
            gwp_applied REAL NOT NULL,
            tco2e REAL NOT NULL,
            confidence REAL NOT NULL,
            scope3_extension TEXT, -- JSON serialized Scope3Extension
            sha256_hash TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY(run_id) REFERENCES runs(run_id)
        )",
        [],
    )?;

    // Create quarantine table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS quarantine (
            row_id TEXT PRIMARY KEY,
            run_id TEXT NOT NULL,
            source_file TEXT NOT NULL,
            raw_row_index INTEGER NOT NULL,
            raw_header TEXT NOT NULL,
            raw_value TEXT NOT NULL,
            error_reason TEXT NOT NULL,
            suggested_fix TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY(run_id) REFERENCES runs(run_id)
        )",
        [],
    )?;

    // WORM Trigger for ledger (prevents UPDATE/DELETE)
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS ledger_worm_update
         BEFORE UPDATE ON ledger
         BEGIN
            SELECT RAISE(FAIL, 'UPDATE forbidden on ledger table (WORM)');
         END;",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS ledger_worm_delete
         BEFORE DELETE ON ledger
         BEGIN
            SELECT RAISE(FAIL, 'DELETE forbidden on ledger table (WORM)');
         END;",
        [],
    )?;

    // WORM Trigger for quarantine
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS quarantine_worm_update
         BEFORE UPDATE ON quarantine
         BEGIN
            SELECT RAISE(FAIL, 'UPDATE forbidden on quarantine table (WORM)');
         END;",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS quarantine_worm_delete
         BEFORE DELETE ON quarantine
         BEGIN
            SELECT RAISE(FAIL, 'DELETE forbidden on quarantine table (WORM)');
         END;",
        [],
    )?;

    Ok(Arc::new(Mutex::new(conn)))
}

/// Inserts a LedgerRow into the database within a transaction
pub fn insert_ledger_row(conn: &mut Connection, run_id: &str, row: &LedgerRow) -> Result<()> {
    let tx = conn.transaction()?;
    
    let scope3_json = if let Some(ext) = &row.scope3_extension {
        serde_json::to_string(ext)?
    } else {
        "null".to_string()
    };

    tx.execute(
        "INSERT INTO ledger (
            row_id, run_id, source_file, raw_row_index, raw_header,
            raw_value, raw_unit, converted_value, converted_unit,
            assumed_unit, ghg_scope, ghg_category, ghg_subcategory,
            emission_factor, ef_source, ef_jurisdiction, gwp_applied,
            tco2e, confidence, scope3_extension, sha256_hash, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            row.row_id.to_string(),
            run_id,
            row.source_file,
            row.raw_row_index,
            row.raw_header,
            row.raw_value,
            row.raw_unit,
            row.converted_value,
            row.converted_unit,
            row.assumed_unit,
            format!("{:?}", row.ghg_scope),
            row.ghg_category,
            row.ghg_subcategory,
            row.emission_factor,
            row.ef_source,
            format!("{:?}", row.ef_jurisdiction),
            row.gwp_applied,
            row.tco2e,
            row.confidence,
            scope3_json,
            row.sha256_hash,
            row.created_at.to_rfc3339(),
        ],
    )?;

    tx.commit()?;
    Ok(())
}

/// Inserts a QuarantineRow into the database
pub fn insert_quarantine_row(conn: &mut Connection, run_id: &str, row: &QuarantineRow) -> Result<()> {
    let tx = conn.transaction()?;

    tx.execute(
        "INSERT INTO quarantine (
            row_id, run_id, source_file, raw_row_index, raw_header,
            raw_value, error_reason, suggested_fix, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            row.row_id.to_string(),
            run_id,
            row.source_file,
            row.raw_row_index,
            row.raw_header,
            row.raw_value,
            format!("{:?}", row.error_reason),
            row.suggested_fix,
            row.created_at.to_rfc3339(),
        ],
    )?;

    tx.commit()?;
    Ok(())
}

/// Clears all data from a previous run (used for testing or restart)
pub fn clear_previous_run(conn: &mut Connection) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM ledger", [])?;
    tx.execute("DELETE FROM quarantine", [])?;
    tx.execute("DELETE FROM runs", [])?;
    tx.commit()?;
    Ok(())
}

/// Creates a new run record
pub fn create_run(
    conn: &mut Connection,
    run_id: &str,
    jurisdiction: &str,
    language: &str,
    industry: &str,
) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO runs (run_id, created_at, jurisdiction, language, industry, status)
         VALUES (?, ?, ?, ?, ?, ?)",
        params![
            run_id,
            Utc::now().to_rfc3339(),
            jurisdiction,
            language,
            industry,
            "processing"
        ],
    )?;
    tx.commit()?;
    Ok(())
}

/// Updates the status of a run
pub fn update_run_status(conn: &mut Connection, run_id: &str, status: &str) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute(
        "UPDATE runs SET status = ? WHERE run_id = ?",
        params![status, run_id],
    )?;
    tx.commit()?;
    Ok(())
}
