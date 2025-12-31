//! XSD Simple Type validators
//!
//! This module implements XSD simple type validation including:
//! - Atomic types (built-in and derived)
//! - List types (whitespace-separated lists)
//! - Union types (value matching any member type)
//!
//! See: https://www.w3.org/TR/xmlschema-2/

use crate::error::{Error, ParseError, Result, ValidationError};
use crate::namespaces::QName;
use crate::validators::base::{TypeValidator, ValidationStatus, Validator};
use crate::validators::builtins::{get_builtin_type, validate_builtin, BuiltinType, XsdValue, XSD_NAMESPACE};
use crate::validators::facets::{
    EnumerationFacet, FractionDigitsFacet, LengthFacet, MaxExclusiveFacet, MaxInclusiveFacet,
    MaxLengthFacet, MinExclusiveFacet, MinInclusiveFacet, MinLengthFacet, PatternFacet,
    TotalDigitsFacet, WhiteSpace,
};
use std::sync::Arc;

// =============================================================================
// Simple Type Variety
// =============================================================================

/// Variety of a simple type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimpleTypeVariety {
    /// Atomic type (single value)
    Atomic,
    /// List type (whitespace-separated values)
    List,
    /// Union type (value matches one of several types)
    Union,
}

// =============================================================================
// Facet Container
// =============================================================================

/// Container for all facets that can constrain a simple type
#[derive(Debug, Clone, Default)]
pub struct FacetSet {
    /// Length facet
    pub length: Option<LengthFacet>,
    /// Minimum length facet
    pub min_length: Option<MinLengthFacet>,
    /// Maximum length facet
    pub max_length: Option<MaxLengthFacet>,
    /// Pattern facets (can have multiple)
    pub patterns: Vec<PatternFacet>,
    /// Enumeration facet
    pub enumeration: Option<EnumerationFacet>,
    /// White space handling
    pub white_space: Option<WhiteSpace>,
    /// Minimum inclusive facet
    pub min_inclusive: Option<MinInclusiveFacet>,
    /// Maximum inclusive facet
    pub max_inclusive: Option<MaxInclusiveFacet>,
    /// Minimum exclusive facet
    pub min_exclusive: Option<MinExclusiveFacet>,
    /// Maximum exclusive facet
    pub max_exclusive: Option<MaxExclusiveFacet>,
    /// Total digits facet
    pub total_digits: Option<TotalDigitsFacet>,
    /// Fraction digits facet
    pub fraction_digits: Option<FractionDigitsFacet>,
}

impl FacetSet {
    /// Create an empty facet set
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate a string value against all facets
    pub fn validate(&self, value: &str) -> Result<()> {
        // Apply white space normalization
        let normalized = if let Some(ws) = &self.white_space {
            ws.normalize(value)
        } else {
            value.to_string()
        };

        // Validate length facets
        if let Some(ref facet) = self.length {
            facet.validate(&normalized)?;
        }
        if let Some(ref facet) = self.min_length {
            facet.validate(&normalized)?;
        }
        if let Some(ref facet) = self.max_length {
            facet.validate(&normalized)?;
        }

        // Validate patterns
        for pattern in &self.patterns {
            pattern.validate(&normalized)?;
        }

        // Validate enumeration
        if let Some(ref facet) = self.enumeration {
            facet.validate(&normalized)?;
        }

        Ok(())
    }
}

// =============================================================================
// Simple Type Trait
// =============================================================================

/// Trait for all simple type validators
pub trait SimpleType: TypeValidator {
    /// Get the variety of this simple type
    fn variety(&self) -> SimpleTypeVariety;

    /// Get the base type (for derived types)
    fn base_type(&self) -> Option<&dyn SimpleType>;

    /// Get the facets applied to this type
    fn facets(&self) -> &FacetSet;

    /// Validate a string value against this type
    fn validate_value(&self, value: &str) -> Result<XsdValue>;

    /// Check if this type allows empty values
    fn allow_empty(&self) -> bool {
        // By default, check min_length facet
        if let Some(ref min_len) = self.facets().min_length {
            min_len.value == 0
        } else {
            true
        }
    }

    /// Get white space handling mode
    fn white_space(&self) -> WhiteSpace {
        self.facets()
            .white_space
            .unwrap_or(WhiteSpace::Preserve)
    }

