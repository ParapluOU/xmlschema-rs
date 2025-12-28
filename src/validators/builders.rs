//! XSD component builders
//!
//! This module provides builder infrastructure for constructing XSD components.
//! It includes versioned builders that support both XSD 1.0 and XSD 1.1.

use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{ParseError, Result};
use crate::namespaces::QName;

use super::base::ValidationMode;
use super::simple_types::{SimpleType, XsdAtomicType, XsdListType, XsdRestrictedType, XsdUnionType};
use super::complex_types::{ComplexTypeBuilder, XsdComplexType};
use super::elements::{ElementScope, ElementType, XsdElement};
use super::attributes::{AttributeForm, AttributeUse, XsdAttribute, XsdAttributeGroup};
use super::groups::{ModelType, XsdGroup};
use super::wildcards::{NamespaceConstraint, ProcessContents, XsdAnyElement, XsdAnyAttribute};
use super::identities::{XsdIdentity, XsdSelector, XsdField};
use super::particles::Occurs;
use super::globals::{XsdGlobals, XsdNotation};
use super::builtins::XSD_NAMESPACE;

/// XSD version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum XsdVersion {
    /// XSD 1.0
    #[default]
    Xsd10,
    /// XSD 1.1
    Xsd11,
}

impl XsdVersion {
    /// Parse from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "1.0" => Ok(XsdVersion::Xsd10),
            "1.1" => Ok(XsdVersion::Xsd11),
            _ => Err(crate::error::Error::Value(format!(
                "Invalid XSD version: '{}'. Must be '1.0' or '1.1'",
                s
            ))),
        }
    }

    /// Get as string
    pub fn as_str(&self) -> &'static str {
        match self {
            XsdVersion::Xsd10 => "1.0",
            XsdVersion::Xsd11 => "1.1",
        }
    }
}

impl std::fmt::Display for XsdVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Builder for XSD components
///
/// Provides factory methods for creating various XSD component types
/// with version-specific behavior for XSD 1.0 and XSD 1.1.
#[derive(Debug)]
pub struct XsdBuilders {
    /// XSD version
    pub version: XsdVersion,
    /// Target namespace for built components
    pub target_namespace: Option<String>,
    /// Validation mode
    pub mode: ValidationMode,
    /// Building errors
    errors: Vec<ParseError>,
}

impl XsdBuilders {
    /// Create a new builder with default settings (XSD 1.0)
    pub fn new() -> Self {
        Self {
            version: XsdVersion::Xsd10,
            target_namespace: None,
            mode: ValidationMode::Strict,
            errors: Vec::new(),
        }
    }

