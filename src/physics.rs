use crate::models::{CalcPath, GhgScope, Jurisdiction, QuarantineReason, Scope3Category};
use anyhow::{anyhow, Result};
use std::collections::HashMap;

// Currency exchange rates to USD (as of spec)
pub const FX_EUR_TO_USD: f64 = 0.92;
pub const FX_GBP_TO_USD: f64 = 0.79;

#[derive(Clone)]
pub struct UnitConverter {
    // Base unit: kWh
    energy_to_kwh: HashMap<String, f64>,
    // Base unit: kg
    mass_to_kg: HashMap<String, f64>,
    // Base unit: liter
    volume_to_l: HashMap<String, f64>,
    // Base unit: km
    distance_to_km: HashMap<String, f64>,
}

impl UnitConverter {
    pub fn new() -> Self {
        let mut energy_to_kwh = HashMap::new();
        energy_to_kwh.insert("kwh".to_string(), 1.0);
        energy_to_kwh.insert("mwh".to_string(), 1000.0);
        energy_to_kwh.insert("gj".to_string(), 277.778);
        energy_to_kwh.insert("btu".to_string(), 0.000293071);
        energy_to_kwh.insert("therm".to_string(), 29.3071);
        energy_to_kwh.insert("mmbtu".to_string(), 293.071);

        let mut mass_to_kg = HashMap::new();
        mass_to_kg.insert("kg".to_string(), 1.0);
        mass_to_kg.insert("t".to_string(), 1000.0);
        mass_to_kg.insert("tonne".to_string(), 1000.0);
        mass_to_kg.insert("lb".to_string(), 0.453592);
        mass_to_kg.insert("lbs".to_string(), 0.453592);
        mass_to_kg.insert("short ton".to_string(), 907.185);
        mass_to_kg.insert("long ton".to_string(), 1016.05);

        let mut volume_to_l = HashMap::new();
        volume_to_l.insert("l".to_string(), 1.0);
        volume_to_l.insert("liter".to_string(), 1.0);
        volume_to_l.insert("liters".to_string(), 1.0);
        volume_to_l.insert("litre".to_string(), 1.0);
        volume_to_l.insert("litres".to_string(), 1.0);
        volume_to_l.insert("m3".to_string(), 1000.0);
        volume_to_l.insert("us gal".to_string(), 3.78541);
        volume_to_l.insert("gallon".to_string(), 3.78541);
        volume_to_l.insert("gallons".to_string(), 3.78541);
        volume_to_l.insert("uk gal".to_string(), 4.54609);
        volume_to_l.insert("barrel".to_string(), 158.987);
        volume_to_l.insert("bbl".to_string(), 158.987);

        let mut distance_to_km = HashMap::new();
        distance_to_km.insert("km".to_string(), 1.0);
        distance_to_km.insert("mile".to_string(), 1.60934);
        distance_to_km.insert("miles".to_string(), 1.60934);
        distance_to_km.insert("nautical mile".to_string(), 1.852);
        distance_to_km.insert("nm".to_string(), 1.852);
        distance_to_km.insert("tkm".to_string(), 1.0); // Tonne-km
        distance_to_km.insert("night".to_string(), 1.0); // For hotel
        distance_to_km.insert("nights".to_string(), 1.0);
        distance_to_km.insert("hour".to_string(), 1.0); // For WFH
        distance_to_km.insert("hours".to_string(), 1.0);

        Self {
            energy_to_kwh,
            mass_to_kg,
            volume_to_l,
            distance_to_km,
        }
    }

    /// Converts a raw numeric value and unit string to a canonical unit
    pub fn convert(&self, value: f64, from_unit: &str, target_category: &str) -> Result<f64> {
        let unit_lower = from_unit.trim().to_lowercase();

        let conversion_map = match target_category {
            "energy" => &self.energy_to_kwh,
            "mass" => &self.mass_to_kg,
            "volume" => &self.volume_to_l,
            "distance" => &self.distance_to_km,
            "currency" => return Ok(value), // Handled separately via FX rates
            _ => return Err(anyhow!("Unknown target category for conversion: {}", target_category)),
        };

        conversion_map
            .get(&unit_lower)
            .map(|factor| value * factor)
            .ok_or_else(|| anyhow!("Unsupported unit '{}' for category '{}'", from_unit, target_category))
    }

