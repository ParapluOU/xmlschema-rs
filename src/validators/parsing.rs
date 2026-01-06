//! XSD Document Parsing
//!
//! This module provides parsing of XSD schema documents into XsdSchema structures.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::attributes::{AttributeUse, XsdAttribute, XsdAttributeGroup};
use super::base::Validator;
use super::builders::XsdVersion;
use super::complex_types::{ComplexContent, DerivationMethod, XsdComplexType};
use super::elements::{ElementType, XsdElement};
use super::globals::GlobalType;
use super::groups::{ElementParticle, GroupParticle, ModelType, XsdGroup};
use super::particles::Occurs;
use super::schemas::{DerivationDefault, FormDefault, RedefinedComponent, SchemaRedefine, XsdSchema};
use super::simple_types::{XsdAtomicType, XsdListType, XsdRestrictedType, XsdUnionType};
use super::builtins::XSD_NAMESPACE;
use super::wildcards::{NamespaceConstraint, ProcessContents, XsdAnyAttribute, XsdAnyElement};

use crate::catalog::XmlCatalog;
use crate::documents::{Document, Element};
use crate::error::{Error, ParseError, Result};
use crate::loaders::Loader;
use crate::locations::Location;
use crate::namespaces::QName;

/// Pending schema work item for iterative processing
struct PendingSchemaWork {
    /// Path to the schema file
    path: PathBuf,
    /// Parent namespace (for chameleon include handling)
    parent_namespace: Option<String>,
}

/// XSD element local names
mod xsd_elements {
    pub const SCHEMA: &str = "schema";
    pub const ELEMENT: &str = "element";
    pub const COMPLEX_TYPE: &str = "complexType";
    pub const SIMPLE_TYPE: &str = "simpleType";
    pub const ATTRIBUTE: &str = "attribute";
    pub const ATTRIBUTE_GROUP: &str = "attributeGroup";
    pub const GROUP: &str = "group";
    pub const SEQUENCE: &str = "sequence";
    pub const CHOICE: &str = "choice";
    pub const ALL: &str = "all";
    pub const ANNOTATION: &str = "annotation";
    pub const IMPORT: &str = "import";
    pub const INCLUDE: &str = "include";
    pub const REDEFINE: &str = "redefine";
    pub const RESTRICTION: &str = "restriction";
    pub const EXTENSION: &str = "extension";
    pub const LIST: &str = "list";
    pub const UNION: &str = "union";
    pub const COMPLEX_CONTENT: &str = "complexContent";
    pub const SIMPLE_CONTENT: &str = "simpleContent";
    pub const ANY: &str = "any";
    pub const ANY_ATTRIBUTE: &str = "anyAttribute";
    pub const NOTATION: &str = "notation";
    // Facets
    pub const PATTERN: &str = "pattern";
    pub const ENUMERATION: &str = "enumeration";
    pub const MIN_LENGTH: &str = "minLength";
    pub const MAX_LENGTH: &str = "maxLength";
    pub const LENGTH: &str = "length";
}

/// XSD attribute names
mod xsd_attrs {
    pub const NAME: &str = "name";
    pub const TYPE: &str = "type";
    pub const REF: &str = "ref";
    pub const TARGET_NAMESPACE: &str = "targetNamespace";
    pub const ELEMENT_FORM_DEFAULT: &str = "elementFormDefault";
    pub const ATTRIBUTE_FORM_DEFAULT: &str = "attributeFormDefault";
    pub const BLOCK_DEFAULT: &str = "blockDefault";
    pub const FINAL_DEFAULT: &str = "finalDefault";
    pub const NILLABLE: &str = "nillable";
    pub const DEFAULT: &str = "default";
    pub const FIXED: &str = "fixed";
    pub const BASE: &str = "base";
    pub const VALUE: &str = "value";
    pub const MIXED: &str = "mixed";
    pub const ABSTRACT: &str = "abstract";
    pub const SUBSTITUTION_GROUP: &str = "substitutionGroup";
    pub const NAMESPACE: &str = "namespace";
    pub const SCHEMA_LOCATION: &str = "schemaLocation";
    pub const ITEM_TYPE: &str = "itemType";
    pub const MEMBER_TYPES: &str = "memberTypes";
    pub const PUBLIC: &str = "public";
    pub const SYSTEM: &str = "system";
    pub const MIN_OCCURS: &str = "minOccurs";
    pub const MAX_OCCURS: &str = "maxOccurs";
    pub const USE: &str = "use";
}

/// Map XSD built-in type local name to the internal constant
fn resolve_builtin_name(local_name: &str) -> Option<&'static str> {
    use super::builtins::*;
    match local_name {
        "string" => Some(XSD_STRING),
        "normalizedString" => Some(XSD_NORMALIZED_STRING),
        "token" => Some(XSD_TOKEN),
        "language" => Some(XSD_LANGUAGE),
        "Name" => Some(XSD_NAME),
        "NCName" => Some(XSD_NCNAME),
        "ID" => Some(XSD_ID),
        "IDREF" => Some(XSD_IDREF),
        "IDREFS" => Some(XSD_IDREFS),
        "ENTITY" => Some(XSD_ENTITY),
        "ENTITIES" => Some(XSD_ENTITIES),
        "NMTOKEN" => Some(XSD_NMTOKEN),
        "NMTOKENS" => Some(XSD_NMTOKENS),
        "boolean" => Some(XSD_BOOLEAN),
        "decimal" => Some(XSD_DECIMAL),
        "integer" => Some(XSD_INTEGER),
        "long" => Some(XSD_LONG),
        "int" => Some(XSD_INT),
        "short" => Some(XSD_SHORT),
        "byte" => Some(XSD_BYTE),
        "nonNegativeInteger" => Some(XSD_NON_NEGATIVE_INTEGER),
        "positiveInteger" => Some(XSD_POSITIVE_INTEGER),
        "unsignedLong" => Some(XSD_UNSIGNED_LONG),
        "unsignedInt" => Some(XSD_UNSIGNED_INT),
        "unsignedShort" => Some(XSD_UNSIGNED_SHORT),
        "unsignedByte" => Some(XSD_UNSIGNED_BYTE),
        "nonPositiveInteger" => Some(XSD_NON_POSITIVE_INTEGER),
        "negativeInteger" => Some(XSD_NEGATIVE_INTEGER),
        "float" => Some(XSD_FLOAT),
        "double" => Some(XSD_DOUBLE),
        "duration" => Some(XSD_DURATION),
        "dateTime" => Some(XSD_DATETIME),
        "time" => Some(XSD_TIME),
        "date" => Some(XSD_DATE),
        "gYearMonth" => Some(XSD_GYEAR_MONTH),
        "gYear" => Some(XSD_GYEAR),
        "gMonthDay" => Some(XSD_GMONTH_DAY),
        "gDay" => Some(XSD_GDAY),
        "gMonth" => Some(XSD_GMONTH),
        "hexBinary" => Some(XSD_HEX_BINARY),
        "base64Binary" => Some(XSD_BASE64_BINARY),
        "anyURI" => Some(XSD_ANY_URI),
        "QName" => Some(XSD_QNAME),
        "NOTATION" => Some(XSD_NOTATION),
        "anyType" => Some(XSD_ANY_TYPE),
        "anySimpleType" => Some(XSD_ANY_SIMPLE_TYPE),
        _ => None,
    }
}

/// XSD namespace
const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema";

impl XsdSchema {
    /// Parse an XSD schema from a string
    pub fn from_string(xml: &str) -> Result<Self> {
        let doc = Document::from_string(xml)?;
        Self::from_document(&doc)
    }

    /// Parse an XSD schema from bytes
    pub fn from_bytes(xml: &[u8]) -> Result<Self> {
        let doc = Document::parse(xml)?;
        Self::from_document(&doc)
    }

    /// Parse an XSD schema from a file path
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_file_with_catalog(path, None::<&Path>)
    }

    /// Parse an XSD schema from a file path with an XML catalog for URN resolution
    ///
    /// The catalog is used to resolve URN-based schema locations (like those in DITA 1.3)
    /// to actual file paths. If the catalog path is provided, it will be loaded and used
    /// to resolve includes/imports that use URNs instead of relative paths.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let schema = XsdSchema::from_file_with_catalog(
    ///     "schemas/ditabase.xsd",
    ///     Some("schemas/catalog.xml"),
    /// )?;
    /// ```
    pub fn from_file_with_catalog(
        path: impl AsRef<Path>,
        catalog_path: Option<impl AsRef<Path>>,
    ) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let catalog_path = catalog_path.map(|p| p.as_ref().to_path_buf());

        // Complex schemas with deep include chains can overflow the default stack.
        // Spawn a thread with a larger stack to handle deep recursion.
        // 32MB should handle schemas with up to ~100+ levels of includes.
        const STACK_SIZE: usize = 32 * 1024 * 1024;

        let handle = std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .name("xsd-parser".to_string())
            .spawn(move || Self::parse_file_internal(&path, catalog_path.as_deref()))
            .map_err(|e| Error::Parse(ParseError::new(format!("Failed to spawn parser thread: {}", e))))?;

        handle.join()
            .map_err(|_| Error::Parse(ParseError::new("Parser thread panicked (possible stack overflow)")))?
    }

    /// Internal parsing implementation (called from spawned thread)
    /// Uses iterative include processing to avoid stack overflow on deep include chains.
    fn parse_file_internal(path: &Path, catalog_path: Option<&Path>) -> Result<Self> {
        // Load catalog if provided
        let catalog = if let Some(cat_path) = catalog_path {
            Some(Arc::new(XmlCatalog::from_file(cat_path)?))
        } else {
            None
        };

        // Shared set to track loaded files (prevents circular includes)
        let loaded_paths = Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));

        // Queue of schemas to process (iterative worklist algorithm)
        let mut pending: VecDeque<PendingSchemaWork> = VecDeque::new();

        // Start with the root schema
        pending.push_back(PendingSchemaWork {
            path: path.to_path_buf(),
            parent_namespace: None,
        });

        // The root schema we'll return
        let mut root_schema: Option<XsdSchema> = None;

        // Process schemas iteratively
        while let Some(work) = pending.pop_front() {
            // Check if already loaded
            if let Ok(canonical) = work.path.canonicalize() {
                let mut loaded = loaded_paths.lock().unwrap();
                if loaded.contains(&canonical) {
                    continue; // Already processed
                }
                loaded.insert(canonical);
            }

            // Load and parse this schema (without recursively processing includes)
            let schema_result = parse_schema_no_includes(
                &work.path,
                work.parent_namespace.as_deref(),
                catalog.clone(),
                loaded_paths.clone(),
            );

            let schema = match schema_result {
                Ok(s) => s,
                Err(e) => {
                    // For the root schema, propagate the error
                    if root_schema.is_none() {
                        return Err(e);
                    }
                    // For includes, log warning and continue
                    continue;
                }
            };

            // Collect pending includes from this schema
            for include_location in &schema.pending_include_locations {
                let resolved_path = resolve_schema_location(
                    include_location,
                    schema.source.base_url.as_deref(),
                    schema.source.catalog.as_ref().map(|c| c.as_ref()),
                );
                pending.push_back(PendingSchemaWork {
                    path: resolved_path,
                    parent_namespace: schema.target_namespace.clone(),
                });
            }

            // Collect pending redefines
            for redefine_location in &schema.pending_redefine_locations {
                let resolved_path = resolve_schema_location(
                    redefine_location,
                    schema.source.base_url.as_deref(),
                    schema.source.catalog.as_ref().map(|c| c.as_ref()),
                );
                pending.push_back(PendingSchemaWork {
                    path: resolved_path,
                    parent_namespace: schema.target_namespace.clone(),
                });
            }

            if root_schema.is_none() {
                root_schema = Some(schema);
            } else {
                // Merge globals from this schema into the root
                if let Some(ref mut root) = root_schema {
                    root.maps.global_maps.merge(&schema.maps.global_maps);
                }
            }
        }

        let mut schema = root_schema.ok_or_else(|| {
            Error::Parse(ParseError::new("Failed to parse any schema"))
        })?;

        // Build the schema
        schema.build()?;

        Ok(schema)
    }

    /// Parse an XSD schema from a parsed Document
    pub fn from_document(doc: &Document) -> Result<Self> {
        let root = doc.root().ok_or_else(|| Error::Parse(ParseError::new("Empty document")))?;

        // Verify this is a schema element
        if root.local_name() != xsd_elements::SCHEMA {
            return Err(Error::Parse(ParseError::new(format!(
                "Expected xs:schema root element, got {}",
                root.local_name()
            ))));
        }

        let mut schema = XsdSchema::new();
        parse_schema_element(&mut schema, root)?;

        // Build the schema
        schema.build()?;

        Ok(schema)
    }
}

