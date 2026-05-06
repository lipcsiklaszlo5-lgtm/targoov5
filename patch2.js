const fs = require('fs');

let content = fs.readFileSync('src/output_factory.rs', 'utf8');

const helpers = `
    fn get_legal_disclaimer(language: &str) -> &'static str {
        match language.to_lowercase().as_str() {
            "hu" => "Ez a dokumentum kizárólag a megrendelő által szolgáltatott nyers adatok feldolgozásával készült. A DataDynamic Kft. nem vállal felelősséget az adatok pontosságáért, teljességéért vagy helytállóságáért. Jelen dokumentum nem minősül hitelesített könyvvizsgálói jelentésnek.",
            "de_at" | "de_ch" | "de" => "Dieses Dokument wurde ausschließlich auf Basis der vom Auftraggeber bereitgestellten Rohdaten erstellt. Die DataDynamic Kft. übernimmt keine Haftung für die Richtigkeit, Vollständigkeit oder Angemessenheit der Daten. Dieses Dokument stellt keinen geprüften Wirtschaftsprüferbericht dar.",
            _ => "This document has been produced exclusively based on raw data provided by the client. DataDynamic Kft. assumes no responsibility for the accuracy, completeness or adequacy of the data. This document does not constitute a certified auditor's report.",
        }
    }

    fn apply_legal_footer(worksheet: &mut Worksheet, language: &str) {
        let text = Self::get_legal_disclaimer(language);
        let footer_string = format!("&C&8&K808080{}", text);
        worksheet.set_footer(&footer_string);
    }
`;

