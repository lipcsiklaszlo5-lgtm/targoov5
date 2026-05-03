use crate::models::{GhgScope, Jurisdiction, QuarantineReason};
use anyhow::{anyhow, Result};
use std::collections::HashMap;

// Deprecated: use ef_database instead
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
        distance_to_km.insert("tkm".to_string(), 1.0);
        distance_to_km.insert("night".to_string(), 1.0);
        distance_to_km.insert("nights".to_string(), 1.0);
        distance_to_km.insert("hour".to_string(), 1.0);
        distance_to_km.insert("hours".to_string(), 1.0);

        Self {
            energy_to_kwh,
            mass_to_kg,
            volume_to_l,
            distance_to_km,
        }
    }

    pub fn convert(&self, value: f64, from_unit: &str, target_category: &str) -> Result<f64> {
        let unit_lower = from_unit.trim().to_lowercase();

        let conversion_map = match target_category {
            "energy" => &self.energy_to_kwh,
            "mass" => &self.mass_to_kg,
            "volume" => &self.volume_to_l,
            "distance" => &self.distance_to_km,
            "currency" => return Ok(value),
            _ => return Err(anyhow!("Unknown target category for conversion: {}", target_category)),
        };

        conversion_map
            .get(&unit_lower)
            .map(|factor| value * factor)
            .ok_or_else(|| anyhow!("Unsupported unit '{}' for category '{}'", from_unit, target_category))
    }

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

    pub fn to_usd(&self, value: f64, currency: &str) -> Result<f64> {
        let curr_lower = currency.trim().to_lowercase();
        match curr_lower.as_str() {
            "usd" | "$" => Ok(value),
            "eur" | "€" => Ok(value / FX_EUR_TO_USD),
            "gbp" | "£" => Ok(value / FX_GBP_TO_USD),
            "huf" | "ft" => Ok(value / 365.0),
            _ => Err(anyhow!("Unsupported currency: {}", currency)),
        }
    }
}

impl Default for UnitConverter {
    fn default() -> Self {
        Self::new()
    }
}

pub fn tco2e_calculator(
    value: f64,
    category: &str,
    scope: GhgScope,
    jurisdiction: Jurisdiction,
    unit: &str,
    spend_usd: Option<f64>,
    eeio_ef: Option<f64>,
    attribution_factor: Option<f64>,
    borrower_tco2e: Option<f64>,
) -> f64 {
    if let Some(attr) = attribution_factor {
        let financed = borrower_tco2e.unwrap_or(0.0);
        return financed * attr;
    }

    // SpendBased logic
    if spend_usd.is_some() {
        if let (Some(usd), Some(ef)) = (spend_usd, eeio_ef) {
            return (usd * ef) / 1000.0;
        }
    }

    // Use ef_database
    let ef_kgco2e = crate::ef_database::get_emission_factor(scope, category, jurisdiction, unit).unwrap_or(0.0);
    let gwp = crate::ef_database::get_gwp("co2").unwrap_or(1.0);

    (value * ef_kgco2e * gwp) / 1000.0
}

pub fn validate_range_guard(
    tco2e: f64,
    scope: GhgScope,
) -> Result<(), QuarantineReason> {
    if tco2e.is_nan() || tco2e.is_infinite() || tco2e < 0.0 {
        return Err(QuarantineReason::RangeGuardFail);
    }
    if tco2e == 0.0 {
        return Ok(());
    }

    const ABS_MAX: f64 = 50_000_000.0;
    if tco2e > ABS_MAX {
        return Err(QuarantineReason::RangeGuardFail);
    }

    Ok(())
}
