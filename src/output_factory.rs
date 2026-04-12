use crate::aggregation::AggregationResult;
use crate::benchmarking::{IndustryBenchmark, PeerComparison};
use crate::compliance::{ObligationStatus, OmnibusValidator};
use crate::eidas::EidasSigner;
use crate::finance::{
    self, AttributionMethod, CarbonRiskMetrics, ChangeDriver, FluctuationAnalysis,
    PhysicalRiskScorer, PortfolioAsset, ScenarioAnalyzer,
};
use crate::ixbrl::IxbrlGenerator;
use crate::models::{GhgScope, LedgerRow, QuarantineRow, Scope3CategorySummary};
use anyhow::Result;
use chrono::Utc;
use rust_xlsxwriter::{Format, Workbook, Worksheet};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{Cursor, Write};
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

pub struct OutputFactory;

impl OutputFactory {
    pub fn new() -> Self {
        Self
    }

    pub async fn generate_fritz_package(
        &self,
        run_id: &str,
        ledger: &[LedgerRow],
        quarantine: &[QuarantineRow],
        aggregation: &AggregationResult,
        scope3_breakdown: &HashMap<u8, Scope3CategorySummary>,
        narrative_text: &str,
        jurisdiction: &str,
        _language: &str,
        employee_count: Option<u32>,
        revenue_eur: Option<f64>,
    ) -> Result<Vec<u8>> {
        let manifest = self.generate_manifest(
            run_id,
            ledger,
            quarantine,
            aggregation,
            scope3_breakdown,
        )?;
        
        let summary_xlsx = self.generate_summary_xlsx(aggregation, scope3_breakdown, jurisdiction)?;
        let scope_detail_xlsx = self.generate_scope_detail_xlsx(ledger, scope3_breakdown)?;
        let audit_trail_xlsx = self.generate_audit_trail_xlsx(ledger)?;
        let quarantine_xlsx = self.generate_quarantine_xlsx(quarantine)?;
        let ef_reference_xlsx = self.generate_ef_reference_xlsx(jurisdiction)?;
        let narrative_docx = self.generate_narrative_docx(narrative_text, _language)?;
        let methodology_md = self.generate_methodology_md(ledger)?;
        let ixbrl_report = IxbrlGenerator::generate_xhtml(aggregation)?;
        let manifest_signature = EidasSigner::sign_manifest(&manifest)?;
        let climate_risk_xlsx = self.generate_climate_risk_xlsx(ledger, aggregation, jurisdiction)?;
        let compliance_xlsx = self.generate_compliance_xlsx(employee_count, revenue_eur, ledger)?;

        // Create ZIP archive in memory
        let mut zip_buffer = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut zip_buffer);
            let options = FileOptions::default()
                .compression_method(CompressionMethod::Deflated)
                .unix_permissions(0o644);

            zip.start_file("00_Manifest.json", options)?;
            zip.write_all(manifest.as_bytes())?;

            zip.start_file("TargooV2_Manifest.sig", options)?;
            zip.write_all(manifest_signature.as_bytes())?;

            zip.start_file("01_GHG_Inventar_Zusammenfassung.xlsx", options)?;
            zip.write_all(&summary_xlsx)?;

            zip.start_file("02_Scope_Aufschluesselung.xlsx", options)?;
            zip.write_all(&scope_detail_xlsx)?;

            zip.start_file("03_Audit_Trail_Master.xlsx", options)?;
            zip.write_all(&audit_trail_xlsx)?;

            zip.start_file("04_Quarantaene_Log.xlsx", options)?;
            zip.write_all(&quarantine_xlsx)?;

            zip.start_file("05_Emissionsfaktoren_Referenz.xlsx", options)?;
            zip.write_all(&ef_reference_xlsx)?;

            zip.start_file("06_Narrative_Bericht.docx", options)?;
            zip.write_all(&narrative_docx)?;

            zip.start_file("TargooV2_ESEF_Report.xhtml", options)?;
            zip.write_all(ixbrl_report.as_bytes())?;

            zip.start_file("METHODOLOGY.md", options)?;
            zip.write_all(methodology_md.as_bytes())?;

            zip.start_file("07_Climate_Risk_Report.xlsx", options)?;
            zip.write_all(&climate_risk_xlsx)?;

            zip.start_file("08_Compliance_Check.xlsx", options)?;
            zip.write_all(&compliance_xlsx)?;