    /// Create a builder for a specific XSD version
    pub fn with_version(version: XsdVersion) -> Self {
        Self {
            version,
            target_namespace: None,
            mode: ValidationMode::Strict,
            errors: Vec::new(),
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

    /// Add a building error
    pub fn add_error(&mut self, error: ParseError) {
        self.errors.push(error);
    }

    /// Get building errors
    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }

    /// Clear building errors
    pub fn clear_errors(&mut self) {
        self.errors.clear();
    }

    // ========== Type Builders ==========

    /// Create a QName in the target namespace
    pub fn qname(&self, local_name: &str) -> QName {
        match &self.target_namespace {
            Some(ns) => QName::new(Some(ns.clone()), local_name.to_string()),
            None => QName::local(local_name),
        }
    }

    /// Create a QName in the XSD namespace
    pub fn xsd_qname(&self, local_name: &str) -> QName {
        QName::new(Some(XSD_NAMESPACE.to_string()), local_name.to_string())
    }

    /// Build an atomic type restriction
    pub fn build_atomic_type(
        &self,
        name: &str,
        base_type: &str,
    ) -> Result<XsdAtomicType> {
        XsdAtomicType::with_name(base_type, self.qname(name))
    }

    /// Build a restricted type
    pub fn build_restricted_type(
        &self,
        name: &str,
        base_type: Arc<dyn SimpleType + Send + Sync>,
    ) -> XsdRestrictedType {
        XsdRestrictedType::with_name(base_type, self.qname(name))
    }

    /// Build a list type
    pub fn build_list_type(
        &self,
        name: &str,
        item_type: Arc<dyn SimpleType + Send + Sync>,
    ) -> XsdListType {
        XsdListType::with_name(item_type, self.qname(name))
    }

    /// Build a union type
    pub fn build_union_type(
        &self,
        name: &str,
        member_types: Vec<Arc<dyn SimpleType + Send + Sync>>,
    ) -> XsdUnionType {
        XsdUnionType::with_name(member_types, self.qname(name))
    }

    /// Build a complex type with sequence content
    pub fn build_complex_type_sequence(
        &self,
        name: &str,
    ) -> XsdComplexType {
        ComplexTypeBuilder::new()
            .name(self.qname(name))
            .content_group(Arc::new(XsdGroup::new(ModelType::Sequence)))
            .build()
    }

    /// Build a complex type with mixed content
    pub fn build_complex_type_mixed(
        &self,
        name: &str,
    ) -> XsdComplexType {
        ComplexTypeBuilder::new()
            .name(self.qname(name))
            .content_group(Arc::new(XsdGroup::new(ModelType::Sequence)))
            .mixed(true)
            .build()
    }

    /// Build an empty complex type
    pub fn build_complex_type_empty(
        &self,
        name: &str,
    ) -> XsdComplexType {
        XsdComplexType::empty(Some(self.qname(name)))
    }

    /// Build a complex type with simple content
    pub fn build_complex_type_simple(
        &self,
        name: &str,
        base_type: Arc<dyn SimpleType + Send + Sync>,
    ) -> XsdComplexType {
        XsdComplexType::with_simple_content(Some(self.qname(name)), base_type)
    }

    // ========== Element Builders ==========

    /// Build a global element declaration
    pub fn build_global_element(
        &self,
        name: &str,
        element_type: ElementType,
    ) -> XsdElement {
        let mut elem = XsdElement::new(self.qname(name), element_type);
        elem.scope = ElementScope::Global;
        elem
    }

    /// Build a local element declaration
    pub fn build_local_element(
        &self,
        name: &str,
        element_type: ElementType,
    ) -> XsdElement {
        let mut elem = XsdElement::new(self.qname(name), element_type);
        elem.scope = ElementScope::Local;
        elem
    }

    /// Build an element with minOccurs/maxOccurs
    pub fn build_element_with_occurs(
        &self,
        name: &str,
        element_type: ElementType,
        occurs: Occurs,
    ) -> XsdElement {
        let mut elem = XsdElement::new(self.qname(name), element_type);
        elem.occurs = occurs;
        elem
    }

    /// Build an element with a nillable flag
    pub fn build_nillable_element(
        &self,
        name: &str,
        element_type: ElementType,
    ) -> XsdElement {
        let mut elem = XsdElement::new(self.qname(name), element_type);
        elem.nillable = true;
        elem
    }

    /// Build an element with a default value
    pub fn build_element_with_default(
        &self,
        name: &str,
        element_type: ElementType,
        default: impl Into<String>,
    ) -> XsdElement {
        let mut elem = XsdElement::new(self.qname(name), element_type);
        elem.default = Some(default.into());
        elem
    }

    /// Build an element with a fixed value
    pub fn build_element_with_fixed(
        &self,
        name: &str,
        element_type: ElementType,
        fixed: impl Into<String>,
    ) -> XsdElement {
        let mut elem = XsdElement::new(self.qname(name), element_type);
        elem.fixed = Some(fixed.into());
        elem
    }

    /// Build an element in a substitution group
    pub fn build_element_in_substitution_group(
        &self,
        name: &str,
        element_type: ElementType,
        head: QName,
    ) -> XsdElement {
        let mut elem = XsdElement::new(self.qname(name), element_type);
        elem.substitution_group = Some(head);
        elem
    }

    // ========== Attribute Builders ==========

    /// Build a qualified attribute (with namespace)
    pub fn build_qualified_attribute(
        &self,
        name: &str,
    ) -> XsdAttribute {
        let mut attr = XsdAttribute::new(self.qname(name));
        attr.set_form(AttributeForm::Qualified);
        attr
    }

    /// Build an unqualified attribute
    pub fn build_unqualified_attribute(
        &self,
        name: &str,
    ) -> XsdAttribute {
        let mut attr = XsdAttribute::new(self.qname(name));
        attr.set_form(AttributeForm::Unqualified);
        attr
    }

    /// Build a required attribute
    pub fn build_required_attribute(
        &self,
        name: &str,
    ) -> XsdAttribute {
        let mut attr = XsdAttribute::new(self.qname(name));
        attr.set_use(AttributeUse::Required);
        attr
    }

    /// Build an optional attribute
    pub fn build_optional_attribute(
        &self,
        name: &str,
    ) -> XsdAttribute {
        let mut attr = XsdAttribute::new(self.qname(name));
        attr.set_use(AttributeUse::Optional);
        attr
    }

    /// Build an attribute with default value
    pub fn build_attribute_with_default(
        &self,
        name: &str,
        default: impl Into<String>,
    ) -> Result<XsdAttribute> {
        let mut attr = XsdAttribute::new(self.qname(name));
        attr.set_default(default.into())?;
        Ok(attr)
    }

    /// Build an attribute with fixed value
    pub fn build_attribute_with_fixed(
        &self,
        name: &str,
        fixed: impl Into<String>,
    ) -> Result<XsdAttribute> {
        let mut attr = XsdAttribute::new(self.qname(name));
        attr.set_fixed(fixed.into())?;
        Ok(attr)
    }

    // ========== Attribute Group Builders ==========

    /// Build a named attribute group
    pub fn build_attribute_group(
        &self,
        name: &str,
    ) -> XsdAttributeGroup {
        XsdAttributeGroup::new(self.qname(name))
    }

    /// Build an attribute group with attributes
    pub fn build_attribute_group_with_attrs(
        &self,
        name: &str,
        attributes: Vec<Arc<XsdAttribute>>,
    ) -> XsdAttributeGroup {
        let mut group = XsdAttributeGroup::new(self.qname(name));
        for attr in attributes {
            let _ = group.add_attribute(attr);
        }
        group
    }

    // ========== Group Builders ==========

    /// Build a sequence group
    pub fn build_sequence_group(
        &self,
        name: Option<&str>,
    ) -> XsdGroup {
        match name {
            Some(n) => XsdGroup::named(self.qname(n), ModelType::Sequence),
            None => XsdGroup::new(ModelType::Sequence),
        }
    }

    /// Build a choice group
    pub fn build_choice_group(
        &self,
        name: Option<&str>,
    ) -> XsdGroup {
        match name {
            Some(n) => XsdGroup::named(self.qname(n), ModelType::Choice),
            None => XsdGroup::new(ModelType::Choice),
        }
    }

    /// Build an all group
    pub fn build_all_group(
        &self,
        name: Option<&str>,
    ) -> XsdGroup {
        match name {
            Some(n) => XsdGroup::named(self.qname(n), ModelType::All),
            None => XsdGroup::new(ModelType::All),
        }
    }

    // ========== Wildcard Builders ==========

    /// Build an any element wildcard
    pub fn build_any_element(
        &self,
        namespace_constraint: NamespaceConstraint,
        process_contents: ProcessContents,
    ) -> XsdAnyElement {
        XsdAnyElement::with_settings(
            namespace_constraint,
            process_contents,
            Occurs::once(),
            self.target_namespace.as_deref(),
        )
    }

    /// Build an any element that accepts any namespace
    pub fn build_any_element_any(&self) -> XsdAnyElement {
        XsdAnyElement::with_settings(
            NamespaceConstraint::Any,
            ProcessContents::Lax,
            Occurs::zero_or_more(),
            self.target_namespace.as_deref(),
        )
    }

    /// Build an any element for the target namespace only
    pub fn build_any_element_target_namespace(&self) -> XsdAnyElement {
        use std::collections::HashSet;

        let constraint = match &self.target_namespace {
            Some(ns) => {
                let mut set = HashSet::new();
                set.insert(ns.clone());
                NamespaceConstraint::Enumeration(set)
            }
            None => {
                // Empty namespace (##local)
                let mut set = HashSet::new();
                set.insert(String::new());
                NamespaceConstraint::Enumeration(set)
            }
        };
        XsdAnyElement::with_settings(
            constraint,
            ProcessContents::Strict,
            Occurs::once(),
            self.target_namespace.as_deref(),
        )
    }

    /// Build an any attribute wildcard
    pub fn build_any_attribute(
        &self,
        namespace_constraint: NamespaceConstraint,
        process_contents: ProcessContents,
    ) -> XsdAnyAttribute {
        XsdAnyAttribute::with_settings(
            namespace_constraint,
            process_contents,
            self.target_namespace.as_deref(),
        )
    }

    /// Build an any attribute that accepts any namespace
    pub fn build_any_attribute_any(&self) -> XsdAnyAttribute {
        XsdAnyAttribute::with_settings(
            NamespaceConstraint::Any,
            ProcessContents::Lax,
            self.target_namespace.as_deref(),
        )
    }

    // ========== Identity Builders ==========

    /// Build a unique constraint
    pub fn build_unique(
        &self,
        name: &str,
        selector: &str,
        fields: Vec<&str>,
    ) -> XsdIdentity {
        let mut identity = XsdIdentity::unique(
            self.qname(name),
            XsdSelector::new(selector),
        );
        for field in fields {
            identity.add_field(XsdField::new(field));
        }
        identity
    }

    /// Build a key constraint
    pub fn build_key(
        &self,
        name: &str,
        selector: &str,
        fields: Vec<&str>,
    ) -> XsdIdentity {
        let mut identity = XsdIdentity::key(
            self.qname(name),
            XsdSelector::new(selector),
        );
        for field in fields {
            identity.add_field(XsdField::new(field));
        }
        identity
    }

    /// Build a keyref constraint
    pub fn build_keyref(
        &self,
        name: &str,
        selector: &str,
        fields: Vec<&str>,
        refer: &str,
    ) -> XsdIdentity {
        let mut identity = XsdIdentity::keyref(
            self.qname(name),
            XsdSelector::new(selector),
            self.qname(refer),
        );
        for field in fields {
            identity.add_field(XsdField::new(field));
        }
        identity
    }

    // ========== Notation Builders ==========

    /// Build a notation
    pub fn build_notation(
        &self,
        name: &str,
        public: Option<&str>,
        system: Option<&str>,
    ) -> XsdNotation {
        let mut notation = XsdNotation::new(self.qname(name));
        if let Some(p) = public {
            notation = notation.with_public(p);
        }
        if let Some(s) = system {
            notation = notation.with_system(s);
        }
        notation
    }

    // ========== Special Type Builders ==========

    /// Build the xs:anyType complex type
    ///
    /// This creates a complex type equivalent that accepts any content.
    pub fn build_any_type(&self) -> XsdComplexType {
        // xs:anyType has mixed content with any element
        let content = Arc::new(self.build_any_content_group());
        ComplexTypeBuilder::new()
            .name(self.xsd_qname("anyType"))
            .content_group(content)
            .mixed(true)
            .build()
    }

    /// Build an any content group for a complex type
    ///
    /// Creates a sequence group with an any element that accepts unbounded any content.
    pub fn build_any_content_group(&self) -> XsdGroup {
        let mut group = XsdGroup::new(ModelType::Sequence);
        let any = XsdAnyElement::with_settings(
            NamespaceConstraint::Any,
            ProcessContents::Lax,
            Occurs::zero_or_more(),
            self.target_namespace.as_deref(),
        );
        group.add_any(any);
        group
    }

    /// Build an empty content group
    pub fn build_empty_content_group(&self, model: ModelType) -> XsdGroup {
        XsdGroup::new(model)
    }

    /// Build an any attribute group
    ///
    /// Creates an anonymous attribute group that accepts any attributes.
    /// Note: Returns the group and the wildcard separately since XsdAttributeGroup
    /// doesn't currently store wildcard references.
    pub fn build_any_attribute_group(&self) -> (XsdAttributeGroup, XsdAnyAttribute) {
        let group = XsdAttributeGroup::anonymous();
        let any_attr = XsdAnyAttribute::with_settings(
            NamespaceConstraint::Any,
            ProcessContents::Lax,
            self.target_namespace.as_deref(),
        );
        (group, any_attr)
    }

    /// Build an empty attribute group (anonymous)
    pub fn build_empty_attribute_group(&self) -> XsdAttributeGroup {
        XsdAttributeGroup::anonymous()
    }

    // ========== Globals Builder ==========

    /// Build a new XsdGlobals instance
    pub fn build_globals(&self) -> XsdGlobals {
        let mut globals = XsdGlobals::new().with_mode(self.mode);
        if let Some(ref ns) = self.target_namespace {
            globals = globals.with_target_namespace(ns.clone());
        }
        globals
    }

    /// Build globals with built-in types registered
    pub fn build_globals_with_builtins(&self) -> Result<XsdGlobals> {
        let mut globals = self.build_globals();
        self.register_builtins(&mut globals)?;
        Ok(globals)
    }

    /// Register built-in XSD types
    pub fn register_builtins(&self, globals: &mut XsdGlobals) -> Result<()> {
        // Register xs:anyType
        let any_type = Arc::new(self.build_any_type());
        globals.register_complex_type(self.xsd_qname("anyType"), any_type);

        // Register basic atomic types using with_name(builtin_name, qname)
        let string_type = Arc::new(XsdAtomicType::with_name("string", self.xsd_qname("string"))?);
        globals.register_simple_type(self.xsd_qname("string"), string_type);

        let boolean_type = Arc::new(XsdAtomicType::with_name("boolean", self.xsd_qname("boolean"))?);
        globals.register_simple_type(self.xsd_qname("boolean"), boolean_type);

        let decimal_type = Arc::new(XsdAtomicType::with_name("decimal", self.xsd_qname("decimal"))?);
        globals.register_simple_type(self.xsd_qname("decimal"), decimal_type);

        let integer_type = Arc::new(XsdAtomicType::with_name("integer", self.xsd_qname("integer"))?);
        globals.register_simple_type(self.xsd_qname("integer"), integer_type);

        let float_type = Arc::new(XsdAtomicType::with_name("float", self.xsd_qname("float"))?);
        globals.register_simple_type(self.xsd_qname("float"), float_type);

        let double_type = Arc::new(XsdAtomicType::with_name("double", self.xsd_qname("double"))?);
        globals.register_simple_type(self.xsd_qname("double"), double_type);

        Ok(())
    }
}

impl Default for XsdBuilders {
    fn default() -> Self {
        Self::new()
    }
}

/// A staged item waiting to be built
#[derive(Debug, Clone)]
pub enum StagedItem<T> {
    /// Not yet built
    Pending {
        /// QName of the item
        qname: QName,
        /// Raw data for building (opaque)
        data: T,
    },
    /// Currently being built (for circular reference detection)
    Building {
        /// QName of the item
        qname: QName,
    },
    /// Successfully built
    Built {
        /// QName of the item
        qname: QName,
    },
}

impl<T> StagedItem<T> {
    /// Create a pending staged item
    pub fn pending(qname: QName, data: T) -> Self {
        StagedItem::Pending { qname, data }
    }

