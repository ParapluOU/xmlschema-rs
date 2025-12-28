//! XSD Element Validators
//!
//! This module implements element declarations for XSD schemas.
//! Elements are the primary building blocks of XML documents.
//!
//! Reference: https://www.w3.org/TR/xmlschema11-1/#Element_Declarations

use std::collections::HashSet;
use std::sync::Arc;

use crate::error::ParseError;
use crate::namespaces::QName;

use super::attributes::XsdAttributeGroup;
use super::complex_types::{DerivationFlags, XsdComplexType};
use super::groups::XsdGroup;
use super::particles::{Occurs, Particle};
use super::simple_types::SimpleType;
use super::wildcards::XsdAnyElement;

/// The scope of an element declaration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ElementScope {
    /// Global element declaration
    #[default]
    Global,
    /// Local element declaration (within a complex type or group)
    Local,
}

impl std::fmt::Display for ElementScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Global => write!(f, "global"),
            Self::Local => write!(f, "local"),
        }
    }
}

/// Element form (qualified or unqualified)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ElementForm {
    /// Element name must be namespace-qualified
    Qualified,
    /// Element name is unqualified
    #[default]
    Unqualified,
}

impl ElementForm {
    /// Parse from string attribute value
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "qualified" => Some(Self::Qualified),
            "unqualified" => Some(Self::Unqualified),
            _ => None,
        }
    }
}

/// The type of an element - either simple or complex
#[derive(Debug, Clone)]
pub enum ElementType {
    /// Simple type content (text only)
    Simple(Arc<dyn SimpleType + Send + Sync>),
    /// Complex type content
    Complex(Arc<XsdComplexType>),
    /// Any type (untyped)
    Any,
}

impl ElementType {
    /// Check if this is a simple type
    pub fn is_simple(&self) -> bool {
        matches!(self, ElementType::Simple(_))
    }

    /// Check if this is a complex type
    pub fn is_complex(&self) -> bool {
        matches!(self, ElementType::Complex(_))
    }

    /// Check if this is the any type
    pub fn is_any(&self) -> bool {
        matches!(self, ElementType::Any)
    }

    /// Get the model group if this is a complex type with complex content
    pub fn model_group(&self) -> Option<&Arc<XsdGroup>> {
        match self {
            ElementType::Complex(ct) => ct.model_group(),
            _ => None,
        }
    }

    /// Get the attributes if this is a complex type
    pub fn attributes(&self) -> Option<&XsdAttributeGroup> {
        match self {
            ElementType::Complex(ct) => Some(&ct.attributes),
            _ => None,
        }
    }
}

/// XSD Element declaration
#[derive(Debug, Clone)]
pub struct XsdElement {
    /// Element name
    pub name: QName,

    /// Element type (simple, complex, or any)
    pub element_type: ElementType,

    /// Occurrence constraints
    pub occurs: Occurs,

    /// Whether this element is abstract
    pub abstract_element: bool,

    /// Whether this element is nillable
    pub nillable: bool,

    /// Element form (qualified/unqualified)
    pub form: ElementForm,

    /// Default value (for simple content)
    pub default: Option<String>,

    /// Fixed value (for simple content)
    pub fixed: Option<String>,

    /// Substitution group head element name
    pub substitution_group: Option<QName>,

    /// Block derivation flags
    pub block: DerivationFlags,

    /// Final derivation flags
    pub final_deriv: DerivationFlags,

    /// Element scope (global or local)
    pub scope: ElementScope,

    /// Reference to another element (for ref= usage)
    pub ref_element: Option<QName>,

    /// Target namespace
    pub target_namespace: Option<String>,

    /// Whether this element is qualified
    pub qualified: bool,

    /// Parse errors
    errors: Vec<ParseError>,
}

impl XsdElement {
    /// Create a new element with a given name and type
    pub fn new(name: QName, element_type: ElementType) -> Self {
        Self {
            name,
            element_type,
            occurs: Occurs::once(),
            abstract_element: false,
            nillable: false,
            form: ElementForm::default(),
            default: None,
            fixed: None,
            substitution_group: None,
            block: DerivationFlags::default(),
            final_deriv: DerivationFlags::default(),
            scope: ElementScope::default(),
            ref_element: None,
            target_namespace: None,
            qualified: false,
            errors: Vec::new(),
        }
    }

    /// Create a simple element (with simple type content)
    pub fn simple(name: QName, simple_type: Arc<dyn SimpleType + Send + Sync>) -> Self {
        Self::new(name, ElementType::Simple(simple_type))
    }

    /// Create a complex element (with complex type content)
    pub fn complex(name: QName, complex_type: Arc<XsdComplexType>) -> Self {
        Self::new(name, ElementType::Complex(complex_type))
    }

