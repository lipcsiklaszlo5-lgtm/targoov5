use super::super::pcaf_attribution::{AssetClass, AttributionMethod, PcafAttribution};

pub struct UseOfProceedsHandler;

impl UseOfProceedsHandler {
    pub fn calculate_attribution(
        outstanding_amount: f64,
        total_project_cost: f64,
    ) -> PcafAttribution {
        PcafAttribution::new(
            AssetClass::UseOfProceeds, 
            outstanding_amount, 
            Some(total_project_cost),
            AttributionMethod::DirectEvic,
            "Sustainability-Linked Loan Agreement".to_string()
        )
    }
}
