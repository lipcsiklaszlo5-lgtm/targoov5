use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issa5000Metadata {
    pub data_source_type: DataSourceType,
    pub collection_method: String,
    pub verification_status: VerificationStatus,
    pub last_verified_date: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataSourceType {
    Primary,
    Secondary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationStatus {
    Verified,
    Unverified,
    InProcess,
}

impl Issa5000Metadata {
    pub fn new_automated(is_primary: bool) -> Self {
        Self {
            data_source_type: if is_primary { DataSourceType::Primary } else { DataSourceType::Secondary },
            collection_method: "Automated API/Direct Upload".to_string(),
            verification_status: VerificationStatus::Verified, // Assuming internal validation
            last_verified_date: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_issa_metadata_creation() {
        let meta = Issa5000Metadata::new_automated(true);
        assert_eq!(meta.data_source_type, DataSourceType::Primary);
        assert_eq!(meta.verification_status, VerificationStatus::Verified);
    }
}
