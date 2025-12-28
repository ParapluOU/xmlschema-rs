//! Base validator infrastructure
//!
//! This module provides the foundation for all XSD validators.

use crate::error::{ParseError, Result, ValidationError};
use crate::namespaces::QName;
use std::fmt;

/// Validation mode for XSD validation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationMode {
    /// Strict validation - all errors are fatal
    Strict,
    /// Lax validation - some errors are warnings
    Lax,
    /// Skip validation - no validation is performed
    Skip,
}

impl ValidationMode {
    /// Parse validation mode from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "strict" => Ok(ValidationMode::Strict),
            "lax" => Ok(ValidationMode::Lax),
            "skip" => Ok(ValidationMode::Skip),
            _ => Err(crate::error::Error::Value(format!(
                "Invalid validation mode: '{}'. Must be 'strict', 'lax', or 'skip'",
                s
            ))),
        }
    }

    /// Get the mode as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            ValidationMode::Strict => "strict",
            ValidationMode::Lax => "lax",
            ValidationMode::Skip => "skip",
        }
    }
}

impl Default for ValidationMode {
    fn default() -> Self {
        ValidationMode::Strict
    }
}

impl fmt::Display for ValidationMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Validation status of a component
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationStatus {
    /// Fully validated
    Full,
    /// Partially validated
    Partial,
    /// Not validated
    None,
}

/// Validity status of a component
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidityStatus {
    /// Valid according to the schema
    Valid,
    /// Invalid according to the schema
    Invalid,
    /// Validity is unknown
    NotKnown,
}

/// Base trait for all XSD validators
pub trait Validator: fmt::Debug {
    /// Check if the validator has been fully built
    fn is_built(&self) -> bool;

    /// Build the validator and its components
    fn build(&mut self) -> Result<()>;

    /// Get the validation status
    fn validation_attempted(&self) -> ValidationStatus;

    /// Get the validity status
    fn validity(&self, mode: ValidationMode) -> ValidityStatus {
        match mode {
            ValidationMode::Skip => ValidityStatus::NotKnown,
            _ => {
                if self.has_errors() {
                    ValidityStatus::Invalid
                } else if self.validation_attempted() == ValidationStatus::Full {
                    ValidityStatus::Valid
                } else {
                    ValidityStatus::NotKnown
                }
            }
        }
    }

    /// Check if the validator has errors
    fn has_errors(&self) -> bool;

    /// Get all building errors
    fn errors(&self) -> Vec<ParseError>;

    /// Check the validator status against a validation mode
    fn check_validator(&self, mode: ValidationMode) -> Result<()> {
        if self.validation_attempted() == ValidationStatus::None
            && mode != ValidationMode::Skip
        {
            return Err(crate::error::Error::Parse(
                ParseError::new("Validator is not built"),
            ));
        }

        if mode == ValidationMode::Strict {
            if self.validation_attempted() != ValidationStatus::Full {
                return Err(crate::error::Error::Parse(ParseError::new(
                    "Validation mode is 'strict' but validator is not fully built",
                )));
            }
            if self.validity(mode) != ValidityStatus::Valid {
                return Err(crate::error::Error::Parse(ParseError::new(
                    "Validation mode is 'strict' but validator is not valid",
                )));
            }
        }

        Ok(())
    }
}

/// Base trait for type validators (simple and complex types)
pub trait TypeValidator: Validator {
    /// Get the type name
    fn name(&self) -> Option<&QName>;

    /// Check if this is a built-in type
    fn is_builtin(&self) -> bool;

    /// Get the base type (for derived types)
    fn base_type(&self) -> Option<&dyn TypeValidator>;
}

/// Base trait for element validators
pub trait ElementValidator: Validator {
    /// Get the element name
    fn name(&self) -> &QName;

    /// Get the element type
    fn element_type(&self) -> Option<&dyn TypeValidator>;

    /// Check if the element is nillable
    fn is_nillable(&self) -> bool;

    /// Get the default value (if any)
    fn default_value(&self) -> Option<&str>;

    /// Get the fixed value (if any)
    fn fixed_value(&self) -> Option<&str>;
}

/// Base trait for attribute validators
pub trait AttributeValidator: Validator {
    /// Get the attribute name
    fn name(&self) -> &QName;

    /// Get the attribute type
    fn attribute_type(&self) -> Option<&dyn TypeValidator>;

