use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize)]
pub struct ClassifyRequest {
    pub query: String,
}

#[derive(Debug, Deserialize)]
pub struct ClassifyResponse {
    pub matched: bool,
    pub ghg_category: Option<String>,
    pub scope3_id: Option<u8>,
    pub scope3_name: Option<String>,
    pub canonical_unit: Option<String>,
    pub ef_value: Option<f64>,
    pub calc_path: Option<String>,
    pub confidence: f32,
    pub matched_keyword: Option<String>,
    pub method: String,
}

pub struct AiBridgeClient {
    client: Client,
    endpoint: String,
}

impl AiBridgeClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to build AI client"),
            endpoint: "http://localhost:9000/classify".to_string(),
        }
    }

    pub async fn classify(&self, header: &str) -> Result<ClassifyResponse> {
        let req = ClassifyRequest {
            query: header.to_string(),
        };

        let resp = self.client
            .post(&self.endpoint)
            .json(&req)
            .send()
            .await?
            .json::<ClassifyResponse>()
            .await?;

        Ok(resp)
    }
}
