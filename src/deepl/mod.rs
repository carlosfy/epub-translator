pub mod models;

use reqwest::Client;
use std::error::Error;

use models::{
    DeepLConfiguration, TranslationRequest, TranslationResponse, UsageResponse,
    DEEPL_TRANSLATE_PATH, DEEPL_USAGE_PATH,
};

// TODO: Rewrite taking client as parameter
// translate.sh
pub async fn translate(
    config: &DeepLConfiguration,
    text: &str,
    target_lang: &str,
) -> Result<String, Box<dyn Error>> {
    println!("Translating text: {} to {}", text, target_lang);
    let client = Client::new();

    let body = TranslationRequest {
        text: vec![text.to_string()],
        target_lang: target_lang.to_string(),
    };

    let request = client
        .post(format!("{}{}", config.api_url, DEEPL_TRANSLATE_PATH))
        .json(&body)
        .header(
            "Authorization",
            format!("DeepL-Auth-Key {}", config.auth_key),
        );

    let response: TranslationResponse = request.send().await?.json().await?;

    Ok(response.translations[0].text.clone())
}

// usage.sh
pub async fn get_usage(config: &DeepLConfiguration) -> Result<UsageResponse, Box<dyn Error>> {
    let client = Client::new();

    let response: UsageResponse = client
        .get(format!("{}{}", config.api_url, DEEPL_USAGE_PATH))
        .header(
            "Authorization",
            format!("DeepL-Auth-Key {}", config.auth_key),
        )
        .send()
        .await?
        .json()
        .await?;

    Ok(response)
}
