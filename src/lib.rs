pub mod deepl;
pub mod epub;
pub mod xhtml;

use crate::deepl::models::DeepLConfiguration;

use std::path::Path;

use epub::unzip_epub_from_path;
use tempfile::tempdir;

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