    /// Check if the attribute is required
    fn is_required(&self) -> bool;

    /// Get the default value (if any)
    fn default_value(&self) -> Option<&str>;

    /// Get the fixed value (if any)
    fn fixed_value(&self) -> Option<&str>;
}

/// Base validator component
#[derive(Debug)]
pub struct XsdValidator {
    /// Validation mode
    pub mode: ValidationMode,
    /// Building errors
    pub errors: Vec<ParseError>,
    /// Whether the validator is built
    pub built: bool,
}

impl XsdValidator {
    /// Create a new validator with strict mode
    pub fn new() -> Self {
        Self {
            mode: ValidationMode::Strict,
            errors: Vec::new(),
            built: false,
        }
    }

    /// Create a validator with a specific mode
    pub fn with_mode(mode: ValidationMode) -> Self {
        Self {
            mode,
            errors: Vec::new(),
            built: false,
        }
    }

    /// Add a parse error
    pub fn add_error(&mut self, error: ParseError) {
        self.errors.push(error);
    }

    /// Mark as built
    pub fn mark_built(&mut self) {
        self.built = true;
    }
}

impl Default for XsdValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator for XsdValidator {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_mode() {
        assert_eq!(ValidationMode::from_str("strict").unwrap(), ValidationMode::Strict);
        assert_eq!(ValidationMode::from_str("lax").unwrap(), ValidationMode::Lax);
        assert_eq!(ValidationMode::from_str("skip").unwrap(), ValidationMode::Skip);
        assert!(ValidationMode::from_str("invalid").is_err());
    }

    #[test]
    fn test_validation_mode_display() {
        assert_eq!(ValidationMode::Strict.to_string(), "strict");
        assert_eq!(ValidationMode::Lax.to_string(), "lax");
        assert_eq!(ValidationMode::Skip.to_string(), "skip");
    }

    #[test]
    fn test_validator_creation() {
        let validator = XsdValidator::new();
        assert_eq!(validator.mode, ValidationMode::Strict);
        assert!(!validator.built);
        assert!(validator.errors.is_empty());
    }

    #[test]
    fn test_validator_build() {
        let mut validator = XsdValidator::new();
        assert!(!validator.is_built());

        validator.build().unwrap();
        assert!(validator.is_built());
    }

    #[test]
    fn test_validation_status() {
        let mut validator = XsdValidator::new();
        assert_eq!(validator.validation_attempted(), ValidationStatus::None);

        validator.build().unwrap();
        assert_eq!(validator.validation_attempted(), ValidationStatus::Full);

        validator.add_error(ParseError::new("test error"));
        assert_eq!(validator.validation_attempted(), ValidationStatus::Partial);
    }

    #[test]
    fn test_validity_status() {
        let mut validator = XsdValidator::new();
        validator.build().unwrap();

        assert_eq!(validator.validity(ValidationMode::Strict), ValidityStatus::Valid);
        assert_eq!(validator.validity(ValidationMode::Skip), ValidityStatus::NotKnown);

        validator.add_error(ParseError::new("test error"));
        assert_eq!(validator.validity(ValidationMode::Strict), ValidityStatus::Invalid);
    }

    #[test]
    fn test_check_validator_strict() {
        let mut validator = XsdValidator::new();

        // Not built - should fail in strict mode
        assert!(validator.check_validator(ValidationMode::Strict).is_err());

        // Build it
        validator.build().unwrap();
        assert!(validator.check_validator(ValidationMode::Strict).is_ok());

        // Add error - should fail in strict mode
        validator.add_error(ParseError::new("test error"));
        assert!(validator.check_validator(ValidationMode::Strict).is_err());
    }

    #[test]
    fn test_check_validator_lax() {
        let mut validator = XsdValidator::new();

        // Not built - should fail in lax mode
        assert!(validator.check_validator(ValidationMode::Lax).is_err());

        // Build it
        validator.build().unwrap();
        assert!(validator.check_validator(ValidationMode::Lax).is_ok());

        // Add error - should still pass in lax mode
        validator.add_error(ParseError::new("test error"));
        assert!(validator.check_validator(ValidationMode::Lax).is_ok());
    }

    #[test]
    fn test_check_validator_skip() {
        let validator = XsdValidator::new();

        // Not built - should still pass in skip mode
        assert!(validator.check_validator(ValidationMode::Skip).is_ok());
    }
}
