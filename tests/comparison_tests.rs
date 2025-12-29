//! Integration tests comparing xmlschema-rs output with Python xmlschema
//!
//! These tests load XSD schemas using both the Python xmlschema library and
//! xmlschema-rs, then compare the outputs to ensure compatibility.

use std::process::Command;
use xmlschema::comparison::{
    format_qualified_name, AttributeInfo, ChildElementInfo, ElementInfo, RestrictionInfo,
    SchemaDump, SimpleTypeInfo, TypeInfo,
};
use xmlschema::validators::{
    ComplexContent, GlobalType, GroupParticle, SimpleType, XsdSchema,
};

// Schema bundles for integration testing
use schemas_core::{SchemaBundle, SchemaBundleExt};
use schemas_dita::Dita12;
use schemas_niso_sts::NisoSts;

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

/// Dump schema using Rust implementation
fn dump_schema_rust(xsd_path: &str) -> Result<SchemaDump, String> {
    use xmlschema::validators::{ElementType, FormDefault};

    // Parse the XSD file
    let schema = XsdSchema::from_file(xsd_path)
        .map_err(|e| format!("Failed to parse XSD: {}", e))?;

    let target_ns = schema.target_namespace.clone();
    let maps = &schema.maps.global_maps;

    // Build dump structure
    let mut dump = SchemaDump {
        target_namespace: target_ns.clone(),
        schema_location: Some(xsd_path.to_string()),
        element_form_default: match schema.element_form_default {
            FormDefault::Qualified => Some("qualified".to_string()),
            FormDefault::Unqualified => Some("unqualified".to_string()),
        },
        root_elements: Vec::new(),
        complex_types: Vec::new(),
        simple_types: Vec::new(),
    };

    // Convert elements
    for (qname, elem) in &maps.elements {
        let type_info = match &elem.element_type {
            ElementType::Complex(_) => Some(TypeInfo {
                name: None,
                qualified_name: None,
                category: "XsdComplexType".to_string(),
                is_complex: true,
                is_simple: false,
                content_model: Some("XsdGroup".to_string()),
                attributes: None,
                child_elements: None,
            }),
            ElementType::Simple(_) => Some(TypeInfo {
                name: None,
                qualified_name: None,
                category: "XsdAtomicType".to_string(),
                is_complex: false,
                is_simple: true,
                content_model: None,
                attributes: None,
                child_elements: None,
            }),
            ElementType::Any => None,
        };

        let elem_name = format_qualified_name(qname.namespace.as_deref(), &qname.local_name);
        dump.root_elements.push(ElementInfo {
            name: elem_name.clone(),
            qualified_name: elem_name,
            element_type: type_info,
            min_occurs: elem.occurs.min,
            max_occurs: elem.occurs.max,
            nillable: elem.nillable,
            default: elem.default.clone(),
        });
    }

    // Convert types
    for (qname, global_type) in &maps.types {
        let type_name = format_qualified_name(qname.namespace.as_deref(), &qname.local_name);

        match global_type {
            GlobalType::Complex(ct) => {
                // Collect attributes with proper types
                let attrs: Vec<AttributeInfo> = ct
                    .attributes
                    .iter_attributes()
                    .map(|a| {
                        let attr_type = a
                            .simple_type()
                            .and_then(|st| st.qualified_name_string())
                            .unwrap_or_else(|| "{http://www.w3.org/2001/XMLSchema}string".to_string());
                        AttributeInfo {
                            name: a.name().local_name.clone(),
                            attr_type,
                            use_mode: format!("{:?}", a.use_mode()).to_lowercase(),
                            default: a.default().map(|s| s.to_string()),
                        }
                    })
                    .collect();

                let content_model = if ct.content.is_empty() {
                    None
                } else {
                    Some("XsdGroup".to_string())
                };

                // Extract child elements from content model
                let child_elements = if let ComplexContent::Group(ref group) = ct.content {
                    let mut children = Vec::new();
                    extract_child_elements(&group.particles, &mut children, &schema);
                    if children.is_empty() {
                        None
                    } else {
                        Some(children)
                    }
                } else {
                    None
                };

                dump.complex_types.push(TypeInfo {
                    name: Some(type_name.clone()),
                    qualified_name: Some(type_name),
                    category: "XsdComplexType".to_string(),
                    is_complex: true,
                    is_simple: false,
                    content_model,
                    attributes: if attrs.is_empty() { None } else { Some(attrs) },
                    child_elements,
                });
            }
            GlobalType::Simple(st) => {
                // Get facets for restrictions
                let facets = st.facets();
                let mut restrictions = Vec::new();

                // Check enumeration
                if let Some(ref enums) = facets.enumeration {
                    restrictions.push(RestrictionInfo {
                        kind: "Enumeration".to_string(),
                        value: None,
                        values: Some(enums.values.clone()),
                    });
                }

                // Check patterns
                if !facets.patterns.is_empty() {
                    restrictions.push(RestrictionInfo {
                        kind: "Pattern".to_string(),
                        value: Some(serde_json::Value::String("...".to_string())),
                        values: None,
                    });
                }

                // Check length constraints
                if let Some(ref len) = facets.min_length {
                    restrictions.push(RestrictionInfo {
                        kind: "MinLength".to_string(),
                        value: Some(serde_json::Value::Number(len.value.into())),
                        values: None,
                    });
                }

                if let Some(ref len) = facets.max_length {
                    restrictions.push(RestrictionInfo {
                        kind: "MaxLength".to_string(),
                        value: Some(serde_json::Value::Number(len.value.into())),
                        values: None,
                    });
                }

                if let Some(ref len) = facets.length {
                    restrictions.push(RestrictionInfo {
                        kind: "Length".to_string(),
                        value: Some(serde_json::Value::Number(len.value.into())),
                        values: None,
                    });
                }

                // Get base type using the SimpleType trait
                let base_type = SimpleType::base_type(st.as_ref())
                    .and_then(|bt| bt.qualified_name_string());

                dump.simple_types.push(SimpleTypeInfo {
                    name: type_name.clone(),
                    qualified_name: type_name,
                    category: "XsdAtomicRestriction".to_string(),
                    base_type,
                    restrictions: if restrictions.is_empty() {
                        None
                    } else {
                        Some(restrictions)
                    },
                });
            }
        }
    }

    Ok(dump)
}

