use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxonomyEligibility {
    pub nace_code: String,
    pub activity_name: String,
    pub is_eligible: bool,
    pub environmental_objective: String,
}

pub struct EligibilityChecker;

impl EligibilityChecker {
    pub fn check_nace(nace_code: &str) -> TaxonomyEligibility {
        // Simplified mapping for MVP
        match nace_code {
            "D35.11" => TaxonomyEligibility {
                nace_code: nace_code.to_string(),
                activity_name: "Production of electricity".to_string(),
                is_eligible: true,
                environmental_objective: "Climate Change Mitigation".to_string(),
            },
            "J62.01" => TaxonomyEligibility {
                nace_code: nace_code.to_string(),
                activity_name: "Computer programming activities".to_string(),
                is_eligible: true,
                environmental_objective: "Climate Change Mitigation".to_string(),
            },
            _ => TaxonomyEligibility {
                nace_code: nace_code.to_string(),
                activity_name: "Other activity".to_string(),
                is_eligible: false,
                environmental_objective: "None".to_string(),
            },
        }
    }
}
