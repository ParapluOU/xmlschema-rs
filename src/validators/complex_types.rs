//! XSD Complex Type Validators
//!
//! This module implements complex type definitions for XSD schemas.
//! Complex types can have element content (model groups), simple content,
//! or mixed content with both text and elements.
//!
//! Reference: https://www.w3.org/TR/xmlschema11-1/#Complex_Type_Definitions

use std::sync::Arc;

use crate::error::ParseError;
use crate::namespaces::QName;

use super::attributes::XsdAttributeGroup;
use super::groups::{ModelType, XsdGroup};
use super::simple_types::SimpleType;
use super::wildcards::XsdAnyElement;

/// Derivation method for complex types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DerivationMethod {
    /// Type derived by restriction
    #[default]
    Restriction,
    /// Type derived by extension
    Extension,
}

impl DerivationMethod {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "restriction" => Some(Self::Restriction),
            "extension" => Some(Self::Extension),
            _ => None,
        }
    }
}

impl std::fmt::Display for DerivationMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Restriction => write!(f, "restriction"),
            Self::Extension => write!(f, "extension"),
        }
    }
}

/// Content type label for complex types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentTypeLabel {
    /// No content (empty element)
    Empty,
    /// Simple content (text only)
    Simple,
    /// Mixed content (text and elements)
    Mixed,
    /// Element-only content
    ElementOnly,
}

impl std::fmt::Display for ContentTypeLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "empty"),
            Self::Simple => write!(f, "simple"),
            Self::Mixed => write!(f, "mixed"),
            Self::ElementOnly => write!(f, "element-only"),
        }
    }
}

/// The content of a complex type - either a model group or simple type
#[derive(Debug, Clone)]
pub enum ComplexContent {
    /// Content is a model group (sequence, choice, all)
    Group(Arc<XsdGroup>),
    /// Content is a simple type (for simpleContent)
    Simple(Arc<dyn SimpleType + Send + Sync>),
}

impl ComplexContent {
    /// Check if content is empty
    /// Note: Simple content is never considered "empty" - it represents text content
    pub fn is_empty(&self) -> bool {
        match self {
            ComplexContent::Group(group) => group.is_empty(),
            ComplexContent::Simple(_) => false, // Simple content has text
        }
    }

    /// Check if content is emptiable
    pub fn is_emptiable(&self) -> bool {
        match self {
            ComplexContent::Group(group) => group.is_emptiable(),
            ComplexContent::Simple(_) => false, // Simple content must have a value
        }
    }

    /// Get as model group if applicable
    pub fn as_group(&self) -> Option<&Arc<XsdGroup>> {
        match self {
            ComplexContent::Group(group) => Some(group),
            ComplexContent::Simple(_) => None,
        }
    }

    /// Get as simple type if applicable
    pub fn as_simple(&self) -> Option<&Arc<dyn SimpleType + Send + Sync>> {
        match self {
            ComplexContent::Group(_) => None,
            ComplexContent::Simple(simple) => Some(simple),
        }
    }
}

/// Open content mode (XSD 1.1)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OpenContentMode {
    /// No open content
    #[default]
    None,
    /// Interleave wildcard with model
    Interleave,
    /// Suffix wildcard after model
    Suffix,
}

impl OpenContentMode {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "none" => Some(Self::None),
            "interleave" => Some(Self::Interleave),
            "suffix" => Some(Self::Suffix),
            _ => None,
        }
    }
}

/// Open content for complex types (XSD 1.1)
#[derive(Debug, Clone)]
pub struct XsdOpenContent {
    /// Open content mode
    pub mode: OpenContentMode,
    /// The wildcard for open content
    pub any_element: Option<Arc<XsdAnyElement>>,
    /// Whether this applies to empty content
    pub applies_to_empty: bool,
}

impl Default for XsdOpenContent {
    fn default() -> Self {
        Self {
            mode: OpenContentMode::None,
            any_element: None,
            applies_to_empty: false,
        }
    }
}

/// Block/final derivation flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DerivationFlags {
    /// Block/finalize restriction
    pub restriction: bool,
    /// Block/finalize extension
    pub extension: bool,
}

impl DerivationFlags {
    /// All derivations blocked/finalized
    pub fn all() -> Self {
        Self {
            restriction: true,
            extension: true,
        }
    }

    /// Parse from attribute value
    pub fn from_attr(value: &str) -> Self {
        let mut flags = DerivationFlags::default();
        for token in value.split_whitespace() {
            match token {
                "#all" => return Self::all(),
                "restriction" => flags.restriction = true,
                "extension" => flags.extension = true,
                _ => {}
            }
        }
        flags
    }

