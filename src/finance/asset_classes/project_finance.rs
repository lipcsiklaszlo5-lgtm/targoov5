use super::super::pcaf_attribution::{AssetClass, AttributionMethod, PcafAttribution};

pub struct ProjectFinanceHandler;

impl ProjectFinanceHandler {
    pub fn calculate_attribution(
        outstanding_amount: f64,
        total_project_cost: f64, // At origination
        _is_on_balance_sheet: bool,
    ) -> PcafAttribution {
        PcafAttribution::new(
            AssetClass::ProjectFinance, 
            outstanding_amount, 
            Some(total_project_cost),
            AttributionMethod::BookValue,
            "Project Financial Plan".to_string()
        )
    }
}