    /// Check if the item is pending
    pub fn is_pending(&self) -> bool {
        matches!(self, StagedItem::Pending { .. })
    }

    /// Check if the item is being built
    pub fn is_building(&self) -> bool {
        matches!(self, StagedItem::Building { .. })
    }

    /// Check if the item is built
    pub fn is_built(&self) -> bool {
        matches!(self, StagedItem::Built { .. })
    }

    /// Get the QName
    pub fn qname(&self) -> &QName {
        match self {
            StagedItem::Pending { qname, .. } => qname,
            StagedItem::Building { qname } => qname,
            StagedItem::Built { qname } => qname,
        }
    }

    /// Mark as building
    pub fn mark_building(&mut self) {
        if let StagedItem::Pending { qname, .. } = self {
            *self = StagedItem::Building { qname: qname.clone() };
        }
    }

    /// Mark as built
    pub fn mark_built(&mut self) {
        let qname = self.qname().clone();
        *self = StagedItem::Built { qname };
    }
}

/// Staged map for lazy building of global components
#[derive(Debug)]
pub struct StagedMap<T, B> {
    /// The built components
    store: HashMap<QName, B>,
    /// Staged items waiting to be built
    staging: HashMap<QName, StagedItem<T>>,
}

impl<T, B> Default for StagedMap<T, B> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, B> StagedMap<T, B> {
    /// Create a new staged map
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
            staging: HashMap::new(),
        }
    }

    /// Get a built component
    pub fn get(&self, qname: &QName) -> Option<&B> {
        self.store.get(qname)
    }

    /// Check if a component exists (built or staged)
    pub fn contains(&self, qname: &QName) -> bool {
        self.store.contains_key(qname) || self.staging.contains_key(qname)
    }

    /// Check if a component is built
    pub fn is_built(&self, qname: &QName) -> bool {
        self.store.contains_key(qname)
    }

    /// Check if a component is staged
    pub fn is_staged(&self, qname: &QName) -> bool {
        self.staging.contains_key(qname)
    }

    /// Stage a component for later building
    pub fn stage(&mut self, qname: QName, data: T) {
        self.staging.insert(qname.clone(), StagedItem::pending(qname, data));
    }

    /// Insert a built component directly
    pub fn insert(&mut self, qname: QName, built: B) {
        self.store.insert(qname, built);
    }

    /// Remove a staged item
    pub fn remove_staged(&mut self, qname: &QName) -> Option<StagedItem<T>> {
        self.staging.remove(qname)
    }

    /// Get number of built components
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    /// Get number of staged components
    pub fn staged_count(&self) -> usize {
        self.staging.len()
    }

    /// Get total count (built + staged)
    pub fn total_count(&self) -> usize {
        self.store.len() + self.staging.len()
    }

    /// Clear all components
    pub fn clear(&mut self) {
        self.store.clear();
        self.staging.clear();
    }

    /// Iterate over built components
    pub fn iter(&self) -> impl Iterator<Item = (&QName, &B)> {
        self.store.iter()
    }

    /// Get all staged QNames
    pub fn staged_names(&self) -> Vec<&QName> {
        self.staging.keys().collect()
    }
}