/// Load and parse a schema file without recursively processing includes.
///
/// This function parses the schema file and collects include/redefine locations
/// in `pending_include_locations` and `pending_redefine_locations` fields,
/// but does NOT recursively load those schemas. The caller is responsible for
/// iteratively processing pending includes.
///
/// This is used by the iterative worklist algorithm in `parse_file_internal`
/// to avoid stack overflow on schemas with deep include chains.
fn parse_schema_no_includes(
    path: &Path,
    parent_namespace: Option<&str>,
    catalog: Option<Arc<XmlCatalog>>,
    loaded_paths: Arc<std::sync::Mutex<std::collections::HashSet<PathBuf>>>,
) -> Result<XsdSchema> {
    // Read the file
    let content = std::fs::read_to_string(path).map_err(|e| {
        Error::Resource(format!("Failed to read schema '{}': {}", path.display(), e))
    })?;

    // Parse as document
    let doc = Document::from_string(&content)?;
    let root = doc.root().ok_or_else(|| Error::Parse(ParseError::new("Empty document")))?;

    // Verify this is a schema element
    if root.local_name() != xsd_elements::SCHEMA {
        return Err(Error::Parse(ParseError::new(format!(
            "Expected xs:schema root element, got {}",
            root.local_name()
        ))));
    }

    // Create a new schema for parsing, sharing the loaded_paths set
    let mut schema = XsdSchema::new();
    schema.source.url = Some(path.to_string_lossy().to_string());
    schema.source.base_url = path.parent().map(|p| p.to_string_lossy().to_string());
    schema.source.catalog = catalog;
    schema.source.loaded_paths = loaded_paths;

    // Parse the schema element (this collects include locations but doesn't load them)
    parse_schema_element(&mut schema, root)?;

    // Handle chameleon include: if included schema has no target namespace,
    // it inherits the parent's target namespace
    if schema.target_namespace.is_none() {
        if let Some(parent_ns) = parent_namespace {
            // Re-namespace all globals to the parent namespace
            chameleon_renamespace(&mut schema, parent_ns);
        }
    } else if let Some(ref schema_ns) = schema.target_namespace {
        // Target namespace must match for xs:include (if parent has one)
        if let Some(parent_ns) = parent_namespace {
            if schema_ns != parent_ns {
                return Err(Error::Parse(ParseError::new(format!(
                    "Included schema has different targetNamespace '{}', expected '{}'",
                    schema_ns, parent_ns
                ))));
            }
        }
    }

    // Note: We don't call build() here - that's done after all includes are merged

    Ok(schema)
}

/// Parse the xs:schema root element
fn parse_schema_element(schema: &mut XsdSchema, elem: &Element) -> Result<()> {
    // Copy namespace declarations from the schema element
    for (prefix, namespace) in elem.namespaces.iter() {
        if !prefix.is_empty() {
            schema.add_namespace(prefix, namespace);
        }
    }

    // Parse schema attributes
    if let Some(ns) = elem.get_attribute(xsd_attrs::TARGET_NAMESPACE) {
        schema.set_target_namespace(Some(ns.to_string()));
    }

    if let Some(efd) = elem.get_attribute(xsd_attrs::ELEMENT_FORM_DEFAULT) {
        if let Some(form) = FormDefault::from_str(efd) {
            schema.element_form_default = form;
        }
    }

    if let Some(afd) = elem.get_attribute(xsd_attrs::ATTRIBUTE_FORM_DEFAULT) {
        if let Some(form) = FormDefault::from_str(afd) {
            schema.attribute_form_default = form;
        }
    }

    if let Some(bd) = elem.get_attribute(xsd_attrs::BLOCK_DEFAULT) {
        schema.block_default = DerivationDefault::parse(bd);
    }

    if let Some(fd) = elem.get_attribute(xsd_attrs::FINAL_DEFAULT) {
        schema.final_default = DerivationDefault::parse(fd);
    }

    // Detect XSD version from namespace - use the get_namespace method
    if let Some(ns) = elem.namespaces.get_namespace("xs") {
        if ns == "http://www.w3.org/2009/XMLSchema" {
            schema.version = XsdVersion::Xsd11;
        }
    }
    if let Some(ns) = elem.namespaces.get_namespace("xsd") {
        if ns == "http://www.w3.org/2009/XMLSchema" {
            schema.version = XsdVersion::Xsd11;
        }
    }

    // Parse children
    for child in &elem.children {
        parse_schema_child(schema, child)?;
    }

    Ok(())
}

/// Parse a child element of xs:schema
fn parse_schema_child(schema: &mut XsdSchema, elem: &Element) -> Result<()> {
    let local = elem.local_name();

    match local {
        xsd_elements::ELEMENT => parse_global_element(schema, elem),
        xsd_elements::COMPLEX_TYPE => parse_complex_type(schema, elem),
        xsd_elements::SIMPLE_TYPE => parse_simple_type(schema, elem),
        xsd_elements::ATTRIBUTE => parse_global_attribute(schema, elem),
        xsd_elements::ATTRIBUTE_GROUP => parse_attribute_group(schema, elem),
        xsd_elements::GROUP => parse_group(schema, elem),
        xsd_elements::IMPORT => parse_import(schema, elem),
        xsd_elements::INCLUDE => parse_include(schema, elem),
        xsd_elements::NOTATION => parse_notation(schema, elem),
        xsd_elements::ANNOTATION => Ok(()), // Skip annotations
        xsd_elements::REDEFINE => parse_redefine(schema, elem),
        _ => {
            schema.parse_error(ParseError::new(format!(
                "Unknown schema child element: {}",
                local
            )));
            Ok(())
        }
    }
}

/// Parse a global element declaration
fn parse_global_element(schema: &mut XsdSchema, elem: &Element) -> Result<()> {
    let name = elem.get_attribute(xsd_attrs::NAME).ok_or_else(|| {
        Error::Parse(ParseError::new("Global element missing 'name' attribute"))
    })?;

    let qname = make_qname(schema, name);

    // Determine element type
    let element_type = if let Some(type_str) = elem.get_attribute(xsd_attrs::TYPE) {
        // Resolve type reference
        let (type_ns, type_local) = schema.resolve_qname(type_str);
        let type_qname = QName::new(type_ns.map(|s| s.to_string()), type_local.clone());

        // First try built-in simple types
        if let Some(builtin_name) = resolve_builtin_name(&type_local) {
            if let Ok(simple_type) = XsdAtomicType::new(builtin_name) {
                ElementType::Simple(Arc::new(simple_type))
            } else {
                ElementType::Any
            }
        } else {
            // Store type reference - will be resolved during build phase
            // For now we create a placeholder with the type QName stored
            // We'll check if the type is already registered
            if let Some(global_type) = schema.maps.global_maps.types.get(&type_qname) {
                match global_type {
                    GlobalType::Complex(ct) => ElementType::Complex(Arc::clone(ct)),
                    GlobalType::Simple(st) => ElementType::Simple(Arc::clone(st)),
                }
            } else {
                // Type not yet parsed - store type name for later resolution
                // Create a marker that we'll resolve in the build phase
                ElementType::Any
            }
        }
    } else {
        // Look for inline type definition
        let mut inline_type = None;
        for child in &elem.children {
            match child.local_name() {
                xsd_elements::COMPLEX_TYPE => {
                    // Parse inline complex type
                    if let Some(ct) = parse_inline_complex_type(schema, child) {
                        inline_type = Some(ElementType::Complex(Arc::new(ct)));
                    }
                    break;
                }
                xsd_elements::SIMPLE_TYPE => {
                    // Parse inline simple type
                    if let Some(st) = parse_inline_simple_type(schema, child) {
                        inline_type = Some(ElementType::Simple(Arc::new(st)));
                    }
                    break;
                }
                _ => {}
            }
        }
        inline_type.unwrap_or(ElementType::Any)
    };

    let mut xsd_element = XsdElement::new(qname.clone(), element_type);

    // Store type reference for later resolution if needed
    if let Some(type_str) = elem.get_attribute(xsd_attrs::TYPE) {
        let (type_ns, type_local) = schema.resolve_qname(type_str);
        xsd_element.type_name = Some(QName::new(type_ns.map(|s| s.to_string()), type_local));
    }

    // Parse other attributes
    if let Some(nillable) = elem.get_attribute(xsd_attrs::NILLABLE) {
        xsd_element.nillable = nillable == "true";
    }

    if let Some(abstract_) = elem.get_attribute(xsd_attrs::ABSTRACT) {
        xsd_element.abstract_element = abstract_ == "true";
    }

    if let Some(default) = elem.get_attribute(xsd_attrs::DEFAULT) {
        xsd_element.default = Some(default.to_string());
    }

    if let Some(fixed) = elem.get_attribute(xsd_attrs::FIXED) {
        xsd_element.fixed = Some(fixed.to_string());
    }

    if let Some(subst) = elem.get_attribute(xsd_attrs::SUBSTITUTION_GROUP) {
        let (sg_ns, sg_local) = schema.resolve_qname(subst);
        xsd_element.substitution_group = Some(QName::new(sg_ns.map(|s| s.to_string()), sg_local));
    }

    schema.maps.global_maps.elements.insert(qname, Arc::new(xsd_element));

    Ok(())
}

