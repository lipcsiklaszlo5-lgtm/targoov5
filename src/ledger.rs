use crate::finance;
use crate::ingest::RawRow;
use crate::models::{
    CalcPath, DataQualityTier, GhgScope, Jurisdiction, LedgerRow, MatchMethod, QuarantineReason,
    QuarantineRow, Scope3Category, Scope3Extension,
};
use crate::physics::{validate_range_guard, UnitConverter};
use crate::triage::{TriageEngine, TriageResult};
use anyhow::Result;
use serde::{Serialize, Deserialize};
use chrono::Utc;
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
    ai_resolver: finance::ai_resolver::AiAssetResolver,
}

impl LedgerProcessor {
    pub fn new() -> Self {
        Self {
            unit_converter: UnitConverter::new(),
            ef_cache: HashMap::new(),
            prev_hash: String::new(),
            ai_resolver: finance::ai_resolver::AiAssetResolver::new(),
        }
    }

    fn find_value_field<'a>(&self, row: &'a RawRow) -> Option<(&'a String, &'a crate::ingest::RawField)> {
        let value_keywords = [
            "value", "wert", "amount", "betrag", "emission", "menge", "quantity",
            "total", "sum", "co2", "tco2", "kgco2", "kwh", "usd", "eur", "gbp",
            "cost", "spend", "consumption", "verbrauch", "fogyasztás",
        ];

        let excluded_headers = [
            "id", "company id", "company_id", "companyid", "company", "name", "year", "date", "period",
            "description", "notes", "comment", "source", "row", "index", "id_number",
            "unternehmen", "jahr", "datum", "beschreibung",
            "azonosito", "ceg", "nev", "ev", "leiras"
        ];

        // 1. Try to find by header keyword
        for (header, field) in &row.fields {
            let norm_header = header.to_lowercase();
            if excluded_headers.iter().any(|ex| norm_header.contains(ex)) {
                continue;
            }

            if value_keywords.iter().any(|kw| norm_header.contains(kw)) {
                if matches!(field, crate::ingest::RawField::Number(_) | crate::ingest::RawField::Integer(_)) {
                    return Some((header, field));
                }
                if let crate::ingest::RawField::Text(s) = field {
                    if crate::ingest_v1::parse_numeric_cell(s).is_some() {
                        return Some((header, field));
                    }
                }
            }
        }

        // 2. Fallback: first numeric field not in excluded
        for (header, field) in &row.fields {
            let norm_header = header.to_lowercase();
            if excluded_headers.iter().any(|ex| norm_header.contains(ex)) {
                continue;
            }
            if matches!(field, crate::ingest::RawField::Number(_) | crate::ingest::RawField::Integer(_)) {
                return Some((header, field));
            }
            if let crate::ingest::RawField::Text(s) = field {
                if crate::ingest_v1::parse_numeric_cell(s).is_some() {
                    return Some((header, field));
                }
            }
        }

        None
    }

    /// Returns ALL numeric fields from a row (for wide-format ERP CSVs)
    pub fn find_all_value_fields<'a>(&self, row: &'a RawRow) -> Vec<(&'a String, &'a crate::ingest::RawField)> {
        let excluded_headers = [
            "id", "company id", "company_id", "companyid", "company", "name", "year", "date", "period",
            "description", "notes", "comment", "source", "row", "index", "id_number",
            "unternehmen", "jahr", "datum", "beschreibung",
            "azonosito", "ceg", "nev", "ev", "leiras"
        ];
        let mut results = Vec::new();
        for (header, field) in &row.fields {
            let norm_header = header.to_lowercase();
            if excluded_headers.iter().any(|ex| norm_header.contains(ex)) {
                continue;
            }
            if matches!(field, crate::ingest::RawField::Number(_) | crate::ingest::RawField::Integer(_)) {
                results.push((header, field));
            } else if let crate::ingest::RawField::Text(s) = field {
                if crate::ingest_v1::parse_numeric_cell(s).is_some() {
                    results.push((header, field));
                }
            }
        }
        results
    }

    /// Processes a single raw row into either a LedgerRow, a QuarantineRow, or skips it
    pub async fn process_row(
        &mut self,
        _run_id: &str,
        row: &RawRow,
        triage_engine: &mut TriageEngine,
        jurisdiction: Jurisdiction,
    ) -> Result<Option<ProcessResult>> {
        // Long-format CSV: if row has "Header" column, use its value as raw_header
        let long_format_header: Option<String> = row.fields.iter()
            .find(|(k, _)| k.to_lowercase() == "header")
            .and_then(|(_, v)| if let crate::ingest::RawField::Text(s) = v { Some(s.clone()) } else { None });

        // 1. Find the numeric value field
        let (raw_header, field) = if let Some(ref lf_header) = long_format_header {
            match row.fields.iter().find(|(k, _)| k.to_lowercase() == "value") {
                Some((_, f)) => (lf_header.clone(), f),
                None => return Ok(None),
            }
        } else {
            match self.find_value_field(row) {
                Some(res) => (res.0.clone(), res.1),
                None => return Ok(None),
            }
        };

        let raw_value_str = field.to_string_lossy();

        // 2. Parse numeric value
        let raw_value = match field {
            crate::ingest::RawField::Number(n) => *n,
            crate::ingest::RawField::Integer(i) => *i as f64,
            crate::ingest::RawField::Text(s) => match crate::ingest_v1::parse_numeric_cell(s) {
                Some(v) => v,
                None => {
                    return Ok(Some(ProcessResult::Quarantine(self.create_quarantine_row(
                        row,
                        &raw_header,
                        raw_value_str,
                        QuarantineReason::NonNumericValue,
                        Some("Value could not be parsed as a number".to_string()),
                    ))));
                }
            },
            _ => {
                return Ok(Some(ProcessResult::Quarantine(self.create_quarantine_row(
                    row,
                    &raw_header,
                    raw_value_str,
                    QuarantineReason::NonNumericValue,
                    Some("Value is not a number".to_string()),
                ))));
            }
        };

        // 3. Triage the header to determine Scope and EF
        let mut triage_result = triage_engine.triage_header(&raw_header, Some(row)).await;

        // Fallback: try all other column VALUES if the value column header didn't match
        if triage_result.is_none() {
            for (header, f) in &row.fields {
                if header == &raw_header {
                    continue;
                }
                let val = f.to_string_lossy();
                if let Some(t) = triage_engine.triage_header(&val, Some(row)).await {
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
                    &raw_header,
                    raw_value_str,
                    QuarantineReason::UnknownHeader,
                    Some(format!("Header '{}' not recognized", raw_header)),
                ))));
            }
        };

        // 4. Unit Normalization
        let raw_unit = self.extract_unit_from_header(&raw_header);
        let (converted_value, assumed_unit) = match self.unit_converter.convert(
            raw_value,
            &raw_unit,
            &triage_result.canonical_unit,
        ) {
            Ok(val) => (val, None),
            Err(_) => {
                // If direct conversion fails, we assume the canonical unit but flag it
                (raw_value, Some(triage_result.canonical_unit.clone()))
            }
        };

        // 5. EF Selection & Calculation
        let ef_value = if triage_result.ef_value == 0.0 {
            let unit = if triage_result.canonical_unit.is_empty() {
                "kWh"
            } else {
                triage_result.canonical_unit.as_str()
            };

            crate::ef_database::get_emission_factor(
                triage_result.ghg_scope,
                &triage_result.ghg_category,
                jurisdiction,
                unit,
            )
            .unwrap_or(0.0)
        } else {
            triage_result.ef_value
        };
        let gwp_applied = self.get_gwp_for_category(&triage_result.ghg_category);

        let mut tco2e = (converted_value * ef_value * gwp_applied) / 1000.0;

        // 6. PCAF Specific Attribution (if applicable)
        if let Some(CalcPath::Pcaf) = triage_result.calc_path {
            if let Some(asset_class) = triage_result.scope3_name.as_ref() {
                 // Placeholder for actual PCAF attribution resolving logic
                 // if asset_class.contains("Equity") { ... }
                 let _ = asset_class;
            }
        }

        // 7. Scope 3 Extension
        let mut scope3_extension = None;
        if triage_result.ghg_scope == GhgScope::SCOPE3 {
            let cat_id = triage_result.scope3_id.unwrap_or(1);
            let calc_path = triage_result.calc_path.unwrap_or(CalcPath::ActivityBased);
            
            let dq_tier = if triage_result.match_method == MatchMethod::Exact && assumed_unit.is_none() {
                DataQualityTier::Primary
            } else if triage_result.confidence > 0.8 {
                DataQualityTier::Secondary
            } else {
                DataQualityTier::Estimated
            };

            scope3_extension = Some(Scope3Extension {
                category_id: cat_id,
                category_name: triage_result.scope3_name.clone().unwrap_or_else(|| "Unknown".to_string()),
                category_match_method: triage_result.match_method,
                category_confidence: triage_result.confidence,
                calc_path,
                spend_usd_normalized: if calc_path == CalcPath::SpendBased { Some(converted_value) } else { None },
                eeio_sector_code: None,
                eeio_source: Some("Targoo Internal".to_string()),
                physical_quantity: if calc_path == CalcPath::ActivityBased { Some(converted_value) } else { None },
                physical_unit: Some(triage_result.canonical_unit.clone()),
                data_quality_tier: dq_tier,
                ghg_protocol_dq_score: match dq_tier {
                    DataQualityTier::Primary => 1,
                    DataQualityTier::Secondary => 2,
                    DataQualityTier::Estimated => 4,
                },
                pcaf_asset_class: triage_result.scope3_name.clone(),
                pcaf_attribution_factor: None,
                pcaf_data_quality_score: None,
            });
        }

        // 10. Range Guard Validation
        if let Err(reason) = validate_range_guard(tco2e, triage_result.ghg_scope) {
             return Ok(Some(ProcessResult::Quarantine(self.create_quarantine_row(
                row,
                &raw_header,
                raw_value_str,
                reason,
                Some(format!("tCO2e value {} out of allowed range", tco2e)),
            ))));
        }

        // 11. Generate SHA-256 Hash
        let hash_input = format!(
            "{}{}{}{}{:.8}{:?}{:.4}",
            self.prev_hash,
            row.source_line,
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
            raw_row_index: row.source_line as usize,
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
        raw_header: &str,
        raw_value: String,
        reason: QuarantineReason,
        suggested_fix: Option<String>,
    ) -> QuarantineRow {
        QuarantineRow {
            row_id: Uuid::new_v4(),
            source_file: row.source_file.clone(),
            raw_row_index: row.source_line as usize,
            raw_header: raw_header.to_string(),
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
        let cat = category.to_lowercase();
        if cat.contains("methane") || cat.contains("ch4") {
            crate::models::GWP_CH4
        } else if cat.contains("nitrous") || cat.contains("n2o") {
            crate::models::GWP_N2O
        } else if cat.contains("sf6") {
            crate::models::GWP_SF6
        } else {
            crate::models::GWP_CO2
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
    pub is_valid: bool,
    pub broken_at_index: Option<usize>,
    pub total_rows: usize,
    pub master_hash: String,
    pub verified_rows: usize,
    pub verification_timestamp: String,
}

pub fn verify_chain(rows: &[LedgerRow]) -> ChainVerificationResult {
    let mut prev_hash = String::new();
    let mut verified_count = 0;
    let mut is_valid = true;
    let mut broken_at = None;

    for (idx, row) in rows.iter().enumerate() {
        let hash_input = format!(
            "{}{}{}{}{:.8}{:?}{:.4}",
            prev_hash,
            row.raw_row_index,
            row.raw_header,
            row.raw_value,
            row.tco2e,
            row.scope3_extension.as_ref().map(|e| e.category_id).unwrap_or(0),
            row.confidence
        );
        let mut hasher = Sha256::new();
        hasher.update(hash_input.as_bytes());
        let current_hash = format!("{:x}", hasher.finalize());
        
        if current_hash != row.sha256_hash {
            is_valid = false;
            broken_at = Some(idx);
            break;
        }
        prev_hash = current_hash.clone();
        verified_count += 1;
    }
    
    ChainVerificationResult {
        is_valid,
        broken_at_index: broken_at,
        total_rows: rows.len(),
        master_hash: prev_hash,
        verified_rows: verified_count,
        verification_timestamp: Utc::now().to_rfc3339(),
    }
}
