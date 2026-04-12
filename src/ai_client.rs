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

#[derive(Clone)]
pub struct AiBridgeClient {
    client: Client,
    base_url: String,
}

impl AiBridgeClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .pool_max_idle_per_host(16)
                .timeout(Duration::from_secs(60))
                .build()
                .expect("Failed to build AI client"),
            base_url: "http://localhost:9000".to_string(),
        }
    }

    pub async fn classify(&self, header: &str) -> Result<ClassifyResponse> {
        let req = ClassifyRequest {
            query: header.to_string(),
        };

        let resp = self.client
            .post(&format!("{}/classify", self.base_url))
            .json(&req)
            .send()
            .await?
            .json::<ClassifyResponse>()
            .await?;

        Ok(resp)
    }

    pub async fn classify_batch(&self, headers: &[String]) -> Result<Vec<ClassifyResponse>> {
        let resp = self.client
            .post(&format!("{}/classify_batch", self.base_url))
            .json(headers)
            .send()
            .await?
            .json::<Vec<ClassifyResponse>>()
            .await?;

        Ok(resp)
    }
}
