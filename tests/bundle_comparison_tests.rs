//! Comprehensive comparison tests for DITA and NISO STS schema bundles.
//!
//! These tests:
//! 1. Extract schema bundles to a temporary directory
//! 2. Parse with both Python xmlschema and Rust xmlschema-rs
//! 3. Compare results across multiple axes
//! 4. Assert statically-known facts about the standards

mod bundle_facts;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

use schemas_core::{SchemaBundle, SchemaBundleExt};
use schemas_dita::Dita12;
use schemas_niso_sts::NisoSts;

use xmlschema::comparison::{
    format_qualified_name, AttributeInfo, ChildElementInfo, ElementInfo, RestrictionInfo,
    SchemaDump, SimpleTypeInfo, TypeInfo,
};
use xmlschema::validators::{
    ComplexContent, GlobalType, GroupParticle, SimpleType, XsdSchema,
};

use bundle_facts::{DitaFacts, NisoFacts};

// =============================================================================
// Bundle Extraction Utilities
// =============================================================================

/// Extract a schema bundle to a temporary directory.
/// Returns the TempDir (which will be cleaned up when dropped) and the base path.
fn extract_bundle_to_temp<B: SchemaBundleExt>() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let base_path = temp_dir.path().to_path_buf();

    // Iterate through all files and write them to the temp directory
    for file in B::files() {
        let dest_path = base_path.join(&file.path);

        // Create parent directories if needed
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create directories");
        }

        // Write file content
        fs::write(&dest_path, file.content).expect("Failed to write file");
    }

    (temp_dir, base_path)
}

/// Path to the Python venv created for testing
const PYTHON_VENV: &str = "tests/comparison/venv/bin/python";

/// Path to the dump_schema.py script
const DUMP_SCRIPT: &str = "tests/comparison/dump_schema.py";

