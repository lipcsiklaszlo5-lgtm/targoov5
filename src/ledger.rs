use crate::finance;
use crate::ingest::{parse_numeric_cell, RawRow};
use crate::models::{
    CalcPath, DataQualityTier, GhgScope, Jurisdiction, LedgerRow, MatchMethod, QuarantineReason,
    QuarantineRow, Scope3Category, Scope3Extension,
};
use crate::physics::{tco2e_calculator, validate_range_guard, UnitConverter};
use crate::triage::{TriageEngine, TriageResult};
use anyhow::Result;
use serde::{Serialize, Deserialize};
use chrono::Utc;
use hex;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone)]
pub struct LedgerProcessor {
    pub unit_converter: UnitConverter,
    // Cached emission factors for performance
    ef_cache: HashMap<String, f64>,
    // Previous hash for SHA-256 chain
    prev_hash: String,
    ai_resolver: finance::AiAssetResolver,
}

impl LedgerProcessor {
    pub fn new() -> Self {
        Self {
            unit_converter: UnitConverter::new(),
            ef_cache: HashMap::new(),
            prev_hash: String::new(),
            ai_resolver: finance::AiAssetResolver::new(),
        }
    }

    /// Processes a single raw row into either a LedgerRow, a QuarantineRow, or skips it
    pub async fn process_row(
        &mut self,
        _run_id: &str,
        row: &RawRow,
        triage_engine: &mut TriageEngine,
        jurisdiction: Jurisdiction,
    ) -> Result<Option<ProcessResult>> {
        // 1. Find the numeric value column
        let value_col_idx = match crate::ingest::IngestionEngine::find_value_column(row) {
            Some(idx) => idx,
            None => {
                // Silently skip metadata rows or header rows
                return Ok(None);
            }
        };

        let raw_header = row.headers.get(value_col_idx).cloned().unwrap_or_default();
        
        // Final sanity check: if the detected header is EXCLUDED, skip the row
        if crate::ingest::is_excluded_header(&raw_header) {
            return Ok(None);
        }

        let raw_value_str = row.values.get(value_col_idx).cloned().unwrap_or_default();

        // 2. Parse numeric value
        let raw_value = match parse_numeric_cell(&raw_value_str) {
            Some(v) => v,
            None => {
                return Ok(Some(ProcessResult::Quarantine(self.create_quarantine_row(
                    row,
                    raw_value_str,
                    QuarantineReason::NonNumericValue,
                    Some("Value could not be parsed as a number".to_string()),
                ))));
            }
        };

        // 3. Triage the header to determine Scope and EF
        let mut triage_result = triage_engine.triage_header(&raw_header, Some(row)).await;

        // Fallback: try all other column VALUES if the value column header didn't match
        if triage_result.is_none() {
            for (idx, value) in row.values.iter().enumerate() {
                if idx == value_col_idx {
                    continue;
                }
                if let Some(t) = triage_engine.triage_header(value, Some(row)).await {
                    triage_result = Some(t);
                    break;
                }
            }
        }

        let triage_result = match triage_result {
            Some(t) => t,
            None => {
                return Ok(Some(ProcessResult::Quarantine(self.create_quarantine_row(
                    row,
                    raw_value_str,
                    QuarantineReason::UnknownHeader,
                    Some(format!("Header '{}' not recognized", raw_header)),
                ))));
            }
        };

        // 4. Determine the unit
        let mut raw_unit = "unit".to_string();

        // 4a. Check if there's a dedicated Unit column
        if let Some(unit_idx) = row.headers.iter().position(|h| {
            let lh = h.to_lowercase();
            lh == "unit" || lh == "einheit" || lh == "egység"
        }) {
            if let Some(u) = row.values.get(unit_idx) {
                if !u.trim().is_empty() {
                    raw_unit = u.trim().to_string();
                }
            }
        }

        // 4b. If not found in Unit column, try extracting from the value column header
        if raw_unit == "unit" {
            let extracted = self.extract_unit_from_header(&raw_header);
            if extracted != "unit" {
                raw_unit = extracted;
            }
        }

        // 4c. If still not found, try extracting from all other column values (descriptive labels)
        if raw_unit == "unit" {
            for (idx, val) in row.values.iter().enumerate() {
                if idx == value_col_idx {
                    continue;
                }
                let extracted = self.extract_unit_from_header(val);
                if extracted != "unit" {
                    raw_unit = extracted;
                    break;
                }
            }
        }

        // 5. Convert value to canonical unit
        let unit_category = self.unit_converter.detect_category(&raw_unit);
        
        let converted_value = match self
            .unit_converter
            .convert(raw_value, &raw_unit, unit_category)
        {
            Ok(v) => v,
            Err(e) => {
                return Ok(Some(ProcessResult::Quarantine(self.create_quarantine_row(
                    row,
                    raw_value_str,
                    QuarantineReason::ParseError,
                    Some(format!("Unsupported unit: {}", raw_unit)),
                ))));
            }
        };

        let assumed_unit = if unit_category == "unknown" {
            Some(raw_unit.clone())
        } else {
            None
        };

        // 6. Determine GWP and Emission Factor
        let mut gwp_applied = self.get_gwp_for_category(&triage_result.ghg_category);
        let mut ef_value = self.get_emission_factor(&triage_result, jurisdiction);
        let mut tco2e = 0.0;
        let mut pcaf_factor = None;
        let mut pcaf_asset_class = None;
        let mut pcaf_dq_score = None;

        // 7. Handle SpendBased / PCAF specific calculations
        let (spend_usd, eeio_ef, attribution_factor, borrower_tco2e) =
            self.prepare_special_calc_params(&triage_result, raw_value, jurisdiction);

        let is_spend_based = matches!(triage_result.calc_path, Some(CalcPath::SpendBased));

        // PCAF 2025 Integration for Cat 15
        if let Some(15) = triage_result.scope3_id {
            // 1. AI-val detektáltasd az eszközosztályt
            let asset_class = self.ai_resolver
                .detect_asset_class(&raw_header)
                .await
                .unwrap_or(finance::AssetClass::ListedEquity);
            
            pcaf_asset_class = Some(format!("{:?}", asset_class));
            
            // 2. PCAF attribúció számítása
            let attribution = finance::PcafAttribution::new(
                asset_class,
                raw_value,  // outstanding amount
                None,       // total_value - placeholder handles default
                finance::AttributionMethod::DirectEvic, // Default method
                "Automated AI Detection".to_string(),   // Default source
            );
            
            // 3. Financed emissions
            let pcaf_result = attribution.calculate_financed_emissions(
                jurisdiction,
            );
            
            pcaf_factor = Some(pcaf_result.attribution_factor);
            tco2e = pcaf_result.financed_emissions_tco2e;
            
            // 4. Data Quality Score
            let dq = finance::PcafDataQuality::from_confidence(
                triage_result.confidence,
                asset_class,
            );
            pcaf_dq_score = Some(dq.as_int());
            
            // Override EF values for audit trail
            ef_value = 0.0; 
            gwp_applied = 1.0;
        } else {
            // 8. Calculate tCO2e (Legacy/Standard paths)
            tco2e = tco2e_calculator(
                converted_value,
                ef_value,
                gwp_applied,
                is_spend_based,
                spend_usd,
                eeio_ef,
                attribution_factor,
                borrower_tco2e,
            );
        }

        // 9. Scope 3 Extension construction
        let scope3_extension = if triage_result.ghg_scope == GhgScope::SCOPE3 {
            triage_result.scope3_id.map(|cat_id| {
                let category_name = triage_result
                    .scope3_name
                    .clone()
                    .unwrap_or_else(|| format!("Category {}", cat_id));
                let calc_path = if cat_id == 15 { CalcPath::Pcaf } else { triage_result.calc_path.unwrap_or(CalcPath::ActivityBased) };
                let data_quality_tier = if triage_result.confidence >= 0.9 {
                    DataQualityTier::Primary
                } else if triage_result.confidence >= 0.6 {
                    DataQualityTier::Secondary
                } else {
                    DataQualityTier::Estimated
                };

                Scope3Extension {
                    category_id: cat_id,
                    category_name,
                    category_match_method: triage_result.match_method,
                    category_confidence: triage_result.confidence,
                    calc_path,
                    spend_usd_normalized: spend_usd,
                    eeio_sector_code: None,
                    eeio_source: Some("EXIOBASE 3.8".to_string()),
                    physical_quantity: if matches!(calc_path, CalcPath::ActivityBased) {
                        Some(converted_value)
                    } else {
                        None
                    },
                    physical_unit: Some(triage_result.canonical_unit.clone()),
                    data_quality_tier,
                    ghg_protocol_dq_score: pcaf_dq_score.unwrap_or_else(|| self.calculate_dq_score(&triage_result, spend_usd.is_some())),
                    pcaf_asset_class,
                    pcaf_attribution_factor: pcaf_factor,
                    pcaf_data_quality_score: pcaf_dq_score,
                }
            })
        } else {
            None
        };

        // 10. Range Guard Validation
        let scope3_cat = scope3_extension
            .as_ref()
            .and_then(|ext| Scope3Category::try_from(ext.category_id).ok());
        if let Err(reason) = validate_range_guard(tco2e, triage_result.ghg_scope, scope3_cat) {
            return Ok(Some(ProcessResult::Quarantine(self.create_quarantine_row(
                row,
                raw_value_str,
                reason,
                Some(format!("tCO2e value {} out of allowed range", tco2e)),
            ))));
        }

        // 11. Generate SHA-256 Hash
        let hash_input = format!(
            "{}{}{}{}{:.8}{:?}{:.4}",
            self.prev_hash,
            row.row_index,
            raw_header,
            raw_value,
            tco2e,
            scope3_extension.as_ref().map(|e| e.category_id).unwrap_or(0),
            triage_result.confidence
        );
        let sha256_hash = self.generate_hash(&hash_input);
        self.prev_hash = sha256_hash.clone();

        // 12. Build LedgerRow
        let ledger_row = LedgerRow {
            row_id: Uuid::new_v4(),
            source_file: row.source_file.clone(),
            raw_row_index: row.row_index,
            raw_header,
            raw_value,
            raw_unit,
            converted_value,
            converted_unit: triage_result.canonical_unit.clone(),
            assumed_unit,
            ghg_scope: triage_result.ghg_scope,
            ghg_category: triage_result.ghg_category,
            ghg_subcategory: triage_result.matched_keyword,
            emission_factor: ef_value,
            ef_source: "Targoo Built-in Dictionary".to_string(),
            ef_jurisdiction: jurisdiction,
            gwp_applied,
            tco2e,
            confidence: triage_result.confidence,
            scope3_extension,
            sha256_hash,
            issa_5000: Some(crate::audit::issa_5000::Issa5000Metadata::new_automated(triage_result.confidence >= 0.9)),
            created_at: Utc::now(),
        };

        Ok(Some(ProcessResult::Ledger(ledger_row)))
    }

