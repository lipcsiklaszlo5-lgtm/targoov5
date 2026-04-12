use super::super::pcaf_attribution::{AssetClass, AttributionMethod, PcafAttribution};

pub struct MortgagesHandler;

impl MortgagesHandler {
    pub fn calculate_attribution(
        outstanding_amount: f64,
        property_value: f64, // At origination
    ) -> PcafAttribution {
        PcafAttribution::new(
            AssetClass::Mortgages, 
            outstanding_amount, 
            Some(property_value),
            AttributionMethod::ProxyEvic,
            "Mortgage Appraisal Document".to_string()
        )
    }
}
