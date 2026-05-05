[Ez egy komoly tervezési feladat. Végiggondolom az egész CCE architektúrát.

Compliance Configuration Engine — Teljes Terv

1. Fájlstruktúra

targoo-v2/
├── config/
│   ├── run_profile.json          ← aktív futtatás konfigja
│   ├── profiles/                 ← előre gyártott sablonok
│   │   ├── de_scope3_lksg.json
│   │   ├── eu_cbam_only.json
│   │   ├── ch_full_csrd.json
│   │   ├── hu_cbam_basic.json
│   │   └── global_quick.json
│   └── defaults.json             ← fallback ha nincs config
│
├── src/
│   ├── config/
│   │   ├── mod.rs                ← modul belépő
│   │   ├── loader.rs             ← JSON betöltés + validáció
│   │   ├── models.rs             ← RunConfig + összes típus
│   │   ├── validator.rs          ← keresztvalidáció, logikai ellenőrzés
│   │   └── persistence.rs        ← WORM SQLite mentés
│   │
│   ├── triage/                   ← meglévő, CCE-hez igazítva
│   ├── ingest/                   ← meglévő
│   ├── output/                   ← meglévő OutputFactory
│   └── main.rs                   ← CLI + CCE belépőpont


2. Adatmodellek — Teljes Rust Definíció

// src/config/models.rs

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

// ─── JOGHATÓSÁG ──────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "UPPERCASE")]
pub enum Jurisdiction {
    DE,      // Németország — UBA faktorok, LkSG
    AT,      // Ausztria — UBA AT
    CH,      // Svájc — CSA, svájci EF
    HU,      // Magyarország — CBAM fókusz
    EU,      // EU átlag — ESRS E1, CBAM
    US,      // USA — EPA eGRID, SEC
    Global,  // GHG Protocol alap
}

impl Jurisdiction {
    /// Melyik emissziós faktor forrást preferálja ez a joghatóság
    pub fn preferred_ef_source(&self) -> &'static str {
        match self {
            Jurisdiction::DE | Jurisdiction::AT => "UBA_2024",
            Jurisdiction::CH                    => "BAFU_2024",
            Jurisdiction::HU | Jurisdiction::EU => "EEA_2024",
            Jurisdiction::US                    => "EPA_2024",
            Jurisdiction::Global                => "IPCC_AR6",
        }
    }

    /// Kötelező lokális modul ez a joghatósághoz
    pub fn mandatory_modules(&self) -> Vec<ComplianceModule> {
        match self {
            Jurisdiction::DE => vec![ComplianceModule::LkSG],
            Jurisdiction::CH => vec![ComplianceModule::SwissCSA],
            Jurisdiction::EU => vec![ComplianceModule::EsrsE1],
            Jurisdiction::HU => vec![ComplianceModule::Cbam],
            _                => vec![],
        }
    }
}

// ─── COMPLIANCE MODULOK ───────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ComplianceModule {
    Scope1_2,    // Közvetlen + vásárolt energia
    Scope3,      // Értéklánc — GHG Protocol Cat 1-15
    Cbam,        // Carbon Border Adjustment Mechanism
    Pcaf,        // Partnership for Carbon Accounting Financials
    LkSG,        // Lieferkettensorgfaltspflichtengesetz (DE)
    EsrsE1,      // CSRD/ESRS E1 teljes csomag
    SwissCSA,    // Svájci Climate and Innovation Act
    SecClimate,  // US SEC Climate Disclosure
}

impl ComplianceModule {
    pub fn requires_scope3(&self) -> bool {
        matches!(self,
            ComplianceModule::Scope3
            | ComplianceModule::EsrsE1
            | ComplianceModule::LkSG
            | ComplianceModule::Cbam
        )
    }

    pub fn output_label(&self) -> &'static str {
        match self {
            ComplianceModule::Cbam     => "CBAM_Annex_I_Report",
            ComplianceModule::LkSG     => "LkSG_Risikoanalyse",
            ComplianceModule::EsrsE1   => "ESRS_E1_Disclosure",
            ComplianceModule::Pcaf     => "PCAF_Financed_Emissions",
            ComplianceModule::SwissCSA => "Swiss_CSA_Report",
            ComplianceModule::SecClimate => "SEC_Climate_Disclosure",
            _                          => "GHG_Inventory",
        }
    }
}