/// Schema build context
///
/// Tracks the state during schema building, including circular reference detection.
#[derive(Debug)]
pub struct BuildContext {
    /// Components currently being built (for circular reference detection)
    building_stack: Vec<QName>,
    /// Errors encountered during building
    errors: Vec<ParseError>,
    /// XSD version
    version: XsdVersion,
}

impl BuildContext {
    /// Create a new build context
    pub fn new(version: XsdVersion) -> Self {
        Self {
            building_stack: Vec::new(),
            errors: Vec::new(),
            version,
        }
    }

    /// Push a component onto the building stack
    pub fn push(&mut self, qname: QName) -> Result<()> {
        if self.building_stack.contains(&qname) {
            return Err(crate::error::Error::Parse(
                ParseError::new(format!(
                    "Circular reference detected for '{}'",
                    qname.to_string()
                ))
            ));
        }
        self.building_stack.push(qname);
        Ok(())
    }

    /// Pop a component from the building stack
    pub fn pop(&mut self) {
        self.building_stack.pop();
    }

    /// Check if a component is being built
    pub fn is_building(&self, qname: &QName) -> bool {
        self.building_stack.contains(qname)
    }

    /// Add an error
    pub fn add_error(&mut self, error: ParseError) {
        self.errors.push(error);
    }

