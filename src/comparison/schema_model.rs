//! Schema model for comparison testing
//!
//! These structures match the JSON output format produced by the Python
//! xmlschema library's dump_schema.py script. The goal is for xmlschema-rs
//! to produce identical output.

use serde::{Deserialize, Serialize};

/// Complete schema dump matching Python xmlschema output
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchemaDump {
    /// Target namespace of the schema
    pub target_namespace: Option<String>,

    /// Schema location (file path or URL)
    pub schema_location: Option<String>,

    /// Default form for elements (qualified/unqualified)
    pub element_form_default: Option<String>,

    /// Root element declarations
    pub root_elements: Vec<ElementInfo>,

    /// Named complex type definitions
    pub complex_types: Vec<TypeInfo>,

    /// Named simple type definitions
    pub simple_types: Vec<SimpleTypeInfo>,
}

impl SchemaDump {
    /// Create a new empty schema dump
    pub fn new() -> Self {
        Self {
            target_namespace: None,
            schema_location: None,
            element_form_default: None,
            root_elements: Vec::new(),
            complex_types: Vec::new(),
            simple_types: Vec::new(),
        }
    }
}

impl Default for SchemaDump {
    fn default() -> Self {
        Self::new()
    }
}

/// Element information matching Python output
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ElementInfo {
    /// Element name (qualified format: {namespace}localName)
    pub name: String,

    /// Qualified name (same as name)
    pub qualified_name: String,

    /// Type information for this element
    #[serde(rename = "type")]
    pub element_type: Option<TypeInfo>,

    /// Minimum occurrences
    pub min_occurs: u32,

    /// Maximum occurrences (None means unbounded)
    pub max_occurs: Option<u32>,

    /// Whether the element is nillable
    pub nillable: bool,

    /// Default value
    pub default: Option<String>,
}

/// Child element reference (simpler than full ElementInfo)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChildElementInfo {
    /// Element name (qualified format)
    pub name: String,

    /// Type name (qualified format)
    #[serde(rename = "type")]
    pub element_type: String,

    /// Minimum occurrences
    pub min_occurs: u32,

    /// Maximum occurrences (None means unbounded)
    pub max_occurs: Option<u32>,
}

/// Attribute information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AttributeInfo {
    /// Attribute name (local name, not qualified)
    pub name: String,

    /// Type name (qualified format)
    #[serde(rename = "type")]
    pub attr_type: String,

    /// Use mode: optional, required, prohibited
    #[serde(rename = "use")]
    pub use_mode: String,

    /// Default value
    pub default: Option<String>,
}

/// Type information for complex types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TypeInfo {
    /// Type name (qualified format)
    pub name: Option<String>,

    /// Qualified name (same as name)
    pub qualified_name: Option<String>,

    /// Category (e.g., XsdComplexType, XsdAtomicRestriction)
    pub category: String,

    /// Whether this is a complex type
    pub is_complex: bool,

    /// Whether this is a simple type
    pub is_simple: bool,

    /// Content model type (e.g., XsdGroup for sequence/choice)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_model: Option<String>,

    /// Attributes for complex types
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Vec<AttributeInfo>>,

    /// Child elements for complex types
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_elements: Option<Vec<ChildElementInfo>>,
}

/// Simple type information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimpleTypeInfo {
    /// Type name (qualified format)
    pub name: String,

    /// Qualified name (same as name)
    pub qualified_name: String,

    /// Category (e.g., XsdAtomicRestriction)
    pub category: String,

    /// Base type (qualified format)
    pub base_type: Option<String>,

    /// Restrictions/facets
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restrictions: Option<Vec<RestrictionInfo>>,
}

/// Restriction/facet information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RestrictionInfo {
    /// Kind of restriction (e.g., Enumeration, Pattern, MinLength)
    pub kind: String,

    /// Value for numeric restrictions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,

    /// Values for enumeration restrictions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
}

/// Format a qualified name in the {namespace}localName format
pub fn format_qualified_name(namespace: Option<&str>, local_name: &str) -> String {
    match namespace {
        Some(ns) => format!("{{{}}}{}", ns, local_name),
        None => local_name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_qualified_name() {
        assert_eq!(
            format_qualified_name(Some("http://example.com"), "test"),
            "{http://example.com}test"
        );
        assert_eq!(format_qualified_name(None, "local"), "local");
    }

    #[test]
    fn test_schema_dump_serialization() {
        let dump = SchemaDump {
            target_namespace: Some("http://example.com/book".to_string()),
            schema_location: Some("file:///test.xsd".to_string()),
            element_form_default: Some("qualified".to_string()),
            root_elements: vec![],
            complex_types: vec![],
            simple_types: vec![],
        };

        let json = serde_json::to_string_pretty(&dump).unwrap();
        assert!(json.contains("http://example.com/book"));

        // Round-trip test
        let parsed: SchemaDump = serde_json::from_str(&json).unwrap();
        assert_eq!(dump, parsed);
    }

    #[test]
    fn test_load_reference_json() {
        // Load the reference JSON from Python output
        let reference = include_str!("../../tests/comparison/schemas/book.expected.json");
        let result: Result<SchemaDump, _> = serde_json::from_str(reference);

        assert!(result.is_ok(), "Failed to parse reference JSON: {:?}", result.err());

        let schema = result.unwrap();
        assert_eq!(schema.target_namespace, Some("http://example.com/book".to_string()));
        assert_eq!(schema.element_form_default, Some("qualified".to_string()));
        assert_eq!(schema.root_elements.len(), 1);
        assert_eq!(schema.complex_types.len(), 2);
        assert_eq!(schema.simple_types.len(), 3);
    }

    #[test]
    fn test_element_info_deserialization() {
        let json = r#"{
            "name": "{http://example.com/book}book",
            "qualified_name": "{http://example.com/book}book",
            "type": {
                "name": "{http://example.com/book}bookType",
                "qualified_name": "{http://example.com/book}bookType",
                "category": "XsdComplexType",
                "is_complex": true,
                "is_simple": false,
                "content_model": "XsdGroup"
            },
            "min_occurs": 1,
            "max_occurs": 1,
            "nillable": false,
            "default": null
        }"#;

        let elem: ElementInfo = serde_json::from_str(json).unwrap();
        assert_eq!(elem.name, "{http://example.com/book}book");
        assert!(elem.element_type.is_some());
        assert!(elem.element_type.unwrap().is_complex);
    }

    #[test]
    fn test_simple_type_deserialization() {
        let json = r#"{
            "name": "{http://example.com/book}isbnType",
            "qualified_name": "{http://example.com/book}isbnType",
            "category": "XsdAtomicRestriction",
            "base_type": "{http://www.w3.org/2001/XMLSchema}string"
        }"#;

        let st: SimpleTypeInfo = serde_json::from_str(json).unwrap();
        assert_eq!(st.name, "{http://example.com/book}isbnType");
        assert_eq!(st.category, "XsdAtomicRestriction");
        assert_eq!(st.base_type, Some("{http://www.w3.org/2001/XMLSchema}string".to_string()));
    }
}
