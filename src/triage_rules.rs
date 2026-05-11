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

    // Egység kinyerése a fejléc végéből is (pl. "Road_Freight_tkm" -> "tkm")
    let unit_from_header = header_lower
        .split('_')
        .last()
        .unwrap_or("")
        .to_string();
    let effective_unit = if unit_lower.is_empty() { &unit_from_header } else { &unit_lower };

    // Stratégia 1: Unit-based inference
    if let Some(result) = infer_from_unit(&header_lower, effective_unit) {
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
    // Refrigerant / Kältemittel / Hűtőközeg
    if header.contains("refrigerant") || header.contains("kältemittel") || header.contains("coolant")
        || header.contains("r410") || header.contains("r134") || header.contains("r22")
        || header.contains("hűtő") || header.contains("kälte") {
        return Some(InferredCategory {
            ghg_scope: GhgScope::SCOPE1,
            ghg_category: "Scope1".to_string(),
            scope3_id: None,
            calc_path: CalcPath::ActivityBased,
            canonical_unit: "kg".to_string(),
            confidence: 0.88,
            reason: "keyword: refrigerant/kältemittel → Scope1 fugitive".to_string(),
        });
    }

    // District Heat / Fernwärme / Távfűtés
    if header.contains("district") || header.contains("fernwärme") || header.contains("távfűtés")
        || header.contains("heat") || header.contains("steam") || header.contains("wärme")
        || header.contains("dampf") || header.contains("gőz") {
        return Some(InferredCategory {
            ghg_scope: GhgScope::SCOPE1,
            ghg_category: "Scope1".to_string(),
            scope3_id: None,
            calc_path: CalcPath::ActivityBased,
            canonical_unit: "kWh".to_string(),
            confidence: 0.82,
            reason: "keyword: district_heat/fernwärme → Scope1".to_string(),
        });
    }

    // Gasoline / Petrol / Benzin
    if header.contains("gasoline") || header.contains("petrol") || header.contains("benzin")
        || header.contains("benzol") || header.contains("benzinkut") {
        return Some(InferredCategory {
            ghg_scope: GhgScope::SCOPE1,
            ghg_category: "Scope1".to_string(),
            scope3_id: None,
            calc_path: CalcPath::ActivityBased,
            canonical_unit: "liter".to_string(),
            confidence: 0.85,
            reason: "keyword: gasoline/petrol/benzin".to_string(),
        });
    }

    // Steel / Plastic / Electronics purchase
    if header.contains("steel") || header.contains("stahl") || header.contains("acél")
        || header.contains("plastic") || header.contains("resin") || header.contains("electronics")
        || header.contains("purchase") || header.contains("einkauf") || header.contains("procurement") {
        return Some(InferredCategory {
            ghg_scope: GhgScope::SCOPE3,
            ghg_category: "Scope3".to_string(),
            scope3_id: Some(1),
            calc_path: CalcPath::SpendBased,
            canonical_unit: "EUR".to_string(),
            confidence: 0.80,
            reason: "keyword: steel/plastic/electronics → Cat1".to_string(),
        });
    }

    // Flight / Air travel
    if header.contains("flight") || header.contains("flug") || header.contains("repülő")
        || header.contains("air_travel") || header.contains("aviation") {
        return Some(InferredCategory {
            ghg_scope: GhgScope::SCOPE3,
            ghg_category: "Scope3".to_string(),
            scope3_id: Some(6),
            calc_path: CalcPath::ActivityBased,
            canonical_unit: "km".to_string(),
            confidence: 0.85,
            reason: "keyword: flight/flug → Cat6".to_string(),
        });
    }

    // Train / Rail travel
    if header.contains("train") || header.contains("bahn") || header.contains("rail")
        || header.contains("vonat") || header.contains("zug") {
        return Some(InferredCategory {
            ghg_scope: GhgScope::SCOPE3,
            ghg_category: "Scope3".to_string(),
            scope3_id: Some(6),
            calc_path: CalcPath::ActivityBased,
            canonical_unit: "km".to_string(),
            confidence: 0.82,
            reason: "keyword: train/bahn → Cat6".to_string(),
        });
    }

    // Taxi / Car travel
    if header.contains("taxi") || header.contains("car_travel") || header.contains("pkw")
        || header.contains("auto") {
        return Some(InferredCategory {
            ghg_scope: GhgScope::SCOPE3,
            ghg_category: "Scope3".to_string(),
            scope3_id: Some(6),
            calc_path: CalcPath::ActivityBased,
            canonical_unit: "km".to_string(),
            confidence: 0.80,
            reason: "keyword: taxi/car → Cat6".to_string(),
        });
    }

    // Transport
    if header.contains("transport") || header.contains("logistik") || header.contains("spedition")
        || header.contains("fuvar") || header.contains("szállít") || header.contains("freight")
        || header.contains("cargo") || header.contains("road") || header.contains("sea")
        || header.contains("air_cargo") {
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
