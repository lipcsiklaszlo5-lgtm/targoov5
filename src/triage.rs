use crate::ingest::RawRow;
use crate::ai_client::AiBridgeClient;
use crate::models::{CalcPath, GhgScope, Jurisdiction, MatchMethod, Scope3Category};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use strsim::normalized_levenshtein;

pub const FUZZY_THRESHOLD: f64 = 0.85;

/// Represents a single entry in the dictionary.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionaryEntry {
    pub keyword: String,
    pub language: String,    // ✅ ÚJ! "EN", "DE", "HU"
    pub ghg_category: String, // "Scope1", "Scope2", "Scope3"
    #[serde(default)]
    pub scope3_id: Option<u8>, // 1-15 if Scope3
    #[serde(default)]
    pub scope3_name: Option<String>,
    #[serde(default)]
    pub calc_path: Option<String>, // "ActivityBased" | "SpendBased"
    pub canonical_unit: String,
    pub ef_value: f64,
    pub ef_unit: String,
    #[serde(default)]
    pub ef_source: String,
    #[serde(default)]
    pub ef_jurisdiction: Option<String>, // "US", "UK", "EU", "GLOBAL"
    pub industry: String,
    pub languages: Vec<String>,
    pub confidence_default: f32,
}

fn detect_language(keyword: &str) -> String {
    // Egyszerű heurisztika:
    // - Ha tartalmaz német specifikus karaktereket (ä, ö, ü, ß) -> "DE"
    // - Ha tartalmaz magyar ékezeteket (á, é, í, ó, ö, ő, ú, ü, ű) -> "HU"
    // - Egyébként -> "EN"
    let k = keyword.to_lowercase();
    if k.contains('ä') || k.contains('ö') || k.contains('ü') || k.contains('ß') {
        "DE".to_string()
    } else if k.contains('á') || k.contains('é') || k.contains('í') || k.contains('ó') || k.contains('ő') || k.contains('ú') || k.contains('ű') {
        "HU".to_string()
    } else {
        "EN".to_string()
    }
}

#[derive(Debug, Clone)]
pub struct TriageResult {
    pub ghg_scope: GhgScope,
    pub ghg_category: String,
    pub scope3_id: Option<u8>,
    pub scope3_name: Option<String>,
    pub calc_path: Option<CalcPath>,
    pub canonical_unit: String,
    pub ef_value: f64,
    pub ef_jurisdiction: Jurisdiction,
    pub match_method: MatchMethod,
    pub confidence: f32,
    pub matched_keyword: String,
}

use crate::config::models::ValidatedConfig;
use std::sync::Arc;

#[derive(Clone)]
pub struct TriageEngine {
    entries: Vec<DictionaryEntry>,
    // Optimization: Pre-computed lowercased keywords for exact matching
    exact_index: HashMap<String, DictionaryEntry>,
    // Priority buckets for iterative lookup
    scope1_2_entries: Vec<DictionaryEntry>,
    scope3_entries: Vec<DictionaryEntry>,
    ai_client: Arc<AiBridgeClient>,
    pub allow_ai: bool, // ✅ ÚJ! Control AI fallback
    pub ef_source: String,
    pub dictionary_paths: Vec<String>,
}

impl TriageEngine {
    pub fn new(cfg: &ValidatedConfig) -> Self {
        let mut engine = Self {
            entries: Vec::new(),
            exact_index: HashMap::new(),
            scope1_2_entries: Vec::new(),
            scope3_entries: Vec::new(),
            ai_client: Arc::new(AiBridgeClient::new_with_config(
                &cfg.config.ai.embedding_url,
                cfg.config.ai.embedding_timeout_ms
            )),
            allow_ai: cfg.config.ai.gemini_enabled,
            ef_source: cfg.ef_source.clone(),
            dictionary_paths: cfg.dictionary_paths.clone(),
        };

        // Automatikus szótár betöltés a konfig alapján
        for path in &cfg.dictionary_paths {
            if let Ok(json) = std::fs::read_to_string(path) {
                if let Err(e) = engine.load_from_json(&json) {
                    eprintln!("[Triage] Hiba a szótár betöltésekor ({}): {}", path, e);
                } else {
                    eprintln!("[Triage] Szótár betöltve: {}", path);
                }
            }
        }

        engine
    }

