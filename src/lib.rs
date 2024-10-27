pub mod deepl;
pub mod epub;
pub mod xhtml;

use crate::deepl::models::DeepLConfiguration;
use crate::deepl::translate;

use std::borrow::Borrow;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use epub::{get_xhtml_paths, unzip_epub_from_path, zip_folder_to_epub};
use reqwest::Client;
use xhtml::{
    get_document_node_from_path, get_text_nodes, get_text_nodes_from_path, serialize_document,
};

use html5ever::tendril::StrTendril;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use markup5ever_rcdom::{Node, NodeData};
use tempfile::tempdir;
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Semaphore,
};

#[macro_export]
macro_rules! profiling_log {
    ($enabled:expr, $($arg:tt)*) => {
        if $enabled {
            eprintln!("[PROFILING] {}:{} - {}", file!(), line!(), format!($($arg)*));
        }

    };
}

macro_rules! timed {
    ($print:expr, $func:ident, $($arg:expr),*) => {{
        let start = Instant::now();
        let result = $func($($arg),*);
        let duration = start.elapsed();
        profiling_log!($print,
            "{}: function took {:?}",
            stringify!($func),
            duration);
        result
    }

    };
}

/// Translates an EPUB file and put the translation into another EPUB file.
pub async fn translate_epub(
    input_file: &Path,
    output_file: &Path,
    target_lang: String,
    source_lang: Option<String>,
    concurrent_requests: usize,
    config: Vec<Arc<DeepLConfiguration>>,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a temporary directory
    let temp_dir = tempdir()?;
    let temp_dir_path = temp_dir.path();

    // Unzips the epub to the output_dir
    timed!(verbose, unzip_epub_from_path, input_file, temp_dir_path)?;

    // Translates the folder in place. Only files that need to be translated will be modified
    translate_folder(
        temp_dir_path,
        target_lang,
        source_lang,
        concurrent_requests,
        config,
        verbose,
    )
    .await?;

    // Zip the temporary directory into the output file
    timed!(verbose, zip_folder_to_epub, temp_dir_path, output_file)?;

    Ok(())
}

/// Counts the number of characters to translate in an EPUB file.
pub fn count_epub_char(epub_path: &Path) -> Result<usize, Box<dyn std::error::Error>> {
    // Create a temporary directory
    let temp_dir = tempdir()?;
    let temp_dir_path = temp_dir.path();

    // Unzip it into a temporary directory
    unzip_epub_from_path(epub_path, temp_dir_path)?;

    // Create iterator over all xhtml files
    let xhtml_files = get_xhtml_paths(temp_dir_path)?;

    let mut counter = 0;

    for xhtml_file in xhtml_files {
        let nodes = get_text_nodes_from_path(&PathBuf::from(xhtml_file))?;

        for handle in nodes {
            if let NodeData::Text { contents } = &handle.data {
                let text = contents.borrow();
                counter += text.len();
            }
        }
    }

    Ok(counter)
}

/// Messages
struct TranslationRequest {
    id: usize,
    text: Arc<String>,
}

struct TranslationResult {
    id: usize,
    translated_text: Arc<Option<String>>,
}

/// Handles a single translation task asynchronously.
///
/// This function is designed to be thread-agnostic and lightweight, making it easily portable.
/// It performs the following steps:
/// 1. Acquires a permit from the semaphore to limit concurrent requests.
/// 2. Calls the external translation API using the `translate` function.
/// 3. Sends the translation result back to the writer through a channel.
///
/// # Error Handling
/// - If translation fails, it logs the error and sends a result with `None` for the translated text.
/// - If sending the result back to the writer fails, it logs the error.
async fn translation_task(
    id: usize,
    text: Arc<String>,
    target_lang: Arc<String>,
    semaphore: Arc<Semaphore>,
    tx_writer: Sender<TranslationResult>,
    configuration: Arc<DeepLConfiguration>,
    client: Client,
) {
    eprintln!("[{}] [Task] Start of translation id", id);
    let out_permit = semaphore.acquire().await.unwrap();
    let remaining_permits = semaphore.available_permits();
    eprintln!(
        "[{}] [Task] Took permit, remaining permits: {}",
        id, remaining_permits
    );
    let translation_result =
        match translate(&configuration, &text, &target_lang, true, &client, id).await {
            Ok(translated_text) => {
                // Drop permit
                TranslationResult {
                    id: id,
                    translated_text: Arc::new(Some(translated_text)),
                }
            }
            Err(error) => {
                println!("[{}] [Task] Error translating node: {}", id, error);
                TranslationResult {
                    id: id,
                    translated_text: Arc::new(None),
                }
            }
        };
    drop(out_permit);

    if let Err(e) = tx_writer.send(translation_result).await {
        eprintln!("Failed to send translation result to writer: {}", e);
    }
    eprintln!("[{}] [Task] End of translation", id);
}

