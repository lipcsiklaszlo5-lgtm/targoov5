use crate::aggregation::AggregationResult;
use crate::models::{Jurisdiction, Scope3CategorySummary};
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;

pub struct GeminiClient {
    client: Client,
    api_key: String,
    model: String,
}

impl GeminiClient {
    pub fn new(api_key: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            api_key,
            model: "gemini-1.5-flash".to_string(),
        }
    }

    /// Generates the narrative report text with Scope 3 analysis
    pub async fn generate_narrative(
        &self,
        aggregation: &AggregationResult,
        jurisdiction: Jurisdiction,
        language: &str,
        industry: &str,
        scope3_breakdown: &HashMap<u8, Scope3CategorySummary>,
    ) -> String {
        let prompt = self.build_prompt(aggregation, jurisdiction, language, industry, scope3_breakdown);
        
        match self.call_api(&prompt).await {
            Ok(text) => text,
            Err(e) => {
                eprintln!("Gemini API error: {}. Using fallback narrative.", e);
                self.generate_fallback_narrative(aggregation, jurisdiction, language, scope3_breakdown)
            }
        }
    }

    async fn call_api(&self, prompt: &str) -> Result<String> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let payload = json!({
            "contents": [{
                "parts": [{
                    "text": prompt
                }]
            }],
            "generationConfig": {
                "temperature": 0.3,
                "maxOutputTokens": 2048,
                "topP": 0.95
            }
        });

        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("API error {}: {}", status, error_text));
        }

        let json_response: Value = response.json().await?;
        
        let text = json_response["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow!("Unexpected API response structure"))?
            .to_string();

        Ok(text)
    }

    fn build_prompt(
        &self,
        aggregation: &AggregationResult,
        jurisdiction: Jurisdiction,
        language: &str,
        industry: &str,
        scope3_breakdown: &HashMap<u8, Scope3CategorySummary>,
    ) -> String {
        let lang_instruction = match language {
            "de" => "Antworte auf Deutsch in einem professionellen, aber verständlichen Ton.",
            "hu" => "Válaszolj magyarul, professzionális, de közérthető stílusban.",
            _ => "Respond in English with a professional yet accessible tone.",
        };

        let jurisdiction_str = format!("{:?}", jurisdiction);
        let categories_covered = scope3_breakdown.len();
        let completeness_pct = (categories_covered as f32 / 15.0) * 100.0;
        
        let mut scope3_summary = String::new();
        let mut top_categories: Vec<&Scope3CategorySummary> = scope3_breakdown.values().collect();
        top_categories.sort_by(|a, b| b.tco2e.partial_cmp(&a.tco2e).unwrap());
        
        for (i, cat) in top_categories.iter().take(5).enumerate() {
            scope3_summary.push_str(&format!(
                "{}. Category {} ({}): {:.2} tCO2e ({} rows, avg confidence: {:.0}%)\n",
                i + 1,
                cat.cat_id,
                cat.cat_name,
                cat.tco2e,
                cat.rows,
                cat.avg_confidence * 100.0
            ));
        }

        let dq_breakdown = format!(
            "Green (high confidence): {} rows, Yellow (medium confidence): {} rows",
            aggregation.green_rows, aggregation.yellow_rows
        );

        format!(
            r#"You are an ESG reporting expert specializing in GHG Protocol and CSRD/ESRS E1 compliance.

TASK: Generate an Executive Summary and Scope 3 Analysis for a corporate carbon footprint report.

CONTEXT:
- Industry: {industry}
- Jurisdiction: {jurisdiction_str}
- Total Emissions: {total:.2} tCO2e
- Scope 1: {scope1:.2} tCO2e
- Scope 2 (Location-Based): {scope2_lb:.2} tCO2e
- Scope 2 (Market-Based): {scope2_mb:.2} tCO2e
- Scope 3: {scope3:.2} tCO2e

SCOPE 3 CATEGORY BREAKDOWN ({covered}/15 categories covered, {completeness_pct:.0}% complete):
{scope3_summary}

DATA QUALITY:
{dq_breakdown}
Total rows processed: {total_rows}
Quarantined rows: {quarantine}

INSTRUCTIONS:
1. Write a concise executive summary (2-3 paragraphs).
2. Analyze the Scope 3 results: identify the top 3 hotspots and suggest one actionable reduction measure for each.
3. Provide a CSRD/ESRS E1 compliance assessment: note which categories are missing and what data collection improvements are needed.
4. Format the response in plain text with clear section headers (EXECUTIVE SUMMARY, SCOPE 3 ANALYSIS, CSRD COMPLIANCE ASSESSMENT).
5. {lang_instruction}

Do not use markdown formatting. Keep the response under 600 words."#,
            total = aggregation.total_tco2e,
            scope1 = aggregation.scope1_tco2e,
            scope2_lb = aggregation.scope2_lb_tco2e,
            scope2_mb = aggregation.scope2_mb_tco2e,
            scope3 = aggregation.scope3_tco2e,
            covered = categories_covered,
            completeness_pct = completeness_pct,
            scope3_summary = scope3_summary,
            dq_breakdown = dq_breakdown,
            total_rows = aggregation.total_rows,
            quarantine = aggregation.red_rows,
            lang_instruction = lang_instruction,
            industry = industry,
            jurisdiction_str = jurisdiction_str,
        )
    }

    fn generate_fallback_narrative(
        &self,
        aggregation: &AggregationResult,
        jurisdiction: Jurisdiction,
        language: &str,
        scope3_breakdown: &HashMap<u8, Scope3CategorySummary>,
    ) -> String {
        let categories_covered = scope3_breakdown.len();
        let completeness_pct = (categories_covered as f32 / 15.0) * 100.0;
        
        let mut top_categories: Vec<&Scope3CategorySummary> = scope3_breakdown.values().collect();
        top_categories.sort_by(|a, b| b.tco2e.partial_cmp(&a.tco2e).unwrap());
        
        let top_cat_text = if let Some(cat) = top_categories.first() {
            format!("The largest Scope 3 contributor is Category {} ({}), accounting for {:.2} tCO2e.", 
                cat.cat_id, cat.cat_name, cat.tco2e)
        } else {
            "No Scope 3 emissions were identified in this inventory.".to_string()
        };

        match language {
            "de" => format!(
                "ZUSAMMENFASSUNG\n\nDiese Treibhausgasbilanz wurde gemäß GHG Protocol Corporate Standard und CSRD/ESRS E1 erstellt.\n\n\
                GESAMTEMISSIONEN: {:.2} tCO2e\n\
                - Scope 1: {:.2} tCO2e\n\
                - Scope 2 (Location-Based): {:.2} tCO2e\n\
                - Scope 3: {:.2} tCO2e\n\n\
                SCOPE 3 ANALYSE\n\n{}/15 Scope-3-Kategorien wurden identifiziert ({:.0}% Abdeckung). {}\n\n\
                DATENQUALITÄT\n\n{} Reihen mit hoher Konfidenz (grün), {} Reihen mit mittlerer Konfidenz (gelb). {} Reihen wurden in Quarantäne gestellt.\n\n\
                CSRD-KONFORMITÄT\n\nDieser Bericht wurde automatisiert erstellt. Für eine vollständige CSRD/ESRS E1 Konformität wird eine manuelle Überprüfung der Quarantäneeinträge und fehlenden Scope-3-Kategorien empfohlen.\n\n\
                ---\nDieser Text wurde automatisch generiert (Gemini API Fallback).",
                aggregation.total_tco2e, aggregation.scope1_tco2e, aggregation.scope2_lb_tco2e, aggregation.scope3_tco2e,
                categories_covered, completeness_pct, top_cat_text,
                aggregation.green_rows, aggregation.yellow_rows, aggregation.red_rows
            ),
            "hu" => format!(
                "VEZETŐI ÖSSZEFOGLALÓ\n\nEz az üvegházhatású gáz leltár a GHG Protocol vállalati szabvány és a CSRD/ESRS E1 szerint készült.\n\n\
                TELJES KIBOCSÁTÁS: {:.2} tCO2e\n\
                - Scope 1: {:.2} tCO2e\n\
                - Scope 2 (Location-Based): {:.2} tCO2e\n\
                - Scope 3: {:.2} tCO2e\n\n\
                SCOPE 3 ELEMZÉS\n\n{}/15 Scope 3 kategória került azonosításra ({:.0}% lefedettség). {}\n\n\
                ADATMINŐSÉG\n\n{} magas megbízhatóságú sor (zöld), {} közepes megbízhatóságú sor (sárga). {} sor karanténba került.\n\n\
                CSRD MEGFELELŐSÉG\n\nEz a jelentés automatikusan készült. A teljes CSRD/ESRS E1 megfelelőséghez javasolt a karanténbejegyzések és a hiányzó Scope 3 kategóriák manuális felülvizsgálata.\n\n\
                ---\nEz a szöveg automatikusan generálódott (Gemini API Fallback).",
                aggregation.total_tco2e, aggregation.scope1_tco2e, aggregation.scope2_lb_tco2e, aggregation.scope3_tco2e,
                categories_covered, completeness_pct, top_cat_text,
                aggregation.green_rows, aggregation.yellow_rows, aggregation.red_rows
            ),
            _ => format!(
                "EXECUTIVE SUMMARY\n\nThis greenhouse gas inventory has been prepared in accordance with the GHG Protocol Corporate Standard and CSRD/ESRS E1.\n\n\
                TOTAL EMISSIONS: {:.2} tCO2e\n\
                - Scope 1: {:.2} tCO2e\n\
                - Scope 2 (Location-Based): {:.2} tCO2e\n\
                - Scope 3: {:.2} tCO2e\n\n\
                SCOPE 3 ANALYSIS\n\n{}/15 Scope 3 categories were identified ({:.0}% coverage). {}\n\n\
                DATA QUALITY\n\n{} high-confidence rows (green), {} medium-confidence rows (yellow). {} rows were quarantined.\n\n\
                CSRD COMPLIANCE ASSESSMENT\n\nThis report was generated automatically. Full CSRD/ESRS E1 compliance requires manual review of quarantined entries and missing Scope 3 categories.\n\n\
                ---\nThis text was auto-generated (Gemini API Fallback).",
                aggregation.total_tco2e, aggregation.scope1_tco2e, aggregation.scope2_lb_tco2e, aggregation.scope3_tco2e,
                categories_covered, completeness_pct, top_cat_text,
                aggregation.green_rows, aggregation.yellow_rows, aggregation.red_rows
            ),
        }
    }
}
