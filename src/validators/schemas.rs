//! XML Schema validators
//!
//! This module contains the main Schema validator that orchestrates
//! XSD parsing, building, and validation.
//!
//! Based on xmlschema/validators/schemas.py

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use super::attributes::{XsdAttribute, XsdAttributeGroup};
use super::base::{ValidationMode, ValidationStatus, ValidityStatus, Validator};
use super::builders::{XsdBuilders, XsdVersion};
use super::elements::XsdElement;
use super::globals::{XsdGlobals, XsdNotation};
use super::groups::XsdGroup;
use super::simple_types::SimpleType;

use crate::error::{ParseError, Result};
use crate::namespaces::QName;

// Re-export from builtins for local use
use super::builtins::XSD_NAMESPACE;
use super::globals::GlobalType;

/// XML namespace
pub const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";

/// XML Schema Instance namespace
pub const XSI_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema-instance";

/// Versioning namespace for XSD 1.1
pub const VC_NAMESPACE: &str = "http://www.w3.org/2007/XMLSchema-versioning";

/// Form default for elements and attributes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FormDefault {
    /// Unqualified (default)
    #[default]
    Unqualified,
    /// Qualified
    Qualified,
}

impl FormDefault {
    /// Parse from string value
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "qualified" => Some(Self::Qualified),
            "unqualified" => Some(Self::Unqualified),
            _ => None,
        }
    }

    /// Check if qualified
    pub fn is_qualified(&self) -> bool {
        matches!(self, Self::Qualified)
    }
}

impl fmt::Display for FormDefault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Qualified => write!(f, "qualified"),
            Self::Unqualified => write!(f, "unqualified"),
        }
    }
}

/// Derivation method flags for blockDefault and finalDefault
#[derive(Debug, Clone, Default)]
pub struct DerivationDefault {
    /// Block/final extension derivation
    pub extension: bool,
    /// Block/final restriction derivation
    pub restriction: bool,
    /// Block/final substitution (for blockDefault only)
    pub substitution: bool,
    /// Block/final list derivation (for types)
    pub list: bool,
    /// Block/final union derivation (for types)
    pub union: bool,
}

impl DerivationDefault {
    /// Create with all flags set
    pub fn all() -> Self {
        Self {
            extension: true,
            restriction: true,
            substitution: true,
            list: true,
            union: true,
        }
    }

    /// Parse from attribute value
    pub fn parse(value: &str) -> Self {
        if value == "#all" {
            return Self::all();
        }

        let mut result = Self::default();
        for token in value.split_whitespace() {
            match token {
                "extension" => result.extension = true,
                "restriction" => result.restriction = true,
                "substitution" => result.substitution = true,
                "list" => result.list = true,
                "union" => result.union = true,
                _ => {}
            }
        }
        result
    }

    /// Check if any flag is set
    pub fn is_empty(&self) -> bool {
        !self.extension && !self.restriction && !self.substitution && !self.list && !self.union
    }
}

/// Schema source information
#[derive(Debug, Clone)]
pub struct SchemaSource {
    /// URL or file path of the schema
    pub url: Option<String>,
    /// Base URL for resolving relative references
    pub base_url: Option<String>,
    /// Namespace declarations from the schema root
    pub namespaces: HashMap<String, String>,
}

impl Default for SchemaSource {
    fn default() -> Self {
        Self {
            url: None,
            base_url: None,
            namespaces: HashMap::new(),
        }
    }
}

/// Import record for a namespace
#[derive(Debug)]
pub struct SchemaImport {
    /// Namespace URI
    pub namespace: String,
    /// Location hint (schemaLocation)
    pub location: Option<String>,
    /// Imported schema (if loaded)
    pub schema: Option<Arc<XsdSchema>>,
}

/// Include record for a schema
#[derive(Debug)]
pub struct SchemaInclude {
    /// Location (schemaLocation)
    pub location: String,
    /// Included schema
    pub schema: Arc<XsdSchema>,
}

