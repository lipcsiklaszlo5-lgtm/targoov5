use std::fs;
use crate::config::models::{RunConfig, ValidatedConfig};
use crate::config::validator::validate;

const DEFAULT_CONFIG_PATH: &str = "config/defaults.json";

pub fn load_config(path: Option<&str>) -> Result<ValidatedConfig, String> {
    let config_path = path.unwrap_or(DEFAULT_CONFIG_PATH);

    // 1. Fájl beolvasás
    let raw = fs::read_to_string(config_path)
        .map_err(|e| format!(
            "[CCE] Konfigurációs fájl nem olvasható: {} — {}. \
             Alapértelmezett konfig betöltése.",
            config_path, e
        ))?;

    // 2. JSON parse
    let mut config: RunConfig = serde_json::from_str(&raw)
        .map_err(|e| format!("[CCE] JSON parse hiba: {}", e))?;

    // 3. Run ID és timestamp injektálás
    config.run_id    = Some(uuid::Uuid::new_v4());
    config.loaded_at = Some(chrono::Utc::now());

    // 4. Validáció
    validate(config)
}

