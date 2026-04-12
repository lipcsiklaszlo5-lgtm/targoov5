use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// GWP100 factors per IPCC AR6
pub const GWP_CO2: f64 = 1.0;
pub const GWP_CH4: f64 = 29.8;
pub const GWP_N2O: f64 = 273.0;
pub const GWP_R410A: f64 = 2088.0;
pub const GWP_R134A: f64 = 1530.0;
pub const GWP_SF6: f64 = 25200.0;

/// Jurisdiction for emission factor selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum Jurisdiction {
    #[default]
    US,
    UK,
    EU,
    GLOBAL,
}

impl std::fmt::Display for Jurisdiction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::US => write!(f, "US"),
            Self::UK => write!(f, "UK"),
            Self::EU => write!(f, "EU"),
            Self::GLOBAL => write!(f, "GLOBAL"),
        }
    }
}

/// GHG Protocol Scope Classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum GhgScope {
    SCOPE1,
    SCOPE2_LB, // Location-Based
    SCOPE2_MB, // Market-Based
    SCOPE3,
}

/// Scope 3 Category IDs (1..=15)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Scope3Category {
    Cat1PurchasedGoodsServices = 1,
    Cat2CapitalGoods = 2,
    Cat3FuelEnergyActivities = 3,
    Cat4UpstreamTransport = 4,
    Cat5WasteGenerated = 5,
    Cat6BusinessTravel = 6,
    Cat7EmployeeCommuting = 7,
    Cat8UpstreamLeasedAssets = 8,
    Cat9DownstreamTransport = 9,
    Cat10ProcessingSoldProducts = 10,
    Cat11UseOfSoldProducts = 11,
    Cat12EndOfLifeTreatment = 12,
    Cat13DownstreamLeasedAssets = 13,
    Cat14Franchises = 14,
    Cat15Investments = 15,
}

impl Scope3Category {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cat1PurchasedGoodsServices => "Purchased Goods & Services",
            Self::Cat2CapitalGoods => "Capital Goods",
            Self::Cat3FuelEnergyActivities => "Fuel & Energy Related Activities",
            Self::Cat4UpstreamTransport => "Upstream Transportation & Distribution",
            Self::Cat5WasteGenerated => "Waste Generated in Operations",
            Self::Cat6BusinessTravel => "Business Travel",
            Self::Cat7EmployeeCommuting => "Employee Commuting",
            Self::Cat8UpstreamLeasedAssets => "Upstream Leased Assets",
            Self::Cat9DownstreamTransport => "Downstream Transportation & Distribution",
            Self::Cat10ProcessingSoldProducts => "Processing of Sold Products",
            Self::Cat11UseOfSoldProducts => "Use of Sold Products",
            Self::Cat12EndOfLifeTreatment => "End-of-Life Treatment of Sold Products",
            Self::Cat13DownstreamLeasedAssets => "Downstream Leased Assets",
            Self::Cat14Franchises => "Franchises",
            Self::Cat15Investments => "Investments",
        }
    }
}

/// Calculation path for Scope 3 emissions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CalcPath {
    ActivityBased,
    SpendBased,
    Pcaf, // Specific to Cat 15
}

/// Data quality tier for CSRD/ESRS E1 reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DataQualityTier {
    Primary,
    Secondary,
    Estimated,
}

/// Match method used for Scope 3 category classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum MatchMethod {
    Exact,
    Fuzzy,
    Inferred,
    Semantic,
}

/// Scope 3 Extension Data (Appended to LedgerRow for all Scope 3 entries)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scope3Extension {
    pub category_id: u8, // 1..=15
    pub category_name: String,
    pub category_match_method: MatchMethod,
    pub category_confidence: f32,
    pub calc_path: CalcPath,
    
    // SpendBased fields
    pub spend_usd_normalized: Option<f64>,
    pub eeio_sector_code: Option<String>,
    pub eeio_source: Option<String>,
    
    // ActivityBased fields
    pub physical_quantity: Option<f64>,
    pub physical_unit: Option<String>,
    
    // Quality Metrics
    pub data_quality_tier: DataQualityTier,
    pub ghg_protocol_dq_score: u8, // 1-5
}

