//! XSD constraining facets
//!
//! This module implements XSD facets that constrain simple types.

use crate::error::{Result, ValidationError};
use regex::Regex;
use rust_decimal::Decimal;
use std::fmt;

/// White space handling modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhiteSpace {
    /// Preserve all white space
    Preserve,
    /// Replace tabs and newlines with spaces
    Replace,
    /// Replace and collapse multiple spaces
    Collapse,
}

impl WhiteSpace {
    /// Parse from string value
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "preserve" => Ok(WhiteSpace::Preserve),
            "replace" => Ok(WhiteSpace::Replace),
            "collapse" => Ok(WhiteSpace::Collapse),
            _ => Err(crate::error::Error::Value(format!(
                "Invalid whiteSpace value: '{}'. Must be 'preserve', 'replace', or 'collapse'",
                s
            ))),
        }
    }

    /// Normalize a string according to this white space mode
    pub fn normalize(&self, s: &str) -> String {
        match self {
            WhiteSpace::Preserve => s.to_string(),
            WhiteSpace::Replace => s.replace(['\t', '\n', '\r'], " "),
            WhiteSpace::Collapse => {
                let replaced = s.replace(['\t', '\n', '\r'], " ");
                let mut result = String::new();
                let mut prev_space = true; // Start with true to trim leading spaces

                for c in replaced.chars() {
                    if c == ' ' {
                        if !prev_space {
                            result.push(' ');
                            prev_space = true;
                        }
                    } else {
                        result.push(c);
                        prev_space = false;
                    }
                }

                result.trim_end().to_string()
            }
        }
    }

    /// Validate that a value conforms to this white space mode
    pub fn validate(&self, value: &str) -> Result<()> {
        match self {
            WhiteSpace::Preserve => Ok(()),
            WhiteSpace::Replace => {
                if value.contains(['\t', '\n', '\r']) {
                    Err(crate::error::Error::Validation(
                        ValidationError::new("Value contains tabs or newlines")
                            .with_reason("whiteSpace facet is 'replace'"),
                    ))
                } else {
                    Ok(())
                }
            }
            WhiteSpace::Collapse => {
                if value.contains(['\t', '\n', '\r']) || value.contains("  ") {
                    Err(crate::error::Error::Validation(
                        ValidationError::new("Value contains non-collapsed white spaces")
                            .with_reason("whiteSpace facet is 'collapse'"),
                    ))
                } else {
                    Ok(())
                }
            }
        }
    }
}

/// Length facet constrains the length of a value
#[derive(Debug, Clone)]
pub struct LengthFacet {
    /// Required length
    pub value: usize,
    /// Whether this facet is fixed
    pub fixed: bool,
}

impl LengthFacet {
    /// Create a new length facet
    pub fn new(value: usize) -> Self {
        Self {
            value,
            fixed: false,
        }
    }

    /// Create a fixed length facet
    pub fn fixed(value: usize) -> Self {
        Self { value, fixed: true }
    }

    /// Validate a value against this facet
    pub fn validate(&self, value: &str) -> Result<()> {
        let len = value.chars().count();
        if len != self.value {
            Err(crate::error::Error::Validation(
                ValidationError::new(format!("Length must be exactly {}", self.value))
                    .with_reason(format!("Actual length: {}", len)),
            ))
        } else {
            Ok(())
        }
    }
}

/// Minimum length facet
#[derive(Debug, Clone)]
pub struct MinLengthFacet {
    /// Minimum length
    pub value: usize,
    /// Whether this facet is fixed
    pub fixed: bool,
}

impl MinLengthFacet {
    /// Create a new minimum length facet
    pub fn new(value: usize) -> Self {
        Self {
            value,
            fixed: false,
        }
    }

    /// Validate a value against this facet
    pub fn validate(&self, value: &str) -> Result<()> {
        let len = value.chars().count();
        if len < self.value {
            Err(crate::error::Error::Validation(
                ValidationError::new(format!("Length must be at least {}", self.value))
                    .with_reason(format!("Actual length: {}", len)),
            ))
        } else {
            Ok(())
        }
    }
}

/// Maximum length facet
#[derive(Debug, Clone)]
pub struct MaxLengthFacet {
    /// Maximum length
    pub value: usize,
    /// Whether this facet is fixed
    pub fixed: bool,
}

impl MaxLengthFacet {
    /// Create a new maximum length facet
    pub fn new(value: usize) -> Self {
        Self {
            value,
            fixed: false,
        }
    }