    /// Get the qualified name as a string in {namespace}localName format
    ///
    /// For named types, returns the formatted name.
    /// For builtin types, returns the XSD namespace qualified name.
    fn qualified_name_string(&self) -> Option<String>;

    /// For List types: get the item type
    /// Returns None for non-List types
    fn item_type(&self) -> Option<&Arc<dyn SimpleType + Send + Sync>> {
        None
    }

    /// For Union types: get the member types
    /// Returns empty slice for non-Union types
    fn member_types(&self) -> &[Arc<dyn SimpleType + Send + Sync>] {
        &[]
    }
}

// =============================================================================
// Atomic Type
// =============================================================================

/// Atomic simple type - represents a single value
#[derive(Debug)]
pub struct XsdAtomicType {
    /// Type name
    name: Option<QName>,
    /// Base built-in type name
    builtin_name: String,
    /// Reference to the built-in type
    builtin: &'static BuiltinType,
    /// Facets constraining this type
    facet_set: FacetSet,
    /// Building errors
    errors: Vec<ParseError>,
    /// Is this type built?
    built: bool,
}

impl XsdAtomicType {
    /// Create a new atomic type from a built-in type name
    pub fn new(builtin_name: &str) -> Result<Self> {
        let builtin = get_builtin_type(builtin_name).ok_or_else(|| {
            Error::Type(format!("Unknown built-in type: {}", builtin_name))
        })?;

        Ok(Self {
            name: None,
            builtin_name: builtin_name.to_string(),
            builtin,
            facet_set: FacetSet {
                white_space: Some(builtin.white_space),
                ..Default::default()
            },
            errors: Vec::new(),
            built: true,
        })
    }

    /// Create a named atomic type
    pub fn with_name(builtin_name: &str, name: QName) -> Result<Self> {
        let mut atomic = Self::new(builtin_name)?;
        atomic.name = Some(name);
        Ok(atomic)
    }

    /// Add a length facet
    pub fn with_length(mut self, length: usize) -> Self {
        self.facet_set.length = Some(LengthFacet::new(length));
        self
    }

    /// Add a min length facet
    pub fn with_min_length(mut self, min_length: usize) -> Self {
        self.facet_set.min_length = Some(MinLengthFacet::new(min_length));
        self
    }

    /// Add a max length facet
    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.facet_set.max_length = Some(MaxLengthFacet::new(max_length));
        self
    }

    /// Add a pattern facet
    pub fn with_pattern(mut self, pattern: &str) -> Result<Self> {
        self.facet_set.patterns.push(PatternFacet::new(pattern)?);
        Ok(self)
    }

    /// Add an enumeration facet
    pub fn with_enumeration(mut self, values: Vec<String>) -> Self {
        self.facet_set.enumeration = Some(EnumerationFacet::new(values));
        self
    }

    /// Get the built-in type name
    pub fn builtin_name(&self) -> &str {
        &self.builtin_name
    }
}

impl Validator for XsdAtomicType {
    fn is_built(&self) -> bool {
        self.built
    }

