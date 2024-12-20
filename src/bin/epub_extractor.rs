use epub_translator::epub::unzip_epub_from_path;
use std::env;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {} <epub_file> <output_dir>", args[0]);
        std::process::exit(1);
    }

    let epub_path = Path::new(&args[1]);
    let output_dir = Path::new(&args[2]);

    unzip_epub_from_path(epub_path, &output_dir)?;

    println!("EPUB file unzipped successfully.");
    Ok(())
}