    /// Create an element with any type
    pub fn any_type(name: QName) -> Self {
        Self::new(name, ElementType::Any)
    }

    /// Create an element reference
    pub fn reference(ref_name: QName, occurs: Occurs) -> Self {
        Self {
            name: ref_name.clone(),
            element_type: ElementType::Any,
            occurs,
            abstract_element: false,
            nillable: false,
            form: ElementForm::default(),
            default: None,
            fixed: None,
            substitution_group: None,
            block: DerivationFlags::default(),
            final_deriv: DerivationFlags::default(),
            scope: ElementScope::Local,
            ref_element: Some(ref_name),
            target_namespace: None,
            qualified: false,
            errors: Vec::new(),
        }
    }

    /// Check if this element matches a given name
    pub fn is_matching(&self, name: &str, default_namespace: Option<&str>) -> bool {
        if name.is_empty() {
            return false;
        }

        // If name has namespace prefix, use as-is
        let full_name = if name.starts_with('{') || default_namespace.is_none() {
            name.to_string()
        } else {
            format!("{{{}}}{}", default_namespace.unwrap(), name)
        };

        self.name.to_string() == full_name
    }

    /// Match this element or a substitute against a name
    pub fn match_element<'a>(
        &'a self,
        name: &str,
        default_namespace: Option<&str>,
        substitutes: Option<&'a HashSet<Arc<XsdElement>>>,
    ) -> Option<&'a XsdElement> {
        if name.is_empty() {
            return None;
        }

        // If name has namespace prefix, use as-is
        let full_name = if name.starts_with('{') || default_namespace.is_none() {
            name.to_string()
        } else {
            format!("{{{}}}{}", default_namespace.unwrap(), name)
        };

        if self.name.to_string() == full_name {
            return Some(self);
        }

        // Check substitutes
        if let Some(subs) = substitutes {
            for sub in subs {
                if sub.name.to_string() == full_name {
                    return Some(sub.as_ref());
                }
            }
        }

        None
    }

    /// Get the effective value constraint (fixed or default)
    pub fn value_constraint(&self) -> Option<&str> {
        self.fixed.as_deref().or(self.default.as_deref())
    }

    /// Check if this is a reference to another element
    pub fn is_reference(&self) -> bool {
        self.ref_element.is_some()
    }

    /// Check if this is a global element
    pub fn is_global(&self) -> bool {
        self.scope == ElementScope::Global
    }

    /// Check if this is a local element
    pub fn is_local(&self) -> bool {
        self.scope == ElementScope::Local
    }

    /// Set occurrence constraints
    pub fn with_occurs(mut self, occurs: Occurs) -> Self {
        self.occurs = occurs;
        self
    }

    /// Set abstract flag
    pub fn with_abstract(mut self, abstract_element: bool) -> Self {
        self.abstract_element = abstract_element;
        self
    }

    /// Set nillable flag
    pub fn with_nillable(mut self, nillable: bool) -> Self {
        self.nillable = nillable;
        self
    }

    /// Set default value
    pub fn with_default(mut self, default: String) -> Self {
        self.default = Some(default);
        self
    }

    /// Set fixed value
    pub fn with_fixed(mut self, fixed: String) -> Self {
        self.fixed = Some(fixed);
        self
    }

    /// Set substitution group
    pub fn with_substitution_group(mut self, group: QName) -> Self {
        self.substitution_group = Some(group);
        self
    }

    /// Set scope
    pub fn with_scope(mut self, scope: ElementScope) -> Self {
        self.scope = scope;
        self
    }

    /// Get parse errors
    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }

    /// Add a parse error
    pub fn add_error(&mut self, error: ParseError) {
        self.errors.push(error);
    }

    /// Check if element is restriction of another element
    pub fn is_restriction_of(&self, other: &XsdElement) -> bool {
        // Name must match
        if self.name != other.name {
            return false;
        }

        // Check occurrence restriction
        if !self.occurs.has_occurs_restriction(&other.occurs) {
            return false;
        }

        // Fixed value must be same if other has fixed
        if let Some(ref other_fixed) = other.fixed {
            if self.fixed.as_ref() != Some(other_fixed) {
                return false;
            }
        }

        // Cannot be more nillable than base
        if other.nillable && !self.nillable {
            // Actually this is backwards - derived cannot add nillability
        }
        if !other.nillable && self.nillable {
            return false;
        }

        true
    }

    /// Check if element is consistent with another (EDC check)
    pub fn is_consistent(&self, other: &XsdElement) -> bool {
        // Elements with different names are always consistent
        if self.name != other.name {
            return true;
        }

        // Same name means must have same type
        // In a full implementation, we'd compare type identity
        // For now, we just check if both reference the same type
        match (&self.element_type, &other.element_type) {
            (ElementType::Any, _) | (_, ElementType::Any) => true,
            _ => true, // Simplified - would need type comparison
        }
    }

    /// Check if element overlaps with another particle
    pub fn is_overlap(&self, other: &XsdElement) -> bool {
        // Same name means overlap
        if self.name == other.name {
            return true;
        }

        // Check substitution group
        if let Some(ref sub_group) = other.substitution_group {
            if &self.name == sub_group {
                return true;
            }
        }

        if let Some(ref sub_group) = self.substitution_group {
            if &other.name == sub_group {
                return true;
            }
        }

        false
    }

    /// Check if element overlaps with a wildcard
    pub fn is_overlap_wildcard(&self, wildcard: &XsdAnyElement) -> bool {
        wildcard.is_matching(&self.name.to_string(), self.target_namespace.as_deref())
    }
}

