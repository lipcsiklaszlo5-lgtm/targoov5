use serde::{Deserialize, Serialize};
use crate::models::Jurisdiction;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalRiskScore {
    pub water_stress_score: u8, // 1-5
    pub flood_risk_score: u8,
    pub heatwave_risk_score: u8,
    pub combined_risk_score: f32,
}

pub struct PhysicalRiskScorer;

impl PhysicalRiskScorer {
    pub fn score_by_jurisdiction(jurisdiction: Jurisdiction) -> PhysicalRiskScore {
        let (water, flood, heat) = match jurisdiction {
            Jurisdiction::US => (3, 4, 4),
            Jurisdiction::UK => (2, 5, 2),
            Jurisdiction::EU => (3, 3, 3),
            Jurisdiction::GLOBAL => (4, 4, 5),
        };

        PhysicalRiskScore {
            water_stress_score: water,
            flood_risk_score: flood,
            heatwave_risk_score: heat,
            combined_risk_score: (water + flood + heat) as f32 / 3.0,
        }
    }
}
