use super::super::pcaf_attribution::{AssetClass, AttributionMethod, PcafAttribution};

pub struct ListedEquityHandler;

impl ListedEquityHandler {
    pub fn calculate_attribution(
        outstanding_amount: f64,
        evic: f64,
        inflation_factor: Option<f64>,
    ) -> PcafAttribution {
        let adjusted_evic = evic * inflation_factor.unwrap_or(1.0);
        PcafAttribution::new(
            AssetClass::ListedEquity, 
            outstanding_amount, 
            Some(adjusted_evic),
            AttributionMethod::DirectEvic,
            "Market Data (EVIC)".to_string()
        )
    }
}
