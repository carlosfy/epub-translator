// The EPUB module is responsible for zipping and unzipping EPUB files.

use std::fs::{self, File};
use std::io;
use std::path::Path;
use zip::{write::FileOptions, ZipArchive, ZipWriter};

pub fn unzip_epub_from_path(
    epub_path: &str,
    output_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Open Epub file
    let file = File::open(epub_path)?;

    // Create the output directory if it does't exist
    fs::create_dir_all(output_dir)?;

    // Open the ZIP archive
    let mut archive = ZipArchive::new(file)?;

    // Extract all files
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = Path::new(output_dir).join(file.name());

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
            continue;
        } else {
            if let Some(parent) = outpath.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            let mut outfile = File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
        }
    }
    Ok(())
}