/// Spawns a Translator actor to manage translation tasks.
///
/// This function:
/// 1. Receives translation requests via the receiver channel.
/// 2. Spawns individual translation tasks for each request.
/// 3. Individual translation tasks will send the result to the sender.
/// 4. Manages concurrent requests using a semaphore.
/// 5. Distributes tasks across multiple DeepL configurations.
///
/// Resources:
/// 1. Client
/// 2. Semaphore
/// 3. Configurations
///
/// The actor continues running until the request channel is closed.
async fn run_translator(
    configurations: Vec<Arc<DeepLConfiguration>>,
    concurrent_requests: usize,
    target_lang: String,
    mut receiver: Receiver<TranslationRequest>,
    sender: Sender<TranslationResult>,
) {
    eprintln!("Created the translator");
    let semaphore = Arc::new(Semaphore::new(concurrent_requests));
    let target_lang = Arc::new(target_lang);
    let client = Client::new();
    let configuration_length = configurations.len();

    while let Some(request) = receiver.recv().await {
        eprintln!("[{}] - [Translator] Received request ", request.id);

        let config_index = request.id % configuration_length;

        let configuration = configurations[config_index].clone();
        let client = client.clone();
        let tx_writer = sender.clone();
        let target_lang = target_lang.clone();
        let semaphore = semaphore.clone();

        let _task = tokio::spawn(translation_task(
            request.id,
            request.text,
            target_lang,
            semaphore,
            tx_writer,
            configuration,
            client,
        ));
    }
    eprintln!("[Translator] End of all task")
}