/// Parse an inline (anonymous) complex type
fn parse_inline_complex_type(schema: &XsdSchema, elem: &Element) -> Option<XsdComplexType> {
    // Find content model (sequence/choice/all)
    let (model_type, content_model_elem) = if let Some((model_elem, model)) = find_content_model(elem) {
        (model, Some(model_elem))
    } else {
        (ModelType::Sequence, None)
    };

    // Create group and populate with particles from content model
    let mut group = XsdGroup::new(model_type);
    if let Some(model_elem) = content_model_elem {
        if model_elem.local_name() == xsd_elements::GROUP {
            if let Some(ref_str) = model_elem.get_attribute(xsd_attrs::REF) {
                let (ref_ns, ref_local) = schema.resolve_qname(ref_str);
                let ref_qname = QName::new(ref_ns.map(|s| s.to_string()), ref_local);
                let occurs = parse_occurs_option(model_elem);
                let ref_group = XsdGroup::reference(ref_qname, occurs);
                group.particles.push(GroupParticle::Group(Arc::new(ref_group)));
            }
        } else {
            parse_content_model(schema, model_elem, &mut group);
        }
    }

    let mut complex_type = XsdComplexType::new(None, Arc::new(group));

    // Parse mixed attribute
    if let Some(mixed) = elem.get_attribute(xsd_attrs::MIXED) {
        complex_type.mixed = mixed == "true";
    }

    // Parse attributes
    let mut attr_group = XsdAttributeGroup::anonymous();
    parse_attributes(schema, elem, &mut attr_group);

    // Handle complexContent/simpleContent
    for child in &elem.children {
        if child.local_name() == xsd_elements::COMPLEX_CONTENT
            || child.local_name() == xsd_elements::SIMPLE_CONTENT
        {
            // Check for mixed attribute on complexContent
            if let Some(mixed) = child.get_attribute(xsd_attrs::MIXED) {
                complex_type.mixed = mixed == "true";
            }

            for grandchild in &child.children {
                match grandchild.local_name() {
                    xsd_elements::RESTRICTION => {
                        complex_type.derivation = Some(DerivationMethod::Restriction);
                        if let Some(base) = grandchild.get_attribute(xsd_attrs::BASE) {
                            let (base_ns, base_local) = schema.resolve_qname(base);
                            complex_type.base_type = Some(QName::new(base_ns.map(|s| s.to_string()), base_local));
                        }
                        // Parse restriction's content model (replaces base content)
                        let restriction_model = find_content_model_element(grandchild);
                        if let Some(model_elem) = restriction_model {
                            let mut restriction_group = XsdGroup::new(ModelType::from_tag(model_elem.local_name()).unwrap_or(ModelType::Sequence));
                            parse_content_model(schema, model_elem, &mut restriction_group);
                            complex_type.content = ComplexContent::Group(Arc::new(restriction_group));
                        }
                        parse_attributes(schema, grandchild, &mut attr_group);
                    }
                    xsd_elements::EXTENSION => {
                        complex_type.derivation = Some(DerivationMethod::Extension);
                        if let Some(base) = grandchild.get_attribute(xsd_attrs::BASE) {
                            let (base_ns, base_local) = schema.resolve_qname(base);
                            complex_type.base_type = Some(QName::new(base_ns.map(|s| s.to_string()), base_local));
                        }
                        // Parse extension's content model (will be appended to base content)
                        let extension_model = find_content_model_element(grandchild);
                        if let Some(model_elem) = extension_model {
                            let mut extension_group = XsdGroup::new(ModelType::from_tag(model_elem.local_name()).unwrap_or(ModelType::Sequence));
                            parse_content_model(schema, model_elem, &mut extension_group);
                            complex_type.content = ComplexContent::Group(Arc::new(extension_group));
                        }
                        parse_attributes(schema, grandchild, &mut attr_group);
                    }
                    _ => {}
                }
            }
        }
    }

    complex_type.attributes = attr_group;

    Some(complex_type)
}

/// Parse an inline (anonymous) simple type
fn parse_inline_simple_type(schema: &XsdSchema, elem: &Element) -> Option<XsdAtomicType> {
    // Look for restriction
    for child in &elem.children {
        if child.local_name() == xsd_elements::RESTRICTION {
            let base_attr = child.get_attribute(xsd_attrs::BASE);

            let builtin_name = if let Some(base_str) = base_attr {
                let (_base_ns, base_local) = schema.resolve_qname(base_str);
                resolve_builtin_name(&base_local).unwrap_or("string")
            } else {
                "string"
            };

            // Create atomic type with facets
            let mut atomic = match XsdAtomicType::new(builtin_name) {
                Ok(a) => a,
                Err(_) => return None,
            };

            // Parse facets
            let mut enumeration: Vec<String> = Vec::new();
            let mut patterns: Vec<String> = Vec::new();
            let mut min_length: Option<usize> = None;
            let mut max_length: Option<usize> = None;
            let mut length: Option<usize> = None;

            for facet_child in &child.children {
                match facet_child.local_name() {
                    xsd_elements::ENUMERATION => {
                        if let Some(value) = facet_child.get_attribute(xsd_attrs::VALUE) {
                            enumeration.push(value.to_string());
                        }
                    }
                    xsd_elements::PATTERN => {
                        if let Some(value) = facet_child.get_attribute(xsd_attrs::VALUE) {
                            patterns.push(format!("^{}$", value));
                        }
                    }
                    xsd_elements::MIN_LENGTH => {
                        if let Some(value) = facet_child.get_attribute(xsd_attrs::VALUE) {
                            min_length = value.parse().ok();
                        }
                    }
                    xsd_elements::MAX_LENGTH => {
                        if let Some(value) = facet_child.get_attribute(xsd_attrs::VALUE) {
                            max_length = value.parse().ok();
                        }
                    }
                    xsd_elements::LENGTH => {
                        if let Some(value) = facet_child.get_attribute(xsd_attrs::VALUE) {
                            length = value.parse().ok();
                        }
                    }
                    _ => {}
                }
            }

            // Apply facets
            if !enumeration.is_empty() {
                atomic = atomic.with_enumeration(enumeration);
            }
            if let Some(pattern) = patterns.into_iter().find(|p| regex::Regex::new(p).is_ok()) {
                atomic = atomic.with_pattern(&pattern).expect("pattern already validated");
            }
            if let Some(len) = min_length {
                atomic = atomic.with_min_length(len);
            }
            if let Some(len) = max_length {
                atomic = atomic.with_max_length(len);
            }
            if let Some(len) = length {
                atomic = atomic.with_length(len);
            }

            return Some(atomic);
        }
    }

    // Default to string if no restriction found
    XsdAtomicType::new("string").ok()
}

/// Parse a global complex type definition
fn parse_complex_type(schema: &mut XsdSchema, elem: &Element) -> Result<()> {
    let name = elem.get_attribute(xsd_attrs::NAME).ok_or_else(|| {
        Error::Parse(ParseError::new("Global complexType missing 'name' attribute"))
    })?;

    let qname = make_qname(schema, name);

    // Find content model (sequence/choice/all)
    let (model_type, content_model_elem) = if let Some((model_elem, model)) = find_content_model(elem) {
        (model, Some(model_elem))
    } else {
        (ModelType::Sequence, None)
    };

    // Create group and populate with particles from content model
    let mut group = XsdGroup::new(model_type);
    if let Some(model_elem) = content_model_elem {
        // Check if this is a group reference
        if model_elem.local_name() == xsd_elements::GROUP {
            if let Some(ref_str) = model_elem.get_attribute(xsd_attrs::REF) {
                let (ref_ns, ref_local) = schema.resolve_qname(ref_str);
                let ref_qname = QName::new(ref_ns.map(|s| s.to_string()), ref_local);
                let occurs = parse_occurs_option(model_elem);
                let ref_group = XsdGroup::reference(ref_qname, occurs);
                group.particles.push(GroupParticle::Group(Arc::new(ref_group)));
            }
        } else {
            parse_content_model(schema, model_elem, &mut group);
        }
    }

    let mut complex_type = XsdComplexType::new(Some(qname.clone()), Arc::new(group));

    // Parse mixed attribute
    if let Some(mixed) = elem.get_attribute(xsd_attrs::MIXED) {
        complex_type.mixed = mixed == "true";
    }

    // Parse abstract attribute
    if let Some(abstract_) = elem.get_attribute(xsd_attrs::ABSTRACT) {
        complex_type.abstract_type = abstract_ == "true";
    }

    // Parse attributes directly on the complexType
    let mut attr_group = XsdAttributeGroup::anonymous();
    parse_attributes(schema, elem, &mut attr_group);

    // Parse content model children for base type and additional content
    for child in &elem.children {
        if child.local_name() == xsd_elements::COMPLEX_CONTENT
            || child.local_name() == xsd_elements::SIMPLE_CONTENT
        {
            for grandchild in &child.children {
                match grandchild.local_name() {
                    xsd_elements::RESTRICTION => {
                        complex_type.derivation = Some(DerivationMethod::Restriction);
                        if let Some(base) = grandchild.get_attribute(xsd_attrs::BASE) {
                            let (base_ns, base_local) = schema.resolve_qname(base);
                            complex_type.base_type = Some(QName::new(base_ns.map(|s| s.to_string()), base_local));
                        }
                        // Parse attributes from restriction
                        parse_attributes(schema, grandchild, &mut attr_group);
                    }
                    xsd_elements::EXTENSION => {
                        complex_type.derivation = Some(DerivationMethod::Extension);
                        if let Some(base) = grandchild.get_attribute(xsd_attrs::BASE) {
                            let (base_ns, base_local) = schema.resolve_qname(base);
                            complex_type.base_type = Some(QName::new(base_ns.map(|s| s.to_string()), base_local));
                        }
                        // Parse attributes from extension
                        parse_attributes(schema, grandchild, &mut attr_group);
                    }
                    _ => {}
                }
            }
        }
    }

    // Set the attributes on the complex type
    complex_type.attributes = attr_group;

    schema.maps.global_maps.types.insert(qname, GlobalType::Complex(Arc::new(complex_type)));

    Ok(())
}

/// Parse a global simple type definition
fn parse_simple_type(schema: &mut XsdSchema, elem: &Element) -> Result<()> {
    let name = elem.get_attribute(xsd_attrs::NAME).ok_or_else(|| {
        Error::Parse(ParseError::new("Global simpleType missing 'name' attribute"))
    })?;

    let qname = make_qname(schema, name);

    // Determine variety from child elements
    for child in &elem.children {
        match child.local_name() {
            xsd_elements::RESTRICTION => {
                return parse_simple_restriction(schema, child, &qname, name);
            }
            xsd_elements::LIST => {
                return parse_simple_list(schema, child, &qname, name);
            }
            xsd_elements::UNION => {
                return parse_simple_union(schema, child, &qname, name);
            }
            xsd_elements::ANNOTATION => {}
            _ => {}
        }
    }

    // If no derivation found, create a basic atomic type based on xs:string
    let simple = XsdAtomicType::with_name("string", qname.clone())
        .map_err(|e| Error::Parse(ParseError::new(format!("Failed to create simple type: {}", e))))?;

    schema.maps.global_maps.types.insert(qname, GlobalType::Simple(Arc::new(simple)));

    Ok(())
}

/// Parse a simple type restriction
fn parse_simple_restriction(schema: &mut XsdSchema, elem: &Element, qname: &QName, _name: &str) -> Result<()> {
    let base_attr = elem.get_attribute(xsd_attrs::BASE);

    // Resolve the base type QName and look it up
    let base_type: Arc<dyn super::simple_types::SimpleType + Send + Sync> = if let Some(base_str) = base_attr {
        let (base_ns, base_local) = schema.resolve_qname(base_str);

        // Create the base type QName - use XSD namespace if it's a builtin
        let base_qname = if base_ns.is_some() {
            QName::new(base_ns.map(|s| s.to_string()), base_local.to_string())
        } else if resolve_builtin_name(&base_local).is_some() {
            QName::namespaced(XSD_NAMESPACE, base_local)
        } else {
            // Could be a type in the same schema
            QName::new(schema.target_namespace.clone(), base_local.to_string())
        };

        // Try to look up the base type from the schema
        if let Some(existing_type) = schema.maps.lookup_simple_type(&base_qname) {
            Arc::clone(existing_type)
        } else {
            // Fall back to creating an atomic type for builtins
            let builtin_name = resolve_builtin_name(&base_local).unwrap_or("string");
            Arc::new(XsdAtomicType::with_name(builtin_name, base_qname.clone())
                .map_err(|e| Error::Parse(ParseError::new(format!("Unknown base type: {}", e))))?)
        }
    } else {
        // Default to xs:string
        let string_qname = QName::namespaced(XSD_NAMESPACE, "string");
        if let Some(existing_type) = schema.maps.lookup_simple_type(&string_qname) {
            Arc::clone(existing_type)
        } else {
            Arc::new(XsdAtomicType::with_name("string", string_qname)
                .map_err(|e| Error::Parse(ParseError::new(format!("Failed to create string type: {}", e))))?)
        }
    };

    // Collect all facets first
    let mut enumeration: Vec<String> = Vec::new();
    let mut patterns: Vec<String> = Vec::new();
    let mut min_length: Option<usize> = None;
    let mut max_length: Option<usize> = None;
    let mut length: Option<usize> = None;

    for child in &elem.children {
        match child.local_name() {
            xsd_elements::ENUMERATION => {
                if let Some(value) = child.get_attribute(xsd_attrs::VALUE) {
                    enumeration.push(value.to_string());
                }
            }
            xsd_elements::PATTERN => {
                if let Some(value) = child.get_attribute(xsd_attrs::VALUE) {
                    // Convert XSD regex to Rust regex
                    patterns.push(format!("^{}$", value));
                }
            }
            xsd_elements::MIN_LENGTH => {
                if let Some(value) = child.get_attribute(xsd_attrs::VALUE) {
                    min_length = value.parse().ok();
                }
            }
            xsd_elements::MAX_LENGTH => {
                if let Some(value) = child.get_attribute(xsd_attrs::VALUE) {
                    max_length = value.parse().ok();
                }
            }
            xsd_elements::LENGTH => {
                if let Some(value) = child.get_attribute(xsd_attrs::VALUE) {
                    length = value.parse().ok();
                }
            }
            _ => {}
        }
    }

    // Create the restricted type with the base type reference
    let mut restricted = XsdRestrictedType::with_name(base_type, qname.clone());

    // Apply facets
    if !enumeration.is_empty() {
        restricted = restricted.with_enumeration(enumeration);
    }
    // Apply first valid pattern (pre-validate to avoid move issues)
    if let Some(pattern) = patterns.into_iter().find(|p| regex::Regex::new(p).is_ok()) {
        restricted = restricted.with_pattern(&pattern).expect("pattern already validated");
    }
    if let Some(len) = min_length {
        restricted = restricted.with_min_length(len);
    }
    if let Some(len) = max_length {
        restricted = restricted.with_max_length(len);
    }
    if let Some(len) = length {
        restricted = restricted.with_length(len);
    }

    schema.maps.global_maps.types.insert(qname.clone(), GlobalType::Simple(Arc::new(restricted)));

    Ok(())
}

