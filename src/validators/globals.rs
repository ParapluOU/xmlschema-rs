//! Global XSD declarations management
//!
//! This module provides the XsdGlobals mediator class that manages global
//! declarations (types, elements, attributes, groups, notations) and provides
//! lookup functionality across schemas.

use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{ParseError, Result};
use crate::namespaces::QName;

use super::base::{ValidationMode, ValidationStatus, Validator};
use super::simple_types::SimpleType;
use super::complex_types::XsdComplexType;
use super::elements::XsdElement;
use super::attributes::{XsdAttribute, XsdAttributeGroup};
use super::groups::XsdGroup;
use super::identities::XsdIdentity;

/// Type map - maps QNames to global types
pub type TypeMap = HashMap<QName, GlobalType>;
/// Notation map - maps QNames to notation declarations
pub type NotationMap = HashMap<QName, XsdNotation>;
/// Attribute map - maps QNames to global attribute declarations
pub type AttributeMap = HashMap<QName, Arc<XsdAttribute>>;
/// Attribute group map - maps QNames to attribute group definitions
pub type AttributeGroupMap = HashMap<QName, Arc<XsdAttributeGroup>>;
/// Element map - maps QNames to global element declarations
pub type ElementMap = HashMap<QName, Arc<XsdElement>>;
/// Group map - maps QNames to model group definitions
pub type GroupMap = HashMap<QName, Arc<XsdGroup>>;
/// Identity map - maps QNames to identity constraints
pub type IdentityMap = HashMap<QName, Arc<XsdIdentity>>;
/// Substitution group map - maps head element QNames to substitute elements
pub type SubstitutionGroupMap = HashMap<QName, Vec<Arc<XsdElement>>>;

/// XSD Notation declaration
#[derive(Debug, Clone)]
pub struct XsdNotation {
    /// Notation name
    pub name: QName,
    /// Public identifier
    pub public: Option<String>,
    /// System identifier
    pub system: Option<String>,
    /// Target namespace
    pub target_namespace: Option<String>,
    /// Building errors
    errors: Vec<ParseError>,
    /// Whether built
    built: bool,
}

impl XsdNotation {
    /// Create a new notation
    pub fn new(name: QName) -> Self {
        Self {
            name,
            public: None,
            system: None,
            target_namespace: None,
            errors: Vec::new(),
            built: false,
        }
    }

    /// Set the public identifier
    pub fn with_public(mut self, public: impl Into<String>) -> Self {
        self.public = Some(public.into());
        self
    }

    /// Set the system identifier
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Set the target namespace
    pub fn with_target_namespace(mut self, ns: impl Into<String>) -> Self {
        self.target_namespace = Some(ns.into());
        self
    }
}

impl Validator for XsdNotation {
    fn is_built(&self) -> bool {
        self.built
    }

    fn build(&mut self) -> Result<()> {
        if self.built {
            return Ok(());
        }

        // Validate notation - must have public or system
        if self.public.is_none() && self.system.is_none() {
            self.errors.push(ParseError::new(format!(
                "notation '{}' must have 'public' or 'system' attribute",
                self.name.to_string()
            )));
        }

        self.built = true;
        Ok(())
    }

    fn validation_attempted(&self) -> ValidationStatus {
        if !self.built {
            ValidationStatus::None
        } else if self.errors.is_empty() {
            ValidationStatus::Full
        } else {
            ValidationStatus::Partial
        }
    }

    fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    fn errors(&self) -> Vec<ParseError> {
        self.errors.clone()
    }
}

/// A global type - either simple or complex
#[derive(Debug, Clone)]
pub enum GlobalType {
    /// Simple type
    Simple(Arc<dyn SimpleType + Send + Sync>),
    /// Complex type
    Complex(Arc<XsdComplexType>),
}

impl GlobalType {
    /// Check if this is a simple type
    pub fn is_simple(&self) -> bool {
        matches!(self, GlobalType::Simple(_))
    }

    /// Check if this is a complex type
    pub fn is_complex(&self) -> bool {
        matches!(self, GlobalType::Complex(_))
    }