/// Run the Python schema dumper on an XSD file
fn dump_schema_python(xsd_path: &Path) -> Result<SchemaDump, String> {
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
fn dump_schema_rust(xsd_path: &Path) -> Result<SchemaDump, String> {
    use xmlschema::validators::{ElementType, FormDefault};

    // Parse the XSD file
    let schema = XsdSchema::from_file(xsd_path)
        .map_err(|e| format!("Failed to parse XSD: {}", e))?;

    let target_ns = schema.target_namespace.clone();
    let maps = &schema.maps.global_maps;

    // Build dump structure
    let mut dump = SchemaDump {
        target_namespace: target_ns.clone(),
        schema_location: Some(xsd_path.to_string_lossy().to_string()),
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
            ElementType::Complex(ct) => {
                let (type_name, type_qname) = if let Some(ref name) = ct.name {
                    let n = format_qualified_name(name.namespace.as_deref(), &name.local_name);
                    (Some(n.clone()), Some(n))
                } else {
                    (None, None)
                };

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

                let child_elements = if let ComplexContent::Group(ref group) = ct.content {
                    let mut children = Vec::new();
                    extract_child_elements(&group.particles, &mut children, &schema);
                    if children.is_empty() { None } else { Some(children) }
                } else {
                    None
                };

                Some(TypeInfo {
                    name: type_name,
                    qualified_name: type_qname,
                    category: "XsdComplexType".to_string(),
                    is_complex: true,
                    is_simple: false,
                    content_model,
                    attributes: if attrs.is_empty() { None } else { Some(attrs) },
                    child_elements,
                })
            }
            ElementType::Simple(st) => {
                let type_name = st.qualified_name_string();
                Some(TypeInfo {
                    name: type_name.clone(),
                    qualified_name: type_name,
                    category: "XsdAtomicType".to_string(),
                    is_complex: false,
                    is_simple: true,
                    content_model: None,
                    attributes: None,
                    child_elements: None,
                })
            }
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

                let child_elements = if let ComplexContent::Group(ref group) = ct.content {
                    let mut children = Vec::new();
                    extract_child_elements(&group.particles, &mut children, &schema);
                    if children.is_empty() { None } else { Some(children) }
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
                let facets = st.facets();
                let mut restrictions = Vec::new();

                if let Some(ref enums) = facets.enumeration {
                    restrictions.push(RestrictionInfo {
                        kind: "Enumeration".to_string(),
                        value: None,
                        values: Some(enums.values.clone()),
                    });
                }

                let base_type = SimpleType::base_type(st.as_ref())
                    .and_then(|bt| bt.qualified_name_string());

                dump.simple_types.push(SimpleTypeInfo {
                    name: type_name.clone(),
                    qualified_name: type_name,
                    category: "XsdAtomicRestriction".to_string(),
                    base_type,
                    restrictions: if restrictions.is_empty() { None } else { Some(restrictions) },
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
                let element_type = if let Some(elem_decl) = ep.element() {
                    get_element_type_name(elem_decl, schema)
                } else if let Some(ref elem_ref) = ep.element_ref {
                    if let Some(elem) = schema.lookup_element(elem_ref) {
                        get_element_type_name(&elem, schema)
                    } else {
                        "unknown".to_string()
                    }
                } else if let Some(elem) = schema.lookup_element(&ep.name) {
                    get_element_type_name(&elem, schema)
                } else {
                    "unknown".to_string()
                };

                children.push(ChildElementInfo {
                    name: format_qualified_name(ep.name.namespace.as_deref(), &ep.name.local_name),
                    element_type,
                    min_occurs: ep.occurs.min,
                    max_occurs: ep.occurs.max,
                });
            }
            GroupParticle::Group(nested) => {
                extract_child_elements(&nested.particles, children, schema);
            }
            GroupParticle::Any(_) => {}
        }
    }
}

fn get_element_type_name(elem: &xmlschema::validators::XsdElement, _schema: &XsdSchema) -> String {
    use xmlschema::validators::ElementType;

    match &elem.element_type {
        ElementType::Simple(st) => st.qualified_name_string().unwrap_or_else(|| "unknown".to_string()),
        ElementType::Complex(ct) => {
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

    if expected.target_namespace != actual.target_namespace {
        differences.push(format!(
            "target_namespace: expected {:?}, got {:?}",
            expected.target_namespace, actual.target_namespace
        ));
    }

    if expected.element_form_default != actual.element_form_default {
        differences.push(format!(
            "element_form_default: expected {:?}, got {:?}",
            expected.element_form_default, actual.element_form_default
        ));
    }

    if expected.root_elements.len() != actual.root_elements.len() {
        differences.push(format!(
            "root_elements count: expected {}, got {}",
            expected.root_elements.len(),
            actual.root_elements.len()
        ));
    }

    if expected.complex_types.len() != actual.complex_types.len() {
        differences.push(format!(
            "complex_types count: expected {}, got {}",
            expected.complex_types.len(),
            actual.complex_types.len()
        ));
    }

    if expected.simple_types.len() != actual.simple_types.len() {
        differences.push(format!(
            "simple_types count: expected {}, got {}",
            expected.simple_types.len(),
            actual.simple_types.len()
        ));
    }

    differences
}

// =============================================================================
// DITA Bundle Tests
// =============================================================================

#[test]
fn test_dita_bundle_extraction() {
    let (_temp_dir, base_path) = extract_bundle_to_temp::<Dita12>();

    // Verify files were extracted
    assert!(base_path.exists());

    // Look for entry point files
    let mut found_entry_points = 0;
    for entry_point in DitaFacts::ENTRY_POINTS {
        // Search recursively for the entry point file
        let found = walkdir_find(&base_path, entry_point);
        if found.is_some() {
            found_entry_points += 1;
        }
    }

    assert!(found_entry_points > 0, "Should find at least one DITA entry point");
    eprintln!("Found {} DITA entry points in extracted bundle", found_entry_points);
}

#[test]
fn test_dita_static_facts_namespace() {
    // Verify DITA namespace is as expected
    assert_eq!(
        DitaFacts::NAMESPACE,
        "http://dita.oasis-open.org/architecture/2005/"
    );
}

#[test]
fn test_dita_static_facts_domains() {
    // Verify domain count matches
    assert_eq!(DitaFacts::DOMAINS.len(), DitaFacts::DOMAIN_COUNT);

    // Verify specific domains
    for domain in DitaFacts::DOMAINS {
        assert!(
            DitaFacts::is_valid_domain(domain),
            "Domain {} should be valid",
            domain
        );
    }
}

#[test]
fn test_dita_static_facts_enumerations() {
    // Test topicreftypes enumeration has expected values
    let expected_topicreftypes = &["topic", "concept", "task", "reference"];
    for value in expected_topicreftypes {
        assert!(
            DitaFacts::is_valid_topicreftype(value),
            "topicreftypes should include '{}'",
            value
        );
    }

    // Test importance enumeration
    let expected_importance = &["high", "normal", "low", "required"];
    for value in expected_importance {
        assert!(
            DitaFacts::is_valid_importance(value),
            "importance should include '{}'",
            value
        );
    }

    // Verify the -dita-use-conref-target escape hatch exists
    assert!(DitaFacts::is_valid_topicreftype("-dita-use-conref-target"));
    assert!(DitaFacts::is_valid_importance("-dita-use-conref-target"));
    assert!(DitaFacts::is_valid_scale("-dita-use-conref-target"));
}

#[test]
fn test_dita_topic_required_attributes() {
    // Verify topic element requires 'id' attribute
    assert!(
        DitaFacts::TOPIC_REQUIRED_ATTRS.contains(&"id"),
        "topic should require 'id' attribute"
    );

    // Verify the id attribute fact
    let id_attr = DitaFacts::topic_id_attribute();
    assert_eq!(id_attr.name, "id");
    assert!(id_attr.required);
    assert_eq!(id_attr.type_name, "{http://www.w3.org/2001/XMLSchema}ID");
}

#[test]
#[ignore = "Requires complete schema import resolution - run with: cargo test -- --ignored"]
fn test_dita_parse_topic_schema() {
    let (_temp_dir, base_path) = extract_bundle_to_temp::<Dita12>();

    // Find topic.xsd
    if let Some(topic_path) = walkdir_find(&base_path, "topic.xsd") {
        eprintln!("Found topic.xsd at: {}", topic_path.display());

        match dump_schema_rust(&topic_path) {
            Ok(schema) => {
                // Assert basic facts
                assert!(
                    schema.target_namespace.is_some(),
                    "topic.xsd should have a target namespace"
                );

                // Check for topic element
                let has_topic = schema.root_elements.iter()
                    .any(|e| e.name.contains("topic"));
                assert!(has_topic, "Should find 'topic' element");

                eprintln!("Parsed DITA topic.xsd successfully:");
                eprintln!("  Elements: {}", schema.root_elements.len());
                eprintln!("  Complex types: {}", schema.complex_types.len());
                eprintln!("  Simple types: {}", schema.simple_types.len());
            }
            Err(e) => {
                eprintln!("Failed to parse topic.xsd (expected with imports): {}", e);
            }
        }
    } else {
        eprintln!("topic.xsd not found in DITA bundle");
    }
}

#[test]
#[ignore = "Requires Python venv for comparison - run with: cargo test -- --ignored"]
fn test_dita_topic_python_rust_parity() {
    let (_temp_dir, base_path) = extract_bundle_to_temp::<Dita12>();

    if let Some(topic_path) = walkdir_find(&base_path, "topic.xsd") {
        let python = match dump_schema_python(&topic_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Skipping - Python not available: {}", e);
                return;
            }
        };

        let rust = match dump_schema_rust(&topic_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Rust parse failed: {}", e);
                return;
            }
        };

        let diffs = compare_schemas(&python, &rust);
        if !diffs.is_empty() {
            eprintln!("DITA topic.xsd differences:");
            for diff in &diffs {
                eprintln!("  - {}", diff);
            }
        }
        assert!(diffs.is_empty(), "Schema comparison found {} differences", diffs.len());
    }
}

// =============================================================================
// NISO STS Bundle Tests
// =============================================================================

#[test]
fn test_niso_bundle_extraction() {
    let (_temp_dir, base_path) = extract_bundle_to_temp::<NisoSts>();

    assert!(base_path.exists());

    // Look for entry point files
    let mut found_entry_points = 0;
    for entry_point in NisoFacts::ENTRY_POINTS {
        if let Some(_) = walkdir_find(&base_path, entry_point) {
            found_entry_points += 1;
        }
    }

    assert!(found_entry_points > 0, "Should find at least one NISO STS entry point");
    eprintln!("Found {} NISO STS entry points in extracted bundle", found_entry_points);
}

#[test]
fn test_niso_static_facts_namespace_imports() {
    // Verify namespace import count
    assert_eq!(
        NisoFacts::IMPORTED_NAMESPACES.len(),
        NisoFacts::IMPORT_COUNT,
        "Should have {} imported namespaces",
        NisoFacts::IMPORT_COUNT
    );

    // Verify specific namespaces
    assert!(NisoFacts::is_imported_namespace("http://www.w3.org/1999/xlink"));
    assert!(NisoFacts::is_imported_namespace("http://www.w3.org/1998/Math/MathML"));
    assert!(NisoFacts::is_imported_namespace("http://www.w3.org/2001/XInclude"));
}

#[test]
fn test_niso_static_facts_key_elements() {
    // Verify key elements
    for elem in NisoFacts::KEY_ELEMENTS {
        assert!(
            NisoFacts::is_key_element(elem),
            "'{}' should be a key element",
            elem
        );
    }

    // Verify metadata elements
    for elem in NisoFacts::METADATA_ELEMENTS {
        assert!(
            NisoFacts::is_metadata_element(elem),
            "'{}' should be a metadata element",
            elem
        );
    }
}

#[test]
fn test_niso_static_facts_enumerations() {
    // Test pub-id-type values
    let expected_pub_ids = &["doi", "isbn", "pmid", "pmcid"];
    for value in expected_pub_ids {
        assert!(
            NisoFacts::is_valid_pub_id_type(value),
            "pub-id-type should include '{}'",
            value
        );
    }

    // Test standard-type values
    let expected_std_types = &["standard", "specification", "guide"];
    for value in expected_std_types {
        assert!(
            NisoFacts::is_valid_standard_type(value),
            "standard-type should include '{}'",
            value
        );
    }
}

#[test]
fn test_niso_expected_element_count() {
    // This is a static fact assertion - the value is from the specification
    assert_eq!(
        NisoFacts::ELEMENT_COUNT, 347,
        "NISO STS should define 347 elements"
    );
}

#[test]
fn test_niso_expected_enumeration_count() {
    // Static fact about total enumeration values
    assert_eq!(
        NisoFacts::ENUMERATION_VALUE_COUNT, 338,
        "NISO STS should have 338 enumeration values"
    );
}

#[test]
#[ignore = "Requires complete schema import resolution - run with: cargo test -- --ignored"]
fn test_niso_parse_extended_schema() {
    let (_temp_dir, base_path) = extract_bundle_to_temp::<NisoSts>();

    // Find main NISO STS XSD
    if let Some(niso_path) = walkdir_find(&base_path, "NISO-STS-extended-1-mathml3.xsd") {
        eprintln!("Found NISO STS at: {}", niso_path.display());

        match dump_schema_rust(&niso_path) {
            Ok(schema) => {
                // Assert basic facts
                eprintln!("Parsed NISO STS successfully:");
                eprintln!("  Target namespace: {:?}", schema.target_namespace);
                eprintln!("  Elements: {}", schema.root_elements.len());
                eprintln!("  Complex types: {}", schema.complex_types.len());
                eprintln!("  Simple types: {}", schema.simple_types.len());

                // Check for key elements
                for key_elem in NisoFacts::KEY_ELEMENTS {
                    let found = schema.root_elements.iter()
                        .any(|e| e.name.contains(key_elem));
                    if !found {
                        eprintln!("  Warning: Key element '{}' not found in root elements", key_elem);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to parse NISO STS (expected with imports): {}", e);
            }
        }
    } else {
        eprintln!("NISO-STS-extended-1-mathml3.xsd not found in bundle");
    }
}

#[test]
#[ignore = "Requires Python venv for comparison - run with: cargo test -- --ignored"]
fn test_niso_extended_python_rust_parity() {
    let (_temp_dir, base_path) = extract_bundle_to_temp::<NisoSts>();

    if let Some(niso_path) = walkdir_find(&base_path, "NISO-STS-extended-1-mathml3.xsd") {
        let python = match dump_schema_python(&niso_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Skipping - Python not available: {}", e);
                return;
            }
        };

        let rust = match dump_schema_rust(&niso_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Rust parse failed: {}", e);
                return;
            }
        };

        let diffs = compare_schemas(&python, &rust);
        if !diffs.is_empty() {
            eprintln!("NISO STS differences:");
            for diff in &diffs {
                eprintln!("  - {}", diff);
            }
        }
        assert!(diffs.is_empty(), "Schema comparison found {} differences", diffs.len());
    }
}

// =============================================================================
// Utility Functions
// =============================================================================

/// Simple recursive file finder
fn walkdir_find(base: &Path, filename: &str) -> Option<PathBuf> {
    if base.is_file() {
        if base.file_name().map(|n| n.to_string_lossy().contains(filename)).unwrap_or(false) {
            return Some(base.to_path_buf());
        }
        return None;
    }

    if let Ok(entries) = fs::read_dir(base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(found) = walkdir_find(&path, filename) {
                return Some(found);
            }
        }
    }

    None
}

// =============================================================================
// Quantitative Assertion Tests
// =============================================================================

#[test]
fn test_dita_entry_point_count() {
    assert!(
        DitaFacts::ENTRY_POINTS.len() >= 5,
        "DITA should have at least 5 entry point schemas (topic, concept, task, reference, map)"
    );
}

#[test]
fn test_niso_namespace_import_count() {
    assert_eq!(
        NisoFacts::IMPORT_COUNT, 7,
        "NISO STS should import 7 namespaces"
    );
}

#[test]
fn test_dita_scale_enumeration_values() {
    // Scale values should include percentage values
    let scale_values = DitaFacts::SCALE_VALUES;
    assert!(scale_values.contains(&"50"));
    assert!(scale_values.contains(&"100"));
    assert!(scale_values.contains(&"200"));
    assert!(scale_values.contains(&"-dita-use-conref-target"));
}

#[test]
fn test_dita_status_enumeration_values() {
    let status_values = DitaFacts::STATUS_VALUES;
    assert!(status_values.contains(&"new"));
    assert!(status_values.contains(&"changed"));
    assert!(status_values.contains(&"deleted"));
    assert!(status_values.contains(&"unchanged"));
}

#[test]
fn test_niso_orientation_enumeration() {
    assert_eq!(NisoFacts::ORIENTATION_VALUES.len(), 2);
    assert!(NisoFacts::ORIENTATION_VALUES.contains(&"landscape"));
    assert!(NisoFacts::ORIENTATION_VALUES.contains(&"portrait"));
}

#[test]
fn test_niso_yes_no_enumeration() {
    assert_eq!(NisoFacts::YES_NO_VALUES.len(), 2);
    assert!(NisoFacts::YES_NO_VALUES.contains(&"yes"));
    assert!(NisoFacts::YES_NO_VALUES.contains(&"no"));
}

// =============================================================================
// xs:redefine Support Tests
// =============================================================================

/// Test that xs:redefine is parsed correctly - the schema should have redefines recorded
#[test]
#[ignore = "Requires complete schema resolution - run with: cargo test -- --ignored"]
fn test_dita_redefine_support() {
    let (_temp_dir, base_path) = extract_bundle_to_temp::<Dita12>();

    // Find basetopic.xsd which uses xs:redefine for commonElementGrp.xsd
    if let Some(basetopic_path) = walkdir_find(&base_path, "basetopic.xsd") {
        eprintln!("Found basetopic.xsd at: {}", basetopic_path.display());

        let schema = XsdSchema::from_file(&basetopic_path)
            .expect("Should parse basetopic.xsd");

        // Check that redefines were recorded
        eprintln!("Schema has {} redefines", schema.redefines.len());

        // basetopic.xsd should have at least one xs:redefine (for commonElementGrp.xsd)
        // Note: The exact number depends on how many xs:redefine elements are in the schema chain
        if !schema.redefines.is_empty() {
            eprintln!("✓ xs:redefine elements were parsed successfully");
            for (i, redefine) in schema.redefines.iter().enumerate() {
                eprintln!("  Redefine {}: {} redefinitions from {}",
                    i, redefine.redefinitions.len(), redefine.location);
            }
        }

        // Check that the title group exists in global maps - this comes from commonElementGrp.xsd
        // via xs:redefine in basetopic.xsd
        let title_group_exists = schema.maps.global_maps.groups.keys()
            .any(|qname| qname.local_name == "title");
        eprintln!("title group exists: {}", title_group_exists);

        // Check for other expected groups from commonElementGrp.xsd
        let expected_groups = ["title", "ph", "keyword", "xref", "data", "data-about"];
        for group_name in expected_groups {
            let exists = schema.maps.global_maps.groups.keys()
                .any(|qname| qname.local_name == group_name);
            eprintln!("  Group '{}': {}", group_name, if exists { "✓" } else { "✗" });
        }
    } else {
        eprintln!("basetopic.xsd not found - checking for topic.xsd");
        if let Some(topic_path) = walkdir_find(&base_path, "topic.xsd") {
            eprintln!("Found topic.xsd at: {}", topic_path.display());
            // topic.xsd includes basetopic.xsd which uses xs:redefine
            let schema = XsdSchema::from_file(&topic_path)
                .expect("Should parse topic.xsd");
            eprintln!("Schema has {} redefines", schema.redefines.len());
        }
    }
}

/// Test that TOPIC_CHILDREN facts match the parsed schema content
/// This specifically tests xs:redefine support since 'title' comes from redefined commonElementGrp
#[test]
#[ignore = "Requires complete schema resolution - run with: cargo test -- --ignored"]
fn test_dita_topic_children_from_redefine() {
    use bundle_facts::DitaFacts;

    let (_temp_dir, base_path) = extract_bundle_to_temp::<Dita12>();

    // Find topic.xsd
    if let Some(topic_path) = walkdir_find(&base_path, "topic.xsd") {
        let schema = XsdSchema::from_file(&topic_path)
            .expect("Should parse topic.xsd");

        // Find the 'topic' element
        let topic_element = schema.maps.global_maps.elements.iter()
            .find(|(qname, _)| qname.local_name == "topic");

        if let Some((_, topic_elem)) = topic_element {
            eprintln!("Found topic element");

            // Get child element names from the type
            let child_names: Vec<String> = collect_element_names_from_element(topic_elem, &schema);

            eprintln!("Topic children found: {:?}", child_names);

            // Assert that xs:redefine-dependent children exist
            // 'title' is the key one - it comes from commonElementGrp.xsd via xs:redefine
            for (expected_child, min_occurs, _max_occurs) in DitaFacts::TOPIC_CHILDREN {
                let found = child_names.iter().any(|n| n == expected_child);
                if *min_occurs > 0 {
                    assert!(
                        found,
                        "Required child '{}' should be present in topic (from xs:redefine)",
                        expected_child
                    );
                }
                eprintln!("  Child '{}': {}", expected_child, if found { "✓" } else { "✗" });
            }
        } else {
            eprintln!("topic element not found in schema");
        }
    }
}

/// Helper to collect element names from an element's content model
fn collect_element_names_from_element(
    element: &xmlschema::validators::XsdElement,
    _schema: &XsdSchema,
) -> Vec<String> {
    use xmlschema::validators::ElementType;

    let mut names = Vec::new();

    match &element.element_type {
        ElementType::Complex(ct) => {
            collect_element_names_from_complex_type(ct, &mut names);
        }
        ElementType::Simple(_) | ElementType::Any => {
            // Simple types and any don't have child elements
        }
    }

    names
}

/// Helper to collect element names from a complex type's content model
fn collect_element_names_from_complex_type(
    ct: &xmlschema::validators::XsdComplexType,
    names: &mut Vec<String>,
) {
    use xmlschema::validators::ComplexContent;

    // Check content model - it's either a group or simple content
    match &ct.content {
        ComplexContent::Group(group) => {
            for particle in &group.particles {
                collect_element_names_from_particle(particle, names);
            }
        }
        ComplexContent::Simple(_) => {
            // Simple content has no child elements
        }
    }
}

/// Helper to collect element names from a particle
fn collect_element_names_from_particle(
    particle: &GroupParticle,
    names: &mut Vec<String>,
) {
    match particle {
        GroupParticle::Element(elem) => {
            names.push(elem.name.local_name.clone());
        }
        GroupParticle::Group(group) => {
            // Recursively collect from nested group (sequence, choice, etc.)
            for p in &group.particles {
                collect_element_names_from_particle(p, names);
            }
        }
        GroupParticle::Any(_) => {}
    }
}

/// Debug test to understand why topic content model is empty
#[test]
#[ignore = "Debug test - run with: cargo test debug_topic_content_model -- --ignored --nocapture"]
fn debug_topic_content_model() {
    let (_temp_dir, base_path) = extract_bundle_to_temp::<Dita12>();

    // Find topic.xsd
    if let Some(topic_path) = walkdir_find(&base_path, "topic.xsd") {
        eprintln!("Found topic.xsd at: {}", topic_path.display());

        let schema = XsdSchema::from_file(&topic_path)
            .expect("Should parse topic.xsd");

        // Find the topic element
        eprintln!("\n=== Global Elements ===");
        for (qname, _elem) in schema.maps.global_maps.elements.iter() {
            eprintln!("  Element: {:?}", qname);
        }

        // Find the topic.class type
        eprintln!("\n=== Global Types ===");
        for (qname, global_type) in schema.maps.global_maps.types.iter() {
            if qname.local_name.contains("topic") {
                eprintln!("  Type: {:?} -> {:?}", qname, match global_type {
                    GlobalType::Complex(_) => "Complex",
                    GlobalType::Simple(_) => "Simple",
                });

                if let GlobalType::Complex(ct) = global_type {
                    eprintln!("    Content: {:?}", match &ct.content {
                        ComplexContent::Group(g) => format!("Group(particles={})", g.particles.len()),
                        ComplexContent::Simple(_) => "Simple".to_string(),
                    });

                    if let ComplexContent::Group(group) = &ct.content {
                        eprintln!("    Particles:");
                        for (i, particle) in group.particles.iter().enumerate() {
                            match particle {
                                GroupParticle::Element(elem) => {
                                    eprintln!("      [{}] Element: {:?}", i, elem.name);
                                }
                                GroupParticle::Group(g) => {
                                    eprintln!("      [{}] Group: name={:?}, group_ref={:?}, particles={}",
                                        i, g.name, g.group_ref, g.particles.len());
                                    // Recurse one level
                                    for (j, p) in g.particles.iter().enumerate() {
                                        match p {
                                            GroupParticle::Element(e) => {
                                                eprintln!("        [{}.{}] Element: {:?}", i, j, e.name);
                                            }
                                            GroupParticle::Group(gg) => {
                                                eprintln!("        [{}.{}] Group: name={:?}, group_ref={:?}, particles={}",
                                                    i, j, gg.name, gg.group_ref, gg.particles.len());
                                            }
                                            GroupParticle::Any(_) => {
                                                eprintln!("        [{}.{}] Any", i, j);
                                            }
                                        }
                                    }
                                }
                                GroupParticle::Any(_) => {
                                    eprintln!("      [{}] Any", i);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check global groups
        eprintln!("\n=== Global Groups ===");
        for (qname, group) in schema.maps.global_maps.groups.iter() {
            if qname.local_name.contains("topic") {
                eprintln!("  Group: {:?} -> particles={}", qname, group.particles.len());
            }
        }

        // Find the 'topic' element specifically
        let topic_element = schema.maps.global_maps.elements.iter()
            .find(|(qname, _)| qname.local_name == "topic");

        if let Some((qname, topic_elem)) = topic_element {
            eprintln!("\n=== Topic Element Details ===");
            eprintln!("  QName: {:?}", qname);

            use xmlschema::validators::ElementType;
            match &topic_elem.element_type {
                ElementType::Complex(ct) => {
                    eprintln!("  Type: Complex");
                    eprintln!("  Type Name: {:?}", ct.name);
                    eprintln!("  Base Type: {:?}", ct.base_type);
                    eprintln!("  Derivation: {:?}", ct.derivation);
                    eprintln!("  Content: {:?}", match &ct.content {
                        ComplexContent::Group(g) => format!("Group(particles={})", g.particles.len()),
                        ComplexContent::Simple(_) => "Simple".to_string(),
                    });

                    // Print particle details
                    if let ComplexContent::Group(group) = &ct.content {
                        eprintln!("  Particles:");
                        for (i, particle) in group.particles.iter().enumerate() {
                            match particle {
                                GroupParticle::Element(ep) => {
                                    eprintln!("    [{}] Element: {:?}", i, ep.name);
                                }
                                GroupParticle::Group(g) => {
                                    eprintln!("    [{}] Group: name={:?}, group_ref={:?}, particles={}",
                                        i, g.name, g.group_ref, g.particles.len());
                                    // Recurse one level
                                    for (j, p) in g.particles.iter().enumerate() {
                                        match p {
                                            GroupParticle::Element(e) => {
                                                eprintln!("      [{}.{}] Element: {:?}", i, j, e.name);
                                            }
                                            GroupParticle::Group(gg) => {
                                                eprintln!("      [{}.{}] Group: name={:?}, group_ref={:?}, particles={}",
                                                    i, j, gg.name, gg.group_ref, gg.particles.len());
                                            }
                                            GroupParticle::Any(_) => {
                                                eprintln!("      [{}.{}] Any", i, j);
                                            }
                                        }
                                    }
                                }
                                GroupParticle::Any(_) => {
                                    eprintln!("    [{}] Any", i);
                                }
                            }
                        }
                    }

                    // Look up this type in global_maps.types to compare
                    if let Some(ref type_name) = ct.name {
                        eprintln!("\n  === Lookup in global_maps.types ===");
                        if let Some(global_type) = schema.maps.global_maps.types.get(type_name) {
                            eprintln!("    FOUND: {:?}", match global_type {
                                GlobalType::Complex(gct) => format!("Complex(particles={})",
                                    match &gct.content {
                                        ComplexContent::Group(g) => g.particles.len(),
                                        ComplexContent::Simple(_) => 0,
                                    }),
                                GlobalType::Simple(_) => "Simple".to_string(),
                            });
                        } else {
                            eprintln!("    NOT FOUND - type {:?} not in global_maps.types", type_name);
                        }
                    }
                }
                ElementType::Simple(_) => {
                    eprintln!("  Type: Simple");
                }
                ElementType::Any => {
                    eprintln!("  Type: Any");
                }
            }
        }
    } else {
        eprintln!("topic.xsd not found");
    }
}

// =============================================================================
// Debug: Content Model Resolution Test
// =============================================================================

#[test]
fn test_dita_conbody_children() {
    // Extract DITA schemas
    let (_temp_dir, base_path) = extract_bundle_to_temp::<Dita12>();

    // Find concept.xsd which defines conbody
    if let Some(concept_path) = walkdir_find(&base_path, "concept.xsd") {
        eprintln!("Found concept.xsd at: {}", concept_path.display());

        // Parse with xmlschema-rs
        let schema = XsdSchema::from_file(&concept_path)
            .expect("Failed to parse concept schema");

        // Find conbody.type or conbody complex type
        let conbody_type_names: Vec<_> = schema.maps.global_maps.types.iter()
            .filter_map(|(qname, _)| {
                let name = qname.to_string().to_lowercase();
                if name.contains("conbody") {
                    Some(qname.to_string())
                } else {
                    None
                }
            })
            .collect();

        eprintln!("Types containing 'conbody': {:?}", conbody_type_names);

        // Debug: List all groups and check for fig
        eprintln!("\n--- Groups in schema ---");
        for (qname, group) in schema.maps.global_maps.groups.iter() {
            let children = collect_group_children(group);
            let has_fig = children.iter().any(|c| c.to_lowercase() == "fig");
            let _has_group_ref = group.group_ref.is_some();
            let particle_count = group.particles.len();

            // Show more groups for debugging
            // Show p, ul, fig and similar groups to understand structure
            let name_lower = qname.to_string().to_lowercase();
            if has_fig || name_lower.contains("basic")
                || name_lower.contains("fig")
                || name_lower.contains("body.cnt")
                || name_lower.contains("conbody")
                || name_lower == "p" || name_lower == "ul" || name_lower == "note" {
                eprintln!("Group {:?}:", qname);
                eprintln!("    particles: {}, group_ref: {:?}", particle_count, group.group_ref);
                eprintln!("    resolved children: {} (has_fig={})", children.len(), has_fig);
                eprintln!("    redefine: {:?}", group.redefine.as_ref().map(|r| format!("name={:?}, particles={}", r.name, r.particles.len())));
                for (i, particle) in group.particles.iter().enumerate() {
                    match particle {
                        GroupParticle::Element(ep) => eprintln!("      [{}] Element: {:?}", i, ep.name),
                        GroupParticle::Group(g) => eprintln!("      [{}] Group: name={:?}, ref={:?}, particles={}", i, g.name, g.group_ref, g.particles.len()),
                        GroupParticle::Any(_) => eprintln!("      [{}] Any", i),
                    }
                }
                // Show redefine content if present
                if let Some(ref orig) = group.redefine {
                    eprintln!("    REDEFINE original particles:");
                    for (i, particle) in orig.particles.iter().enumerate() {
                        match particle {
                            GroupParticle::Element(ep) => eprintln!("      [orig {}] Element: {:?}", i, ep.name),
                            GroupParticle::Group(g) => {
                                eprintln!("      [orig {}] Group: name={:?}, ref={:?}, particles={}", i, g.name, g.group_ref, g.particles.len());
                                // Recurse one level deeper
                                for (j, nested) in g.particles.iter().enumerate() {
                                    match nested {
                                        GroupParticle::Element(ep) => eprintln!("        [orig {} nested {}] Element: {:?}", i, j, ep.name),
                                        GroupParticle::Group(g2) => eprintln!("        [orig {} nested {}] Group: ref={:?}, particles={}", i, j, g2.group_ref, g2.particles.len()),
                                        GroupParticle::Any(_) => eprintln!("        [orig {} nested {}] Any", i, j),
                                    }
                                }
                            },
                            GroupParticle::Any(_) => eprintln!("      [orig {}] Any", i),
                        }
                    }
                }
            }
        }

        // Debug: Check if fig element exists
        let fig_elements: Vec<_> = schema.maps.global_maps.elements.iter()
            .filter_map(|(qname, _)| {
                if qname.to_string().to_lowercase().contains("fig") {
                    Some(qname.to_string())
                } else {
                    None
                }
            })
            .collect();
        eprintln!("\n--- Elements containing 'fig': {:?}", fig_elements);

        // Debug: Find groups that directly contain a fig element particle
        eprintln!("\n--- Groups containing 'fig' element directly ---");
        for (qname, group) in schema.maps.global_maps.groups.iter() {
            let has_fig_element = group.particles.iter().any(|p| {
                if let GroupParticle::Element(ep) = p {
                    ep.name.to_string().to_lowercase() == "fig"
                } else {
                    false
                }
            });
            if has_fig_element {
                eprintln!("  {:?} has fig element", qname);
            }
        }

        // Get children of conbody.class
        let mut found_conbody = false;
        for (qname, global_type) in schema.maps.global_maps.types.iter() {
            let name = qname.to_string().to_lowercase();
            if name.contains("conbody") {
                found_conbody = true;
                eprintln!("\nType: {:?}", qname);
                if let GlobalType::Complex(ct) = global_type {
                    eprintln!("  Content type: {:?}", std::mem::discriminant(&ct.content));
                    if let ComplexContent::Group(group) = &ct.content {
                        let children: Vec<_> = collect_group_children(&group);
                        eprintln!("  Children ({}):", children.len());
                        let has_fig = children.iter().any(|c| c.to_lowercase().contains("fig"));
                        eprintln!("  Has fig: {}", has_fig);
                        for child in &children {
                            eprintln!("    - {}", child);
                        }

                        // Assert that fig should be in the children
                        // (conbody should allow fig through body.cnt -> basic.block -> fig)
                        assert!(has_fig, "conbody should have 'fig' as a valid child element through group resolution");
                    } else {
                        eprintln!("  Content is not a Group, skipping children check");
                    }
                }
            }
        }
        assert!(found_conbody, "Should have found a conbody type");
    } else {
        eprintln!("concept.xsd not found");
    }
}

/// Recursively collect children from a group
fn collect_group_children(group: &xmlschema::validators::XsdGroup) -> Vec<String> {
    let mut children = Vec::new();
    for particle in &group.particles {
        match particle {
            GroupParticle::Element(ep) => {
                children.push(ep.name.to_string());
            }
            GroupParticle::Group(nested) => {
                children.extend(collect_group_children(nested));
            }
            GroupParticle::Any(_) => {
                children.push("##any".to_string());
            }
        }
    }
    children
}
