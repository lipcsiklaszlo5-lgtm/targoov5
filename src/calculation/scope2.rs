use crate::calculation::models::{CalculationError, CalculationResult};
use crate::calculation::CalculationEngine;
use crate::calculation::utils::calculate_tco2e;
use crate::models::GhgScope;

/// Scope 2: Vásárolt energia számítása.
pub fn calculate_scope2(
    engine: &CalculationEngine,
    scope: GhgScope,
    category: &str,
    amount: f64,
    unit: &str,
    row: &crate::ingest::RawRow,
) -> Result<CalculationResult, CalculationError> {
    // 1. Normalizálás kWh-ra
    // A Scope 2 kanonikus egysége a kWh.
    let unit_cat = engine.unit_converter.detect_category(unit);
    
    if unit_cat != "energy" && unit_cat != "unknown" {
         return Err(CalculationError::InvalidUnit(format!(
            "Scope 2 számításhoz energia mértékegység szükséges (kWh, MWh, GJ stb.), de a kapott egység: '{}'",
            unit
        )));
    }

    let converted_kwh = if unit.to_lowercase() != "kwh" {
        engine.unit_converter.convert(amount, unit, "energy")
            .map_err(|_| CalculationError::InvalidUnitConversion(unit.to_string(), "kWh".to_string()))?
    } else {
        amount
    };

    // 2. EF lekérése
    // Market-Based esetén ellenőrizzük, hogy megújuló energiáról van-e szó
    let is_renewable = row.get_value_as_f64("renewable").unwrap_or(0.0) > 0.0 
        || row.fields.iter().any(|(h, v)| h.to_lowercase().contains("renewable") && v.to_string_lossy().to_lowercase() == "true");

    let (ef, confidence) = if scope == GhgScope::Scope2Mb && is_renewable {
        (0.0, 1.0) // Megújuló energia esetén 100% bizalommal 0 az EF
    } else {
        engine.get_effective_emission_factor(scope, category, "kWh")?
    };

    // 3. GWP lekérése (Scope 2 esetén alapértelmezett a CO2)
    let gwp = engine.get_gwp_value("CO2");

    // 4. Számítás
    Ok(CalculationResult {
        tco2e: calculate_tco2e(converted_kwh, ef, gwp),
        confidence,
    })
}
