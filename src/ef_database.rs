use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use once_cell::sync::Lazy;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmissionFactorDatabase {
    pub gwp: HashMap<String, f64>,
    pub uk_defra_2024: serde_json::Value,
    pub us_epa_2024: serde_json::Value,
    pub useeio_v2_1: serde_json::Value,
    pub refrigerants: serde_json::Value,
    pub constants: serde_json::Value,
}

static DB: Lazy<EmissionFactorDatabase> = Lazy::new(|| {
    let path = "/workspaces/targoov5/data/efactors/database.json";
    let content = std::fs::read_to_string(path).expect("Failed to read EF database");
    serde_json::from_str(&content).expect("Failed to parse EF database")
});

pub fn get_gwp(gas: &str) -> Option<f64> {
    DB.gwp.get(gas).copied()
}

pub fn get_emission_factor(
    scope: crate::models::GhgScope,
    category: &str,
    jurisdiction: crate::models::Jurisdiction,
    unit: &str,
) -> Option<f64> {
    match scope {
        crate::models::GhgScope::SCOPE1 => {
            if jurisdiction == crate::models::Jurisdiction::EU || jurisdiction == crate::models::Jurisdiction::UK {
                match category {
                    "natural_gas" => DB.uk_defra_2024["natural_gas"]["totals"][unit].as_f64(),
                    "diesel" => DB.uk_defra_2024["liquid_fuels"]["diesel"]["combustion_litre"].as_f64(),
                    _ => None,
                }
            } else {
                None
            }
        }
        crate::models::GhgScope::Scope2Lb => {
            if jurisdiction == crate::models::Jurisdiction::EU || jurisdiction == crate::models::Jurisdiction::UK {
                DB.uk_defra_2024["electricity"]["location_based_kwh"].as_f64()
            } else if jurisdiction == crate::models::Jurisdiction::US {
                DB.us_epa_2024["egrid_2023"]["us_average"].as_f64()
            } else {
                None
            }
        }
        crate::models::GhgScope::SCOPE3 => {
            match category {
                "waste" => DB.uk_defra_2024["water_waste"]["waste"]["landfill_mixed_tonne"].as_f64(),
                _ => None,
            }
        }
        _ => None,
    }
}
