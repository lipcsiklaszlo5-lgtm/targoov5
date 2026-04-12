use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustryBenchmark {
    pub sector: String,
    pub avg_carbon_intensity: f64,         // tCO2e / M€ revenue
    pub avg_energy_intensity: f64,         // kWh / M€ revenue
    pub avg_waste_intensity: f64,          // kg / M€ revenue
    pub avg_water_intensity: f64,          // m³ / M€ revenue
}

impl IndustryBenchmark {
    pub fn get_for_sector(sector: &str) -> Self {
        match sector {
            "Manufacturing" => Self {
                sector: sector.to_string(),
                avg_carbon_intensity: 120.0,
                avg_energy_intensity: 45000.0,
                avg_waste_intensity: 8500.0,
                avg_water_intensity: 1200.0,
            },
            "Financial" => Self {
                sector: sector.to_string(),
                avg_carbon_intensity: 25.0,
                avg_energy_intensity: 12000.0,
                avg_waste_intensity: 150.0,
                avg_water_intensity: 45.0,
            },
            "Energy" => Self {
                sector: sector.to_string(),
                avg_carbon_intensity: 850.0,
                avg_energy_intensity: 5000.0, // Energy to produce energy is different, but for intensity:
                avg_waste_intensity: 12000.0,
                avg_water_intensity: 25000.0,
            },
            "Technology" => Self {
                sector: sector.to_string(),
                avg_carbon_intensity: 35.0,
                avg_energy_intensity: 85000.0,
                avg_waste_intensity: 200.0,
                avg_water_intensity: 80.0,
            },
            "Retail" => Self {
                sector: sector.to_string(),
                avg_carbon_intensity: 45.0,
                avg_energy_intensity: 32000.0,
                avg_waste_intensity: 4500.0,
                avg_water_intensity: 350.0,
            },
            _ => Self {
                sector: "General".to_string(),
                avg_carbon_intensity: 100.0,
                avg_energy_intensity: 25000.0,
                avg_waste_intensity: 2500.0,
                avg_water_intensity: 500.0,
            },
        }
    }
}
