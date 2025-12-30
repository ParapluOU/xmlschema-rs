//! XSD attribute validators
//!
//! This module implements validators for XSD attribute declarations
//! and attribute groups.

use crate::error::{ParseError, Result, ValidationError};
use crate::namespaces::QName;
use std::collections::HashMap;
use std::sync::Arc;

use super::base::{
    AttributeValidator, TypeValidator, ValidationMode, ValidationStatus, Validator,
};
use super::builtins::XsdValue;
use super::simple_types::SimpleType;
use super::wildcards::XsdAnyAttribute;

/// Attribute use mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AttributeUse {
    /// Attribute is optional (default)
    #[default]
    Optional,
    /// Attribute is required
    Required,
    /// Attribute is prohibited
    Prohibited,
}

impl AttributeUse {
    /// Parse from string value
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "optional" => Ok(AttributeUse::Optional),
            "required" => Ok(AttributeUse::Required),
            "prohibited" => Ok(AttributeUse::Prohibited),
            _ => Err(crate::error::Error::Value(format!(
                "Invalid attribute use value: '{}'. Must be 'optional', 'required', or 'prohibited'",
                s
            ))),
        }
    }

    /// Get the use as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            AttributeUse::Optional => "optional",
            AttributeUse::Required => "required",
            AttributeUse::Prohibited => "prohibited",
        }
    }
}

impl std::fmt::Display for AttributeUse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Form for attribute declarations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AttributeForm {
    /// Attribute name is not qualified with namespace
    #[default]
    Unqualified,
    /// Attribute name is qualified with namespace
    Qualified,
}

impl AttributeForm {
    /// Parse from string value
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "unqualified" => Ok(AttributeForm::Unqualified),
            "qualified" => Ok(AttributeForm::Qualified),
            _ => Err(crate::error::Error::Value(format!(
                "Invalid attribute form value: '{}'. Must be 'unqualified' or 'qualified'",
                s
            ))),
        }
    }

    /// Get the form as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            AttributeForm::Unqualified => "unqualified",
            AttributeForm::Qualified => "qualified",
        }
    }
}

/// Scope of an attribute declaration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeScope {
    /// Global attribute declaration
    Global,
    /// Local attribute declaration
    Local,
}

/// XSD attribute declaration
///
/// Represents an XSD attribute declaration which can appear as:
/// - A global declaration at schema level
/// - A local declaration inside a complex type
/// - A reference to a global declaration
#[derive(Debug)]
pub struct XsdAttribute {
    /// Attribute name
    name: QName,
    /// Attribute type (must be a simple type)
    attr_type: Option<Arc<dyn SimpleType + Send + Sync>>,
    /// Type name reference (for forward reference resolution)
    pub type_name: Option<QName>,
    /// Usage mode
    use_mode: AttributeUse,
    /// Form (qualified/unqualified)
    form: AttributeForm,
    /// Default value
    default: Option<String>,
    /// Fixed value
    fixed: Option<String>,
    /// Whether this is inheritable (XSD 1.1)
    inheritable: bool,
    /// Reference to another attribute (if this is a ref)
    reference: Option<Arc<XsdAttribute>>,
    /// Build errors
    errors: Vec<ParseError>,
    /// Whether fully built
    built: bool,
}

impl XsdAttribute {
    /// Create a new attribute declaration
    pub fn new(name: QName) -> Self {
        Self {
            name,
            attr_type: None,
            type_name: None,
            use_mode: AttributeUse::Optional,
            form: AttributeForm::Unqualified,
            default: None,
            fixed: None,
            inheritable: false,
            reference: None,
            errors: Vec::new(),
            built: false,
        }
    }

    /// Create an attribute with a type
    pub fn with_type(name: QName, attr_type: Arc<dyn SimpleType + Send + Sync>) -> Self {
        Self {
            name,
            attr_type: Some(attr_type),
            type_name: None,
            use_mode: AttributeUse::Optional,
            form: AttributeForm::Unqualified,
            default: None,
            fixed: None,
            inheritable: false,
            reference: None,
            errors: Vec::new(),
            built: false,
        }
    }

    /// Set the attribute type
    pub fn set_type(&mut self, attr_type: Arc<dyn SimpleType + Send + Sync>) {
        self.attr_type = Some(attr_type);
    }

