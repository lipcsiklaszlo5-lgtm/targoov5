use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PcafDataQuality {
    Score1, // Audited/verified emissions data
    Score2, // Unaudited but primary physical activity data
    Score3, // Primary activity data with emission factors
    Score4, // Sector-average activity data × revenue
    Score5, // Industry-average revenue-based estimates
}

impl PcafDataQuality {
    pub fn as_int(&self) -> u8 {
        match self {
            Self::Score1 => 1,
            Self::Score2 => 2,
            Self::Score3 => 3,
            Self::Score4 => 4,
            Self::Score5 => 5,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Score1 => "Verified reported emissions",
            Self::Score2 => "Unverified reported emissions or primary physical activity",
            Self::Score3 => "Reported emissions based on primary activity data",
            Self::Score4 => "Proxy-based emissions (e.g., sector averages)",
            Self::Score5 => "Estimated emissions based on economic activity",
        }
    }

    pub fn from_confidence(confidence: f32, _asset_class: super::pcaf_attribution::AssetClass) -> Self {
        if confidence >= 0.9 {
            Self::Score1
        } else if confidence >= 0.8 {
            Self::Score2
        } else if confidence >= 0.7 {
            Self::Score3
        } else if confidence >= 0.5 {
            Self::Score4
        } else {
            Self::Score5
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_quality_order() {
        assert!(PcafDataQuality::Score1 < PcafDataQuality::Score5);
    }
}
