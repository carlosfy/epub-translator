use epub_translator::deepl::models::DeepLConfiguration;
use epub_translator::deepl::{get_languages, get_test_config, get_usage, start_deepl_server};
use epub_translator::{count_epub_char, translate_epub};
use rand::seq::SliceRandom;
use rand::thread_rng;

#[macro_use]
extern crate epub_translator;

use clap::Parser;
use futures::future::join_all;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
#[derive(Parser, Debug)]
#[command(author = "Carlos Yago, @carlosfy", version = "0.1.0", about = "Translate EPUB files", long_about = None)]
struct Args {
    /// Path to the EPUB file
    input_file: PathBuf,

    /// Path to the output translation EPUB file
    output_file: PathBuf,

    /// Target language code
    #[arg(short, long)]
    target_lang: String,

    /// Source language code (optional, auto-detect if not provided)
    #[arg(short, long)]
    source_lang: Option<String>,

    /// Number of parallel translation requests (default: 1)
    #[arg(short, long, default_value_t = 1)]
    parallel: usize,

    /// DeepL API key (optional, defaults to DEEPL_API_KEY environment variable)
    #[arg(short = 'k', long)]
    api_key: Option<String>,

    // Verbose
    #[arg(short = 'v', long, default_value_t = false)]
    verbose: bool,

    /// Use test configuration, call to mock server
    #[arg(long)]
    test: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Verify input file exists
    if !args.input_file.exists() {
        eprintln!("Error: Input file does not exist");
        std::process::exit(1);
    }

    // Verify input file is an EPUB
    if args.input_file.extension().unwrap_or_default() != "epub" {
        eprintln!("Error: Input file is not an EPUB");
        std::process::exit(1);
    }

    let mut balanced_configurations = Vec::new();
    let mut total_capacity = 0;
    let mut primary_configuration = get_test_config();

    if args.test {
        let test_config = get_test_config();
        balanced_configurations.push(Arc::new(test_config))
    } else if let Some(api_key) = args.api_key.or_else(|| std::env::var("DEEPL_API_KEY").ok()) {
        let mut api_keys = Vec::new();

        api_keys.push(api_key);

        // Get extra keys.
        let mut indice = 1;
        loop {
            if let Some(key) = std::env::var(format!("DEEPL_API_KEY_{}", indice)).ok() {
                api_keys.push(key);
                indice += 1;
            } else {
                break;
            }
        }
        let configuration_handlers: Vec<_> = api_keys
            .iter()
            .map(|key| {
                let key = key.clone();
                tokio::spawn(async move {
                    let configuration = DeepLConfiguration::new_with_determine(key).await.unwrap();
                    let usage = get_usage(&configuration, args.verbose).await.unwrap();
                    let capacity = usage.character_limit - usage.character_count;
                    (configuration, capacity)
                })
            })
            .collect();

        let configurations_with_capacity: Vec<(DeepLConfiguration, u64)> =
            join_all(configuration_handlers)
                .await
                .into_iter()
                .filter_map(|result| result.ok())
                .filter(|(_, capacity)| *capacity > 20000) // TODO Improve safety
                .collect();

        total_capacity = configurations_with_capacity
            .iter()
            .fold(0, |acc, (_, capacity)| acc + capacity);

        (primary_configuration, _) = configurations_with_capacity[0].clone(); // Todo add check

        for (configuration, capacity) in configurations_with_capacity.iter() {
            if *capacity > 0 {
                let proportion =
                    ((*capacity as f64 / total_capacity as f64) * 100.0).round() as usize;
                for _ in 0..proportion {
                    balanced_configurations.push(Arc::new(configuration.clone()))
                }
            }
        }

        // Shuffle the balanced_configuration_vector
        balanced_configurations.shuffle(&mut thread_rng())
    } else {
        eprintln!(
            "Error: DeepL API key not provided and DEEPL_API_KEY environment variable not set"
        );
        std::process::exit(1);
    }

    println!("");

    // If test then start mock server
    let shutdown_mock_server_signal = if args.test {
        println!("Starting mock server for test mode...");
        match start_deepl_server().await {
            Ok(signal) => {
                println!("Mock server started successfully");
                Some(signal)
            }
            Err(e) => {
                eprintln!("Error starting mock server: {}", e);
                return Err(e);
            }
        }
    } else {
        None
    };

    // Double check if mock server is running
    if args.test {
        let config = get_test_config();
        match get_usage(&config, args.verbose).await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error: The mock server is not running or not responding correctly.");
                eprintln!("Please ensure the mock server is started before running in test mode.");
                eprintln!("Error details: {}", e);
                std::process::exit(1);
            }
        }
    }

    // Test languages code
    let languages = get_languages(&primary_configuration, args.verbose).await?;
    if !languages.0.iter().any(|l| l.language == args.target_lang) {
        eprintln!("Error: Target language code not supported");
        eprintln!(
            "Supported languages: {}",
            languages
                .0
                .iter()
                .map(|l| l.language.clone())
                .collect::<Vec<String>>()
                .join(", ")
        );
        std::process::exit(1);
    }

    println!("       -----------        ");

    // Count the number of characters to translate
    let char_count = count_epub_char(&args.input_file)?;

    let usage = get_usage(&primary_configuration, args.verbose).await?;

    // Show user the usage and the char count
    println!(
        "DeepL Usage: Your limit is: {}, you have already use: {}",
        &usage.character_limit, &usage.character_count
    );
    println!(" Your character translation capacity is {}", total_capacity);
    println!(" Number of characters to translate: {}", char_count);

    // Ask for user confirmation
    println!("Do you want to proceed with the translation? (y/n)");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    if input.trim().to_lowercase() != "y" {
        println!("Translation cancelled by user.");
        std::process::exit(0);
    }

    let start = Instant::now();
    match translate_epub(
        &args.input_file,
        &args.output_file,
        args.target_lang.to_string(),
        args.source_lang,
        args.parallel,
        balanced_configurations,
        args.verbose,
    )
    .await
    {
        Ok(_) => println!("Translation completed successfully!"),
        Err(e) => {
            eprintln!("Error during translation: {}", e);
            std::process::exit(1);
        }
    }

    let total_duration = start.elapsed();
    profiling_log!(args.verbose, "Total duration: {:?}", total_duration);
    eprintln!("End");

    // Shutdown mock server if test mode
    if let Some(signal) = shutdown_mock_server_signal {
        println!("Shutting down mock server...");
        if let Err(e) = signal.send(()) {
            eprintln!("Error shutting down mock server: {:?}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
