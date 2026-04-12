use super::super::pcaf_attribution::{AssetClass, AttributionMethod, PcafAttribution};

pub struct CommercialRealEstateHandler;

impl CommercialRealEstateHandler {
    pub fn calculate_attribution(
        outstanding_amount: f64,
        property_value: f64, // At origination
    ) -> PcafAttribution {
        PcafAttribution::new(
            AssetClass::CommercialRealEstate, 
            outstanding_amount, 
            Some(property_value),
            AttributionMethod::ProxyEvic,
            "Property Valuation Report".to_string()
        )
    }
}
