# epub-translator

A Rust project for translating EPUB files using the DeepL API.

## Minimum Viable Product (MVP)

This MVP focuses on the core functionality of translating EPUB files and performance. The key features include:

1. **EPUB Parsing**: Extract content from EPUB files.
2. **Text Extraction**: Isolate translatable text from the EPUB structure.
3. **DeepL API Integration**: Connect with DeepL for translation services.
4. **Parallel Translation Processing**: Translate multiple text chunks simultaneously.
5. **EPUB Reconstruction**: Rebuild the EPUB with translated content.
6. **Command-Line Interface**: Provide a simple CLI for easy use.

### MVP Features

- Parse and extract text from EPUB files
- Integrate with DeepL API for translation
- Implement parallel processing for improved performance
- Reconstruct EPUB files with translated content
- Provide a CLI with options for input/output files, source/target languages, and parallel processing control

### Usage (Planned)

```
translate-epub [OPTIONS] <INPUT_FILE> <OUTPUT_FILE> -t <TARGET_LANG>

ARGS:
    <INPUT_FILE>     Path to the input EPUB file
    <OUTPUT_FILE>    Path to the output translated EPUB file
    <TARGET_LANG>    Target language code

OPTIONS:
    -s, --source-lang <LANG>    Source language code (optional, auto-detect if not provided)
    -p, --parallel <NUM>        Number of parallel translation requests (default: 4)
    -k, --api-key <KEY> DeepL API key (optional, defaults to DEEPL_API_KEY environment variable)
    -h, --help --help Display usage information
```

## Development Status

This project is being optimized for performance. It works but it will be much faster.
See performance analysis [here](./tests/benchmark/performance_analysis.ipynb)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgements

This project uses the DeepL API for translations. Please ensure you comply with DeepL's terms of service when using this tool.

## Resources
- EPUBCheck: https://github.com/w3c/epubcheck
- EPUB 3.3 Specs: https://www.w3.org/TR/epub-33/
- Docker image for EPUBCheck: https://hub.docker.com/repository/docker/carlosfy/epubcheck/general
