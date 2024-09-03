use std::fs::File;
use std::io::{Read, Write};

use html5ever::serialize::SerializeOpts;
use html5ever::serialize::TraversalScope;
use html5ever::tendril::{StrTendril, TendrilSink};
use html5ever::{parse_document, serialize};

use markup5ever_rcdom::SerializableHandle;
use markup5ever_rcdom::{Handle, NodeData, RcDom};

pub fn iterate_text_nodes(
    file_path: &str,
    output_file: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read the file content
    let mut file = File::open(file_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut content.as_bytes())?;

    // Modify text nodes
    walk(&dom.document);

    let mut output = Vec::new();
    let opts = SerializeOpts {
        traversal_scope: TraversalScope::IncludeNode,
        ..Default::default()
    };

    // Serialize from html node, not from document
    for child in dom.document.children.borrow().iter() {
        let serializable = SerializableHandle::from(child.clone());
        serialize(&mut output, &serializable, opts.clone())?;
    }

    // Write the modified content to the output file
    let mut file = File::create(output_file)?;
    file.write_all(&output)?;

    Ok(())
}

fn walk(handle: &Handle) {
    // Modify text nodes
    if let NodeData::Text { contents } = &handle.data {
        let mut text = contents.borrow_mut();
        if !text.trim().is_empty() {
            *text = StrTendril::from(format!("Text({})EndText", text));
        }
    }

    // Don't modify children from style node
    match handle.data {
        // I don't want to modify children from style node
        NodeData::Element { ref name, .. } if name.local.as_ref() == "style" => {}
        _ => {
            for child in handle.children.borrow().iter() {
                walk(child);
            }
        }
    }
}