/// Core function: Translates text in all XHTML files within a folder
///
/// This function:
/// 1. Creates document iterators, one per file, each as an HTML root.
/// 2. Iterates through text nodes in each document.
/// 3. Sets up two channels:
///     - Translator_Channel (TranslationRequest)
///     - Writer_Channel (TranslationResult)
/// 4. Spawns Translator:
///     - Listens on Translator_Channel, spawning TranslationTasks as needed
/// 5. For each text node:
///     - Sends a TranslationRequest to Translator
/// 6. Spawns Writer:
///     - Listens on Writer_Channel for TranslationResults
///         - Modifies nodes if successful; retries if not (up to max attempts)
///     - Closes channels when done (note: deadlock risk if incomplete)
/// 7. Serializes documents back to files.
///
/// Note: Ideally, TranslationRequests would be sent post-Writer spawn, but this requires moving
/// Writer (owner of nodes, Vec<Rc<Node>>) across threads.
pub async fn translate_folder(
    dir_path: &Path,
    target_lang: String,
    _source_lang: Option<String>,
    concurrent_requests: usize,
    configurations: Vec<Arc<DeepLConfiguration>>,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let max_retries = 4;
    let start = Instant::now();

    let xhtml_files = get_xhtml_paths(dir_path)?;

    // 1. Create document iterator
    let documents = xhtml_files
        .map(|file| {
            let file_path = PathBuf::from(file);
            let document = get_document_node_from_path(&file_path).unwrap(); // Care about this
            (document, file_path)
        })
        .collect::<Vec<(Rc<Node>, PathBuf)>>();

    // 2. Create text node iterator
    // This approach enables parallelization across all documents,
    let nodes = documents
        .iter()
        .flat_map(|(document, _)| get_text_nodes(&document).expect("Failed to get text nodes."))
        .collect::<Vec<Rc<Node>>>();

    let total_nodes = nodes.len();

    let end_preprocessing = Instant::now();
    let preprocessing_duration = end_preprocessing - start;
    profiling_log!(
        verbose,
        "Preprocessing duration: {:?}",
        preprocessing_duration
    );

    // Create a progress bar
    let progress_bar =
        ProgressBar::with_draw_target(Some((total_nodes + 1) as u64), ProgressDrawTarget::stdout());
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} ({percent}%)")
            .unwrap()
            .progress_chars("##-"),
    );

    progress_bar.inc(1);

    // 3. Create Channels
    let writer_queue_size = 15_000;

    let (tx_translator, rx_translator) = mpsc::channel::<TranslationRequest>(writer_queue_size);
    let (tx_writer, mut rx_writer) = mpsc::channel::<TranslationResult>(writer_queue_size);

    // 4. Spawn a Translator
    let _translator_handle = tokio::spawn(run_translator(
        configurations,
        concurrent_requests,
        target_lang,
        rx_translator,
        tx_writer,
    ));

    let texts_enumerated: Vec<Arc<String>> = nodes
        .iter()
        .map(|node| {
            if let NodeData::Text { contents } = &node.data {
                let text = contents.borrow().to_string();
                Arc::new(text)
            } else {
                Arc::new(String::new())
            }
        })
        .collect();

    // 5. Send initial translation requests to the Translator
    // Note: Ensure the Translator is created and listening before sending requests
    // to avoid potential failures in message transmission
    let mut completed = 0;
    for (id, text) in texts_enumerated.iter().enumerate() {
        eprintln!("[{}] Sending request to Translator", id);
        if let Err(error) = tx_translator
            .send(TranslationRequest {
                id: id,
                text: text.clone(),
            })
            .await
        {
            eprintln!("[{}] Error sending message to translator: {}", id, error);
            completed += 1;
            progress_bar.inc(1);
        };
    }

    let mut retries: Vec<usize> = vec![0, total_nodes];

    // 6. Spawn Writer
    //
    // Ressources:
    // - Writer receiver `rx_writer`
    // - Node reference `vector nodes` (Vec<Rc<Node>>)
    // - Translator sender `tx_translator`
    // - Progress counter `completed`
    // - retries
    while let Some(TranslationResult {
        id,
        translated_text,
    }) = rx_writer.recv().await
    {
        eprintln!(
            "[{}] [Writer] Received: {}, Received result: {:?}",
            id, completed, translated_text
        );
        if let Some(translated_text) = translated_text.borrow() {
            if let NodeData::Text { contents } = &nodes[id].data {
                let mut text = contents.borrow_mut();
                *text = StrTendril::from_slice(translated_text);
                completed += 1;
                progress_bar.inc(1);
            }
        } else {
            if retries[id] < max_retries {
                retries[id] += 1;
                if let Err(error) = tx_translator
                    .send(TranslationRequest {
                        id: id,
                        text: texts_enumerated[id].clone(),
                    })
                    .await
                {
                    eprintln!("[{}] Error sending message to translator: {}", id, error);
                    completed += 1;
                    progress_bar.inc(1);
                };
            } else {
                completed += 1;
                progress_bar.inc(1);
            }
        }
        // Exit condition: All nodes have been processed
        // Note: This breaks the loop to avoid a deadlock scenario
        // TODO: Implement a more robust termination mechanism
        if completed == total_nodes {
            break;
        }
    }

    eprintln!("The Writer ended");

    progress_bar.finish_with_message("Translation completed");

    let end_translation = Instant::now();
    let translation_duration = end_translation - end_preprocessing;
    profiling_log!(verbose, "Translation duration: {:?}", translation_duration);

    // 7. Serialize all documents
    for (document, path) in &documents {
        serialize_document(&document, &path)?;
    }

    let end_serialization = Instant::now();
    let serialization_duration = end_serialization - end_translation;
    profiling_log!(
        verbose,
        "Serialization duration: {:?}",
        serialization_duration
    );

    Ok(())
}

// Integration test for the whole process.
// It creates a mock server that will be used by all the other tests that need it.
// So this test should be the last one to end.
#[cfg(test)]
mod tests {
    use super::*;
    use deepl::{get_test_config, start_deepl_server};
    use epub::epubcheck;
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_translate_epub() -> Result<(), Box<dyn std::error::Error>> {
        let input_file = PathBuf::from("tests/data/epub_to_test/input.epub");

        let temp_dir = tempfile::tempdir()?;

        let output_file = temp_dir.path().join("output.epub");
        let target_lang = "ES".to_string();
        let source_lang: Option<String> = None;
        let parallel = 1000;
        let config = get_test_config();
        let configurations = vec![Arc::new(config)];

        let shutdown_signal = start_deepl_server().await?;

        let start = Instant::now();
        translate_epub(
            &input_file,
            &output_file,
            target_lang,
            source_lang,
            parallel,
            configurations,
            true,
        )
        .await?;

        let duration = start.elapsed();
        eprintln!("=========> Time taken {:?}", duration);

        // Check the format of the translated epub
        epubcheck(&output_file)?;

        // Wait one extra second to be sure that other tests have finished.
        tokio::time::sleep(Duration::from_millis(1000)).await;

        // Shut down mock server
        shutdown_signal.send(()).unwrap();

        Ok(())
    }
}