    /// Check if a derivation method is blocked
    pub fn is_blocked(&self, method: DerivationMethod) -> bool {
        match method {
            DerivationMethod::Restriction => self.restriction,
            DerivationMethod::Extension => self.extension,
        }
    }
}

/// XSD Complex Type definition
#[derive(Debug, Clone)]
pub struct XsdComplexType {
    /// Type name (None for anonymous types)
    pub name: Option<QName>,

    /// Content model (group or simple type)
    pub content: ComplexContent,

    /// Attribute group for this type
    pub attributes: XsdAttributeGroup,

    /// Base type (for derived types)
    pub base_type: Option<QName>,

    /// Derivation method
    pub derivation: Option<DerivationMethod>,

    /// Whether this is a mixed content type
    pub mixed: bool,

    /// Whether this type is abstract
    pub abstract_type: bool,

    /// Block derivation flags
    pub block: DerivationFlags,

    /// Final derivation flags
    pub final_deriv: DerivationFlags,

    /// Open content (XSD 1.1)
    pub open_content: Option<XsdOpenContent>,

    /// Parse errors
    errors: Vec<ParseError>,
}

impl XsdComplexType {
    /// Create a new complex type with a model group content
    pub fn new(name: Option<QName>, content: Arc<XsdGroup>) -> Self {
        Self {
            name,
            content: ComplexContent::Group(content),
            attributes: XsdAttributeGroup::anonymous(),
            base_type: None,
            derivation: None,
            mixed: false,
            abstract_type: false,
            block: DerivationFlags::default(),
            final_deriv: DerivationFlags::default(),
            open_content: None,
            errors: Vec::new(),
        }
    }

    /// Create a complex type with simple content
    pub fn with_simple_content(name: Option<QName>, content: Arc<dyn SimpleType + Send + Sync>) -> Self {
        Self {
            name,
            content: ComplexContent::Simple(content),
            attributes: XsdAttributeGroup::anonymous(),
            base_type: None,
            derivation: None,
            mixed: false,
            abstract_type: false,
            block: DerivationFlags::default(),
            final_deriv: DerivationFlags::default(),
            open_content: None,
            errors: Vec::new(),
        }
    }

    /// Create an empty complex type
    pub fn empty(name: Option<QName>) -> Self {
        Self {
            name,
            content: ComplexContent::Group(Arc::new(XsdGroup::new(ModelType::Sequence))),
            attributes: XsdAttributeGroup::anonymous(),
            base_type: None,
            derivation: None,
            mixed: false,
            abstract_type: false,
            block: DerivationFlags::default(),
            final_deriv: DerivationFlags::default(),
            open_content: None,
            errors: Vec::new(),
        }
    }

    /// Get the model group content, if any
    pub fn model_group(&self) -> Option<&Arc<XsdGroup>> {
        self.content.as_group()
    }

    /// Get the simple type content, if any
    pub fn simple_type(&self) -> Option<&Arc<dyn SimpleType + Send + Sync>> {
        self.content.as_simple()
    }

    /// Get the content type label
    pub fn content_type_label(&self) -> ContentTypeLabel {
        if self.is_empty() {
            ContentTypeLabel::Empty
        } else if self.content.as_simple().is_some() {
            ContentTypeLabel::Simple
        } else if self.mixed {
            ContentTypeLabel::Mixed
        } else {
            ContentTypeLabel::ElementOnly
        }
    }

    /// Check if this is a simple type (always false for complex types)
    pub fn is_simple(&self) -> bool {
        false
    }

    /// Check if this is a complex type (always true)
    pub fn is_complex(&self) -> bool {
        true
    }

    /// Check if content is empty
    pub fn is_empty(&self) -> bool {
        if let Some(ref open) = self.open_content {
            if open.mode != OpenContentMode::None {
                return false;
            }
        }
        self.content.is_empty()
    }

    /// Check if content is emptiable
    pub fn is_emptiable(&self) -> bool {
        self.content.is_emptiable()
    }

    /// Check if this type has simple content
    pub fn has_simple_content(&self) -> bool {
        match &self.content {
            ComplexContent::Simple(_) => true, // Simple content always has content
            ComplexContent::Group(group) => {
                if group.len() > 0 || group.mixed {
                    false
                } else {
                    // Check base type for simple content
                    self.base_type.is_some()
                }
            }
        }
    }

    /// Check if this type has complex content
    pub fn has_complex_content(&self) -> bool {
        match &self.content {
            ComplexContent::Simple(_) => false,
            ComplexContent::Group(group) => {
                if let Some(ref open) = self.open_content {
                    if open.mode != OpenContentMode::None {
                        return true;
                    }
                }
                !group.is_empty()
            }
        }
    }