    /// Get the type name
    pub fn name(&self) -> Option<&QName> {
        match self {
            // SimpleType trait extends TypeValidator which has name() method
            GlobalType::Simple(t) => t.name(),
            GlobalType::Complex(t) => t.name.as_ref(),
        }
    }

    /// Get as simple type
    pub fn as_simple(&self) -> Option<&Arc<dyn SimpleType + Send + Sync>> {
        match self {
            GlobalType::Simple(t) => Some(t),
            GlobalType::Complex(_) => None,
        }
    }

    /// Get as complex type
    pub fn as_complex(&self) -> Option<&Arc<XsdComplexType>> {
        match self {
            GlobalType::Simple(_) => None,
            GlobalType::Complex(t) => Some(t),
        }
    }
}

/// Collection of global maps for XSD components
#[derive(Debug, Default)]
pub struct GlobalMaps {
    /// Global type definitions (simple and complex)
    pub types: TypeMap,
    /// Notation declarations
    pub notations: NotationMap,
    /// Global attribute declarations
    pub attributes: AttributeMap,
    /// Attribute group definitions
    pub attribute_groups: AttributeGroupMap,
    /// Global element declarations
    pub elements: ElementMap,
    /// Model group definitions
    pub groups: GroupMap,
}

impl GlobalMaps {
    /// Create empty global maps
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all maps
    pub fn clear(&mut self) {
        self.types.clear();
        self.notations.clear();
        self.attributes.clear();
        self.attribute_groups.clear();
        self.elements.clear();
        self.groups.clear();
    }

    /// Check if all maps are empty
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
            && self.notations.is_empty()
            && self.attributes.is_empty()
            && self.attribute_groups.is_empty()
            && self.elements.is_empty()
            && self.groups.is_empty()
    }

    /// Get total count of all globals
    pub fn len(&self) -> usize {
        self.types.len()
            + self.notations.len()
            + self.attributes.len()
            + self.attribute_groups.len()
            + self.elements.len()
            + self.groups.len()
    }

    /// Add a simple type
    pub fn add_simple_type(&mut self, name: QName, typ: Arc<dyn SimpleType + Send + Sync>) {
        self.types.insert(name, GlobalType::Simple(typ));
    }

    /// Add a complex type
    pub fn add_complex_type(&mut self, name: QName, typ: Arc<XsdComplexType>) {
        self.types.insert(name, GlobalType::Complex(typ));
    }

    /// Add a notation
    pub fn add_notation(&mut self, name: QName, notation: XsdNotation) {
        self.notations.insert(name, notation);
    }

    /// Add an attribute
    pub fn add_attribute(&mut self, name: QName, attr: Arc<XsdAttribute>) {
        self.attributes.insert(name, attr);
    }

    /// Add an attribute group
    pub fn add_attribute_group(&mut self, name: QName, group: Arc<XsdAttributeGroup>) {
        self.attribute_groups.insert(name, group);
    }

    /// Add an element
    pub fn add_element(&mut self, name: QName, element: Arc<XsdElement>) {
        self.elements.insert(name, element);
    }

    /// Add a group
    pub fn add_group(&mut self, name: QName, group: Arc<XsdGroup>) {
        self.groups.insert(name, group);
    }

    /// Merge another set of global maps
    pub fn merge(&mut self, other: &GlobalMaps) {
        self.types.extend(other.types.clone());
        self.notations.extend(other.notations.clone());
        self.attributes.extend(other.attributes.clone());
        self.attribute_groups.extend(other.attribute_groups.clone());
        self.elements.extend(other.elements.clone());
        self.groups.extend(other.groups.clone());
    }

    /// Iterate over all global types
    pub fn iter_types(&self) -> impl Iterator<Item = (&QName, &GlobalType)> {
        self.types.iter()
    }

    /// Iterate over all global elements
    pub fn iter_elements(&self) -> impl Iterator<Item = (&QName, &Arc<XsdElement>)> {
        self.elements.iter()
    }

    /// Iterate over all global attributes
    pub fn iter_attributes(&self) -> impl Iterator<Item = (&QName, &Arc<XsdAttribute>)> {
        self.attributes.iter()
    }
}