content = content.replace(
    /pub struct OutputFactory;\r?\n\r?\nimpl OutputFactory \{\r?\n\s*pub fn new\(\) -> Self \{\r?\n\s*Self\r?\n\s*\}/,
    `pub struct OutputFactory;\n\nimpl OutputFactory {\n    pub fn new() -> Self {\n        Self\n    }\n${helpers}`
);

const calls = [
    ["self.generate_summary_xlsx(aggregation, scope3_breakdown, &jurisdiction)?", "self.generate_summary_xlsx(aggregation, scope3_breakdown, &jurisdiction, &language)?"],
    ["self.generate_scope_detail_xlsx(ledger, scope3_breakdown)?", "self.generate_scope_detail_xlsx(ledger, scope3_breakdown, &language)?"],
    ["self.generate_audit_trail_xlsx(ledger, &verification_result)?", "self.generate_audit_trail_xlsx(ledger, &verification_result, &language)?"],
    ["self.generate_quarantine_xlsx(quarantine)?", "self.generate_quarantine_xlsx(quarantine, &language)?"],
    ["self.generate_ef_reference_xlsx(&jurisdiction)?", "self.generate_ef_reference_xlsx(&jurisdiction, &language)?"],
    ["self.generate_climate_risk_xlsx(ledger, aggregation, &jurisdiction)?", "self.generate_climate_risk_xlsx(ledger, aggregation, &jurisdiction, &language)?"],
    ["self.generate_compliance_xlsx(employee_count, revenue_eur, ledger)?", "self.generate_compliance_xlsx(employee_count, revenue_eur, ledger, &language)?"],
    ["self.generate_taxonomy_xlsx(ledger)?", "self.generate_taxonomy_xlsx(ledger, &language)?"],
    ["self.generate_issa_5000_report_xlsx(ledger)?", "self.generate_issa_5000_report_xlsx(ledger, &language)?"],
    ["self.generate_gap_analysis_xlsx(&gap_results)?", "self.generate_gap_analysis_xlsx(&gap_results, &language)?"],
    ["self.generate_benchmark_report_xlsx(&benchmark_results)?", "self.generate_benchmark_report_xlsx(&benchmark_results, &language)?"],
    ["self.generate_supply_chain_stress_test_xlsx(&supply_chain_results)?", "self.generate_supply_chain_stress_test_xlsx(&supply_chain_results, &language)?"],
    ["self.generate_lksg_report_xlsx(&lksg_results)?", "self.generate_lksg_report_xlsx(&lksg_results, &language)?"],
    ["self.generate_ixbrl_mapping_xlsx(ledger)?", "self.generate_ixbrl_mapping_xlsx(ledger, &language)?"],
];

for (let [old_c, new_c] of calls) {
    content = content.replace(old_c, new_c);
}

// Re-write all function signatures using regex
content = content.replace(/fn generate_summary_xlsx\([\s\S]*?jurisdiction: &str,\r?\n\s*\) -> Result<Vec<u8>> \{/,
    `fn generate_summary_xlsx(\n        &self,\n        aggregation: &AggregationResult,\n        scope3_breakdown: &HashMap<u8, Scope3CategorySummary>,\n        jurisdiction: &str,\n        language: &str,\n    ) -> Result<Vec<u8>> {`);

content = content.replace(/fn generate_scope_detail_xlsx\([\s\S]*?scope3_breakdown: &HashMap<u8, Scope3CategorySummary>,\r?\n\s*\) -> Result<Vec<u8>> \{/,
    `fn generate_scope_detail_xlsx(\n        &self,\n        ledger: &[LedgerRow],\n        scope3_breakdown: &HashMap<u8, Scope3CategorySummary>,\n        language: &str,\n    ) -> Result<Vec<u8>> {`);

content = content.replace(/fn generate_audit_trail_xlsx\([\s\S]*?verification_result: &ChainVerificationResult,\r?\n\s*\) -> Result<Vec<u8>> \{/,
    `fn generate_audit_trail_xlsx(\n        &self,\n        ledger: &[LedgerRow],\n        verification_result: &ChainVerificationResult,\n        language: &str,\n    ) -> Result<Vec<u8>> {`);

content = content.replace(/fn generate_quarantine_xlsx\(&self, quarantine: &\[QuarantineRow\]\) -> Result<Vec<u8>> \{/,
    `fn generate_quarantine_xlsx(&self, quarantine: &[QuarantineRow], language: &str) -> Result<Vec<u8>> {`);

content = content.replace(/fn generate_ef_reference_xlsx\(&self, _jurisdiction: &str\) -> Result<Vec<u8>> \{/,
    `fn generate_ef_reference_xlsx(&self, _jurisdiction: &str, language: &str) -> Result<Vec<u8>> {`);

content = content.replace(/fn generate_climate_risk_xlsx\([\s\S]*?jurisdiction: &str,\r?\n\s*\) -> Result<Vec<u8>> \{/,
    `fn generate_climate_risk_xlsx(\n        &self,\n        ledger: &[LedgerRow],\n        aggregation: &AggregationResult,\n        jurisdiction: &str,\n        language: &str,\n    ) -> Result<Vec<u8>> {`);

content = content.replace(/fn generate_compliance_xlsx\([\s\S]*?ledger: &\[LedgerRow\],\r?\n\s*\) -> Result<Vec<u8>> \{/,
    `fn generate_compliance_xlsx(\n        &self,\n        employee_count: Option<u32>,\n        revenue_eur: Option<f64>,\n        ledger: &[LedgerRow],\n        language: &str,\n    ) -> Result<Vec<u8>> {`);

content = content.replace(/fn generate_taxonomy_xlsx\(&self, _ledger: &\[LedgerRow\]\) -> Result<Vec<u8>> \{/,
    `fn generate_taxonomy_xlsx(&self, _ledger: &[LedgerRow], language: &str) -> Result<Vec<u8>> {`);

content = content.replace(/fn generate_issa_5000_report_xlsx\(&self, ledger: &\[LedgerRow\]\) -> Result<Vec<u8>> \{/,
    `fn generate_issa_5000_report_xlsx(&self, ledger: &[LedgerRow], language: &str) -> Result<Vec<u8>> {`);

content = content.replace(/fn generate_gap_analysis_xlsx\(&self, results: &\[GapResult\]\) -> Result<Vec<u8>> \{/,
    `fn generate_gap_analysis_xlsx(&self, results: &[GapResult], language: &str) -> Result<Vec<u8>> {`);

content = content.replace(/fn generate_benchmark_report_xlsx\(&self, results: &\[BenchmarkResult\]\) -> Result<Vec<u8>> \{/,
    `fn generate_benchmark_report_xlsx(&self, results: &[BenchmarkResult], language: &str) -> Result<Vec<u8>> {`);

content = content.replace(/fn generate_supply_chain_stress_test_xlsx\(&self, results: &\[SupplierRisk\]\) -> Result<Vec<u8>> \{/,
    `fn generate_supply_chain_stress_test_xlsx(&self, results: &[SupplierRisk], language: &str) -> Result<Vec<u8>> {`);

content = content.replace(/fn generate_ixbrl_mapping_xlsx\(&self, ledger: &\[LedgerRow\]\) -> Result<Vec<u8>> \{/,
    `fn generate_ixbrl_mapping_xlsx(&self, ledger: &[LedgerRow], language: &str) -> Result<Vec<u8>> {`);

content = content.replace(/fn generate_lksg_report_xlsx\(&self, results: &\[LksgComplianceRow\]\) -> Result<Vec<u8>> \{/,
    `fn generate_lksg_report_xlsx(&self, results: &[LksgComplianceRow], language: &str) -> Result<Vec<u8>> {`);


// Inject apply_legal_footer
// Notice we use \`p1\` WITHOUT \`&mut\` since \`p1\` is already \`&mut Worksheet\`
content = content.replace(/(ws\d*)\.set_name\([^\)]+\)\?;/g, (match, p1) => {
    return `${match}\n        Self::apply_legal_footer(${p1}, language);`;
});
content = content.replace(/(worksheet)\.set_name\([^\)]+\)\?;/g, (match, p1) => {
    return `${match}\n        Self::apply_legal_footer(${p1}, language);`;
});

// Fix narrative docx logic
const narrativeRegex = /fn generate_narrative_docx\(&self, text: &str, _language: &str\) -> Result<Vec<u8>> \{[\s\S]*?Ok\(wrapped\.into_bytes\(\)\)\s*\}/;
const narrativeNew = `fn generate_narrative_docx(&self, text: &str, language: &str) -> Result<Vec<u8>> {
        let disclaimer = Self::get_legal_disclaimer(language);
        let wrapped = format!(
            "TARGOO V2 NARRATIVE REPORT\\n==========================\\n\\n{}\\n\\n---\\nGenerated by Targoo V2 ESG Data Refinery\\nCSRD/ESRS E1 Compliant Report\\n\\nLEGAL DISCLAIMER:\\n{}",
            text, disclaimer
        );
        Ok(wrapped.into_bytes())
    }`;
content = content.replace(narrativeRegex, narrativeNew);

fs.writeFileSync('src/output_factory.rs', content);
console.log('Patched with JS!');
