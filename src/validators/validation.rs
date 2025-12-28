//! XML Validation Infrastructure
//!
//! This module provides the core validation context and traits for validating
//! XML documents against XSD schemas.

use std::collections::HashMap;
use crate::error::Result;
use super::exceptions::{ValidationError, XsdValidatorError};
use super::base::ValidationMode;

/// Validation context for handling the validation process
///
/// Stores status-related fields that are updated during validation,
/// along with parameters and configuration.
#[derive(Debug)]
pub struct ValidationContext {
    /// Current validation mode
    pub mode: ValidationMode,
    /// Collected validation errors
    pub errors: Vec<ValidationError>,
    /// Current nesting level
    pub level: usize,
    /// Maximum depth for validation (None = unlimited)
    pub max_depth: Option<usize>,
    /// Whether to use default values
    pub use_defaults: bool,
    /// Whether to preserve mixed content
    pub preserve_mixed: bool,
    /// Whether to check identity constraints
    pub check_identities: bool,
    /// Whether to process skipped content
    pub process_skipped: bool,
    /// Whether to use location hints for schema resolution
    pub use_location_hints: bool,
    /// Namespace mappings
    pub namespaces: HashMap<String, String>,
    /// ID map for tracking ID values (for xs:ID validation)
    pub id_map: HashMap<String, usize>,
    /// Inherited attributes
    pub inherited: HashMap<String, String>,
    /// Current element being validated
    pub current_element: Option<String>,
    /// Current attribute being validated
    pub current_attribute: Option<String>,
}

impl ValidationContext {
    /// Create a new validation context
    pub fn new() -> Self {
        Self {
            mode: ValidationMode::Strict,
            errors: Vec::new(),
            level: 0,
            max_depth: None,
            use_defaults: true,
            preserve_mixed: false,
            check_identities: false,
            process_skipped: false,
            use_location_hints: false,
            namespaces: HashMap::new(),
            id_map: HashMap::new(),
            inherited: HashMap::new(),
            current_element: None,
            current_attribute: None,
        }
    }

