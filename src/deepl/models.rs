use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

pub const DEEPL_FREE_API_URL: &str = "https://api-free.deepl.com/v2";
pub const DEEPL_PRO_API_URL: &str = "https://api.deepl.com/v2";
pub const DEEPL_MOCK_API_URL: &str = "http://127.0.0.1:3030";

pub const DEEPL_TRANSLATE_PATH: &str = "/translate";
pub const DEEPL_USAGE_PATH: &str = "/usage";

#[derive(Debug, Serialize, Deserialize)]
pub struct TranslationRequest {
    pub text: Vec<String>,
    pub target_lang: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TranslationResponse {
    pub translations: Vec<Translation>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Translation {
    pub detected_source_language: String,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UsageResponse {
    pub character_count: u64,
    pub character_limit: u64,
}

#[derive(Debug)]
pub struct DeepLConfiguration {
    pub auth_key: String,
    pub api_url: String,
}

impl DeepLConfiguration {
    pub fn new(auth_key: String, is_pro: bool) -> Self {
        let api_url = if is_pro {
            DEEPL_PRO_API_URL
        } else {
            DEEPL_FREE_API_URL
        };
        Self {
            auth_key,
            api_url: api_url.to_string(),
        }
    }

    pub async fn new_with_determine(auth_key: &str) -> Result<Self, Box<dyn Error>> {
        let is_pro = Self::determine_api_type(auth_key).await?;
        Ok(Self::new(auth_key.to_string(), is_pro))
    }

    pub async fn determine_api_type(auth_key: &str) -> Result<bool, Box<dyn Error>> {
        println!("Determining API type...");
        let client = Client::new();

        let response = client
            .get(format!("{}{}", DEEPL_PRO_API_URL, DEEPL_USAGE_PATH))
            .header("Authorization", format!("DeepL-Auth-Key {}", auth_key))
            .send()
            .await?;

        Ok(response.status().is_success())
    }
}
