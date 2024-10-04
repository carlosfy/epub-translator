use deepl::models::DeepLConfiguration;
use epub_translator::deepl;
use reqwest::Client;
use std::env;

use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Get API key from env variable
    let api_key = env::var("DEEPL_API_KEY").expect("DEEP_API_KEY environment variable not set");

    let config = DeepLConfiguration::new_with_determine(api_key).await?;

    let text_to_translate = "Hello, world!";

    let client = Client::new();

    let translated_text =
        deepl::translate(&config, &text_to_translate, "ES", true, &client, 1).await?;

    println!(
        "Text: {} got translated to {}",
        text_to_translate, translated_text
    );

    let usage = deepl::get_usage(&config, true).await?;

    println!("Usage: {:?}", usage);

    Ok(())
}