/// XSD Globals - mediator class for schema global declarations
#[derive(Debug)]
pub struct XsdGlobals {
    /// Validation mode
    pub mode: ValidationMode,
    /// Target namespace
    pub target_namespace: Option<String>,
    /// Global maps
    pub global_maps: GlobalMaps,
    /// Substitution groups (head element -> substitutes)
    pub substitution_groups: SubstitutionGroupMap,
    /// Identity constraints
    pub identities: IdentityMap,
    /// Building errors
    errors: Vec<ParseError>,
    /// Whether built
    built: bool,
}

impl XsdGlobals {
    /// Create new empty globals
    pub fn new() -> Self {
        Self {
            mode: ValidationMode::Strict,
            target_namespace: None,
            global_maps: GlobalMaps::new(),
            substitution_groups: HashMap::new(),
            identities: HashMap::new(),
            errors: Vec::new(),
            built: false,
        }
    }

    /// Set the target namespace
    pub fn with_target_namespace(mut self, ns: impl Into<String>) -> Self {
        self.target_namespace = Some(ns.into());
        self
    }

    /// Set validation mode
    pub fn with_mode(mut self, mode: ValidationMode) -> Self {
        self.mode = mode;
        self
    }

    // ========== Type Lookups ==========

    /// Look up a type by name
    pub fn lookup_type(&self, name: &QName) -> Option<&GlobalType> {
        self.global_maps.types.get(name)
    }

    /// Look up a simple type by name
    pub fn lookup_simple_type(&self, name: &QName) -> Option<&Arc<dyn SimpleType + Send + Sync>> {
        self.global_maps.types.get(name).and_then(|t| t.as_simple())
    }

    /// Look up a complex type by name
    pub fn lookup_complex_type(&self, name: &QName) -> Option<&Arc<XsdComplexType>> {
        self.global_maps.types.get(name).and_then(|t| t.as_complex())
    }

    // ========== Element Lookups ==========

    /// Look up an element by name
    pub fn lookup_element(&self, name: &QName) -> Option<&Arc<XsdElement>> {
        self.global_maps.elements.get(name)
    }

    // ========== Attribute Lookups ==========

    /// Look up an attribute by name
    pub fn lookup_attribute(&self, name: &QName) -> Option<&Arc<XsdAttribute>> {
        self.global_maps.attributes.get(name)
    }

    /// Look up an attribute group by name
    pub fn lookup_attribute_group(&self, name: &QName) -> Option<&Arc<XsdAttributeGroup>> {
        self.global_maps.attribute_groups.get(name)
    }

    // ========== Group Lookups ==========

    /// Look up a group by name
    pub fn lookup_group(&self, name: &QName) -> Option<&Arc<XsdGroup>> {
        self.global_maps.groups.get(name)
    }

    // ========== Notation Lookups ==========

    /// Look up a notation by name
    pub fn lookup_notation(&self, name: &QName) -> Option<&XsdNotation> {
        self.global_maps.notations.get(name)
    }

    // ========== Identity Lookups ==========

    /// Look up an identity constraint by name
    pub fn lookup_identity(&self, name: &QName) -> Option<&Arc<XsdIdentity>> {
        self.identities.get(name)
    }

    // ========== Substitution Groups ==========

    /// Register an element in a substitution group
    pub fn add_to_substitution_group(&mut self, head: QName, member: Arc<XsdElement>) {
        self.substitution_groups
            .entry(head)
            .or_default()
            .push(member);
    }

    /// Get substitution group members for a head element
    pub fn get_substitution_group(&self, head: &QName) -> Option<&Vec<Arc<XsdElement>>> {
        self.substitution_groups.get(head)
    }

    /// Check if an element is substitutable for another
    pub fn is_substitutable(&self, element: &QName, head: &QName) -> bool {
        if let Some(group) = self.substitution_groups.get(head) {
            group.iter().any(|e| e.name == *element)
        } else {
            false
        }
    }