    fn create_quarantine_row(
        &self,
        row: &RawRow,
        raw_value: String,
        reason: QuarantineReason,
        suggested_fix: Option<String>,
    ) -> QuarantineRow {
        QuarantineRow {
            row_id: Uuid::new_v4(),
            source_file: row.source_file.clone(),
            raw_row_index: row.row_index,
            raw_header: row.headers.get(0).cloned().unwrap_or_default(),
            raw_value,
            error_reason: reason,
            suggested_fix,
            created_at: Utc::now(),
        }
    }

    fn extract_unit_from_header(&self, header: &str) -> String {
        let lower = header.to_lowercase();
        if lower.contains("kwh") {
            "kWh".to_string()
        } else if lower.contains("mwh") {
            "MWh".to_string()
        } else if lower.contains("gj") {
            "GJ".to_string()
        } else if lower.contains("tonne") || lower.contains("metric ton") {
            "tonne".to_string()
        } else if lower.contains("kg") {
            "kg".to_string()
        } else if lower.contains("lb") {
            "lb".to_string()
        } else if lower.contains("usd") || lower.contains("$") {
            "USD".to_string()
        } else if lower.contains("eur") || lower.contains("€") {
            "EUR".to_string()
        } else if lower.contains("gbp") || lower.contains("£") {
            "GBP".to_string()
        } else if lower.contains("km") {
            "km".to_string()
        } else if lower.contains("mile") {
            "mile".to_string()
        } else if lower.contains("liter") || lower.contains(" l ") {
            "liter".to_string()
        } else {
            "unit".to_string()
        }
    }

