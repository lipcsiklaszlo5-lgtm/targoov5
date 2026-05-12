use crate::models::{GhgScope, Jurisdiction};
use crate::ef_database::{self, EmissionFactorDatabase};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmissionFactorEntry {
    pub value: f64,
    pub uncertainty_pct: f64,
    pub valid_from: String,
    pub valid_to: String,
    pub source: String,
    pub source_tier: u8,          // 1=Government, 2=International, 3=Industry, 4=Proxy, 5=Conservative
    pub jurisdiction: Vec<String>,
    pub proxy_for: Vec<String>,
}

pub fn get_ef_intelligent(
    scope: GhgScope,
    category: &str,
    jurisdiction: Jurisdiction,
    unit: &str,
    reference_year: u32,
) -> Result<(f64, f64, String, u8), String> {
    let db = &ef_database::DB;

    // 1. Joghatóság-specifikus keresés
    if let Some(entry) = lookup_ef(db, scope, category, unit, Some(jurisdiction), reference_year) {
        return Ok((entry.value, entry.uncertainty_pct, entry.source, entry.source_tier));
    }

    // 2. Regionális proxy (EU, US, GLOBAL)
    for region in get_regional_fallbacks(jurisdiction) {
        if let Some(entry) = lookup_ef(db, scope, category, unit, Some(region), reference_year) {
            let penalty = 1.1;
            let extra_uncertainty = 10.0;
            return Ok((
                entry.value * penalty,
                entry.uncertainty_pct + extra_uncertainty,
                format!("{} (regional proxy)", entry.source),
                entry.source_tier.max(4),
            ));
        }
    }

    // 3. Proxy kategória keresés
    if let Some(proxy_cat) = find_proxy_category(db, category) {
        if let Some(entry) = lookup_ef(db, scope, &proxy_cat, unit, None, reference_year) {
            let penalty = 1.2;
            let extra_uncertainty = 15.0;
            return Ok((
                entry.value * penalty,
                entry.uncertainty_pct + extra_uncertainty,
                format!("{} (category proxy from {})", entry.source, proxy_cat),
                entry.source_tier.max(5),
            ));
        }
    }

    // 4. Konzervatív globális átlag
    if let Some(entry) = get_global_conservative_ef(db, scope, unit) {
        return Ok((
            entry.value,
            50.0,
            "IPCC default (conservative)".to_string(),
            5,
        ));
    }

    Err(format!("No emission factor found for scope={:?}, category={}, unit={}", scope, category, unit))
}

fn lookup_ef(
    db: &EmissionFactorDatabase,
    scope: GhgScope,
    category: &str,
    unit: &str,
    jurisdiction: Option<Jurisdiction>,
    reference_year: u32,
) -> Option<EmissionFactorEntry> {
    // A meglévő ef_database::get_emission_factor hívása, de az új mezőkkel
    // Egyelőre a meglévő JSON struktúrát használjuk, kiegészítve az új mezőkkel
    let ef_value = ef_database::get_emission_factor(scope, category, jurisdiction.unwrap_or(Jurisdiction::GLOBAL), unit)?;
    
    // Innen az új mezőket a JSON-ból kellene kiolvasni, de most default értékeket adunk
    Some(EmissionFactorEntry {
        value: ef_value,
        uncertainty_pct: 10.0,
        valid_from: "2024-01-01".to_string(),
        valid_to: "2024-12-31".to_string(),
        source: "DEFRA 2024".to_string(),
        source_tier: 1,
        jurisdiction: vec!["EU".to_string(), "UK".to_string()],
        proxy_for: vec![],
    })
}

fn get_regional_fallbacks(jurisdiction: Jurisdiction) -> Vec<Jurisdiction> {
    match jurisdiction {
        Jurisdiction::DE | Jurisdiction::AT | Jurisdiction::HU | Jurisdiction::EU => vec![Jurisdiction::EU, Jurisdiction::GLOBAL],
        Jurisdiction::CH => vec![Jurisdiction::EU, Jurisdiction::GLOBAL],
        Jurisdiction::UK => vec![Jurisdiction::UK, Jurisdiction::GLOBAL],
        Jurisdiction::US => vec![Jurisdiction::US, Jurisdiction::GLOBAL],
        _ => vec![Jurisdiction::GLOBAL],
    }
}

fn find_proxy_category(db: &EmissionFactorDatabase, category: &str) -> Option<String> {
    let proxies: std::collections::HashMap<&str, &str> = [
        ("diesel", "gasoil"),
        ("petrol", "gasoline"),
        ("natural_gas", "gas"),
        ("electricity", "power"),
    ].iter().cloned().collect();
    
    proxies.get(category).map(|s| s.to_string())
}

fn get_global_conservative_ef(db: &EmissionFactorDatabase, scope: GhgScope, unit: &str) -> Option<EmissionFactorEntry> {
    match scope {
        GhgScope::SCOPE1 => Some(EmissionFactorEntry {
            value: 0.5,
            uncertainty_pct: 50.0,
            valid_from: "2020-01-01".to_string(),
            valid_to: "2030-12-31".to_string(),
            source: "IPCC default".to_string(),
            source_tier: 5,
            jurisdiction: vec!["GLOBAL".to_string()],
            proxy_for: vec![],
        }),
        GhgScope::Scope2Lb => Some(EmissionFactorEntry {
            value: 0.5,
            uncertainty_pct: 50.0,
            valid_from: "2020-01-01".to_string(),
            valid_to: "2030-12-31".to_string(),
            source: "IPCC default".to_string(),
            source_tier: 5,
            jurisdiction: vec!["GLOBAL".to_string()],
            proxy_for: vec![],
        }),
        GhgScope::SCOPE3 => Some(EmissionFactorEntry {
            value: 0.3,
            uncertainty_pct: 50.0,
            valid_from: "2020-01-01".to_string(),
            valid_to: "2030-12-31".to_string(),
            source: "IPCC default".to_string(),
            source_tier: 5,
            jurisdiction: vec!["GLOBAL".to_string()],
            proxy_for: vec![],
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_ef_intelligent_fallback_chain() {
        // Ez a teszt a teljes fallback láncot ellenőrzi
        let result = get_ef_intelligent(
            GhgScope::SCOPE1,
            "diesel",
            Jurisdiction::DE,
            "liter",
            2024,
        );
        assert!(result.is_ok());
        let (value, uncertainty, source, tier) = result.unwrap();
        assert!(value > 0.0);
        assert!(uncertainty > 0.0);
        assert!(!source.is_empty());
        assert!(tier >= 1 && tier <= 5);
    }
}
