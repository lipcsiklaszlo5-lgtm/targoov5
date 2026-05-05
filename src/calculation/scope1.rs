use crate::calculation::models::{CalculationError, CalculationResult};
use crate::calculation::CalculationEngine;
use crate::calculation::utils::calculate_tco2e;
use crate::models::GhgScope;

/// Scope 1: Közvetlen emissziók számítása.
pub fn calculate_scope1(
    engine: &CalculationEngine,
    category: &str,
    amount: f64,
    unit: &str,
    row: &crate::ingest::RawRow,
) -> Result<CalculationResult, CalculationError> {
    // 1. Megállapítjuk, hogy hűtőközegről vagy üzemanyagról van-e szó
    if is_refrigerant(category) {
        calculate_refrigerant(engine, category, amount, unit, row)
    } else {
        calculate_combustion(engine, category, amount, unit)
    }
}

/// Stacionárius és Mobil égés (Üzemanyagok)
fn calculate_combustion(
    engine: &CalculationEngine,
    category: &str,
    amount: f64,
    unit: &str,
) -> Result<CalculationResult, CalculationError> {
    // 1. Normalizálás a kanonikus egységre (kWh, liter, kg)
    let unit_cat = engine.unit_converter.detect_category(unit);
    
    // Ha ismeretlen a mértékegység típus, próbáljuk kitalálni a kategória alapján
    let target_unit = match category {
        "natural_gas" | "biomethane" => "kWh",
        "diesel" | "petrol" | "heating_oil" | "lpg" | "lng" | "cng" | "biodiesel" | "bioethanol" => "liter",
        "coal" | "coke" | "wood_pellets" | "biomass" => "kg",
        _ => "unit",
    };

    let converted_value = if unit_cat != "unknown" {
        engine.unit_converter.convert(amount, unit, unit_cat)
            .map_err(|_| CalculationError::InvalidUnitConversion(unit.to_string(), target_unit.to_string()))?
    } else {
        amount // Ha nem tudjuk konvertálni, feltételezzük a bemenetet jónak
    };

    // 2. EF lekérése
    let (ef, confidence) = engine.get_effective_emission_factor(GhgScope::SCOPE1, category, target_unit)?;

    // 3. GWP lekérése (Égés esetén alapértelmezett a CO2)
    let gwp = engine.get_gwp_value("CO2");

    // 4. Számítás
    Ok(CalculationResult {
        tco2e: calculate_tco2e(converted_value, ef, gwp),
        confidence,
    })
}

/// Fugitív emissziók (Hűtőközegek)
fn calculate_refrigerant(
    engine: &CalculationEngine,
    category: &str,
    amount: f64,
    unit: &str,
    row: &crate::ingest::RawRow,
) -> Result<CalculationResult, CalculationError> {
    // 1. Normalizálás kg-ra
    let converted_kg = if unit.to_lowercase() != "kg" {
        engine.unit_converter.convert(amount, unit, "mass")
            .map_err(|_| CalculationError::InvalidUnitConversion(unit.to_string(), "kg".to_string()))?
    } else {
        amount
    };

    // 2. Szivárgási ráta kezelése
    let leakage_rate = row.get_value_as_f64("leakage_rate").unwrap_or(1.0);
    let effective_kg = converted_kg * leakage_rate;

    // 3. GWP érték lekérése
    let gwp = engine.get_gwp_value(category);

    // 4. EF lekérése (többnyire 1.0 hűtőközegeknél, ha a GWP-t használjuk)
    let (ef, confidence) = engine.get_effective_emission_factor(GhgScope::SCOPE1, category, "kg")
        .unwrap_or((1.0, 0.7)); // Ha nincs EF, 0.7-es bizalmi szinttel számolunk proxyként

    Ok(CalculationResult {
        tco2e: (effective_kg * ef * gwp) / 1000.0,
        confidence,
    })
}

fn is_refrigerant(category: &str) -> bool {
    let cat = category.to_lowercase();
    // Hűtőközegek általában R-számmal kezdődnek vagy specifikus gázok
    cat.starts_with('r') || cat == "sf6" || cat == "n2o" || cat == "hfc" || cat == "pfc"
}