    /// Validate a value against this facet
    pub fn validate(&self, value: &str) -> Result<()> {
        let len = value.chars().count();
        if len > self.value {
            Err(crate::error::Error::Validation(
                ValidationError::new(format!("Length must be at most {}", self.value))
                    .with_reason(format!("Actual length: {}", len)),
            ))
        } else {
            Ok(())
        }
    }
}

/// Pattern facet using regular expressions
#[derive(Debug, Clone)]
pub struct PatternFacet {
    /// Regular expression pattern
    pub pattern: String,
    /// Compiled regex
    regex: Regex,
}

impl PatternFacet {
    /// Create a new pattern facet
    pub fn new(pattern: &str) -> Result<Self> {
        let regex = Regex::new(pattern).map_err(|e| {
            crate::error::Error::Value(format!("Invalid pattern '{}': {}", pattern, e))
        })?;

        Ok(Self {
            pattern: pattern.to_string(),
            regex,
        })
    }

    /// Validate a value against this pattern
    pub fn validate(&self, value: &str) -> Result<()> {
        if self.regex.is_match(value) {
            Ok(())
        } else {
            Err(crate::error::Error::Validation(
                ValidationError::new(format!("Value does not match pattern '{}'", self.pattern))
                    .with_reason(format!("Value: '{}'", value)),
            ))
        }
    }
}

/// Enumeration facet restricts values to a specific set
#[derive(Debug, Clone)]
pub struct EnumerationFacet {
    /// Allowed values
    pub values: Vec<String>,
}

impl EnumerationFacet {
    /// Create a new enumeration facet
    pub fn new(values: Vec<String>) -> Self {
        Self { values }
    }

    /// Validate a value against this enumeration
    pub fn validate(&self, value: &str) -> Result<()> {
        if self.values.contains(&value.to_string()) {
            Ok(())
        } else {
            Err(crate::error::Error::Validation(
                ValidationError::new("Value is not in the enumeration")
                    .with_reason(format!("Allowed values: {:?}", self.values)),
            ))
        }
    }
}

/// Numeric bounds for validation
#[derive(Debug, Clone)]
pub enum NumericBound {
    /// Integer bound
    Integer(i64),
    /// Decimal bound
    Decimal(Decimal),
    /// Float bound
    Float(f64),
}

impl NumericBound {
    /// Compare with an integer value
    pub fn compare_int(&self, value: i64) -> std::cmp::Ordering {
        match self {
            NumericBound::Integer(bound) => value.cmp(bound),
            NumericBound::Decimal(bound) => {
                Decimal::from(value).cmp(bound)
            }
            NumericBound::Float(bound) => {
                let val_f64 = value as f64;
                if val_f64 < *bound {
                    std::cmp::Ordering::Less
                } else if val_f64 > *bound {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            }
        }
    }

    /// Compare with a decimal value
    pub fn compare_decimal(&self, value: &Decimal) -> std::cmp::Ordering {
        match self {
            NumericBound::Integer(bound) => {
                value.cmp(&Decimal::from(*bound))
            }
            NumericBound::Decimal(bound) => value.cmp(bound),
            NumericBound::Float(bound) => {
                let val_f64 = value.to_string().parse::<f64>().unwrap_or(0.0);
                if val_f64 < *bound {
                    std::cmp::Ordering::Less
                } else if val_f64 > *bound {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            }
        }
    }
}

impl fmt::Display for NumericBound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NumericBound::Integer(v) => write!(f, "{}", v),
            NumericBound::Decimal(v) => write!(f, "{}", v),
            NumericBound::Float(v) => write!(f, "{}", v),
        }
    }
}

/// Minimum inclusive bound facet
#[derive(Debug, Clone)]
pub struct MinInclusiveFacet {
    /// Minimum value (inclusive)
    pub value: NumericBound,
}

impl MinInclusiveFacet {
    /// Create a new minimum inclusive facet
    pub fn new_int(value: i64) -> Self {
        Self {
            value: NumericBound::Integer(value),
        }
    }

    /// Create a new minimum inclusive facet with decimal
    pub fn new_decimal(value: Decimal) -> Self {
        Self {
            value: NumericBound::Decimal(value),
        }
    }

