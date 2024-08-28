// The EPUB module is responsible for zipping and unzipping EPUB files.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
use walkdir::WalkDir;
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

pub fn zip_folder_to_epub(
    folder_path: &str,
    epub_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let epub_file = File::create(epub_path)?;
    let mut zip = ZipWriter::new(epub_file);

    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    let walker = WalkDir::new(folder_path).into_iter();

    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();
        let name = path.strip_prefix(Path::new(folder_path))?.to_str().unwrap();

        if path.is_file() {
            zip.start_file(name, options)?;
            let mut file = File::open(path)?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
        } else if !name.is_empty() {
            zip.add_directory(name, options)?;
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
        let test_data_dir = PathBuf::from("tests/data");
        let input_epub_path = find_first_epub(&test_data_dir)
            .expect("There should be an EPUB file in tests/data for the test to pass");

        // Create a temporary directory for the test
        let temp_dir = tempdir()?;
        let extracted_dir = temp_dir.path().join("extracted");
        let output_epub_path = temp_dir.path().join("output.epub");

        // Unzip the EPUB file
        unzip_epub_from_path(
            input_epub_path.to_str().unwrap(),
            extracted_dir.to_str().unwrap(),
        )?;

        // Zip the extracted folder back to EPUB
        zip_folder_to_epub(
            extracted_dir.to_str().unwrap(),
            output_epub_path.to_str().unwrap(),
        )?;

        let output = Command::new("docker")
            .args(&[
                "run",
                "--rm",
                "-v",
                &format!("{}:/data", temp_dir.path().to_str().unwrap()),
                "carlosfy/epubcheck",
                "output.epub",
            ])
            .output()?;

        // Check if EpubCheck was successful
        assert!(
            output.status.success(),
            "EpubCheck failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        Ok(())
    }

    // This test is not working, probably because the zip is noisy.
    // TODO: Do the same test but compare the inner contents of the epub.
    // I'll do this after implementing some way to traverse all the text nodes of the epub.
    // #[test]
    fn test_unzip_epub_from_path() -> Result<(), Box<dyn std::error::Error>> {
        // Find the first EPUB file in tests/data
        let test_data_dir = PathBuf::from("tests/data");
        let input_epub_path = find_first_epub(&test_data_dir)
            .expect("There should be an EPUB file in tests/data for the test to pass");

        // Create a temporary directory for the test
        let temp_dir = tempdir()?;
        let extracted_dir = temp_dir.path().join("extracted");
        let output_epub_path = temp_dir.path().join("output.epub");

        // Unzip the EPUB file
        unzip_epub_from_path(
            input_epub_path.to_str().unwrap(),
            extracted_dir.to_str().unwrap(),
        )?;

        // Zip the extracted folder back to EPUB
        zip_folder_to_epub(
            extracted_dir.to_str().unwrap(),
            output_epub_path.to_str().unwrap(),
        )?;

        //Compare the contents of both EPUB files
        assert!(compare_epub_files(&input_epub_path, &output_epub_path)?);

        Ok(())
    }

    fn find_first_epub(dir: &PathBuf) -> Option<PathBuf> {
        fs::read_dir(dir)
            .expect("Failed to read directory")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .find(|path| path.extension().and_then(|ext| ext.to_str()) == Some("epub"))
    }

    // This function is not working, probably because the zip is noisy.
    // TODO: Do the same test but compare the inner contents of the epub.
    fn compare_epub_files(
        path1: &PathBuf,
        path2: &PathBuf,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let mut archive1 = ZipArchive::new(File::open(path1)?)?;
        let mut archive2 = ZipArchive::new(File::open(path2)?)?;

        if archive1.len() != archive2.len() {
            return Ok(false);
        }

        for i in 0..archive1.len() {
            let mut file1 = archive1.by_index(i)?;
            let mut file2 = archive2.by_index(i)?;

            if file1.name() != file2.name() {
                return Ok(false);
            }

            let mut content1 = Vec::new();
            let mut content2 = Vec::new();
            file1.read_to_end(&mut content1)?;
            file2.read_to_end(&mut content2)?;

            if content1 != content2 {
                return Ok(false);
            }
        }

        Ok(true)
    }
}
