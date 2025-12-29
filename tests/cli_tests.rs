//! CLI integration tests
//!
//! These tests verify the CLI commands work correctly by running the binary.

use std::process::Command;
use std::path::PathBuf;

fn xmlschema_bin() -> PathBuf {
    // Get the path to the built binary
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    path.push("xmlschema");
    path
}

fn fixtures_dir() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path
}

fn schemas_dir() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("comparison");
    path.push("schemas");
    path
}

// ============================================================================
// Inspect Command Tests
// ============================================================================

#[test]
fn test_cli_inspect_basic() {
    let output = Command::new(xmlschema_bin())
        .args(["inspect", schemas_dir().join("book.xsd").to_str().unwrap()])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "inspect should succeed");
    assert!(stdout.contains("xmlschema-rs"), "should show version");
    assert!(stdout.contains("http://example.com/book"), "should show namespace");
    assert!(stdout.contains("Global Elements: 1"), "should show element count");
    assert!(stdout.contains("Global Types: 5"), "should show type count");
}

#[test]
fn test_cli_inspect_json_output() {
    let output = Command::new(xmlschema_bin())
        .args(["inspect", "--json", schemas_dir().join("book.xsd").to_str().unwrap()])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "inspect --json should succeed");

    // Parse as JSON to verify valid output
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Output should be valid JSON");

    assert_eq!(json["targetNamespace"], "http://example.com/book");
    assert_eq!(json["statistics"]["globalElements"], 1);
    assert_eq!(json["statistics"]["globalTypes"], 5);
}

