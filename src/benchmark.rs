use crate::models::{LedgerRow, GhgScope};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
struct BenchmarkEntry {
    industry: &'static str,
    scope: GhgScope,
    scope3_cat: Option<u8>,
    metric: &'static str,
    value_p25: f64,
    value_p50: f64,
    value_p75: f64,
    source: &'static str,
    year: u16,
}

const BENCHMARKS: &[BenchmarkEntry] = &[
    BenchmarkEntry {
        industry: "Manufacturing",
        scope: GhgScope::SCOPE1,
        scope3_cat: None,
        metric: "tCO2e/revenue_meur",
        value_p25: 45.0,
        value_p50: 120.0,
        value_p75: 380.0,
        source: "UBA Industriesektor 2023",
        year: 2023,
    },
    BenchmarkEntry {
        industry: "Logistics",
        scope: GhgScope::SCOPE1,
        scope3_cat: None,
        metric: "tCO2e/revenue_meur",
        value_p25: 89.0,
        value_p50: 210.0,
        value_p75: 520.0,
        source: "EEA Transport Sector 2023",
        year: 2023,
    },
];

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BenchmarkResult {
    pub scope: String,
    pub company_value: f64,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub percentile_position: String,
    pub materiality_flag: bool,
    pub source: String,
}

pub fn run_benchmark(
    ledger: &[LedgerRow],
    industry: &str,
    revenue_meur: Option<f64>,
) -> Vec<BenchmarkResult> {
    let mut results = Vec::new();

    // Számítsd ki a cég összes kibocsátását scope-onként
    let scope1_total: f64 = ledger.iter()
        .filter(|r| r.ghg_scope == GhgScope::SCOPE1)
        .map(|r| r.tco2e)
        .sum();
    
    let revenue = revenue_meur.unwrap_or(1.0); // Elkerüljük a 0-val osztást
    let company_intensity = scope1_total / revenue;

    for entry in BENCHMARKS.iter().filter(|e| e.industry == industry) {
        let percentile_position = if company_intensity <= entry.value_p25 {
            "TOP 25%"
        } else if company_intensity <= entry.value_p50 {
            "ÁTLAG"
        } else if company_intensity <= entry.value_p75 {
            "ALSÓ 25%"
        } else {
            "P75 FELETT"
        };

        let materiality_flag = company_intensity > entry.value_p75;

        results.push(BenchmarkResult {
            scope: format!("{:?}", entry.scope),
            company_value: company_intensity,
            p25: entry.value_p25,
            p50: entry.value_p50,
            p75: entry.value_p75,
            percentile_position: percentile_position.to_string(),
            materiality_flag,
            source: entry.source.to_string(),
        });
    }
    results
}