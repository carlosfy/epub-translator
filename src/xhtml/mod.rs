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

    let mut file = File::create(output_path)?;
    file.write_all(&output)?;

    Ok(())
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
        let input_file = "tests/data/lorem.xhtml";
        let output_file = "temp/lorem_processed1.xhtml";
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
        serialize_document(&document, &PathBuf::from(output_file))?;

        let processed_content = fs::read_to_string(output_file)?;
        let expected_content = fs::read_to_string(expected_file)?;

        assert_eq!(processed_content, expected_content);

        Ok(())
    }
}
