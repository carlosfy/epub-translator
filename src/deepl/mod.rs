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

macro_rules! conditional_named_log {
    ($enabled:expr, $name:expr, $($arg:tt)*) => {
        if $enabled {
            eprintln!("[{}] {}:{} - {}", stringify!($name), file!(), line!(), format!($($arg)*));
        }
    };
}

macro_rules! api_log {
    ($enabled:expr, $($arg:tt)*) => {
        conditional_named_log!($enabled, "API CALL", $($arg)*);
    };
}

macro_rules! mock_log {
    ($($arg:tt)*) => {
        conditional_named_log!(true, "MOCK SERVER", $($arg)*);
    };
}

// translate.sh
pub async fn translate(
    config: &DeepLConfiguration,
    text: &str,
    target_lang: &str,
    verbose: bool,
    client: &Client,
    id: usize,
) -> Result<String, Box<dyn Error>> {
    api_log!(
        verbose,
        "Request id: {} - Translation of text: |{}| to {}",
        id,
        text,
        target_lang
    );

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
    let translated_text = response.translations[0].text.clone();
    api_log!(
        verbose,
        "Translated to {}: |{}| -> |{}|",
        target_lang,
        text,
        &translated_text
    );

    Ok(translated_text)
}

// usage.sh
pub async fn get_usage(
    config: &DeepLConfiguration,
    verbose: bool,
) -> Result<UsageResponse, Box<dyn Error>> {
    api_log!(verbose, "Getting usage from {}", config.api_url);
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
    verbose: bool,
) -> Result<LanguagesResponse, Box<dyn Error>> {
    api_log!(verbose, "Getting languages from {}", config.api_url);
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
    let text_to_translate = &req.text[0];
    mock_log!("Received translate request: |{}|", text_to_translate);

    sleep(Duration::from_millis(400)).await;

    let translations = vec![Translation {
        detected_source_language: "EN".to_string(),
        text: format!("--|{}|-- Translated to {}", req.text[0], req.target_lang),
    }];

    HttpResponse::Ok().json(TranslationResponse { translations })
}

#[get("/v2/usage")]
async fn r_usage() -> impl Responder {
    mock_log!("[MOCK SERVER] Received usage request");
    let usage_response = UsageResponse {
        character_count: 1000,
        character_limit: 500000,
    };

    HttpResponse::Ok().json(usage_response)
}

#[get("/v2/languages")]
async fn r_languages(query: web::Query<HashMap<String, String>>) -> impl Responder {
    mock_log!("Received languages request");
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
        mock_log!("Shutdown signal received, stopping server...");
        server_handle.stop(true).await;
    });

    mock_log!("Server started and listening on http://127.0.0.1:3030");

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

        let client = Client::new();

        let translate_result = translate(&config, "Hello", "ES", true, &client, 0).await?;
        let usage_result = get_usage(&config, true).await?;
        let languages_result = get_languages(&config, true).await?;

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
