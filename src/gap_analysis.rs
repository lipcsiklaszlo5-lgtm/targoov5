use crate::models::{LedgerRow, GhgScope};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
struct EsrsDataPoint {
    esrs_code: &'static str,
    description: &'static str,
    required_scope: GhgScope,
    required_scope3_cat: Option<u8>,
    min_rows_expected: usize,
    severity: &'static str,
}

const ESRS_E1_REQUIRED: &[EsrsDataPoint] = &[
    EsrsDataPoint {
        esrs_code: "ESRS E1-6 §44a",
        description: "Scope 1 gross GHG emissions (tCO2e)",
        required_scope: GhgScope::SCOPE1,
        required_scope3_cat: None,
        min_rows_expected: 1,
        severity: "BLOCKER",
    },
    EsrsDataPoint {
        esrs_code: "ESRS E1-6 §44b",
        description: "Scope 2 location-based GHG emissions",
        required_scope: GhgScope::SCOPE2_LB,
        required_scope3_cat: None,
        min_rows_expected: 1,
        severity: "BLOCKER",
    },
    EsrsDataPoint {
        esrs_code: "ESRS E1-6 §44c",
        description: "Scope 3 Cat 1 Purchased Goods",
        required_scope: GhgScope::SCOPE3,
        required_scope3_cat: Some(1),
        min_rows_expected: 1,
        severity: "MAJOR",
    },
    EsrsDataPoint {
        esrs_code: "ESRS E1-6 §44c",
        description: "Scope 3 Cat 6 Business Travel",
        required_scope: GhgScope::SCOPE3,
        required_scope3_cat: Some(6),
        min_rows_expected: 1,
        severity: "MAJOR",
    },
    EsrsDataPoint {
        esrs_code: "ESRS E1-6 §44c",
        description: "Scope 3 Cat 7 Employee Commuting",
        required_scope: GhgScope::SCOPE3,
        required_scope3_cat: Some(7),
        min_rows_expected: 1,
        severity: "MINOR",
    },
];

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum GapStatus {
    Found,
    Missing,
    Partial,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GapResult {
    pub esrs_code: String,
    pub description: String,
    pub status: GapStatus,
    pub found_rows: usize,
    pub found_tco2e: f64,
    pub severity: String,
    pub suggested_data_source: String,
    pub suggested_action: String,
}

fn suggest_data_source(point: &EsrsDataPoint) -> String {
    match point.required_scope {
        GhgScope::SCOPE1 => "ERP energia modul / számlák".to_string(),
        GhgScope::SCOPE2_LB | GhgScope::SCOPE2_MB => "Áramszolgáltatói számlák".to_string(),
        GhgScope::SCOPE3 => "Beszerzési / logisztikai rendszer".to_string(),
    }
}

fn suggest_action(point: &EsrsDataPoint, status: &GapStatus) -> String {
    match status {
        GapStatus::Missing => format!("Adatgyűjtés indítása: {}", point.description),
        GapStatus::Partial => "Adatok bővítése szükséges".to_string(),
        GapStatus::Found => "Megfelelő".to_string(),
    }
}

pub fn run_gap_analysis(ledger: &[LedgerRow]) -> Vec<GapResult> {
    ESRS_E1_REQUIRED.iter().map(|point| {
        let matching_rows: Vec<_> = ledger.iter().filter(|row| {
            row.ghg_scope == point.required_scope &&
            match point.required_scope3_cat {
                None => true,
                Some(cat_id) => row.scope3_extension
                    .as_ref()
                    .map(|s3| s3.category_id == cat_id)
                    .unwrap_or(false),
            }
        }).collect();

        let status = match matching_rows.len() {
            0 => GapStatus::Missing,
            n if n < point.min_rows_expected => GapStatus::Partial,
            _ => GapStatus::Found,
        };

        GapResult {
            esrs_code: point.esrs_code.to_string(),
            description: point.description.to_string(),
            status,
            found_rows: matching_rows.len(),
            found_tco2e: matching_rows.iter().map(|r| r.tco2e).sum(),
            severity: point.severity.to_string(),
            suggested_data_source: suggest_data_source(point),
            suggested_action: suggest_action(point, &status),
        }
    }).collect()
}