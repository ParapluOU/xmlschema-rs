//! Integration tests comparing xmlschema-rs output with Python xmlschema
//!
//! These tests load XSD schemas using both the Python xmlschema library and
//! xmlschema-rs, then compare the outputs to ensure compatibility.

use std::process::Command;
use xmlschema::comparison::{SchemaDump, SimpleTypeInfo, TypeInfo};

/// Path to the Python venv created for testing
const PYTHON_VENV: &str = "tests/comparison/venv/bin/python";

/// Path to the dump_schema.py script
const DUMP_SCRIPT: &str = "tests/comparison/dump_schema.py";

/// Run the Python schema dumper on an XSD file
fn dump_schema_python(xsd_path: &str) -> Result<SchemaDump, String> {
    let output = Command::new(PYTHON_VENV)
        .arg(DUMP_SCRIPT)
        .arg(xsd_path)
        .arg("--pretty")
        .output()
        .map_err(|e| format!("Failed to run Python: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Python script failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse Python JSON output: {}", e))
}

/// Placeholder: Dump schema using Rust implementation
/// TODO: Implement actual schema parsing once Wave 6+ is complete
fn dump_schema_rust(_xsd_path: &str) -> Result<SchemaDump, String> {
    // Placeholder - return empty schema until full implementation
    Ok(SchemaDump::new())
}

/// Compare two SchemaDumps and report differences
fn compare_schemas(expected: &SchemaDump, actual: &SchemaDump) -> Vec<String> {
    let mut differences = Vec::new();

    // Compare target namespace
    if expected.target_namespace != actual.target_namespace {
        differences.push(format!(
            "target_namespace: expected {:?}, got {:?}",
            expected.target_namespace, actual.target_namespace
        ));
    }

    // Compare element form default
    if expected.element_form_default != actual.element_form_default {
        differences.push(format!(
            "element_form_default: expected {:?}, got {:?}",
            expected.element_form_default, actual.element_form_default
        ));
    }

    // Compare root elements count
    if expected.root_elements.len() != actual.root_elements.len() {
        differences.push(format!(
            "root_elements count: expected {}, got {}",
            expected.root_elements.len(),
            actual.root_elements.len()
        ));
    }

    // Compare complex types count
    if expected.complex_types.len() != actual.complex_types.len() {
        differences.push(format!(
            "complex_types count: expected {}, got {}",
            expected.complex_types.len(),
            actual.complex_types.len()
        ));
    }

    // Compare simple types count
    if expected.simple_types.len() != actual.simple_types.len() {
        differences.push(format!(
            "simple_types count: expected {}, got {}",
            expected.simple_types.len(),
            actual.simple_types.len()
        ));
    }

    // Detailed comparison of simple types
    for expected_st in &expected.simple_types {
        if let Some(actual_st) = actual.simple_types.iter().find(|st| st.name == expected_st.name) {
            compare_simple_types(expected_st, actual_st, &mut differences);
        } else {
            differences.push(format!("Missing simple type: {}", expected_st.name));
        }
    }

    // Detailed comparison of complex types
    for expected_ct in &expected.complex_types {
        if let Some(name) = &expected_ct.name {
            if let Some(actual_ct) = actual.complex_types.iter().find(|ct| ct.name.as_ref() == Some(name)) {
                compare_complex_types(expected_ct, actual_ct, &mut differences);
            } else {
                differences.push(format!("Missing complex type: {}", name));
            }
        }
    }

    differences
}

fn compare_simple_types(expected: &SimpleTypeInfo, actual: &SimpleTypeInfo, diffs: &mut Vec<String>) {
    if expected.category != actual.category {
        diffs.push(format!(
            "simple_type '{}' category: expected '{}', got '{}'",
            expected.name, expected.category, actual.category
        ));
    }
    if expected.base_type != actual.base_type {
        diffs.push(format!(
            "simple_type '{}' base_type: expected {:?}, got {:?}",
            expected.name, expected.base_type, actual.base_type
        ));
    }
}

