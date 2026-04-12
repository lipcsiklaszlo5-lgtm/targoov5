use crate::models::{GhgScope, LedgerRow, Scope3CategorySummary};
use std::collections::HashMap;

pub struct Aggregator;

#[derive(Debug, Default)]
pub struct AggregationResult {
    pub total_tco2e: f64,
    pub scope1_tco2e: f64,
    pub scope2_lb_tco2e: f64,
    pub scope2_mb_tco2e: f64,
    pub scope3_tco2e: f64,
    pub scope3_breakdown: HashMap<u8, Scope3CategorySummary>,
    pub green_rows: usize,
    pub yellow_rows: usize,
    pub red_rows: usize, // Actually quarantine count, but named for UI
    pub total_rows: usize,
    pub categories_covered: usize,
}

impl Aggregator {
    pub fn new() -> Self {
        Self
    }

    /// Main aggregation function
    pub fn aggregate(&self, ledger: &[LedgerRow], quarantine_count: usize) -> AggregationResult {
        let mut result = AggregationResult {
            red_rows: quarantine_count,
            ..Default::default()
        };

        let mut scope3_cat_map: HashMap<u8, (usize, f64, f64, String, HashMap<String, usize>)> =
            HashMap::new();

        for row in ledger {
            result.total_rows += 1;

            // Confidence coloring
            if row.confidence >= 0.9 {
                result.green_rows += 1;
            } else {
                result.yellow_rows += 1;
            }

            // Scope totals
            match row.ghg_scope {
                GhgScope::SCOPE1 => result.scope1_tco2e += row.tco2e,
                GhgScope::SCOPE2_LB => result.scope2_lb_tco2e += row.tco2e,
                GhgScope::SCOPE2_MB => result.scope2_mb_tco2e += row.tco2e,
                GhgScope::SCOPE3 => {
                    result.scope3_tco2e += row.tco2e;
                    
                    // Scope 3 category breakdown
                    if let Some(ext) = &row.scope3_extension {
                        let cat_id = ext.category_id;
                        let entry = scope3_cat_map
                            .entry(cat_id)
                            .or_insert_with(|| (0, 0.0, 0.0, ext.category_name.clone(), HashMap::new()));
                        
                        entry.0 += 1; // count
                        entry.1 += row.tco2e; // sum tco2e
                        entry.2 += row.confidence as f64; // sum confidence for avg
                        
                        // Track calc path for dominant path detection
                        *entry.4.entry(format!("{:?}", ext.calc_path)).or_insert(0) += 1;
                    }
                }
            }
        }

        result.total_tco2e = result.scope1_tco2e + result.scope2_lb_tco2e + result.scope2_mb_tco2e + result.scope3_tco2e;

        // Build Scope 3 breakdown summaries
        for (cat_id, (count, sum_tco2e, sum_conf, cat_name, path_counts)) in scope3_cat_map {
            let avg_confidence = if count > 0 {
                (sum_conf / count as f64) as f32
            } else {
                0.0
            };

            let dominant_calc_path = path_counts
                .into_iter()
                .max_by_key(|(_, c)| *c)
                .map(|(path, _)| {
                    if path.contains("Activity") {
                        crate::models::CalcPath::ActivityBased
                    } else if path.contains("Spend") {
                        crate::models::CalcPath::SpendBased
                    } else {
                        crate::models::CalcPath::Pcaf
                    }
                })
                .unwrap_or(crate::models::CalcPath::ActivityBased);

            result.scope3_breakdown.insert(
                cat_id,
                Scope3CategorySummary {
                    cat_id,
                    cat_name,
                    rows: count,
                    tco2e: sum_tco2e,
                    dominant_calc_path,
                    avg_confidence,
                },
            );
        }

        result.categories_covered = result.scope3_breakdown.len();
        result
    }

    /// Calculates CSRD/ESRS E1 intensity metrics
    pub fn calculate_intensities(&self, total_tco2e: f64, revenue: Option<f64>, fte: Option<f64>, sqm: Option<f64>) -> IntensityMetrics {
        IntensityMetrics {
            tco2e_per_million_eur_revenue: revenue.map(|r| total_tco2e / (r / 1_000_000.0)),
            tco2e_per_fte: fte.map(|f| total_tco2e / f),
            tco2e_per_sqm: sqm.map(|s| total_tco2e / s),
        }
    }
}

impl Default for Aggregator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct IntensityMetrics {
    pub tco2e_per_million_eur_revenue: Option<f64>,
    pub tco2e_per_fte: Option<f64>,
    pub tco2e_per_sqm: Option<f64>,
}

/// Calculates completeness percentage for CSRD badge (categories covered / 15)
pub fn calculate_csrd_completeness(covered_categories: usize) -> f32 {
    (covered_categories as f32 / 15.0) * 100.0
}

/// Helper to format the category breakdown for the frontend
pub fn format_scope3_breakdown(breakdown: &HashMap<u8, Scope3CategorySummary>) -> Vec<&Scope3CategorySummary> {
    let mut summaries: Vec<&Scope3CategorySummary> = breakdown.values().collect();
    summaries.sort_by_key(|s| s.cat_id);
    summaries
}
