use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarbonRiskMetrics {
    pub waci: f64,                  // Weighted Average Carbon Intensity
    pub carbon_footprint: f64,      // tCO2e / M€ invested
    pub high_carbon_exposure: f64,  // % of portfolio in high-intensity sectors
}

pub struct PortfolioAsset {
    pub investment_amount: f64,
    pub emissions_tco2e: f64,
    pub revenue_meur: f64,
}

impl CarbonRiskMetrics {
    pub fn calculate(assets: &[PortfolioAsset], total_portfolio_value: f64) -> Self {
        if total_portfolio_value <= 0.0 || assets.is_empty() {
            return Self {
                waci: 0.0,
                carbon_footprint: 0.0,
                high_carbon_exposure: 0.0,
            };
        }

        let mut total_weighted_intensity = 0.0;
        let mut total_financed_emissions = 0.0;
        let mut high_intensity_investment = 0.0;

        for asset in assets {
            let weight = asset.investment_amount / total_portfolio_value;
            let intensity = if asset.revenue_meur > 0.0 {
                asset.emissions_tco2e / asset.revenue_meur
            } else {
                0.0
            };

            total_weighted_intensity += weight * intensity;
            total_financed_emissions += asset.emissions_tco2e;

            if intensity > 500.0 {
                high_intensity_investment += asset.investment_amount;
            }
        }

        Self {
            waci: total_weighted_intensity,
            carbon_footprint: total_financed_emissions / (total_portfolio_value / 1_000_000.0),
            high_carbon_exposure: (high_intensity_investment / total_portfolio_value) * 100.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_metrics_calculation() {
        let assets = vec![
            PortfolioAsset {
                investment_amount: 1_000_000.0,
                emissions_tco2e: 600.0,
                revenue_meur: 1.0, // Intensity 600
            },
            PortfolioAsset {
                investment_amount: 1_000_000.0,
                emissions_tco2e: 100.0,
                revenue_meur: 1.0, // Intensity 100
            },
        ];
        let metrics = CarbonRiskMetrics::calculate(&assets, 2_000_000.0);
        
        assert_eq!(metrics.waci, 350.0);
        assert_eq!(metrics.carbon_footprint, 350.0);
        assert_eq!(metrics.high_carbon_exposure, 50.0);
    }
}