    // ========== Registration ==========

    /// Register a simple type
    pub fn register_simple_type(&mut self, name: QName, typ: Arc<dyn SimpleType + Send + Sync>) {
        self.global_maps.add_simple_type(name.clone(), typ);
        self.built = false;
    }

    /// Register a complex type
    pub fn register_complex_type(&mut self, name: QName, typ: Arc<XsdComplexType>) {
        self.global_maps.add_complex_type(name.clone(), typ);
        self.built = false;
    }

    /// Register an element
    pub fn register_element(&mut self, name: QName, element: Arc<XsdElement>) {
        // Handle substitution group
        if let Some(ref subst) = element.substitution_group {
            self.add_to_substitution_group(subst.clone(), element.clone());
        }
        self.global_maps.add_element(name, element);
        self.built = false;
    }

    /// Register an attribute
    pub fn register_attribute(&mut self, name: QName, attr: Arc<XsdAttribute>) {
        self.global_maps.add_attribute(name, attr);
        self.built = false;
    }

    /// Register an attribute group
    pub fn register_attribute_group(&mut self, name: QName, group: Arc<XsdAttributeGroup>) {
        self.global_maps.add_attribute_group(name, group);
        self.built = false;
    }

    /// Register a model group
    pub fn register_group(&mut self, name: QName, group: Arc<XsdGroup>) {
        self.global_maps.add_group(name, group);
        self.built = false;
    }

    /// Register a notation
    pub fn register_notation(&mut self, name: QName, notation: XsdNotation) {
        self.global_maps.add_notation(name, notation);
        self.built = false;
    }

    /// Register an identity constraint
    pub fn register_identity(&mut self, name: QName, identity: Arc<XsdIdentity>) {
        self.identities.insert(name, identity);
        self.built = false;
    }

    // ========== Clearing ==========

    /// Clear all global declarations
    pub fn clear(&mut self) {
        self.global_maps.clear();
        self.substitution_groups.clear();
        self.identities.clear();
        self.errors.clear();
        self.built = false;
    }

    // ========== Statistics ==========

    /// Get total number of global declarations
    pub fn total_globals(&self) -> usize {
        self.global_maps.len() + self.identities.len()
    }

    /// Get number of types
    pub fn type_count(&self) -> usize {
        self.global_maps.types.len()
    }

    /// Get number of elements
    pub fn element_count(&self) -> usize {
        self.global_maps.elements.len()
    }

    /// Get number of attributes
    pub fn attribute_count(&self) -> usize {
        self.global_maps.attributes.len()
    }

    /// Get number of groups
    pub fn group_count(&self) -> usize {
        self.global_maps.groups.len()
    }

    // ========== Validation ==========

    /// Validate all substitution groups for circularity
    fn check_substitution_groups(&mut self) {
        for (head, members) in &self.substitution_groups {
            // Check for self-substitution
            for member in members {
                if member.name == *head {
                    self.errors.push(ParseError::new(format!(
                        "circularity found for substitution group with head element '{}'",
                        head.to_string()
                    )));
                }
            }
        }
    }

    /// Validate all identity constraints
    fn check_identities(&mut self) {
        // Validate keyref references
        for (name, identity) in &self.identities {
            if let Some(ref refer) = identity.refer {
                if !self.identities.contains_key(refer) {
                    self.errors.push(ParseError::new(format!(
                        "keyref '{}' refers to unknown constraint '{}'",
                        name.to_string(),
                        refer.to_string()
                    )));
                }
            }
        }
    }
}

impl Default for XsdGlobals {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator for XsdGlobals {
    fn is_built(&self) -> bool {
        self.built
    }

    fn build(&mut self) -> Result<()> {
        if self.built {
            return Ok(());
        }

        self.errors.clear();

        // Validate components
        self.check_substitution_groups();
        self.check_identities();

        self.built = true;
        Ok(())
    }

