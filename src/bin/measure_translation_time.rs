use csv::ReaderBuilder;
use epub_translator::deepl::models::DeepLConfiguration;
use epub_translator::deepl::translate;
use std::fs::File;
use std::io::BufReader;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let csv_file_path = "tests/benchmark/data/input.csv";
    let key = std::env::var("DEEPL_API_KEY").expect("DEEPL_API_KEY must be set");
    let config = DeepLConfiguration::new(key, false);

    let file = File::open(csv_file_path)?;
    let reader = BufReader::new(file);
    let mut csv_reader = ReaderBuilder::new().has_headers(true).from_reader(reader);

    for result in csv_reader.records() {
        let record = result?;

        if record.len() != 3 {
            eprintln!("Invalid CSV format. Expected 3 columns: id, text, target_lang");
            continue;
        }

        let id = &record[0];
        let text = &record[1];
        let target_lang = &record[2];

        let char_count = text.chars().count();

        let start = Instant::now();
        let translated = translate(&config, text, target_lang).await?;
        let duration = start.elapsed();
        let dpc = if char_count > 0 {
            duration.as_millis() as f32 / char_count as f32
        } else {
            0.0
        };

        println!(
            "{},{},{},{},{},\"{}\",\"{}\"",
            id,
            char_count,
            duration.as_millis(),
            target_lang,
            dpc,
            text,
            translated,
        );
    }

    Ok(())
}
