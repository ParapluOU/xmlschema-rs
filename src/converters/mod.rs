//! XML Schema Converters
//!
//! This module provides converters for transforming XML data to various formats
//! (JSON, dictionaries) using different conventions.
//!
//! Supported conventions:
//! - Default: XMLSchema standard conversion
//! - Parker: http://wiki.open311.org/JSON_and_XML_Conversion/#the-parker-convention
//! - BadgerFish: http://www.sklar.com/badgerfish/
//! - Unordered: Unordered element content
//! - JsonML: JSON Markup Language

mod base;
mod parker;
mod badgerfish;
mod unordered;

pub use base::{
    Converter, ConverterConfig, ElementData, XmlnsProcessing,
    XmlSchemaConverter,
};
pub use parker::ParkerConverter;
pub use badgerfish::BadgerFishConverter;
pub use unordered::UnorderedConverter;

use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Type alias for namespace mappings
pub type NamespaceMap = HashMap<String, String>;

/// Type alias for attribute mappings
pub type AttributeMap = HashMap<String, String>;

/// Type alias for xmlns declarations (prefix, uri)
pub type XmlnsDecl = Vec<(String, String)>;

/// Trait for converters that can decode ElementData to JSON
pub trait JsonConverter {
    /// Decode element data to a JSON value
    fn decode(&self, data: &ElementData, level: usize) -> JsonValue;

    /// Encode JSON value to element data
    fn encode(&self, value: &JsonValue, tag: &str, level: usize) -> ElementData;

    /// Returns true if the converter may lose information during conversion
    fn is_lossy(&self) -> bool;

    /// Returns true if namespace information is lost during conversion
    fn loses_xmlns(&self) -> bool;

    /// Returns true if the converter can perform lossless round-trips
    fn is_lossless(&self) -> bool {
        !self.is_lossy() && !self.loses_xmlns()
    }
}

/// Converter type enumeration for selecting conversion strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConverterType {
    /// Default XMLSchema converter
    Default,
    /// Parker convention (simplified, lossy)
    Parker,
    /// BadgerFish convention (preserves structure)
    BadgerFish,
    /// Unordered content handling
    Unordered,
    /// JsonML format
    JsonML,
    /// Columnar format (for data tables)
    Columnar,
}

impl Default for ConverterType {
    fn default() -> Self {
        Self::Default
    }
}

/// Create a converter by type
pub fn create_converter(conv_type: ConverterType) -> Box<dyn JsonConverter> {
    match conv_type {
        ConverterType::Default => Box::new(XmlSchemaConverter::new()),
        ConverterType::Parker => Box::new(ParkerConverter::new()),
        ConverterType::BadgerFish => Box::new(BadgerFishConverter::new()),
        ConverterType::Unordered => Box::new(UnorderedConverter::new()),
        // Others can use default for now
        ConverterType::JsonML | ConverterType::Columnar => Box::new(XmlSchemaConverter::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_data_creation() {
        let data = ElementData::new("test");
        assert_eq!(data.tag(), "test");
        assert!(data.text().is_none());
        assert!(data.content().is_empty());
        assert!(data.attributes().is_empty());
    }

    #[test]
    fn test_element_data_with_text() {
        let data = ElementData::new("element")
            .with_text("Hello, World!");
        assert_eq!(data.text(), Some("Hello, World!"));
    }

    #[test]
    fn test_converter_config_defaults() {
        let config = ConverterConfig::default();
        assert_eq!(config.text_key(), "$");
        assert_eq!(config.attr_prefix(), "@");
    }

    #[test]
    fn test_converter_type_default() {
        assert_eq!(ConverterType::default(), ConverterType::Default);
    }

    #[test]
    fn test_create_converter_default() {
        let converter = create_converter(ConverterType::Default);
        // Default converter may be lossy (no cdata_prefix), but doesn't lose xmlns
        assert!(!converter.loses_xmlns());
    }

    #[test]
    fn test_create_converter_parker() {
        let converter = create_converter(ConverterType::Parker);
        assert!(converter.is_lossy());
        assert!(converter.loses_xmlns());
    }

    #[test]
    fn test_create_converter_badgerfish() {
        let converter = create_converter(ConverterType::BadgerFish);
        assert!(!converter.is_lossy());
    }
}
