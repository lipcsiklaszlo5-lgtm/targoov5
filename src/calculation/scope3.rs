use crate::calculation::models::{CalculationError, CalculationResult};
use crate::calculation::CalculationEngine;
use crate::calculation::utils::calculate_tco2e;
use crate::models::GhgScope;
use crate::ingest::RawRow;

/// Scope 3 számítási logika kategóriák szerint szétválogatva.
pub fn calculate_scope3(
    engine: &CalculationEngine,
    category_id: u8,
    row: &RawRow,
) -> Result<CalculationResult, CalculationError> {
    // Kinyerjük az alap összeget és mértékegységet (vagy spend, vagy activity)
    let (amount, unit) = row.get_spend_amount_and_unit()
        .map_err(|e| CalculationError::InternalError(e.to_string()))?;

    match category_id {
        // Spend-Based kategóriák (Költségalapú számítás)
        1 | 2 | 8 | 10 | 14 => {
            if category_id == 1 && !is_currency(&unit) {
                calculate_activity_based(engine, category_id, amount, &unit)
            } else {
                calculate_spend_based(engine, category_id, amount, &unit)
            }
        },
        
        // Activity-Based kategóriák (Fizikai mértékegység alapú számítás)
        3 | 5 | 6 | 7 | 11 | 12 | 13 => {
            calculate_activity_based(engine, category_id, amount, &unit)
        },
        
        // Cat 4 & 9: Transportation (tkm alapú)
        4 | 9 => {
            calculate_transportation(engine, category_id, row, amount, &unit)
        },

        // Cat 15: Investments (PCAF 2025)
        15 => {
            calculate_pcaf_investment(engine, row, amount, &unit)
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
) -> Result<CalculationResult, CalculationError> {
    if !is_currency(unit) {
        return Err(CalculationError::InvalidUnit(format!(
            "A(z) {}. kategória spend-based számítást igényelne, de a mértékegység '{}' nem pénznem.",
            category_id, unit
        )));
    }
    let category_key = format!("scope3_cat_{}", category_id);
    let (ef, confidence) = engine.get_effective_emission_factor(GhgScope::SCOPE3, &category_key, unit)?;
    let gwp = engine.get_gwp_value("CO2");
    Ok(CalculationResult {
        tco2e: calculate_tco2e(amount, ef, gwp),
        confidence,
    })
}

/// Activity-Based (Tevékenységalapú) számítás implementálása
fn calculate_activity_based(
    engine: &CalculationEngine,
    category_id: u8,
    amount: f64,
    unit: &str,
) -> Result<CalculationResult, CalculationError> {
    let unit_cat = engine.unit_converter.detect_category(unit);
    if unit_cat == "unknown" || unit_cat == "currency" {
        return Err(CalculationError::InvalidUnit(format!(
            "A(z) {}. kategória activity-based számítást igényel, de a mértékegység '{}' nem fizikai egység.",
            category_id, unit
        )));
    }
    
    let target_unit = match unit_cat {
        "energy" => "kWh",
        "mass" => "kg",
        "volume" => "liter",
        "distance" => "km",
        _ => unit,
    };

    let converted_value = engine.unit_converter.convert(amount, unit, unit_cat)
        .map_err(|_| CalculationError::InvalidUnitConversion(unit.to_string(), target_unit.to_string()))?;

    let category_key = format!("scope3_cat_{}", category_id);
    let (ef, confidence) = engine.get_effective_emission_factor(GhgScope::SCOPE3, &category_key, target_unit)?;
    let gwp = engine.get_gwp_value("CO2");
    Ok(CalculationResult {
        tco2e: calculate_tco2e(converted_value, ef, gwp),
        confidence,
    })
}

/// Cat 4 & 9: Transportation számítás (tkm támogatással)
fn calculate_transportation(
    engine: &CalculationEngine,
    category_id: u8,
    row: &RawRow,
    amount: f64,
    unit: &str,
) -> Result<CalculationResult, CalculationError> {
    let final_tkm = if unit.to_lowercase() == "tkm" {
        amount
    } else {
        // Ha nem tkm jött, megpróbáljuk kinyerni a súlyt és a távolságot külön
        let weight_ton = if engine.unit_converter.detect_category(unit) == "mass" {
            engine.unit_converter.convert(amount, unit, "mass").map_err(|_| CalculationError::InvalidUnit(unit.to_string()))? / 1000.0
        } else {
            // Megpróbálunk egy 'weight' nevű mezőt keresni (tonnában várjuk)
            row.get_value_as_f64("weight").unwrap_or(0.0)
        };

        let distance_km = if engine.unit_converter.detect_category(unit) == "distance" {
            engine.unit_converter.convert(amount, unit, "distance").map_err(|_| CalculationError::InvalidUnit(unit.to_string()))?
        } else {
            row.get_value_as_f64("distance").unwrap_or(0.0)
        };

        if weight_ton > 0.0 && distance_km > 0.0 {
            weight_ton * distance_km
        } else {
            return Err(CalculationError::InvalidUnit(format!("Transportation kategóriához tkm vagy súly+távolság szükséges. Egység: {}", unit)));
        }
    };

    let category_key = format!("scope3_cat_{}", category_id);
    let (ef, confidence) = engine.get_effective_emission_factor(GhgScope::SCOPE3, &category_key, "tkm")?;
    let gwp = engine.get_gwp_value("CO2");
    
    Ok(CalculationResult {
        tco2e: calculate_tco2e(final_tkm, ef, gwp),
        confidence,
    })
}

/// Cat 15: Investments (PCAF 2025 standard)
fn calculate_pcaf_investment(
    engine: &CalculationEngine,
    row: &RawRow,
    outstanding_amount: f64,
    unit: &str,
) -> Result<CalculationResult, CalculationError> {
    // 1. Attribution Factor számítás
    // PCAF szerint: Outstanding / EVIC (vagy Equity + Debt)
    let evic = row.get_value_as_f64("evic");
    let denominator = if let Some(e) = evic {
        e
    } else {
        let total_equity = row.get_value_as_f64("total_equity").unwrap_or(0.0);
        let total_debt = row.get_value_as_f64("total_debt").unwrap_or(0.0);
        total_equity + total_debt
    };

    let attribution_factor = if denominator > 0.0 {
        outstanding_amount / denominator
    } else {
        1.0 // Konzervatív becslés
    };

    // 2. EF lekérése
    let category_key = "scope3_cat_15";
    let (ef, confidence) = engine.get_effective_emission_factor(GhgScope::SCOPE3, category_key, unit)?;
    let gwp = engine.get_gwp_value("CO2");

    // Ha az EF "kgCO2e/Currency" formátumú (EEIO), akkor az outstanding_amount-ra vetítjük
    let total_tco2e = calculate_tco2e(outstanding_amount, ef, gwp);
    
    // Alkalmazzuk az attribution factort
    Ok(CalculationResult {
        tco2e: total_tco2e * attribution_factor,
        confidence,
    })
}

/// Segédfüggvény: a mértékegység pénznem-e
fn is_currency(unit: &str) -> bool {
    let u = unit.to_uppercase();
    matches!(u.as_str(), "USD" | "EUR" | "HUF" | "GBP" | "CHF" | "PLN" | "CZK")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calculation::CalculationEngine;
    use crate::config::models::ValidatedConfig;
    use crate::ingest::{RawRow, RawField};
    use std::collections::HashMap;

    fn create_mock_row(header: &str, value: f64, unit: &str) -> RawRow {
        let mut fields = HashMap::new();
        fields.insert(format!("{} [{}]", header, unit), RawField::Number(value));
        RawRow {
            source_file: "test.csv".to_string(),
            source_line: 1,
            sheet_name: None,
            fields,
            raw_bytes: None,
        }
    }

    #[tokio::test]
    async fn test_cat4_tkm_logic() {
        let config = crate::calculation::tests::create_mock_config(); // Reusing mock config helper
        let engine = CalculationEngine::new(&config);
        
        let row = create_mock_row("Freight", 500.0, "tkm");
        let result = calculate_scope3(&engine, 4, &row);
        
        // Mock EF for scope3_cat_4 [tkm] is 0.161 in database.json
        // tCO2e = (500 * 0.161 * 1.0) / 1000 = 0.0805
        assert!(result.is_ok());
        assert_eq!(result.unwrap().tco2e, 0.0805);
    }

    #[tokio::test]
    async fn test_cat1_spend_based_reverification() {
        let config = crate::calculation::tests::create_mock_config();
        let engine = CalculationEngine::new(&config);
        
        let row = create_mock_row("Steel Purchase", 2000.0, "EUR");
        let result = calculate_scope3(&engine, 1, &row);
        
        // Mock EF for scope3_cat_1 [EUR] is 1.234
        // tCO2e = (2000 * 1.234 * 1.0) / 1000 = 2.468
        assert!(result.is_ok());
        assert_eq!(result.unwrap().tco2e, 2.468);
    }
}
