/// Rule-Based Inference Engine
/// Ha a szótár nem talál egyezést, ez a modul próbál következtetni
/// a fejléc szövegéből, mértékegységéből és kontextusából.

use crate::models::{CalcPath, GhgScope};

#[derive(Debug, Clone)]
pub struct InferredCategory {
    pub ghg_scope: GhgScope,
    pub ghg_category: String,
    pub scope3_id: Option<u8>,
    pub calc_path: CalcPath,
    pub canonical_unit: String,
    pub confidence: f32,
    pub reason: String,
}

/// Főbelépési pont — sorban próbálja a stratégiákat
pub fn infer_category(header: &str, unit: &str) -> Option<InferredCategory> {
    let header_lower = header.to_lowercase();
    let unit_lower = unit.to_lowercase();

    // Stratégia 1: Unit-based inference
    if let Some(result) = infer_from_unit(&header_lower, &unit_lower) {
        return Some(result);
    }

    // Stratégia 2: Keyword decomposition
    if let Some(result) = infer_from_keywords(&header_lower) {
        return Some(result);
    }

    None
}

/// Stratégia 1: Mértékegység alapján következtet
fn infer_from_unit(header: &str, unit: &str) -> Option<InferredCategory> {
    match unit {
        u if u.contains("tkm") || u.contains("t-km") => Some(InferredCategory {
            ghg_scope: GhgScope::SCOPE3,
            ghg_category: "Scope3".to_string(),
            scope3_id: Some(4),
            calc_path: CalcPath::ActivityBased,
            canonical_unit: "tkm".to_string(),
            confidence: 0.80,
            reason: "unit=tkm → Cat4/Cat9 Transport".to_string(),
        }),
        u if u.contains("nacht") || u.contains("night") || u.contains("éjszaka") => Some(InferredCategory {
            ghg_scope: GhgScope::SCOPE3,
            ghg_category: "Scope3".to_string(),
            scope3_id: Some(6),
            calc_path: CalcPath::ActivityBased,
            canonical_unit: "night".to_string(),
            confidence: 0.75,
            reason: "unit=night → Cat6 Business Travel".to_string(),
        }),
        u if u.contains("m2") || u.contains("m²") || u.contains("sqm") => {
            // Header alapján döntjük el Cat8 vs Cat13
            let cat_id = if header.contains("downstream") || header.contains("mieten") {
                13u8
            } else {
                8u8
            };
            Some(InferredCategory {
                ghg_scope: GhgScope::SCOPE3,
                ghg_category: "Scope3".to_string(),
                scope3_id: Some(cat_id),
                calc_path: CalcPath::ActivityBased,
                canonical_unit: "m2".to_string(),
                confidence: 0.72,
                reason: format!("unit=m2 → Cat{} Leasing", cat_id),
            })
        },
        _ => None,
    }
}

/// Stratégia 2: Fejléc kulcsszó darabolás
fn infer_from_keywords(header: &str) -> Option<InferredCategory> {
    // Transport
    if header.contains("transport") || header.contains("logistik") || header.contains("spedition")
        || header.contains("fuvar") || header.contains("szállít") {
        return Some(InferredCategory {
            ghg_scope: GhgScope::SCOPE3,
            ghg_category: "Scope3".to_string(),
            scope3_id: Some(4),
            calc_path: CalcPath::ActivityBased,
            canonical_unit: "tkm".to_string(),
            confidence: 0.78,
            reason: "keyword: transport/logistik".to_string(),
        });
    }

    // Hotel / Übernachtung
    if header.contains("hotel") || header.contains("übernacht") || header.contains("nacht")
        || header.contains("szállás") || header.contains("accommodation") {
        return Some(InferredCategory {
            ghg_scope: GhgScope::SCOPE3,
            ghg_category: "Scope3".to_string(),
            scope3_id: Some(6),
            calc_path: CalcPath::ActivityBased,
            canonical_unit: "night".to_string(),
            confidence: 0.78,
            reason: "keyword: hotel/übernachtung".to_string(),
        });
    }

    // Leasing / Miete
    if header.contains("leasing") || header.contains("miete") || header.contains("lager")
        || header.contains("bérleti") || header.contains("bérlet") {
        let cat_id = if header.contains("downstream") || header.contains("13") { 13u8 } else { 8u8 };
        return Some(InferredCategory {
            ghg_scope: GhgScope::SCOPE3,
            ghg_category: "Scope3".to_string(),
            scope3_id: Some(cat_id),
            calc_path: CalcPath::ActivityBased,
            canonical_unit: "m2".to_string(),
            confidence: 0.75,
            reason: format!("keyword: leasing/miete → Cat{}", cat_id),
        });
    }

    // Franchise
    if header.contains("franchise") || header.contains("lizenz") {
        return Some(InferredCategory {
            ghg_scope: GhgScope::SCOPE3,
            ghg_category: "Scope3".to_string(),
            scope3_id: Some(14),
            calc_path: CalcPath::SpendBased,
            canonical_unit: "EUR".to_string(),
            confidence: 0.75,
            reason: "keyword: franchise".to_string(),
        });
    }

    // Investment / Portfolio
    if header.contains("invest") || header.contains("portfolio") || header.contains("aktien") {
        return Some(InferredCategory {
            ghg_scope: GhgScope::SCOPE3,
            ghg_category: "Scope3".to_string(),
            scope3_id: Some(15),
            calc_path: CalcPath::Pcaf,
            canonical_unit: "EUR".to_string(),
            confidence: 0.75,
            reason: "keyword: investment/portfolio → Cat15".to_string(),
        });
    }

    // Waste / Abfall
    if header.contains("abfall") || header.contains("waste") || header.contains("müll")
        || header.contains("hulladék") || header.contains("recycl") {
        return Some(InferredCategory {
            ghg_scope: GhgScope::SCOPE3,
            ghg_category: "Scope3".to_string(),
            scope3_id: Some(5),
            calc_path: CalcPath::ActivityBased,
            canonical_unit: "tonne".to_string(),
            confidence: 0.80,
            reason: "keyword: abfall/waste".to_string(),
        });
    }

    // Water / Wasser
    if header.contains("wasser") || header.contains("water") || header.contains("víz") {
        return Some(InferredCategory {
            ghg_scope: GhgScope::SCOPE3,
            ghg_category: "Scope3".to_string(),
            scope3_id: Some(5),
            calc_path: CalcPath::ActivityBased,
            canonical_unit: "m3".to_string(),
            confidence: 0.72,
            reason: "keyword: wasser/water".to_string(),
        });
    }

    None
}