    /// Get errors
    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }

    /// Check if there are errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get the XSD version
    pub fn version(&self) -> XsdVersion {
        self.version
    }

    /// Get building depth
    pub fn depth(&self) -> usize {
        self.building_stack.len()
    }
}

impl Default for BuildContext {
    fn default() -> Self {
        Self::new(XsdVersion::Xsd10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::base::AttributeValidator;
    use super::super::identities::IdentityConstraintKind;

    #[test]
    fn test_xsd_version() {
        assert_eq!(XsdVersion::from_str("1.0").unwrap(), XsdVersion::Xsd10);
        assert_eq!(XsdVersion::from_str("1.1").unwrap(), XsdVersion::Xsd11);
        assert!(XsdVersion::from_str("2.0").is_err());

        assert_eq!(XsdVersion::Xsd10.as_str(), "1.0");
        assert_eq!(XsdVersion::Xsd11.as_str(), "1.1");
    }

    #[test]
    fn test_xsd_builders_creation() {
        let builder = XsdBuilders::new();
        assert_eq!(builder.version, XsdVersion::Xsd10);
        assert!(builder.target_namespace.is_none());
        assert!(builder.errors.is_empty());
    }

    #[test]
    fn test_xsd_builders_with_settings() {
        let builder = XsdBuilders::with_version(XsdVersion::Xsd11)
            .with_target_namespace("http://example.com/test")
            .with_mode(ValidationMode::Lax);

        assert_eq!(builder.version, XsdVersion::Xsd11);
        assert_eq!(builder.target_namespace, Some("http://example.com/test".to_string()));
        assert_eq!(builder.mode, ValidationMode::Lax);
    }

    #[test]
    fn test_qname_creation() {
        let builder = XsdBuilders::new()
            .with_target_namespace("http://example.com");

        let qname = builder.qname("myType");
        assert_eq!(qname.namespace, Some("http://example.com".to_string()));
        assert_eq!(qname.local_name, "myType");

        let xsd_qname = builder.xsd_qname("string");
        assert_eq!(xsd_qname.namespace, Some(XSD_NAMESPACE.to_string()));
        assert_eq!(xsd_qname.local_name, "string");
    }

    #[test]
    fn test_build_global_element() {
        let builder = XsdBuilders::new()
            .with_target_namespace("http://example.com");

        let elem = builder.build_global_element("myElement", ElementType::Any);
        assert_eq!(elem.name.local_name, "myElement");
        assert_eq!(elem.scope, ElementScope::Global);
    }

    #[test]
    fn test_build_local_element() {
        let builder = XsdBuilders::new();

        let elem = builder.build_local_element("localElem", ElementType::Any);
        assert_eq!(elem.name.local_name, "localElem");
        assert_eq!(elem.scope, ElementScope::Local);
    }

    #[test]
    fn test_build_nillable_element() {
        let builder = XsdBuilders::new();

        let elem = builder.build_nillable_element("nillableElem", ElementType::Any);
        assert!(elem.nillable);
    }

    #[test]
    fn test_build_element_with_default() {
        let builder = XsdBuilders::new();

        let elem = builder.build_element_with_default("elem", ElementType::Any, "defaultValue");
        assert_eq!(elem.default, Some("defaultValue".to_string()));
    }

    #[test]
    fn test_build_element_with_fixed() {
        let builder = XsdBuilders::new();

        let elem = builder.build_element_with_fixed("elem", ElementType::Any, "fixedValue");
        assert_eq!(elem.fixed, Some("fixedValue".to_string()));
    }

    #[test]
    fn test_build_qualified_attribute() {
        let builder = XsdBuilders::new()
            .with_target_namespace("http://example.com");

        let attr = builder.build_qualified_attribute("myAttr");
        assert_eq!(attr.name().local_name, "myAttr");
        assert!(attr.is_qualified());
    }

    #[test]
    fn test_build_required_attribute() {
        let builder = XsdBuilders::new();

        let attr = builder.build_required_attribute("requiredAttr");
        assert_eq!(attr.use_mode(), AttributeUse::Required);
    }

    #[test]
    fn test_build_sequence_group() {
        let builder = XsdBuilders::new();

        let group = builder.build_sequence_group(Some("myGroup"));
        assert_eq!(group.model, ModelType::Sequence);
        assert!(group.name.is_some());
    }

    #[test]
    fn test_build_choice_group() {
        let builder = XsdBuilders::new();

        let group = builder.build_choice_group(None);
        assert_eq!(group.model, ModelType::Choice);
        assert!(group.name.is_none());
    }

    #[test]
    fn test_build_any_element() {
        let builder = XsdBuilders::new();

        let any = builder.build_any_element_any();
        assert_eq!(any.wildcard.namespace, NamespaceConstraint::Any);
        assert_eq!(any.process_contents(), ProcessContents::Lax);
    }

    #[test]
    fn test_build_any_attribute() {
        let builder = XsdBuilders::new();

        let any = builder.build_any_attribute_any();
        assert_eq!(any.wildcard.namespace, NamespaceConstraint::Any);
        assert_eq!(any.process_contents(), ProcessContents::Lax);
    }

    #[test]
    fn test_build_unique_constraint() {
        let builder = XsdBuilders::new();

        let identity = builder.build_unique(
            "uniqueId",
            ".//item",
            vec!["@id"],
        );
        assert_eq!(identity.kind, IdentityConstraintKind::Unique);
        assert_eq!(identity.fields.len(), 1);
    }

    #[test]
    fn test_build_key_constraint() {
        let builder = XsdBuilders::new();

        let identity = builder.build_key(
            "itemKey",
            ".//item",
            vec!["@id", "@name"],
        );
        assert_eq!(identity.kind, IdentityConstraintKind::Key);
        assert_eq!(identity.fields.len(), 2);
    }

    #[test]
    fn test_build_keyref_constraint() {
        let builder = XsdBuilders::new();

        let identity = builder.build_keyref(
            "itemRef",
            ".//ref",
            vec!["@itemId"],
            "itemKey",
        );
        assert_eq!(identity.kind, IdentityConstraintKind::Keyref);
        assert!(identity.refer.is_some());
    }

    #[test]
    fn test_build_notation() {
        let builder = XsdBuilders::new();

        let notation = builder.build_notation(
            "myNotation",
            Some("public-id"),
            Some("system-id"),
        );
        assert_eq!(notation.name.local_name, "myNotation");
        assert_eq!(notation.public, Some("public-id".to_string()));
        assert_eq!(notation.system, Some("system-id".to_string()));
    }

    #[test]
    fn test_build_any_type() {
        use super::super::complex_types::ContentTypeLabel;

        let builder = XsdBuilders::new();

        let any_type = builder.build_any_type();
        assert_eq!(any_type.name.as_ref().unwrap().local_name, "anyType");
        assert_eq!(any_type.content_type_label(), ContentTypeLabel::Mixed);
    }

    #[test]
    fn test_build_any_content_group() {
        let builder = XsdBuilders::new();

        let group = builder.build_any_content_group();
        assert_eq!(group.model, ModelType::Sequence);
    }

    #[test]
    fn test_build_globals() {
        let builder = XsdBuilders::new()
            .with_target_namespace("http://example.com")
            .with_mode(ValidationMode::Lax);

        let globals = builder.build_globals();
        assert_eq!(globals.target_namespace, Some("http://example.com".to_string()));
        assert_eq!(globals.mode, ValidationMode::Lax);
    }

    #[test]
    fn test_build_globals_with_builtins() {
        let builder = XsdBuilders::new();
        let globals = builder.build_globals_with_builtins().unwrap();

        // Check that basic types are registered
        let string_type = globals.lookup_type(&builder.xsd_qname("string"));
        assert!(string_type.is_some());
        assert!(string_type.unwrap().is_simple());

        let any_type = globals.lookup_type(&builder.xsd_qname("anyType"));
        assert!(any_type.is_some());
        assert!(any_type.unwrap().is_complex());
    }

    #[test]
    fn test_staged_item() {
        let mut item: StagedItem<String> = StagedItem::pending(
            QName::local("test"),
            "data".to_string(),
        );

        assert!(item.is_pending());
        assert!(!item.is_building());
        assert!(!item.is_built());

        item.mark_building();
        assert!(!item.is_pending());
        assert!(item.is_building());

        item.mark_built();
        assert!(item.is_built());
    }

    #[test]
    fn test_staged_map() {
        let mut map: StagedMap<String, i32> = StagedMap::new();

        // Stage an item
        map.stage(QName::local("item1"), "data1".to_string());
        assert!(map.is_staged(&QName::local("item1")));
        assert!(!map.is_built(&QName::local("item1")));
        assert!(map.contains(&QName::local("item1")));
        assert_eq!(map.staged_count(), 1);

        // Insert a built item
        map.insert(QName::local("item2"), 42);
        assert!(map.is_built(&QName::local("item2")));
        assert_eq!(map.get(&QName::local("item2")), Some(&42));
        assert_eq!(map.len(), 1);

        // Clear
        map.clear();
        assert!(map.is_empty());
        assert_eq!(map.staged_count(), 0);
    }

    #[test]
    fn test_build_context() {
        let mut ctx = BuildContext::new(XsdVersion::Xsd10);

        // Push and pop normally
        ctx.push(QName::local("type1")).unwrap();
        assert!(ctx.is_building(&QName::local("type1")));
        assert_eq!(ctx.depth(), 1);

        ctx.push(QName::local("type2")).unwrap();
        assert_eq!(ctx.depth(), 2);

        ctx.pop();
        assert_eq!(ctx.depth(), 1);
        assert!(!ctx.is_building(&QName::local("type2")));
    }

    #[test]
    fn test_build_context_circular_detection() {
        let mut ctx = BuildContext::new(XsdVersion::Xsd10);

        ctx.push(QName::local("type1")).unwrap();
        ctx.push(QName::local("type2")).unwrap();

        // Attempting to push type1 again should fail
        let result = ctx.push(QName::local("type1"));
        assert!(result.is_err());
    }

    #[test]
    fn test_build_context_errors() {
        let mut ctx = BuildContext::new(XsdVersion::Xsd11);

        assert!(!ctx.has_errors());

        ctx.add_error(ParseError::new("test error"));
        assert!(ctx.has_errors());
        assert_eq!(ctx.errors().len(), 1);
    }

    #[test]
    fn test_builder_error_handling() {
        let mut builder = XsdBuilders::new();

        assert!(builder.errors().is_empty());

        builder.add_error(ParseError::new("build error 1"));
        builder.add_error(ParseError::new("build error 2"));

        assert_eq!(builder.errors().len(), 2);

        builder.clear_errors();
        assert!(builder.errors().is_empty());
    }
}