    fn validation_attempted(&self) -> ValidationStatus {
        if !self.built {
            ValidationStatus::None
        } else if self.errors.is_empty() {
            ValidationStatus::Full
        } else {
            ValidationStatus::Partial
        }
    }

    fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    fn errors(&self) -> Vec<ParseError> {
        self.errors.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::simple_types::XsdAtomicType;
    use super::super::elements::ElementType;

    /// Helper function to create a test atomic type
    fn create_test_atomic_type(name: &str) -> Arc<XsdAtomicType> {
        Arc::new(XsdAtomicType::with_name("string", QName::local(name)).unwrap())
    }

    /// Helper function to create a test element
    fn create_test_element(name: &str) -> Arc<XsdElement> {
        Arc::new(XsdElement::new(QName::local(name), ElementType::Any))
    }

    #[test]
    fn test_notation_creation() {
        let notation = XsdNotation::new(QName::local("myNotation"))
            .with_public("public-id")
            .with_system("system-id");

        assert_eq!(notation.name.local_name, "myNotation");
        assert_eq!(notation.public, Some("public-id".to_string()));
        assert_eq!(notation.system, Some("system-id".to_string()));
    }

    #[test]
    fn test_notation_validation() {
        // Valid notation with public
        let mut notation = XsdNotation::new(QName::local("valid"))
            .with_public("public-id");
        notation.build().unwrap();
        assert!(!notation.has_errors());

        // Invalid notation without public or system
        let mut invalid = XsdNotation::new(QName::local("invalid"));
        invalid.build().unwrap();
        assert!(invalid.has_errors());
    }

    #[test]
    fn test_global_type_enum() {
        let simple = create_test_atomic_type("myString");
        let global_simple = GlobalType::Simple(simple);

        assert!(global_simple.is_simple());
        assert!(!global_simple.is_complex());
        assert!(global_simple.as_simple().is_some());
        assert!(global_simple.as_complex().is_none());
    }

    #[test]
    fn test_global_maps() {
        let mut maps = GlobalMaps::new();
        assert!(maps.is_empty());
        assert_eq!(maps.len(), 0);

        // Add a simple type
        let typ = create_test_atomic_type("testType");
        maps.add_simple_type(QName::local("testType"), typ);
        assert!(!maps.is_empty());
        assert_eq!(maps.len(), 1);

        // Add an element
        let elem = create_test_element("testElement");
        maps.add_element(QName::local("testElement"), elem);
        assert_eq!(maps.len(), 2);

        // Clear
        maps.clear();
        assert!(maps.is_empty());
    }

    #[test]
    fn test_xsd_globals_creation() {
        let globals = XsdGlobals::new()
            .with_target_namespace("http://example.com/test")
            .with_mode(ValidationMode::Lax);

        assert_eq!(globals.target_namespace, Some("http://example.com/test".to_string()));
        assert_eq!(globals.mode, ValidationMode::Lax);
        assert!(!globals.built);
    }

    #[test]
    fn test_xsd_globals_type_registration() {
        let mut globals = XsdGlobals::new();

        // Register a simple type
        let typ = create_test_atomic_type("myType");
        globals.register_simple_type(QName::local("myType"), typ);

        // Look it up
        let found = globals.lookup_type(&QName::local("myType"));
        assert!(found.is_some());
        assert!(found.unwrap().is_simple());

        let simple = globals.lookup_simple_type(&QName::local("myType"));
        assert!(simple.is_some());

        let complex = globals.lookup_complex_type(&QName::local("myType"));
        assert!(complex.is_none());
    }

    #[test]
    fn test_xsd_globals_element_registration() {
        let mut globals = XsdGlobals::new();

        let elem = create_test_element("myElement");
        globals.register_element(QName::local("myElement"), elem);

        let found = globals.lookup_element(&QName::local("myElement"));
        assert!(found.is_some());
        assert_eq!(found.unwrap().name.local_name, "myElement");

        assert_eq!(globals.element_count(), 1);
    }

    #[test]
    fn test_substitution_groups() {
        let mut globals = XsdGlobals::new();

        // Register head element
        let head = create_test_element("head");
        globals.register_element(QName::local("head"), head);

        // Register member with substitution group
        let mut member = XsdElement::new(QName::local("member"), ElementType::Any);
        member.substitution_group = Some(QName::local("head"));
        let member = Arc::new(member);
        globals.register_element(QName::local("member"), member);

        // Check substitution group
        let group = globals.get_substitution_group(&QName::local("head"));
        assert!(group.is_some());
        assert_eq!(group.unwrap().len(), 1);

        // Check substitutability
        assert!(globals.is_substitutable(&QName::local("member"), &QName::local("head")));
        assert!(!globals.is_substitutable(&QName::local("other"), &QName::local("head")));
    }

    #[test]
    fn test_identity_registration() {
        use super::super::identities::{XsdIdentity, XsdSelector};

        let mut globals = XsdGlobals::new();

        let identity = Arc::new(
            XsdIdentity::key(
                QName::local("myKey"),
                XsdSelector::new(".//item"),
            )
        );
        globals.register_identity(QName::local("myKey"), identity);

        let found = globals.lookup_identity(&QName::local("myKey"));
        assert!(found.is_some());
    }

    #[test]
    fn test_xsd_globals_build() {
        let mut globals = XsdGlobals::new();

        // Register some elements
        let elem1 = create_test_element("elem1");
        let elem2 = create_test_element("elem2");

        globals.register_element(QName::local("elem1"), elem1);
        globals.register_element(QName::local("elem2"), elem2);

        // Build
        globals.build().unwrap();
        assert!(globals.is_built());
        assert!(!globals.has_errors());
    }

    #[test]
    fn test_xsd_globals_clear() {
        let mut globals = XsdGlobals::new();

        // Add some data
        let typ = create_test_atomic_type("testType");
        globals.register_simple_type(QName::local("testType"), typ);

        let elem = create_test_element("testElement");
        globals.register_element(QName::local("testElement"), elem);

        assert!(globals.total_globals() > 0);

        // Clear
        globals.clear();
        assert_eq!(globals.total_globals(), 0);
        assert!(!globals.is_built());
    }

    #[test]
    fn test_global_maps_merge() {
        let mut maps1 = GlobalMaps::new();
        let mut maps2 = GlobalMaps::new();

        // Add to maps1
        let typ1 = create_test_atomic_type("type1");
        maps1.add_simple_type(QName::local("type1"), typ1);

        // Add to maps2
        let typ2 = create_test_atomic_type("type2");
        maps2.add_simple_type(QName::local("type2"), typ2);

        // Merge
        maps1.merge(&maps2);
        assert_eq!(maps1.types.len(), 2);
        assert!(maps1.types.contains_key(&QName::local("type1")));
        assert!(maps1.types.contains_key(&QName::local("type2")));
    }

    #[test]
    fn test_keyref_validation() {
        use super::super::identities::{XsdIdentity, XsdSelector};

        let mut globals = XsdGlobals::new();

        // Register a keyref that refers to unknown constraint
        let keyref = Arc::new(
            XsdIdentity::keyref(
                QName::local("myKeyref"),
                XsdSelector::new(".//item"),
                QName::local("unknownKey"),
            )
        );
        globals.register_identity(QName::local("myKeyref"), keyref);

        // Build - should detect missing reference
        globals.build().unwrap();
        assert!(globals.has_errors());
        assert!(globals.errors().iter().any(|e| e.message.contains("unknownKey")));
    }

    #[test]
    fn test_statistics() {
        let mut globals = XsdGlobals::new();

        // Add various declarations
        let typ = create_test_atomic_type("type1");
        globals.register_simple_type(QName::local("type1"), typ);

        let elem = create_test_element("elem1");
        globals.register_element(QName::local("elem1"), elem);

        assert_eq!(globals.type_count(), 1);
        assert_eq!(globals.element_count(), 1);
        assert_eq!(globals.attribute_count(), 0);
        assert_eq!(globals.group_count(), 0);
        assert_eq!(globals.total_globals(), 2);
    }
}
