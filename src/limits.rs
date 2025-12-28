//! Limits and constraints for XML Schema processing
//!
//! This module defines various limits to prevent resource exhaustion
//! and protect against XML attacks (e.g., billion laughs, XML bombs).

use crate::error::{Error, Result};

/// Global limits configuration
#[derive(Debug, Clone)]
pub struct Limits {
    /// Maximum number of XML nodes to process
    pub max_xml_depth: usize,

    /// Maximum XML file size in bytes
    pub max_xml_size: usize,

    /// Maximum number of entity expansions
    pub max_entity_expansions: usize,

    /// Maximum entity expansion size in bytes
    pub max_entity_expansion_size: usize,

    /// Maximum number of attributes per element
    pub max_attributes: usize,

    /// Maximum number of namespaces
    pub max_namespaces: usize,

    /// Maximum schema depth (includes/imports)
    pub max_schema_depth: usize,

    /// Maximum number of schema components
    pub max_schema_components: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_xml_depth: 1000,
            max_xml_size: 100 * 1024 * 1024, // 100 MB
            max_entity_expansions: 10000,
            max_entity_expansion_size: 10 * 1024 * 1024, // 10 MB
            max_attributes: 1000,
            max_namespaces: 1000,
            max_schema_depth: 100,
            max_schema_components: 100000,
        }
    }
}

impl Limits {
    /// Create a new Limits with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create strict limits (more restrictive)
    pub fn strict() -> Self {
        Self {
            max_xml_depth: 100,
            max_xml_size: 10 * 1024 * 1024, // 10 MB
            max_entity_expansions: 1000,
            max_entity_expansion_size: 1024 * 1024, // 1 MB
            max_attributes: 100,
            max_namespaces: 100,
            max_schema_depth: 20,
            max_schema_components: 10000,
        }
    }

    /// Create permissive limits (less restrictive, use with caution)
    pub fn permissive() -> Self {
        Self {
            max_xml_depth: 10000,
            max_xml_size: 1024 * 1024 * 1024, // 1 GB
            max_entity_expansions: 100000,
            max_entity_expansion_size: 100 * 1024 * 1024, // 100 MB
            max_attributes: 10000,
            max_namespaces: 10000,
            max_schema_depth: 1000,
            max_schema_components: 1000000,
        }
    }

    /// Check if XML depth is within limits
    pub fn check_xml_depth(&self, depth: usize) -> Result<()> {
        if depth > self.max_xml_depth {
            Err(Error::LimitExceeded(format!(
                "XML depth {} exceeds maximum {}",
                depth, self.max_xml_depth
            )))
        } else {
            Ok(())
        }
    }

    /// Check if XML size is within limits
    pub fn check_xml_size(&self, size: usize) -> Result<()> {
        if size > self.max_xml_size {
            Err(Error::LimitExceeded(format!(
                "XML size {} bytes exceeds maximum {} bytes",
                size, self.max_xml_size
            )))
        } else {
            Ok(())
        }
    }

    /// Check if entity expansions are within limits
    pub fn check_entity_expansions(&self, count: usize) -> Result<()> {
        if count > self.max_entity_expansions {
            Err(Error::LimitExceeded(format!(
                "Entity expansions {} exceeds maximum {}",
                count, self.max_entity_expansions
            )))
        } else {
            Ok(())
        }
    }

    /// Check if entity expansion size is within limits
    pub fn check_entity_expansion_size(&self, size: usize) -> Result<()> {
        if size > self.max_entity_expansion_size {
            Err(Error::LimitExceeded(format!(
                "Entity expansion size {} bytes exceeds maximum {} bytes",
                size, self.max_entity_expansion_size
            )))
        } else {
            Ok(())
        }
    }

    /// Check if number of attributes is within limits
    pub fn check_attributes(&self, count: usize) -> Result<()> {
        if count > self.max_attributes {
            Err(Error::LimitExceeded(format!(
                "Attribute count {} exceeds maximum {}",
                count, self.max_attributes
            )))
        } else {
            Ok(())
        }
    }

    /// Check if number of namespaces is within limits
    pub fn check_namespaces(&self, count: usize) -> Result<()> {
        if count > self.max_namespaces {
            Err(Error::LimitExceeded(format!(
                "Namespace count {} exceeds maximum {}",
                count, self.max_namespaces
            )))
        } else {
            Ok(())
        }
    }

    /// Check if schema depth is within limits
    pub fn check_schema_depth(&self, depth: usize) -> Result<()> {
        if depth > self.max_schema_depth {
            Err(Error::LimitExceeded(format!(
                "Schema depth {} exceeds maximum {}",
                depth, self.max_schema_depth
            )))
        } else {
            Ok(())
        }
    }

    /// Check if number of schema components is within limits
    pub fn check_schema_components(&self, count: usize) -> Result<()> {
        if count > self.max_schema_components {
            Err(Error::LimitExceeded(format!(
                "Schema component count {} exceeds maximum {}",
                count, self.max_schema_components
            )))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_limits() {
        let limits = Limits::default();
        assert_eq!(limits.max_xml_depth, 1000);
        assert!(limits.check_xml_depth(500).is_ok());
        assert!(limits.check_xml_depth(1500).is_err());
    }

    #[test]
    fn test_strict_limits() {
        let limits = Limits::strict();
        assert!(limits.max_xml_depth < Limits::default().max_xml_depth);
        assert!(limits.check_xml_depth(150).is_err());
    }

    #[test]
    fn test_permissive_limits() {
        let limits = Limits::permissive();
        assert!(limits.max_xml_depth > Limits::default().max_xml_depth);
        assert!(limits.check_xml_depth(5000).is_ok());
    }

    #[test]
    fn test_check_xml_size() {
        let limits = Limits::default();
        assert!(limits.check_xml_size(1024).is_ok());
        assert!(limits.check_xml_size(200 * 1024 * 1024).is_err());
    }

    #[test]
    fn test_check_entity_expansions() {
        let limits = Limits::default();
        assert!(limits.check_entity_expansions(100).is_ok());
        assert!(limits.check_entity_expansions(20000).is_err());
    }
}