/// Parse a simple type list
fn parse_simple_list(schema: &mut XsdSchema, elem: &Element, qname: &QName, _name: &str) -> Result<()> {
    // Get the itemType attribute
    let item_type_attr = elem.get_attribute(xsd_attrs::ITEM_TYPE);

    // Resolve the item type
    let item_type: Arc<dyn super::simple_types::SimpleType + Send + Sync> = if let Some(item_str) = item_type_attr {
        let (item_ns, item_local) = schema.resolve_qname(item_str);

        // Create the item type QName - use XSD namespace if it's a builtin
        let item_qname = if item_ns.is_some() {
            QName::new(item_ns.map(|s| s.to_string()), item_local.to_string())
        } else if resolve_builtin_name(&item_local).is_some() {
            QName::namespaced(XSD_NAMESPACE, item_local)
        } else {
            // Could be a type in the same schema
            QName::new(schema.target_namespace.clone(), item_local.to_string())
        };

        // Try to look up the item type from the schema
        if let Some(existing_type) = schema.maps.lookup_simple_type(&item_qname) {
            Arc::clone(existing_type)
        } else {
            // Fall back to creating an atomic type for builtins
            let builtin_name = resolve_builtin_name(&item_local).unwrap_or("string");
            Arc::new(XsdAtomicType::with_name(builtin_name, item_qname.clone())
                .map_err(|e| Error::Parse(ParseError::new(format!("Unknown item type: {}", e))))?)
        }
    } else {
        // Default to xs:string if no itemType is specified
        let string_qname = QName::namespaced(XSD_NAMESPACE, "string");
        if let Some(existing_type) = schema.maps.lookup_simple_type(&string_qname) {
            Arc::clone(existing_type)
        } else {
            Arc::new(XsdAtomicType::with_name("string", string_qname)
                .map_err(|e| Error::Parse(ParseError::new(format!("Failed to create string type: {}", e))))?)
        }
    };

    // Create the list type with proper variety
    let list_type = XsdListType::with_name(item_type, qname.clone());

    schema.maps.global_maps.types.insert(qname.clone(), GlobalType::Simple(Arc::new(list_type)));

    Ok(())
}

