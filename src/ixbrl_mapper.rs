use crate::models::{LedgerRow, GhgScope};

#[derive(Debug, Clone)]
pub struct XbrlTag {
    pub esrs_code: &'static str,
    pub paragraph: &'static str,
    pub xbrl_concept: &'static str,
    pub unit: &'static str,
    pub period_type: &'static str,
}

const XBRL_MAPPING: &[(GhgScope, Option<u8>, XbrlTag)] = &[
    (GhgScope::SCOPE1, None, XbrlTag {
        esrs_code: "ESRS E1-6",
        paragraph: "§44a",
        xbrl_concept: "esrs:GrossScope1GHGEmissions",
        unit: "tCO2e",
        period_type: "duration",
    }),
    (GhgScope::SCOPE2_LB, None, XbrlTag {
        esrs_code: "ESRS E1-6",
        paragraph: "§44b",
        xbrl_concept: "esrs:GrossScope2GHGEmissionsLocationBased",
        unit: "tCO2e",
        period_type: "duration",
    }),
    (GhgScope::SCOPE3, Some(1), XbrlTag {
        esrs_code: "ESRS E1-6",
        paragraph: "§44c",
        xbrl_concept: "esrs:GrossScope3GHGEmissionsPurchasedGoodsServices",
        unit: "tCO2e",
        period_type: "duration",
    }),
    (GhgScope::SCOPE3, Some(6), XbrlTag {
        esrs_code: "ESRS E1-6",
        paragraph: "§44c",
        xbrl_concept: "esrs:GrossScope3GHGEmissionsBusinessTravel",
        unit: "tCO2e",
        period_type: "duration",
    }),
    (GhgScope::SCOPE3, Some(7), XbrlTag {
        esrs_code: "ESRS E1-6",
        paragraph: "§44c",
        xbrl_concept: "esrs:GrossScope3GHGEmissionsEmployeeCommuting",
        unit: "tCO2e",
        period_type: "duration",
    }),
];

pub fn map_to_xbrl(row: &LedgerRow) -> Option<&'static XbrlTag> {
    let scope3_cat = row.scope3_extension
        .as_ref()
        .map(|s3| s3.category_id);

    XBRL_MAPPING.iter()
        .find(|(scope, cat, _)| {
            *scope == row.ghg_scope &&
            *cat == scope3_cat
        })
        .map(|(_, _, tag)| tag)
}