/// Core Ledger Row (Immutable, Auditable)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerRow {
    pub row_id: Uuid,
    pub source_file: String,
    pub raw_row_index: usize,
    pub raw_header: String,
    pub raw_value: f64,
    pub raw_unit: String,
    pub converted_value: f64,
    pub converted_unit: String,
    pub assumed_unit: Option<String>, // None = Green, Some = Yellow
    pub ghg_scope: GhgScope,
    pub ghg_category: String,
    pub ghg_subcategory: String,
    pub emission_factor: f64, // kgCO2e / canonical unit
    pub ef_source: String,
    pub ef_jurisdiction: Jurisdiction,
    pub gwp_applied: f64, // IPCC AR6 GWP100
    pub tco2e: f64,
    pub confidence: f32, // 1.0 = Green, 0.8 = Yellow
    pub scope3_extension: Option<Scope3Extension>,
    pub sha256_hash: String,
    pub created_at: DateTime<Utc>,
}

/// Quarantine Row (Errors, Ambiguities, Missing Data)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuarantineRow {
    pub row_id: Uuid,
    pub source_file: String,
    pub raw_row_index: usize,
    pub raw_header: String,
    pub raw_value: String, // Original string value before parsing attempt
    pub error_reason: QuarantineReason,
    pub suggested_fix: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QuarantineReason {
    UnknownHeader,
    NonNumericValue,
    RangeGuardFail,
    AmbiguousScope3,
    MissingEmissionFactor,
    ParseError,
    DoubleCountingRisk,
    EmptyValue,
}

/// Global Application State
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub status: String, // "idle", "processing", "finished", "error"
    pub current_step: u8, // 0-6
    pub run_id: Option<String>,
    pub jurisdiction: Option<Jurisdiction>,
    pub language: Option<String>,
    pub industry: Option<String>,
    
    pub ledger: Vec<LedgerRow>,
    pub quarantine: Vec<QuarantineRow>,
    pub staged_files: Vec<String>,
    
    pub scope3_breakdown: HashMap<u8, Scope3CategorySummary>,
    pub zip_package: Option<Vec<u8>>,
    
    pub progress_message: Option<String>,
    pub total_tco2e: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scope3CategorySummary {
    pub cat_id: u8,
    pub cat_name: String,
    pub rows: usize,
    pub tco2e: f64,
    pub dominant_calc_path: CalcPath,
    pub avg_confidence: f32,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            status: "idle".to_string(),
            current_step: 0,
            run_id: None,
            jurisdiction: None,
            language: None,
            industry: None,
            ledger: Vec::new(),
            quarantine: Vec::new(),
            staged_files: Vec::new(),
            scope3_breakdown: HashMap::new(),
            zip_package: None,
            progress_message: None,
            total_tco2e: None,
        }
    }
}

// API Request/Response Models
#[derive(Debug, Deserialize)]
pub struct RunRequest {
    pub jurisdiction: Jurisdiction,
    pub language: String,
    pub industry: String,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub status: String,
    pub current_step: u8,
    pub progress: f32, // 0.0 - 1.0
    pub total_rows: usize,
    pub green_rows: usize,
    pub yellow_rows: usize,
    pub red_rows: usize,
    pub scope3_categories_covered: usize,
    pub total_tco2e: Option<f64>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ResultsResponse {
    pub run_id: String,
    pub total_tco2e: f64,
    pub scope1_tco2e: f64,
    pub scope2_lb_tco2e: f64,
    pub scope2_mb_tco2e: f64,
    pub scope3_tco2e: f64,
    pub scope3_breakdown: Vec<Scope3CategorySummary>,
    pub data_quality_tier_breakdown: HashMap<String, usize>,
    pub quarantine_count: usize,
    pub csrd_completeness_pct: f32, // Categories covered / 15
}
