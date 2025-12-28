//! Resource location resolution
//!
//! This module handles resolution of resource locations (URLs, file paths, etc.)
//! for loading schemas and XML documents.

use crate::error::Result;
use std::path::PathBuf;
use url::Url;

/// Resource location - can be a URL, file path, or string identifier
#[derive(Debug, Clone)]
pub enum Location {
    /// File system path
    Path(PathBuf),
    /// URL (http, https, ftp, etc.)
    Url(Url),
    /// String identifier (for in-memory resources)
    String(String),
}

impl Location {
    /// Create a location from a string (auto-detect type)
    pub fn from_str(s: &str) -> Result<Self> {
        // Try to parse as URL first
        if let Ok(url) = Url::parse(s) {
            if url.scheme() != "file" {
                return Ok(Location::Url(url));
            }
        }

        // Try as file path
        let path = PathBuf::from(s);
        if path.exists() || s.starts_with('/') || s.starts_with('.') {
            return Ok(Location::Path(path));
        }

        // Otherwise treat as string identifier
        Ok(Location::String(s.to_string()))
    }

    /// Get the location as a string
    pub fn as_str(&self) -> String {
        match self {
            Location::Path(p) => p.to_string_lossy().to_string(),
            Location::Url(u) => u.to_string(),
            Location::String(s) => s.clone(),
        }
    }

    /// Check if this is a remote location (URL)
    pub fn is_remote(&self) -> bool {
        matches!(self, Location::Url(_))
    }

    /// Check if this is a local file
    pub fn is_file(&self) -> bool {
        matches!(self, Location::Path(_))
    }
}

// TODO: Implement
// - Base URL resolution
// - Relative path resolution
// - URL normalization
// - Access control checks
// - Location caching

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_location_from_url() {
        let loc = Location::from_str("http://example.com/schema.xsd").unwrap();
        assert!(matches!(loc, Location::Url(_)));
        assert!(loc.is_remote());
    }

    #[test]
    fn test_location_from_path() {
        let loc = Location::from_str("/tmp/schema.xsd").unwrap();
        assert!(matches!(loc, Location::Path(_)));
        assert!(loc.is_file());
    }

    #[test]
    fn test_location_as_str() {
        let loc = Location::String("test".to_string());
        assert_eq!(loc.as_str(), "test");
    }
}