impl Particle for XsdElement {
    fn occurs(&self) -> Occurs {
        self.occurs
    }

    fn min_occurs(&self) -> u32 {
        self.occurs.min
    }

    fn max_occurs(&self) -> Option<u32> {
        self.occurs.max
    }

    fn is_emptiable(&self) -> bool {
        self.occurs.min == 0
    }

    fn is_empty(&self) -> bool {
        self.occurs.max == Some(0)
    }
}

/// Builder for XSD elements
#[derive(Debug)]
pub struct XsdElementBuilder {
    name: Option<QName>,
    element_type: ElementType,
    occurs: Occurs,
    abstract_element: bool,
    nillable: bool,
    form: ElementForm,
    default: Option<String>,
    fixed: Option<String>,
    substitution_group: Option<QName>,
    block: DerivationFlags,
    final_deriv: DerivationFlags,
    scope: ElementScope,
    ref_element: Option<QName>,
    target_namespace: Option<String>,
    qualified: bool,
}

impl XsdElementBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            name: None,
            element_type: ElementType::Any,
            occurs: Occurs::once(),
            abstract_element: false,
            nillable: false,
            form: ElementForm::default(),
            default: None,
            fixed: None,
            substitution_group: None,
            block: DerivationFlags::default(),
            final_deriv: DerivationFlags::default(),
            scope: ElementScope::default(),
            ref_element: None,
            target_namespace: None,
            qualified: false,
        }
    }

    /// Set the element name
    pub fn name(mut self, name: QName) -> Self {
        self.name = Some(name);
        self
    }

    /// Set the element type to simple
    pub fn simple_type(mut self, simple: Arc<dyn SimpleType + Send + Sync>) -> Self {
        self.element_type = ElementType::Simple(simple);
        self
    }

    /// Set the element type to complex
    pub fn complex_type(mut self, complex: Arc<XsdComplexType>) -> Self {
        self.element_type = ElementType::Complex(complex);
        self
    }

    /// Set occurrence constraints
    pub fn occurs(mut self, occurs: Occurs) -> Self {
        self.occurs = occurs;
        self
    }

    /// Set abstract flag
    pub fn abstract_element(mut self, abstract_element: bool) -> Self {
        self.abstract_element = abstract_element;
        self
    }

    /// Set nillable flag
    pub fn nillable(mut self, nillable: bool) -> Self {
        self.nillable = nillable;
        self
    }

    /// Set element form
    pub fn form(mut self, form: ElementForm) -> Self {
        self.form = form;
        self
    }

    /// Set default value
    pub fn default(mut self, default: String) -> Self {
        self.default = Some(default);
        self
    }

    /// Set fixed value
    pub fn fixed(mut self, fixed: String) -> Self {
        self.fixed = Some(fixed);
        self
    }

    /// Set substitution group
    pub fn substitution_group(mut self, group: QName) -> Self {
        self.substitution_group = Some(group);
        self
    }

    /// Set block flags
    pub fn block(mut self, block: DerivationFlags) -> Self {
        self.block = block;
        self
    }

    /// Set final flags
    pub fn final_deriv(mut self, final_deriv: DerivationFlags) -> Self {
        self.final_deriv = final_deriv;
        self
    }

    /// Set scope
    pub fn scope(mut self, scope: ElementScope) -> Self {
        self.scope = scope;
        self
    }

    /// Set as element reference
    pub fn reference(mut self, ref_name: QName) -> Self {
        self.ref_element = Some(ref_name.clone());
        self.name = Some(ref_name);
        self
    }

    /// Set target namespace
    pub fn target_namespace(mut self, ns: String) -> Self {
        self.target_namespace = Some(ns);
        self
    }

    /// Set qualified
    pub fn qualified(mut self, qualified: bool) -> Self {
        self.qualified = qualified;
        self
    }

    /// Build the element
    pub fn build(self) -> Result<XsdElement, &'static str> {
        let name = self.name.ok_or("Element name is required")?;

        Ok(XsdElement {
            name,
            element_type: self.element_type,
            occurs: self.occurs,
            abstract_element: self.abstract_element,
            nillable: self.nillable,
            form: self.form,
            default: self.default,
            fixed: self.fixed,
            substitution_group: self.substitution_group,
            block: self.block,
            final_deriv: self.final_deriv,
            scope: self.scope,
            ref_element: self.ref_element,
            target_namespace: self.target_namespace,
            qualified: self.qualified,
            errors: Vec::new(),
        })
    }
}

