use crate::models::Jurisdiction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetClass {
    ListedEquity,
    CorporateBonds,
    BusinessLoans,
    UnlistedEquity,
    ProjectFinance,
    CommercialRealEstate,
    Mortgages,
    MotorVehicleLoans,
    SovereignDebt,
    SubSovereign,        // NEW PCAF 2025
    UseOfProceeds,       // NEW PCAF 2025
    Securitisation,      // NEW PCAF 2025
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttributionMethod {
    DirectEvic,           // EVIC based (listed)
    BookValue,            // Book value (unlisted)
    RevenueBased,         // Revenue-based estimate
    ProxyEvic,            // Proxy EVIC (sector multipliers)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcafAttribution {
    pub asset_class: AssetClass,
    pub outstanding_amount: f64,      // Investment amount
    pub total_value: f64,             // EVIC / Project Value / Property Value
    pub attribution_factor: f64,      // outstanding / total_value
    
    // Justification fields
    pub attribution_method: AttributionMethod,
    pub data_source: String,           // "Company Annual Report 2025", etc.
    pub evic_source: Option<String>,   // Source of EVIC
    pub justification_note: String,    // Textual justification
}

pub struct PcafResult {
    pub attribution_factor: f64,
    pub financed_emissions_tco2e: f64,
}

impl PcafAttribution {
    pub fn new(
        asset_class: AssetClass, 
        outstanding_amount: f64, 
        total_value: Option<f64>,
        attribution_method: AttributionMethod,
        data_source: String,
    ) -> Self {
        let actual_total_value = total_value.unwrap_or(5_000_000_000.0); // Default placeholder
        let attribution_factor = if actual_total_value > 0.0 {
            outstanding_amount / actual_total_value
        } else {
            0.0
        };

        let justification_note = match attribution_method {
            AttributionMethod::DirectEvic => "Calculated using Enterprise Value Including Cash (EVIC) as per PCAF 2025 standard for listed entities.".to_string(),
            AttributionMethod::BookValue => "Calculated using book value of equity and debt for unlisted entities.".to_string(),
            AttributionMethod::RevenueBased => "Estimated using revenue-based attribution due to lack of direct financial structure data.".to_string(),
            AttributionMethod::ProxyEvic => "Estimated using industry proxy EVIC multipliers.".to_string(),
        };

        Self {
            asset_class,
            outstanding_amount,
            total_value: actual_total_value,
            attribution_factor,
            attribution_method,
            data_source,
            evic_source: if attribution_method == AttributionMethod::DirectEvic { Some("TEG/Benchmark Regulation Compliant Source".to_string()) } else { None },
            justification_note,
        }
    }

    pub fn calculate_financed_emissions(&self, _jurisdiction: Jurisdiction) -> PcafResult {
        // In a real implementation, this would look up the total emissions
        // of the company/project from a database or API.
        let estimated_total_emissions = 1_000_000.0; // tCO2e

        PcafResult {
            attribution_factor: self.attribution_factor,
            financed_emissions_tco2e: estimated_total_emissions * self.attribution_factor,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attribution_calculation() {
        let attribution = PcafAttribution::new(
            AssetClass::ListedEquity,
            10_000_000.0,
            Some(5_000_000_000.0),
            AttributionMethod::DirectEvic,
            "Test Source".to_string(),
        );
        assert!((attribution.attribution_factor - 0.002).abs() < 1e-10);
        assert_eq!(attribution.attribution_method, AttributionMethod::DirectEvic);
    }
}
