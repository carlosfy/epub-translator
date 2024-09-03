use kuchiki::iter::Descendants;
use kuchiki::parse_html;
use kuchiki::traits::*;
use kuchiki::NodeRef;

pub fn print_all_text_nodes(html_content: &str) -> Result<(), Box<dyn std::error::Error>> {
    let document = parse_html().one(html_content);

    let iterator = if let Some(root) = document.select_first("html").ok() {
        root.as_node().descendants()
    } else {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No HTML element found",
        )));
    };

    println!("FROM FILE_PATH: Printing from inside-------------------------");

    for node in iterator {
        if let Some(text) = node.as_text() {
            let text = text.borrow();
            println!("Text:{}", &text);
        }
    }

    println!("FROM FILE_PATH: End printing from inside---------------------");

    Ok(())
}

pub fn get_text_nodes_iterator(
    html_content: &str,
) -> Result<Descendants, Box<dyn std::error::Error>> {
    let document = parse_html().one(html_content);

    let iterator = if let Some(root) = document.select_first("html").ok() {
        root.as_node().descendants()
    } else {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No HTML element found",
        )));
    };

    Ok(iterator)
}

pub fn print_text_nodes_from_document(
    document: &NodeRef,
) -> Result<(), Box<dyn std::error::Error>> {
    let iterator = get_text_nodes_iterator_from_document(document)?;
    println!("FROM DOCUMENT: Printing from inside-------------------------");

    for node in iterator {
        if let Some(text) = node.as_text() {
            let text = text.borrow();
            println!("Text: {}", &text);
        }
    }
    println!("FROM DOCUMENT: End printing from inside---------------------");
    Ok(())
}

pub fn get_text_nodes_iterator_from_document(
    document: &NodeRef,
) -> Result<Descendants, Box<dyn std::error::Error>> {
    let iterator = document
        .select_first("html")
        .ok()
        .unwrap()
        .as_node()
        .descendants();
    Ok(iterator)
}

// This script demostrate a bug when trying to traverse all nodes with kuchiki
// If there is a head element in the xml, and the iterator is created from a
// function where the document is create, the iterator only includes the first
// node of the head. And not the rest.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let basic_xhtml_with_head = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <html xmlns="http://www.w3.org/1999/xhtml">
            <head>
                <title>Sample Title</title>
                <style type="text/css">
                    body { font-family: Arial, sans-serif; }
                    h1 { color: #333; }
                </style>
            </head>
            <body>
                <h1>Hello, World!</h1>
                <p>This is a sample paragraph.</p>
                <div>
                    <p>P1</p>
                    <p>P2</p>
                </div>
                <p> Last P</p>
            </body>
        </html>
    "#
    .trim();

    // If I create the iterator from the document (NodeRef) and I print from
    // inside the function, it works.
    // This prints all the text nodes from the inside of the function
    print_all_text_nodes(basic_xhtml_with_head)?;

    // If I create the iterator from the file path and I print from outside
    // only the first text node is printed.
    println!("FROM FILE_PATH: Printing from outside------------------------");
    let iterator = get_text_nodes_iterator(basic_xhtml_with_head)?;
    for node in iterator {
        if let Some(text) = node.as_text() {
            let text = text.borrow();
            println!("Text: {}", &text);
        }
    }
    println!("FROM FILE_PATH: End printing from outside--------------------");

    // If I create the document from here, it works both ways.
    let document = parse_html().one(basic_xhtml_with_head);

    print_text_nodes_from_document(&document)?;

    println!("FROM DOCUMENT: Printing from outside---------------------");
    let iterator = get_text_nodes_iterator_from_document(&document)?;
    for node in iterator {
        if let Some(text) = node.as_text() {
            let text = text.borrow();
            println!("Text: {}", &text);
        }
    }
    println!("FROM DOCUMENT: End printing from outside---------------------");

    // Now without head
    let basic_xhtml_without_head = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <html xmlns="http://www.w3.org/1999/xhtml">
            <body>
                <h1>Hello, World!</h1>
                <p>This is a sample paragraph.</p>
                <div>
                    <p>P1</p>
                    <p>P2</p>
                </div>
                <p> Last P</p>
            </body>
        </html>
    "#
    .trim();

    println!("======================= WITHOUT HEAD =======================");

    print_all_text_nodes(basic_xhtml_without_head)?;

    // If I create the iterator from the file path and I print from outside
    // only the first text node is printed.
    println!("FROM FILE_PATH: Printing from outside------------------------");
    let iterator = get_text_nodes_iterator(basic_xhtml_without_head)?;
    for node in iterator {
        if let Some(text) = node.as_text() {
            let text = text.borrow();
            println!("Text: {}", &text);
        }
    }
    println!("FROM FILE_PATH: End printing from outside--------------------");

    // If I create the document from here, it works both ways.
    let document = parse_html().one(basic_xhtml_without_head);

    print_text_nodes_from_document(&document)?;

    println!("FROM DOCUMENT: Printing from outside---------------------");
    let iterator = get_text_nodes_iterator_from_document(&document)?;
    for node in iterator {
        if let Some(text) = node.as_text() {
            let text = text.borrow();
            println!("Text: {}", &text);
        }
    }
    println!("FROM DOCUMENT: End printing from outside---------------------");

    // I don't know why this happends I don't have time to investigate any further
    // this was a pain in the ass. I realized that this lib is not longer maintained.

    Ok(())
}