// ─── SZÁMÍTÁSI MÉLYSÉG ────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CalculationDepth {
    /// Csak alap számítások, szótár lookup, nincs AI
    /// Gyors, offline, determinisztikus
    Quick,

    /// Teljes szótár + Range-Guard + SHA-256 lánc
    /// AI bridge nélkül — audit-ready de lassabb
    Deterministic,

    /// Full pipeline: AI embedding + Gemini narratíva
    /// Leglassabb, legteljesebb output
    Full,
}

// ─── NYELV ────────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ReportLanguage {
    EN,
    DE,
    HU,
}

impl ReportLanguage {
    pub fn triage_dictionary_suffix(&self) -> &'static str {
        match self {
            ReportLanguage::EN => "en",
            ReportLanguage::DE => "de",
            ReportLanguage::HU => "hu",
        }
    }

    pub fn gemini_instruction(&self) -> &'static str {
        match self {
            ReportLanguage::EN => "Respond in English. Use GHG Protocol terminology.",
            ReportLanguage::DE => "Antworte auf Deutsch. Verwende CSRD/ESRS-Terminologie.",
            ReportLanguage::HU => "Válaszolj magyarul. Használj GHG Protokoll terminológiát.",
        }
    }
}

// ─── ÜGYFÉL METAADATOK ────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientMeta {
    pub name:        String,
    pub industry:    String,           // Manufacturing, Logistics, Finance...
    pub reference_year: u16,          // 2024
    pub contact_email: Option<String>,
    pub internal_id:   Option<String>, // Saját nyilvántartási szám
}

