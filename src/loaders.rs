//! Resource loading utilities
//!
//! This module handles loading of XML schemas and documents from various sources.

use crate::error::{Error, Result};
use crate::limits::Limits;
use crate::locations::Location;
use std::fs;
use std::io::Read;

/// Resource loader for schemas and documents
#[derive(Debug)]
pub struct Loader {
    /// Resource limits
    limits: Limits,
    /// Whether to allow remote resources
    allow_remote: bool,
}

impl Loader {
    /// Create a new loader with default settings
    pub fn new() -> Self {
        Self {
            limits: Limits::default(),
            allow_remote: true,
        }
    }

    /// Set the limits
    pub fn with_limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }

    /// Set whether to allow remote resources
    pub fn with_allow_remote(mut self, allow: bool) -> Self {
        self.allow_remote = allow;
        self
    }

    /// Load a resource as a string
    pub fn load(&self, location: &Location) -> Result<String> {
        match location {
            Location::Path(path) => {
                let content = fs::read_to_string(path).map_err(|e| {
                    Error::Resource(format!("Failed to read file '{}': {}", path.display(), e))
                })?;

                // Check size limits
                self.limits.check_xml_size(content.len())?;

                Ok(content)
            }
            Location::Url(url) => {
                if !self.allow_remote {
                    return Err(Error::Resource(
                        "Remote resources are not allowed".to_string(),
                    ));
                }

                // TODO: Implement HTTP/HTTPS loading
                Err(Error::Resource(format!(
                    "URL loading not yet implemented: {}",
                    url
                )))
            }
            Location::String(s) => Ok(s.clone()),
        }
    }

    /// Load a resource as bytes
    pub fn load_bytes(&self, location: &Location) -> Result<Vec<u8>> {
        match location {
            Location::Path(path) => {
                let content = fs::read(path).map_err(|e| {
                    Error::Resource(format!("Failed to read file '{}': {}", path.display(), e))
                })?;

                // Check size limits
                self.limits.check_xml_size(content.len())?;

                Ok(content)
            }
            Location::Url(_url) => {
                if !self.allow_remote {
                    return Err(Error::Resource(
                        "Remote resources are not allowed".to_string(),
                    ));
                }

                // TODO: Implement HTTP/HTTPS loading
                Err(Error::Resource("URL loading not yet implemented".to_string()))
            }
            Location::String(s) => Ok(s.as_bytes().to_vec()),
        }
    }
}

impl Default for Loader {
    fn default() -> Self {
        Self::new()
    }
}

// TODO: Implement
// - HTTP/HTTPS resource loading
// - Resource caching
// - Access control
// - Progress callbacks
// - Timeout handling

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_from_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "<root>test</root>").unwrap();

        let location = Location::Path(file.path().to_path_buf());
        let loader = Loader::new();
        let content = loader.load(&location).unwrap();

        assert!(content.contains("<root>test</root>"));
    }

    #[test]
    fn test_load_from_string() {
        let location = Location::String("<root>test</root>".to_string());
        let loader = Loader::new();
        let content = loader.load(&location).unwrap();

        assert_eq!(content, "<root>test</root>");
    }

    #[test]
    fn test_size_limit() {
        let mut file = NamedTempFile::new().unwrap();
        let large_content = "x".repeat(11 * 1024 * 1024); // 11 MB
        write!(file, "{}", large_content).unwrap();

        let location = Location::Path(file.path().to_path_buf());
        let loader = Loader::new().with_limits(Limits::strict());
        let result = loader.load(&location);

        // Strict limits (10 MB max) should reject 11MB file
        assert!(result.is_err());
    }
}