/// Main XML Schema validator
///
/// This is the central orchestrator for XSD parsing and validation.
/// It manages global declarations, namespace imports, and validation context.
#[derive(Debug)]
pub struct XsdSchema {
    /// XSD version (1.0 or 1.1)
    pub version: XsdVersion,
    /// Target namespace of the schema
    pub target_namespace: Option<String>,
    /// Validation mode
    pub validation: ValidationMode,
    /// Schema source information
    pub source: SchemaSource,
    /// Global declarations
    pub maps: XsdGlobals,
    /// Builder factory
    pub builders: XsdBuilders,
    /// Schema's attributeFormDefault
    pub attribute_form_default: FormDefault,
    /// Schema's elementFormDefault
    pub element_form_default: FormDefault,
    /// Schema's blockDefault
    pub block_default: DerivationDefault,
    /// Schema's finalDefault
    pub final_default: DerivationDefault,
    /// XSD 1.1: Default attributes group
    pub default_attributes: Option<String>,
    /// Imported namespaces
    pub imports: HashMap<String, SchemaImport>,
    /// Included schemas
    pub includes: Vec<SchemaInclude>,
    /// Parse errors
    pub errors: Vec<ParseError>,
    /// Whether the schema has been built
    built: bool,
}

impl Default for XsdSchema {
    fn default() -> Self {
        Self::new()
    }
}

impl XsdSchema {
    /// Create a new empty schema
    pub fn new() -> Self {
        Self {
            version: XsdVersion::default(),
            target_namespace: None,
            validation: ValidationMode::default(),
            source: SchemaSource::default(),
            maps: XsdGlobals::new(),
            builders: XsdBuilders::new(),
            attribute_form_default: FormDefault::default(),
            element_form_default: FormDefault::default(),
            block_default: DerivationDefault::default(),
            final_default: DerivationDefault::default(),
            default_attributes: None,
            imports: HashMap::new(),
            includes: Vec::new(),
            errors: Vec::new(),
            built: false,
        }
    }

    /// Create a schema with a specific version
    pub fn with_version(version: XsdVersion) -> Self {
        let mut schema = Self::new();
        schema.version = version;
        schema.builders = XsdBuilders::with_version(version);
        schema
    }

    /// Create a schema with a target namespace
    pub fn with_namespace(namespace: &str) -> Self {
        let mut schema = Self::new();
        schema.target_namespace = Some(namespace.to_string());
        schema.builders.target_namespace = Some(namespace.to_string());
        schema
    }

    /// Create a schema with version and namespace
    pub fn with_version_and_namespace(version: XsdVersion, namespace: &str) -> Self {
        let mut schema = Self::with_version(version);
        schema.target_namespace = Some(namespace.to_string());
        schema.builders.target_namespace = Some(namespace.to_string());
        schema
    }

    /// Set the target namespace
    pub fn set_target_namespace(&mut self, namespace: Option<String>) {
        self.target_namespace = namespace.clone();
        self.builders.target_namespace = namespace;
    }

    /// Set the validation mode
    pub fn set_validation_mode(&mut self, mode: ValidationMode) {
        self.validation = mode;
        self.builders.mode = mode;
    }