    /// Validate an integer value
    pub fn validate_int(&self, value: i64) -> Result<()> {
        use std::cmp::Ordering;
        // compare_int returns: Less if value < bound, Equal if value == bound, Greater if value > bound
        // We error if value < bound
        match self.value.compare_int(value) {
            Ordering::Less => Err(crate::error::Error::Validation(
                ValidationError::new(format!("Value must be >= {}", self.value))
                    .with_reason(format!("Value: {}", value)),
            )),
            _ => Ok(()),
        }
    }

    /// Validate a decimal value
    pub fn validate_decimal(&self, value: &Decimal) -> Result<()> {
        use std::cmp::Ordering;
        match self.value.compare_decimal(value) {
            Ordering::Less => Err(crate::error::Error::Validation(
                ValidationError::new(format!("Value must be >= {}", self.value))
                    .with_reason(format!("Value: {}", value)),
            )),
            _ => Ok(()),
        }
    }
}

/// Maximum inclusive bound facet
#[derive(Debug, Clone)]
pub struct MaxInclusiveFacet {
    /// Maximum value (inclusive)
    pub value: NumericBound,
}

impl MaxInclusiveFacet {
    /// Create a new maximum inclusive facet
    pub fn new_int(value: i64) -> Self {
        Self {
            value: NumericBound::Integer(value),
        }
    }

    /// Validate an integer value
    pub fn validate_int(&self, value: i64) -> Result<()> {
        use std::cmp::Ordering;
        // compare_int returns: Less if value < bound, Equal if value == bound, Greater if value > bound
        // We error if value > bound
        match self.value.compare_int(value) {
            Ordering::Greater => Err(crate::error::Error::Validation(
                ValidationError::new(format!("Value must be <= {}", self.value))
                    .with_reason(format!("Value: {}", value)),
            )),
            _ => Ok(()),
        }
    }
}

/// Minimum exclusive bound facet
#[derive(Debug, Clone)]
pub struct MinExclusiveFacet {
    /// Minimum value (exclusive - value must be > than this)
    pub value: NumericBound,
}

impl MinExclusiveFacet {
    /// Create a new minimum exclusive facet
    pub fn new_int(value: i64) -> Self {
        Self {
            value: NumericBound::Integer(value),
        }
    }

    /// Create a new minimum exclusive facet with a decimal value
    pub fn new_decimal(value: Decimal) -> Self {
        Self {
            value: NumericBound::Decimal(value),
        }
    }

    /// Validate an integer value
    pub fn validate_int(&self, value: i64) -> Result<()> {
        use std::cmp::Ordering;
        // Value must be strictly greater than bound
        // compare_int returns: Less if value < bound, Equal if ==, Greater if value > bound
        match self.value.compare_int(value) {
            Ordering::Greater => Ok(()), // value > bound is valid
            Ordering::Equal | Ordering::Less => Err(crate::error::Error::Validation(
                ValidationError::new(format!("Value must be > {}", self.value))
                    .with_reason(format!("Value: {}", value)),
            )),
        }
    }

    /// Validate a decimal value
    pub fn validate_decimal(&self, value: &Decimal) -> Result<()> {
        use std::cmp::Ordering;
        match self.value.compare_decimal(value) {
            Ordering::Greater => Ok(()), // value > bound is valid
            Ordering::Equal | Ordering::Less => Err(crate::error::Error::Validation(
                ValidationError::new(format!("Value must be > {}", self.value))
                    .with_reason(format!("Value: {}", value)),
            )),
        }
    }
}

/// Maximum exclusive bound facet
#[derive(Debug, Clone)]
pub struct MaxExclusiveFacet {
    /// Maximum value (exclusive - value must be < than this)
    pub value: NumericBound,
}

impl MaxExclusiveFacet {
    /// Create a new maximum exclusive facet
    pub fn new_int(value: i64) -> Self {
        Self {
            value: NumericBound::Integer(value),
        }
    }

    /// Create a new maximum exclusive facet with a decimal value
    pub fn new_decimal(value: Decimal) -> Self {
        Self {
            value: NumericBound::Decimal(value),
        }
    }

    /// Validate an integer value
    pub fn validate_int(&self, value: i64) -> Result<()> {
        use std::cmp::Ordering;
        // Value must be strictly less than bound
        // compare_int returns: Less if value < bound, Equal if ==, Greater if value > bound
        match self.value.compare_int(value) {
            Ordering::Less => Ok(()), // value < bound is valid
            Ordering::Equal | Ordering::Greater => Err(crate::error::Error::Validation(
                ValidationError::new(format!("Value must be < {}", self.value))
                    .with_reason(format!("Value: {}", value)),
            )),
        }
    }