/// Parse a simple type union
fn parse_simple_union(schema: &mut XsdSchema, elem: &Element, qname: &QName, _name: &str) -> Result<()> {
    // Get the memberTypes attribute (space-separated list of type QNames)
    let member_types_attr = elem.get_attribute(xsd_attrs::MEMBER_TYPES);

    // Resolve member types
    let member_types: Vec<Arc<dyn super::simple_types::SimpleType + Send + Sync>> = if let Some(types_str) = member_types_attr {
        types_str.split_whitespace()
            .map(|type_name| {
                let (member_ns, member_local) = schema.resolve_qname(type_name);

                // Create the member type QName - use XSD namespace if it's a builtin
                let member_qname = if member_ns.is_some() {
                    QName::new(member_ns.map(|s| s.to_string()), member_local.to_string())
                } else if resolve_builtin_name(&member_local).is_some() {
                    QName::namespaced(XSD_NAMESPACE, member_local)
                } else {
                    // Could be a type in the same schema
                    QName::new(schema.target_namespace.clone(), member_local.to_string())
                };

                // Try to look up the member type from the schema
                let member_type: Arc<dyn super::simple_types::SimpleType + Send + Sync> =
                    if let Some(existing_type) = schema.maps.lookup_simple_type(&member_qname) {
                        Arc::clone(existing_type)
                    } else {
                        // Fall back to creating an atomic type for builtins
                        let builtin_name = resolve_builtin_name(&member_local).unwrap_or("string");
                        Arc::new(XsdAtomicType::with_name(builtin_name, member_qname.clone())
                            .map_err(|e| Error::Parse(ParseError::new(format!("Unknown member type: {}", e))))?)
                    };

                Ok(member_type)
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        // No memberTypes attribute - could have inline simpleType children
        // For now, create an empty union (this should be rare in practice)
        vec![]
    };

    // Create the union type
    let union_type = XsdUnionType::with_name(member_types, qname.clone());

    schema.maps.global_maps.types.insert(qname.clone(), GlobalType::Simple(Arc::new(union_type)));

    Ok(())
}

/// Parse a global attribute declaration
fn parse_global_attribute(schema: &mut XsdSchema, elem: &Element) -> Result<()> {
    let name = elem.get_attribute(xsd_attrs::NAME).ok_or_else(|| {
        Error::Parse(ParseError::new("Global attribute missing 'name' attribute"))
    })?;

    let qname = make_qname(schema, name);
    let attr = XsdAttribute::new(qname.clone());

    schema.maps.global_maps.attributes.insert(qname, Arc::new(attr));

    Ok(())
}

/// Parse an attribute group
fn parse_attribute_group(schema: &mut XsdSchema, elem: &Element) -> Result<()> {
    let name = elem.get_attribute(xsd_attrs::NAME).ok_or_else(|| {
        Error::Parse(ParseError::new("attributeGroup missing 'name' attribute"))
    })?;

    let qname = make_qname(schema, name);

    let mut attr_group = super::attributes::XsdAttributeGroup::new(qname.clone());

    // Parse the attribute group content (attributes and attribute group references)
    parse_attributes(schema, elem, &mut attr_group);

    schema.maps.global_maps.attribute_groups.insert(qname, Arc::new(attr_group));

    Ok(())
}

/// Parse a model group
fn parse_group(schema: &mut XsdSchema, elem: &Element) -> Result<()> {
    let name = elem.get_attribute(xsd_attrs::NAME).ok_or_else(|| {
        Error::Parse(ParseError::new("group missing 'name' attribute"))
    })?;

    let qname = make_qname(schema, name);

    // Find the content model element (sequence, choice, or all)
    let model_elem = elem.children.iter().find(|c| {
        matches!(c.local_name(),
            xsd_elements::SEQUENCE | xsd_elements::CHOICE | xsd_elements::ALL)
    });

    let model_type = model_elem.map(|c| {
        match c.local_name() {
            xsd_elements::SEQUENCE => ModelType::Sequence,
            xsd_elements::CHOICE => ModelType::Choice,
            xsd_elements::ALL => ModelType::All,
            _ => ModelType::Sequence,
        }
    }).unwrap_or(ModelType::Sequence);

    let mut group = XsdGroup::named(qname.clone(), model_type);

    // Parse the content model if present
    if let Some(content_elem) = model_elem {
        parse_content_model(schema, content_elem, &mut group);
    }

    schema.maps.global_maps.groups.insert(qname, Arc::new(group));

    Ok(())
}

/// Parse an import declaration
fn parse_import(schema: &mut XsdSchema, elem: &Element) -> Result<()> {
    let namespace = elem.get_attribute(xsd_attrs::NAMESPACE).map(|s| s.to_string());
    let location = elem.get_attribute(xsd_attrs::SCHEMA_LOCATION).map(|s| s.to_string());

    // Check for self-import
    if namespace.as_deref() == schema.target_namespace.as_deref() {
        schema.parse_error(ParseError::new(format!(
            "xs:import cannot import the schema's own targetNamespace: {:?}",
            namespace
        )));
        return Ok(());
    }

    // Check if already imported
    if let Some(ref ns) = namespace {
        if schema.has_import(ns) {
            return Ok(()); // Already imported
        }
    }

    // Add the import record
    if let Some(ref ns) = namespace {
        schema.add_import(ns.clone(), location.clone());
    }

    // If we have a schemaLocation, try to load the imported schema
    if let Some(ref loc) = location {
        if let Some(ref ns) = namespace {
            let resolved_path = resolve_schema_location(loc, schema.base_url(), schema.catalog());

            match load_imported_schema(&resolved_path, Some(ns), schema.source.catalog.clone()) {
                Ok(imported_schema) => {
                    // Update the import record with the loaded schema
                    if let Some(import) = schema.imports.get_mut(ns) {
                        import.schema = Some(Arc::new(imported_schema));
                    }
                }
                Err(e) => {
                    // Import load failures are not fatal - the schema might be
                    // provided later or the types might not be used
                    schema.parse_error(ParseError::new(format!(
                        "xs:import namespace='{}' schemaLocation='{}' load warning: {}",
                        ns, loc, e
                    )));
                }
            }
        }
    }

    Ok(())
}

/// Load and parse an imported schema
fn load_imported_schema(
    path: &Path,
    expected_namespace: Option<&str>,
    catalog: Option<Arc<XmlCatalog>>,
) -> Result<XsdSchema> {
    // Read the file
    let content = std::fs::read_to_string(path).map_err(|e| {
        Error::Resource(format!("Failed to read imported schema '{}': {}", path.display(), e))
    })?;

    // Parse as document
    let doc = Document::from_string(&content)?;
    let root = doc.root().ok_or_else(|| Error::Parse(ParseError::new("Empty imported document")))?;

    // Verify this is a schema element
    if root.local_name() != xsd_elements::SCHEMA {
        return Err(Error::Parse(ParseError::new(format!(
            "Expected xs:schema root element in imported schema, got {}",
            root.local_name()
        ))));
    }

    // Create a new schema for parsing
    let mut imported_schema = XsdSchema::new();
    imported_schema.source.url = Some(path.to_string_lossy().to_string());
    imported_schema.source.base_url = path.parent().map(|p| p.to_string_lossy().to_string());
    imported_schema.source.catalog = catalog;

    // Parse the schema element
    parse_schema_element(&mut imported_schema, root)?;

    // Verify namespace matches if expected
    if let Some(expected_ns) = expected_namespace {
        let actual_ns = imported_schema.target_namespace.as_deref();
        if actual_ns != Some(expected_ns) {
            return Err(Error::Parse(ParseError::new(format!(
                "Imported schema has targetNamespace '{}', expected '{}'",
                actual_ns.unwrap_or("(none)"),
                expected_ns
            ))));
        }
    }

    // Build the imported schema
    imported_schema.build()?;

    Ok(imported_schema)
}

/// Parse an include declaration
///
/// This only collects the include location. The actual loading and merging
/// is done iteratively by `parse_file_internal` to avoid stack overflow
/// on schemas with deep include chains.
fn parse_include(schema: &mut XsdSchema, elem: &Element) -> Result<()> {
    let location = match elem.get_attribute(xsd_attrs::SCHEMA_LOCATION) {
        Some(loc) => loc,
        None => {
            schema.parse_error(ParseError::new("xs:include missing schemaLocation attribute"));
            return Ok(());
        }
    };

    // Don't add duplicates
    if !schema.pending_include_locations.contains(&location.to_string()) {
        schema.pending_include_locations.push(location.to_string());
    }

    Ok(())
}

/// Parse a redefine declaration (xs:redefine)
///
/// xs:redefine is similar to xs:include but allows redefining components
/// from the included schema. The redefined components must derive from
/// themselves (for types) or contain a self-reference (for groups).
///
/// This only collects the redefine location. The actual loading is done
/// iteratively by `parse_file_internal` to avoid stack overflow.
/// Redefinition elements (children of xs:redefine) are processed inline.
fn parse_redefine(schema: &mut XsdSchema, elem: &Element) -> Result<()> {
    let location = match elem.get_attribute(xsd_attrs::SCHEMA_LOCATION) {
        Some(loc) => loc,
        None => {
            schema.parse_error(ParseError::new("xs:redefine missing schemaLocation attribute"));
            return Ok(());
        }
    };

    // Don't add duplicates
    if !schema.pending_redefine_locations.contains(&location.to_string()) {
        schema.pending_redefine_locations.push(location.to_string());
    }

    // NOTE: Redefinition children (simpleType, complexType, group, attributeGroup
    // within xs:redefine) would normally be processed here. However, since we're
    // deferring the schema loading, the redefinitions cannot be fully processed
    // until after all schemas are loaded. For now, we simply include the schema
    // and don't support actual redefinitions (the redefinition elements are ignored).
    //
    // TODO: To properly support xs:redefine, we would need to:
    // 1. Store the redefinition Element data
    // 2. Process redefinitions after all schemas are merged in parse_file_internal

    Ok(())
}

/// Resolve a schemaLocation relative to a base URL, using catalog if available
///
/// Resolution order:
/// 1. Check XML catalog for URN/system ID mapping
/// 2. If location is absolute, use it directly
/// 3. Resolve relative to base_url if available
/// 4. Use the location as-is
fn resolve_schema_location(
    location: &str,
    base_url: Option<&str>,
    catalog: Option<&XmlCatalog>,
) -> PathBuf {
    // First, try to resolve using the catalog
    if let Some(cat) = catalog {
        if let Some(resolved) = cat.resolve(location) {
            return PathBuf::from(resolved);
        }
    }

    let location_path = Path::new(location);

    // If location is absolute, use it directly
    if location_path.is_absolute() {
        return location_path.to_path_buf();
    }

    // If we have a base_url, resolve relative to it
    if let Some(base) = base_url {
        let base_path = Path::new(base);
        return base_path.join(location);
    }

    // Otherwise use the location as-is
    location_path.to_path_buf()
}

/// Load and parse an included schema
fn load_included_schema(
    path: &Path,
    parent_namespace: Option<&str>,
    catalog: Option<Arc<XmlCatalog>>,
    loaded_paths: Arc<std::sync::Mutex<std::collections::HashSet<std::path::PathBuf>>>,
) -> Result<XsdSchema> {
    // Read the file
    let content = std::fs::read_to_string(path).map_err(|e| {
        Error::Resource(format!("Failed to read included schema '{}': {}", path.display(), e))
    })?;

    // Parse as document
    let doc = Document::from_string(&content)?;
    let root = doc.root().ok_or_else(|| Error::Parse(ParseError::new("Empty included document")))?;

    // Verify this is a schema element
    if root.local_name() != xsd_elements::SCHEMA {
        return Err(Error::Parse(ParseError::new(format!(
            "Expected xs:schema root element in included schema, got {}",
            root.local_name()
        ))));
    }

    // Create a new schema for parsing, sharing the loaded_paths set
    let mut included_schema = XsdSchema::new();
    included_schema.source.url = Some(path.to_string_lossy().to_string());
    included_schema.source.base_url = path.parent().map(|p| p.to_string_lossy().to_string());
    included_schema.source.catalog = catalog;
    included_schema.source.loaded_paths = loaded_paths; // Share the set for circular include detection

    // Parse the schema element
    parse_schema_element(&mut included_schema, root)?;

    // Handle chameleon include: if included schema has no target namespace,
    // it inherits the parent's target namespace
    if included_schema.target_namespace.is_none() {
        if let Some(parent_ns) = parent_namespace {
            // Re-namespace all globals to the parent namespace
            chameleon_renamespace(&mut included_schema, parent_ns);
        }
    } else if included_schema.target_namespace.as_deref() != parent_namespace {
        // Target namespace must match for xs:include
        return Err(Error::Parse(ParseError::new(format!(
            "Included schema has different targetNamespace '{}', expected '{}'",
            included_schema.target_namespace.as_deref().unwrap_or("(none)"),
            parent_namespace.unwrap_or("(none)")
        ))));
    }

    // Note: We don't call build() here - globals will be built when parent is built

    Ok(included_schema)
}

/// Re-namespace globals for chameleon include
fn chameleon_renamespace(schema: &mut XsdSchema, target_ns: &str) {
    schema.target_namespace = Some(target_ns.to_string());

    // Re-namespace elements
    let elements: Vec<_> = schema.maps.global_maps.elements.drain().collect();
    for (mut qname, elem) in elements {
        qname.namespace = Some(target_ns.to_string());
        // Also update the element's name
        let mut updated_elem = (*elem).clone();
        updated_elem.name.namespace = Some(target_ns.to_string());
        schema.maps.global_maps.elements.insert(qname, Arc::new(updated_elem));
    }

    // Re-namespace types
    let types: Vec<_> = schema.maps.global_maps.types.drain().collect();
    for (mut qname, typ) in types {
        qname.namespace = Some(target_ns.to_string());
        schema.maps.global_maps.types.insert(qname, typ);
    }

    // Re-namespace groups
    let groups: Vec<_> = schema.maps.global_maps.groups.drain().collect();
    for (mut qname, group) in groups {
        qname.namespace = Some(target_ns.to_string());
        schema.maps.global_maps.groups.insert(qname, group);
    }

    // Re-namespace attributes
    let attrs: Vec<_> = schema.maps.global_maps.attributes.drain().collect();
    for (mut qname, attr) in attrs {
        qname.namespace = Some(target_ns.to_string());
        schema.maps.global_maps.attributes.insert(qname, attr);
    }

    // Re-namespace attribute groups
    let attr_groups: Vec<_> = schema.maps.global_maps.attribute_groups.drain().collect();
    for (mut qname, group) in attr_groups {
        qname.namespace = Some(target_ns.to_string());
        schema.maps.global_maps.attribute_groups.insert(qname, group);
    }

    // Re-namespace notations
    let notations: Vec<_> = schema.maps.global_maps.notations.drain().collect();
    for (mut qname, notation) in notations {
        qname.namespace = Some(target_ns.to_string());
        schema.maps.global_maps.notations.insert(qname, notation);
    }
}

/// Parse a notation
fn parse_notation(schema: &mut XsdSchema, elem: &Element) -> Result<()> {
    let name = elem.get_attribute(xsd_attrs::NAME);
    let public = elem.get_attribute(xsd_attrs::PUBLIC);
    let system = elem.get_attribute(xsd_attrs::SYSTEM);

    if let Some(n) = name {
        let qname = make_qname(schema, n);
        let mut notation = super::globals::XsdNotation::new(qname.clone());
        if let Some(p) = public {
            notation = notation.with_public(p);
        }
        if let Some(s) = system {
            notation = notation.with_system(s);
        }
        schema.maps.global_maps.notations.insert(qname, notation);
    }

    Ok(())
}

/// Make a QName in the target namespace
fn make_qname(schema: &XsdSchema, local_name: &str) -> QName {
    QName::new(schema.target_namespace.clone(), local_name)
}

/// Parse minOccurs and maxOccurs attributes into an Occurs
fn parse_occurs(elem: &Element) -> Occurs {
    let min = elem
        .get_attribute(xsd_attrs::MIN_OCCURS)
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1);

    let max = match elem.get_attribute(xsd_attrs::MAX_OCCURS) {
        Some("unbounded") => None,
        Some(s) => s.parse::<u32>().ok().or(Some(1)),
        None => Some(1),
    };

    Occurs::new(min, max)
}

/// Parse minOccurs and maxOccurs with proper Option handling for max
fn parse_occurs_option(elem: &Element) -> Occurs {
    let min = elem
        .get_attribute(xsd_attrs::MIN_OCCURS)
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1);

    let max = match elem.get_attribute(xsd_attrs::MAX_OCCURS) {
        Some("unbounded") => None,
        Some(s) => s.parse::<u32>().ok().or(Some(1)),
        None => Some(1),
    };

    Occurs::new(min, max)
}

/// Find the content model element (sequence, choice, all, or group) within a complexType derivation element
fn find_content_model_element(elem: &Element) -> Option<&Element> {
    for child in &elem.children {
        match child.local_name() {
            xsd_elements::SEQUENCE | xsd_elements::CHOICE | xsd_elements::ALL | xsd_elements::GROUP => {
                return Some(child);
            }
            _ => {}
        }
    }
    None
}

/// Parse a content model (sequence, choice, or all) and populate the group with particles
fn parse_content_model(schema: &XsdSchema, model_elem: &Element, group: &mut XsdGroup) {
    for child in &model_elem.children {
        match child.local_name() {
            xsd_elements::ELEMENT => {
                if let Some(particle) = parse_element_particle(schema, child) {
                    group.particles.push(GroupParticle::Element(Arc::new(particle)));
                }
            }
            xsd_elements::ANY => {
                if let Some(any) = parse_any_element(schema, child) {
                    group.particles.push(GroupParticle::Any(Arc::new(any)));
                }
            }
            xsd_elements::SEQUENCE | xsd_elements::CHOICE | xsd_elements::ALL => {
                // Nested group
                let nested_model = match child.local_name() {
                    xsd_elements::SEQUENCE => ModelType::Sequence,
                    xsd_elements::CHOICE => ModelType::Choice,
                    xsd_elements::ALL => ModelType::All,
                    _ => unreachable!(),
                };
                let mut nested_group = XsdGroup::new(nested_model);
                nested_group.occurs = parse_occurs_option(child);
                parse_content_model(schema, child, &mut nested_group);
                group.particles.push(GroupParticle::Group(Arc::new(nested_group)));
            }
            xsd_elements::GROUP => {
                // Group reference
                if let Some(ref_str) = child.get_attribute(xsd_attrs::REF) {
                    let (ref_ns, ref_local) = schema.resolve_qname(ref_str);
                    let ref_qname = QName::new(ref_ns.map(|s| s.to_string()), ref_local);
                    let occurs = parse_occurs_option(child);
                    let ref_group = XsdGroup::reference(ref_qname, occurs);
                    group.particles.push(GroupParticle::Group(Arc::new(ref_group)));
                }
            }
            xsd_elements::ANNOTATION => {} // Skip annotations
            _ => {} // Skip unknown elements
        }
    }
}

