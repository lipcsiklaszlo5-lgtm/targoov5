use super::super::pcaf_attribution::{AssetClass, AttributionMethod, PcafAttribution};

pub struct SubSovereignHandler;

impl SubSovereignHandler {
    pub fn calculate_attribution(
        outstanding_amount: f64,
        regional_gdp_or_budget: f64,
    ) -> PcafAttribution {
        PcafAttribution::new(
            AssetClass::SubSovereign, 
            outstanding_amount, 
            Some(regional_gdp_or_budget),
            AttributionMethod::RevenueBased,
            "Regional Statistical Office Data".to_string()
        )
    }
}
