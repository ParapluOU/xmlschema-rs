//! Validator helper functions
//!
//! This module provides utility functions for XSD validation,
//! including type validators and conversion functions.

use crate::error::{Error, Result, ValidationError};
use base64::Engine;
use rust_decimal::Decimal;
use std::collections::HashMap;

/// XSD final attribute values
pub const XSD_FINAL_ATTRIBUTE_VALUES: &[&str] = &["restriction", "extension", "list", "union"];

lazy_static::lazy_static! {
    /// XSD boolean value mapping
    pub static ref XSD_BOOLEAN_MAP: HashMap<&'static str, bool> = {
        let mut m = HashMap::new();
        m.insert("false", false);
        m.insert("0", false);
        m.insert("true", true);
        m.insert("1", true);
        m
    };
}

// =============================================================================
// Numeric Validators
// =============================================================================

/// Validate a decimal value
pub fn decimal_validator(value: &str) -> Result<Decimal> {
    value.trim().parse::<Decimal>().map_err(|_| {
        Error::Validation(ValidationError::new("value is not a valid xs:decimal"))
    })
}

/// Validate a byte value (-128 to 127)
pub fn byte_validator(value: i64) -> Result<()> {
    if !(-128..=127).contains(&value) {
        return Err(Error::Validation(
            ValidationError::new("value must be -128 <= x <= 127")
                .with_reason(format!("Actual value: {}", value)),
        ));
    }
    Ok(())
}

/// Validate a short value (-32768 to 32767)
pub fn short_validator(value: i64) -> Result<()> {
    if !(-32768..=32767).contains(&value) {
        return Err(Error::Validation(
            ValidationError::new("value must be -32768 <= x <= 32767")
                .with_reason(format!("Actual value: {}", value)),
        ));
    }
    Ok(())
}

/// Validate an int value (-2^31 to 2^31-1)
pub fn int_validator(value: i64) -> Result<()> {
    if !(-2147483648..=2147483647).contains(&value) {
        return Err(Error::Validation(
            ValidationError::new("value must be -2147483648 <= x <= 2147483647")
                .with_reason(format!("Actual value: {}", value)),
        ));
    }
    Ok(())
}

/// Validate a long value (-2^63 to 2^63-1)
pub fn long_validator(value: i64) -> Result<()> {
    // i64 can represent the full range, so this is always valid
    let _ = value;
    Ok(())
}

/// Validate an unsigned byte value (0 to 255)
pub fn unsigned_byte_validator(value: i64) -> Result<()> {
    if !(0..=255).contains(&value) {
        return Err(Error::Validation(
            ValidationError::new("value must be 0 <= x <= 255")
                .with_reason(format!("Actual value: {}", value)),
        ));
    }
    Ok(())
}

/// Validate an unsigned short value (0 to 65535)
pub fn unsigned_short_validator(value: i64) -> Result<()> {
    if !(0..=65535).contains(&value) {
        return Err(Error::Validation(
            ValidationError::new("value must be 0 <= x <= 65535")
                .with_reason(format!("Actual value: {}", value)),
        ));
    }
    Ok(())
}

/// Validate an unsigned int value (0 to 2^32-1)
pub fn unsigned_int_validator(value: i64) -> Result<()> {
    if !(0..=4294967295).contains(&value) {
        return Err(Error::Validation(
            ValidationError::new("value must be 0 <= x <= 4294967295")
                .with_reason(format!("Actual value: {}", value)),
        ));
    }
    Ok(())
}

/// Validate an unsigned long value (0 to 2^64-1)
pub fn unsigned_long_validator(value: u64) -> Result<()> {
    // u64 can represent the full range, so this is always valid
    let _ = value;
    Ok(())
}

/// Validate a negative integer value (< 0)
pub fn negative_int_validator(value: i64) -> Result<()> {
    if value >= 0 {
        return Err(Error::Validation(
            ValidationError::new("value must be negative")
                .with_reason(format!("Actual value: {}", value)),
        ));
    }
    Ok(())
}

/// Validate a positive integer value (> 0)
pub fn positive_int_validator(value: i64) -> Result<()> {
    if value <= 0 {
        return Err(Error::Validation(
            ValidationError::new("value must be positive")
                .with_reason(format!("Actual value: {}", value)),
        ));
    }
    Ok(())
}

/// Validate a non-positive integer value (<= 0)
pub fn non_positive_int_validator(value: i64) -> Result<()> {
    if value > 0 {
        return Err(Error::Validation(
            ValidationError::new("value must be non-positive")
                .with_reason(format!("Actual value: {}", value)),
        ));
    }
    Ok(())
}

