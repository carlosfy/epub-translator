use epub_translator::deepl;

use deepl::models::{Translation, TranslationRequest, TranslationResponse, UsageResponse};
use warp::Filter;

#[tokio::main]
async fn main() {
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
                        "Hello, world! Translated to {}",
                        translation_request.target_lang
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
    println!("Mock Server running on http://localhost:3030");
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
