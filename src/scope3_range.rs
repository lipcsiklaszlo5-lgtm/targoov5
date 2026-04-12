use crate::models::{CalcPath, Scope3Category};

pub struct Scope3RangeGuard;

impl Scope3RangeGuard {
    pub fn new() -> Self {
        Self
    }

    /// Validates tCO2e against category-specific range limits
    pub fn validate(&self, category: Scope3Category, calc_path: CalcPath, tco2e: f64) -> Result<(), String> {
        if tco2e.is_nan() || tco2e.is_infinite() || tco2e < 0.0 {
            return Err("tCO2e value is invalid (NaN, infinite, or negative)".to_string());
        }

        let (min, max_act, max_spend, abs_max) = self.get_limits(category);

        // Check absolute maximum (global safety cap)
        if tco2e > abs_max {
            return Err(format!(
                "tCO2e value {} exceeds absolute maximum of {} for any row",
                tco2e, abs_max
            ));
        }

        // Check minimum
        if tco2e < min {
            return Err(format!(
                "tCO2e value {} is below minimum threshold of {} for category {}",
                tco2e, min, category.as_str()
            ));
        }

        // Check category and calc-path specific maximum
        let max_allowed = match calc_path {
            CalcPath::ActivityBased | CalcPath::Pcaf => max_act,
            CalcPath::SpendBased => max_spend,
        };

        if tco2e > max_allowed {
            return Err(format!(
                "tCO2e value {} exceeds maximum allowed ({}) for {} calculation path in category {}",
                tco2e, max_allowed,
                match calc_path {
                    CalcPath::ActivityBased => "ActivityBased",
                    CalcPath::SpendBased => "SpendBased",
                    CalcPath::Pcaf => "PCAF",
                },
                category.as_str()
            ));
        }

        Ok(())
    }

    fn get_limits(&self, category: Scope3Category) -> (f64, f64, f64, f64) {
        // Returns: (min, max_activity_based, max_spend_based, abs_max)
        match category {
            Scope3Category::Cat1PurchasedGoodsServices => {
                (0.001, 50_000.0, 500_000.0, 1_000_000.0)
            }
            Scope3Category::Cat2CapitalGoods => {
                (0.01, 100_000.0, 2_000_000.0, 5_000_000.0)
            }
            Scope3Category::Cat3FuelEnergyActivities => {
                (0.0001, 10_000.0, 10_000.0, 50_000.0)
            }
            Scope3Category::Cat4UpstreamTransport => {
                (0.001, 200_000.0, 500_000.0, 1_000_000.0)
            }
            Scope3Category::Cat5WasteGenerated => {
                (0.0001, 5_000.0, 5_000.0, 20_000.0)
            }
            Scope3Category::Cat6BusinessTravel => {
                (0.001, 10_000.0, 50_000.0, 100_000.0)
            }
            Scope3Category::Cat7EmployeeCommuting => {
                (0.001, 20_000.0, 100_000.0, 200_000.0)
            }
            Scope3Category::Cat8UpstreamLeasedAssets => {
                (0.001, 50_000.0, 200_000.0, 500_000.0)
            }
            Scope3Category::Cat9DownstreamTransport => {
                (0.001, 200_000.0, 500_000.0, 1_000_000.0)
            }
            Scope3Category::Cat10ProcessingSoldProducts => {
                (0.001, 100_000.0, 1_000_000.0, 2_000_000.0)
            }
            Scope3Category::Cat11UseOfSoldProducts => {
                (0.001, 500_000.0, 500_000.0, 2_000_000.0)
            }
            Scope3Category::Cat12EndOfLifeTreatment => {
                (0.0001, 50_000.0, 50_000.0, 200_000.0)
            }
            Scope3Category::Cat13DownstreamLeasedAssets => {
                (0.001, 200_000.0, 500_000.0, 1_000_000.0)
            }
            Scope3Category::Cat14Franchises => {
                (0.01, 500_000.0, 2_000_000.0, 5_000_000.0)
            }
            Scope3Category::Cat15Investments => {
                (0.01, 5_000_000.0, 5_000_000.0, 10_000_000.0)
            }
        }
    }

    /// Provides a human-readable description of the limits for a category
    pub fn describe_limits(&self, category: Scope3Category) -> String {
        let (min, max_act, max_spend, abs_max) = self.get_limits(category);
        format!(
            "Category {}: min={:.4} tCO2e, max_activity={:.0} tCO2e, max_spend={:.0} tCO2e, abs_max={:.0} tCO2e",
            category.as_str(), min, max_act, max_spend, abs_max
        )
    }
}

impl Default for Scope3RangeGuard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cat1_limits() {
        let guard = Scope3RangeGuard::new();
        assert!(guard
            .validate(
                Scope3Category::Cat1PurchasedGoodsServices,
                CalcPath::SpendBased,
                100.0
            )
            .is_ok());
        assert!(guard
            .validate(
                Scope3Category::Cat1PurchasedGoodsServices,
                CalcPath::SpendBased,
                2_000_000.0
            )
            .is_err());
    }

    #[test]
    fn test_cat15_pcaf_limits() {
        let guard = Scope3RangeGuard::new();
        assert!(guard
            .validate(
                Scope3Category::Cat15Investments,
                CalcPath::Pcaf,
                1_000_000.0
            )
            .is_ok());
        assert!(guard
            .validate(
                Scope3Category::Cat15Investments,
                CalcPath::Pcaf,
                15_000_000.0
            )
            .is_err());
    }
}
