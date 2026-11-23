//! XML Catalog support for schema location resolution
//!
//! This module implements OASIS XML Catalog support for resolving URN-based
//! schema locations (like those used in DITA 1.3) to actual file paths.
//!
//! XML Catalogs are defined by OASIS:
//! https://www.oasis-open.org/committees/entity/spec-2001-08-06.html
//!
//! # Supported Elements
//!
//! - `<catalog>` - Root element
//! - `<group>` - Grouping element (inherits base from parent)
//! - `<system>` - Maps system identifiers to URIs
//! - `<uri>` - Maps URN names to URIs
//! - `<nextCatalog>` - Includes another catalog file
//!
//! # Example
//!
//! ```xml
//! <catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
//!   <system systemId="urn:oasis:names:tc:dita:xsd:topic.xsd:1.3"
//!           uri="xsd/topic.xsd"/>
//!   <nextCatalog catalog="base/catalog.xml"/>
//! </catalog>
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;

use crate::documents::Document;
use crate::error::{Error, ParseError, Result};

/// The XML Catalog namespace
const CATALOG_NS: &str = "urn:oasis:names:tc:entity:xmlns:xml:catalog";

/// XML Catalog for resolving schema locations
#[derive(Debug, Clone, Default)]
pub struct XmlCatalog {
    /// System ID to URI mappings (systemId -> uri)
    system_mappings: HashMap<String, String>,
    /// URI name to URI mappings (name -> uri)
    uri_mappings: HashMap<String, String>,
    /// Base directory for resolving relative URIs
    base_dir: Option<PathBuf>,
}

impl XmlCatalog {
    /// Create an empty catalog
    pub fn new() -> Self {
        Self::default()
    }

    /// Load a catalog from a file
    ///
    /// This will recursively load any catalogs referenced via `<nextCatalog>`.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let base_dir = path.parent().map(|p| p.to_path_buf());

        let content = fs::read_to_string(path).map_err(|e| {
            Error::Resource(format!("Failed to read catalog '{}': {}", path.display(), e))
        })?;

        let mut catalog = Self::new();
        catalog.base_dir = base_dir.clone();
        catalog.parse_catalog(&content, base_dir.as_deref())?;

