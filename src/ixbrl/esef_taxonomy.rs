pub const ESRS_SCOPE1: &str = "esrs:Scope1Emissions";
pub const ESRS_SCOPE2: &str = "esrs:Scope2Emissions";
pub const ESRS_SCOPE3: &str = "esrs:Scope3Emissions";
pub const ESRS_TOTAL: &str = "esrs:TotalEmissions";

pub struct TaxonomyMapper;

impl TaxonomyMapper {
    pub fn get_tag_for_scope(scope: &str) -> &'static str {
        match scope {
            "SCOPE1" => ESRS_SCOPE1,
            "SCOPE2_LB" | "SCOPE2_MB" => ESRS_SCOPE2,
            "SCOPE3" => ESRS_SCOPE3,
            _ => ESRS_TOTAL,
        }
    }
}