    /// Set the use mode
    pub fn set_use(&mut self, use_mode: AttributeUse) {
        self.use_mode = use_mode;
    }

    /// Set the form
    pub fn set_form(&mut self, form: AttributeForm) {
        self.form = form;
    }

    /// Set the default value
    pub fn set_default(&mut self, value: String) -> Result<()> {
        if self.fixed.is_some() {
            return Err(crate::error::Error::Value(
                "'default' and 'fixed' attributes are mutually exclusive".to_string(),
            ));
        }
        if self.use_mode != AttributeUse::Optional {
            return Err(crate::error::Error::Value(
                "Attribute 'use' must be 'optional' if 'default' is present".to_string(),
            ));
        }
        self.default = Some(value);
        Ok(())
    }

    /// Set the fixed value
    pub fn set_fixed(&mut self, value: String) -> Result<()> {
        if self.default.is_some() {
            return Err(crate::error::Error::Value(
                "'default' and 'fixed' attributes are mutually exclusive".to_string(),
            ));
        }
        self.fixed = Some(value);
        Ok(())
    }

    /// Set whether the attribute is inheritable (XSD 1.1)
    pub fn set_inheritable(&mut self, inheritable: bool) {
        self.inheritable = inheritable;
    }

    /// Set a reference to another attribute
    pub fn set_reference(&mut self, reference: Arc<XsdAttribute>) {
        self.reference = Some(reference);
    }

    /// Get the attribute name
    pub fn name(&self) -> &QName {
        &self.name
    }

    /// Get the use mode
    pub fn use_mode(&self) -> AttributeUse {
        self.use_mode
    }

    /// Get the default value
    pub fn default(&self) -> Option<&str> {
        self.default.as_deref()
    }

    /// Get the form
    pub fn form(&self) -> AttributeForm {
        self.form
    }

    /// Check if qualified
    pub fn is_qualified(&self) -> bool {
        self.form == AttributeForm::Qualified
    }

    /// Check if optional
    pub fn is_optional(&self) -> bool {
        self.use_mode == AttributeUse::Optional
    }

    /// Check if prohibited
    pub fn is_prohibited(&self) -> bool {
        self.use_mode == AttributeUse::Prohibited
    }

    /// Check if inheritable (XSD 1.1)
    pub fn is_inheritable(&self) -> bool {
        self.inheritable
    }

    /// Get the simple type
    pub fn simple_type(&self) -> Option<&(dyn SimpleType + Send + Sync)> {
        self.attr_type.as_ref().map(|t| t.as_ref())
    }

    /// Get the value constraint (fixed or default)
    pub fn value_constraint(&self) -> Option<&str> {
        self.fixed.as_deref().or(self.default.as_deref())
    }

    /// Validate an attribute value
    pub fn validate_value(&self, value: Option<&str>) -> Result<XsdValue> {
        // If value is None, check for default/fixed
        let actual_value = match value {
            Some(v) => v,
            None => {
                if let Some(ref fixed) = self.fixed {
                    fixed
                } else if let Some(ref default) = self.default {
                    default
                } else if self.is_required() {
                    return Err(crate::error::Error::Validation(
                        ValidationError::new(format!("Attribute '{}' is required", self.name.to_string()))
                            .with_reason("No value provided for required attribute"),
                    ));
                } else {
                    // Optional attribute with no value and no default
                    return Ok(XsdValue::Null);
                }
            }
        };

        // If fixed, check that value matches
        if let Some(ref fixed) = self.fixed {
            if value.is_some() && actual_value != fixed {
                return Err(crate::error::Error::Validation(
                    ValidationError::new(format!(
                        "Attribute '{}' has fixed value '{}'",
                        self.name.to_string(), fixed
                    ))
                    .with_reason(format!("Provided value: '{}'", actual_value)),
                ));
            }
        }

        // Validate against type if available
        if let Some(ref attr_type) = self.attr_type {
            attr_type.validate_value(actual_value)
        } else {
            // No type means anySimpleType - accept any value
            Ok(XsdValue::String(actual_value.to_string()))
        }
    }
}

impl Validator for XsdAttribute {
    fn is_built(&self) -> bool {
        self.built
    }

