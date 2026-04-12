use super::super::pcaf_attribution::{AssetClass, AttributionMethod, PcafAttribution};

pub struct MotorVehicleHandler;

impl MotorVehicleHandler {
    pub fn calculate_attribution(
        outstanding_amount: f64,
        total_value_at_origination: f64,
    ) -> PcafAttribution {
        PcafAttribution::new(
            AssetClass::MotorVehicleLoans, 
            outstanding_amount, 
            Some(total_value_at_origination),
            AttributionMethod::ProxyEvic,
            "Vehicle Purchase Invoice".to_string()
        )
    }
}