    /// Check if this type has mixed content
    pub fn has_mixed_content(&self) -> bool {
        match &self.content {
            ComplexContent::Simple(_) => false,
            ComplexContent::Group(group) => !group.is_empty() && group.mixed,
        }
    }

    /// Check if this type is element-only
    pub fn is_element_only(&self) -> bool {
        match &self.content {
            ComplexContent::Simple(_) => false,
            ComplexContent::Group(group) => !group.is_empty() && !group.mixed,
        }
    }

    /// Check if a derivation method is blocked
    pub fn is_derivation_blocked(&self, method: DerivationMethod) -> bool {
        self.block.is_blocked(method)
    }

    /// Check if a derivation method is finalized
    pub fn is_derivation_final(&self, method: DerivationMethod) -> bool {
        self.final_deriv.is_blocked(method)
    }

    /// Set the base type
    pub fn set_base_type(&mut self, base: QName, method: DerivationMethod) {
        self.base_type = Some(base);
        self.derivation = Some(method);
    }

    /// Set mixed content mode
    pub fn set_mixed(&mut self, mixed: bool) {
        self.mixed = mixed;
        if let ComplexContent::Group(ref mut group) = self.content {
            // Need to get mutable access - clone and modify
            let mut new_group = (**group).clone();
            new_group.mixed = mixed;
            *group = Arc::new(new_group);
        }
    }

    /// Add an attribute to this type
    pub fn add_attribute(&mut self, attr: Arc<super::attributes::XsdAttribute>) {
        let _ = self.attributes.add_attribute(attr);
    }

    /// Get parse errors
    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }

    /// Add a parse error
    pub fn add_error(&mut self, error: ParseError) {
        self.errors.push(error);
    }

    /// Check if type is derived from another
    pub fn is_derived_from(&self, other_name: &QName) -> bool {
        if let Some(ref base) = self.base_type {
            if base == other_name {
                return true;
            }
        }
        false
    }
}

/// Builder for complex types
#[derive(Debug)]
pub struct ComplexTypeBuilder {
    name: Option<QName>,
    content: Option<ComplexContent>,
    attributes: XsdAttributeGroup,
    base_type: Option<QName>,
    derivation: Option<DerivationMethod>,
    mixed: bool,
    abstract_type: bool,
    block: DerivationFlags,
    final_deriv: DerivationFlags,
    open_content: Option<XsdOpenContent>,
}

impl ComplexTypeBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            name: None,
            content: None,
            attributes: XsdAttributeGroup::anonymous(),
            base_type: None,
            derivation: None,
            mixed: false,
            abstract_type: false,
            block: DerivationFlags::default(),
            final_deriv: DerivationFlags::default(),
            open_content: None,
        }
    }

    /// Set the type name
    pub fn name(mut self, name: QName) -> Self {
        self.name = Some(name);
        self
    }

    /// Set the content as a model group
    pub fn content_group(mut self, group: Arc<XsdGroup>) -> Self {
        self.content = Some(ComplexContent::Group(group));
        self
    }

    /// Set the content as a simple type
    pub fn content_simple(mut self, simple: Arc<dyn SimpleType + Send + Sync>) -> Self {
        self.content = Some(ComplexContent::Simple(simple));
        self
    }

    /// Set the base type
    pub fn base(mut self, base: QName, method: DerivationMethod) -> Self {
        self.base_type = Some(base);
        self.derivation = Some(method);
        self
    }

    /// Set mixed mode
    pub fn mixed(mut self, mixed: bool) -> Self {
        self.mixed = mixed;
        self
    }

    /// Set abstract flag
    pub fn abstract_type(mut self, abstract_type: bool) -> Self {
        self.abstract_type = abstract_type;
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

    /// Add an attribute
    pub fn attribute(mut self, attr: Arc<super::attributes::XsdAttribute>) -> Self {
        let _ = self.attributes.add_attribute(attr);
        self
    }

    /// Build the complex type
    pub fn build(self) -> XsdComplexType {
        let content = self.content.unwrap_or_else(|| {
            ComplexContent::Group(Arc::new(XsdGroup::new(ModelType::Sequence)))
        });

        XsdComplexType {
            name: self.name,
            content,
            attributes: self.attributes,
            base_type: self.base_type,
            derivation: self.derivation,
            mixed: self.mixed,
            abstract_type: self.abstract_type,
            block: self.block,
            final_deriv: self.final_deriv,
            open_content: self.open_content,
            errors: Vec::new(),
        }
    }
}