// ─── FUTTATÁSI KONFIG — A TELJES MASTER STRUCT ───────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    // Azonosítók
    pub config_version: String,        // "1.0"
    pub profile_name:   String,        // "de_scope3_lksg"

    // Ügyfél
    pub client:         ClientMeta,

    // Joghatóság és szabályok
    pub jurisdiction:   Jurisdiction,
    pub modules:        Vec<ComplianceModule>,
    pub language:       ReportLanguage,
    pub depth:          CalculationDepth,

    // Emissziós faktor preferenciák
    pub ef_source_override: Option<String>, // Ha None: jurisdiction.preferred_ef_source()
    pub gwp_standard:       GwpStandard,

    // Output kontrol
    pub fritz_package:      FritzPackageConfig,

    // AI konfig
    pub ai:                 AiConfig,

    // Belső — futáskor töltődik ki
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id:    Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loaded_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GwpStandard {
    IpccAr5,
    IpccAr6,   // default
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FritzPackageConfig {
    pub include_narrative:   bool,   // 06_Narrative_Bericht.docx
    pub include_ef_ref:      bool,   // 05_EF_Referenz.xlsx
    pub include_quarantine:  bool,   // 04_Quarantaene_Log.xlsx
    pub watermark:           bool,   // "DRAFT" vízjel ha true
    pub output_dir:          String, // "./output"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub embedding_url:    String,  // "http://127.0.0.1:9000/classify"
    pub embedding_timeout_ms: u64, // 300
    pub gemini_enabled:   bool,
    pub gemini_model:     String,  // "gemini-1.5-flash"
}

// ─── VALIDÁLT KONFIG — ami ténylegesen fut ────────────────────
/// A RunConfig-ból levezetett, runtime-használható struktúra.
/// A validator.rs állítja elő — ha idáig eljutott, garantáltan konzisztens.
#[derive(Debug, Clone)]
pub struct ValidatedConfig {
    pub run_id:        Uuid,
    pub config:        RunConfig,
    pub ef_source:     String,         // végleges EF forrás
    pub active_modules: Vec<ComplianceModule>, // modules + mandatory_modules
    pub dictionary_paths: Vec<String>, // melyik szótárfájlokat töltse be
    pub output_label:  String,         // a ZIP neve
}


3. A config/run_profile.json Sémája

Példa: Német, Scope 3 + LkSG futtatás

{
  "config_version": "1.0",
  "profile_name": "de_scope3_lksg",

  "client": {
    "name": "Mustermann GmbH",
    "industry": "Manufacturing",
    "reference_year": 2024,
    "contact_email": "esg@mustermann.de",
    "internal_id": "MUSTER-2024-003"
  },

  "jurisdiction": "DE",
  "modules": ["Scope3", "LkSG"],
  "language": "DE",
  "depth": "Full",

  "ef_source_override": null,
  "gwp_standard": "IpccAr6",

  "fritz_package": {
    "include_narrative":  true,
    "include_ef_ref":     true,
    "include_quarantine": true,
    "watermark":          false,
    "output_dir":         "./output/mustermann_2024"
  },

  "ai": {
    "embedding_url":          "http://127.0.0.1:9000/classify",
    "embedding_timeout_ms":   300,
    "gemini_enabled":         true,
    "gemini_model":           "gemini-1.5-flash"
  }
}


Példa: Magyar CBAM-only, gyors futtatás

{
  "config_version": "1.0",
  "profile_name": "hu_cbam_quick",

  "client": {
    "name": "Acélgyártó Kft.",
    "industry": "Steel",
    "reference_year": 2024,
    "internal_id": "HU-CBAM-2024-001"
  },

  "jurisdiction": "HU",
  "modules": ["Cbam"],
  "language": "HU",
  "depth": "Deterministic",

  "ef_source_override": null,
  "gwp_standard": "IpccAr6",

  "fritz_package": {
    "include_narrative":  false,
    "include_ef_ref":     true,
    "include_quarantine": true,
    "watermark":          true,
    "output_dir":         "./output/acelgyarto_cbam"
  },

  "ai": {
    "embedding_url":          "http://127.0.0.1:9000/classify",
    "embedding_timeout_ms":   300,
    "gemini_enabled":         false,
    "gemini_model":           "gemini-1.5-flash"
  }
}


4. A Három Kulcsfájl Kódja

// src/config/loader.rs

use std::{fs, path::Path};
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

pub fn load_or_default(path: Option<&str>) -> ValidatedConfig {
    match load_config(path) {
        Ok(cfg) => {
            eprintln!("[CCE] ✓ Konfig betöltve: {}", cfg.config.profile_name);
            cfg
        }
        Err(e) => {
            eprintln!("[CCE] ⚠ {} — Alapértelmezett indítás.", e);
            load_config(Some(DEFAULT_CONFIG_PATH))
                .expect("[CCE] FATAL: defaults.json sem olvasható")
        }
    }
}


// src/config/validator.rs

use crate::config::models::*;
use uuid::Uuid;
use chrono::Utc;

pub fn validate(config: RunConfig) -> Result<ValidatedConfig, String> {
    // ── 1. Üres module lista ──────────────────────────────────
    if config.modules.is_empty() {
        return Err("Legalább egy ComplianceModule szükséges.".into());
    }

    // ── 2. Kötelező modulok hozzáadása (joghatóság alapján) ──
    let mut active_modules = config.modules.clone();
    for mandatory in config.jurisdiction.mandatory_modules() {
        if !active_modules.contains(&mandatory) {
            eprintln!(
                "[CCE] ℹ {:?} joghatósághoz {:?} modul automatikusan hozzáadva.",
                config.jurisdiction, mandatory
            );
            active_modules.push(mandatory);
        }
    }

    // ── 3. EF forrás meghatározás ────────────────────────────
    let ef_source = config.ef_source_override
        .clone()
        .unwrap_or_else(|| config.jurisdiction.preferred_ef_source().to_string());

    // ── 4. Szótár útvonalak összerakása ──────────────────────
    let lang_suffix = config.language.triage_dictionary_suffix();
    let dictionary_paths = vec![
        format!("data/dictionary_{}.json", lang_suffix),
        format!("data/dictionary_en.json"), // fallback mindig
    ];

    // ── 5. Output ZIP neve ───────────────────────────────────
    let primary_module = active_modules.first()
        .map(|m| m.output_label())
        .unwrap_or("GHG_Report");
    let output_label = format!(
        "Fritz_Package_{}_{}_{}.zip",
        config.client.name.replace(' ', "_"),
        config.client.reference_year,
        primary_module
    );

    // ── 6. AI depth konzisztencia ────────────────────────────
    if config.depth == CalculationDepth::Quick && config.ai.gemini_enabled {
        eprintln!(
            "[CCE] ⚠ Quick depth + Gemini enabled — Gemini ki lesz kapcsolva."
        );
    }

    Ok(ValidatedConfig {
        run_id: config.run_id.unwrap_or_else(Uuid::new_v4),
        config,
        ef_source,
        active_modules,
        dictionary_paths,
        output_label,
    })
}


// src/config/persistence.rs

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


5. Integráció — main.rs

// src/main.rs (releváns rész)

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "targoo", about = "Targoo V2 — Compliance Data Refinery")]
struct Cli {
    /// Input fájl vagy mappa
    #[arg(long)]
    input: String,

    /// Konfigurációs profil JSON fájl
    #[arg(long, default_value = "config/run_profile.json")]
    config: String,

    /// Gyors override: csak ezt a modult futtassa
    #[arg(long)]
    module: Option<String>,

    /// Scope 3 only mode (régi CLI kompatibilitás)
    #[arg(long, default_value_t = false)]
    scope3_only: bool,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // ── 1. CCE betöltés ──────────────────────────────────────
    let mut validated = config::loader::load_or_default(Some(&cli.config));