#[test]
fn test_cli_inspect_element_lookup() {
    let output = Command::new(xmlschema_bin())
        .args([
            "inspect",
            "--element", "book",
            schemas_dir().join("book.xsd").to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "element lookup should succeed");
    assert!(stdout.contains("Element:"), "should show element details");
    assert!(stdout.contains("book"), "should show element name");
    assert!(stdout.contains("bookType"), "should show element type");
}

#[test]
fn test_cli_inspect_type_lookup() {
    let output = Command::new(xmlschema_bin())
        .args([
            "inspect",
            "--type-name", "personType",
            schemas_dir().join("book.xsd").to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "type lookup should succeed");
    assert!(stdout.contains("Type:"), "should show type details");
    assert!(stdout.contains("personType"), "should show type name");
    assert!(stdout.contains("complex"), "should show type kind");
}

#[test]
fn test_cli_inspect_nonexistent_element() {
    let output = Command::new(xmlschema_bin())
        .args([
            "inspect",
            "--element", "nonexistent",
            schemas_dir().join("book.xsd").to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success(), "should fail for nonexistent element");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found"), "should report element not found");
}

// ============================================================================
// Validate Command Tests
// ============================================================================

#[test]
fn test_cli_validate_valid_document() {
    let output = Command::new(xmlschema_bin())
        .args([
            "validate",
            "--schema", schemas_dir().join("book.xsd").to_str().unwrap(),
            fixtures_dir().join("book_simple.xml").to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "validation should succeed for valid document");
    assert!(stdout.contains("Document is valid"), "should report document valid");
}

#[test]
fn test_cli_validate_invalid_document() {
    let output = Command::new(xmlschema_bin())
        .args([
            "validate",
            "--schema", schemas_dir().join("book.xsd").to_str().unwrap(),
            fixtures_dir().join("book_invalid.xml").to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(!output.status.success(), "validation should fail for invalid document");
    assert!(stdout.contains("Document is invalid"), "should report document invalid");
    assert!(stdout.contains("isbn"), "should mention the invalid attribute");
    assert!(stdout.contains("pattern"), "should mention pattern violation");
}

#[test]
fn test_cli_validate_wrong_namespace() {
    // Create a temp file with wrong namespace
    let temp_dir = std::env::temp_dir();
    let wrong_ns_file = temp_dir.join("wrong_ns.xml");
    std::fs::write(&wrong_ns_file, r#"<?xml version="1.0"?>
<book xmlns="http://wrong.namespace.com">
    <title>Test</title>
</book>
"#).expect("Failed to write temp file");

    let output = Command::new(xmlschema_bin())
        .args([
            "validate",
            "--schema", schemas_dir().join("book.xsd").to_str().unwrap(),
            wrong_ns_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success(), "validation should fail for wrong namespace");

    // Cleanup
    let _ = std::fs::remove_file(wrong_ns_file);
}

#[test]
fn test_cli_validate_lax_mode() {
    let output = Command::new(xmlschema_bin())
        .args([
            "validate",
            "--schema", schemas_dir().join("book.xsd").to_str().unwrap(),
            "--mode", "lax",
            fixtures_dir().join("book_simple.xml").to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "lax validation should succeed for valid document");
}

// ============================================================================
// XML to JSON Command Tests
// ============================================================================

#[test]
fn test_cli_xml2json_basic() {
    let output = Command::new(xmlschema_bin())
        .args([
            "xml2json",
            fixtures_dir().join("book_simple.xml").to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "xml2json should succeed");

    // Parse as JSON to verify valid output
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Output should be valid JSON");

    assert!(json.get("book").is_some(), "should have root element");
}

#[test]
fn test_cli_xml2json_pretty() {
    let output = Command::new(xmlschema_bin())
        .args([
            "xml2json",
            "--pretty",
            fixtures_dir().join("book_simple.xml").to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "xml2json --pretty should succeed");
    assert!(stdout.contains('\n'), "pretty output should have newlines");
    assert!(stdout.contains("  "), "pretty output should have indentation");
}

#[test]
fn test_cli_xml2json_parker_format() {
    let output = Command::new(xmlschema_bin())
        .args([
            "xml2json",
            "--format", "parker",
            fixtures_dir().join("book_simple.xml").to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "xml2json --format parker should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let _json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Parker output should be valid JSON");
}

#[test]
fn test_cli_xml2json_badgerfish_format() {
    let output = Command::new(xmlschema_bin())
        .args([
            "xml2json",
            "--format", "badgerfish",
            fixtures_dir().join("book_simple.xml").to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "xml2json --format badgerfish should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let _json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("BadgerFish output should be valid JSON");
}

#[test]
fn test_cli_xml2json_output_file() {
    let temp_dir = std::env::temp_dir();
    let output_file = temp_dir.join("cli_test_output.json");

    // Remove if exists from previous run
    let _ = std::fs::remove_file(&output_file);

    let output = Command::new(xmlschema_bin())
        .args([
            "xml2json",
            "--output", output_file.to_str().unwrap(),
            fixtures_dir().join("book_simple.xml").to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "xml2json --output should succeed");
    assert!(output_file.exists(), "output file should be created");

    // Verify file contents
    let contents = std::fs::read_to_string(&output_file)
        .expect("Should be able to read output file");
    let _json: serde_json::Value = serde_json::from_str(&contents)
        .expect("Output file should contain valid JSON");

    // Cleanup
    let _ = std::fs::remove_file(output_file);
}

#[test]
fn test_cli_xml2json_invalid_format() {
    let output = Command::new(xmlschema_bin())
        .args([
            "xml2json",
            "--format", "invalid_format",
            fixtures_dir().join("book_simple.xml").to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success(), "should fail for invalid format");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unknown format") || stderr.contains("invalid_format"),
            "should report unknown format");
}

// ============================================================================
// Known Bugs - Tests that document current issues
// ============================================================================

/// BUG: Content model validation fails when unbounded element is followed by optional elements.
///
/// Schema defines: sequence(title, author[1..∞], published[0..1], pages[0..1])
/// Document has: title, author, author, published, pages
///
/// The content model visitor doesn't correctly advance past the unbounded `author`
/// elements to allow the subsequent optional `published` element.
///
/// This test asserts the CURRENT BROKEN BEHAVIOR. When the bug is fixed,
/// this test will fail - update it to assert success instead.
#[test]
fn test_bug_unbounded_followed_by_optional() {
    // book.xml has multiple authors followed by optional published/pages elements
    let output = Command::new(xmlschema_bin())
        .args([
            "validate",
            "--schema", schemas_dir().join("book.xsd").to_str().unwrap(),
            fixtures_dir().join("book.xml").to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // BUG: This document IS valid but validation incorrectly rejects it.
    // When this test fails, the bug has been fixed! Update to assert success.
    assert!(
        !output.status.success(),
        "BUG FIXED? This valid document was previously rejected incorrectly. \
         If validation now passes, update this test to assert success!"
    );
    assert!(
        stdout.contains("Unexpected child element 'published'"),
        "BUG FIXED? Expected the 'published' rejection error. \
         If the error changed, the content model logic may have been updated."
    );
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_cli_inspect_nonexistent_file() {
    let output = Command::new(xmlschema_bin())
        .args(["inspect", "/nonexistent/path/schema.xsd"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success(), "should fail for nonexistent file");
}

#[test]
fn test_cli_validate_nonexistent_schema() {
    let output = Command::new(xmlschema_bin())
        .args([
            "validate",
            "--schema", "/nonexistent/schema.xsd",
            fixtures_dir().join("book_simple.xml").to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success(), "should fail for nonexistent schema");
}

#[test]
fn test_cli_validate_nonexistent_document() {
    let output = Command::new(xmlschema_bin())
        .args([
            "validate",
            "--schema", schemas_dir().join("book.xsd").to_str().unwrap(),
            "/nonexistent/document.xml",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success(), "should fail for nonexistent document");
}

// ============================================================================
// Help and Version Tests
// ============================================================================

#[test]
fn test_cli_help() {
    let output = Command::new(xmlschema_bin())
        .args(["--help"])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "--help should succeed");
    assert!(stdout.contains("inspect"), "help should mention inspect command");
    assert!(stdout.contains("xml2json"), "help should mention xml2json command");
    assert!(stdout.contains("validate"), "help should mention validate command");
}

#[test]
fn test_cli_version() {
    let output = Command::new(xmlschema_bin())
        .args(["--version"])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "--version should succeed");
    assert!(stdout.contains("xmlschema"), "version should show program name");
}

#[test]
fn test_cli_subcommand_help() {
    for subcommand in ["inspect", "validate", "xml2json"] {
        let output = Command::new(xmlschema_bin())
            .args([subcommand, "--help"])
            .output()
            .expect("Failed to execute command");

        assert!(output.status.success(), "{} --help should succeed", subcommand);
    }
}
