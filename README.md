# epub-translator

A Rust project for translating EPUB files using the DeepL API.

## Features

- Translate EPUB files directly to another language.
- Supports large file sizes without limitations.
- Highly concurrent translation, supporting up to 700 translation channels (DeepL API limitations).
- Allows the use of multiple API keys for high-volume translations.

---

## Usage

### Build the Project

First, build the project with `cargo`:

```bash
cargo build --release
```

The executable will be located in the target/release directory. For a development build, you can use:

```bash
cargo build
```

The executable will then be in the target/debug directory.

### Command Line Usage

#### Basic usage

Translate EPUB file with default values to target language:

```bash
epub-translator [OPTIONS] --target-lang <TARGET_LANG> <INPUT_FILE> <OUTPUT_FILE>
```

Run `epub-translator --help` to get a detailed description of all available options.

#### Test Mode (Mock DeepL API)

Run the translation process using a mock server. This allows testing without using the DeepL API. The mock server will start automatically and terminate when the program ends.

```bash
epub-translator [OPTIONS] --test --target-lang <TARGET_LANG> <INPUT_FILE> <OUTPUT_FILE>
```

Example:

```bash
epub-translator --test --target-lang es book.epub translated_book.epub
```

#### Higher concurrency

Increase the number of concurrent translation channels using the -p option. Note that the DeepL API becomes unstable with more than 700 channels on the free-tier API. The default is set to 400.

Example, translate using 1000 concurrent channels:

```bash
epub-translator -p 1000 --target-lang es book.epub translated_book.epub
```

---

## Logs

Debug logs are stored in a hidden file .epub-translator-logs located in the current working directory. If you encounter issues or need detailed information about the execution, you can review this file.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgements

This project uses the DeepL API for translations. Please ensure you comply with DeepL's terms of service when using this tool.

## Resources

- EPUBCheck: https://github.com/w3c/epubcheck
- EPUB 3.3 Specs: https://www.w3.org/TR/epub-33/
- Docker image for EPUBCheck: https://hub.docker.com/repository/docker/carlosfy/epubcheck/general

## Contribution

Contributions are welcome! Feel free to open issues or submit pull requests.