    fn build(&mut self) -> Result<()> {
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

impl TypeValidator for XsdAtomicType {
    fn name(&self) -> Option<&QName> {
        self.name.as_ref()
    }

    fn is_builtin(&self) -> bool {
        self.name.is_none()
    }

    fn base_type(&self) -> Option<&dyn TypeValidator> {
        None // Built-in types don't expose base_type through this interface
    }
}

impl SimpleType for XsdAtomicType {
    fn variety(&self) -> SimpleTypeVariety {
        SimpleTypeVariety::Atomic
    }

    fn base_type(&self) -> Option<&dyn SimpleType> {
        None
    }

    fn facets(&self) -> &FacetSet {
        &self.facet_set
    }

    fn validate_value(&self, value: &str) -> Result<XsdValue> {
        // First validate against facets
        self.facet_set.validate(value)?;

        // Then validate against the built-in type
        validate_builtin(&self.builtin_name, value)
    }

    fn qualified_name_string(&self) -> Option<String> {
        if let Some(ref name) = self.name {
            Some(format_qname(name))
        } else {
            // Builtin type - use XSD namespace
            Some(format!("{{{}}}{}", XSD_NAMESPACE, self.builtin_name))
        }
    }
}

/// Format a QName as {namespace}localName
fn format_qname(qname: &QName) -> String {
    match &qname.namespace {
        Some(ns) => format!("{{{}}}{}", ns, qname.local_name),
        None => qname.local_name.clone(),
    }
}

// =============================================================================
// List Type
// =============================================================================

/// List simple type - whitespace-separated list of values
#[derive(Debug)]
pub struct XsdListType {
    /// Type name
    name: Option<QName>,
    /// Item type for list elements
    item_type: Arc<dyn SimpleType + Send + Sync>,
    /// Facets constraining the list
    facet_set: FacetSet,
    /// Building errors
    errors: Vec<ParseError>,
    /// Is this type built?
    built: bool,
}

impl XsdListType {
    /// Create a new list type with the given item type
    pub fn new(item_type: Arc<dyn SimpleType + Send + Sync>) -> Self {
        Self {
            name: None,
            item_type,
            facet_set: FacetSet {
                white_space: Some(WhiteSpace::Collapse),
                ..Default::default()
            },
            errors: Vec::new(),
            built: true,
        }
    }

    /// Create a named list type
    pub fn with_name(item_type: Arc<dyn SimpleType + Send + Sync>, name: QName) -> Self {
        let mut list = Self::new(item_type);
        list.name = Some(name);
        list
    }

    /// Get the item type
    pub fn item_type(&self) -> &Arc<dyn SimpleType + Send + Sync> {
        &self.item_type
    }

    /// Add a length facet (number of items)
    pub fn with_length(mut self, length: usize) -> Self {
        self.facet_set.length = Some(LengthFacet::new(length));
        self
    }

    /// Add a min length facet
    pub fn with_min_length(mut self, min_length: usize) -> Self {
        self.facet_set.min_length = Some(MinLengthFacet::new(min_length));
        self
    }

    /// Add a max length facet
    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.facet_set.max_length = Some(MaxLengthFacet::new(max_length));
        self
    }
}

impl Validator for XsdListType {
    fn is_built(&self) -> bool {
        self.built
    }

