use crate::models::{CalcPath, GhgScope, Jurisdiction, MatchMethod, Scope3Category};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strsim::normalized_levenshtein;

pub const FUZZY_THRESHOLD: f64 = 0.85;

/// Represents a single entry in the dictionary.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionaryEntry {
    pub keyword: String,
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
    pub ef_jurisdiction: Option<String>, // "US", "UK", "EU", "GLOBAL"
    pub industry: String,
    pub languages: Vec<String>,
    pub confidence_default: f32,
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

pub struct TriageEngine {
    entries: Vec<DictionaryEntry>,
    // Optimization: Pre-computed lowercased keywords for exact matching
    exact_index: HashMap<String, DictionaryEntry>,
    // Priority buckets for iterative lookup
    scope1_2_entries: Vec<DictionaryEntry>,
    scope3_entries: Vec<DictionaryEntry>,
}

impl TriageEngine {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            exact_index: HashMap::new(),
            scope1_2_entries: Vec::new(),
            scope3_entries: Vec::new(),
        }
    }

    /// Loads dictionary from JSON string (embedded or read from disk)
    pub fn load_from_json(&mut self, json_str: &str) -> Result<()> {
        let entries: Vec<DictionaryEntry> = serde_json::from_str(json_str)?;
        self.entries = entries;

        // Build indexes
        for entry in &self.entries {
            let key = entry.keyword.to_lowercase();
            self.exact_index.insert(key, entry.clone());

            if entry.ghg_category == "Scope1" || entry.ghg_category == "Scope2" {
                self.scope1_2_entries.push(entry.clone());
            } else if entry.ghg_category == "Scope3" {
                self.scope3_entries.push(entry.clone());
            }
        }

        // Sort by priority: longer keywords first to prioritize specificity
        self.scope1_2_entries
            .sort_by(|a, b| b.keyword.len().cmp(&a.keyword.len()));
        self.scope3_entries
            .sort_by(|a, b| b.keyword.len().cmp(&a.keyword.len()));

        Ok(())
    }

    /// Normalizes a raw header string for matching
    pub fn normalize_header(input: &str) -> String {
        input
            .to_lowercase()
            .replace('_', " ")
            .replace('-', " ")
            .replace('.', " ")
            .trim()
            .to_string()
    }

    /// Main triage logic for a single header string
    pub fn triage_header(&self, raw_header: &str) -> Option<TriageResult> {
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

        // 3. Scope 1/2 Fuzzy Match (Prioritized to prevent Scope 3 misclassification)
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

        // 5. Currency Heuristic Fallback (SpendBased Cat 1)
        if self.is_currency_header(&normalized) {
            // Find the fallback entry for Cat 1 SpendBased USD
            let fallback_entry = self
                .scope3_entries
                .iter()
                .find(|e| e.scope3_id == Some(1) && e.calc_path.as_deref() == Some("SpendBased"))
                .cloned();

            if let Some(entry) = fallback_entry {
                return Some(self.build_result(
                    &entry,
                    &entry.keyword,
                    0.5, // Low confidence
                    MatchMethod::Inferred,
                ));
            }
        }

        None
    }

    fn fuzzy_match<'a>(
        &self,
        entries: &'a [DictionaryEntry],
        target: &str,
    ) -> Option<(&'a DictionaryEntry, f64)> {
        let mut best_match: Option<(&DictionaryEntry, f64)> = None;

        for entry in entries {
            let score = normalized_levenshtein(&entry.keyword.to_lowercase(), target);
            if score > best_match.map(|(_, s)| s).unwrap_or(0.0) {
                best_match = Some((entry, score));
            }
        }

        best_match
    }

    fn is_currency_header(&self, normalized: &str) -> bool {
        let currency_keywords = [
            "usd", "eur", "gbp", "$", "€", "£", "cost", "spend", "price", "amount", "betrag",
            "kosten", "preis", "summe", "összeg", "ár", "költség", "huf", "ft",
        ];
        let volume_keywords = ["gallon", "liter", "litre"];
        
        currency_keywords.iter().any(|kw| normalized.contains(kw)) && 
        !volume_keywords.iter().any(|kw| normalized.contains(kw))
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
            "Scope2" => GhgScope::SCOPE2_LB,
            "Scope3" => GhgScope::SCOPE3,
            _ => GhgScope::SCOPE3, // Fallback, should not happen with valid dict
        };

        let calc_path = entry.calc_path.as_ref().and_then(|cp| match cp.as_str() {
            "ActivityBased" => Some(CalcPath::ActivityBased),
            "SpendBased" => Some(CalcPath::SpendBased),
            "Pcaf" => Some(CalcPath::Pcaf),
            _ => None,
        });

        let ef_jurisdiction = entry
            .ef_jurisdiction
            .as_ref()
            .map(|j| match j.as_str() {
                "US" => Jurisdiction::US,
                "UK" => Jurisdiction::UK,
                "EU" => Jurisdiction::EU,
                _ => Jurisdiction::GLOBAL,
            })
            .unwrap_or(Jurisdiction::GLOBAL);

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
        Self::new()
    }
}
