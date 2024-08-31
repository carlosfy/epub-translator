pub mod models;

use reqwest::Client;
use std::error::Error;

use models::{
    DeepLConfiguration, Translation, TranslationRequest, TranslationResponse, UsageResponse,
    DEEPL_MOCK_API_URL, DEEPL_TRANSLATE_PATH, DEEPL_USAGE_PATH,
};

use std::net::SocketAddr;
use warp::Filter;
use tokio::sync::oneshot;
use tokio::time::Duration;

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
    println!("Getting usage");
    let client = Client::new();

    let request = client
        .get(format!("{}{}", config.api_url, DEEPL_USAGE_PATH))
        .header(
            "Authorization",
            format!("DeepL-Auth-Key {}", config.auth_key),
        );

    let response: UsageResponse = request.send().await?.json().await?;

    Ok(response)
}

// Sets up a mock server on DEEPL_MOCK_API_URL that emulates the DeepL API
// Returns a oneshot::Sender<()> to signal the server to shut down
pub async fn run_mock_server() -> oneshot::Sender<()> {
    let addr: SocketAddr = ([127, 0, 0, 1], 3030).into(); // How to use DEEPL_MOCK_API_URL?

    // Create routes
    let translate = warp::post()
        .and(warp::path("v2"))
        .and(warp::path("translate"))
        .and(warp::body::json::<TranslationRequest>())
        .map(|translation_request: TranslationRequest| {
            println!("Mock Server:Translation request {:?}", translation_request);

            let response = TranslationResponse {
                translations: vec![Translation {
                    detected_source_language: "EN".to_string(),
                    text: format!(
                        "--|{}|-- Translated to {}",
                        translation_request.text[0], translation_request.target_lang
                    ),
                }],
            };

            warp::reply::json(&response)
        });

    let usage = warp::get()
        .and(warp::path("v2"))
        .and(warp::path("usage"))
        .map(|| {
            println!("Mock Server: Usage request");
            let usage_response = UsageResponse {
                character_count: 1000,
                character_limit: 500000,
            };

            warp::reply::json(&usage_response)
        });

    let routes = translate.or(usage);

    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        println!("Mock Server running on {}", DEEPL_MOCK_API_URL);
        let (_, server) = warp::serve(routes).bind_with_graceful_shutdown(addr, async {
            rx.await.ok();
        });

        server.await
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    tx
}

#[cfg(test)]
pub fn get_test_config() -> DeepLConfiguration {
    DeepLConfiguration {
        api_url: format!("{}/v2", DEEPL_MOCK_API_URL),
        auth_key: "mock_auth_key".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_translate_and_usage() -> Result<(), Box<dyn Error>> {
        let _shutdown_signal = run_mock_server().await;
        let config = get_test_config();

        let translate_result = translate(&config, "Hello", "ES").await?;
        let usage_result = get_usage(&config).await?;

        _shutdown_signal
            .send(())
            .expect("Failed to send shutdown signal");

        // Translate check
        assert_eq!(translate_result, "--|Hello|-- Translated to ES");
        // // Usage check
        assert_eq!(usage_result.character_count, 1000);
        assert_eq!(usage_result.character_limit, 500000);

        Ok(())
    }
}