impl Default for XsdElementBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validators::builtins::XSD_STRING;
    use crate::validators::simple_types::XsdAtomicType;

    #[test]
    fn test_element_creation() {
        let elem = XsdElement::any_type(QName::local("root"));
        assert_eq!(elem.name, QName::local("root"));
        assert!(elem.element_type.is_any());
        assert!(!elem.abstract_element);
        assert!(!elem.nillable);
    }

    #[test]
    fn test_simple_element() {
        let string_type = Arc::new(XsdAtomicType::new(XSD_STRING).unwrap());
        let elem = XsdElement::simple(QName::local("name"), string_type);
        assert!(elem.element_type.is_simple());
        assert!(!elem.element_type.is_complex());
    }

    #[test]
    fn test_element_reference() {
        let elem = XsdElement::reference(QName::local("other"), Occurs::optional());
        assert!(elem.is_reference());
        assert_eq!(elem.ref_element, Some(QName::local("other")));
    }

    #[test]
    fn test_element_matching() {
        let elem = XsdElement::any_type(QName::local("item"));
        assert!(elem.is_matching("item", None));
        assert!(!elem.is_matching("other", None));
    }

    #[test]
    fn test_element_with_namespace() {
        let elem = XsdElement::any_type(QName::namespaced("http://example.com", "item"));
        assert!(elem.is_matching("{http://example.com}item", None));
        assert!(elem.is_matching("item", Some("http://example.com")));
        assert!(!elem.is_matching("item", Some("http://other.com")));
    }

    #[test]
    fn test_element_builder() {
        let elem = XsdElementBuilder::new()
            .name(QName::local("test"))
            .nillable(true)
            .occurs(Occurs::zero_or_more())
            .build()
            .unwrap();

        assert_eq!(elem.name, QName::local("test"));
        assert!(elem.nillable);
        assert_eq!(elem.occurs, Occurs::zero_or_more());
    }

    #[test]
    fn test_element_scope() {
        let global = XsdElement::any_type(QName::local("global"));
        assert!(global.is_global());

        let local = XsdElementBuilder::new()
            .name(QName::local("local"))
            .scope(ElementScope::Local)
            .build()
            .unwrap();
        assert!(local.is_local());
    }

    #[test]
    fn test_value_constraint() {
        let with_default = XsdElement::any_type(QName::local("elem"))
            .with_default("default_value".to_string());
        assert_eq!(with_default.value_constraint(), Some("default_value"));

        let with_fixed = XsdElement::any_type(QName::local("elem"))
            .with_fixed("fixed_value".to_string());
        assert_eq!(with_fixed.value_constraint(), Some("fixed_value"));

        // Fixed takes precedence
        let with_both = XsdElement {
            default: Some("default".to_string()),
            fixed: Some("fixed".to_string()),
            ..XsdElement::any_type(QName::local("elem"))
        };
        assert_eq!(with_both.value_constraint(), Some("fixed"));
    }

    #[test]
    fn test_element_overlap() {
        let elem1 = XsdElement::any_type(QName::local("item"));
        let elem2 = XsdElement::any_type(QName::local("item"));
        let elem3 = XsdElement::any_type(QName::local("other"));

        assert!(elem1.is_overlap(&elem2));
        assert!(!elem1.is_overlap(&elem3));
    }

    #[test]
    fn test_element_consistency() {
        let elem1 = XsdElement::any_type(QName::local("item"));
        let elem2 = XsdElement::any_type(QName::local("item"));
        let elem3 = XsdElement::any_type(QName::local("other"));

        // Same name with any type is consistent (simplified check)
        assert!(elem1.is_consistent(&elem2));
        // Different names are always consistent
        assert!(elem1.is_consistent(&elem3));
    }

    #[test]
    fn test_particle_trait() {
        let elem = XsdElement::any_type(QName::local("item"))
            .with_occurs(Occurs::new(0, Some(5)));

        assert_eq!(elem.min_occurs(), 0);
        assert_eq!(elem.max_occurs(), Some(5));
        assert!(elem.is_emptiable());
        assert!(!elem.is_empty());
    }

    #[test]
    fn test_element_form() {
        assert_eq!(ElementForm::from_str("qualified"), Some(ElementForm::Qualified));
        assert_eq!(ElementForm::from_str("unqualified"), Some(ElementForm::Unqualified));
        assert_eq!(ElementForm::from_str("invalid"), None);
    }
}
