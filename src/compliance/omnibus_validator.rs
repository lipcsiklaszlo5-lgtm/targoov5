use serde::{Deserialize, Serialize};
use super::reporting_scope::ReportingScope;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmnibusValidator {
    pub employee_count: Option<u32>,
    pub revenue_eur: Option<f64>,
    pub balance_sheet_eur: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObligationStatus {
    Obligated,      // Must report under CSRD/ESRS
    Voluntary,      // Voluntary reporting
    Unknown,        // Insufficient data
}

impl OmnibusValidator {
    pub fn new(employee_count: Option<u32>, revenue_eur: Option<f64>, balance_sheet_eur: Option<f64>) -> Self {
        Self {
            employee_count,
            revenue_eur,
            balance_sheet_eur,
        }
    }

    pub fn is_csrd_obligated(&self) -> ObligationStatus {
        match (self.employee_count, self.revenue_eur) {
            (Some(employees), Some(revenue)) => {
                if employees > 1000 && revenue > 450_000_000.0 {
                    ObligationStatus::Obligated
                } else {
                    ObligationStatus::Voluntary
                }
            }
            _ => ObligationStatus::Unknown,
        }
    }

    pub fn get_reporting_scope(&self, has_financial_data: bool) -> ReportingScope {
        super::reporting_scope::determine_scope(self.is_csrd_obligated(), has_financial_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_obligation_logic() {
        // 500 fő, 200M€ -> Voluntary
        let v1 = OmnibusValidator::new(Some(500), Some(200_000_000.0), None);
        assert_eq!(v1.is_csrd_obligated(), ObligationStatus::Voluntary);

        // 1500 fő, 500M€ -> Obligated
        let v2 = OmnibusValidator::new(Some(1500), Some(500_000_000.0), None);
        assert_eq!(v2.is_csrd_obligated(), ObligationStatus::Obligated);

        // 2000 fő, 100M€ -> Voluntary (both must meet threshold)
        let v3 = OmnibusValidator::new(Some(2000), Some(100_000_000.0), None);
        assert_eq!(v3.is_csrd_obligated(), ObligationStatus::Voluntary);
        
        // Missing data -> Unknown
        let v4 = OmnibusValidator::new(None, Some(500_000_000.0), None);
        assert_eq!(v4.is_csrd_obligated(), ObligationStatus::Unknown);
    }
}
