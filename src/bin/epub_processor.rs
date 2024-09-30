// This script processes an epub as if it was translated to the same language.
// The translation function is the Identity.
// Epub -unzip-> folder -unserialize-> Vec<Rc<Node>> -serialize-> folder -zip-> Epub
//
// Arguments
//  1. Input-epub-path: to the existing epub
//  2. Output-epub-path: to the epub that will be created

use epub_translator::epub::{get_xhtml_paths, unzip_epub_from_path, zip_folder_to_epub};
use epub_translator::xhtml::{get_document_node_from_path, serialize_document};
use std::env;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tempfile::tempdir;

use markup5ever_rcdom::Node;
use std::rc::Rc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get arguments
    let args: Vec<String> = env::args().collect();

    // Check arguments, must be 2 arguments: input_epub_path, output_epub_path
    if args.len() != 3 {
        eprintln!("Usage: {} <input_epub_path> <output_epub_path>", args[0]);
        std::process::exit(1);
    }

    let input_epub_path = Path::new(&args[1]);
    let output_epub_path = Path::new(&args[2]);

    // Create temporary folder to unzip
    let temp_dir = tempdir()?;
    let temp_dir_path = temp_dir.path();

    let start = Instant::now();

    // Unzip to temporary folder
    unzip_epub_from_path(input_epub_path, &temp_dir_path)?;

    let end_unzip = Instant::now();
    let unzip_duration = end_unzip - start;

    println!("Unzip duration: {:?}", unzip_duration);

    // Unserialize documents
    let documents = get_xhtml_paths(temp_dir_path)?
        .map(|file| {
            let file_path = PathBuf::from(file);
            let document = get_document_node_from_path(&file_path).unwrap(); // Care about this
            (document, file_path)
        })
        .collect::<Vec<(Rc<Node>, PathBuf)>>();

    // Serialize documents
    for (document, path) in &documents {
        serialize_document(&document, &path)?;
    }

    let end_serialization = Instant::now();
    let serialization_duration = end_serialization - end_unzip;

    println!(
        "Serialization duration duration: {:?}",
        serialization_duration
    );

    // Zip folder into epub
    zip_folder_to_epub(&temp_dir_path, output_epub_path)?;

    let end_zip = Instant::now();
    let zip_duration = end_zip - end_serialization;

    println!("Zip duration duration: {:?}", zip_duration);

    Ok(())
}