    /// Get the XSD version as a string
    pub fn xsd_version(&self) -> &'static str {
        match self.version {
            XsdVersion::Xsd10 => "1.0",
            XsdVersion::Xsd11 => "1.1",
        }
    }

    /// Check if this is XSD 1.1
    pub fn is_xsd11(&self) -> bool {
        self.version == XsdVersion::Xsd11
    }

    /// Get the URL of the schema source
    pub fn url(&self) -> Option<&str> {
        self.source.url.as_deref()
    }

    /// Get the base URL
    pub fn base_url(&self) -> Option<&str> {
        self.source.base_url.as_deref()
    }

    /// Record a parse error
    pub fn parse_error(&mut self, error: ParseError) {
        self.errors.push(error);
    }

    /// Register built-in types
    pub fn register_builtins(&mut self) -> Result<()> {
        self.builders.register_builtins(&mut self.maps)
    }

    /// Look up a global type by QName
    pub fn lookup_type(&self, qname: &QName) -> Option<&GlobalType> {
        self.maps.lookup_type(qname)
    }

    /// Look up a simple type by QName
    pub fn lookup_simple_type(&self, qname: &QName) -> Option<&Arc<dyn SimpleType + Send + Sync>> {
        self.maps.lookup_simple_type(qname)
    }

    /// Look up a global element by QName
    pub fn lookup_element(&self, qname: &QName) -> Option<&Arc<XsdElement>> {
        self.maps.lookup_element(qname)
    }

    /// Look up a global attribute by QName
    pub fn lookup_attribute(&self, qname: &QName) -> Option<&Arc<XsdAttribute>> {
        self.maps.lookup_attribute(qname)
    }

    /// Look up a global group by QName
    pub fn lookup_group(&self, qname: &QName) -> Option<&Arc<XsdGroup>> {
        self.maps.lookup_group(qname)
    }

    /// Look up an attribute group by QName
    pub fn lookup_attribute_group(&self, qname: &QName) -> Option<&Arc<XsdAttributeGroup>> {
        self.maps.lookup_attribute_group(qname)
    }

    /// Look up a notation by QName
    pub fn lookup_notation(&self, qname: &QName) -> Option<&XsdNotation> {
        self.maps.lookup_notation(qname)
    }

    /// Get the number of global elements
    pub fn element_count(&self) -> usize {
        self.maps.global_maps.elements.len()
    }

    /// Get the number of global types
    pub fn type_count(&self) -> usize {
        self.maps.global_maps.types.len()
    }

    /// Iterate over global element names
    pub fn element_names(&self) -> impl Iterator<Item = &QName> {
        self.maps.global_maps.elements.keys()
    }

    /// Iterate over global type names
    pub fn type_names(&self) -> impl Iterator<Item = &QName> {
        self.maps.global_maps.types.keys()
    }

    /// Iterate over global elements
    pub fn elements(&self) -> impl Iterator<Item = (&QName, &Arc<XsdElement>)> {
        self.maps.global_maps.elements.iter()
    }

    /// Iterate over global types
    pub fn types(&self) -> impl Iterator<Item = (&QName, &GlobalType)> {
        self.maps.global_maps.types.iter()
    }

    /// Iterate over global attributes
    pub fn attributes(&self) -> impl Iterator<Item = (&QName, &Arc<XsdAttribute>)> {
        self.maps.global_maps.attributes.iter()
    }

    /// Iterate over global groups
    pub fn groups(&self) -> impl Iterator<Item = (&QName, &Arc<XsdGroup>)> {
        self.maps.global_maps.groups.iter()
    }

    /// Iterate over attribute groups
    pub fn attribute_groups(&self) -> impl Iterator<Item = (&QName, &Arc<XsdAttributeGroup>)> {
        self.maps.global_maps.attribute_groups.iter()
    }

    /// Iterate over notations
    pub fn notations(&self) -> impl Iterator<Item = (&QName, &XsdNotation)> {
        self.maps.global_maps.notations.iter()
    }

    /// Resolve a QName string to a namespace and local name
    pub fn resolve_qname<'a>(&'a self, qname: &'a str) -> (Option<&'a str>, &'a str) {
        if let Some(colon_pos) = qname.find(':') {
            let prefix = &qname[..colon_pos];
            let local_name = &qname[colon_pos + 1..];
            let namespace = self.source.namespaces.get(prefix).map(|s| s.as_str());
            (namespace, local_name)
        } else {
            (self.target_namespace.as_deref(), qname)
        }
    }

    /// Create a QName with the target namespace prefix
    pub fn create_qname(&self, local_name: &str) -> String {
        if let Some(ns) = &self.target_namespace {
            // Find prefix for namespace
            for (prefix, namespace) in &self.source.namespaces {
                if namespace == ns && !prefix.is_empty() {
                    return format!("{}:{}", prefix, local_name);
                }
            }
        }
        local_name.to_string()
    }

    /// Resolve element form for a local element
    pub fn resolve_element_form(&self, explicit_form: Option<FormDefault>) -> FormDefault {
        explicit_form.unwrap_or(self.element_form_default)
    }

    /// Resolve attribute form for a local attribute
    pub fn resolve_attribute_form(&self, explicit_form: Option<FormDefault>) -> FormDefault {
        explicit_form.unwrap_or(self.attribute_form_default)
    }

    /// Add a namespace declaration
    pub fn add_namespace(&mut self, prefix: &str, namespace: &str) {
        self.source.namespaces.insert(prefix.to_string(), namespace.to_string());
    }

    /// Get namespace for a prefix
    pub fn get_namespace(&self, prefix: &str) -> Option<&str> {
        self.source.namespaces.get(prefix).map(|s| s.as_str())
    }

    /// Check if a namespace is imported
    pub fn has_import(&self, namespace: &str) -> bool {
        self.imports.contains_key(namespace)
    }

    /// Get an imported schema
    pub fn get_import(&self, namespace: &str) -> Option<&SchemaImport> {
        self.imports.get(namespace)
    }

    /// Add an import record
    pub fn add_import(&mut self, namespace: String, location: Option<String>) {
        self.imports.insert(namespace.clone(), SchemaImport {
            namespace,
            location,
            schema: None,
        });
    }
}