    /// Create a context with a specific mode
    pub fn with_mode(mut self, mode: ValidationMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set maximum depth
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Set namespace mappings
    pub fn with_namespaces(mut self, namespaces: HashMap<String, String>) -> Self {
        self.namespaces = namespaces;
        self
    }

    /// Enable identity constraint checking
    pub fn with_identity_check(mut self) -> Self {
        self.check_identities = true;
        self
    }

    /// Check if we've exceeded max depth
    pub fn is_max_depth_exceeded(&self) -> bool {
        if let Some(max) = self.max_depth {
            self.level >= max
        } else {
            false
        }
    }

    /// Enter a new level
    pub fn enter_level(&mut self) {
        self.level += 1;
    }

    /// Exit current level
    pub fn exit_level(&mut self) {
        if self.level > 0 {
            self.level -= 1;
        }
    }

    /// Clear the context for reuse
    pub fn clear(&mut self) {
        self.errors.clear();
        self.id_map.clear();
        self.inherited.clear();
        self.level = 0;
        self.current_element = None;
        self.current_attribute = None;
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get the error count
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// Add a validation error
    pub fn add_error(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    /// Raise or collect an error based on validation mode
    pub fn raise_or_collect(&mut self, error: ValidationError) -> Result<()> {
        match self.mode {
            ValidationMode::Strict => {
                // Convert to crate error type
                let crate_error = crate::error::ValidationError::new(error.message())
                    .with_reason(error.reason.clone().unwrap_or_default());
                Err(crate::error::Error::Validation(crate_error))
            }
            ValidationMode::Lax => {
                self.errors.push(error);
                Ok(())
            }
            ValidationMode::Skip => {
                Ok(())
            }
        }
    }

    /// Create a validation error and handle it according to mode
    pub fn validation_error(
        &mut self,
        message: impl Into<String>,
        reason: Option<String>,
    ) -> Result<()> {
        let mut error = ValidationError::new(message);
        if let Some(r) = reason {
            error = error.with_reason(r);
        }
        if let Some(ref elem) = self.current_element {
            error = error.with_element(elem.clone());
        }
        self.raise_or_collect(error)
    }

    /// Register an ID value
    pub fn register_id(&mut self, id: &str) -> Result<()> {
        let count = self.id_map.entry(id.to_string()).or_insert(0);
        *count += 1;

        if *count > 1 {
            self.validation_error(
                format!("Duplicate ID value: '{}'", id),
                Some("xs:ID values must be unique within the document".to_string()),
            )
        } else {
            Ok(())
        }
    }

    /// Check if an IDREF is valid
    pub fn check_idref(&self, idref: &str) -> bool {
        self.id_map.contains_key(idref)
    }

    /// Get all unresolved IDREFs
    pub fn get_unresolved_idrefs(&self, idrefs: &[String]) -> Vec<String> {
        idrefs
            .iter()
            .filter(|id| !self.id_map.contains_key(*id))
            .cloned()
            .collect()
    }
}

impl Default for ValidationContext {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ValidationContext {
    fn clone(&self) -> Self {
        Self {
            mode: self.mode,
            errors: self.errors.clone(),
            level: self.level,
            max_depth: self.max_depth,
            use_defaults: self.use_defaults,
            preserve_mixed: self.preserve_mixed,
            check_identities: self.check_identities,
            process_skipped: self.process_skipped,
            use_location_hints: self.use_location_hints,
            namespaces: self.namespaces.clone(),
            id_map: self.id_map.clone(),
            inherited: self.inherited.clone(),
            current_element: self.current_element.clone(),
            current_attribute: self.current_attribute.clone(),
        }
    }
}

/// Decode context for XML to value decoding
#[derive(Debug, Clone)]
pub struct DecodeContext {
    /// Base validation context
    pub validation: ValidationContext,
    /// Whether to keep datetime types as native types
    pub datetime_types: bool,
    /// Whether to keep binary types as native types
    pub binary_types: bool,
    /// Decimal type preference (None = keep as Decimal)
    pub decimal_type: Option<DecimalTypePreference>,
    /// Whether to fill missing optional elements
    pub fill_missing: bool,
    /// Whether to keep empty elements
    pub keep_empty: bool,
    /// Whether to keep unknown elements
    pub keep_unknown: bool,
}

/// Preference for decimal type conversion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecimalTypePreference {
    /// Convert to f64
    Float,
    /// Convert to string
    String,
}

impl DecodeContext {
    /// Create a new decode context
    pub fn new() -> Self {
        Self {
            validation: ValidationContext::new(),
            datetime_types: false,
            binary_types: false,
            decimal_type: None,
            fill_missing: false,
            keep_empty: false,
            keep_unknown: false,
        }
    }

    /// Set validation mode
    pub fn with_mode(mut self, mode: ValidationMode) -> Self {
        self.validation.mode = mode;
        self
    }
}

impl Default for DecodeContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Encode context for value to XML encoding
#[derive(Debug, Clone)]
pub struct EncodeContext {
    /// Base validation context
    pub validation: ValidationContext,
    /// Whether to allow unordered content
    pub unordered: bool,
    /// Whether data is untyped
    pub untyped_data: bool,
    /// Indentation level
    pub indent: usize,
}

impl EncodeContext {
    /// Create a new encode context
    pub fn new() -> Self {
        Self {
            validation: ValidationContext::new(),
            unordered: false,
            untyped_data: false,
            indent: 4,
        }
    }

    /// Set validation mode
    pub fn with_mode(mut self, mode: ValidationMode) -> Self {
        self.validation.mode = mode;
        self
    }

    /// Set indent level
    pub fn with_indent(mut self, indent: usize) -> Self {
        self.indent = indent;
        self
    }

    /// Allow unordered content
    pub fn with_unordered(mut self) -> Self {
        self.unordered = true;
        self
    }
}

impl Default for EncodeContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Outcome of validation with detailed error information
#[derive(Debug, Clone)]
pub struct ValidationOutcome {
    /// Whether validation passed
    pub is_valid: bool,
    /// Collected validation errors
    pub errors: Vec<ValidationError>,
    /// Collected warnings
    pub warnings: Vec<String>,
}

impl ValidationOutcome {
    /// Create a successful outcome
    pub fn success() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Create a failed outcome
    pub fn failure(errors: Vec<ValidationError>) -> Self {
        Self {
            is_valid: false,
            errors,
            warnings: Vec::new(),
        }
    }

    /// Add a warning
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }

    /// Check if there are any issues (errors or warnings)
    pub fn has_issues(&self) -> bool {
        !self.errors.is_empty() || !self.warnings.is_empty()
    }
}

/// Trait for types that can validate XML data
pub trait XmlValidator {
    /// Validate XML data
    fn validate(&self, data: &str, context: &mut ValidationContext) -> Result<()>;

    /// Check if data is valid
    fn is_valid(&self, data: &str) -> bool {
        let mut context = ValidationContext::new().with_mode(ValidationMode::Lax);
        self.validate(data, &mut context).is_ok() && !context.has_errors()
    }

    /// Get all validation errors
    fn iter_errors(&self, data: &str) -> Vec<ValidationError> {
        let mut context = ValidationContext::new().with_mode(ValidationMode::Lax);
        let _ = self.validate(data, &mut context);
        context.errors
    }
}

/// Trait for types that can decode XML to values
pub trait XmlDecoder<T> {
    /// Decode XML data to a value
    fn decode(&self, data: &str, context: &mut DecodeContext) -> Result<T>;

    /// Decode with lax validation, returning value and errors
    fn decode_lax(&self, data: &str) -> Result<(T, Vec<ValidationError>)> {
        let mut context = DecodeContext::new().with_mode(ValidationMode::Lax);
        let value = self.decode(data, &mut context)?;
        Ok((value, context.validation.errors))
    }
}

/// Trait for types that can encode values to XML
pub trait XmlEncoder<T> {
    /// Encode a value to XML
    fn encode(&self, value: &T, context: &mut EncodeContext) -> Result<String>;

    /// Encode with lax validation, returning XML and errors
    fn encode_lax(&self, value: &T) -> Result<(String, Vec<ValidationError>)> {
        let mut context = EncodeContext::new().with_mode(ValidationMode::Lax);
        let xml = self.encode(value, &mut context)?;
        Ok((xml, context.validation.errors))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_context_creation() {
        let context = ValidationContext::new();
        assert_eq!(context.mode, ValidationMode::Strict);
        assert_eq!(context.level, 0);
        assert!(!context.has_errors());
    }

    #[test]
    fn test_validation_context_with_mode() {
        let context = ValidationContext::new().with_mode(ValidationMode::Lax);
        assert_eq!(context.mode, ValidationMode::Lax);
    }

    #[test]
    fn test_validation_context_levels() {
        let mut context = ValidationContext::new();
        assert_eq!(context.level, 0);

        context.enter_level();
        assert_eq!(context.level, 1);

        context.enter_level();
        assert_eq!(context.level, 2);

        context.exit_level();
        assert_eq!(context.level, 1);
    }

    #[test]
    fn test_max_depth_check() {
        let mut context = ValidationContext::new().with_max_depth(2);

        assert!(!context.is_max_depth_exceeded());

        context.enter_level();
        assert!(!context.is_max_depth_exceeded());

        context.enter_level();
        assert!(context.is_max_depth_exceeded());
    }

    #[test]
    fn test_error_collection() {
        let mut context = ValidationContext::new().with_mode(ValidationMode::Lax);

        context.add_error(ValidationError::new("Error 1"));
        context.add_error(ValidationError::new("Error 2"));

        assert!(context.has_errors());
        assert_eq!(context.error_count(), 2);
    }

    #[test]
    fn test_id_registration() {
        let mut context = ValidationContext::new().with_mode(ValidationMode::Lax);

        // First registration should succeed
        context.register_id("id1").unwrap();
        assert!(context.check_idref("id1"));

        // Duplicate should add error
        context.register_id("id1").unwrap();
        assert!(context.has_errors());
    }

    #[test]
    fn test_unresolved_idrefs() {
        let mut context = ValidationContext::new();
        context.register_id("id1").unwrap();
        context.register_id("id2").unwrap();

        let idrefs = vec![
            "id1".to_string(),
            "id2".to_string(),
            "id3".to_string(),
        ];

        let unresolved = context.get_unresolved_idrefs(&idrefs);
        assert_eq!(unresolved, vec!["id3"]);
    }

    #[test]
    fn test_context_clear() {
        let mut context = ValidationContext::new();
        context.add_error(ValidationError::new("Error"));
        context.register_id("id1").unwrap();
        context.level = 5;

        context.clear();

        assert!(!context.has_errors());
        assert!(context.id_map.is_empty());
        assert_eq!(context.level, 0);
    }

    #[test]
    fn test_raise_or_collect_strict() {
        let mut context = ValidationContext::new().with_mode(ValidationMode::Strict);
        let result = context.raise_or_collect(ValidationError::new("Error"));
        assert!(result.is_err());
    }

    #[test]
    fn test_raise_or_collect_lax() {
        let mut context = ValidationContext::new().with_mode(ValidationMode::Lax);
        let result = context.raise_or_collect(ValidationError::new("Error"));
        assert!(result.is_ok());
        assert!(context.has_errors());
    }

    #[test]
    fn test_raise_or_collect_skip() {
        let mut context = ValidationContext::new().with_mode(ValidationMode::Skip);
        let result = context.raise_or_collect(ValidationError::new("Error"));
        assert!(result.is_ok());
        assert!(!context.has_errors()); // Errors not collected in skip mode
    }

    #[test]
    fn test_decode_context() {
        let context = DecodeContext::new().with_mode(ValidationMode::Lax);
        assert_eq!(context.validation.mode, ValidationMode::Lax);
    }

    #[test]
    fn test_encode_context() {
        let context = EncodeContext::new()
            .with_mode(ValidationMode::Lax)
            .with_indent(2)
            .with_unordered();

        assert_eq!(context.validation.mode, ValidationMode::Lax);
        assert_eq!(context.indent, 2);
        assert!(context.unordered);
    }

    #[test]
    fn test_validation_outcome_success() {
        let result = ValidationOutcome::success();
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validation_outcome_failure() {
        let errors = vec![ValidationError::new("Error")];
        let result = ValidationOutcome::failure(errors);
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_validation_outcome_with_warnings() {
        let result = ValidationOutcome::success()
            .with_warning("Warning 1")
            .with_warning("Warning 2");

        assert!(result.is_valid);
        assert!(result.has_issues());
        assert_eq!(result.warnings.len(), 2);
    }

    #[test]
    fn test_context_clone() {
        let mut context = ValidationContext::new();
        context.add_error(ValidationError::new("Error"));
        context.level = 3;

        let cloned = context.clone();
        assert_eq!(cloned.level, 3);
        assert_eq!(cloned.error_count(), 1);
    }
}
