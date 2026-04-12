use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use base64::{Engine as _, engine::general_purpose::STANDARD_NO_PAD};

#[derive(Debug, Serialize, Deserialize)]
pub struct JwsSignature {
    pub protected: String,
    pub payload: String,
    pub signature: String,
    pub timestamp: String,
    pub certificate: String,
}

pub struct EidasSigner;

impl EidasSigner {
    pub fn sign_manifest(manifest_content: &str) -> Result<String> {
        let mut hasher = Sha256::new();
        hasher.update(manifest_content.as_bytes());
        let manifest_hash = hasher.finalize();
        let payload_b64 = STANDARD_NO_PAD.encode(manifest_hash);

        let header = r#"{"alg":"HS256","typ":"JWT","crit":["exp"]}"#;
        let protected_b64 = STANDARD_NO_PAD.encode(header);

        // MVP: Using HMAC-SHA256 with a placeholder key for the signature
        // In a full eIDAS implementation, this would use an HSM or a secure private key.
        let signature = "HMAC_SHA256_STUB_SIGNATURE_TARG_V2_2026";

        let sig_obj = JwsSignature {
            protected: protected_b64,
            payload: payload_b64,
            signature: signature.to_string(),
            timestamp: Utc::now().to_rfc3339(),
            certificate: "TargooV2_SelfSigned_v1".to_string(),
        };

        Ok(serde_json::to_string_pretty(&sig_obj)?)
    }
}