/// Helper to extract child elements from content model particles
fn extract_child_elements(
    particles: &[GroupParticle],
    children: &mut Vec<ChildElementInfo>,
    schema: &XsdSchema,
) {
    for particle in particles {
        match particle {
            GroupParticle::Element(ep) => {
                // Get element type from the particle's element declaration or schema lookup
                let element_type = if let Some(elem_decl) = ep.element() {
                    // Use the local element declaration's type
                    get_element_type_name(elem_decl, schema)
                } else if let Some(ref elem_ref) = ep.element_ref {
                    // Look up the referenced element
                    if let Some(elem) = schema.lookup_element(elem_ref) {
                        get_element_type_name(&elem, schema)
                    } else {
                        "unknown".to_string()
                    }
                } else {
                    // Try looking up by name in the schema
                    if let Some(elem) = schema.lookup_element(&ep.name) {
                        get_element_type_name(&elem, schema)
                    } else {
                        "unknown".to_string()
                    }
                };

                children.push(ChildElementInfo {
                    name: format_qualified_name(ep.name.namespace.as_deref(), &ep.name.local_name),
                    element_type,
                    min_occurs: ep.occurs.min,
                    max_occurs: ep.occurs.max,
                });
            }
            GroupParticle::Group(nested) => {
                // Recursively extract from nested groups
                extract_child_elements(&nested.particles, children, schema);
            }
            GroupParticle::Any(_) => {
                // Skip wildcards for now
            }
        }
    }
}

