use crate::calculation::models::CalculationError;
use crate::calculation::CalculationEngine;
use crate::calculation::utils::calculate_tco2e;
use crate::models::GhgScope;

/// Scope 3 számítási logika kategóriák szerint szétválogatva.
pub fn calculate_scope3(
    engine: &CalculationEngine,
    category_id: u8,
    amount: f64,
    unit: &str,
) -> Result<f64, CalculationError> {
    match category_id {
        // Spend-Based kategóriák (Költségalapú számítás)
        1 | 2 | 8 | 10 | 14 => calculate_spend_based(engine, category_id, amount, unit),
        
        // Egyéb kategóriák (pl. Activity-based) egyelőre nincsenek implementálva
        3..=7 | 9 | 11..=13 | 15 => {
            unimplemented!("A(z) {}. kategória Activity-based számítása még nincs implementálva.", category_id)
        }
        
        _ => Err(CalculationError::InternalError(format!("Érvénytelen Scope 3 kategória ID: {}", category_id))),
    }
}

/// Spend-Based (Költségalapú) számítás implementálása
fn calculate_spend_based(
    engine: &CalculationEngine,
    category_id: u8,
    amount: f64,
    unit: &str,
) -> Result<f64, CalculationError> {
    // 1. Ellenőrizzük, hogy a mértékegység pénznem-e
    if !is_currency(unit) {
        return Err(CalculationError::InvalidUnit(format!(
            "A(z) {}. kategória spend-based számítást igényel, de a mértékegység '{}'.",
            category_id, unit
        )));
    }

    // 2. Emissziós faktor lekérése (kgCO2e / Currency)
    // Megjegyzés: a kategória nevét a specifikáció/szótár alapján képezzük le
    let category_key = format!("scope3_cat_{}", category_id);
    let ef = engine.get_effective_emission_factor(GhgScope::SCOPE3, &category_key, unit)?;

    // 3. GWP érték lekérése (Spend-based esetén általában a CO2 dominál, GWP=1, de a motor rugalmas)
    let gwp = engine.get_gwp_value("CO2");

    // 4. Végső tCO2e számítás
    Ok(calculate_tco2e(amount, ef, gwp))
}

/// Segédfüggvény: a mértékegység pénznem-e (ISO 4217 kódok alapján)
fn is_currency(unit: &str) -> bool {
    let u = unit.to_uppercase();
    matches!(u.as_str(), "USD" | "EUR" | "HUF" | "GBP" | "CHF" | "PLN" | "CZK")
}
