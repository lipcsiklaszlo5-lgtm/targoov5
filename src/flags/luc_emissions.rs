use serde::{Deserialize, Serialize};
use super::lsr_categories::{LandOwnership, LsrCategory};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LucCalculation {
    pub hectares: f64,
    pub carbon_loss_per_hectare: f64, // Total carbon lost during conversion
    pub amortization_period_years: u32, // Usually 20 years
    pub ownership: LandOwnership,
}

impl LucCalculation {
    pub fn new(hectares: f64, carbon_loss: f64, ownership: LandOwnership) -> Self {
        Self {
            hectares,
            carbon_loss_per_hectare: carbon_loss,
            amortization_period_years: 20, // Default 20 years per GHG Protocol LSR
            ownership,
        }
    }

    pub fn calculate_annual_emissions(&self) -> f64 {
        if self.amortization_period_years == 0 {
            return 0.0;
        }
        (self.hectares * self.carbon_loss_per_hectare) / (self.amortization_period_years as f64)
    }

    pub fn get_lsr_category(&self) -> LsrCategory {
        LsrCategory::LandUseChange
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_luc_deforestation_calc() {
        // Example: 10 hectares of forest cleared, 200 tCO2e loss per hectare
        let calc = LucCalculation::new(10.0, 200.0, LandOwnership::Owned);
        
        // Total loss = 2000 tCO2e
        // Annual (20 year amort) = 100 tCO2e
        let annual = calc.calculate_annual_emissions();
        assert_eq!(annual, 100.0);
    }
}