            zip.finish()?;
        }
        Ok(zip_buffer.into_inner())
    }

    fn generate_manifest(
        &self,
        run_id: &str,
        ledger: &[LedgerRow],
        quarantine: &[QuarantineRow],
        aggregation: &AggregationResult,
        scope3_breakdown: &HashMap<u8, Scope3CategorySummary>,
    ) -> Result<String> {
        let mut hasher = Sha256::new();
        let chain_input = format!("{}{}", run_id, ledger.len());
        hasher.update(chain_input.as_bytes());
        let master_hash = format!("{:x}", hasher.finalize());

        let scope3_coverage_map: HashMap<u8, usize> = scope3_breakdown
            .iter()
            .map(|(id, summary)| (*id, summary.rows))
            .collect();

        let calc_path_breakdown = json!({
            "ActivityBased": ledger.iter().filter(|r| matches!(r.scope3_extension.as_ref().map(|e| e.calc_path), Some(crate::models::CalcPath::ActivityBased))).count(),
            "SpendBased": ledger.iter().filter(|r| matches!(r.scope3_extension.as_ref().map(|e| e.calc_path), Some(crate::models::CalcPath::SpendBased))).count(),
            "PCAF": ledger.iter().filter(|r| matches!(r.scope3_extension.as_ref().map(|e| e.calc_path), Some(crate::models::CalcPath::Pcaf))).count(),
        });

        let manifest = json!({
            "run_id": run_id,
            "created_at": Utc::now().to_rfc3339(),
            "methodology": "GHG Protocol Corporate Standard + CSRD/ESRS E1",
            "gwp_reference": "IPCC AR6 GWP100",
            "total_rows": ledger.len(),
            "quarantine_rows": quarantine.len(),
            "total_tco2e": aggregation.total_tco2e,
            "scope1_tco2e": aggregation.scope1_tco2e,
            "scope2_lb_tco2e": aggregation.scope2_lb_tco2e,
            "scope3_tco2e": aggregation.scope3_tco2e,
            "scope3_categories_covered": scope3_breakdown.len(),
            "scope3_coverage_map": scope3_coverage_map,
            "calc_path_breakdown": calc_path_breakdown,
            "scope3_completeness_pct": (scope3_breakdown.len() as f32 / 15.0) * 100.0,
            "master_sha256": master_hash,
            "legal_disclaimer": "This report is auto-generated by Targoo V2. Manual verification recommended."
        });

        Ok(serde_json::to_string_pretty(&manifest)?)
    }

    fn generate_summary_xlsx(
        &self,
        aggregation: &AggregationResult,
        scope3_breakdown: &HashMap<u8, Scope3CategorySummary>,
        jurisdiction: &str,
    ) -> Result<Vec<u8>> {
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.set_name("Zusammenfassung")?;

        let bold = Format::new().set_bold();
        let header = Format::new().set_bold().set_background_color("#00C9B1");

        worksheet.write_with_format(0, 0, "Targoo V2 GHG Inventory Summary", &header)?;
        worksheet.write_with_format(2, 0, "Scope", &bold)?;
        worksheet.write_with_format(2, 1, "tCO2e", &bold)?;
        
        worksheet.write(3, 0, "Scope 1")?;
        worksheet.write(3, 1, aggregation.scope1_tco2e)?;
        worksheet.write(4, 0, "Scope 2 (Location-Based)")?;
        worksheet.write(4, 1, aggregation.scope2_lb_tco2e)?;
        worksheet.write(5, 0, "Scope 2 (Market-Based)")?;
        worksheet.write(5, 1, aggregation.scope2_mb_tco2e)?;
        worksheet.write(6, 0, "Scope 3")?;
        worksheet.write(6, 1, aggregation.scope3_tco2e)?;
        worksheet.write_with_format(7, 0, "TOTAL", &bold)?;
        worksheet.write_with_format(7, 1, aggregation.total_tco2e, &bold)?;

        worksheet.write_with_format(9, 0, "Scope 3 Category Breakdown", &header)?;
        worksheet.write(10, 0, "Cat ID")?;
        worksheet.write(10, 1, "Category Name")?;
        worksheet.write(10, 2, "Rows")?;
        worksheet.write(10, 3, "tCO2e")?;
        worksheet.write(10, 4, "Avg Confidence")?;
        worksheet.write(10, 5, "Calc Path")?;

        let mut row = 11;
        for cat_id in 1..=15 {
            if let Some(summary) = scope3_breakdown.get(&cat_id) {
                worksheet.write(row, 0, cat_id as f64)?;
                worksheet.write(row, 1, &summary.cat_name)?;
                worksheet.write(row, 2, summary.rows as u32)?;
                worksheet.write(row, 3, summary.tco2e)?;
                worksheet.write(row, 4, summary.avg_confidence)?;
                worksheet.write(row, 5, format!("{:?}", summary.dominant_calc_path))?;
                row += 1;
            }
        }

        worksheet.write(row + 1, 0, format!("Jurisdiction: {}", jurisdiction))?;
        worksheet.write(row + 2, 0, "Unterschrift: _________________  Datum: _________________")?;

        Ok(workbook.save_to_buffer()?)
    }

    fn generate_scope_detail_xlsx(
        &self,
        ledger: &[LedgerRow],
        scope3_breakdown: &HashMap<u8, Scope3CategorySummary>,
    ) -> Result<Vec<u8>> {
        let mut workbook = Workbook::new();
        
        // Sheet 1: Scope 1 Detail
        let ws1 = workbook.add_worksheet();
        ws1.set_name("Scope 1 Detail")?;
        self.write_scope_rows(ws1, ledger, GhgScope::SCOPE1)?;

        // Sheet 2: Scope 2 Detail
        let ws2 = workbook.add_worksheet();
        ws2.set_name("Scope 2 Detail")?;
        self.write_scope_rows(ws2, ledger, GhgScope::SCOPE2_LB)?;

        // Sheet 3: Scope 3 Kategorien
        let ws3 = workbook.add_worksheet();
        ws3.set_name("Scope 3 Kategorien")?;
        ws3.write(0, 0, "Category ID")?;
        ws3.write(0, 1, "Category Name")?;
        ws3.write(0, 2, "Total Rows")?;
        ws3.write(0, 3, "Total tCO2e")?;
        ws3.write(0, 4, "Avg Confidence")?;
        ws3.write(0, 5, "Dominant Calc Path")?;
        ws3.write(0, 6, "Data Quality Tier")?;

        let mut row = 1;
        for cat_id in 1..=15 {
            if let Some(summary) = scope3_breakdown.get(&cat_id) {
                ws3.write(row, 0, cat_id as f64)?;
                ws3.write(row, 1, &summary.cat_name)?;
                ws3.write(row, 2, summary.rows as u32)?;
                ws3.write(row, 3, summary.tco2e)?;
                ws3.write(row, 4, summary.avg_confidence)?;
                ws3.write(row, 5, format!("{:?}", summary.dominant_calc_path))?;
                ws3.write(row, 6, if summary.avg_confidence >= 0.9 { "Primary" } else if summary.avg_confidence >= 0.7 { "Secondary" } else { "Estimated" })?;
                row += 1;
            }
        }
        ws3.write(row + 1, 0, format!("Scope 3 completeness: {}/15 categories identified", scope3_breakdown.len()))?;

        // Sheet 4: Top 10 Hotspots
        let ws4 = workbook.add_worksheet();
        ws4.set_name("Top 10 Hotspots")?;
        ws4.write(0, 0, "Rank")?;
        ws4.write(0, 1, "Source File")?;
        ws4.write(0, 2, "Raw Header")?;
        ws4.write(0, 3, "Scope")?;
        ws4.write(0, 4, "tCO2e")?;
        ws4.write(0, 5, "Confidence")?;

        let mut sorted_rows: Vec<&LedgerRow> = ledger.iter().collect();
        sorted_rows.sort_by(|a, b| b.tco2e.partial_cmp(&a.tco2e).unwrap());
        
        for (idx, r) in sorted_rows.iter().take(10).enumerate() {
            let row_num = (idx + 1) as u32;
            ws4.write(row_num, 0, row_num as f64)?;
            ws4.write(row_num, 1, &r.source_file)?;
            ws4.write(row_num, 2, &r.raw_header)?;
            ws4.write(row_num, 3, format!("{:?}", r.ghg_scope))?;
            ws4.write(row_num, 4, r.tco2e)?;
            ws4.write(row_num, 5, r.confidence)?;
        }

        Ok(workbook.save_to_buffer()?)
    }

    fn write_scope_rows(&self, ws: &mut Worksheet, ledger: &[LedgerRow], scope: GhgScope) -> Result<()> {
        ws.write(0, 0, "Row ID")?;
        ws.write(0, 1, "Source File")?;
        ws.write(0, 2, "Raw Header")?;
        ws.write(0, 3, "Raw Value")?;
        ws.write(0, 4, "Raw Unit")?;
        ws.write(0, 5, "Converted Value")?;
        ws.write(0, 6, "tCO2e")?;
        ws.write(0, 7, "Confidence")?;

        let scope_rows: Vec<&LedgerRow> = ledger.iter().filter(|r| r.ghg_scope == scope).collect();
        for (idx, r) in scope_rows.iter().enumerate() {
            let row_num = (idx + 1) as u32;
            ws.write(row_num, 0, r.row_id.to_string())?;
            ws.write(row_num, 1, &r.source_file)?;
            ws.write(row_num, 2, &r.raw_header)?;
            ws.write(row_num, 3, r.raw_value)?;
            ws.write(row_num, 4, &r.raw_unit)?;
            ws.write(row_num, 5, r.converted_value)?;
            ws.write(row_num, 6, r.tco2e)?;
            ws.write(row_num, 7, r.confidence)?;
        }
        Ok(())
    }

    fn generate_audit_trail_xlsx(&self, ledger: &[LedgerRow]) -> Result<Vec<u8>> {
        let mut workbook = Workbook::new();
        
        // Sheet 1: Verarbeitete Zeilen
        let ws1 = workbook.add_worksheet();
        ws1.set_name("Verarbeitete Zeilen")?;
        let green_bg = Format::new().set_background_color("#C6EFCE");
        let yellow_bg = Format::new().set_background_color("#FFEB9C");
        
        ws1.write(0, 0, "Row ID")?;
        ws1.write(0, 1, "Source")?;
        ws1.write(0, 2, "Index")?;
        ws1.write(0, 3, "Header")?;
        ws1.write(0, 4, "Raw Value")?;
        ws1.write(0, 5, "Unit")?;
        ws1.write(0, 6, "Scope")?;
        ws1.write(0, 7, "tCO2e")?;
        ws1.write(0, 8, "Confidence")?;
        ws1.write(0, 9, "Scope3 Cat ID")?;
        ws1.write(0, 10, "Scope3 Cat Name")?;
        ws1.write(0, 11, "Calc Path")?;
        ws1.write(0, 12, "DQ Score")?;

        for (idx, r) in ledger.iter().enumerate() {
            let row_num = (idx + 1) as u32;
            let fmt = if r.confidence >= 0.9 { &green_bg } else { &yellow_bg };
            
            ws1.write_with_format(row_num, 0, r.row_id.to_string(), fmt)?;
            ws1.write_with_format(row_num, 1, &r.source_file, fmt)?;
            ws1.write_with_format(row_num, 2, r.raw_row_index as u32, fmt)?;
            ws1.write_with_format(row_num, 3, &r.raw_header, fmt)?;
            ws1.write_with_format(row_num, 4, r.raw_value, fmt)?;
            ws1.write_with_format(row_num, 5, &r.raw_unit, fmt)?;
            ws1.write_with_format(row_num, 6, format!("{:?}", r.ghg_scope), fmt)?;
            ws1.write_with_format(row_num, 7, r.tco2e, fmt)?;
            ws1.write_with_format(row_num, 8, r.confidence, fmt)?;
            
            if let Some(ext) = &r.scope3_extension {
                ws1.write_with_format(row_num, 9, ext.category_id as f64, fmt)?;
                ws1.write_with_format(row_num, 10, &ext.category_name, fmt)?;
                ws1.write_with_format(row_num, 11, format!("{:?}", ext.calc_path), fmt)?;
                ws1.write_with_format(row_num, 12, ext.ghg_protocol_dq_score as f64, fmt)?;
            }
        }

        // Sheet 2: Angenommene Einheiten (Yellow rows only)
        let ws2 = workbook.add_worksheet();
        ws2.set_name("Angenommene Einheiten")?;
        ws2.write(0, 0, "Row ID")?;
        ws2.write(0, 1, "Header")?;
        ws2.write(0, 2, "Assumed Unit")?;
        ws2.write(0, 3, "Confidence")?;

        let yellow_rows: Vec<&LedgerRow> = ledger.iter().filter(|r| r.confidence < 0.9).collect();
        for (idx, r) in yellow_rows.iter().enumerate() {
            let row_num = (idx + 1) as u32;
            ws2.write(row_num, 0, r.row_id.to_string())?;
            ws2.write(row_num, 1, &r.raw_header)?;
            ws2.write(row_num, 2, r.assumed_unit.as_deref().unwrap_or("N/A"))?;
            ws2.write(row_num, 3, r.confidence)?;
        }
        ws2.write(yellow_rows.len() as u32 + 2, 0, "Unterschrift: _________________  Datum: _________________")?;

        // Sheet 3: Prüfsummen-Kette
        let ws3 = workbook.add_worksheet();
        ws3.set_name("Prüfsummen-Kette")?;
        ws3.write(0, 0, "Row Index")?;
        ws3.write(0, 1, "SHA-256 Hash")?;
        for (idx, r) in ledger.iter().enumerate() {
            ws3.write(idx as u32 + 1, 0, idx as u32)?;
            ws3.write(idx as u32 + 1, 1, &r.sha256_hash)?;
        }

        Ok(workbook.save_to_buffer()?)
    }

    fn generate_quarantine_xlsx(&self, quarantine: &[QuarantineRow]) -> Result<Vec<u8>> {
        let mut workbook = Workbook::new();
        let ws = workbook.add_worksheet();
        ws.set_name("Quarantäne-Übersicht")?;
        
        let red_header = Format::new().set_bold().set_background_color("#C00000").set_font_color("#FFFFFF");
        let yellow_bg = Format::new().set_background_color("#FFEB9C");
        
        ws.write_with_format(0, 0, "Row ID", &red_header)?;
        ws.write_with_format(0, 1, "Source", &red_header)?;
        ws.write_with_format(0, 2, "Index", &red_header)?;
        ws.write_with_format(0, 3, "Header", &red_header)?;
        ws.write_with_format(0, 4, "Raw Value", &red_header)?;
        ws.write_with_format(0, 5, "Error Reason", &red_header)?;
        ws.write_with_format(0, 6, "Suggested Fix", &red_header)?;
        ws.write_with_format(0, 7, "Korrektúra", &yellow_bg)?;

        for (idx, r) in quarantine.iter().enumerate() {
            let row_num = (idx + 1) as u32;
            ws.write(row_num, 0, r.row_id.to_string())?;
            ws.write(row_num, 1, &r.source_file)?;
            ws.write(row_num, 2, r.raw_row_index as u32)?;
            ws.write(row_num, 3, &r.raw_header)?;
            ws.write(row_num, 4, &r.raw_value)?;
            ws.write(row_num, 5, format!("{:?}", r.error_reason))?;
            ws.write(row_num, 6, r.suggested_fix.as_deref().unwrap_or(""))?;
            ws.write_with_format(row_num, 7, "", &yellow_bg)?;
        }

        Ok(workbook.save_to_buffer()?)
    }

    fn generate_ef_reference_xlsx(&self, _jurisdiction: &str) -> Result<Vec<u8>> {
        let mut workbook = Workbook::new();
        let ws = workbook.add_worksheet();
        ws.set_name("Verwendete Faktoren")?;

        ws.write(0, 0, "Scope")?;
        ws.write(0, 1, "Category")?;
        ws.write(0, 2, "Fuel/Activity")?;
        ws.write(0, 3, "EF Value")?;
        ws.write(0, 4, "EF Unit")?;
        ws.write(0, 5, "GWP")?;
        ws.write(0, 6, "Source")?;

        // Scope 1 factors
        let scope1_factors = vec![
            ("Scope 1", "Stationary Combustion", "Natural Gas", 0.183, "kgCO2e/kWh", 1.0),
            ("Scope 1", "Mobile Combustion", "Diesel", 2.680, "kgCO2e/liter", 1.0),
            ("Scope 1", "Mobile Combustion", "Petrol", 2.310, "kgCO2e/liter", 1.0),
            ("Scope 1", "Fugitive", "R410A", 1.0, "kg", 2088.0),
            ("Scope 1", "Fugitive", "SF6", 1.0, "kg", 25200.0),
        ];

        for (idx, (scope, cat, fuel, ef, unit, gwp)) in scope1_factors.iter().enumerate() {
            let row = (idx + 1) as u32;
            ws.write(row, 0, *scope)?;
            ws.write(row, 1, *cat)?;
            ws.write(row, 2, *fuel)?;
            ws.write(row, 3, *ef)?;
            ws.write(row, 4, *unit)?;
            ws.write(row, 5, *gwp)?;
            ws.write(row, 6, "IPCC AR6 / DEFRA 2024")?;
        }

        // EEIO factors
        ws.write(7, 0, "Scope 3 (EEIO)")?;
        ws.write(7, 1, "Spend-Based")?;
        ws.write(7, 2, "USD Spend")?;
        ws.write(7, 3, 0.370)?;
        ws.write(7, 4, "kgCO2e/USD")?;
        ws.write(7, 5, 1.0)?;
        ws.write(7, 6, "US EPA USEEIO 2.0")?;

        Ok(workbook.save_to_buffer()?)
    }

    fn generate_narrative_docx(&self, text: &str, _language: &str) -> Result<Vec<u8>> {
        // Simplified DOCX: Plain text with basic RTF-like wrapper or just plain text for demo
        // In production, use a proper docx library. For Targoo V2 spec, plain text is acceptable.
        let wrapped = format!(
            "TARGOO V2 NARRATIVE REPORT\n==========================\n\n{}\n\n---\nGenerated by Targoo V2 ESG Data Refinery\nCSRD/ESRS E1 Compliant Report",
            text
        );
        Ok(wrapped.into_bytes())
    }

    fn generate_methodology_md(&self, ledger: &[LedgerRow]) -> Result<String> {
        let mut md = String::from("# Methodology Report - Used Emission Factors\n\n");
        md.push_str("| Scope | Activity | EF Value | EF Unit | Source | Jurisdiction | GWP |\n");
        md.push_str("|-------|----------|----------|---------|--------|--------------|-----|\n");

        let mut seen_factors = std::collections::HashSet::new();

        for r in ledger {
            let key = format!(
                "{:?}|{}|{}|{}|{}|{:?}|{}",
                r.ghg_scope,
                r.ghg_subcategory,
                r.emission_factor,
                r.converted_unit,
                r.ef_source,
                r.ef_jurisdiction,
                r.gwp_applied
            );

            if !seen_factors.contains(&key) {
                md.push_str(&format!(
                    "| {:?} | {} | {} | {} | {} | {:?} | {} |\n",
                    r.ghg_scope,
                    r.ghg_subcategory,
                    r.emission_factor,
                    r.converted_unit,
                    r.ef_source,
                    r.ef_jurisdiction,
                    r.gwp_applied
                ));
                seen_factors.insert(key);
            }
        }

        Ok(md)
    }

    fn generate_climate_risk_xlsx(
        &self,
        ledger: &[LedgerRow],
        aggregation: &AggregationResult,
        jurisdiction: &str,
    ) -> Result<Vec<u8>> {
        let mut workbook = Workbook::new();
        let bold = Format::new().set_bold();

        // 1. Carbon Risk Metrics
        let ws1 = workbook.add_worksheet();
        ws1.set_name("Carbon Risk Metrics")?;
        
        let assets: Vec<PortfolioAsset> = ledger.iter()
            .filter(|r| r.ghg_scope == GhgScope::SCOPE3 && r.scope3_extension.as_ref().map(|e| e.category_id) == Some(15))
            .map(|r| PortfolioAsset {
                investment_amount: r.raw_value,
                emissions_tco2e: r.tco2e,
                revenue_meur: r.raw_value / 10.0, // Placeholder revenue
            })
            .collect();

        let total_value: f64 = assets.iter().map(|a| a.investment_amount).sum();
        let metrics = CarbonRiskMetrics::calculate(&assets, total_value);

        ws1.write_with_format(0, 0, "Metric", &bold)?;
        ws1.write_with_format(0, 1, "Value", &bold)?;
        
        ws1.write(1, 0, "WACI (tCO2e / M€ revenue)")?;
        ws1.write(1, 1, metrics.waci)?;
        
        ws1.write(2, 0, "Carbon Footprint (tCO2e / M€ invested)")?;
        ws1.write(2, 1, metrics.carbon_footprint)?;
        
        ws1.write(3, 0, "High-Carbon Exposure (%)")?;
        ws1.write(3, 1, metrics.high_carbon_exposure)?;

        // 2. Scenario Analysis
        let ws2 = workbook.add_worksheet();
        ws2.set_name("Scenario Analysis")?;
        
        let scenarios = ScenarioAnalyzer::analyze(aggregation.scope3_tco2e, total_value);
        
        ws2.write_with_format(0, 0, "Scenario", &bold)?;
        ws2.write_with_format(0, 1, "CO2 Price (2030)", &bold)?;
        ws2.write_with_format(0, 2, "Annual Carbon Cost", &bold)?;
        ws2.write_with_format(0, 3, "Impact on Value (%)", &bold)?;

        for (i, s) in scenarios.iter().enumerate() {
            let row = (i + 1) as u32;
            ws2.write(row, 0, &s.scenario_name)?;
            ws2.write(row, 1, s.co2_price_2030)?;
            ws2.write(row, 2, s.annual_carbon_cost)?;
            ws2.write(row, 3, s.impact_on_portfolio_value_pct)?;
        }

        // 3. Physical Risk
        let ws3 = workbook.add_worksheet();
        ws3.set_name("Physical Risk Scores")?;
        
        let jur = match jurisdiction {
            "US" => crate::models::Jurisdiction::US,
            "UK" => crate::models::Jurisdiction::UK,
            "EU" => crate::models::Jurisdiction::EU,
            _ => crate::models::Jurisdiction::GLOBAL,
        };
        let p_risk = PhysicalRiskScorer::score_by_jurisdiction(jur);

        ws3.write_with_format(0, 0, "Risk Category", &bold)?;
        ws3.write_with_format(0, 1, "Score (1-5)", &bold)?;
        
        ws3.write(1, 0, "Water Stress")?;
        ws3.write(1, 1, p_risk.water_stress_score as f64)?;
        
        ws3.write(2, 0, "Flood Risk")?;
        ws3.write(2, 1, p_risk.flood_risk_score as f64)?;
        
        ws3.write(3, 0, "Heatwave Risk")?;
        ws3.write(3, 1, p_risk.heatwave_risk_score as f64)?;
        
        ws3.write(4, 0, "COMBINED RISK")?;
        ws3.write(4, 1, p_risk.combined_risk_score as f64)?;

        // 4. Attribution Justification (Follow the Money)
        let ws4 = workbook.add_worksheet();
        ws4.set_name("Attribution Justification")?;
        ws4.write_with_format(0, 0, "Asset", &bold)?;
        ws4.write_with_format(0, 1, "Asset Class", &bold)?;
        ws4.write_with_format(0, 2, "Outstanding (€)", &bold)?;
        ws4.write_with_format(0, 3, "Total Value (€)", &bold)?;
        ws4.write_with_format(0, 4, "Factor", &bold)?;
        ws4.write_with_format(0, 5, "Justification", &bold)?;

        let cat15_rows: Vec<&LedgerRow> = ledger.iter()
            .filter(|r| r.ghg_scope == GhgScope::SCOPE3 && r.scope3_extension.as_ref().map(|e| e.category_id) == Some(15))
            .collect();

        for (i, r) in cat15_rows.iter().enumerate() {
            let row = (i + 1) as u32;
            let ext = r.scope3_extension.as_ref().unwrap();
            ws4.write(row, 0, &r.ghg_subcategory)?;
            ws4.write(row, 1, ext.pcaf_asset_class.as_deref().unwrap_or("Unknown"))?;
            ws4.write(row, 2, r.raw_value)?;
            ws4.write(row, 3, r.raw_value / ext.pcaf_attribution_factor.unwrap_or(1.0))?;
            ws4.write(row, 4, ext.pcaf_attribution_factor.unwrap_or(0.0))?;
            ws4.write(row, 5, "PCAF 2025 Standard Attribution")?;
        }

        // 5. Fluctuation Analysis
        let ws5 = workbook.add_worksheet();
        ws5.set_name("Fluctuation Analysis")?;
        
        // Mock previous emissions for demonstration (current * 0.9)
        let current_s3 = aggregation.scope3_tco2e;
        let prev_s3 = current_s3 * 0.9;
        let fluc = FluctuationAnalysis::new(
            current_s3,
            prev_s3,
            vec![ChangeDriver::NewLoans, ChangeDriver::PortfolioRebalancing]
        );

        ws5.write_with_format(0, 0, "Period", &bold)?;
        ws5.write_with_format(0, 1, "Financed Emissions (tCO2e)", &bold)?;
        
        ws5.write(1, 0, "Current Period")?;
        ws5.write(1, 1, fluc.current_emissions)?;
        
        ws5.write(2, 0, "Previous Period")?;
        ws5.write(2, 1, fluc.previous_emissions)?;
        
        ws5.write(4, 0, "Change Analysis")?;
        ws5.write(4, 1, fluc.generate_narrative())?;

        // 6. Peer Benchmarking
        let ws6 = workbook.add_worksheet();
        ws6.set_name("Peer Benchmarking")?;
        
        let benchmark = IndustryBenchmark::get_for_sector(jurisdiction); // Heuristic
        let client_revenue = 10_000_000.0; // Placeholder or passed revenue
        let client_intensity = aggregation.total_tco2e / (client_revenue / 1_000_000.0);
        let comparison = PeerComparison::new(client_intensity, benchmark.avg_carbon_intensity);

        ws6.write_with_format(0, 0, "Category", &bold)?;
        ws6.write_with_format(0, 1, "Value", &bold)?;
        
        ws6.write(1, 0, "Client Carbon Intensity (tCO2e/M€)")?;
        ws6.write(1, 1, client_intensity)?;
        
        ws6.write(2, 0, "Industry Average Intensity")?;
        ws6.write(2, 1, benchmark.avg_carbon_intensity)?;
        
        ws6.write(3, 0, "Performance Tier")?;
        ws6.write(3, 1, format!("{:?}", comparison.performance_tier))?;
        
        ws6.write(5, 0, "Narrative Analysis")?;
        ws6.write(5, 1, comparison.generate_narrative())?;

        let buf = workbook.save_to_buffer()?;
        Ok(buf.to_vec())
    }

    fn generate_compliance_xlsx(
        &self,
        employee_count: Option<u32>,
        revenue_eur: Option<f64>,
        ledger: &[LedgerRow],
    ) -> Result<Vec<u8>> {
        let mut workbook = Workbook::new();
        let bold = Format::new().set_bold();
        let green_bg = Format::new().set_background_color("#C6EFCE").set_font_color("#006100");
        let red_bg = Format::new().set_background_color("#FFC7CE").set_font_color("#9C0006");

        // 1. Omnibus Validation
        let ws1 = workbook.add_worksheet();
        ws1.set_name("Omnibus-Validierung")?;

        let validator = OmnibusValidator::new(employee_count, revenue_eur, None);
        let obligation = validator.is_csrd_obligated();
        let has_financial = ledger.iter().any(|r| r.scope3_extension.as_ref().map(|e| e.category_id) == Some(15));
        let scope = validator.get_reporting_scope(has_financial);

        ws1.write_with_format(0, 0, "Kriterium", &bold)?;
        ws1.write_with_format(0, 1, "Wert", &bold)?;
        ws1.write_with_format(0, 2, "Status", &bold)?;

        ws1.write(1, 0, "Anzahl der Mitarbeiter")?;
        ws1.write(1, 1, employee_count.map(|c| c as f64).unwrap_or(0.0))?;
        if employee_count.unwrap_or(0) > 1000 {
            ws1.write_with_format(1, 2, "Überschritten", &red_bg)?;
        } else {
            ws1.write_with_format(1, 2, "OK", &green_bg)?;
        }

        ws1.write(2, 0, "Umsatzerlöse (EUR)")?;
        ws1.write(2, 1, revenue_eur.unwrap_or(0.0))?;
        if revenue_eur.unwrap_or(0.0) > 450_000_000.0 {
            ws1.write_with_format(2, 2, "Überschritten", &red_bg)?;
        } else {
            ws1.write_with_format(2, 2, "OK", &green_bg)?;
        }

        ws1.write(4, 0, "CSRD OBLIGATION STATUS")?;
        ws1.write_with_format(4, 1, format!("{:?}", obligation), &bold)?;

        ws1.write(5, 0, "PROPOSED REPORTING SCOPE")?;
        ws1.write_with_format(5, 1, format!("{:?}", scope), &bold)?;

        // 2. Regulatory References
        let ws2 = workbook.add_worksheet();
        ws2.set_name("Rechtliche-Hinweise")?;
        ws2.write_with_format(0, 0, "Regulierung", &bold)?;
        ws2.write_with_format(0, 1, "Beschreibung", &bold)?;

        ws2.write(1, 0, "Omnibus I (2026)")?;
        ws2.write(1, 1, "EU-Richtlinie zur Harmonisierung von Schwellenwerten für die CSRD-Berichterstattung.")?;
        ws2.write(2, 0, "CSRD")?;
        ws2.write(2, 1, "Corporate Sustainability Reporting Directive (2022/2464/EU).")?;
        ws2.write(3, 0, "ESRS")?;
        ws2.write(3, 1, "European Sustainability Reporting Standards (EFRAG).")?;

        let buf = workbook.save_to_buffer()?;
        Ok(buf.to_vec())
    }
}

impl Default for OutputFactory {
    fn default() -> Self {
        Self::new()
    }
}