/// Validate a non-negative integer value (>= 0)
pub fn non_negative_int_validator(value: i64) -> Result<()> {
    if value < 0 {
        return Err(Error::Validation(
            ValidationError::new("value must be non-negative")
                .with_reason(format!("Actual value: {}", value)),
        ));
    }
    Ok(())
}

// =============================================================================
// Binary Validators
// =============================================================================

/// Pattern for validating hexadecimal binary strings
const HEX_BINARY_PATTERN: &str = r"^([0-9a-fA-F]{2})*$";

lazy_static::lazy_static! {
    static ref HEX_BINARY_REGEX: regex::Regex = regex::Regex::new(HEX_BINARY_PATTERN).unwrap();
}

/// Validate a hex binary value
pub fn hex_binary_validator(value: &str) -> Result<Vec<u8>> {
    if !HEX_BINARY_REGEX.is_match(value) {
        return Err(Error::Validation(
            ValidationError::new("not a valid hexadecimal encoding"),
        ));
    }

    // Decode hex string
    (0..value.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&value[i..i + 2], 16).map_err(|_| {
                Error::Validation(ValidationError::new("invalid hex byte"))
            })
        })
        .collect()
}

/// Validate a base64 binary value
pub fn base64_binary_validator(value: &str) -> Result<Vec<u8>> {
    let cleaned = value.replace(' ', "");
    if cleaned.is_empty() {
        return Ok(Vec::new());
    }

    base64::engine::general_purpose::STANDARD
        .decode(&cleaned)
        .map_err(|_| Error::Validation(ValidationError::new("not a valid base64 encoding")))
}

// =============================================================================
// QName Validator
// =============================================================================

/// QName pattern (prefix:localname or just localname)
const QNAME_PATTERN: &str = r"^([A-Za-z_][A-Za-z0-9._-]*:)?[A-Za-z_][A-Za-z0-9._-]*$";

lazy_static::lazy_static! {
    static ref QNAME_REGEX: regex::Regex = regex::Regex::new(QNAME_PATTERN).unwrap();
}

/// Validate a QName value
pub fn qname_validator(value: &str) -> Result<()> {
    if !QNAME_REGEX.is_match(value) {
        return Err(Error::Validation(
            ValidationError::new("value is not a valid xs:QName"),
        ));
    }
    Ok(())
}

// =============================================================================
// Error Type Validator
// =============================================================================

/// Validator for xs:error type - always fails
pub fn error_type_validator<T>(_value: &T) -> Result<()> {
    Err(Error::Validation(ValidationError::new(
        "no value is allowed for xs:error type",
    )))
}

// =============================================================================
// Boolean Conversions
// =============================================================================

/// Convert XSD boolean string to Rust bool
pub fn boolean_to_rust(value: &str) -> Result<bool> {
    XSD_BOOLEAN_MAP.get(value).copied().ok_or_else(|| {
        Error::Value(format!("'{}' is not a valid boolean value", value))
    })
}

/// Convert Rust bool to XSD boolean string
pub fn rust_to_boolean(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

// =============================================================================
// Float Conversions
// =============================================================================

/// Convert Rust float to XSD float string
pub fn rust_to_float(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_string()
    } else if value == f64::INFINITY {
        "INF".to_string()
    } else if value == f64::NEG_INFINITY {
        "-INF".to_string()
    } else {
        value.to_string()
    }
}

/// Convert XSD float string to Rust float
pub fn float_to_rust(value: &str) -> Result<f64> {
    match value {
        "NaN" => Ok(f64::NAN),
        "INF" => Ok(f64::INFINITY),
        "-INF" => Ok(f64::NEG_INFINITY),
        _ => value.parse::<f64>().map_err(|_| {
            Error::Value(format!("'{}' is not a valid float value", value))
        }),
    }
}

// =============================================================================
// Integer Conversions
// =============================================================================

/// Convert Rust int to XSD integer string
pub fn rust_to_int(value: i64) -> String {
    value.to_string()
}

