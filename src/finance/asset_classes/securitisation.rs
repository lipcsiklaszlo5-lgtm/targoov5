use super::super::pcaf_attribution::{AssetClass, AttributionMethod, PcafAttribution};

pub struct SecuritisationHandler;

impl SecuritisationHandler {
    pub fn calculate_attribution(
        outstanding_amount: f64,
        total_pool_value: f64,
    ) -> PcafAttribution {
        PcafAttribution::new(
            AssetClass::Securitisation, 
            outstanding_amount, 
            Some(total_pool_value),
            AttributionMethod::ProxyEvic,
            "Collateral Pool Audit Report".to_string()
        )
    }
}
