pub mod deepl;
pub mod epub;
pub mod xhtml;

use crate::deepl::models::DeepLConfiguration;
use crate::deepl::translate;

use std::path::{Path, PathBuf};

use epub::{get_xhtml_paths, unzip_epub_from_path, zip_folder_to_epub};
use xhtml::{
    get_document_node_from_path, get_text_nodes, get_text_nodes_from_path, serialize_document,
};

use html5ever::tendril::StrTendril;
use markup5ever_rcdom::NodeData;
use tempfile::tempdir;

pub async fn translate_epub(
    input_file: &Path,
    output_file: &Path,
    target_lang: &str,
    source_lang: Option<String>,
    parallel: u8,
    config: DeepLConfiguration,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a temporary directory
    let temp_dir = tempdir()?;
    let temp_dir_path = temp_dir.path();

    // Unzip it into a temporary directory
    unzip_epub_from_path(input_file, temp_dir_path)?;

    // Create iterator over all xhtml files
    let xhtml_files = get_xhtml_paths(temp_dir_path)?;

    for file in xhtml_files {
        let file_path = PathBuf::from(file);
        // Get text nodes from all xhtml files
        let document = get_document_node_from_path(&file_path)?;
        let nodes = get_text_nodes(&document)?;

        // Translate text nodes
        for node in nodes {
            if let NodeData::Text { contents } = &node.data {
                let mut text = contents.borrow_mut();
                if !text.trim().is_empty() {
                    let translated = translate(&config, &text, target_lang).await?;
                    *text = StrTendril::from(translated);
                }
            }
        }

        // Serialize the xhtml files
        serialize_document(&document, &file_path)?;
    }

    // Zip the temporary directory into the output file
    zip_folder_to_epub(temp_dir_path, output_file)?;

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

// Translates an EPUB file to a folder, for testing purposes
pub async fn translate_epub_to_folder(
    input_file: &Path,
    output_dir: &Path,
    target_lang: &str,
    source_lang: Option<String>,
    parallel: u8,
    config: DeepLConfiguration,
) -> Result<(), Box<dyn std::error::Error>> {
    // Unzip it into a temporary directory
    unzip_epub_from_path(input_file, output_dir)?;

    // Create iterator over all xhtml files
    let xhtml_files = get_xhtml_paths(output_dir)?;

    for file in xhtml_files {
        let file_path = PathBuf::from(file);
        // Get text nodes from all xhtml files
        let document = get_document_node_from_path(&file_path)?;
        let nodes = get_text_nodes(&document)?;

        // Translate text nodes
        for node in nodes {
            if let NodeData::Text { contents } = &node.data {
                let mut text = contents.borrow_mut();
                if !text.trim().is_empty() {
                    let translated = translate(&config, &text, target_lang).await?;
                    *text = StrTendril::from(translated);
                }
            }
        }

        // Serialize the xhtml files
        serialize_document(&document, &file_path)?;
    }

    Ok(())
}

// Integration test for the whole process.

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
        let parallel = 1;
        let config = get_test_config();

        let shutdown_signal = start_deepl_server().await?;

        translate_epub(
            &input_file,
            &output_file,
            target_lang,
            source_lang,
            parallel,
            config,
        )
        .await?;

        epubcheck(&output_file)?;

        // Wait for the mock server to finish to other tests
        tokio::time::sleep(Duration::from_millis(1000)).await;

        shutdown_signal.send(()).unwrap();

        Ok(())
    }
}
