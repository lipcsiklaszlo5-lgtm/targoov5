use crate::models::{CalcPath, DataQualityTier, Jurisdiction, Scope3Category};
use anyhow::Result;

pub struct EEIOEngine;

impl EEIOEngine {
    pub fn new() -> Self {
        Self
    }

    /// Returns the EEIO emission factor in kgCO2e per USD for a given sector and jurisdiction
    pub fn get_eeio_factor(
        &self,
        jurisdiction: Jurisdiction,
        _sector_code: Option<&str>,
        _category: Scope3Category,
    ) -> f64 {
        // Base factors per jurisdiction (US EPA USEEIO 2.0 + EXIOBASE 3.8)
        let base_factor = match jurisdiction {
            Jurisdiction::US => 0.370,
            Jurisdiction::EU => 0.340,
            Jurisdiction::UK => 0.310,
            Jurisdiction::GLOBAL => 0.370,
        };

        // Apply sector-specific multipliers if sector code is provided
        if let Some(code) = _sector_code {
            self.get_sector_multiplier(code) * base_factor
        } else {
            base_factor
        }
    }

    /// Sector-specific multipliers based on NAICS/EXIOBASE codes
    fn get_sector_multiplier(&self, sector_code: &str) -> f64 {
        match sector_code {
            // High emission intensity sectors
            "31-33" | "MANUF" => 1.5, // Manufacturing
            "22" | "UTIL" => 2.0,     // Utilities
            "23" | "CONST" => 1.3,    // Construction
            "48-49" | "TRANS" => 1.8, // Transportation
            "11" | "AGRI" => 1.4,     // Agriculture
            "21" | "MINING" => 1.6,   // Mining
            
            // Medium emission intensity
            "42" | "WHOLE" => 1.1,    // Wholesale Trade
            "44-45" | "RET" => 1.0,   // Retail Trade
            
            // Low emission intensity
            "51" | "INFO" => 0.7,     // Information
            "52" | "FIN" => 0.5,      // Finance and Insurance
            "54" | "PROF" => 0.6,     // Professional Services
            "61" | "EDU" => 0.5,      // Education
            "62" | "HEALTH" => 0.6,   // Healthcare
            
            _ => 1.0, // Default multiplier
        }
    }

    /// PCAF calculation for Category 15 (Investments)
    /// Returns (financed_emissions_tco2e, attribution_factor, dq_score)
    pub fn calculate_pcaf_emissions(
        &self,
        investment_usd: f64,
        asset_type: &PcafAssetType,
        borrower_data: Option<BorrowerData>,
        jurisdiction: Jurisdiction,
    ) -> PcafResult {
        match asset_type {
            PcafAssetType::ListedEquity | PcafAssetType::CorporateBond => {
                if let Some(data) = borrower_data {
                    let attribution_factor = investment_usd / data.enterprise_value_usd;
                    let financed_emissions = attribution_factor * data.total_tco2e;
                    
                    let dq_score = if data.is_verified {
                        if data.is_reported { 1 } else { 2 }
                    } else {
                        3
                    };

                    PcafResult {
                        financed_emissions_tco2e: financed_emissions,
                        attribution_factor,
                        dq_score,
                        data_coverage_pct: 100.0,
                        methodology: "PCAF Listed Equity & Corporate Bonds".to_string(),
                    }
                } else {
                    // Fallback: Revenue-based EEIO estimation
                    self.estimate_via_revenue(investment_usd, jurisdiction)
                }
            }
            
            PcafAssetType::BusinessLoan => {
                if let Some(data) = borrower_data {
                    let total_capital = data.equity_usd + data.debt_usd;
                    let attribution_factor = if total_capital > 0.0 {
                        data.loan_outstanding_usd / total_capital
                    } else {
                        0.0
                    };
                    let financed_emissions = attribution_factor * data.total_tco2e;
                    
                    let dq_score = if data.is_verified {
                        if data.is_reported { 1 } else { 2 }
                    } else {
                        3
                    };

                    PcafResult {
                        financed_emissions_tco2e: financed_emissions,
                        attribution_factor,
                        dq_score,
                        data_coverage_pct: 100.0,
                        methodology: "PCAF Business Loans".to_string(),
                    }
                } else {
                    self.estimate_via_revenue(investment_usd, jurisdiction)
                }
            }
            
            PcafAssetType::ProjectFinance => {
                if let Some(data) = borrower_data {
                    let attribution_factor = if data.total_project_cost_usd > 0.0 {
                        investment_usd / data.total_project_cost_usd
                    } else {
                        0.0
                    };
                    let financed_emissions = attribution_factor * data.project_tco2e;
                    
                    PcafResult {
                        financed_emissions_tco2e: financed_emissions,
                        attribution_factor,
                        dq_score: 3,
                        data_coverage_pct: 100.0,
                        methodology: "PCAF Project Finance".to_string(),
                    }
                } else {
                    self.estimate_via_revenue(investment_usd, jurisdiction)
                }
            }
            
            PcafAssetType::CommercialRealEstate => {
                // Intensity-based approach (kgCO2e/m²/year)
                let intensity = match jurisdiction {
                    Jurisdiction::US => 25.0,
                    Jurisdiction::UK => 19.5,
                    Jurisdiction::EU => 17.8,
                    Jurisdiction::GLOBAL => 20.0,
                };
                
                // Assume average 10 m² per 100,000 USD invested (simplified)
                let estimated_sqm = investment_usd / 10_000.0;
                let financed_emissions = (estimated_sqm * intensity) / 1000.0; // Convert kg to tonnes
                
                PcafResult {
                    financed_emissions_tco2e: financed_emissions,
                    attribution_factor: 1.0,
                    dq_score: 4,
                    data_coverage_pct: 50.0,
                    methodology: "PCAF Commercial Real Estate (intensity-based)".to_string(),
                }
            }
            
            PcafAssetType::Mortgage => {
                // Similar to commercial real estate but residential intensity
                let intensity = match jurisdiction {
                    Jurisdiction::US => 15.0,
                    Jurisdiction::UK => 12.0,
                    Jurisdiction::EU => 10.0,
                    Jurisdiction::GLOBAL => 12.0,
                };
                
                let estimated_sqm = investment_usd / 5_000.0; // Residential is cheaper per m²
                let financed_emissions = (estimated_sqm * intensity) / 1000.0;
                
                PcafResult {
                    financed_emissions_tco2e: financed_emissions,
                    attribution_factor: 1.0,
                    dq_score: 4,
                    data_coverage_pct: 50.0,
                    methodology: "PCAF Mortgages (intensity-based)".to_string(),
                }
            }
            
            PcafAssetType::MotorVehicleLoan => {
                // Average car emissions intensity
                let financed_emissions = investment_usd * 0.0001; // 0.1 kgCO2e per USD financed
                
                PcafResult {
                    financed_emissions_tco2e: financed_emissions,
                    attribution_factor: 1.0,
                    dq_score: 4,
                    data_coverage_pct: 30.0,
                    methodology: "PCAF Motor Vehicle Loans (estimated)".to_string(),
                }
            }
        }
    }

