//! XML to JSON Conversion Example
//!
//! This example demonstrates how to convert XML documents to JSON
//! using different conversion conventions (Parker, BadgerFish).
//!
//! Run with: cargo run --example xml_to_json

use std::path::PathBuf;
use xmlschema::converters::{create_converter, ConverterType, ElementData, JsonConverter as _};
use xmlschema::documents::Document;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let examples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/data");
    let xml_path = examples_dir.join("book_valid.xml");

    println!("=== XML to JSON Conversion Example ===\n");
    println!("Source: {}\n", xml_path.display());

    // Load and display the XML
    let xml_content = std::fs::read_to_string(&xml_path)?;
    println!("--- Original XML ---");
    println!("{}", xml_content);

    // Parse the document
    let doc = Document::from_string(&xml_content)?;
    let root = doc.root.as_ref().ok_or("No root element")?;

    // Convert Element to ElementData
    let element_data = element_to_data(root);
    let root_name = root.local_name();

    // Default Convention
    println!("--- Default Convention ---\n");
    let default_conv = create_converter(ConverterType::Default);
    let default_json = default_conv.decode(&element_data, 0);
    let wrapped = serde_json::json!({ root_name: default_json });
    println!("{}\n", serde_json::to_string_pretty(&wrapped)?);

    // Parker Convention - simple, compact output
    println!("--- Parker Convention ---");
    println!("(Simple element-to-value mapping, attributes may be lost)\n");
    let parker = create_converter(ConverterType::Parker);
    let parker_json = parker.decode(&element_data, 0);
    let wrapped = serde_json::json!({ root_name: parker_json });
    println!("{}\n", serde_json::to_string_pretty(&wrapped)?);

    // BadgerFish Convention - preserves attributes
    println!("--- BadgerFish Convention ---");
    println!("(Preserves attributes with @ prefix, text with $ key)\n");
    let badgerfish = create_converter(ConverterType::BadgerFish);
    let badgerfish_json = badgerfish.decode(&element_data, 0);
    let wrapped = serde_json::json!({ root_name: badgerfish_json });
    println!("{}\n", serde_json::to_string_pretty(&wrapped)?);

    Ok(())
}

/// Convert a Document Element to ElementData for conversion
fn element_to_data(elem: &xmlschema::documents::Element) -> ElementData {
    let mut data = ElementData::new(elem.local_name());

    // Add text content
    if let Some(text) = &elem.text {
        data = data.with_text(text.clone());
    }

    // Add attributes
    for (qname, value) in &elem.attributes {
        data = data.with_attribute(qname.local_name.clone(), value.clone());
    }

    // Add xmlns declarations
    for (prefix, uri) in elem.namespaces.iter() {
        data = data.with_xmlns(prefix.clone(), uri.clone());
    }

    // Add child elements recursively
    for child in &elem.children {
        let child_data = element_to_data(child);
        let child_json = create_converter(ConverterType::Default).decode(&child_data, 1);
        data = data.with_child(child.local_name(), child_json);
    }

    data
}