    pub fn with_client(ai_client: Arc<AiBridgeClient>) -> Self {
        Self {
            entries: Vec::new(),
            exact_index: HashMap::new(),
            scope1_2_entries: Vec::new(),
            scope3_entries: Vec::new(),
            ai_client,
            allow_ai: true,
            ef_source: "DEFAULT".to_string(),
            dictionary_paths: Vec::new(),
        }
    }

    pub fn load_from_json(&mut self, json: &str) -> Result<()> {
        let entries: Vec<DictionaryEntry> = serde_json::from_str(json)?;
        self.entries = entries;
        self.rebuild_indexes();
        Ok(())
    }

    fn rebuild_indexes(&mut self) {
        self.exact_index.clear();
        self.scope1_2_entries.clear();
        self.scope3_entries.clear();

        for entry in &self.entries {
            self.exact_index.insert(entry.keyword.to_lowercase(), entry.clone());
            if entry.ghg_category == "Scope3" {
                self.scope3_entries.push(entry.clone());
            } else {
                self.scope1_2_entries.push(entry.clone());
            }
        }
    }

    /// Normalizes a header for matching (lowercase, trim, remove special chars)
    fn normalize_header(header: &str) -> String {
        header
            .to_lowercase()
            .replace('_', " ")
            .replace('-', " ")
            .replace('.', " ")
            .replace('/', " ")
            .trim()
            .to_string()
    }

    /// Main triage logic for a single header string
    pub async fn triage_header(&mut self, raw_header: &str, raw_row: Option<&RawRow>) -> Option<TriageResult> {
        let normalized = Self::normalize_header(raw_header);
        if normalized.is_empty() {
            return None;
        }

        // 1. Exact match in index
        if let Some(entry) = self.exact_index.get(&normalized) {
            return Some(self.build_result(entry, &normalized, 1.0, MatchMethod::Exact));
        }

        // 2. Keyword containment (robust heuristic)
        for entry in &self.scope1_2_entries {
            let kw = entry.keyword.to_lowercase();
            if normalized.contains(&kw) || kw.contains(&normalized) {
                return Some(self.build_result(entry, &entry.keyword, 0.95, MatchMethod::Exact));
            }
        }

        for entry in &self.scope3_entries {
            let kw = entry.keyword.to_lowercase();
            if normalized.contains(&kw) || kw.contains(&normalized) {
                return Some(self.build_result(entry, &entry.keyword, 0.9, MatchMethod::Exact));
            }
        }

        // 3. Scope 1/2 Fuzzy Match
        let best_scope1_2 = self.fuzzy_match(&self.scope1_2_entries, &normalized);
        if let Some((entry, score)) = best_scope1_2 {
            if score >= FUZZY_THRESHOLD {
                return Some(self.build_result(
                    entry,
                    &entry.keyword,
                    entry.confidence_default * (score as f32),
                    MatchMethod::Fuzzy,
                ));
            }
        }

        // 4. Scope 3 Fuzzy Match
        let best_scope3 = self.fuzzy_match(&self.scope3_entries, &normalized);
        if let Some((entry, score)) = best_scope3 {
            if score >= FUZZY_THRESHOLD {
                return Some(self.build_result(
                    entry,
                    &entry.keyword,
                    entry.confidence_default * (score as f32),
                    MatchMethod::Fuzzy,
                ));
            }
        }

        // 5. AI Bridge Fallback (LIVING DICTIONARY)
        if self.allow_ai {
            if let Ok(ai_resp) = self.ai_client.classify(raw_header).await {
                const AI_CONFIDENCE_HIGH: f32 = 0.75;
                const AI_CONFIDENCE_MEDIUM: f32 = 0.4;

                if ai_resp.matched && ai_resp.confidence >= AI_CONFIDENCE_MEDIUM {
                    let ghg_category = ai_resp.ghg_category.unwrap_or_else(|| "Scope3".to_string());
                    let lang = detect_language(raw_header);
                    
                    // Adjust confidence for Medium tier
                    let final_confidence = if ai_resp.confidence >= AI_CONFIDENCE_HIGH {
                        ai_resp.confidence
                    } else {
                        ai_resp.confidence * 0.9 // Slightly penalized for estimated tier
                    };

                    let entry = DictionaryEntry {
                        keyword: raw_header.to_string(),
                        language: lang.clone(),
                        ghg_category: ghg_category.clone(),
                        scope3_id: ai_resp.scope3_id,
                        scope3_name: ai_resp.scope3_name.clone(),
                        calc_path: ai_resp.calc_path.clone(),
                        canonical_unit: ai_resp.canonical_unit.clone().unwrap_or_else(|| "unit".to_string()),
                        ef_value: ai_resp.ef_value.unwrap_or(0.0),
                        ef_unit: format!("kgCO2e/{}", ai_resp.canonical_unit.unwrap_or_else(|| "unit".to_string())),
                        ef_source: "AI-Generated".to_string(),
                        ef_jurisdiction: Some("GLOBAL".to_string()),
                        industry: "General".to_string(),
                        languages: vec![lang],
                        confidence_default: final_confidence,
                    };

                    // a) Save to disk (append to dictionary.json) - Only for High confidence
                    if ai_resp.confidence >= AI_CONFIDENCE_HIGH {
                        let _ = self.save_entry_to_dictionary(&entry);
                    }

                    // b) Update in-memory
                    self.entries.push(entry.clone());
                    self.rebuild_indexes();

                    return Some(self.build_result(&entry, raw_header, final_confidence, MatchMethod::Semantic));
                }
            }
        }

        // 6. CONTEXT-AWARE FALLBACK (Infer from other columns)
        if self.allow_ai {
            if let Some(row) = raw_row {
                if let Some((activity, confidence)) = crate::triage_context::infer_activity_from_row(row, &self.ai_client).await {
                    // Now that we have a potential activity string, we re-run triage but without the row context
                    // to avoid infinite recursion and use our regular Exact/Fuzzy/AI logic.
                    if let Some(mut result) = Box::pin(self.triage_header(&activity, None)).await {
                        result.confidence *= confidence; // Penalize by inference confidence
                        result.match_method = MatchMethod::Inferred;
                        return Some(result);
                    }
                }
            }
        }

        None
    }

