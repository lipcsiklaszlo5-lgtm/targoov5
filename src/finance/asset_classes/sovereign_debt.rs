use super::super::pcaf_attribution::{AssetClass, AttributionMethod, PcafAttribution};

pub struct SovereignDebtHandler;

impl SovereignDebtHandler {
    pub fn calculate_attribution(
        outstanding_amount: f64,
        ppp_adjusted_gdp: f64,
    ) -> PcafAttribution {
        PcafAttribution::new(
            AssetClass::SovereignDebt, 
            outstanding_amount, 
            Some(ppp_adjusted_gdp),
            AttributionMethod::RevenueBased,
            "IMF / World Bank Economic Data".to_string()
        )
    }
}
