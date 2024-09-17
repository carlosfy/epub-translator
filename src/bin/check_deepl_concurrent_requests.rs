use deepl::models::DeepLConfiguration;
use epub_translator::deepl;
use reqwest::Client;
use std::env;

use std::error::Error;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Barrier, Semaphore};

const CONCURRENT_REQUESTS: usize = 1000;
const TEXT_TO_TRANSLATE: &str = "Hi";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let start = Instant::now();
    // Get API key from env variable
    let api_key = env::var("DEEPL_API_KEY").expect("DEEP_API_KEY environment variable not set");
    let config = Arc::new(DeepLConfiguration::new_with_determine(api_key).await?);

    let semaphore = Arc::new(Semaphore::new(CONCURRENT_REQUESTS));
    let barrier = Arc::new(Barrier::new(CONCURRENT_REQUESTS));

    let client = Arc::new(Client::new());

    let mut handles = Vec::new();

    for i in 0..CONCURRENT_REQUESTS {
        let config = Arc::clone(&config);
        let semaphore = Arc::clone(&semaphore);
        let barrier = Arc::clone(&barrier);

        let client = client.clone();

        let task = tokio::spawn(async move {
            println!("Thread {} starting", i);
            let _permit = semaphore.acquire().await.unwrap();
            let permits_available = semaphore.available_permits();
            println!(
                "Permits before barrier: {}, thread: {}",
                permits_available, i
            );
            // Wait for all threads to reach this point
            barrier.wait().await;

            let permits_available = semaphore.available_permits();
            println!("Permits available: {}, thread: {}", permits_available, i);
            deepl::translate(&config, &TEXT_TO_TRANSLATE, "ES", true, &client)
                .await
                .ok()
        });
        handles.push(task);
    }

    for handle in handles {
        if let Some(text) = handle.await? {
            println!("Text: {}", text);
        }
    }

    let duration = start.elapsed();
    println!("Time taken: {:?}", duration);

    Ok(())
}
