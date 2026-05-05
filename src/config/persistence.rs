use rusqlite::{Connection, params};
use crate::config::models::ValidatedConfig;

pub fn save_run_config(conn: &Connection, cfg: &ValidatedConfig) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS run_configs (
            run_id        TEXT PRIMARY KEY,
            profile_name  TEXT NOT NULL,
            jurisdiction  TEXT NOT NULL,
            modules       TEXT NOT NULL,
            language      TEXT NOT NULL,
            depth         TEXT NOT NULL,
            client_name   TEXT NOT NULL,
            ref_year      INTEGER NOT NULL,
            ef_source     TEXT NOT NULL,
            config_json   TEXT NOT NULL,
            created_at    TEXT NOT NULL
        )",
        [],
    )?;

    let modules_json = serde_json::to_string(&cfg.active_modules)
        .unwrap_or_default();
    let config_json  = serde_json::to_string(&cfg.config)
        .unwrap_or_default();

    conn.execute(
        "INSERT OR REPLACE INTO run_configs VALUES
         (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        params![
            cfg.run_id.to_string(),
            cfg.config.profile_name,
            format!("{:?}", cfg.config.jurisdiction),
            modules_json,
            format!("{:?}", cfg.config.language),
            format!("{:?}", cfg.config.depth),
            cfg.config.client.name,
            cfg.config.client.reference_year,
            cfg.ef_source,
            config_json,
            chrono::Utc::now().to_rfc3339(),
        ],
    )?;

    Ok(())
}
