use serde::{Deserialize, Serialize};
use super::lsr_categories::{LandOwnership, LsrCategory};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiogenicEmission {
    pub description: String,
    pub amount_tco2e: f64,
    pub is_removal: bool, // Negative if removal
    pub ownership: LandOwnership,
}

impl BiogenicEmission {
    pub fn new(description: &str, amount: f64, is_removal: bool, ownership: LandOwnership) -> Self {
        Self {
            description: description.to_string(),
            amount_tco2e: if is_removal { -amount.abs() } else { amount.abs() },
            is_removal,
            ownership,
        }
    }

    pub fn get_lsr_category(&self) -> LsrCategory {
        if self.is_removal {
            LsrCategory::CarbonRemoval
        } else {
            LsrCategory::BiogenicProduct
        }
    }
}
