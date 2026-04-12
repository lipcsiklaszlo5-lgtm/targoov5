use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerComparison {
    pub client_carbon_intensity: f64,      // Client value
    pub industry_average: f64,             // Industry average
    pub percentile: Option<u8>,            // Percentile (1-100)
    pub performance_tier: PerformanceTier, // Tier
    pub improvement_potential: f64,        // tCO2e saving potential
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PerformanceTier {
    TopPerformer,      // Top 25% (lowest intensity)
    AboveAverage,      // 50-75%
    Average,           // 25-50%
    BelowAverage,      // 0-25% (highest intensity)
}

impl PeerComparison {
    pub fn new(client_intensity: f64, industry_avg: f64) -> Self {
        // Simplified percentile calculation based on ratio to average
        let ratio = if industry_avg > 0.0 { client_intensity / industry_avg } else { 1.0 };
        
        let (tier, percentile) = if ratio < 0.5 {
            (PerformanceTier::TopPerformer, 90)
        } else if ratio < 0.8 {
            (PerformanceTier::AboveAverage, 75)
        } else if ratio < 1.1 {
            (PerformanceTier::Average, 50)
        } else {
            (PerformanceTier::BelowAverage, 15)
        };

        let potential = if client_intensity > industry_avg {
            client_intensity - industry_avg
        } else {
            0.0
        };

        Self {
            client_carbon_intensity: client_intensity,
            industry_average: industry_avg,
            percentile: Some(percentile),
            performance_tier: tier,
            improvement_potential: potential,
        }
    }

    pub fn generate_narrative(&self) -> String {
        let diff_pct = if self.industry_average > 0.0 {
            ((self.client_carbon_intensity - self.industry_average) / self.industry_average * 100.0).abs()
        } else {
            0.0
        };

        let relation = if self.client_carbon_intensity <= self.industry_average {
            format!("{:.1}%-kal alacsonyabb", diff_pct)
        } else {
            format!("{:.1}%-kal magasabb", diff_pct)
        };

        let tier_desc = match self.performance_tier {
            PerformanceTier::TopPerformer => "a szektor felső 25%-ában (legjobban teljesítők) helyezkedik el",
            PerformanceTier::AboveAverage => "átlag feletti teljesítményt nyújt",
            PerformanceTier::Average => "átlagos piaci teljesítményt nyújt",
            PerformanceTier::BelowAverage => "fejlesztési potenciállal rendelkezik az iparági átlaghoz képest",
        };

        format!(
            "Az Ön carbon intensity értéke {:.1} tCO2e/M€, ami {} az iparági átlagnál ({:.1} tCO2e/M€). Ezzel a teljesítménnyel {}.",
            self.client_carbon_intensity,
            relation,
            self.industry_average,
            tier_desc
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_comparison_above_average() {
        let comp = PeerComparison::new(85.0, 120.0);
        assert_eq!(comp.performance_tier, PerformanceTier::AboveAverage);
        assert_eq!(comp.percentile, Some(75));
        
        let narrative = comp.generate_narrative();
        assert!(narrative.contains("29.2%-kal alacsonyabb"));
    }
    
    #[test]
    fn test_peer_comparison_below_average() {
        let comp = PeerComparison::new(150.0, 120.0);
        assert_eq!(comp.performance_tier, PerformanceTier::BelowAverage);
        assert!(comp.improvement_potential > 0.0);
    }
}