    fn build(&mut self) -> Result<()> {
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

impl TypeValidator for XsdListType {
    fn name(&self) -> Option<&QName> {
        self.name.as_ref()
    }

    fn is_builtin(&self) -> bool {
        false
    }

    fn base_type(&self) -> Option<&dyn TypeValidator> {
        None
    }
}

impl SimpleType for XsdListType {
    fn variety(&self) -> SimpleTypeVariety {
        SimpleTypeVariety::List
    }

    fn base_type(&self) -> Option<&dyn SimpleType> {
        None
    }

    fn facets(&self) -> &FacetSet {
        &self.facet_set
    }

    fn validate_value(&self, value: &str) -> Result<XsdValue> {
        // Normalize whitespace (always collapse for lists)
        let normalized = WhiteSpace::Collapse.normalize(value);

        // Split into items
        let items: Vec<&str> = if normalized.is_empty() {
            Vec::new()
        } else {
            normalized.split(' ').collect()
        };

        // Validate list length facets
        let item_count = items.len();
        if let Some(ref facet) = self.facet_set.length {
            if item_count != facet.value {
                return Err(Error::Validation(
                    ValidationError::new(format!("List must have exactly {} items", facet.value))
                        .with_reason(format!("Actual count: {}", item_count)),
                ));
            }
        }
        if let Some(ref facet) = self.facet_set.min_length {
            if item_count < facet.value {
                return Err(Error::Validation(
                    ValidationError::new(format!("List must have at least {} items", facet.value))
                        .with_reason(format!("Actual count: {}", item_count)),
                ));
            }
        }
        if let Some(ref facet) = self.facet_set.max_length {
            if item_count > facet.value {
                return Err(Error::Validation(
                    ValidationError::new(format!("List must have at most {} items", facet.value))
                        .with_reason(format!("Actual count: {}", item_count)),
                ));
            }
        }

        // Validate each item against the item type
        let mut values = Vec::with_capacity(items.len());
        for (i, item) in items.iter().enumerate() {
            match self.item_type.validate_value(item) {
                Ok(v) => values.push(v),
                Err(e) => {
                    return Err(Error::Validation(
                        ValidationError::new(format!("Invalid list item at position {}", i + 1))
                            .with_reason(format!("Item '{}': {}", item, e)),
                    ));
                }
            }
        }

        // Return as string representation (list doesn't have a special XsdValue variant)
        Ok(XsdValue::String(normalized))
    }

    fn qualified_name_string(&self) -> Option<String> {
        self.name.as_ref().map(format_qname)
    }

    fn item_type(&self) -> Option<&Arc<dyn SimpleType + Send + Sync>> {
        Some(&self.item_type)
    }
}

// =============================================================================
// Union Type
// =============================================================================

/// Union simple type - value matches one of several member types
#[derive(Debug)]
pub struct XsdUnionType {
    /// Type name
    name: Option<QName>,
    /// Member types
    member_types: Vec<Arc<dyn SimpleType + Send + Sync>>,
    /// Facets constraining the union
    facet_set: FacetSet,
    /// Building errors
    errors: Vec<ParseError>,
    /// Is this type built?
    built: bool,
}

impl XsdUnionType {
    /// Create a new union type with the given member types
    pub fn new(member_types: Vec<Arc<dyn SimpleType + Send + Sync>>) -> Self {
        Self {
            name: None,
            member_types,
            facet_set: FacetSet {
                white_space: Some(WhiteSpace::Collapse),
                ..Default::default()
            },
            errors: Vec::new(),
            built: true,
        }
    }

    /// Create a named union type
    pub fn with_name(
        member_types: Vec<Arc<dyn SimpleType + Send + Sync>>,
        name: QName,
    ) -> Self {
        let mut union = Self::new(member_types);
        union.name = Some(name);
        union
    }

    /// Get the member types
    pub fn member_types(&self) -> &[Arc<dyn SimpleType + Send + Sync>] {
        &self.member_types
    }

    /// Add a pattern facet
    pub fn with_pattern(mut self, pattern: &str) -> Result<Self> {
        self.facet_set.patterns.push(PatternFacet::new(pattern)?);
        Ok(self)
    }

    /// Add an enumeration facet
    pub fn with_enumeration(mut self, values: Vec<String>) -> Self {
        self.facet_set.enumeration = Some(EnumerationFacet::new(values));
        self
    }
}

impl Validator for XsdUnionType {
    fn is_built(&self) -> bool {
        self.built
    }

    fn build(&mut self) -> Result<()> {
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

impl TypeValidator for XsdUnionType {
    fn name(&self) -> Option<&QName> {
        self.name.as_ref()
    }

    fn is_builtin(&self) -> bool {
        false
    }

    fn base_type(&self) -> Option<&dyn TypeValidator> {
        None
    }
}

impl SimpleType for XsdUnionType {
    fn variety(&self) -> SimpleTypeVariety {
        SimpleTypeVariety::Union
    }

    fn base_type(&self) -> Option<&dyn SimpleType> {
        None
    }

    fn facets(&self) -> &FacetSet {
        &self.facet_set
    }

    fn validate_value(&self, value: &str) -> Result<XsdValue> {
        // First validate against union-level facets (pattern, enumeration)
        self.facet_set.validate(value)?;

        // Try each member type in order
        let mut last_error = None;
        for member in &self.member_types {
            match member.validate_value(value) {
                Ok(v) => return Ok(v),
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        // No member type matched
        Err(Error::Validation(
            ValidationError::new("Value does not match any member type of the union")
                .with_reason(format!(
                    "Value '{}' failed validation against all {} member types. Last error: {}",
                    value,
                    self.member_types.len(),
                    last_error.map(|e| e.to_string()).unwrap_or_default()
                )),
        ))
    }

    fn qualified_name_string(&self) -> Option<String> {
        self.name.as_ref().map(format_qname)
    }

    fn member_types(&self) -> &[Arc<dyn SimpleType + Send + Sync>] {
        &self.member_types
    }
}

// =============================================================================
// Restricted Type (for derived types with additional facets)
// =============================================================================

/// Atomic type derived by restriction from another simple type
#[derive(Debug)]
pub struct XsdRestrictedType {
    /// Type name
    name: Option<QName>,
    /// Base type being restricted
    base_type_ref: Arc<dyn SimpleType + Send + Sync>,
    /// Additional facets
    facet_set: FacetSet,
    /// Back-reference to original type when this is a redefinition (xs:redefine)
    pub redefine: Option<Arc<dyn SimpleType + Send + Sync>>,
    /// Building errors
    errors: Vec<ParseError>,
    /// Is this type built?
    built: bool,
}

impl XsdRestrictedType {
    /// Create a new restricted type from a base type
    pub fn new(base_type: Arc<dyn SimpleType + Send + Sync>) -> Self {
        let white_space = base_type.white_space();
        Self {
            name: None,
            base_type_ref: base_type,
            facet_set: FacetSet {
                white_space: Some(white_space),
                ..Default::default()
            },
            redefine: None,
            errors: Vec::new(),
            built: true,
        }
    }

    /// Create a named restricted type
    pub fn with_name(base_type: Arc<dyn SimpleType + Send + Sync>, name: QName) -> Self {
        let mut restricted = Self::new(base_type);
        restricted.name = Some(name);
        restricted
    }

    /// Add a length facet
    pub fn with_length(mut self, length: usize) -> Self {
        self.facet_set.length = Some(LengthFacet::new(length));
        self
    }

    /// Add a min length facet
    pub fn with_min_length(mut self, min_length: usize) -> Self {
        self.facet_set.min_length = Some(MinLengthFacet::new(min_length));
        self
    }

    /// Add a max length facet
    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.facet_set.max_length = Some(MaxLengthFacet::new(max_length));
        self
    }

    /// Add a pattern facet
    pub fn with_pattern(mut self, pattern: &str) -> Result<Self> {
        self.facet_set.patterns.push(PatternFacet::new(pattern)?);
        Ok(self)
    }

    /// Add an enumeration facet
    pub fn with_enumeration(mut self, values: Vec<String>) -> Self {
        self.facet_set.enumeration = Some(EnumerationFacet::new(values));
        self
    }

    /// Get the base type
    pub fn base(&self) -> &Arc<dyn SimpleType + Send + Sync> {
        &self.base_type_ref
    }
}

impl Validator for XsdRestrictedType {
    fn is_built(&self) -> bool {
        self.built
    }

    fn build(&mut self) -> Result<()> {
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

impl TypeValidator for XsdRestrictedType {
    fn name(&self) -> Option<&QName> {
        self.name.as_ref()
    }

    fn is_builtin(&self) -> bool {
        false
    }

    fn base_type(&self) -> Option<&dyn TypeValidator> {
        None // We don't expose the base type through this interface
    }
}

impl SimpleType for XsdRestrictedType {
    fn variety(&self) -> SimpleTypeVariety {
        self.base_type_ref.variety()
    }

    fn base_type(&self) -> Option<&dyn SimpleType> {
        Some(self.base_type_ref.as_ref())
    }

    fn facets(&self) -> &FacetSet {
        &self.facet_set
    }

    fn validate_value(&self, value: &str) -> Result<XsdValue> {
        // First validate against our own facets
        self.facet_set.validate(value)?;

        // Then delegate to the base type
        self.base_type_ref.validate_value(value)
    }

    fn qualified_name_string(&self) -> Option<String> {
        self.name.as_ref().map(format_qname)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validators::builtins::{XSD_INTEGER, XSD_STRING};

    #[test]
    fn test_atomic_type_string() {
        let atomic = XsdAtomicType::new(XSD_STRING).unwrap();
        assert_eq!(atomic.variety(), SimpleTypeVariety::Atomic);
        assert!(atomic.validate_value("hello").is_ok());
    }

    #[test]
    fn test_atomic_type_integer() {
        let atomic = XsdAtomicType::new(XSD_INTEGER).unwrap();
        assert!(atomic.validate_value("123").is_ok());
        assert!(atomic.validate_value("-456").is_ok());
        assert!(atomic.validate_value("abc").is_err());
    }

    #[test]
    fn test_atomic_with_length() {
        let atomic = XsdAtomicType::new(XSD_STRING).unwrap().with_length(5);
        assert!(atomic.validate_value("hello").is_ok());
        assert!(atomic.validate_value("hi").is_err());
        assert!(atomic.validate_value("toolong").is_err());
    }

    #[test]
    fn test_atomic_with_min_max_length() {
        let atomic = XsdAtomicType::new(XSD_STRING)
            .unwrap()
            .with_min_length(2)
            .with_max_length(5);
        assert!(atomic.validate_value("hi").is_ok());
        assert!(atomic.validate_value("hello").is_ok());
        assert!(atomic.validate_value("a").is_err());
        assert!(atomic.validate_value("toolong").is_err());
    }

    #[test]
    fn test_atomic_with_pattern() {
        let atomic = XsdAtomicType::new(XSD_STRING)
            .unwrap()
            .with_pattern(r"^\d{3}-\d{4}$")
            .unwrap();
        assert!(atomic.validate_value("123-4567").is_ok());
        assert!(atomic.validate_value("12-4567").is_err());
    }

    #[test]
    fn test_atomic_with_enumeration() {
        let atomic = XsdAtomicType::new(XSD_STRING)
            .unwrap()
            .with_enumeration(vec!["red".to_string(), "green".to_string(), "blue".to_string()]);
        assert!(atomic.validate_value("red").is_ok());
        assert!(atomic.validate_value("green").is_ok());
        assert!(atomic.validate_value("yellow").is_err());
    }

    #[test]
    fn test_list_type() {
        let item_type = Arc::new(XsdAtomicType::new(XSD_INTEGER).unwrap());
        let list = XsdListType::new(item_type);
        assert_eq!(list.variety(), SimpleTypeVariety::List);
        assert!(list.validate_value("1 2 3").is_ok());
        assert!(list.validate_value("1 abc 3").is_err());
    }

    #[test]
    fn test_list_with_length() {
        let item_type = Arc::new(XsdAtomicType::new(XSD_INTEGER).unwrap());
        let list = XsdListType::new(item_type).with_length(3);
        assert!(list.validate_value("1 2 3").is_ok());
        assert!(list.validate_value("1 2").is_err());
        assert!(list.validate_value("1 2 3 4").is_err());
    }

    #[test]
    fn test_list_empty() {
        let item_type = Arc::new(XsdAtomicType::new(XSD_INTEGER).unwrap());
        let list = XsdListType::new(item_type);
        assert!(list.validate_value("").is_ok());
    }

    #[test]
    fn test_union_type() {
        let int_type = Arc::new(XsdAtomicType::new(XSD_INTEGER).unwrap());
        let str_type = Arc::new(
            XsdAtomicType::new(XSD_STRING)
                .unwrap()
                .with_enumeration(vec!["none".to_string()]),
        );
        let union = XsdUnionType::new(vec![int_type, str_type]);
        assert_eq!(union.variety(), SimpleTypeVariety::Union);
        assert!(union.validate_value("123").is_ok());
        assert!(union.validate_value("none").is_ok());
        assert!(union.validate_value("invalid").is_err());
    }

    #[test]
    fn test_restricted_type() {
        let base_type = Arc::new(XsdAtomicType::new(XSD_STRING).unwrap());
        let restricted = XsdRestrictedType::new(base_type).with_max_length(10);
        assert!(restricted.validate_value("hello").is_ok());
        assert!(restricted.validate_value("this is too long").is_err());
    }

    #[test]
    fn test_restricted_with_pattern() {
        let base_type = Arc::new(XsdAtomicType::new(XSD_STRING).unwrap());
        let restricted = XsdRestrictedType::new(base_type)
            .with_pattern(r"^[A-Z]{2}\d{4}$")
            .unwrap();
        assert!(restricted.validate_value("AB1234").is_ok());
        assert!(restricted.validate_value("ab1234").is_err());
    }

    #[test]
    fn test_type_validator_trait() {
        let atomic = XsdAtomicType::new(XSD_STRING).unwrap();
        assert!(atomic.is_builtin());
        assert!(atomic.name().is_none());
        assert!(atomic.is_built());
    }

    #[test]
    fn test_named_type() {
        let name = QName::new(None::<String>, "myStringType".to_string());
        let atomic = XsdAtomicType::with_name(XSD_STRING, name.clone()).unwrap();
        assert!(!atomic.is_builtin());
        assert_eq!(atomic.name(), Some(&name));
    }
}
