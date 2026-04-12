use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FluctuationAnalysis {
    pub current_emissions: f64,        // Current financed emissions
    pub previous_emissions: f64,       // Previous period
    pub absolute_change: f64,          // Absolute change
    pub percentage_change: f64,        // Percentage change
    
    pub change_drivers: Vec<ChangeDriver>,  // Drivers of change
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeDriver {
    NewLoans,               // New loans
    LoanRepayment,          // Loan repayment
    CompanyEmissionsChange, // Change in company emissions
    EnergyEfficiency,       // Energy efficiency improvement
    PortfolioRebalancing,   // Portfolio rebalancing
    MethodologyChange,      // Change in methodology
}

impl FluctuationAnalysis {
    pub fn new(current: f64, previous: f64, drivers: Vec<ChangeDriver>) -> Self {
        let abs = current - previous;
        let pct = if previous != 0.0 {
            (abs / previous) * 100.0
        } else {
            0.0
        };

        Self {
            current_emissions: current,
            previous_emissions: previous,
            absolute_change: abs,
            percentage_change: pct,
            change_drivers: drivers,
        }
    }

    pub fn generate_narrative(&self) -> String {
        let direction = if self.absolute_change >= 0.0 { "increased" } else { "decreased" };
        let drivers_str = self.change_drivers.iter()
            .map(|d| match d {
                ChangeDriver::NewLoans => "new investments",
                ChangeDriver::LoanRepayment => "divestments or repayments",
                ChangeDriver::CompanyEmissionsChange => "changes in company performance",
                ChangeDriver::EnergyEfficiency => "energy efficiency improvements",
                ChangeDriver::PortfolioRebalancing => "portfolio rebalancing",
                ChangeDriver::MethodologyChange => "methodological updates",
            })
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "Financed emissions {} by {:.1}% ({:.2} tCO2e), primarily driven by {}.",
            direction,
            self.percentage_change.abs(),
            self.absolute_change.abs(),
            if drivers_str.is_empty() { "market fluctuations".to_string() } else { drivers_str }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fluctuation_calculation() {
        let analysis = FluctuationAnalysis::new(
            1125.0,
            1000.0,
            vec![ChangeDriver::NewLoans, ChangeDriver::CompanyEmissionsChange]
        );
        
        assert_eq!(analysis.percentage_change, 12.5);
        assert_eq!(analysis.absolute_change, 125.0);
        
        let narrative = analysis.generate_narrative();
        assert!(narrative.contains("increased by 12.5%"));
        assert!(narrative.contains("new investments"));
    }
}