impl Validator for XsdSchema {
    fn is_built(&self) -> bool {
        self.built
    }

    fn build(&mut self) -> Result<()> {
        if self.built {
            return Ok(());
        }

        // Register built-in types if not already done
        if self.maps.global_maps.types.is_empty() {
            self.register_builtins()?;
        }

        // Mark as built
        self.built = true;
        Ok(())
    }

    fn validation_attempted(&self) -> ValidationStatus {
        if self.built {
            ValidationStatus::Full
        } else {
            ValidationStatus::None
        }
    }

    fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    fn errors(&self) -> Vec<ParseError> {
        self.errors.clone()
    }
}

/// A namespace view provides access to global declarations within a specific namespace
#[derive(Debug)]
pub struct NamespaceView<'a> {
    schema: &'a XsdSchema,
    namespace: Option<&'a str>,
}

impl<'a> NamespaceView<'a> {
    /// Create a new namespace view
    pub fn new(schema: &'a XsdSchema, namespace: Option<&'a str>) -> Self {
        Self { schema, namespace }
    }

    /// Check if a QName matches this namespace
    fn matches_namespace(&self, qname: &QName) -> bool {
        qname.namespace.as_ref().map(|n| n.as_str()) == self.namespace
    }

    /// Get elements in this namespace
    pub fn elements(&self) -> impl Iterator<Item = (&QName, &Arc<XsdElement>)> + '_ {
        self.schema.elements().filter(|(name, _)| self.matches_namespace(name))
    }

    /// Get types in this namespace
    pub fn types(&self) -> impl Iterator<Item = (&QName, &GlobalType)> + '_ {
        self.schema.types().filter(|(name, _)| self.matches_namespace(name))
    }

    /// Get attributes in this namespace
    pub fn attributes(&self) -> impl Iterator<Item = (&QName, &Arc<XsdAttribute>)> + '_ {
        self.schema.attributes().filter(|(name, _)| self.matches_namespace(name))
    }

    /// Get groups in this namespace
    pub fn groups(&self) -> impl Iterator<Item = (&QName, &Arc<XsdGroup>)> + '_ {
        self.schema.groups().filter(|(name, _)| self.matches_namespace(name))
    }

    /// Get attribute groups in this namespace
    pub fn attribute_groups(&self) -> impl Iterator<Item = (&QName, &Arc<XsdAttributeGroup>)> + '_ {
        self.schema.attribute_groups().filter(|(name, _)| self.matches_namespace(name))
    }
}