    fn fuzzy_match<'a>(
        &self,
        entries: &'a [DictionaryEntry],
        header: &str,
    ) -> Option<(&'a DictionaryEntry, f64)> {
        let mut best: Option<(&DictionaryEntry, f64)> = None;

        for entry in entries {
            let score = normalized_levenshtein(&entry.keyword.to_lowercase(), header);
            if score > best.map(|(_, s)| s).unwrap_or(0.0) {
                best = Some((entry, score));
            }
        }

        best
    }

    fn save_entry_to_dictionary(&self, entry: &DictionaryEntry) -> Result<()> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open("data/dictionary.json")?;

        let mut entries: Vec<DictionaryEntry> = serde_json::from_reader(&file)?;
        
        // Avoid duplicates
        if entries.iter().any(|e| e.keyword == entry.keyword) {
            return Ok(());
        }

        entries.push(entry.clone());
        
        // Rewind and overwrite
        file.set_len(0)?;
        use std::io::Seek;
        file.seek(std::io::SeekFrom::Start(0))?;
        serde_json::to_writer_pretty(file, &entries)?;
        
        Ok(())
    }

    fn build_result(
        &self,
        entry: &DictionaryEntry,
        matched_keyword: &str,
        confidence: f32,
        method: MatchMethod,
    ) -> TriageResult {
        let ghg_scope = match entry.ghg_category.as_str() {
            "Scope1" => GhgScope::SCOPE1,
            "Scope2" => GhgScope::Scope2Lb,
            _ => GhgScope::SCOPE3,
        };

        let calc_path = match entry.calc_path.as_deref() {
            Some("SpendBased") => Some(CalcPath::SpendBased),
            Some("Pcaf") => Some(CalcPath::Pcaf),
            _ => Some(CalcPath::ActivityBased),
        };

        let ef_jurisdiction = match entry.ef_jurisdiction.as_deref() {
            Some("UK") => Jurisdiction::UK,
            Some("EU") => Jurisdiction::EU,
            Some("DE") => Jurisdiction::DE,
            Some("AT") => Jurisdiction::AT,
            Some("CH") => Jurisdiction::CH,
            Some("HU") => Jurisdiction::HU,
            Some("GLOBAL") => Jurisdiction::GLOBAL,
            _ => Jurisdiction::US,
        };

        TriageResult {
            ghg_scope,
            ghg_category: entry.ghg_category.clone(),
            scope3_id: entry.scope3_id,
            scope3_name: entry.scope3_name.clone(),
            calc_path,
            canonical_unit: entry.canonical_unit.clone(),
            ef_value: entry.ef_value,
            ef_jurisdiction,
            match_method: method,
            confidence,
            matched_keyword: matched_keyword.to_string(),
        }
    }
}

impl Default for TriageEngine {
    fn default() -> Self {
        Self::with_client(Arc::new(AiBridgeClient::new()))
    }
}
