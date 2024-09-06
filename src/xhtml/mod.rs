use regex::Regex;
use std::fs::File;
use std::io::{Read, Write};

use std::path::PathBuf;
use std::rc::Rc;

use html5ever::serialize::SerializeOpts;
use html5ever::serialize::TraversalScope;
use html5ever::tendril::TendrilSink;
use html5ever::{parse_document, serialize};

use markup5ever_rcdom::SerializableHandle;
use markup5ever_rcdom::{Node, NodeData, RcDom};

// Parses a string containing XHTML and returns the document node.
pub fn get_document_node(content: &str) -> Result<Rc<Node>, Box<dyn std::error::Error>> {
    // remove self closing span tags <span*/> => <span*></span>
    let re = Regex::new(r"<span([^>]*?)/>")?;
    let content = re.replace_all(&content, "<span$1></span>");

    let rc_dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut content.as_bytes())?;

    Ok(rc_dom.document)
}

// TODO: Optimize
// Gets all descendant text nodes from a node, use it on document node to get all text nodes
// Depth-first search, but this is not used for serialization so the order is not important so far.
pub fn get_text_nodes(node: &Rc<Node>) -> Result<Vec<Rc<Node>>, Box<dyn std::error::Error>> {
    let mut text_nodes = Vec::new();

    match &node.data {
        NodeData::Text { .. } => {
            text_nodes.push(node.clone());
        }
        NodeData::Element { ref name, .. } if name.local.as_ref() == "style" => {}
        _ => {
            for child in node.children.borrow().iter() {
                let child_text_nodes = get_text_nodes(&child)?;
                text_nodes.extend(child_text_nodes);
            }
        }
    }

    Ok(text_nodes)
}

pub fn serialize_document(
    document: &Rc<Node>,
    output_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let output_string = serialize_document_to_string(document)?;

    let mut file = File::create(output_path)?;
    file.write_all(output_string.as_bytes())?;

    Ok(())
}

pub fn serialize_document_to_string(
    document: &Rc<Node>,
) -> Result<String, Box<dyn std::error::Error>> {
    let opts = SerializeOpts {
        traversal_scope: TraversalScope::IncludeNode,
        ..Default::default()
    };

    let mut output = Vec::new();

    // Serialize from html node, not from document. The html node is the only child of document.
    for node in document.children.borrow().iter() {
        let serializable = SerializableHandle::from(node.clone());
        serialize(&mut output, &serializable, opts.clone())?;
    }

    let mut output_string = String::from_utf8(output)?;

    // Change `>` to `/>` in self closing tags
    let self_closing_tags = ["br", "hr", "img", "input", "link", "meta"];
    for tag in self_closing_tags.iter() {
        let re = Regex::new(&format!(r"(<{}[^>]*?)>", tag).to_string())?;
        output_string = re.replace_all(&output_string, "$1/>").to_string();
    }

    // Todo: test
    let re_nbsp = Regex::new(r"&nbsp;")?;
    output_string = re_nbsp.replace_all(&output_string, "\u{00A0}").to_string();

    // Todo replace empty spans by selfclosing spans?
    let re_span = Regex::new(r"<span([^>]*?)></span>")?;
    output_string = re_span.replace_all(&output_string, "<span$1/>").to_string();

    Ok(output_string)
}

pub fn get_document_node_from_path(
    file_path: &PathBuf,
) -> Result<Rc<Node>, Box<dyn std::error::Error>> {
    // Read file content
    let mut file = File::open(file_path)?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;

    let document = get_document_node(&buf)?;

    Ok(document)
}

pub fn get_text_nodes_from_path(
    file_path: &PathBuf,
) -> Result<Vec<Rc<Node>>, Box<dyn std::error::Error>> {
    let document = get_document_node_from_path(file_path)?;
    get_text_nodes(&document)
}

#[cfg(test)]
mod tests {
    use super::*;
    use html5ever::tendril::StrTendril;
    use std::fs;

    #[test]
    fn test_modify_text_nodes() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;

        let input_file = "tests/data/lorem.xhtml";
        let output_file = temp_dir.path().join("lorem_processed.xhtml");
        let expected_file = "tests/data/lorem_expected.xhtml";

        // Create the root node of the DOM.
        let document: Rc<Node> = get_document_node_from_path(&PathBuf::from(input_file))?;

        // Get all the descendant text nodes.
        let text_nodes = get_text_nodes(&document)?;

        // Modify the text nodes
        for node in text_nodes.iter() {
            if let NodeData::Text { contents } = &node.data {
                let mut text = contents.borrow_mut();
                if !text.trim().is_empty() {
                    *text = StrTendril::from(format!("Text({})EndText", text));
                }
            }
        }

        // Serialize the DOM
        serialize_document(&document, &output_file)?;

        let processed_content = fs::read_to_string(output_file)?;
        let expected_content = fs::read_to_string(expected_file)?;

        assert_eq!(processed_content, expected_content);

        Ok(())
    }

    #[test]
    fn test_serialize_and_deserialize() -> Result<(), Box<dyn std::error::Error>> {
        let input_xhtml = r#"
            <html>
                <head>
                </head>
                <body>
                    <figure class="figure-class" id="img-ge1">
                        <span epub:type="pagebreak" id="pg5"/>
                        <img alt="ima" id="im01" src="../images/pg01.jpg">
                        <figcaption id="fig01">Figure caption.</figcaption>
                    </figure>
                </body>
            </html>
        "#;

        let expexted_xhtml = "<html><head>\n                </head>\n                <body>\n                    <figure class=\"figure-class\" id=\"img-ge1\">\n                        <span epub:type=\"pagebreak\" id=\"pg5\"/>\n                        <img alt=\"ima\" id=\"im01\" src=\"../images/pg01.jpg\"/>\n                        <figcaption id=\"fig01\">Figure caption.</figcaption>\n                    </figure>\n                \n            \n        </body></html>";

        let document = get_document_node(input_xhtml)?;
        let processed_input_xhtml = serialize_document_to_string(&document)?;

        assert_eq!(processed_input_xhtml, expexted_xhtml);

        Ok(())
    }
}
