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
    UK,      // UK — DEFRA
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
            Jurisdiction::UK                    => "DEFRA_2024",
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