fn compare_complex_types(expected: &TypeInfo, actual: &TypeInfo, diffs: &mut Vec<String>) {
    let name = expected.name.as_deref().unwrap_or("anonymous");

    if expected.category != actual.category {
        diffs.push(format!(
            "complex_type '{}' category: expected '{}', got '{}'",
            name, expected.category, actual.category
        ));
    }

    if expected.content_model != actual.content_model {
        diffs.push(format!(
            "complex_type '{}' content_model: expected {:?}, got {:?}",
            name, expected.content_model, actual.content_model
        ));
    }

    // Compare attributes
    let expected_attrs = expected.attributes.as_ref().map(|a| a.len()).unwrap_or(0);
    let actual_attrs = actual.attributes.as_ref().map(|a| a.len()).unwrap_or(0);
    if expected_attrs != actual_attrs {
        diffs.push(format!(
            "complex_type '{}' attributes count: expected {}, got {}",
            name, expected_attrs, actual_attrs
        ));
    }

    // Compare child elements
    let expected_elems = expected.child_elements.as_ref().map(|e| e.len()).unwrap_or(0);
    let actual_elems = actual.child_elements.as_ref().map(|e| e.len()).unwrap_or(0);
    if expected_elems != actual_elems {
        diffs.push(format!(
            "complex_type '{}' child_elements count: expected {}, got {}",
            name, expected_elems, actual_elems
        ));
    }
}

#[test]
fn test_load_python_reference() {
    // Test that we can load the pre-generated reference JSON
    let reference_json = include_str!("comparison/schemas/book.expected.json");
    let schema: SchemaDump = serde_json::from_str(reference_json)
        .expect("Failed to parse reference JSON");

    assert_eq!(schema.target_namespace, Some("http://example.com/book".to_string()));
    assert_eq!(schema.root_elements.len(), 1);
    assert_eq!(schema.complex_types.len(), 2);
    assert_eq!(schema.simple_types.len(), 3);

    // Check specific types
    let book_type = schema.complex_types.iter()
        .find(|t| t.name.as_deref() == Some("{http://example.com/book}bookType"))
        .expect("bookType not found");

    assert!(book_type.is_complex);
    assert!(!book_type.is_simple);
    assert_eq!(book_type.content_model, Some("XsdGroup".to_string()));
    assert_eq!(book_type.attributes.as_ref().map(|a| a.len()), Some(2));
    assert_eq!(book_type.child_elements.as_ref().map(|e| e.len()), Some(4));
}

#[test]
#[ignore = "Requires Python venv - run with: cargo test -- --ignored"]
fn test_python_dump_schema() {
    // Test running Python dump_schema.py
    let result = dump_schema_python("tests/comparison/schemas/book.xsd");

    match result {
        Ok(schema) => {
            assert_eq!(schema.target_namespace, Some("http://example.com/book".to_string()));
            assert_eq!(schema.root_elements.len(), 1);
            assert_eq!(schema.complex_types.len(), 2);
            assert_eq!(schema.simple_types.len(), 3);
        }
        Err(e) => {
            // Skip if Python venv not available
            if e.contains("Failed to run Python") {
                eprintln!("Skipping test - Python venv not available: {}", e);
                return;
            }
            panic!("Python dump failed: {}", e);
        }
    }
}

#[test]
#[ignore = "Rust schema parser not yet implemented"]
fn test_compare_book_schema() {
    // Load Python reference
    let python_schema = dump_schema_python("tests/comparison/schemas/book.xsd")
        .expect("Failed to get Python schema dump");

    // Load Rust output
    let rust_schema = dump_schema_rust("tests/comparison/schemas/book.xsd")
        .expect("Failed to get Rust schema dump");

    // Compare
    let differences = compare_schemas(&python_schema, &rust_schema);

    if !differences.is_empty() {
        eprintln!("Schema differences found:");
        for diff in &differences {
            eprintln!("  - {}", diff);
        }
        panic!("Schemas do not match: {} differences", differences.len());
    }
}
