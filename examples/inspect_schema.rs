//! Schema Inspection Example
//!
//! This example demonstrates how to parse and inspect an XSD schema,
//! examining its elements, types, and structure.
//!
//! Run with: cargo run --example inspect_schema

use std::path::PathBuf;
use xmlschema::validators::{FormDefault, GlobalType, XsdSchema};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let examples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/data");
    let schema_path = examples_dir.join("book.xsd");

    println!("=== Schema Inspection Example ===\n");
    println!("Loading: {}\n", schema_path.display());

    let schema = XsdSchema::from_file(&schema_path)?;

    // Basic schema information
    println!("--- Schema Metadata ---");
    println!("Target Namespace: {:?}", schema.target_namespace);
    println!(
        "Element Form Default: {}",
        match schema.element_form_default {
            FormDefault::Qualified => "qualified",
            FormDefault::Unqualified => "unqualified",
        }
    );

    // Count components
    let elements = &schema.maps.global_maps.elements;
    let types = &schema.maps.global_maps.types;
    let groups = &schema.maps.global_maps.groups;

    println!("\n--- Component Counts ---");
    println!("Global Elements: {}", elements.len());
    println!("Global Types: {}", types.len());
    println!("Model Groups: {}", groups.len());

    // List global elements
    println!("\n--- Global Elements ---");
    for (name, _elem) in elements.iter() {
        println!("  - {}", name.local_name);
    }

    // List global types
    println!("\n--- Global Types ---");
    for (name, type_def) in types.iter() {
        let kind = match type_def {
            GlobalType::Simple(_) => "simple",
            GlobalType::Complex(_) => "complex",
        };
        println!("  - {} ({})", name.local_name, kind);
    }

    Ok(())
}