    fn estimate_via_revenue(&self, investment_usd: f64, _jurisdiction: Jurisdiction) -> PcafResult {
        // Simplified revenue-based estimation
        // Assume average carbon intensity of 100 tCO2e per million USD revenue
        let estimated_revenue = investment_usd * 2.0; // Assume 0.5x revenue multiple
        let estimated_emissions = (estimated_revenue / 1_000_000.0) * 100.0;
        
        PcafResult {
            financed_emissions_tco2e: estimated_emissions,
            attribution_factor: investment_usd / estimated_revenue,
            dq_score: 5,
            data_coverage_pct: 10.0,
            methodology: "PCAF Revenue-based estimation".to_string(),
        }
    }

    /// Calculates Weighted Average Carbon Intensity (WACI) for portfolio
    pub fn calculate_waci(
        &self,
        investments: &[PcafInvestment],
        total_portfolio_value: f64,
    ) -> f64 {
        if total_portfolio_value <= 0.0 {
            return 0.0;
        }

        let weighted_sum: f64 = investments
            .iter()
            .map(|inv| (inv.value_usd / total_portfolio_value) * inv.carbon_intensity_tco2e_per_m_usd)
            .sum();

        weighted_sum
    }

    /// Maps a header string to a probable EEIO sector code
    pub fn infer_sector_code(&self, header: &str) -> Option<String> {
        let header_lower = header.to_lowercase();
        
        if header_lower.contains("manufact") || header_lower.contains("produktion") {
            Some("MANUF".to_string())
        } else if header_lower.contains("util") || header_lower.contains("energy") || header_lower.contains("strom") {
            Some("UTIL".to_string())
        } else if header_lower.contains("construct") || header_lower.contains("bau") {
            Some("CONST".to_string())
        } else if header_lower.contains("transport") || header_lower.contains("logistic") {
            Some("TRANS".to_string())
        } else if header_lower.contains("agric") || header_lower.contains("farm") {
            Some("AGRI".to_string())
        } else if header_lower.contains("mining") || header_lower.contains("bergbau") {
            Some("MINING".to_string())
        } else if header_lower.contains("it ") || header_lower.contains("software") || header_lower.contains("tech") {
            Some("INFO".to_string())
        } else if header_lower.contains("financ") || header_lower.contains("bank") {
            Some("FIN".to_string())
        } else {
            None
        }
    }
}

impl Default for EEIOEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum PcafAssetType {
    ListedEquity,
    CorporateBond,
    BusinessLoan,
    ProjectFinance,
    CommercialRealEstate,
    Mortgage,
    MotorVehicleLoan,
}

#[derive(Debug, Clone)]
pub struct BorrowerData {
    pub enterprise_value_usd: f64,
    pub equity_usd: f64,
    pub debt_usd: f64,
    pub loan_outstanding_usd: f64,
    pub total_project_cost_usd: f64,
    pub total_tco2e: f64,
    pub project_tco2e: f64,
    pub is_reported: bool,
    pub is_verified: bool,
}

#[derive(Debug, Clone)]
pub struct PcafResult {
    pub financed_emissions_tco2e: f64,
    pub attribution_factor: f64,
    pub dq_score: u8,
    pub data_coverage_pct: f64,
    pub methodology: String,
}

#[derive(Debug, Clone)]
pub struct PcafInvestment {
    pub value_usd: f64,
    pub carbon_intensity_tco2e_per_m_usd: f64,
    pub dq_score: u8,
}
