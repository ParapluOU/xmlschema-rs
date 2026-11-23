//! The schema for schemas: XSD described in XSD.
//!
//! `tests/spec/` holds the W3C's own copies, fetched from `www.w3.org/2001/`
//! and kept verbatim:
//!
//! | file | what it is |
//! |---|---|
//! | `XMLSchema.xsd` | the schema for schemas, Part 1 (Structures) |
//! | `xml.xsd` | the schema of the `xml:` namespace, which it imports |
//! | `XMLSchema.dtd` | the DTD the schema document declares |
//! | `catalog.xml` | ours: resolves that import to the copy beside it |
//!
//! They carry the W3C's terms (`https://www.w3.org/Consortium/Legal/copyright-documents`).
//! The schema document says of itself that it is **not** normative — "the prose
//! copy in the structures REC is the normative version" — so this is a test of
//! reading what the working group published, not of conformance to the
//! Recommendation. That is what the W3C test suite is for (`w3c_suite.rs`).
//!
//! # Why this is worth a test of its own
//!
//! It is the one schema every XSD processor must be able to read, it is written
//! by the people who defined the language, and it uses nearly all of it: 40-odd
//! global elements, substitution groups, recursive content models, key and
//! keyref, attribute groups, a redefinable structure, and an import resolved
//! through a catalog. A parser that reads this is past the beginner's set of
//! problems.

use std::path::{Path, PathBuf};

use xmlschema::validators::XsdSchema;

fn spec(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/spec").join(name)
}

/// The schema for schemas parses, with its import resolved through a catalog
/// rather than over the network.
#[test]
fn the_schema_for_schemas_parses() {
    let catalog = spec("catalog.xml");
    let schema = XsdSchema::from_file_with_catalog(spec("XMLSchema.xsd"), Some(&catalog))
        .expect("the schema for schemas parses");

    // What it declares. The counts are the schema's, not ours: `xs:schema`,
    // `xs:element`, `xs:complexType` and the rest are global elements of it,
    // because a schema document is itself an XML document of this vocabulary.
    let elements = &schema.maps.global_maps.elements;
    for declared in ["schema", "element", "complexType", "simpleType", "attribute", "group", "attributeGroup", "redefine", "import", "include", "annotation"] {
        assert!(
            elements.keys().any(|name| name.local_name == declared),
            "the schema for schemas declares `{declared}`; it has {:?}",
            elements.keys().map(|name| name.local_name.as_str()).take(12).collect::<Vec<_>>()
        );
    }
    assert!(elements.len() >= 40, "it declares about forty elements, not {}", elements.len());

    // And the types behind them, including the ones every schema leans on.
    let types = &schema.maps.global_maps.types;
    for declared in ["openAttrs", "annotated", "topLevelElement", "localElement", "namedGroup", "attributeGroupRef"] {
        assert!(
            types.keys().any(|name| name.local_name == declared),
            "the schema for schemas declares the type `{declared}`"
        );
    }

    // The import of the `xml:` namespace resolved — through the catalog, to the
    // copy beside it. An import is kept apart rather than merged (that is what
    // separates it from an include: the components stay in their own
    // namespace), so what to look at is the import record.
    let xml_namespace = "http://www.w3.org/XML/1998/namespace";
    let imported = schema
        .imports
        .get(xml_namespace)
        .unwrap_or_else(|| panic!("the schema imports `{xml_namespace}`; it has {:?}", schema.imports.keys().collect::<Vec<_>>()));
    let brought = imported
        .schema
        .as_ref()
        .expect("the import resolved through the catalog, rather than being left as a namespace nobody could find");
    assert!(
        brought.maps.global_maps.attributes.keys().any(|name| name.local_name == "lang"),
        "`xml:lang` and its siblings came with it; it brought {:?}",
        brought.maps.global_maps.attributes.keys().map(|name| name.local_name.as_str()).collect::<Vec<_>>()
    );
}

/// Without the catalog the same import cannot resolve — it names a URL, and
/// nothing here goes to the network.
///
/// The schema still parses: an unresolvable `schemaLocation` is not an error in
/// XSD, it simply contributes nothing. That is the quiet failure mode this
/// whole area keeps producing, so it is pinned here: the difference between
/// having a catalog and not is not an error, it is the `xml:` namespace
/// silently going missing.
#[test]
fn without_the_catalog_the_import_silently_brings_nothing() {
    let schema = XsdSchema::from_file(spec("XMLSchema.xsd")).expect("it still parses");
    let xml_namespace = "http://www.w3.org/XML/1998/namespace";
    let brought = schema.imports.get(xml_namespace).and_then(|import| import.schema.as_ref());
    assert!(
        brought.is_none(),
        "with no catalog the `xml:` import cannot resolve, and nothing says so"
    );
}