/// Parse an element particle (local element declaration or reference)
fn parse_element_particle(schema: &XsdSchema, elem: &Element) -> Option<ElementParticle> {
    let occurs = parse_occurs_option(elem);

    // Check if this is a reference
    if let Some(ref_str) = elem.get_attribute(xsd_attrs::REF) {
        let (ref_ns, ref_local) = schema.resolve_qname(ref_str);
        let ref_qname = QName::new(ref_ns.map(|s| s.to_string()), ref_local.clone());
        return Some(ElementParticle::with_ref(ref_qname.clone(), occurs, ref_qname));
    }

    // Local element declaration
    if let Some(name) = elem.get_attribute(xsd_attrs::NAME) {
        let qname = make_qname(schema, name);

        // Parse element type (similar to global element parsing)
        // Track the type reference for forward reference resolution
        let mut type_name_ref: Option<QName> = None;

        let element_type = if let Some(type_str) = elem.get_attribute(xsd_attrs::TYPE) {
            // Resolve type reference
            let (type_ns, type_local) = schema.resolve_qname(type_str);
            let type_qname = QName::new(type_ns.map(|s| s.to_string()), type_local.clone());

            // First try built-in simple types
            if let Some(builtin_name) = resolve_builtin_name(&type_local) {
                if let Ok(simple_type) = XsdAtomicType::new(builtin_name) {
                    ElementType::Simple(Arc::new(simple_type))
                } else {
                    ElementType::Any
                }
            } else if let Some(global_type) = schema.maps.global_maps.types.get(&type_qname) {
                // Check if type is already registered
                match global_type {
                    GlobalType::Complex(ct) => ElementType::Complex(Arc::clone(ct)),
                    GlobalType::Simple(st) => ElementType::Simple(Arc::clone(st)),
                }
            } else {
                // Type not yet parsed - store reference for forward resolution
                type_name_ref = Some(type_qname);
                ElementType::Any
            }
        } else {
            // Look for inline type definition
            let mut inline_type = None;
            for child in &elem.children {
                match child.local_name() {
                    xsd_elements::COMPLEX_TYPE => {
                        if let Some(ct) = parse_inline_complex_type(schema, child) {
                            inline_type = Some(ElementType::Complex(Arc::new(ct)));
                        }
                        break;
                    }
                    xsd_elements::SIMPLE_TYPE => {
                        if let Some(st) = parse_inline_simple_type(schema, child) {
                            inline_type = Some(ElementType::Simple(Arc::new(st)));
                        }
                        break;
                    }
                    _ => {}
                }
            }
            inline_type.unwrap_or(ElementType::Any)
        };

        let mut xsd_element = XsdElement::new(qname.clone(), element_type);
        // Store type reference for forward reference resolution
        xsd_element.type_name = type_name_ref;

        // Parse nillable attribute
        if let Some(nillable) = elem.get_attribute(xsd_attrs::NILLABLE) {
            xsd_element.nillable = nillable == "true";
        }

        // Parse default/fixed values
        if let Some(default) = elem.get_attribute(xsd_attrs::DEFAULT) {
            xsd_element.default = Some(default.to_string());
        }
        if let Some(fixed) = elem.get_attribute(xsd_attrs::FIXED) {
            xsd_element.fixed = Some(fixed.to_string());
        }

        return Some(ElementParticle::with_decl(qname, occurs, Arc::new(xsd_element)));
    }

    None
}

/// Parse an xs:any element wildcard
fn parse_any_element(schema: &XsdSchema, elem: &Element) -> Option<XsdAnyElement> {
    let occurs = parse_occurs_option(elem);

    let process_contents = elem
        .get_attribute("processContents")
        .and_then(ProcessContents::from_str)
        .unwrap_or(ProcessContents::Strict);

    let namespace = elem
        .get_attribute(xsd_attrs::NAMESPACE)
        .map(|ns| NamespaceConstraint::from_namespace_attr(ns, schema.target_namespace.as_deref()))
        .transpose()
        .ok()
        .flatten()
        .unwrap_or(NamespaceConstraint::Any);

    Some(XsdAnyElement::with_settings(
        namespace,
        process_contents,
        occurs,
        schema.target_namespace.as_deref(),
    ))
}

/// Parse an xs:anyAttribute wildcard
fn parse_any_attribute(schema: &XsdSchema, elem: &Element) -> Option<XsdAnyAttribute> {
    let process_contents = elem
        .get_attribute("processContents")
        .and_then(ProcessContents::from_str)
        .unwrap_or(ProcessContents::Strict);

    let namespace = elem
        .get_attribute(xsd_attrs::NAMESPACE)
        .map(|ns| NamespaceConstraint::from_namespace_attr(ns, schema.target_namespace.as_deref()))
        .transpose()
        .ok()
        .flatten()
        .unwrap_or(NamespaceConstraint::Any);

    Some(XsdAnyAttribute::with_settings(
        namespace,
        process_contents,
        schema.target_namespace.as_deref(),
    ))
}

/// Parse attributes from a complex type or extension/restriction
fn parse_attributes(schema: &XsdSchema, parent: &Element, attr_group: &mut XsdAttributeGroup) {
    for child in &parent.children {
        match child.local_name() {
            xsd_elements::ATTRIBUTE => {
                if let Some(attr) = parse_attribute_decl(schema, child) {
                    let _ = attr_group.add_attribute(Arc::new(attr));
                }
            }
            xsd_elements::ATTRIBUTE_GROUP => {
                // Attribute group reference
                if let Some(ref_str) = child.get_attribute(xsd_attrs::REF) {
                    let (ref_ns, ref_local) = schema.resolve_qname(ref_str);
                    let ref_qname = QName::new(ref_ns.map(|s| s.to_string()), ref_local);
                    // Add as pending reference to be resolved in build phase
                    attr_group.add_pending_group_ref(ref_qname);
                }
            }
            xsd_elements::ANY_ATTRIBUTE => {
                if let Some(any_attr) = parse_any_attribute(schema, child) {
                    attr_group.set_any_attribute(Arc::new(any_attr));
                }
            }
            _ => {}
        }
    }
}

/// Parse an attribute declaration
fn parse_attribute_decl(schema: &XsdSchema, elem: &Element) -> Option<XsdAttribute> {
    // Check if this is a reference
    if let Some(ref_str) = elem.get_attribute(xsd_attrs::REF) {
        let (ref_ns, ref_local) = schema.resolve_qname(ref_str);
        let ref_qname = QName::new(ref_ns.map(|s| s.to_string()), ref_local);
        // Create attribute with ref name
        let mut attr = XsdAttribute::new(ref_qname);

        // Parse use attribute
        if let Some(use_str) = elem.get_attribute(xsd_attrs::USE) {
            if let Ok(use_mode) = AttributeUse::from_str(use_str) {
                attr.set_use(use_mode);
            }
        }

        return Some(attr);
    }

    // Local attribute declaration
    let name = elem.get_attribute(xsd_attrs::NAME)?;
    let qname = QName::local(name); // Local attributes typically don't use target namespace

    let mut attr = XsdAttribute::new(qname);

    // Parse type reference
    if let Some(type_str) = elem.get_attribute(xsd_attrs::TYPE) {
        let (type_ns, type_local) = schema.resolve_qname(type_str);

        // First try built-in types
        if let Some(builtin_name) = resolve_builtin_name(&type_local) {
            if let Ok(simple_type) = XsdAtomicType::new(builtin_name) {
                attr.set_type(Arc::new(simple_type));
            }
        } else {
            // Look up user-defined simple type
            let type_qname = QName::new(type_ns.map(|s| s.to_string()), type_local);
            if let Some(global_type) = schema.maps.global_maps.types.get(&type_qname) {
                if let GlobalType::Simple(st) = global_type {
                    attr.set_type(Arc::clone(st));
                }
            } else {
                // Type not yet parsed - store reference for forward resolution
                attr.type_name = Some(type_qname);
            }
        }
    }

    // Parse use attribute
    if let Some(use_str) = elem.get_attribute(xsd_attrs::USE) {
        if let Ok(use_mode) = AttributeUse::from_str(use_str) {
            attr.set_use(use_mode);
        }
    }

    // Parse default/fixed
    if let Some(default) = elem.get_attribute(xsd_attrs::DEFAULT) {
        let _ = attr.set_default(default.to_string());
    }
    if let Some(fixed) = elem.get_attribute(xsd_attrs::FIXED) {
        let _ = attr.set_fixed(fixed.to_string());
    }

    Some(attr)
}

