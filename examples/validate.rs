//! Document Validation Example
//!
//! This example demonstrates how to validate XML documents against an XSD schema.
//!
//! Run with: cargo run --example validate

use std::path::PathBuf;
use xmlschema::documents::Document;
use xmlschema::validators::XsdSchema;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let examples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/data");

    // Load the schema
    let schema_path = examples_dir.join("book.xsd");
    println!("Loading schema: {}", schema_path.display());
    let schema = XsdSchema::from_file(&schema_path)?;
    println!("Schema loaded successfully!\n");

    // Validate a valid document
    let valid_path = examples_dir.join("book_valid.xml");
    println!("Validating: {}", valid_path.display());
    let valid_xml = std::fs::read_to_string(&valid_path)?;
    let valid_doc = Document::from_string(&valid_xml)?;

    if schema.is_valid(&valid_doc) {
        println!("  Result: Document is valid!\n");
    } else {
        println!("  Result: Document is invalid!\n");
    }

    // Validate an invalid document
    let invalid_path = examples_dir.join("book_invalid.xml");
    println!("Validating: {}", invalid_path.display());
    let invalid_xml = std::fs::read_to_string(&invalid_path)?;
    let invalid_doc = Document::from_string(&invalid_xml)?;

    if schema.is_valid(&invalid_doc) {
        println!("  Result: Document is valid!\n");
    } else {
        println!("  Result: Document is invalid!\n");
    }

    // Using validate() for detailed errors
    println!("Detailed validation of invalid document:");
    let result = schema.validate(&invalid_doc);
    if result.valid {
        println!("  Valid!");
    } else {
        for error in &result.errors {
            println!("  - {}", error);
        }
    }

    Ok(())
}