    /// Detects the physical category of a unit (energy, mass, volume, distance, currency)
    pub fn detect_category(&self, unit: &str) -> &'static str {
        let unit_lower = unit.trim().to_lowercase();
        if self.energy_to_kwh.contains_key(&unit_lower) {
            "energy"
        } else if self.mass_to_kg.contains_key(&unit_lower) {
            "mass"
        } else if self.volume_to_l.contains_key(&unit_lower) {
            "volume"
        } else if self.distance_to_km.contains_key(&unit_lower) {
            "distance"
        } else if ["usd", "eur", "gbp", "huf", "ft", "$", "€", "£"].contains(&unit_lower.as_str()) {
            "currency"
        } else {
            "unknown"
        }
    }

    /// Normalize currency to USD for SpendBased calculations
    pub fn to_usd(&self, value: f64, currency: &str) -> Result<f64> {
        let curr_lower = currency.trim().to_lowercase();
        match curr_lower.as_str() {
            "usd" | "$" => Ok(value),
            "eur" | "€" => Ok(value / FX_EUR_TO_USD),
            "gbp" | "£" => Ok(value / FX_GBP_TO_USD),
            "huf" | "ft" => Ok(value / 365.0), // Approximate rate for test
            _ => Err(anyhow!("Unsupported currency: {}", currency)),
        }
    }
}

impl Default for UnitConverter {
    fn default() -> Self {
        Self::new()
    }
}

/// Core tCO2e calculation formula.
/// **IMPORTANT**: Emission Factors (EF) are in kgCO2e.
/// The result MUST be divided by 1000.0 to convert kgCO2e -> tCO2e.
pub fn tco2e_calculator(
    value: f64,
    ef_kgco2e: f64,
    gwp: f64,
    is_spend_based: bool,
    spend_usd: Option<f64>,
    eeio_ef: Option<f64>,
    attribution_factor: Option<f64>,
    borrower_tco2e: Option<f64>,
) -> f64 {
    if let Some(attr) = attribution_factor {
        // Cat 15 PCAF calculation
        let financed = borrower_tco2e.unwrap_or(0.0);
        return financed * attr;
    }

    if is_spend_based {
        if let (Some(usd), Some(ef)) = (spend_usd, eeio_ef) {
            // SpendBased: (Spend_USD * EEIO_EF_kgCO2e_per_USD) / 1000.0
            return (usd * ef) / 1000.0;
        }
    }

    // Standard ActivityBased calculation
    // (value_in_canonical_unit * EF_kgCO2e * GWP) / 1000.0
    (value * ef_kgco2e * gwp) / 1000.0
}

/// Range Guard Validation for tCO2e values
pub fn validate_range_guard(
    tco2e: f64,
    scope: GhgScope,
    scope3_cat: Option<Scope3Category>,
) -> Result<(), QuarantineReason> {
    if tco2e.is_nan() || tco2e.is_infinite() || tco2e < 0.0 {
        return Err(QuarantineReason::RangeGuardFail);
    }

    // Allow exactly 0.0 (no impact rows)
    if tco2e == 0.0 {
        return Ok(());
    }

    // Absolute global maximum for any row
    const ABS_MAX: f64 = 50_000_000.0;
    if tco2e > ABS_MAX {
        return Err(QuarantineReason::RangeGuardFail);
    }

    match scope {
        GhgScope::SCOPE1 => {
            if tco2e > 50_000_000.0 {
                return Err(QuarantineReason::RangeGuardFail);
            }
        }
        GhgScope::SCOPE2_LB | GhgScope::SCOPE2_MB => {
            if tco2e > 50_000_000.0 {
                return Err(QuarantineReason::RangeGuardFail);
            }
        }
        GhgScope::SCOPE3 => {
            if tco2e > 50_000_000.0 {
                return Err(QuarantineReason::RangeGuardFail);
            }
        }
    }

    Ok(())
}