/// Schema validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation succeeded
    pub valid: bool,
    /// Validation errors
    pub errors: Vec<String>,
    /// Validation warnings
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Create a valid result
    pub fn valid() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Create an invalid result with errors
    pub fn invalid(errors: Vec<String>) -> Self {
        Self {
            valid: false,
            errors,
            warnings: Vec::new(),
        }
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    /// Add an error
    pub fn add_error(&mut self, error: impl Into<String>) {
        self.valid = false;
        self.errors.push(error.into());
    }
}

/// Schema collection for managing multiple schemas
///
/// This is useful for handling imports and includes across namespaces.
#[derive(Debug, Default)]
pub struct SchemaCollection {
    /// Schemas indexed by target namespace
    pub schemas: HashMap<Option<String>, Vec<Arc<XsdSchema>>>,
    /// Primary schema (first loaded)
    pub primary: Option<Arc<XsdSchema>>,
}

impl SchemaCollection {
    /// Create a new empty collection
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a schema to the collection
    pub fn add(&mut self, schema: Arc<XsdSchema>) {
        let namespace = schema.target_namespace.clone();
        if self.primary.is_none() {
            self.primary = Some(Arc::clone(&schema));
        }
        self.schemas.entry(namespace).or_default().push(schema);
    }

    /// Get schemas for a namespace
    pub fn get(&self, namespace: Option<&str>) -> Option<&Vec<Arc<XsdSchema>>> {
        self.schemas.get(&namespace.map(|s| s.to_string()))
    }

    /// Get the primary schema
    pub fn primary(&self) -> Option<&Arc<XsdSchema>> {
        self.primary.as_ref()
    }

    /// Check if a namespace is loaded
    pub fn has_namespace(&self, namespace: Option<&str>) -> bool {
        self.schemas.contains_key(&namespace.map(|s| s.to_string()))
    }

    /// Get all loaded namespaces
    pub fn namespaces(&self) -> impl Iterator<Item = Option<&String>> {
        self.schemas.keys().map(|k| k.as_ref())
    }

    /// Get count of schemas
    pub fn len(&self) -> usize {
        self.schemas.values().map(|v| v.len()).sum()
    }

