use epub_translator::epub::zip_folder_to_epub;
use std::env;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {} <input_dir> <output_file>", args[0]);
        std::process::exit(1);
    }

    let input_dir = Path::new(&args[1]);
    let epub_path = Path::new(&args[2]);

    zip_folder_to_epub(&input_dir, epub_path)?;

    println!("EPUB file unzipped successfully from folder:");
    println!("{}", &input_dir.to_str().unwrap());
    println!("to file:");
    println!("{}", &epub_path.to_str().unwrap());
    Ok(())
}