        Ok(catalog)
    }

    /// Parse catalog XML content
    fn parse_catalog(&mut self, xml: &str, base_dir: Option<&Path>) -> Result<()> {
        let doc = Document::from_string(xml)?;
        let root = doc.root().ok_or_else(|| {
            Error::Parse(ParseError::new("Empty catalog document"))
        })?;

        // Verify this is a catalog element
        if root.local_name() != "catalog" {
            return Err(Error::Parse(ParseError::new(format!(
                "Expected catalog root element, got {}",
                root.local_name()
            ))));
        }

        // Process children, from the catalog element's own base: `xml:base` on
        // the root applies to everything in it.
        let root_base = Self::base_of(root, base_dir);
        self.process_catalog_children(&root.children, root_base.as_deref())?;

        Ok(())
    }

    /// The base an element's relative URIs resolve against: `xml:base` where it
    /// carries one, joined onto the base it inherited, and that base otherwise.
    ///
    /// Catalogs lean on this. OASIS ships DITA with one catalog at the root of
    /// the bundle and the directory of each entry in its `xml:base`:
    ///
    /// ```xml
    /// <system systemId="urn:oasis:names:tc:dita:xsd:topicMod.xsd:1.2"
    ///         uri="topicMod.xsd" xml:base="base/xsd/"/>
    /// ```
    ///
    /// Ignoring it resolves every one of those to a file beside the catalog
    /// that is not there — and a `schemaLocation` that does not resolve is not
    /// an error to a schema parser, it simply contributes nothing. The symptom
    /// is a schema that parses and declares no element at all.
    fn base_of(element: &crate::documents::Element, inherited: Option<&Path>) -> Option<PathBuf> {
        // `xml:base`'s local name is `base`, which is how it is read here.
        match element.get_attribute("base") {
            Some(given) => Some(match inherited {
                Some(base) => base.join(given),
                None => PathBuf::from(given),
            }),
            None => inherited.map(Path::to_path_buf),
        }
    }

    /// Process children of a catalog or group element
    fn process_catalog_children(
        &mut self,
        children: &[crate::documents::Element],
        base_dir: Option<&Path>,
    ) -> Result<()> {
        for child in children {
            // Each entry may move its own base, and a group moves it for
            // everything inside.
            let here = Self::base_of(child, base_dir);
            let base_dir = here.as_deref();
            match child.local_name() {
                "system" => {
                    // <system systemId="..." uri="..."/>
                    if let (Some(system_id), Some(uri)) = (
                        child.get_attribute("systemId"),
                        child.get_attribute("uri"),
                    ) {
                        // Resolve relative URI against base_dir
                        let resolved_uri = if let Some(base) = base_dir {
                            base.join(uri).to_string_lossy().to_string()
                        } else {
                            uri.to_string()
                        };
                        self.system_mappings.insert(system_id.to_string(), resolved_uri);
                    }
                }
                "uri" => {
                    // <uri name="..." uri="..."/>
                    if let (Some(name), Some(uri)) = (
                        child.get_attribute("name"),
                        child.get_attribute("uri"),
                    ) {
                        // Resolve relative URI against base_dir
                        let resolved_uri = if let Some(base) = base_dir {
                            base.join(uri).to_string_lossy().to_string()
                        } else {
                            uri.to_string()
                        };
                        self.uri_mappings.insert(name.to_string(), resolved_uri);
                    }
                }
                "nextCatalog" => {
                    // <nextCatalog catalog="..."/>
                    if let Some(catalog_path) = child.get_attribute("catalog") {
                        // Resolve relative catalog path
                        let resolved_path = if let Some(base) = base_dir {
                            base.join(catalog_path)
                        } else {
                            PathBuf::from(catalog_path)
                        };

                        // Load the referenced catalog
                        if resolved_path.exists() {
                            if let Ok(content) = fs::read_to_string(&resolved_path) {
                                let next_base = resolved_path.parent().map(|p| p.to_path_buf());
                                // Silently ignore parse errors in nested catalogs
                                let _ = self.parse_catalog(&content, next_base.as_deref());
                            }
                        }
                    }
                }
                "group" => {
                    // A group carries its children's base, which `here` already
                    // took from its `xml:base`.
                    self.process_catalog_children(&child.children, base_dir)?;
                }
                _ => {
                    // Skip unknown elements (annotations, etc.)
                }
            }
        }

        Ok(())
    }

    /// Resolve a schema location using the catalog
    ///
    /// Tries to resolve the location in this order:
    /// 1. Check system ID mappings
    /// 2. Check URI name mappings
    /// 3. Return None if not found
    pub fn resolve(&self, location: &str) -> Option<&str> {
        // Try system ID first
        if let Some(uri) = self.system_mappings.get(location) {
            return Some(uri);
        }

        // Try URI name
        if let Some(uri) = self.uri_mappings.get(location) {
            return Some(uri);
        }

        None
    }

    /// Check if this catalog is empty (has no mappings)
    pub fn is_empty(&self) -> bool {
        self.system_mappings.is_empty() && self.uri_mappings.is_empty()
    }

    /// Get the number of mappings
    pub fn len(&self) -> usize {
        self.system_mappings.len() + self.uri_mappings.len()
    }

    /// Merge another catalog into this one
    pub fn merge(&mut self, other: &XmlCatalog) {
        for (k, v) in &other.system_mappings {
            self.system_mappings.entry(k.clone()).or_insert_with(|| v.clone());
        }
        for (k, v) in &other.uri_mappings {
            self.uri_mappings.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    /// `xml:base` on an entry moves where its `uri` points, and a group's moves
    /// it for everything inside.
    ///
    /// This is how OASIS ships DITA: one catalog at the root of the bundle,
    /// every entry naming its own directory in `xml:base`. Ignoring it pointed
    /// all 694 of DITA 1.2's mappings at files beside the catalog that are not
    /// there — and a `schemaLocation` that does not resolve is not an error to
    /// a schema parser, it simply contributes nothing, so the symptom was a
    /// schema that parsed and declared no element at all.
    #[test]
    fn an_entry_resolves_against_its_xml_base() {
        let catalog_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
    <system systemId="urn:example:topicMod.xsd:1.2" uri="topicMod.xsd" xml:base="base/xsd/"/>
    <uri name="urn:example:types.xsd" uri="types.xsd" xml:base="common/"/>
    <system systemId="urn:example:plain.xsd" uri="plain.xsd"/>
    <group xml:base="technicalContent/xsd/">
        <system systemId="urn:example:uiDomain.xsd:1.2" uri="uiDomain.xsd"/>
        <group xml:base="deeper/">
            <system systemId="urn:example:nested.xsd" uri="nested.xsd"/>
        </group>
    </group>
</catalog>"#;
        let directory = TempDir::new().expect("a directory");
        let at = directory.path().join("catalog.xml");
        let mut file = std::fs::File::create(&at).expect("a file");
        file.write_all(catalog_xml.as_bytes()).expect("written");

        let catalog = XmlCatalog::from_file(&at).expect("the catalog loads");
        let expected = |path: &str| directory.path().join(path).to_string_lossy().to_string();
        assert_eq!(catalog.resolve("urn:example:topicMod.xsd:1.2"), Some(expected("base/xsd/topicMod.xsd").as_str()));
        assert_eq!(catalog.resolve("urn:example:types.xsd"), Some(expected("common/types.xsd").as_str()));
        // An entry without one still resolves beside the catalog.
        assert_eq!(catalog.resolve("urn:example:plain.xsd"), Some(expected("plain.xsd").as_str()));
        // A group's base applies to its children, and nests.
        assert_eq!(catalog.resolve("urn:example:uiDomain.xsd:1.2"), Some(expected("technicalContent/xsd/uiDomain.xsd").as_str()));
        assert_eq!(catalog.resolve("urn:example:nested.xsd"), Some(expected("technicalContent/xsd/deeper/nested.xsd").as_str()));
    }

    #[test]
    fn test_parse_simple_catalog() {
        let catalog_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
    <system systemId="urn:example:schema.xsd" uri="schemas/schema.xsd"/>
    <uri name="urn:example:types.xsd" uri="schemas/types.xsd"/>
</catalog>"#;

        let temp_dir = TempDir::new().unwrap();
        let catalog_path = temp_dir.path().join("catalog.xml");
        std::fs::write(&catalog_path, catalog_xml).unwrap();

        // Create the schemas directory so paths resolve
        std::fs::create_dir_all(temp_dir.path().join("schemas")).unwrap();

        let catalog = XmlCatalog::from_file(&catalog_path).unwrap();

        assert_eq!(catalog.len(), 2);

        // Check system mapping
        let resolved = catalog.resolve("urn:example:schema.xsd").unwrap();
        assert!(resolved.ends_with("schemas/schema.xsd"));

        // Check uri mapping
        let resolved = catalog.resolve("urn:example:types.xsd").unwrap();
        assert!(resolved.ends_with("schemas/types.xsd"));

        // Check non-existent
        assert!(catalog.resolve("urn:example:not-found.xsd").is_none());
    }

    #[test]
    fn test_nested_catalogs() {
        let temp_dir = TempDir::new().unwrap();

        // Create main catalog
        let main_catalog = r#"<?xml version="1.0" encoding="UTF-8"?>
<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
    <system systemId="urn:main:schema.xsd" uri="main.xsd"/>
    <nextCatalog catalog="sub/catalog.xml"/>
</catalog>"#;

        // Create sub catalog
        let sub_dir = temp_dir.path().join("sub");
        std::fs::create_dir_all(&sub_dir).unwrap();

        let sub_catalog = r#"<?xml version="1.0" encoding="UTF-8"?>
<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
    <system systemId="urn:sub:schema.xsd" uri="sub.xsd"/>
</catalog>"#;

        std::fs::write(temp_dir.path().join("catalog.xml"), main_catalog).unwrap();
        std::fs::write(sub_dir.join("catalog.xml"), sub_catalog).unwrap();

        let catalog = XmlCatalog::from_file(temp_dir.path().join("catalog.xml")).unwrap();

        // Should have both mappings
        assert_eq!(catalog.len(), 2);
        assert!(catalog.resolve("urn:main:schema.xsd").is_some());
        assert!(catalog.resolve("urn:sub:schema.xsd").is_some());
    }

    #[test]
    fn test_group_element() {
        let catalog_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
    <group>
        <system systemId="urn:grouped:schema.xsd" uri="grouped.xsd"/>
    </group>
</catalog>"#;

        let temp_dir = TempDir::new().unwrap();
        let catalog_path = temp_dir.path().join("catalog.xml");
        std::fs::write(&catalog_path, catalog_xml).unwrap();

        let catalog = XmlCatalog::from_file(&catalog_path).unwrap();

        assert_eq!(catalog.len(), 1);
        assert!(catalog.resolve("urn:grouped:schema.xsd").is_some());
    }
}
