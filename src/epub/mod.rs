use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::{write::FileOptions, ZipArchive, ZipWriter};

// Get an operator over all the xhtml files in the epub folder
pub fn get_xhtml_paths(
    epub_folder_path: &Path,
) -> Result<impl Iterator<Item = String>, Box<dyn std::error::Error>> {
    if !epub_folder_path.exists() || !epub_folder_path.is_dir() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "The path is not a directory or does not exist",
        )));
    }

    let walker = WalkDir::new(epub_folder_path).into_iter();
    let xhtml_files = walker
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map(|ext| ext == "xhtml" || ext == "html")
                .unwrap_or(false)
        })
        .filter_map(|entry| entry.path().to_str().map(|s| s.to_string()));

    Ok(xhtml_files)
}

pub fn unzip_epub_from_path(
    epub_path: &Path,
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // Open Epub file
    let file = File::open(epub_path)?;

    // Create the output directory if it does't exist
    fs::create_dir_all(output_dir)?;

    // Open the ZIP archive
    let mut archive = ZipArchive::new(file)?;

    // Create a PathBuf for the output directory
    let output_path_buf = PathBuf::from(output_dir);

    // Extract all files
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = output_path_buf.join(file.name());

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

pub fn zip_folder_to_epub(
    folder_path: &Path,
    epub_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let epub_file = File::create(epub_path)?;
    let mut zip = ZipWriter::new(epub_file);

    // Add mimetype file first, withtout compression
    let stored_options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .unix_permissions(0o644);

    let mimetype_path = folder_path.join("mimetype");
    if mimetype_path.exists() {
        zip.start_file("mimetype", stored_options)?;
        let mut mimetype_file = File::open(mimetype_path)?;
        let mut buffer = Vec::new();
        mimetype_file.read_to_end(&mut buffer)?;
        zip.write_all(&buffer)?;
    }

    // Add the rest of the files with compression
    let deflated_options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    let walker = WalkDir::new(folder_path).into_iter();

    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();
        let name = path.strip_prefix(folder_path)?.to_str().unwrap();

        if name == "mimetype" {
            continue;
        }

        if path.is_file() {
            zip.start_file(name, deflated_options)?;
            let mut file = File::open(path)?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
        } else if !name.is_empty() {
            zip.add_directory(name, deflated_options)?;
        }
    }

    zip.finish()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::process::Command;
    use tempfile::tempdir;

    // This test will find the first epub file in tests/data, unzip it, and then zip it back.
    // Then it will check that epub is still valid by using epubcheck.
    // https://github.com/w3c/epubcheck
    #[test]
    fn test_correctness_after_unzip_and_zip() -> Result<(), Box<dyn std::error::Error>> {
        let test_data_dir = Path::new("tests/data");
        let input_epub_path = find_first_epub(&test_data_dir)
            .expect("There should be an EPUB file in tests/data for the test to pass");

        // Create a temporary directory for the test
        let temp_dir = tempdir()?;
        let temp_dir_path = temp_dir.path();
        let extracted_dir = temp_dir_path.join("extracted");
        let output_epub_path = temp_dir_path.join("output.epub");

        // Unzip the EPUB file
        unzip_epub_from_path(&input_epub_path, &extracted_dir)?;

        // Zip the extracted folder back to EPUB
        zip_folder_to_epub(&extracted_dir, &output_epub_path)?;

        let output = Command::new("docker")
            .args(&[
                "run",
                "--rm",
                "-v",
                &format!("{}:/data", temp_dir_path.to_str().unwrap()),
                "carlosfy/epubcheck",
                "output.epub",
            ])
            .output()?;

        // Check if EpubCheck was successful, needs to be logged in docker.
        assert!(
            output.status.success(),
            "EpubCheck failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        Ok(())
    }

    fn find_first_epub(dir: &Path) -> Option<PathBuf> {
        fs::read_dir(dir)
            .expect("Failed to read directory")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .find(|path| path.extension().and_then(|ext| ext.to_str()) == Some("epub"))
    }

    #[test]
    fn test_get_xhtml_paths() -> Result<(), Box<dyn std::error::Error>> {
        let test_data_dir = Path::new("tests/data/epub_folder");
        let xhtml_files = get_xhtml_paths(test_data_dir)?;
        assert_eq!(xhtml_files.count(), 10);
        Ok(())
    }
}