    // ── 2. CLI override-ok alkalmazása ───────────────────────
    if cli.scope3_only {
        validated.config.modules = vec![config::models::ComplianceModule::Scope3];
        validated.active_modules = vec![config::models::ComplianceModule::Scope3];
    }

    // ── 3. DB init + konfig mentés (WORM) ────────────────────
    let conn = db::init_connection("data/targoo.db").unwrap();
    config::persistence::save_run_config(&conn, &validated).unwrap();

    eprintln!("[TARGOO] Run ID: {}", validated.run_id);
    eprintln!("[TARGOO] Profil: {}", validated.config.profile_name);
    eprintln!("[TARGOO] Modulok: {:?}", validated.active_modules);
    eprintln!("[TARGOO] EF forrás: {}", validated.ef_source);

    // ── 4. Ingest ────────────────────────────────────────────
    let raw_data = ingest::run(&cli.input, &validated).await.unwrap();

    // ── 5. Triage — konfig alapján ───────────────────────────
    let (ledger, quarantine) = triage::TriageEngine::new(&validated)
        .process(raw_data)
        .await;

    // ── 6. Aggregáció ─────────────────────────────────────────
    let aggregation = aggregation::run(&ledger, &validated);

    // ── 7. Output Factory ────────────────────────────────────
    output::FritzFactory::new(&validated)
        .generate(validated.run_id, &aggregation, &ledger, &quarantine)
        .await
        .unwrap();

    eprintln!("[TARGOO] ✓ Kész: {}", validated.output_label);
}


6. Folyamatábra — Motor Indulástól Outputig

CLI: targoo --input data/client.xlsx --config config/run_profile.json
        │
        ▼
[CCE loader.rs]
  fájl beolvasás → JSON parse → run_id + timestamp inject
        │
        ├── Hiba: defaults.json fallback + warning log
        │
        ▼
[CCE validator.rs]
  ├── modulok nem üresek?
  ├── jurisdiction.mandatory_modules() → hozzáadás ha hiányzik
  │     pl. DE → LkSG automatikusan aktiválva
  ├── ef_source meghatározás
  │     override? → override / else → jurisdiction.preferred_ef_source()
  ├── dictionary_paths összerakás (language + en fallback)
  ├── output_label generálás
  └── depth + AI konzisztencia check
        │
        ▼
[persistence.rs] → run_configs táblába mentés (WORM)
        │
        ▼
[ingest/] → CSV/XLSX/PDF streamelő olvasás
  ValidatedConfig.active_modules alapján szűri:
    CBAM → csak CBAM-releváns oszlopok
    Scope3 → Cat 1-15 fejlécek
        │
        ▼
[triage/TriageEngine]
  ValidatedConfig-ból kapja:
    - dictionary_paths → melyik szótárat töltse be
    - ef_source → melyik faktortáblát
    - depth → Quick/Deterministic/Full pipeline
    - embedding_url + timeout → AI bridge hívás
  Minden sorra: GREEN / YELLOW / RED
        │
        ▼
[aggregation/]
  active_modules alapján számolja:
    Scope1_2 → stationary + mobile combustion
    Scope3   → Cat 1-15 összesítés
    CBAM     → beágyazott kibocsátás CN kód szerint
    LkSG     → kockázati kategorizálás
        │
        ▼
[output/FritzFactory]
  fritz_package config alapján:
    include_narrative  → Gemini Flash API (ha depth=Full)
    include_ef_ref     → 05_EF_Referenz.xlsx
    include_quarantine → 04_Quarantaene_Log.xlsx
    watermark          → "DRAFT" bélyeg
    output_label       → ZIP fájlnév
  language → Gemini prompt nyelve, Excel fejlécek nyelve
        │
        ▼
Fritz_Package_Mustermann_GmbH_2024_LkSG_Risikoanalyse.zip


A CCE értéke a szolgáltatásodban

Ez nem technikai luxus — ez az üzleti modell alapja. Ahelyett hogy minden ügyfélhez kódot módosítasz, csak egy JSON-t töltesz ki. Öt perc és a motor már a következő ügyfél szabályai szerint dolgozik. A profiles/ mappában előre gyártott sablonok vannak a leggyakoribb esetekre — hu_cbam_quick.json, de_scope3_lksg.json, eu_full_csrd.json — és ezeket adod oda az ügyfeleidnek mint “konfigurációs kérdőív.” Ők kitöltik, te betolod, a motor dolgozik.
]
