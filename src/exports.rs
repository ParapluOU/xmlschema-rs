//! Schema Export Utilities
//!
//! This module provides utilities for exporting XML Schemas
//! and their dependencies to a directory structure.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// Configuration for schema export
#[derive(Debug, Clone)]
pub struct ExportConfig {
    /// Target directory for export
    pub target_dir: PathBuf,
    /// Whether to include remote schemas
    pub include_remote: bool,
    /// Whether to flatten the directory structure
    pub flatten: bool,
    /// Whether to replace schema locations
    pub replace_locations: bool,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            target_dir: PathBuf::from("."),
            include_remote: true,
            flatten: false,
            replace_locations: true,
        }
    }
}

impl ExportConfig {
    /// Create a new export configuration
    pub fn new(target_dir: impl Into<PathBuf>) -> Self {
        Self {
            target_dir: target_dir.into(),
            ..Default::default()
        }
    }

    /// Set whether to include remote schemas
    pub fn with_include_remote(mut self, include: bool) -> Self {
        self.include_remote = include;
        self
    }

    /// Set whether to flatten the directory structure
    pub fn with_flatten(mut self, flatten: bool) -> Self {
        self.flatten = flatten;
        self
    }

    /// Set whether to replace schema locations
    pub fn with_replace_locations(mut self, replace: bool) -> Self {
        self.replace_locations = replace;
        self
    }
}

/// Information about a schema source for export
#[derive(Debug, Clone)]
pub struct SchemaSource {
    /// Original location/path of the schema
    pub original_path: PathBuf,
    /// The schema content as text
    pub text: String,
    /// Whether this source has been processed
    pub processed: bool,
    /// Whether the content was modified during export
    pub modified: bool,
    /// Locations referenced by this schema (imports/includes)
    pub schema_locations: HashSet<String>,
}

impl SchemaSource {
    /// Create a new schema source
    pub fn new(path: impl Into<PathBuf>, text: impl Into<String>) -> Self {
        Self {
            original_path: path.into(),
            text: text.into(),
            processed: false,
            modified: false,
            schema_locations: HashSet::new(),
        }
    }

    /// Extract schema locations from the source text
    pub fn extract_locations(&mut self) {
        // Simple regex-like extraction of schemaLocation attributes
        let text = &self.text;
        let mut locations = HashSet::new();

        // Look for schemaLocation="..." patterns
        for line in text.lines() {
            if let Some(start) = line.find("schemaLocation=") {
                let rest = &line[start + 15..]; // After schemaLocation=
                if let Some(quote) = rest.chars().next() {
                    if quote == '"' || quote == '\'' {
                        if let Some(end) = rest[1..].find(quote) {
                            let location = rest[1..end + 1].trim().to_string();
                            if !location.is_empty() {
                                locations.insert(location);
                            }
                        }
                    }
                }
            }
        }

        self.schema_locations = locations;
        self.processed = true;
    }

    /// Replace a schema location in the text
    pub fn replace_location(&mut self, old_location: &str, new_location: &str) {
        if old_location == new_location {
            return;
        }

        // Replace the schemaLocation value
        let pattern = format!("schemaLocation=\"{}\"", old_location);
        let replacement = format!("schemaLocation=\"{}\"", new_location);
        self.text = self.text.replace(&pattern, &replacement);

        // Also try single quotes
        let pattern = format!("schemaLocation='{}'", old_location);
        let replacement = format!("schemaLocation='{}'", new_location);
        self.text = self.text.replace(&pattern, &replacement);

        self.modified = true;
    }
}

/// Schema exporter
///
/// Exports XML schemas and their dependencies to a directory structure.
#[derive(Debug)]
pub struct SchemaExporter {
    config: ExportConfig,
    sources: Vec<SchemaSource>,
}

impl SchemaExporter {
    /// Create a new schema exporter
    pub fn new(config: ExportConfig) -> Self {
        Self {
            config,
            sources: Vec::new(),
        }
    }

    /// Add a schema source to export
    pub fn add_source(&mut self, source: SchemaSource) {
        self.sources.push(source);
    }

    /// Get the number of sources
    pub fn source_count(&self) -> usize {
        self.sources.len()
    }

    /// Get the export configuration
    pub fn config(&self) -> &ExportConfig {
        &self.config
    }

