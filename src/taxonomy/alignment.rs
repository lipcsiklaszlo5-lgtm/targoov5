use serde::{Deserialize, Serialize};
use super::eligibility::TaxonomyEligibility;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxonomyAlignment {
    pub eligibility: TaxonomyEligibility,
    pub technical_screening_passed: bool,
    pub dnsh_passed: bool,
    pub minimum_safeguards_passed: bool,
    pub is_aligned: bool,
}

pub struct AlignmentChecker;

impl AlignmentChecker {
    pub fn check_alignment(eligibility: TaxonomyEligibility, criteria_met: bool) -> TaxonomyAlignment {
        let is_aligned = eligibility.is_eligible && criteria_met;
        
        TaxonomyAlignment {
            eligibility,
            technical_screening_passed: criteria_met,
            dnsh_passed: criteria_met,
            minimum_safeguards_passed: criteria_met,
            is_aligned,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taxonomy::eligibility::EligibilityChecker;

    #[test]
    fn test_wind_power_alignment() {
        let eligibility = EligibilityChecker::check_nace("D35.11");
        assert!(eligibility.is_eligible);
        
        let alignment = AlignmentChecker::check_alignment(eligibility, true);
        assert!(alignment.is_aligned);
        assert_eq!(alignment.eligibility.environmental_objective, "Climate Change Mitigation");
    }
}