    fn get_gwp_for_category(&self, category: &str) -> f64 {
        match category {
            "R410A" => crate::models::GWP_R410A,
            "R134A" => crate::models::GWP_R134A,
            "SF6" => crate::models::GWP_SF6,
            "N2O" => crate::models::GWP_N2O,
            "CH4" => crate::models::GWP_CH4,
            _ => crate::models::GWP_CO2,
        }
    }

    fn get_emission_factor(&mut self, triage: &TriageResult, jurisdiction: Jurisdiction) -> f64 {
        let key = format!("{:?}_{}_{}", triage.ghg_scope, triage.matched_keyword, jurisdiction);
        if let Some(ef) = self.ef_cache.get(&key) {
            return *ef;
        }
        // For now, return the dictionary EF. Later steps will add dynamic jurisdiction overrides.
        let ef = triage.ef_value;
        self.ef_cache.insert(key, ef);
        ef
    }

    fn prepare_special_calc_params(
        &self,
        triage: &TriageResult,
        raw_value: f64,
        jurisdiction: Jurisdiction,
    ) -> (Option<f64>, Option<f64>, Option<f64>, Option<f64>) {
        let mut spend_usd = None;
        let mut eeio_ef = None;
        let mut attribution_factor = None;
        let mut borrower_tco2e = None;

        if matches!(triage.calc_path, Some(CalcPath::SpendBased)) {
            spend_usd = self
                .unit_converter
                .to_usd(raw_value, &triage.canonical_unit)
                .ok();
            eeio_ef = Some(match jurisdiction {
                Jurisdiction::US => 0.370,
                Jurisdiction::EU => 0.340,
                Jurisdiction::UK => 0.310,
                Jurisdiction::GLOBAL => 0.370,
            });
        }

        if matches!(triage.calc_path, Some(CalcPath::Pcaf)) {
            attribution_factor = Some(0.5); // Placeholder, real calculation comes from eeio_engine
            borrower_tco2e = Some(raw_value * 0.1); // Placeholder
        }

        (spend_usd, eeio_ef, attribution_factor, borrower_tco2e)
    }