    /// Export all sources to the target directory
    pub fn export(&mut self) -> Result<ExportResult> {
        let target_dir = &self.config.target_dir;

        // Ensure target directory exists
        if !target_dir.exists() {
            std::fs::create_dir_all(target_dir).map_err(|e| {
                Error::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to create export directory: {}", e),
                ))
            })?;
        }

        let mut exported_files = Vec::new();
        let mut modified_count = 0;

        for source in &mut self.sources {
            // Extract and process locations if needed
            if !source.processed {
                source.extract_locations();
                source.processed = true;
            }

            // Determine target path
            let target_path = if self.config.flatten {
                // Just use the filename
                target_dir.join(
                    source
                        .original_path
                        .file_name()
                        .unwrap_or_else(|| std::ffi::OsStr::new("schema.xsd")),
                )
            } else {
                // Preserve relative structure
                target_dir.join(&source.original_path)
            };

            // Create parent directories if needed
            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    Error::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to create parent directory: {}", e),
                    ))
                })?;
            }

            // Write the file
            std::fs::write(&target_path, &source.text).map_err(|e| {
                Error::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to write schema file: {}", e),
                ))
            })?;

            exported_files.push(target_path);
            if source.modified {
                modified_count += 1;
            }
        }

        Ok(ExportResult {
            exported_files,
            modified_count,
        })
    }
}

/// Result of a schema export operation
#[derive(Debug)]
pub struct ExportResult {
    /// List of exported file paths
    pub exported_files: Vec<PathBuf>,
    /// Number of files that were modified during export
    pub modified_count: usize,
}

impl ExportResult {
    /// Get the number of exported files
    pub fn file_count(&self) -> usize {
        self.exported_files.len()
    }

    /// Check if any files were modified
    pub fn has_modifications(&self) -> bool {
        self.modified_count > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_config_default() {
        let config = ExportConfig::default();
        assert_eq!(config.target_dir, PathBuf::from("."));
        assert!(config.include_remote);
        assert!(!config.flatten);
    }

    #[test]
    fn test_export_config_builder() {
        let config = ExportConfig::new("/tmp/export")
            .with_flatten(true)
            .with_include_remote(false);

        assert_eq!(config.target_dir, PathBuf::from("/tmp/export"));
        assert!(config.flatten);
        assert!(!config.include_remote);
    }

    #[test]
    fn test_schema_source_new() {
        let source = SchemaSource::new("test.xsd", "<schema/>");
        assert_eq!(source.original_path, PathBuf::from("test.xsd"));
        assert_eq!(source.text, "<schema/>");
        assert!(!source.processed);
        assert!(!source.modified);
    }

    #[test]
    fn test_schema_source_extract_locations() {
        let mut source = SchemaSource::new(
            "main.xsd",
            r#"<xs:import schemaLocation="types.xsd"/>
               <xs:include schemaLocation="common.xsd"/>"#,
        );

        source.extract_locations();

        assert!(source.processed);
        assert_eq!(source.schema_locations.len(), 2);
        assert!(source.schema_locations.contains("types.xsd"));
        assert!(source.schema_locations.contains("common.xsd"));
    }

    #[test]
    fn test_schema_source_replace_location() {
        let mut source = SchemaSource::new(
            "main.xsd",
            r#"<xs:import schemaLocation="http://example.com/types.xsd"/>"#,
        );

        source.replace_location("http://example.com/types.xsd", "types.xsd");

        assert!(source.modified);
        assert!(source.text.contains("schemaLocation=\"types.xsd\""));
    }

    #[test]
    fn test_schema_exporter_new() {
        let config = ExportConfig::new("/tmp/test");
        let exporter = SchemaExporter::new(config);

        assert_eq!(exporter.source_count(), 0);
    }

    #[test]
    fn test_schema_exporter_add_source() {
        let config = ExportConfig::new("/tmp/test");
        let mut exporter = SchemaExporter::new(config);

        exporter.add_source(SchemaSource::new("a.xsd", "<schema/>"));
        exporter.add_source(SchemaSource::new("b.xsd", "<schema/>"));

        assert_eq!(exporter.source_count(), 2);
    }

    #[test]
    fn test_export_result() {
        let result = ExportResult {
            exported_files: vec![PathBuf::from("a.xsd"), PathBuf::from("b.xsd")],
            modified_count: 1,
        };

        assert_eq!(result.file_count(), 2);
        assert!(result.has_modifications());
    }
}