    /// Validate a decimal value
    pub fn validate_decimal(&self, value: &Decimal) -> Result<()> {
        use std::cmp::Ordering;
        match self.value.compare_decimal(value) {
            Ordering::Less => Ok(()), // value < bound is valid
            Ordering::Equal | Ordering::Greater => Err(crate::error::Error::Validation(
                ValidationError::new(format!("Value must be < {}", self.value))
                    .with_reason(format!("Value: {}", value)),
            )),
        }
    }
}

/// Total digits facet - constrains the maximum number of decimal digits
#[derive(Debug, Clone)]
pub struct TotalDigitsFacet {
    /// Maximum total number of digits allowed
    pub value: u32,
}

impl TotalDigitsFacet {
    /// Create a new total digits facet
    pub fn new(value: u32) -> Self {
        Self { value }
    }

    /// Validate an integer value
    pub fn validate_int(&self, value: i64) -> Result<()> {
        let abs_value = value.abs();
        let digits = if abs_value == 0 {
            1
        } else {
            ((abs_value as f64).log10().floor() as u32) + 1
        };

        if digits > self.value {
            Err(crate::error::Error::Validation(
                ValidationError::new(format!("Value exceeds totalDigits limit of {}", self.value))
                    .with_reason(format!("Value {} has {} digits", value, digits)),
            ))
        } else {
            Ok(())
        }
    }

    /// Validate a decimal value
    pub fn validate_decimal(&self, value: &Decimal) -> Result<()> {
        // For decimals, count all significant digits (before and after decimal point)
        // Normalize to remove trailing zeros and get the actual significant digits
        let normalized = value.normalize();
        let s = normalized.to_string();
        let digit_count: u32 = s
            .chars()
            .filter(|c| c.is_ascii_digit())
            .count() as u32;

        if digit_count > self.value {
            Err(crate::error::Error::Validation(
                ValidationError::new(format!("Value exceeds totalDigits limit of {}", self.value))
                    .with_reason(format!("Value {} has {} significant digits", value, digit_count)),
            ))
        } else {
            Ok(())
        }
    }
}

/// Fraction digits facet - constrains the maximum number of decimal places
#[derive(Debug, Clone)]
pub struct FractionDigitsFacet {
    /// Maximum number of fractional digits allowed
    pub value: u32,
}

impl FractionDigitsFacet {
    /// Create a new fraction digits facet
    pub fn new(value: u32) -> Self {
        Self { value }
    }

    /// Validate a decimal value
    pub fn validate_decimal(&self, value: &Decimal) -> Result<()> {
        let scale = value.scale();

        if scale > self.value {
            Err(crate::error::Error::Validation(
                ValidationError::new(format!("Value exceeds fractionDigits limit of {}", self.value))
                    .with_reason(format!("Value {} has {} fractional digits", value, scale)),
            ))
        } else {
            Ok(())
        }
    }
}

