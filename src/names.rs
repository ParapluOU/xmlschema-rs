//! XML name validation and utilities
//!
//! This module provides validation for XML names, NCNames, and QNames
//! according to XML specifications.

use crate::error::{Error, Result};
use once_cell::sync::Lazy;
use regex::Regex;

// XML Name patterns (simplified - should follow XML spec exactly)
static NAME_START_CHAR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[A-Z_a-z\u{C0}-\u{D6}\u{D8}-\u{F6}\u{F8}-\u{2FF}\u{370}-\u{37D}]").unwrap()
});

static NAME_CHAR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[A-Z_a-z\u{C0}-\u{D6}\u{D8}-\u{F6}\u{F8}-\u{2FF}\u{370}-\u{37D}\-\.0-9\u{B7}]")
        .unwrap()
});

static NCNAME: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[A-Z_a-z\u{C0}-\u{D6}\u{D8}-\u{F6}][A-Z_a-z\u{C0}-\u{D6}\u{D8}-\u{F6}\-\.0-9]*$")
        .unwrap()
});

/// Check if a string is a valid XML Name
pub fn is_valid_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // TODO: Implement full XML Name validation per spec
    // For now, use a simplified check
    name.chars()
        .next()
        .map(|c| c.is_alphabetic() || c == '_')
        .unwrap_or(false)
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
}

/// Check if a string is a valid NCName (non-colonized name)
pub fn is_valid_ncname(name: &str) -> bool {
    if name.is_empty() || name.contains(':') {
        return false;
    }

    // NCName is a Name without colons
    is_valid_name(name)
}

/// Check if a string is a valid QName (qualified name)
pub fn is_valid_qname(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // QName can be "prefix:localName" or just "localName"
    if let Some((prefix, local)) = name.split_once(':') {
        is_valid_ncname(prefix) && is_valid_ncname(local)
    } else {
        is_valid_ncname(name)
    }
}

/// Validate an XML Name and return an error if invalid
pub fn validate_name(name: &str) -> Result<()> {
    if is_valid_name(name) {
        Ok(())
    } else {
        Err(Error::Name(format!("Invalid XML Name: '{}'", name)))
    }
}

/// Validate an NCName and return an error if invalid
pub fn validate_ncname(name: &str) -> Result<()> {
    if is_valid_ncname(name) {
        Ok(())
    } else {
        Err(Error::Name(format!("Invalid NCName: '{}'", name)))
    }
}

/// Validate a QName and return an error if invalid
pub fn validate_qname(name: &str) -> Result<()> {
    if is_valid_qname(name) {
        Ok(())
    } else {
        Err(Error::Name(format!("Invalid QName: '{}'", name)))
    }
}

/// Split a QName into prefix and local name
pub fn split_qname(qname: &str) -> (Option<&str>, &str) {
    if let Some((prefix, local)) = qname.split_once(':') {
        (Some(prefix), local)
    } else {
        (None, qname)
    }
}

// TODO: Implement full XML spec compliance
// - Complete NameStartChar and NameChar patterns
// - Handle Unicode ranges properly
// - Normalize names according to spec

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_name() {
        assert!(is_valid_name("element"));
        assert!(is_valid_name("my-element"));
        assert!(is_valid_name("my_element"));
        assert!(is_valid_name("element123"));
        assert!(is_valid_name("_element"));

        assert!(!is_valid_name(""));
        assert!(!is_valid_name("123element"));
        assert!(!is_valid_name("-element"));
    }

    #[test]
    fn test_is_valid_ncname() {
        assert!(is_valid_ncname("element"));
        assert!(is_valid_ncname("my-element"));

        assert!(!is_valid_ncname(""));
        assert!(!is_valid_ncname("prefix:element"));
    }

    #[test]
    fn test_is_valid_qname() {
        assert!(is_valid_qname("element"));
        assert!(is_valid_qname("prefix:element"));
        assert!(is_valid_qname("xs:schema"));

        assert!(!is_valid_qname(""));
        assert!(!is_valid_qname(":element"));
        assert!(!is_valid_qname("element:"));
    }

    #[test]
    fn test_split_qname() {
        assert_eq!(split_qname("element"), (None, "element"));
        assert_eq!(split_qname("xs:element"), (Some("xs"), "element"));
    }

    #[test]
    fn test_validate_name() {
        assert!(validate_name("element").is_ok());
        assert!(validate_name("123").is_err());
    }
}
