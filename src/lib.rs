pub mod deepl;
pub mod epub;
pub mod xhtml;

use crate::deepl::models::DeepLConfiguration;
use crate::deepl::translate;

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use epub::{get_xhtml_paths, unzip_epub_from_path, zip_folder_to_epub};
use xhtml::{
    get_document_node_from_path, get_text_nodes, get_text_nodes_from_path, serialize_document,
};

use html5ever::tendril::StrTendril;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use markup5ever_rcdom::{Node, NodeData};
use tempfile::tempdir;
use tokio::sync::Semaphore;

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

pub async fn translate_epub(
    input_file: &Path,
    output_file: &Path,
    target_lang: &str,
    source_lang: Option<String>,
    concurrent_requests: usize,
    config: Vec<Arc<DeepLConfiguration>>,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a temporary directory
    let temp_dir = tempdir()?;
    let temp_dir_path = temp_dir.path();

    // Translates the content of the EPUB file into a temporary directory
    translate_epub_to_folder(
        input_file,
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

// Unzips an EPUB folder into it and translate its content.
pub async fn translate_epub_to_folder(
    input_file: &Path,
    output_dir: &Path,
    target_lang: &str,
    source_lang: Option<String>,
    concurrent_requests: usize,
    config: Vec<Arc<DeepLConfiguration>>,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Unzips the epub to the output_dir
    timed!(verbose, unzip_epub_from_path, input_file, output_dir)?;

    // Translates the folder in place. Only files that need to be translated will be modified
    translate_folder(
        output_dir,
        target_lang,
        source_lang,
        concurrent_requests,
        config,
        verbose,
    )
    .await?;

    Ok(())
}

// Counts the number of characters to translate in an EPUB file
// This could be done in parallel, but it's not a bottleneck
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

// Translates the text content of all xhtml files of a folder
// This is the core function
pub async fn translate_folder(
    dir_path: &Path,
    target_lang: &str,
    source_lang: Option<String>,
    concurrent_requests: usize,
    config: Vec<Arc<DeepLConfiguration>>,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create semaphore to control the number of concurrent requests
    let semaphore = Arc::new(Semaphore::new(concurrent_requests));
    let retries = 3;

    let start = Instant::now();

    let xhtml_files = get_xhtml_paths(dir_path)?;

    let documents = xhtml_files
        .map(|file| {
            let file_path = PathBuf::from(file);
            let document = get_document_node_from_path(&file_path).unwrap(); // Care about this
            (document, file_path)
        })
        .collect::<Vec<(Rc<Node>, PathBuf)>>();

    // Get all text nodes from all documents, having all together in the same iterator
    // allows to parallelize across all documents, so more threads can be used.
    let nodes = documents
        .iter()
        .flat_map(|(document, _)| get_text_nodes(&document).unwrap())
        .collect::<Vec<Rc<Node>>>();

    let mut tasks = Vec::new();

    let end_preprocessing = Instant::now();
    let preprocessing_duration = end_preprocessing - start;
    profiling_log!(
        verbose,
        "Preprocessing duration: {:?}",
        preprocessing_duration
    );

    // Translate text nodes
    for (i, node) in nodes.iter().enumerate() {
        if let NodeData::Text { contents } = &node.data {
            let text = contents.borrow().to_string();
            if !text.trim().is_empty() {
                let config_index = i % config.len();
                let config_clone = config[config_index].clone();
                let target_lang = target_lang.to_string();
                let semaphore_clone = semaphore.clone();

                let task = tokio::spawn(async move {
                    let _permit = semaphore_clone.acquire().await.unwrap();
                    let permits_available = semaphore_clone.available_permits();
                    eprintln!("Permits available: {}", permits_available);

                    let mut remaining_attemps = retries;
                    while remaining_attemps > 0 {
                        match translate(&config_clone, &text, &target_lang, verbose).await {
                            Ok(translated) => return Some(translated),
                            Err(e) => {
                                if !remaining_attemps > 0 {
                                    eprintln!(
                                        "Error translating text: |{}|, error: {}. No more retries",
                                        &text, e
                                    );
                                    return None;
                                }
                                eprintln!("Error translating text: |{}|, error: {}, remaining attempts: {}", &text, e, remaining_attemps);
                                remaining_attemps -= 1;
                            }
                        }
                    }
                    None
                });

                tasks.push((node, task));
            }
        }
    }

    // Create a progress bar
    let progress_bar =
        ProgressBar::with_draw_target(Some(tasks.len() as u64), ProgressDrawTarget::stdout());
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} ({percent}%)")
            .unwrap()
            .progress_chars("##-"),
    );

    // Wait for all tasks to finish
    for (node, task) in tasks {
        if let Some(translated) = task.await? {
            if let NodeData::Text { contents } = &node.data {
                let mut text = contents.borrow_mut();
                *text = StrTendril::from(translated);
            }
        }
        progress_bar.inc(1);
    }
    progress_bar.finish_with_message("Translation completed");

    let end_translation = Instant::now();
    let translation_duration = end_translation - end_preprocessing;
    profiling_log!(verbose, "Translation duration: {:?}", translation_duration);

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
        let target_lang = "ES";
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
