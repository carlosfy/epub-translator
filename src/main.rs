use epub_translator::deepl::models::DeepLConfiguration;
use epub_translator::deepl::{get_languages, get_test_config, get_usage, start_deepl_server};
use epub_translator::{count_epub_char, translate_epub};

#[macro_use]
extern crate epub_translator;

use clap::Parser;
use std::path::PathBuf;
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

    // Get API key from args or environment variable
    let api_key = args.api_key.or_else(|| std::env::var("DEEPL_API_KEY").ok());
    if api_key.is_none() && !args.test {
        eprintln!(
            "Error: DeepL API key not provided and DEEPL_API_KEY environment variable not set"
        );
        std::process::exit(1);
    }

    // If test then create test configuration
    let config = if args.test {
        get_test_config()
    } else {
        DeepLConfiguration::new_with_determine(&api_key.unwrap())
            .await
            .expect("Error creating DeepL configuration")
    };

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
    let languages = get_languages(&config, args.verbose).await?;
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

    let usage = get_usage(&config, args.verbose).await?;

    // Show user the usage and the char count
    println!(
        "DeepL Usage: Your limit is: {}, you have already use: {}",
        &usage.character_limit, &usage.character_count
    );
    println!("Number of characters to translate: {}", char_count);

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
        &args.target_lang,
        args.source_lang,
        args.parallel,
        config,
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