impl Default for ComplexTypeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validators::builtins::XSD_STRING;
    use crate::validators::particles::Occurs;
    use crate::validators::simple_types::XsdAtomicType;

    #[test]
    fn test_empty_complex_type() {
        let ct = XsdComplexType::empty(Some(QName::local("EmptyType")));
        assert!(ct.is_empty());
        assert!(!ct.is_simple());
        assert!(ct.is_complex());
        assert_eq!(ct.content_type_label(), ContentTypeLabel::Empty);
    }

    #[test]
    fn test_complex_type_with_group() {
        let mut group = XsdGroup::new(ModelType::Sequence);
        group.add_element(QName::local("child"), Occurs::once());

        let ct = XsdComplexType::new(
            Some(QName::local("MyType")),
            Arc::new(group),
        );

        assert!(!ct.is_empty());
        assert!(ct.has_complex_content());
        assert!(!ct.has_simple_content());
        assert!(ct.is_element_only());
        assert_eq!(ct.content_type_label(), ContentTypeLabel::ElementOnly);
    }

    #[test]
    fn test_complex_type_mixed() {
        let mut group = XsdGroup::new(ModelType::Sequence);
        group.mixed = true;
        group.add_element(QName::local("child"), Occurs::once());

        let mut ct = XsdComplexType::new(
            Some(QName::local("MixedType")),
            Arc::new(group),
        );
        ct.mixed = true;

        assert!(ct.has_mixed_content());
        assert!(!ct.is_element_only());
        assert_eq!(ct.content_type_label(), ContentTypeLabel::Mixed);
    }

    #[test]
    fn test_complex_type_with_simple_content() {
        let simple = XsdAtomicType::new(XSD_STRING).unwrap();

        let ct = XsdComplexType::with_simple_content(
            Some(QName::local("SimpleContentType")),
            Arc::new(simple),
        );

        assert!(!ct.is_empty());
        assert!(ct.has_simple_content());
        assert!(!ct.has_complex_content());
        assert_eq!(ct.content_type_label(), ContentTypeLabel::Simple);
    }

    #[test]
    fn test_derivation_method() {
        assert_eq!(DerivationMethod::from_str("restriction"), Some(DerivationMethod::Restriction));
        assert_eq!(DerivationMethod::from_str("extension"), Some(DerivationMethod::Extension));
        assert_eq!(DerivationMethod::from_str("invalid"), None);
    }

    #[test]
    fn test_derivation_flags() {
        let flags = DerivationFlags::from_attr("restriction extension");
        assert!(flags.restriction);
        assert!(flags.extension);

        let all = DerivationFlags::from_attr("#all");
        assert!(all.restriction);
        assert!(all.extension);

        let none = DerivationFlags::default();
        assert!(!none.restriction);
        assert!(!none.extension);
    }

    #[test]
    fn test_complex_type_builder() {
        let group = Arc::new(XsdGroup::new(ModelType::Sequence));

        let ct = ComplexTypeBuilder::new()
            .name(QName::local("BuiltType"))
            .content_group(group)
            .mixed(true)
            .abstract_type(false)
            .build();

        assert_eq!(ct.name, Some(QName::local("BuiltType")));
        assert!(ct.mixed);
        assert!(!ct.abstract_type);
    }

    #[test]
    fn test_base_type() {
        let group = Arc::new(XsdGroup::new(ModelType::Sequence));
        let mut ct = XsdComplexType::new(
            Some(QName::local("DerivedType")),
            group,
        );

        ct.set_base_type(
            QName::local("BaseType"),
            DerivationMethod::Extension,
        );

        assert_eq!(ct.base_type, Some(QName::local("BaseType")));
        assert_eq!(ct.derivation, Some(DerivationMethod::Extension));
        assert!(ct.is_derived_from(&QName::local("BaseType")));
        assert!(!ct.is_derived_from(&QName::local("OtherType")));
    }

    #[test]
    fn test_open_content_mode() {
        assert_eq!(OpenContentMode::from_str("none"), Some(OpenContentMode::None));
        assert_eq!(OpenContentMode::from_str("interleave"), Some(OpenContentMode::Interleave));
        assert_eq!(OpenContentMode::from_str("suffix"), Some(OpenContentMode::Suffix));
        assert_eq!(OpenContentMode::from_str("invalid"), None);
    }

    #[test]
    fn test_content_type_label() {
        assert_eq!(ContentTypeLabel::Empty.to_string(), "empty");
        assert_eq!(ContentTypeLabel::Simple.to_string(), "simple");
        assert_eq!(ContentTypeLabel::Mixed.to_string(), "mixed");
        assert_eq!(ContentTypeLabel::ElementOnly.to_string(), "element-only");
    }
}