    fn calculate_dq_score(&self, triage: &TriageResult, is_spend_based: bool) -> u8 {
        if triage.confidence >= 0.95 {
            1
        } else if triage.confidence >= 0.85 {
            2
        } else if is_spend_based {
            4
        } else {
            3
        }
    }

    fn generate_hash(&self, input: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn reset_chain(&mut self) {
        self.prev_hash = String::new();
    }
}

impl Default for LedgerProcessor {
    fn default() -> Self {
        Self::new()
    }
}

pub enum ProcessResult {
    Ledger(LedgerRow),
    Quarantine(QuarantineRow),
}

impl TryFrom<u8> for Scope3Category {
    type Error = ();
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            1 => Ok(Scope3Category::Cat1PurchasedGoodsServices),
            2 => Ok(Scope3Category::Cat2CapitalGoods),
            3 => Ok(Scope3Category::Cat3FuelEnergyActivities),
            4 => Ok(Scope3Category::Cat4UpstreamTransport),
            5 => Ok(Scope3Category::Cat5WasteGenerated),
            6 => Ok(Scope3Category::Cat6BusinessTravel),
            7 => Ok(Scope3Category::Cat7EmployeeCommuting),
            8 => Ok(Scope3Category::Cat8UpstreamLeasedAssets),
            9 => Ok(Scope3Category::Cat9DownstreamTransport),
            10 => Ok(Scope3Category::Cat10ProcessingSoldProducts),
            11 => Ok(Scope3Category::Cat11UseOfSoldProducts),
            12 => Ok(Scope3Category::Cat12EndOfLifeTreatment),
            13 => Ok(Scope3Category::Cat13DownstreamLeasedAssets),
            14 => Ok(Scope3Category::Cat14Franchises),
            15 => Ok(Scope3Category::Cat15Investments),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainVerificationResult {
    pub total_rows: usize,
    pub verified_rows: usize,
    pub broken_at_index: Option<usize>,
    pub master_hash: String,
    pub verification_timestamp: String,
    pub is_valid: bool,
}

pub fn verify_chain(ledger: &[LedgerRow], run_id: &str) -> ChainVerificationResult {
    let mut prev_hash = "GENESIS".to_string();
    let mut broken_at = None;

    for (idx, row) in ledger.iter().enumerate() {
        let expected_input = format!(
            "{}{}{}{}{}{}{}",
            run_id, row.raw_row_index, row.raw_header,
            row.raw_value, row.tco2e,
            row.scope3_extension.as_ref()
                .map(|s| s.category_id.to_string())
                .unwrap_or_default(),
            prev_hash
        );
        let expected_hash = hex::encode(
            Sha256::digest(expected_input.as_bytes())
        );

        if expected_hash != row.sha256_hash {
            broken_at = Some(idx);
            break;
        }
        prev_hash = row.sha256_hash.clone();
    }

    let master_input = format!("{}{}", run_id, prev_hash);
    let master_hash = hex::encode(
        Sha256::digest(master_input.as_bytes())
    );

    ChainVerificationResult {
        total_rows: ledger.len(),
        verified_rows: broken_at.unwrap_or(ledger.len()),
        broken_at_index: broken_at,
        master_hash,
        verification_timestamp: chrono::Utc::now().to_rfc3339(),
        is_valid: broken_at.is_none(),
    }
}