    fn build(&mut self) -> Result<()> {
        // Validate default value if present
        if let Some(ref default) = self.default {
            if let Some(ref attr_type) = self.attr_type {
                if let Err(e) = attr_type.validate_value(default) {
                    self.errors.push(ParseError::new(format!(
                        "Default value '{}' is not valid: {}",
                        default, e
                    )));
                }
            }
        }

        // Validate fixed value if present
        if let Some(ref fixed) = self.fixed {
            if let Some(ref attr_type) = self.attr_type {
                if let Err(e) = attr_type.validate_value(fixed) {
                    self.errors.push(ParseError::new(format!(
                        "Fixed value '{}' is not valid: {}",
                        fixed, e
                    )));
                }
            }
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

impl AttributeValidator for XsdAttribute {
    fn name(&self) -> &QName {
        &self.name
    }

    fn attribute_type(&self) -> Option<&dyn TypeValidator> {
        self.attr_type.as_ref().map(|t| t.as_ref() as &dyn TypeValidator)
    }

    fn is_required(&self) -> bool {
        self.use_mode == AttributeUse::Required
    }

    fn default_value(&self) -> Option<&str> {
        self.default.as_deref()
    }

    fn fixed_value(&self) -> Option<&str> {
        self.fixed.as_deref()
    }
}

/// XSD Attribute Group
///
/// Represents a named group of attributes that can be referenced
/// from complex type definitions.
#[derive(Debug, Clone)]
pub struct XsdAttributeGroup {
    /// Group name
    name: Option<QName>,
    /// Attributes in the group
    attributes: HashMap<QName, Arc<XsdAttribute>>,
    /// Attribute groups referenced by this group
    attribute_groups: Vec<Arc<XsdAttributeGroup>>,
    /// Pending attribute group references (QNames to be resolved in build phase)
    pending_group_refs: Vec<QName>,
    /// Optional any attribute wildcard
    any_attribute: Option<Arc<XsdAnyAttribute>>,
    /// Back-reference to original attribute group when this is a redefinition (xs:redefine)
    pub redefine: Option<Arc<XsdAttributeGroup>>,
    /// Build errors
    errors: Vec<ParseError>,
    /// Whether fully built
    built: bool,
}

impl XsdAttributeGroup {
    /// Create a new named attribute group
    pub fn new(name: QName) -> Self {
        Self {
            name: Some(name),
            attributes: HashMap::new(),
            attribute_groups: Vec::new(),
            pending_group_refs: Vec::new(),
            any_attribute: None,
            redefine: None,
            errors: Vec::new(),
            built: false,
        }
    }

    /// Create an anonymous attribute group
    pub fn anonymous() -> Self {
        Self {
            name: None,
            attributes: HashMap::new(),
            attribute_groups: Vec::new(),
            pending_group_refs: Vec::new(),
            any_attribute: None,
            redefine: None,
            errors: Vec::new(),
            built: false,
        }
    }

    /// Get the group name
    pub fn name(&self) -> Option<&QName> {
        self.name.as_ref()
    }

    /// Add an attribute to the group
    pub fn add_attribute(&mut self, attr: Arc<XsdAttribute>) -> Result<()> {
        let name = attr.name.clone();
        if self.attributes.contains_key(&name) {
            return Err(crate::error::Error::Value(format!(
                "Duplicate attribute declaration: '{}'",
                name.to_string()
            )));
        }
        self.attributes.insert(name, attr);
        Ok(())
    }

    /// Set (update or add) an attribute
    ///
    /// Unlike `add_attribute`, this will replace an existing attribute with the same name.
    pub fn set_attribute(&mut self, attr: Arc<XsdAttribute>) {
        let name = attr.name.clone();
        self.attributes.insert(name, attr);
    }

    /// Add a reference to another attribute group
    pub fn add_group_ref(&mut self, group: Arc<XsdAttributeGroup>) {
        self.attribute_groups.push(group);
    }

    /// Add a pending attribute group reference (to be resolved in build phase)
    pub fn add_pending_group_ref(&mut self, qname: QName) {
        self.pending_group_refs.push(qname);
    }

    /// Get pending attribute group references
    pub fn pending_group_refs(&self) -> &[QName] {
        &self.pending_group_refs
    }

    /// Check if there are pending references
    pub fn has_pending_refs(&self) -> bool {
        !self.pending_group_refs.is_empty()
    }

    /// Clear pending references (after resolution)
    pub fn clear_pending_refs(&mut self) {
        self.pending_group_refs.clear();
    }

    /// Set the any attribute wildcard
    pub fn set_any_attribute(&mut self, any: Arc<XsdAnyAttribute>) {
        self.any_attribute = Some(any);
    }

    /// Get the any attribute wildcard
    pub fn any_attribute(&self) -> Option<&Arc<XsdAnyAttribute>> {
        self.any_attribute.as_ref()
    }

    /// Check if this group has an any attribute wildcard
    pub fn has_any_attribute(&self) -> bool {
        self.any_attribute.is_some()
    }

    /// Get an attribute by name
    pub fn get_attribute(&self, name: &QName) -> Option<&Arc<XsdAttribute>> {
        self.attributes.get(name)
    }

    /// Iterate over all attributes (including from referenced groups)
    pub fn iter_attributes(&self) -> impl Iterator<Item = &Arc<XsdAttribute>> {
        self.attributes.values()
    }

    /// Iterate over required attribute names
    pub fn iter_required(&self) -> impl Iterator<Item = &QName> {
        self.attributes
            .iter()
            .filter(|(_, attr)| attr.is_required())
            .map(|(name, _)| name)
    }

    /// Get the number of attributes
    pub fn len(&self) -> usize {
        self.attributes.len()
    }

    /// Check if the group is empty
    pub fn is_empty(&self) -> bool {
        self.attributes.is_empty()
    }

    /// Validate attributes against this group
    pub fn validate_attributes(
        &self,
        attrs: &HashMap<QName, String>,
        _mode: ValidationMode,
    ) -> Result<Vec<(QName, XsdValue)>> {
        let mut results = Vec::new();

        // Check for required attributes
        for required in self.iter_required() {
            if !attrs.contains_key(required) {
                return Err(crate::error::Error::Validation(
                    ValidationError::new(format!("Missing required attribute: '{}'", required.to_string()))
                        .with_reason("Attribute is required by schema"),
                ));
            }
        }

        // Validate each provided attribute
        for (name, value) in attrs {
            if let Some(attr_decl) = self.get_attribute(name) {
                let validated = attr_decl.validate_value(Some(value))?;
                results.push((name.clone(), validated));
            } else {
                // Attribute not declared - in strict mode this is an error
                // For now, we'll allow it but not include in results
            }
        }

        // Add defaults for missing optional attributes
        for (name, attr) in &self.attributes {
            if !attrs.contains_key(name) {
                if let Some(default) = attr.default_value() {
                    let validated = attr.validate_value(Some(default))?;
                    results.push((name.clone(), validated));
                }
            }
        }

        Ok(results)
    }
}

impl Validator for XsdAttributeGroup {
    fn is_built(&self) -> bool {
        self.built
    }

    fn build(&mut self) -> Result<()> {
        // Merge attributes from referenced groups
        for group_ref in &self.attribute_groups {
            for (name, attr) in &group_ref.attributes {
                if !self.attributes.contains_key(name) {
                    self.attributes.insert(name.clone(), attr.clone());
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validators::builtins::{XSD_INTEGER, XSD_STRING};
    use crate::validators::simple_types::XsdAtomicType;

    #[test]
    fn test_attribute_use() {
        assert_eq!(AttributeUse::from_str("optional").unwrap(), AttributeUse::Optional);
        assert_eq!(AttributeUse::from_str("required").unwrap(), AttributeUse::Required);
        assert_eq!(AttributeUse::from_str("prohibited").unwrap(), AttributeUse::Prohibited);
        assert!(AttributeUse::from_str("invalid").is_err());

        assert_eq!(AttributeUse::Optional.as_str(), "optional");
        assert_eq!(AttributeUse::Required.as_str(), "required");
        assert_eq!(AttributeUse::Prohibited.as_str(), "prohibited");
    }

    #[test]
    fn test_attribute_form() {
        assert_eq!(
            AttributeForm::from_str("unqualified").unwrap(),
            AttributeForm::Unqualified
        );
        assert_eq!(
            AttributeForm::from_str("qualified").unwrap(),
            AttributeForm::Qualified
        );
        assert!(AttributeForm::from_str("invalid").is_err());
    }

    #[test]
    fn test_attribute_creation() {
        let name = QName::local("myAttr");
        let attr = XsdAttribute::new(name.clone());

        assert_eq!(attr.name(), &name);
        assert!(attr.attribute_type().is_none());
        assert!(!attr.is_required());
        assert!(attr.is_optional());
        assert!(!attr.is_prohibited());
        assert!(attr.default_value().is_none());
        assert!(attr.fixed_value().is_none());
    }

    #[test]
    fn test_attribute_with_type() {
        let name = QName::local("count");
        let int_type = XsdAtomicType::new(XSD_INTEGER).unwrap();
        let attr = XsdAttribute::with_type(name.clone(), Arc::new(int_type));

        assert!(attr.attribute_type().is_some());
    }

    #[test]
    fn test_attribute_required() {
        let name = QName::local("id");
        let mut attr = XsdAttribute::new(name);
        attr.set_use(AttributeUse::Required);

        assert!(attr.is_required());
        assert!(!attr.is_optional());
    }

    #[test]
    fn test_attribute_default() {
        let name = QName::local("status");
        let mut attr = XsdAttribute::new(name);

        // Can set default on optional attribute
        attr.set_default("active".to_string()).unwrap();
        assert_eq!(attr.default_value(), Some("active"));

        // Cannot set default on required attribute
        let mut required_attr = XsdAttribute::new(QName::local("req"));
        required_attr.set_use(AttributeUse::Required);
        assert!(required_attr.set_default("value".to_string()).is_err());
    }

    #[test]
    fn test_attribute_fixed() {
        let name = QName::local("version");
        let mut attr = XsdAttribute::new(name);

        attr.set_fixed("1.0".to_string()).unwrap();
        assert_eq!(attr.fixed_value(), Some("1.0"));

        // Cannot set default when fixed is set
        assert!(attr.set_default("2.0".to_string()).is_err());
    }

    #[test]
    fn test_default_and_fixed_mutex() {
        let name = QName::local("value");
        let mut attr = XsdAttribute::new(name);

        // Set default first
        attr.set_default("default".to_string()).unwrap();

        // Cannot set fixed
        assert!(attr.set_fixed("fixed".to_string()).is_err());
    }

    #[test]
    fn test_attribute_validate_value() {
        let name = QName::local("amount");
        let int_type = XsdAtomicType::new(XSD_INTEGER).unwrap();
        let attr = XsdAttribute::with_type(name, Arc::new(int_type));

        // Valid integer
        let result = attr.validate_value(Some("42"));
        assert!(result.is_ok());
        match result.unwrap() {
            XsdValue::Integer(v) => assert_eq!(v, 42),
            _ => panic!("Expected integer value"),
        }

        // Invalid integer
        let result = attr.validate_value(Some("not-a-number"));
        assert!(result.is_err());
    }

    #[test]
    fn test_attribute_fixed_value_validation() {
        let name = QName::local("locked");
        let string_type = XsdAtomicType::new(XSD_STRING).unwrap();
        let mut attr = XsdAttribute::with_type(name, Arc::new(string_type));
        attr.set_fixed("yes".to_string()).unwrap();

        // Correct fixed value
        let result = attr.validate_value(Some("yes"));
        assert!(result.is_ok());

        // Wrong value
        let result = attr.validate_value(Some("no"));
        assert!(result.is_err());

        // No value - should use fixed
        let result = attr.validate_value(None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_required_attribute_missing() {
        let name = QName::local("id");
        let mut attr = XsdAttribute::new(name);
        attr.set_use(AttributeUse::Required);

        // Missing required attribute
        let result = attr.validate_value(None);
        assert!(result.is_err());
    }

    #[test]
    fn test_attribute_group_creation() {
        let group_name = QName::local("commonAttrs");
        let group = XsdAttributeGroup::new(group_name.clone());

        assert_eq!(group.name(), Some(&group_name));
        assert!(group.is_empty());
    }

    #[test]
    fn test_attribute_group_add_attribute() {
        let mut group = XsdAttributeGroup::anonymous();

        let attr1 = Arc::new(XsdAttribute::new(QName::local("attr1")));
        let attr2 = Arc::new(XsdAttribute::new(QName::local("attr2")));

        group.add_attribute(attr1).unwrap();
        group.add_attribute(attr2).unwrap();

        assert_eq!(group.len(), 2);
        assert!(group.get_attribute(&QName::local("attr1")).is_some());
        assert!(group.get_attribute(&QName::local("attr2")).is_some());
        assert!(group.get_attribute(&QName::local("attr3")).is_none());
    }

    #[test]
    fn test_attribute_group_duplicate() {
        let mut group = XsdAttributeGroup::anonymous();

        let attr1 = Arc::new(XsdAttribute::new(QName::local("attr")));
        let attr2 = Arc::new(XsdAttribute::new(QName::local("attr")));

        group.add_attribute(attr1).unwrap();
        assert!(group.add_attribute(attr2).is_err());
    }

    #[test]
    fn test_attribute_group_required() {
        let mut group = XsdAttributeGroup::anonymous();

        let mut req_attr = XsdAttribute::new(QName::local("required"));
        req_attr.set_use(AttributeUse::Required);

        let opt_attr = XsdAttribute::new(QName::local("optional"));

        group.add_attribute(Arc::new(req_attr)).unwrap();
        group.add_attribute(Arc::new(opt_attr)).unwrap();

        let required: Vec<_> = group.iter_required().collect();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], &QName::local("required"));
    }

    #[test]
    fn test_attribute_group_validate() {
        let mut group = XsdAttributeGroup::anonymous();

        let string_type = Arc::new(XsdAtomicType::new(XSD_STRING).unwrap());
        let int_type = Arc::new(XsdAtomicType::new(XSD_INTEGER).unwrap());

        let mut name_attr = XsdAttribute::with_type(QName::local("name"), string_type);
        name_attr.set_use(AttributeUse::Required);

        let count_attr = XsdAttribute::with_type(QName::local("count"), int_type);

        group.add_attribute(Arc::new(name_attr)).unwrap();
        group.add_attribute(Arc::new(count_attr)).unwrap();

        // Valid attributes
        let mut attrs = HashMap::new();
        attrs.insert(QName::local("name"), "test".to_string());
        attrs.insert(QName::local("count"), "5".to_string());

        let result = group.validate_attributes(&attrs, ValidationMode::Strict);
        assert!(result.is_ok());

        // Missing required attribute
        let mut missing_required = HashMap::new();
        missing_required.insert(QName::local("count"), "5".to_string());

        let result = group.validate_attributes(&missing_required, ValidationMode::Strict);
        assert!(result.is_err());
    }

    #[test]
    fn test_attribute_build() {
        let name = QName::local("count");
        let int_type = XsdAtomicType::new(XSD_INTEGER).unwrap();
        let mut attr = XsdAttribute::with_type(name, Arc::new(int_type));

        assert!(!attr.is_built());

        attr.build().unwrap();
        assert!(attr.is_built());
        assert_eq!(attr.validation_attempted(), ValidationStatus::Full);
    }

    #[test]
    fn test_attribute_build_with_invalid_default() {
        let name = QName::local("count");
        let int_type = XsdAtomicType::new(XSD_INTEGER).unwrap();
        let mut attr = XsdAttribute::with_type(name, Arc::new(int_type));

        // Set an invalid default (not an integer)
        attr.default = Some("not-a-number".to_string());

        attr.build().unwrap();

        // Should have recorded an error
        assert!(attr.has_errors());
        assert_eq!(attr.validation_attempted(), ValidationStatus::Partial);
    }

    #[test]
    fn test_attribute_group_build() {
        let mut group = XsdAttributeGroup::new(QName::local("group"));

        let attr = Arc::new(XsdAttribute::new(QName::local("attr")));
        group.add_attribute(attr).unwrap();

        assert!(!group.is_built());

        group.build().unwrap();
        assert!(group.is_built());
    }

    #[test]
    fn test_attribute_inheritable() {
        let name = QName::local("lang");
        let mut attr = XsdAttribute::new(name);

        assert!(!attr.is_inheritable());

        attr.set_inheritable(true);
        assert!(attr.is_inheritable());
    }

    #[test]
    fn test_value_constraint() {
        let name = QName::local("attr");

        // With fixed
        let mut fixed_attr = XsdAttribute::new(name.clone());
        fixed_attr.set_fixed("fixed_value".to_string()).unwrap();
        assert_eq!(fixed_attr.value_constraint(), Some("fixed_value"));

        // With default
        let mut default_attr = XsdAttribute::new(QName::local("attr2"));
        default_attr.set_default("default_value".to_string()).unwrap();
        assert_eq!(default_attr.value_constraint(), Some("default_value"));

        // Without either
        let plain_attr = XsdAttribute::new(QName::local("attr3"));
        assert!(plain_attr.value_constraint().is_none());
    }
}
