use super::super::pcaf_attribution::{AssetClass, AttributionMethod, PcafAttribution};

pub struct BusinessLoansHandler;

impl BusinessLoansHandler {
    pub fn calculate_attribution(
        outstanding_amount: f64,
        total_equity: f64,
        total_debt: f64,
    ) -> PcafAttribution {
        let total_value = total_equity + total_debt;
        PcafAttribution::new(
            AssetClass::BusinessLoans, 
            outstanding_amount, 
            Some(total_value),
            AttributionMethod::BookValue,
            "Company Financial Reports".to_string()
        )
    }
}