    /// Check if collection is empty
    pub fn is_empty(&self) -> bool {
        self.schemas.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_creation() {
        let schema = XsdSchema::new();
        assert_eq!(schema.version, XsdVersion::Xsd10);
        assert!(schema.target_namespace.is_none());
        assert!(!schema.is_built());
    }

    #[test]
    fn test_schema_with_version() {
        let schema = XsdSchema::with_version(XsdVersion::Xsd11);
        assert_eq!(schema.version, XsdVersion::Xsd11);
        assert!(schema.is_xsd11());
        assert_eq!(schema.xsd_version(), "1.1");
    }

    #[test]
    fn test_schema_with_namespace() {
        let schema = XsdSchema::with_namespace("http://example.com/test");
        assert_eq!(schema.target_namespace.as_deref(), Some("http://example.com/test"));
    }

    #[test]
    fn test_schema_with_version_and_namespace() {
        let schema = XsdSchema::with_version_and_namespace(
            XsdVersion::Xsd11,
            "http://example.com/test",
        );
        assert_eq!(schema.version, XsdVersion::Xsd11);
        assert_eq!(schema.target_namespace.as_deref(), Some("http://example.com/test"));
    }

    #[test]
    fn test_form_default_parse() {
        assert_eq!(FormDefault::from_str("qualified"), Some(FormDefault::Qualified));
        assert_eq!(FormDefault::from_str("unqualified"), Some(FormDefault::Unqualified));
        assert_eq!(FormDefault::from_str("invalid"), None);
    }

    #[test]
    fn test_form_default_is_qualified() {
        assert!(FormDefault::Qualified.is_qualified());
        assert!(!FormDefault::Unqualified.is_qualified());
    }

    #[test]
    fn test_derivation_default_parse() {
        let dd = DerivationDefault::parse("#all");
        assert!(dd.extension);
        assert!(dd.restriction);
        assert!(dd.substitution);

        let dd = DerivationDefault::parse("extension restriction");
        assert!(dd.extension);
        assert!(dd.restriction);
        assert!(!dd.substitution);

        let dd = DerivationDefault::parse("");
        assert!(dd.is_empty());
    }

    #[test]
    fn test_schema_build() {
        let mut schema = XsdSchema::new();
        assert!(!schema.is_built());
        assert!(schema.build().is_ok());
        assert!(schema.is_built());
    }

    #[test]
    fn test_schema_namespaces() {
        let mut schema = XsdSchema::new();
        schema.add_namespace("xs", XSD_NAMESPACE);
        schema.add_namespace("tns", "http://example.com/test");

        assert_eq!(schema.get_namespace("xs"), Some(XSD_NAMESPACE));
        assert_eq!(schema.get_namespace("tns"), Some("http://example.com/test"));
        assert_eq!(schema.get_namespace("unknown"), None);
    }

    #[test]
    fn test_resolve_qname() {
        let mut schema = XsdSchema::new();
        schema.target_namespace = Some("http://example.com/default".to_string());
        schema.add_namespace("tns", "http://example.com/test");
        schema.add_namespace("xs", XSD_NAMESPACE);

        // Prefixed QName
        let (ns, local) = schema.resolve_qname("tns:element");
        assert_eq!(ns, Some("http://example.com/test"));
        assert_eq!(local, "element");

        // Unprefixed QName uses target namespace
        let (ns, local) = schema.resolve_qname("element");
        assert_eq!(ns, Some("http://example.com/default"));
        assert_eq!(local, "element");
    }

    #[test]
    fn test_schema_imports() {
        let mut schema = XsdSchema::new();

        schema.add_import(
            "http://example.com/imported".to_string(),
            Some("imported.xsd".to_string()),
        );

        assert!(schema.has_import("http://example.com/imported"));
        assert!(!schema.has_import("http://example.com/unknown"));

        let import = schema.get_import("http://example.com/imported").unwrap();
        assert_eq!(import.namespace, "http://example.com/imported");
        assert_eq!(import.location.as_deref(), Some("imported.xsd"));
    }

    #[test]
    fn test_resolve_form() {
        let mut schema = XsdSchema::new();
        schema.element_form_default = FormDefault::Qualified;
        schema.attribute_form_default = FormDefault::Unqualified;

        // Default forms
        assert!(schema.resolve_element_form(None).is_qualified());
        assert!(!schema.resolve_attribute_form(None).is_qualified());

        // Explicit forms override defaults
        assert!(!schema.resolve_element_form(Some(FormDefault::Unqualified)).is_qualified());
        assert!(schema.resolve_attribute_form(Some(FormDefault::Qualified)).is_qualified());
    }

    #[test]
    fn test_validation_result() {
        let valid = ValidationResult::valid();
        assert!(valid.valid);
        assert!(valid.errors.is_empty());

        let invalid = ValidationResult::invalid(vec!["test error".to_string()]);
        assert!(!invalid.valid);
        assert_eq!(invalid.errors.len(), 1);
    }

    #[test]
    fn test_validation_result_modifications() {
        let mut result = ValidationResult::valid();
        assert!(result.valid);

        result.add_warning("Warning!".to_string());
        assert!(result.valid);
        assert_eq!(result.warnings.len(), 1);

        result.add_error("Error!");
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_schema_collection() {
        let mut collection = SchemaCollection::new();
        assert!(collection.is_empty());
        assert_eq!(collection.len(), 0);

        let schema1 = Arc::new(XsdSchema::with_namespace("http://example.com/a"));
        let schema2 = Arc::new(XsdSchema::with_namespace("http://example.com/b"));
        let schema3 = Arc::new(XsdSchema::with_namespace("http://example.com/a"));

        collection.add(Arc::clone(&schema1));
        assert_eq!(collection.len(), 1);
        assert!(collection.primary().is_some());

        collection.add(Arc::clone(&schema2));
        collection.add(Arc::clone(&schema3));
        assert_eq!(collection.len(), 3);

        assert!(collection.has_namespace(Some("http://example.com/a")));
        assert!(collection.has_namespace(Some("http://example.com/b")));
        assert!(!collection.has_namespace(Some("http://example.com/c")));

        let schemas_a = collection.get(Some("http://example.com/a")).unwrap();
        assert_eq!(schemas_a.len(), 2);
    }

    #[test]
    fn test_namespace_view() {
        let mut schema = XsdSchema::new();
        schema.set_target_namespace(Some("http://example.com/test".to_string()));
        schema.add_namespace("tns", "http://example.com/test");

        let view = NamespaceView::new(&schema, Some("http://example.com/test"));
        // View should filter correctly (empty since no elements registered)
        assert_eq!(view.elements().count(), 0);
    }

    #[test]
    fn test_validator_trait() {
        let schema = XsdSchema::new();
        assert_eq!(schema.validation, ValidationMode::Strict);
        assert!(!schema.has_errors());
        assert!(schema.errors().is_empty());
        assert_eq!(schema.validation_attempted(), ValidationStatus::None);
    }

    #[test]
    fn test_validation_status() {
        let schema = XsdSchema::new();
        assert_eq!(schema.validation_attempted(), ValidationStatus::None);
        assert_eq!(schema.validity(ValidationMode::Strict), ValidityStatus::NotKnown);

        let mut schema = XsdSchema::new();
        schema.build().unwrap();
        assert_eq!(schema.validation_attempted(), ValidationStatus::Full);
        assert_eq!(schema.validity(ValidationMode::Strict), ValidityStatus::Valid);

        let mut schema = XsdSchema::new();
        schema.parse_error(ParseError::new("test error"));
        assert_eq!(schema.validity(ValidationMode::Strict), ValidityStatus::Invalid);
    }

    #[test]
    fn test_schema_source() {
        let source = SchemaSource {
            url: Some("http://example.com/schema.xsd".to_string()),
            base_url: Some("http://example.com/".to_string()),
            namespaces: HashMap::new(),
        };

        assert_eq!(source.url.as_deref(), Some("http://example.com/schema.xsd"));
        assert_eq!(source.base_url.as_deref(), Some("http://example.com/"));
    }

    #[test]
    fn test_schema_accessors() {
        let mut schema = XsdSchema::new();
        schema.source.url = Some("http://example.com/schema.xsd".to_string());
        schema.source.base_url = Some("http://example.com/".to_string());

        assert_eq!(schema.url(), Some("http://example.com/schema.xsd"));
        assert_eq!(schema.base_url(), Some("http://example.com/"));
    }

    #[test]
    fn test_create_qname() {
        let mut schema = XsdSchema::new();
        schema.set_target_namespace(Some("http://example.com/test".to_string()));
        schema.add_namespace("tns", "http://example.com/test");

        let qname = schema.create_qname("element");
        assert_eq!(qname, "tns:element");

        // Without matching namespace prefix
        let mut schema2 = XsdSchema::new();
        schema2.set_target_namespace(Some("http://example.com/test".to_string()));
        let qname2 = schema2.create_qname("element");
        assert_eq!(qname2, "element");
    }

    #[test]
    fn test_empty_counts() {
        let schema = XsdSchema::new();
        assert_eq!(schema.element_count(), 0);
        assert_eq!(schema.type_count(), 0);
        assert_eq!(schema.element_names().count(), 0);
        assert_eq!(schema.type_names().count(), 0);
    }
}