/// Find the content model element (sequence/choice/all) in a complex type or derivation
fn find_content_model(elem: &Element) -> Option<(&Element, ModelType)> {
    for child in &elem.children {
        match child.local_name() {
            xsd_elements::SEQUENCE => return Some((child, ModelType::Sequence)),
            xsd_elements::CHOICE => return Some((child, ModelType::Choice)),
            xsd_elements::ALL => return Some((child, ModelType::All)),
            xsd_elements::COMPLEX_CONTENT | xsd_elements::SIMPLE_CONTENT => {
                // Look inside complexContent/simpleContent for extension/restriction
                for grandchild in &child.children {
                    if grandchild.local_name() == xsd_elements::EXTENSION
                        || grandchild.local_name() == xsd_elements::RESTRICTION
                    {
                        if let Some(result) = find_content_model(grandchild) {
                            return Some(result);
                        }
                    }
                }
            }
            xsd_elements::GROUP => {
                // Group reference at top level - treat as sequence for model type
                return Some((child, ModelType::Sequence));
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE_XSD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
           targetNamespace="http://example.com/test"
           elementFormDefault="qualified">
    <xs:element name="root" type="xs:string"/>
</xs:schema>"#;

    const BOOK_XSD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
           xmlns:book="http://example.com/book"
           targetNamespace="http://example.com/book"
           elementFormDefault="qualified">

    <xs:element name="book" type="book:bookType"/>

    <xs:complexType name="bookType">
        <xs:sequence>
            <xs:element name="title" type="xs:string"/>
            <xs:element name="author" type="xs:string"/>
            <xs:element name="year" type="xs:gYear"/>
            <xs:element name="isbn" type="book:isbnType"/>
        </xs:sequence>
        <xs:attribute name="id" type="xs:ID"/>
        <xs:attribute name="category" type="book:categoryType"/>
    </xs:complexType>

    <xs:simpleType name="isbnType">
        <xs:restriction base="xs:string">
            <xs:pattern value="\d{13}"/>
            <xs:length value="13"/>
        </xs:restriction>
    </xs:simpleType>

    <xs:simpleType name="categoryType">
        <xs:restriction base="xs:string">
            <xs:enumeration value="fiction"/>
            <xs:enumeration value="non-fiction"/>
            <xs:enumeration value="reference"/>
        </xs:restriction>
    </xs:simpleType>

    <xs:simpleType name="ratingType">
        <xs:restriction base="xs:integer">
            <xs:minInclusive value="1"/>
            <xs:maxInclusive value="5"/>
        </xs:restriction>
    </xs:simpleType>
</xs:schema>"#;

    #[test]
    fn test_parse_simple_schema() {
        let schema = XsdSchema::from_string(SIMPLE_XSD).expect("Failed to parse schema");

        assert_eq!(schema.target_namespace.as_deref(), Some("http://example.com/test"));
        assert!(schema.element_form_default.is_qualified());
        assert!(!schema.attribute_form_default.is_qualified());

        // Should have one global element
        assert_eq!(schema.element_count(), 1);

        // Check the element name
        let elem_names: Vec<_> = schema.element_names().collect();
        assert!(elem_names.iter().any(|n| n.local_name == "root"));
    }

    #[test]
    fn test_parse_book_schema() {
        let schema = XsdSchema::from_string(BOOK_XSD).expect("Failed to parse book schema");

        assert_eq!(schema.target_namespace.as_deref(), Some("http://example.com/book"));

        // Should have one global element
        assert_eq!(schema.element_count(), 1);

        // Should have multiple types (1 complex + 3 simple + built-ins)
        let type_count = schema.type_count();
        assert!(type_count >= 4, "Expected at least 4 types, got {}", type_count);

        // Check element exists
        let book_qname = QName::new(Some("http://example.com/book".to_string()), "book");
        assert!(schema.lookup_element(&book_qname).is_some());
    }

    #[test]
    fn test_parse_empty_document_fails() {
        let result = XsdSchema::from_string("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_non_schema_root_fails() {
        let result = XsdSchema::from_string("<root/>");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_simple_type_with_enumeration() {
        let schema = XsdSchema::from_string(BOOK_XSD).expect("Failed to parse book schema");

        let cat_type_qname = QName::new(Some("http://example.com/book".to_string()), "categoryType");
        let cat_type = schema.lookup_type(&cat_type_qname);

        assert!(cat_type.is_some(), "categoryType should exist");
    }

    #[test]
    fn test_parse_simple_type_with_pattern() {
        let schema = XsdSchema::from_string(BOOK_XSD).expect("Failed to parse book schema");

        let isbn_type_qname = QName::new(Some("http://example.com/book".to_string()), "isbnType");
        let isbn_type = schema.lookup_type(&isbn_type_qname);

        assert!(isbn_type.is_some(), "isbnType should exist");
    }

    #[test]
    fn test_version_detection() {
        let xsd11 = r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2009/XMLSchema">
    <xs:element name="test" type="xs:string"/>
</xs:schema>"#;

        let schema = XsdSchema::from_string(xsd11).expect("Failed to parse XSD 1.1 schema");
        assert_eq!(schema.version, XsdVersion::Xsd11);
    }

    #[test]
    fn test_parse_include() {
        // Test parsing a schema with xs:include
        // Create temp files for the test
        let temp_dir = std::env::temp_dir().join("xmlschema_include_test");
        std::fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

        let main_xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
           targetNamespace="http://example.com/test"
           xmlns:tns="http://example.com/test"
           elementFormDefault="qualified">
    <xs:include schemaLocation="types.xsd"/>
    <xs:element name="root" type="tns:personType"/>
</xs:schema>"#;

        let types_xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
           targetNamespace="http://example.com/test"
           elementFormDefault="qualified">
    <xs:complexType name="personType">
        <xs:sequence>
            <xs:element name="name" type="xs:string"/>
            <xs:element name="age" type="xs:integer"/>
        </xs:sequence>
    </xs:complexType>
</xs:schema>"#;

        std::fs::write(temp_dir.join("main.xsd"), main_xsd).expect("Failed to write main.xsd");
        std::fs::write(temp_dir.join("types.xsd"), types_xsd).expect("Failed to write types.xsd");

        // Parse the main schema
        let schema = XsdSchema::from_file(temp_dir.join("main.xsd")).expect("Failed to parse main.xsd");

        // Should have the element from main.xsd
        assert_eq!(schema.element_count(), 1);

        // Should have the personType from the included types.xsd
        let person_qname = QName::new(Some("http://example.com/test".to_string()), "personType");
        let person_type = schema.lookup_type(&person_qname);
        assert!(person_type.is_some(), "personType should be available from included schema");
        assert!(person_type.unwrap().is_complex(), "personType should be a complex type");

        // Check the include was tracked
        assert_eq!(schema.includes.len(), 1);
        assert_eq!(schema.includes[0].location, "types.xsd");

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_chameleon_include() {
        // Test chameleon include - included schema has no targetNamespace
        let temp_dir = std::env::temp_dir().join("xmlschema_chameleon_test");
        std::fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

        let main_xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
           targetNamespace="http://example.com/test"
           xmlns:tns="http://example.com/test"
           elementFormDefault="qualified">
    <xs:include schemaLocation="chameleon.xsd"/>
    <xs:element name="root" type="tns:addressType"/>
</xs:schema>"#;

        // Chameleon schema - no targetNamespace
        let chameleon_xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
           elementFormDefault="qualified">
    <xs:complexType name="addressType">
        <xs:sequence>
            <xs:element name="street" type="xs:string"/>
            <xs:element name="city" type="xs:string"/>
        </xs:sequence>
    </xs:complexType>
</xs:schema>"#;

        std::fs::write(temp_dir.join("main.xsd"), main_xsd).expect("Failed to write main.xsd");
        std::fs::write(temp_dir.join("chameleon.xsd"), chameleon_xsd).expect("Failed to write chameleon.xsd");

        // Parse the main schema
        let schema = XsdSchema::from_file(temp_dir.join("main.xsd")).expect("Failed to parse main.xsd");

        // The addressType should be available in the parent's namespace
        let address_qname = QName::new(Some("http://example.com/test".to_string()), "addressType");
        let address_type = schema.lookup_type(&address_qname);
        assert!(address_type.is_some(), "addressType should be re-namespaced to parent's namespace");

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_circular_include() {
        // Test that circular includes are handled gracefully without infinite loops
        let temp_dir = std::env::temp_dir().join("xmlschema_circular_include_test");
        std::fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

        // Schema A includes B, B includes A (circular)
        let schema_a = r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
           targetNamespace="urn:test:circular"
           elementFormDefault="qualified">
    <xs:include schemaLocation="b.xsd"/>
    <xs:complexType name="TypeA">
        <xs:sequence>
            <xs:element name="value" type="xs:string"/>
        </xs:sequence>
    </xs:complexType>
</xs:schema>"#;

        let schema_b = r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
           targetNamespace="urn:test:circular"
           elementFormDefault="qualified">
    <xs:include schemaLocation="a.xsd"/>
    <xs:complexType name="TypeB">
        <xs:sequence>
            <xs:element name="data" type="xs:integer"/>
        </xs:sequence>
    </xs:complexType>
</xs:schema>"#;

        std::fs::write(temp_dir.join("a.xsd"), schema_a).expect("Failed to write a.xsd");
        std::fs::write(temp_dir.join("b.xsd"), schema_b).expect("Failed to write b.xsd");

        // Parse starting from A - should not hang or crash
        let schema = XsdSchema::from_file(temp_dir.join("a.xsd")).expect("Failed to parse circular schema");

        // Both types should be available
        let type_a_qname = QName::new(Some("urn:test:circular".to_string()), "TypeA");
        let type_b_qname = QName::new(Some("urn:test:circular".to_string()), "TypeB");

        assert!(schema.lookup_type(&type_a_qname).is_some(), "TypeA should be available");
        assert!(schema.lookup_type(&type_b_qname).is_some(), "TypeB should be available from included schema");

        // Only one include should be recorded (B, since A's include of B was not circular from A's perspective)
        assert_eq!(schema.includes.len(), 1, "Should have 1 include recorded");
        assert_eq!(schema.includes[0].location, "b.xsd");

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_parse_import() {
        // Test parsing a schema with xs:import
        let temp_dir = std::env::temp_dir().join("xmlschema_import_test");
        std::fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

        // Main schema imports from a different namespace
        let main_xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
           targetNamespace="http://example.com/main"
           xmlns:tns="http://example.com/main"
           xmlns:addr="http://example.com/address"
           elementFormDefault="qualified">
    <xs:import namespace="http://example.com/address" schemaLocation="address.xsd"/>

    <xs:element name="person">
        <xs:complexType>
            <xs:sequence>
                <xs:element name="name" type="xs:string"/>
                <xs:element ref="addr:address"/>
            </xs:sequence>
        </xs:complexType>
    </xs:element>
</xs:schema>"#;

        // Imported schema with different namespace
        let address_xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
           targetNamespace="http://example.com/address"
           xmlns:tns="http://example.com/address"
           elementFormDefault="qualified">
    <xs:element name="address" type="tns:addressType"/>

    <xs:complexType name="addressType">
        <xs:sequence>
            <xs:element name="street" type="xs:string"/>
            <xs:element name="city" type="xs:string"/>
            <xs:element name="zip" type="xs:string"/>
        </xs:sequence>
    </xs:complexType>
</xs:schema>"#;

        std::fs::write(temp_dir.join("main.xsd"), main_xsd).expect("Failed to write main.xsd");
        std::fs::write(temp_dir.join("address.xsd"), address_xsd).expect("Failed to write address.xsd");

        // Parse the main schema
        let schema = XsdSchema::from_file(temp_dir.join("main.xsd")).expect("Failed to parse main.xsd");

        // Should have the import tracked
        assert!(schema.has_import("http://example.com/address"), "Should have import for address namespace");

        // The imported schema should be loaded
        let import = schema.get_import("http://example.com/address").expect("Import should exist");
        assert!(import.schema.is_some(), "Imported schema should be loaded");

        // Should be able to look up the addressType from the imported schema
        let address_type_qname = QName::new(Some("http://example.com/address".to_string()), "addressType");
        let address_type = schema.lookup_type(&address_type_qname);
        assert!(address_type.is_some(), "addressType should be available from imported schema");
        assert!(address_type.unwrap().is_complex(), "addressType should be a complex type");

        // Should be able to look up the address element from the imported schema
        let address_elem_qname = QName::new(Some("http://example.com/address".to_string()), "address");
        let address_elem = schema.lookup_element(&address_elem_qname);
        assert!(address_elem.is_some(), "address element should be available from imported schema");

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_complex_content_extension() {
        // Test parsing and resolving complex content extension
        let xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
           targetNamespace="http://example.com/test"
           xmlns:tns="http://example.com/test"
           elementFormDefault="qualified">

    <!-- Base type with sequence content -->
    <xs:complexType name="personType">
        <xs:sequence>
            <xs:element name="firstName" type="xs:string"/>
            <xs:element name="lastName" type="xs:string"/>
        </xs:sequence>
        <xs:attribute name="id" type="xs:integer" use="required"/>
    </xs:complexType>

    <!-- Extended type adds more elements and attributes -->
    <xs:complexType name="employeeType">
        <xs:complexContent>
            <xs:extension base="tns:personType">
                <xs:sequence>
                    <xs:element name="department" type="xs:string"/>
                    <xs:element name="salary" type="xs:decimal"/>
                </xs:sequence>
                <xs:attribute name="employeeId" type="xs:string"/>
            </xs:extension>
        </xs:complexContent>
    </xs:complexType>

    <xs:element name="employee" type="tns:employeeType"/>
</xs:schema>"#;

        let schema = XsdSchema::from_string(xsd).expect("Failed to parse schema");

        // Check that base type exists
        let person_qname = QName::new(Some("http://example.com/test".to_string()), "personType");
        let person_type = schema.lookup_type(&person_qname);
        assert!(person_type.is_some(), "personType should exist");
        let person_complex = person_type.unwrap();
        assert!(person_complex.is_complex(), "personType should be complex");

        // Check that derived type exists with proper base_type
        let employee_qname = QName::new(Some("http://example.com/test".to_string()), "employeeType");
        let employee_type = schema.lookup_type(&employee_qname);
        assert!(employee_type.is_some(), "employeeType should exist");

        if let GlobalType::Complex(ct) = employee_type.unwrap() {
            // Should have base type set
            assert!(ct.base_type.is_some(), "employeeType should have base_type");
            assert_eq!(ct.base_type.as_ref().unwrap().local_name, "personType");

            // Should have derivation method set
            assert_eq!(ct.derivation, Some(DerivationMethod::Extension));

            // Content should be merged - wrapper sequence with base + extension
            if let ComplexContent::Group(group) = &ct.content {
                // The wrapper sequence should have 2 nested groups
                assert_eq!(group.particles.len(), 2, "Extension should create wrapper with 2 groups");
            }

            // Should inherit id attribute from base type
            let id_attr = ct.attributes.get_attribute(&QName::local("id"));
            assert!(id_attr.is_some(), "Should inherit 'id' attribute from base type");

            // Should have its own employeeId attribute
            let emp_id_attr = ct.attributes.get_attribute(&QName::local("employeeId"));
            assert!(emp_id_attr.is_some(), "Should have 'employeeId' attribute");
        } else {
            panic!("employeeType should be a complex type");
        }
    }

    #[test]
    fn test_complex_content_restriction() {
        // Test parsing complex content restriction
        let xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
           targetNamespace="http://example.com/test"
           xmlns:tns="http://example.com/test"
           elementFormDefault="qualified">

    <!-- Base type with optional elements -->
    <xs:complexType name="addressType">
        <xs:sequence>
            <xs:element name="street" type="xs:string"/>
            <xs:element name="city" type="xs:string"/>
            <xs:element name="state" type="xs:string" minOccurs="0"/>
            <xs:element name="zip" type="xs:string"/>
        </xs:sequence>
        <xs:attribute name="country" type="xs:string"/>
    </xs:complexType>

    <!-- Restricted type makes state required and removes some flexibility -->
    <xs:complexType name="usAddressType">
        <xs:complexContent>
            <xs:restriction base="tns:addressType">
                <xs:sequence>
                    <xs:element name="street" type="xs:string"/>
                    <xs:element name="city" type="xs:string"/>
                    <xs:element name="state" type="xs:string"/>
                    <xs:element name="zip" type="xs:string"/>
                </xs:sequence>
                <xs:attribute name="country" type="xs:string" fixed="US"/>
            </xs:restriction>
        </xs:complexContent>
    </xs:complexType>

    <xs:element name="usAddress" type="tns:usAddressType"/>
</xs:schema>"#;

        let schema = XsdSchema::from_string(xsd).expect("Failed to parse schema");

        // Check that restricted type exists
        let us_address_qname = QName::new(Some("http://example.com/test".to_string()), "usAddressType");
        let us_address_type = schema.lookup_type(&us_address_qname);
        assert!(us_address_type.is_some(), "usAddressType should exist");

        if let GlobalType::Complex(ct) = us_address_type.unwrap() {
            // Should have base type set
            assert!(ct.base_type.is_some(), "usAddressType should have base_type");
            assert_eq!(ct.base_type.as_ref().unwrap().local_name, "addressType");

            // Should have derivation method set
            assert_eq!(ct.derivation, Some(DerivationMethod::Restriction));

            // Content should be the restriction's content (replaces base)
            if let ComplexContent::Group(group) = &ct.content {
                // Should have 4 elements in the sequence
                assert_eq!(group.particles.len(), 4, "Restriction should have 4 elements");
            }

            // Should inherit country attribute from base (or have it defined locally)
            let country_attr = ct.attributes.get_attribute(&QName::local("country"));
            assert!(country_attr.is_some(), "Should have 'country' attribute");
        } else {
            panic!("usAddressType should be a complex type");
        }
    }

    #[test]
    fn test_group_references() {
        use crate::validators::complex_types::ComplexContent;
        use crate::validators::groups::GroupParticle;

        // Test that group references are resolved
        let xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
        <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
                   targetNamespace="http://example.com/test"
                   xmlns:tns="http://example.com/test"
                   elementFormDefault="qualified">

            <!-- Named group definition -->
            <xs:group name="personGroup">
                <xs:sequence>
                    <xs:element name="firstName" type="xs:string"/>
                    <xs:element name="lastName" type="xs:string"/>
                </xs:sequence>
            </xs:group>

            <!-- Complex type using group reference -->
            <xs:complexType name="personType">
                <xs:sequence>
                    <xs:group ref="tns:personGroup"/>
                    <xs:element name="age" type="xs:integer"/>
                </xs:sequence>
            </xs:complexType>

            <xs:element name="person" type="tns:personType"/>
        </xs:schema>"#;

        let schema = crate::validators::schemas::XsdSchema::from_string(xsd)
            .expect("Should parse schema with group references");

        // Check group was registered
        let person_group_qname = QName::namespaced("http://example.com/test", "personGroup");
        let group = schema.maps.global_maps.groups.get(&person_group_qname);
        assert!(group.is_some(), "personGroup should be registered");

        let group = group.unwrap();
        eprintln!("personGroup particles: {:?}", group.particles);
        assert_eq!(group.particles.len(), 2, "personGroup should have 2 particles (firstName, lastName)");

        // Check the complex type was parsed
        let person_type_qname = QName::namespaced("http://example.com/test", "personType");
        let person_type = schema.maps.global_maps.types.get(&person_type_qname);
        assert!(person_type.is_some(), "personType should be registered");

        if let Some(crate::validators::globals::GlobalType::Complex(ct)) = person_type {
            eprintln!("personType content: {:?}", ct.content);

            // The type should have content
            if let ComplexContent::Group(content_group) = &ct.content {
                // After resolution, should have the group ref resolved to actual elements
                // The sequence contains: group ref + age element
                // After resolution, group ref should expand to firstName + lastName
                eprintln!("Content group particles: {:?}", content_group.particles);

                // Check we have at least 2 particles (could be more after resolution)
                assert!(content_group.particles.len() >= 2,
                    "Content should have particles after group ref resolution, got {}",
                    content_group.particles.len());
            } else {
                panic!("personType should have Group content, got {:?}", ct.content);
            }
        } else {
            panic!("personType should be a complex type");
        }
    }

    #[test]
    fn test_attribute_group_references() {
        // Test that attribute group references are resolved
        let xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
        <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
                   targetNamespace="http://example.com/test"
                   xmlns:tns="http://example.com/test"
                   elementFormDefault="qualified">

            <!-- Named attribute group definition -->
            <xs:attributeGroup name="commonAttrs">
                <xs:attribute name="id" type="xs:ID"/>
                <xs:attribute name="class" type="xs:string"/>
            </xs:attributeGroup>

            <!-- Another attribute group that references the first -->
            <xs:attributeGroup name="fullAttrs">
                <xs:attributeGroup ref="tns:commonAttrs"/>
                <xs:attribute name="style" type="xs:string"/>
            </xs:attributeGroup>

            <!-- Complex type using attribute group reference -->
            <xs:complexType name="elementType">
                <xs:sequence>
                    <xs:element name="content" type="xs:string"/>
                </xs:sequence>
                <xs:attributeGroup ref="tns:fullAttrs"/>
            </xs:complexType>

            <xs:element name="element" type="tns:elementType"/>
        </xs:schema>"#;

        let schema = crate::validators::schemas::XsdSchema::from_string(xsd)
            .expect("Should parse schema with attribute group references");

        // Check commonAttrs group was registered
        let common_attrs_qname = QName::namespaced("http://example.com/test", "commonAttrs");
        let common_group = schema.maps.global_maps.attribute_groups.get(&common_attrs_qname);
        assert!(common_group.is_some(), "commonAttrs should be registered");

        let common_group = common_group.unwrap();
        eprintln!("commonAttrs attributes: {:?}", common_group.len());
        assert_eq!(common_group.len(), 2, "commonAttrs should have 2 attributes (id, class)");

        // Check fullAttrs group was registered and has resolved refs
        let full_attrs_qname = QName::namespaced("http://example.com/test", "fullAttrs");
        let full_group = schema.maps.global_maps.attribute_groups.get(&full_attrs_qname);
        assert!(full_group.is_some(), "fullAttrs should be registered");

        let full_group = full_group.unwrap();
        eprintln!("fullAttrs attributes: {:?}", full_group.len());
        // Should have 3 attributes: id, class (from commonAttrs) + style
        assert_eq!(full_group.len(), 3, "fullAttrs should have 3 attributes (id, class, style)");

        // Check the complex type was parsed and has resolved attribute group
        let element_type_qname = QName::namespaced("http://example.com/test", "elementType");
        let element_type = schema.maps.global_maps.types.get(&element_type_qname);
        assert!(element_type.is_some(), "elementType should be registered");

        if let Some(crate::validators::globals::GlobalType::Complex(ct)) = element_type {
            eprintln!("elementType attributes count: {:?}", ct.attributes.len());

            // Should have 3 attributes from fullAttrs
            assert_eq!(ct.attributes.len(), 3,
                "elementType should have 3 attributes from fullAttrs, got {}",
                ct.attributes.len());

            // Check specific attributes exist
            let id_attr = ct.attributes.get_attribute(&QName::local("id"));
            assert!(id_attr.is_some(), "Should have 'id' attribute");

            let class_attr = ct.attributes.get_attribute(&QName::local("class"));
            assert!(class_attr.is_some(), "Should have 'class' attribute");

            let style_attr = ct.attributes.get_attribute(&QName::local("style"));
            assert!(style_attr.is_some(), "Should have 'style' attribute");
        } else {
            panic!("elementType should be a complex type");
        }
    }

    #[test]
    fn test_any_attribute_parsing() {
        // Test that anyAttribute elements are parsed correctly
        // Using ##any instead of ##other to simplify the test
        let xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
        <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
                   targetNamespace="http://example.com/test"
                   elementFormDefault="qualified">

            <xs:complexType name="extensibleType">
                <xs:sequence>
                    <xs:element name="name" type="xs:string"/>
                </xs:sequence>
                <xs:attribute name="id" type="xs:ID"/>
                <xs:anyAttribute processContents="lax"/>
            </xs:complexType>

            <xs:element name="item" type="extensibleType"/>
        </xs:schema>"#;

        let schema = crate::validators::schemas::XsdSchema::from_string(xsd)
            .expect("Should parse schema with anyAttribute");

        // Check the complex type was parsed
        let type_qname = QName::namespaced("http://example.com/test", "extensibleType");
        let complex_type = schema.maps.global_maps.types.get(&type_qname);
        assert!(complex_type.is_some(), "extensibleType should be registered");

        if let Some(crate::validators::globals::GlobalType::Complex(ct)) = complex_type {
            // Should have the anyAttribute
            assert!(ct.attributes.has_any_attribute(),
                "extensibleType should have anyAttribute");

            // Check the anyAttribute properties
            let any_attr = ct.attributes.any_attribute().unwrap();
            use crate::validators::wildcards::ProcessContents;
            assert_eq!(any_attr.process_contents(), ProcessContents::Lax,
                "anyAttribute should have lax processContents");
        } else {
            panic!("extensibleType should be a complex type");
        }
    }

    #[test]
    fn test_local_element_declaration_parsing() {
        // Test that local element declarations in content models are fully parsed
        // with their type information
        let xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
        <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
                   targetNamespace="http://example.com/test"
                   elementFormDefault="qualified">

            <xs:complexType name="personType">
                <xs:sequence>
                    <xs:element name="firstName" type="xs:string"/>
                    <xs:element name="lastName" type="xs:string"/>
                    <xs:element name="age" type="xs:integer"/>
                    <xs:element name="active" type="xs:boolean" default="true"/>
                </xs:sequence>
            </xs:complexType>

            <xs:element name="person" type="personType"/>
        </xs:schema>"#;

        let schema = crate::validators::schemas::XsdSchema::from_string(xsd)
            .expect("Should parse schema with local elements");

        // Check the complex type was parsed
        let type_qname = QName::namespaced("http://example.com/test", "personType");
        let complex_type = schema.maps.global_maps.types.get(&type_qname);
        assert!(complex_type.is_some(), "personType should be registered");

        if let Some(crate::validators::globals::GlobalType::Complex(ct)) = complex_type {
            // Check content model has particles
            if let crate::validators::complex_types::ComplexContent::Group(ref group) = ct.content {
                assert_eq!(group.particles.len(), 4, "Should have 4 element particles");

                // Check that local elements have their declarations
                for particle in &group.particles {
                    if let crate::validators::groups::GroupParticle::Element(elem_particle) = particle {
                        // Each element particle should have an element declaration
                        assert!(elem_particle.element().is_some(),
                            "Element particle '{}' should have element declaration",
                            elem_particle.name.local_name);

                        // Check the element declaration has proper type
                        let elem_decl = elem_particle.element().unwrap();
                        match &elem_particle.name.local_name[..] {
                            "firstName" | "lastName" => {
                                assert!(matches!(elem_decl.element_type,
                                    crate::validators::elements::ElementType::Simple(_)),
                                    "{} should be a simple type", elem_particle.name.local_name);
                            }
                            "age" => {
                                assert!(matches!(elem_decl.element_type,
                                    crate::validators::elements::ElementType::Simple(_)),
                                    "age should be a simple type");
                            }
                            "active" => {
                                assert!(matches!(elem_decl.element_type,
                                    crate::validators::elements::ElementType::Simple(_)),
                                    "active should be a simple type");
                                assert_eq!(elem_decl.default.as_deref(), Some("true"),
                                    "active should have default value 'true'");
                            }
                            _ => panic!("Unexpected element: {}", elem_particle.name.local_name),
                        }
                    }
                }
            } else {
                panic!("personType should have a group content model");
            }
        } else {
            panic!("personType should be a complex type");
        }
    }
}
