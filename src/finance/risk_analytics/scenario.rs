use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioImpact {
    pub scenario_name: String,
    pub co2_price_2030: f64,
    pub annual_carbon_cost: f64,
    pub impact_on_portfolio_value_pct: f64,
}

pub struct ScenarioAnalyzer;

impl ScenarioAnalyzer {
    pub fn analyze(financed_emissions: f64, portfolio_value: f64) -> Vec<ScenarioImpact> {
        let scenarios = vec![
            ("Net Zero 2050", 150.0),
            ("Stated Policies", 75.0),
            ("Current Policies", 25.0),
        ];

        scenarios
            .into_iter()
            .map(|(name, price)| {
                let annual_cost = financed_emissions * price;
                let impact_pct = if portfolio_value > 0.0 {
                    (annual_cost / portfolio_value) * 100.0
                } else {
                    0.0
                };

                ScenarioImpact {
                    scenario_name: name.to_string(),
                    co2_price_2030: price,
                    annual_carbon_cost: annual_cost,
                    impact_on_portfolio_value_pct: impact_pct,
                }
            })
            .collect()
    }
}