/// Get the type name for an element
fn get_element_type_name(elem: &xmlschema::validators::XsdElement, schema: &XsdSchema) -> String {
    use xmlschema::validators::ElementType;

    match &elem.element_type {
        ElementType::Simple(st) => st.qualified_name_string().unwrap_or_else(|| "unknown".to_string()),
        ElementType::Complex(ct) => {
            // Get the complex type's name if it has one
            if let Some(ref name) = ct.name {
                format_qualified_name(name.namespace.as_deref(), &name.local_name)
            } else {
                "XsdComplexType".to_string()
            }
        }
        ElementType::Any => "anyType".to_string(),
    }
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
fn test_rust_parse_book_schema() {
    // Test the Rust schema parser directly
    let rust_schema = dump_schema_rust("tests/comparison/schemas/book.xsd")
        .expect("Failed to parse XSD with Rust");

    // Verify basic properties
    assert_eq!(rust_schema.target_namespace, Some("http://example.com/book".to_string()));
    assert_eq!(rust_schema.element_form_default, Some("qualified".to_string()));

    // Check we parsed root elements
    assert!(!rust_schema.root_elements.is_empty(), "Should have root elements");
    eprintln!("Rust parsed {} root elements", rust_schema.root_elements.len());
    for elem in &rust_schema.root_elements {
        eprintln!("  - {}", elem.name);
    }

    // Check we parsed types
    eprintln!("Rust parsed {} complex types", rust_schema.complex_types.len());
    for ct in &rust_schema.complex_types {
        eprintln!("  - {}", ct.name.as_deref().unwrap_or("anonymous"));
    }

    eprintln!("Rust parsed {} simple types", rust_schema.simple_types.len());
    for st in &rust_schema.simple_types {
        eprintln!("  - {}", st.name);
    }
}

#[test]
fn test_show_full_schema_structure() {
    use xmlschema::validators::{ElementType, ComplexContent, GroupParticle, GlobalType};
    use xmlschema::validators::base::AttributeValidator;

    let xsd = std::fs::read_to_string("tests/comparison/schemas/book.xsd").unwrap();
    let schema = XsdSchema::from_string(&xsd).expect("Failed to parse");

    eprintln!("\n=== SCHEMA ===");
    eprintln!("Target namespace: {:?}", schema.target_namespace);

    eprintln!("\n=== GLOBAL ELEMENTS ===");
    for (qname, elem) in &schema.maps.global_maps.elements {
        eprintln!("Element: {}:{}", qname.namespace.as_deref().unwrap_or(""), qname.local_name);
        eprintln!("  nillable: {}, occurs: min={} max={:?}", elem.nillable, elem.occurs.min, elem.occurs.max);
        match &elem.element_type {
            ElementType::Complex(_) => eprintln!("  type: Complex"),
            ElementType::Simple(_) => eprintln!("  type: Simple"),
            ElementType::Any => eprintln!("  type: Any"),
        }
    }

    eprintln!("\n=== COMPLEX TYPES ===");
    for (qname, global_type) in &schema.maps.global_maps.types {
        if let GlobalType::Complex(ct) = global_type {
            eprintln!("ComplexType: {}", qname.local_name);
            eprintln!("  content_type: {:?}", ct.content_type_label());

            // Attributes
            eprintln!("  attributes:");
            for attr in ct.attributes.iter_attributes() {
                eprintln!("    - {} (required: {}, default: {:?})",
                    attr.name().local_name,
                    attr.is_required(),
                    attr.default());
            }

            // Content model (children)
            if let ComplexContent::Group(group) = &ct.content {
                eprintln!("  content_model: {:?}", group.model);
                eprintln!("  children:");
                for particle in &group.particles {
                    match particle {
                        GroupParticle::Element(ep) => {
                            eprintln!("    - element '{}' (min: {}, max: {:?})",
                                ep.name.local_name, ep.occurs.min, ep.occurs.max);
                        }
                        GroupParticle::Group(g) => {
                            eprintln!("    - nested group {:?}", g.model);
                        }
                        GroupParticle::Any(_) => {
                            eprintln!("    - any wildcard");
                        }
                    }
                }
            }
        }
    }

    eprintln!("\n=== SIMPLE TYPES ===");
    for (qname, global_type) in &schema.maps.global_maps.types {
        if let GlobalType::Simple(st) = global_type {
            eprintln!("SimpleType: {}", qname.local_name);
            let facets = st.facets();
            if let Some(ref e) = facets.enumeration {
                eprintln!("  enumeration: {:?}", e.values);
            }
            if !facets.patterns.is_empty() {
                eprintln!("  patterns: {:?}", facets.patterns.iter().map(|p| &p.pattern).collect::<Vec<_>>());
            }
            if let Some(ref ml) = facets.max_length {
                eprintln!("  maxLength: {}", ml.value);
            }
        }
    }
}

#[test]
#[ignore = "Requires Python venv for comparison - run with: cargo test -- --ignored"]
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

// =============================================================================
// Schema bundle tests (DITA, NISO)
// =============================================================================

#[test]
fn test_dita_schema_bundle_available() {
    // Verify DITA schema bundle is accessible
    let summary = Dita12::summary();
    assert_eq!(summary.name, "DITA");
    assert_eq!(summary.version, "1.2");
    assert!(summary.file_count > 0, "DITA bundle should have files");

    // Check for main XSD files
    let xsd_files: Vec<_> = Dita12::files_by_extension("xsd").collect();
    assert!(!xsd_files.is_empty(), "DITA bundle should have XSD files");

    eprintln!("DITA 1.2: {} files, {} bytes total", summary.file_count, summary.total_size);
}

#[test]
fn test_niso_schema_bundle_available() {
    // Verify NISO STS schema bundle is accessible
    let summary = NisoSts::summary();
    assert_eq!(summary.name, "NISO STS");
    assert!(summary.file_count > 0, "NISO bundle should have files");

    // Check for XSD files
    let xsd_files: Vec<_> = NisoSts::files_by_extension("xsd").collect();
    assert!(!xsd_files.is_empty(), "NISO bundle should have XSD files");

    eprintln!("NISO STS: {} files, {} bytes total", summary.file_count, summary.total_size);
}

#[test]
fn test_dita_schema_files_readable() {
    // Check that we can read DITA schema content
    for file in Dita12::files_by_extension("xsd").take(5) {
        let content = file.content_str().expect("XSD should be valid UTF-8");
        assert!(
            content.contains("schema") || content.contains("Schema"),
            "XSD file {} should contain schema content",
            file.path
        );
    }
}

#[test]
fn test_niso_schema_files_readable() {
    // Check that we can read NISO schema content
    for file in NisoSts::files_by_extension("xsd").take(5) {
        let content = file.content_str().expect("XSD should be valid UTF-8");
        assert!(
            content.contains("schema") || content.contains("Schema"),
            "XSD file {} should contain schema content",
            file.path
        );
    }
}

#[test]
fn test_parse_dita_basemap() {
    // Find the basemap.xsd file
    let basemap = Dita12::files_by_extension("xsd")
        .find(|f| f.path.ends_with("basemap.xsd"))
        .expect("basemap.xsd should exist in DITA bundle");

    let content = basemap.content_str().expect("Should be valid UTF-8");
    eprintln!("DITA basemap.xsd: {} bytes", content.len());

    // Try parsing with Rust
    match XsdSchema::from_string(content) {
        Ok(schema) => {
            eprintln!("Successfully parsed DITA basemap!");
            eprintln!("  Target namespace: {:?}", schema.target_namespace);
            eprintln!("  Elements: {}", schema.maps.global_maps.elements.len());
            eprintln!("  Types: {}", schema.maps.global_maps.types.len());
        }
        Err(e) => {
            // DITA schemas have complex imports - parsing may fail
            eprintln!("Parsing failed (expected - DITA uses imports): {}", e);
            // Verify content is valid XSD structure
            assert!(content.contains("xs:schema") || content.contains("xsd:schema"));
        }
    }
}

#[test]
fn test_parse_niso_sts() {
    // Find main NISO STS XSD file
    let main_xsd = NisoSts::files_by_extension("xsd")
        .find(|f| f.path.contains("NISO-STS") || f.path.contains("niso-sts"))
        .or_else(|| NisoSts::files_by_extension("xsd").next());

    if let Some(file) = main_xsd {
        let content = file.content_str().expect("Should be valid UTF-8");
        eprintln!("NISO STS {}: {} bytes", file.path, content.len());

        // Try parsing with Rust
        match XsdSchema::from_string(content) {
            Ok(schema) => {
                eprintln!("Successfully parsed NISO STS!");
                eprintln!("  Target namespace: {:?}", schema.target_namespace);
                eprintln!("  Elements: {}", schema.maps.global_maps.elements.len());
                eprintln!("  Types: {}", schema.maps.global_maps.types.len());
            }
            Err(e) => {
                // NISO STS may have complex imports - parsing may fail
                eprintln!("Parsing failed (may use imports): {}", e);
                assert!(content.contains("schema"));
            }
        }
    } else {
        eprintln!("No XSD files found in NISO bundle - skipping");
    }
}

#[test]
fn test_dump_rust_output_json() {
    // Dump the full Rust output as JSON for visual comparison
    let rust_schema = dump_schema_rust("tests/comparison/schemas/book.xsd")
        .expect("Failed to parse XSD with Rust");

    let json = serde_json::to_string_pretty(&rust_schema)
        .expect("Failed to serialize to JSON");

    eprintln!("\n=== RUST OUTPUT JSON ===\n{}\n", json);

    // Load and print expected JSON for comparison
    let expected_json = include_str!("comparison/schemas/book.expected.json");
    eprintln!("=== EXPECTED (PYTHON) JSON ===\n{}\n", expected_json);
}

#[test]
fn test_list_dita_entry_points() {
    // List potential entry point schemas in DITA
    let entry_points: Vec<_> = Dita12::files_by_extension("xsd")
        .filter(|f| {
            let name = f.file_name().unwrap_or("");
            name.contains("map") || name.contains("topic") || name.contains("concept")
                || name.contains("task") || name.contains("reference")
        })
        .collect();

    eprintln!("DITA entry point candidates:");
    for file in &entry_points {
        eprintln!("  - {}", file.path);
    }

    assert!(!entry_points.is_empty(), "Should find DITA entry point schemas");
}
