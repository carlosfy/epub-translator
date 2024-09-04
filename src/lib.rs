pub mod deepl;
pub mod epub;
pub mod xhtml;

use crate::deepl::models::DeepLConfiguration;

use std::path::{Path, PathBuf};

use epub::{get_xhtml_paths, unzip_epub_from_path};
use tempfile::tempdir;
use xhtml::get_text_nodes_from_path;

use markup5ever_rcdom::NodeData;

pub async fn translate_epub(
    input_file: &Path,
    output_file: &Path,
    target_lang: &str,
    source_lang: Option<String>,
    parallel: u8,
    config: DeepLConfiguration,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Calling translate_epub with arguments:");
    println!("  input_file: {:?}", input_file);
    println!("  output_file: {:?}", output_file);
    println!("  target_lang: {}", target_lang);
    println!("  source_lang: {:?}", source_lang);
    println!("  parallel: {}", parallel);
    println!("  config: {:?}", config);
    println!("Function not implemented yet.");

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
