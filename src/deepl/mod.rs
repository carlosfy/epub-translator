pub mod models;

use reqwest::Client;
use std::error::Error;
use std::fs;

use std::collections::HashMap;

use models::{
    DeepLConfiguration, LanguagesResponse, Translation, TranslationRequest, TranslationResponse,
    UsageResponse, DEEPL_LANGUAGES_PATH, DEEPL_MOCK_API_URL, DEEPL_TRANSLATE_PATH,
    DEEPL_USAGE_PATH,
};

use tokio::sync::oneshot;
use tokio::time::sleep;
use tokio::time::Duration;

use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};

// TODO: Rewrite taking client as parameter
// translate.sh
pub async fn translate(
    config: &DeepLConfiguration,
    text: &str,
    target_lang: &str,
) -> Result<String, Box<dyn Error>> {
    eprintln!(
        "Requesting translation of text: {} to {}",
        text, target_lang
    );
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
    println!("Getting usage from {}", config.api_url);
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

// languages.sh
pub async fn get_languages(
    config: &DeepLConfiguration,
) -> Result<LanguagesResponse, Box<dyn Error>> {
    eprintln!("Getting languages from {}", config.api_url);
    let client = Client::new();

    let request = client
        .get(format!("{}{}", config.api_url, DEEPL_LANGUAGES_PATH))
        .header(
            "Authorization",
            format!("DeepL-Auth-Key {}", config.auth_key),
        );

    let response: LanguagesResponse = request.send().await?.json().await?;

    Ok(response)
}

pub fn get_test_config() -> DeepLConfiguration {
    DeepLConfiguration {
        api_url: format!("{}/v2", DEEPL_MOCK_API_URL),
        auth_key: "mock_auth_key".to_string(),
    }
}

#[post("/v2/translate")]
async fn r_translate(req: web::Json<TranslationRequest>) -> impl Responder {
    eprintln!("[MOCK SERVER] Received translate request");
    sleep(Duration::from_millis(400)).await;

    let translations = vec![Translation {
        detected_source_language: "EN".to_string(),
        text: format!("--|{}|-- Translated to {}", req.text[0], req.target_lang),
    }];

    HttpResponse::Ok().json(TranslationResponse { translations })
}

#[get("/v2/usage")]
async fn r_usage() -> impl Responder {
    println!("[MOCK SERVER] Received usage request");
    let usage_response = UsageResponse {
        character_count: 1000,
        character_limit: 500000,
    };

    HttpResponse::Ok().json(usage_response)
}

#[get("/v2/languages")]
async fn r_languages(query: web::Query<HashMap<String, String>>) -> impl Responder {
    eprintln!("[MOCK SERVER] Received languages request");
    let languages_json =
        fs::read_to_string("src/deepl/languages.json").expect("Failed to read languages.json");
    let languages_response: LanguagesResponse =
        serde_json::from_str(&languages_json).expect("Failed to parse languages.json");

    let type_param = query.get("type");

    if type_param.is_none() || type_param.unwrap() != "target" {
        HttpResponse::Ok().json(&languages_response)
    } else {
        HttpResponse::Ok().json(&LanguagesResponse(vec![]))
    }
}

pub async fn start_deepl_server() -> Result<oneshot::Sender<()>, Box<dyn Error>> {
    let (tx, rx) = oneshot::channel::<()>();

    let server = HttpServer::new(|| {
        App::new()
            .service(r_translate)
            .service(r_usage)
            .service(r_languages)
    })
    .bind("127.0.0.1:3030")?;

    let server = server.run();
    let server_handle = server.handle();

    // Spawn a task to run the server
    tokio::spawn(async move {
        server.await.expect("Server failed to run");
    });

    tokio::spawn(async move {
        rx.await.ok();
        println!("Shutdown signal received, stopping server...");
        server_handle.stop(true).await;
    });

    println!("Server started and listening on http://127.0.0.1:3030");

    Ok(tx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_translate_usage_and_languages() -> Result<(), Box<dyn Error>> {
        // Wait for the mock server to be ready, run by the tests in lib.rs
        tokio::time::sleep(Duration::from_millis(100)).await;

        let config = get_test_config();

        let translate_result = translate(&config, "Hello", "ES").await?;
        let usage_result = get_usage(&config).await?;
        let languages_result = get_languages(&config).await?;

        // Translate check
        assert_eq!(translate_result, "--|Hello|-- Translated to ES");
        // // Usage check
        assert_eq!(usage_result.character_count, 1000);
        assert_eq!(usage_result.character_limit, 500000);

        // Languages check
        let expected_languages =
            fs::read_to_string("src/deepl/languages.json").expect("Failed to read languages.json");
        let expected_languages_response: LanguagesResponse =
            serde_json::from_str(&expected_languages).expect("Failed to parse languages.json");
        assert_eq!(languages_result, expected_languages_response);

        Ok(())
    }
}