/// Convert XSD integer string to Rust int
pub fn int_to_rust(value: &str) -> Result<i64> {
    value.trim().parse::<i64>().map_err(|_| {
        Error::Value(format!("'{}' is not a valid integer value", value))
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_validator() {
        assert!(byte_validator(0).is_ok());
        assert!(byte_validator(-128).is_ok());
        assert!(byte_validator(127).is_ok());
        assert!(byte_validator(-129).is_err());
        assert!(byte_validator(128).is_err());
    }

    #[test]
    fn test_short_validator() {
        assert!(short_validator(0).is_ok());
        assert!(short_validator(-32768).is_ok());
        assert!(short_validator(32767).is_ok());
        assert!(short_validator(-32769).is_err());
        assert!(short_validator(32768).is_err());
    }

    #[test]
    fn test_int_validator() {
        assert!(int_validator(0).is_ok());
        assert!(int_validator(-2147483648).is_ok());
        assert!(int_validator(2147483647).is_ok());
        assert!(int_validator(-2147483649).is_err());
        assert!(int_validator(2147483648).is_err());
    }

    #[test]
    fn test_unsigned_byte_validator() {
        assert!(unsigned_byte_validator(0).is_ok());
        assert!(unsigned_byte_validator(255).is_ok());
        assert!(unsigned_byte_validator(-1).is_err());
        assert!(unsigned_byte_validator(256).is_err());
    }

    #[test]
    fn test_negative_int_validator() {
        assert!(negative_int_validator(-1).is_ok());
        assert!(negative_int_validator(-100).is_ok());
        assert!(negative_int_validator(0).is_err());
        assert!(negative_int_validator(1).is_err());
    }

    #[test]
    fn test_positive_int_validator() {
        assert!(positive_int_validator(1).is_ok());
        assert!(positive_int_validator(100).is_ok());
        assert!(positive_int_validator(0).is_err());
        assert!(positive_int_validator(-1).is_err());
    }

    #[test]
    fn test_hex_binary_validator() {
        assert!(hex_binary_validator("").is_ok());
        assert!(hex_binary_validator("0A").is_ok());
        assert!(hex_binary_validator("0a1B2c").is_ok());
        assert!(hex_binary_validator("0").is_err()); // odd number of chars
        assert!(hex_binary_validator("GH").is_err()); // invalid chars
    }

    #[test]
    fn test_base64_binary_validator() {
        assert!(base64_binary_validator("").is_ok());
        assert!(base64_binary_validator("SGVsbG8=").is_ok());
        assert!(base64_binary_validator("SGVs bG8=").is_ok()); // with space
        assert!(base64_binary_validator("!!!").is_err());
    }

    #[test]
    fn test_qname_validator() {
        assert!(qname_validator("element").is_ok());
        assert!(qname_validator("xs:element").is_ok());
        assert!(qname_validator("_element").is_ok());
        assert!(qname_validator("123element").is_err());
        assert!(qname_validator("").is_err());
    }

    #[test]
    fn test_decimal_validator() {
        assert!(decimal_validator("123").is_ok());
        assert!(decimal_validator("123.456").is_ok());
        assert!(decimal_validator("-123.456").is_ok());
        assert!(decimal_validator("abc").is_err());
    }

    #[test]
    fn test_boolean_conversion() {
        assert_eq!(boolean_to_rust("true").unwrap(), true);
        assert_eq!(boolean_to_rust("false").unwrap(), false);
        assert_eq!(boolean_to_rust("1").unwrap(), true);
        assert_eq!(boolean_to_rust("0").unwrap(), false);
        assert!(boolean_to_rust("yes").is_err());

        assert_eq!(rust_to_boolean(true), "true");
        assert_eq!(rust_to_boolean(false), "false");
    }

    #[test]
    fn test_float_conversion() {
        assert_eq!(rust_to_float(f64::NAN), "NaN");
        assert_eq!(rust_to_float(f64::INFINITY), "INF");
        assert_eq!(rust_to_float(f64::NEG_INFINITY), "-INF");
        assert_eq!(rust_to_float(123.456), "123.456");

        assert!(float_to_rust("NaN").unwrap().is_nan());
        assert_eq!(float_to_rust("INF").unwrap(), f64::INFINITY);
        assert_eq!(float_to_rust("-INF").unwrap(), f64::NEG_INFINITY);
        assert_eq!(float_to_rust("123.456").unwrap(), 123.456);
    }

    #[test]
    fn test_int_conversion() {
        assert_eq!(rust_to_int(123), "123");
        assert_eq!(rust_to_int(-456), "-456");

        assert_eq!(int_to_rust("123").unwrap(), 123);
        assert_eq!(int_to_rust("-456").unwrap(), -456);
        assert!(int_to_rust("abc").is_err());
    }

    #[test]
    fn test_error_type_validator() {
        assert!(error_type_validator(&"anything").is_err());
        assert!(error_type_validator(&123).is_err());
    }
}