// TODO: Implement more facets:
// - AssertionFacet (XSD 1.1)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whitespace_modes() {
        assert_eq!(WhiteSpace::from_str("preserve").unwrap(), WhiteSpace::Preserve);
        assert_eq!(WhiteSpace::from_str("replace").unwrap(), WhiteSpace::Replace);
        assert_eq!(WhiteSpace::from_str("collapse").unwrap(), WhiteSpace::Collapse);
        assert!(WhiteSpace::from_str("invalid").is_err());
    }

    #[test]
    fn test_whitespace_normalize() {
        let text = "  hello\t\nworld  ";

        assert_eq!(WhiteSpace::Preserve.normalize(text), text);
        assert_eq!(WhiteSpace::Replace.normalize(text), "  hello  world  ");
        assert_eq!(WhiteSpace::Collapse.normalize(text), "hello world");
    }

    #[test]
    fn test_whitespace_validate() {
        assert!(WhiteSpace::Preserve.validate("hello\tworld").is_ok());
        assert!(WhiteSpace::Replace.validate("hello\tworld").is_err());
        assert!(WhiteSpace::Replace.validate("hello world").is_ok());
        assert!(WhiteSpace::Collapse.validate("hello  world").is_err());
        assert!(WhiteSpace::Collapse.validate("hello world").is_ok());
    }

    #[test]
    fn test_length_facet() {
        let facet = LengthFacet::new(5);

        assert!(facet.validate("hello").is_ok());
        assert!(facet.validate("hi").is_err());
        assert!(facet.validate("toolong").is_err());
    }

    #[test]
    fn test_min_length_facet() {
        let facet = MinLengthFacet::new(3);

        assert!(facet.validate("hello").is_ok());
        assert!(facet.validate("hi").is_err());
        assert!(facet.validate("abc").is_ok());
    }

    #[test]
    fn test_max_length_facet() {
        let facet = MaxLengthFacet::new(5);

        assert!(facet.validate("hello").is_ok());
        assert!(facet.validate("hi").is_ok());
        assert!(facet.validate("toolong").is_err());
    }

    #[test]
    fn test_pattern_facet() {
        let facet = PatternFacet::new(r"^\d{3}-\d{4}$").unwrap();

        assert!(facet.validate("123-4567").is_ok());
        assert!(facet.validate("123-456").is_err());
        assert!(facet.validate("abc-4567").is_err());
    }

    #[test]
    fn test_enumeration_facet() {
        let facet = EnumerationFacet::new(vec![
            "red".to_string(),
            "green".to_string(),
            "blue".to_string(),
        ]);

        assert!(facet.validate("red").is_ok());
        assert!(facet.validate("green").is_ok());
        assert!(facet.validate("yellow").is_err());
    }

    #[test]
    fn test_min_inclusive_facet() {
        let facet = MinInclusiveFacet::new_int(10);

        assert!(facet.validate_int(10).is_ok());
        assert!(facet.validate_int(11).is_ok());
        assert!(facet.validate_int(9).is_err());
    }

    #[test]
    fn test_max_inclusive_facet() {
        let facet = MaxInclusiveFacet::new_int(100);

        assert!(facet.validate_int(100).is_ok());
        assert!(facet.validate_int(99).is_ok());
        assert!(facet.validate_int(101).is_err());
    }

    #[test]
    fn test_numeric_bound_comparison() {
        let int_bound = NumericBound::Integer(10);
        let dec_bound = NumericBound::Decimal(Decimal::new(105, 1)); // 10.5

        assert_eq!(int_bound.compare_int(10), std::cmp::Ordering::Equal);
        assert_eq!(int_bound.compare_int(11), std::cmp::Ordering::Greater);
        assert_eq!(int_bound.compare_int(9), std::cmp::Ordering::Less);

        assert_eq!(dec_bound.compare_decimal(&Decimal::new(105, 1)), std::cmp::Ordering::Equal);
        assert_eq!(dec_bound.compare_decimal(&Decimal::new(110, 1)), std::cmp::Ordering::Greater);
    }

    #[test]
    fn test_min_exclusive_facet() {
        let facet = MinExclusiveFacet::new_int(10);

        assert!(facet.validate_int(11).is_ok());
        assert!(facet.validate_int(100).is_ok());
        assert!(facet.validate_int(10).is_err()); // Equal is not allowed
        assert!(facet.validate_int(9).is_err());
    }

    #[test]
    fn test_max_exclusive_facet() {
        let facet = MaxExclusiveFacet::new_int(100);

        assert!(facet.validate_int(99).is_ok());
        assert!(facet.validate_int(0).is_ok());
        assert!(facet.validate_int(100).is_err()); // Equal is not allowed
        assert!(facet.validate_int(101).is_err());
    }

    #[test]
    fn test_total_digits_facet() {
        let facet = TotalDigitsFacet::new(5);

        assert!(facet.validate_int(12345).is_ok());
        assert!(facet.validate_int(1234).is_ok());
        assert!(facet.validate_int(123456).is_err());
        assert!(facet.validate_int(0).is_ok());
        assert!(facet.validate_int(-12345).is_ok());

        // Decimal tests
        assert!(facet.validate_decimal(&Decimal::new(12345, 0)).is_ok()); // 12345
        assert!(facet.validate_decimal(&Decimal::new(1234, 1)).is_ok());  // 123.4 (4 digits)
        assert!(facet.validate_decimal(&Decimal::new(123456, 0)).is_err()); // 123456 (6 digits)
    }

    #[test]
    fn test_fraction_digits_facet() {
        let facet = FractionDigitsFacet::new(2);

        assert!(facet.validate_decimal(&Decimal::new(123, 2)).is_ok());   // 1.23
        assert!(facet.validate_decimal(&Decimal::new(12, 1)).is_ok());    // 1.2
        assert!(facet.validate_decimal(&Decimal::new(1234, 3)).is_err()); // 1.234 (3 fraction digits)
    }
}
