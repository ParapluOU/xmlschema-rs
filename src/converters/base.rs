//! Base converter types and traits
//!
//! This module provides the foundational types for XML to JSON conversion.

use serde_json::{Map, Value as JsonValue};
use std::collections::HashMap;
use super::{AttributeMap, JsonConverter, XmlnsDecl};

/// Processing mode for xmlns declarations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum XmlnsProcessing {
    /// Stack xmlns declarations through the tree
    #[default]
    Stacked,
    /// Collapse xmlns to root only
    Collapsed,
    /// Include xmlns only at root
    RootOnly,
    /// No xmlns processing
    None,
}

/// Configuration for converters
#[derive(Debug, Clone)]
pub struct ConverterConfig {
    /// Key used for text content in decoded output
    text_key: Option<String>,
    /// Prefix for attribute names in decoded output
    attr_prefix: Option<String>,
    /// Prefix for character data in mixed content
    cdata_prefix: Option<String>,
    /// Whether to preserve the root element
    preserve_root: bool,
    /// Whether to force dictionary output for simple content
    force_dict: bool,
    /// Whether to force list output for children
    force_list: bool,
    /// Indentation for XML output
    indent: usize,
    /// Xmlns processing mode
    xmlns_processing: XmlnsProcessing,
    /// Whether to process namespaces
    process_namespaces: bool,
    /// Whether to strip namespaces
    strip_namespaces: bool,
}

impl Default for ConverterConfig {
    fn default() -> Self {
        Self {
            text_key: Some("$".to_string()),
            attr_prefix: Some("@".to_string()),
            cdata_prefix: None,
            preserve_root: false,
            force_dict: false,
            force_list: false,
            indent: 4,
            xmlns_processing: XmlnsProcessing::default(),
            process_namespaces: true,
            strip_namespaces: false,
        }
    }
}

impl ConverterConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the text key
    pub fn text_key(&self) -> &str {
        self.text_key.as_deref().unwrap_or("$")
    }

    /// Get the attribute prefix
    pub fn attr_prefix(&self) -> &str {
        self.attr_prefix.as_deref().unwrap_or("@")
    }

    /// Get the cdata prefix
    pub fn cdata_prefix(&self) -> Option<&str> {
        self.cdata_prefix.as_deref()
    }

    /// Check if root should be preserved
    pub fn preserve_root(&self) -> bool {
        self.preserve_root
    }

    /// Check if dict should be forced
    pub fn force_dict(&self) -> bool {
        self.force_dict
    }

    /// Check if list should be forced
    pub fn force_list(&self) -> bool {
        self.force_list
    }

    /// Get indentation level
    pub fn indent(&self) -> usize {
        self.indent
    }

    /// Get xmlns processing mode
    pub fn xmlns_processing(&self) -> XmlnsProcessing {
        self.xmlns_processing
    }

    /// Set text key
    pub fn with_text_key(mut self, key: Option<String>) -> Self {
        self.text_key = key;
        self
    }

    /// Set attribute prefix
    pub fn with_attr_prefix(mut self, prefix: Option<String>) -> Self {
        self.attr_prefix = prefix;
        self
    }

    /// Set cdata prefix
    pub fn with_cdata_prefix(mut self, prefix: Option<String>) -> Self {
        self.cdata_prefix = prefix;
        self
    }

    /// Set preserve root
    pub fn with_preserve_root(mut self, preserve: bool) -> Self {
        self.preserve_root = preserve;
        self
    }

    /// Set force dict
    pub fn with_force_dict(mut self, force: bool) -> Self {
        self.force_dict = force;
        self
    }

    /// Set force list
    pub fn with_force_list(mut self, force: bool) -> Self {
        self.force_list = force;
        self
    }

    /// Set indentation
    pub fn with_indent(mut self, indent: usize) -> Self {
        self.indent = indent;
        self
    }

    /// Set xmlns processing mode
    pub fn with_xmlns_processing(mut self, mode: XmlnsProcessing) -> Self {
        self.xmlns_processing = mode;
        self
    }
}

/// Content item in element data
#[derive(Debug, Clone)]
pub enum ContentItem {
    /// Child element with name and value
    Element(String, JsonValue),
    /// Character data with index
    CData(usize, String),
}

/// Element data for conversion between XML and JSON
///
/// Represents the data extracted from an XML element for conversion.
#[derive(Debug, Clone, Default)]
pub struct ElementData {
    /// The element tag name
    pub tag: String,
    /// The text content
    pub text: Option<String>,
    /// Child content (elements and cdata)
    pub content: Vec<ContentItem>,
    /// Element attributes
    pub attributes: AttributeMap,
    /// Xmlns declarations
    pub xmlns: XmlnsDecl,
}

impl ElementData {
    /// Create new element data with a tag
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            tag: tag.into(),
            ..Default::default()
        }
    }

    /// Get the tag name
    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// Get the text content
    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    /// Get the content items
    pub fn content(&self) -> &[ContentItem] {
        &self.content
    }

    /// Get the attributes
    pub fn attributes(&self) -> &AttributeMap {
        &self.attributes
    }

    /// Get xmlns declarations
    pub fn xmlns(&self) -> &XmlnsDecl {
        &self.xmlns
    }

    /// Set text content
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Add an attribute
    pub fn with_attribute(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(name.into(), value.into());
        self
    }

    /// Add multiple attributes
    pub fn with_attributes(mut self, attrs: impl IntoIterator<Item = (String, String)>) -> Self {
        self.attributes.extend(attrs);
        self
    }

    /// Add a child element
    pub fn with_child(mut self, name: impl Into<String>, value: JsonValue) -> Self {
        self.content.push(ContentItem::Element(name.into(), value));
        self
    }

    /// Add character data
    pub fn with_cdata(mut self, index: usize, data: impl Into<String>) -> Self {
        self.content.push(ContentItem::CData(index, data.into()));
        self
    }

    /// Add xmlns declaration
    pub fn with_xmlns(mut self, prefix: impl Into<String>, uri: impl Into<String>) -> Self {
        self.xmlns.push((prefix.into(), uri.into()));
        self
    }

    /// Check if the element has content
    pub fn has_content(&self) -> bool {
        !self.content.is_empty()
    }

    /// Check if the element has attributes
    pub fn has_attributes(&self) -> bool {
        !self.attributes.is_empty()
    }

    /// Check if the element has xmlns declarations
    pub fn has_xmlns(&self) -> bool {
        !self.xmlns.is_empty()
    }
}

/// Legacy converter struct for backward compatibility
#[derive(Debug, Clone, Default)]
pub struct Converter {
    config: ConverterConfig,
}

impl Converter {
    /// Create a new converter
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with configuration
    pub fn with_config(config: ConverterConfig) -> Self {
        Self { config }
    }

    /// Get the configuration
    pub fn config(&self) -> &ConverterConfig {
        &self.config
    }
}

/// Default XML Schema converter
///
/// Converts XML elements to JSON using the standard XMLSchema convention:
/// - Attributes are prefixed with '@'
/// - Text content uses '$' key
/// - Child elements become object properties
#[derive(Debug, Clone)]
pub struct XmlSchemaConverter {
    config: ConverterConfig,
    namespaces: HashMap<String, String>,
}

impl Default for XmlSchemaConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl XmlSchemaConverter {
    /// Create a new XmlSchemaConverter
    pub fn new() -> Self {
        Self {
            config: ConverterConfig::default(),
            namespaces: HashMap::new(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: ConverterConfig) -> Self {
        Self {
            config,
            namespaces: HashMap::new(),
        }
    }

    /// Set namespace mappings
    pub fn with_namespaces(mut self, namespaces: HashMap<String, String>) -> Self {
        self.namespaces = namespaces;
        self
    }

    /// Get the configuration
    pub fn config(&self) -> &ConverterConfig {
        &self.config
    }

    /// Map a qname using namespace mappings
    pub fn map_qname(&self, name: &str) -> String {
        // For now, just return the name as-is
        // Full implementation would handle namespace prefix mapping
        name.to_string()
    }

    /// Unmap a qname back to qualified form
    pub fn unmap_qname(&self, name: &str) -> String {
        name.to_string()
    }
}

impl JsonConverter for XmlSchemaConverter {
    fn decode(&self, data: &ElementData, level: usize) -> JsonValue {
        let mut result = Map::new();
        let attr_prefix = self.config.attr_prefix();
        let text_key = self.config.text_key();

        // Add xmlns declarations if at root and processing namespaces
        if level == 0 && self.config.process_namespaces && !data.xmlns.is_empty() {
            for (prefix, uri) in &data.xmlns {
                let key = if prefix.is_empty() {
                    format!("{}xmlns", attr_prefix)
                } else {
                    format!("{}xmlns:{}", attr_prefix, prefix)
                };
                result.insert(key, JsonValue::String(uri.clone()));
            }
        }

        // Add attributes
        for (name, value) in &data.attributes {
            let key = format!("{}{}", attr_prefix, self.map_qname(name));
            result.insert(key, JsonValue::String(value.clone()));
        }

        // Handle content
        if data.content.is_empty() {
            // Simple content - just text
            if let Some(text) = &data.text {
                if result.is_empty() && !self.config.force_dict {
                    if level == 0 && self.config.preserve_root {
                        let mut wrapper = Map::new();
                        wrapper.insert(self.map_qname(data.tag()), JsonValue::String(text.clone()));
                        return JsonValue::Object(wrapper);
                    }
                    return JsonValue::String(text.clone());
                }
                result.insert(text_key.to_string(), JsonValue::String(text.clone()));
            }
        } else {
            // Complex content - has children
            if let Some(text) = &data.text {
                result.insert(text_key.to_string(), JsonValue::String(text.clone()));
            }

            for item in &data.content {
                match item {
                    ContentItem::Element(name, value) => {
                        let key = self.map_qname(name);
                        if let Some(existing) = result.get_mut(&key) {
                            // Convert to array if not already
                            if let JsonValue::Array(arr) = existing {
                                arr.push(value.clone());
                            } else {
                                let old = existing.take();
                                *existing = JsonValue::Array(vec![old, value.clone()]);
                            }
                        } else if self.config.force_list {
                            result.insert(key, JsonValue::Array(vec![value.clone()]));
                        } else {
                            result.insert(key, value.clone());
                        }
                    }
                    ContentItem::CData(index, text) => {
                        if let Some(prefix) = self.config.cdata_prefix() {
                            let key = format!("{}{}", prefix, index);
                            result.insert(key, JsonValue::String(text.clone()));
                        }
                    }
                }
            }
        }

        if level == 0 && self.config.preserve_root {
            let mut wrapper = Map::new();
            wrapper.insert(
                self.map_qname(data.tag()),
                if result.is_empty() {
                    JsonValue::Null
                } else {
                    JsonValue::Object(result)
                },
            );
            JsonValue::Object(wrapper)
        } else if result.is_empty() {
            JsonValue::Null
        } else {
            JsonValue::Object(result)
        }
    }

    fn encode(&self, value: &JsonValue, tag: &str, level: usize) -> ElementData {
        let mut data = ElementData::new(tag);
        let attr_prefix = self.config.attr_prefix();
        let text_key = self.config.text_key();

        match value {
            JsonValue::Object(obj) => {
                // Handle root wrapper if preserve_root
                let obj = if level == 0 && self.config.preserve_root && obj.len() == 1 {
                    if let Some((_, inner)) = obj.iter().next() {
                        if let JsonValue::Object(inner_obj) = inner {
                            inner_obj
                        } else {
                            obj
                        }
                    } else {
                        obj
                    }
                } else {
                    obj
                };

                for (key, val) in obj {
                    if key == text_key {
                        // Text content
                        if let JsonValue::String(s) = val {
                            data.text = Some(s.clone());
                        }
                    } else if key.starts_with(attr_prefix) && key != attr_prefix {
                        // Attribute
                        let attr_name = &key[attr_prefix.len()..];
                        if let JsonValue::String(s) = val {
                            data.attributes.insert(attr_name.to_string(), s.clone());
                        } else {
                            data.attributes
                                .insert(attr_name.to_string(), val.to_string());
                        }
                    } else if let Some(prefix) = self.config.cdata_prefix() {
                        if key.starts_with(prefix) {
                            // Character data
                            if let Ok(index) = key[prefix.len()..].parse::<usize>() {
                                if let JsonValue::String(s) = val {
                                    data.content.push(ContentItem::CData(index, s.clone()));
                                }
                            }
                        } else {
                            // Child element
                            match val {
                                JsonValue::Array(arr) => {
                                    for item in arr {
                                        data.content
                                            .push(ContentItem::Element(key.clone(), item.clone()));
                                    }
                                }
                                _ => {
                                    data.content
                                        .push(ContentItem::Element(key.clone(), val.clone()));
                                }
                            }
                        }
                    } else {
                        // Child element
                        match val {
                            JsonValue::Array(arr) => {
                                for item in arr {
                                    data.content
                                        .push(ContentItem::Element(key.clone(), item.clone()));
                                }
                            }
                            _ => {
                                data.content
                                    .push(ContentItem::Element(key.clone(), val.clone()));
                            }
                        }
                    }
                }
            }
            JsonValue::String(s) => {
                data.text = Some(s.clone());
            }
            JsonValue::Null => {}
            _ => {
                data.text = Some(value.to_string());
            }
        }

        data
    }

    fn is_lossy(&self) -> bool {
        self.config.cdata_prefix.is_none()
            || self.config.text_key.is_none()
            || self.config.attr_prefix.is_none()
    }

    fn loses_xmlns(&self) -> bool {
        !self.config.process_namespaces
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_converter_config_builder() {
        let config = ConverterConfig::new()
            .with_text_key(Some("#text".to_string()))
            .with_attr_prefix(Some("-".to_string()))
            .with_preserve_root(true)
            .with_indent(2);

        assert_eq!(config.text_key(), "#text");
        assert_eq!(config.attr_prefix(), "-");
        assert!(config.preserve_root());
        assert_eq!(config.indent(), 2);
    }

    #[test]
    fn test_element_data_builder() {
        let data = ElementData::new("root")
            .with_text("Hello")
            .with_attribute("id", "123")
            .with_xmlns("", "http://example.com");

        assert_eq!(data.tag(), "root");
        assert_eq!(data.text(), Some("Hello"));
        assert_eq!(data.attributes().get("id"), Some(&"123".to_string()));
        assert!(data.has_xmlns());
    }

    #[test]
    fn test_xml_schema_converter_simple() {
        let converter = XmlSchemaConverter::new();
        let data = ElementData::new("element").with_text("value");

        let json = converter.decode(&data, 0);
        assert_eq!(json, JsonValue::String("value".to_string()));
    }

    #[test]
    fn test_xml_schema_converter_with_attributes() {
        let converter = XmlSchemaConverter::new();
        let data = ElementData::new("element")
            .with_text("value")
            .with_attribute("id", "1");

        let json = converter.decode(&data, 0);

        if let JsonValue::Object(obj) = json {
            assert_eq!(obj.get("@id"), Some(&JsonValue::String("1".to_string())));
            assert_eq!(obj.get("$"), Some(&JsonValue::String("value".to_string())));
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_xml_schema_converter_with_children() {
        let converter = XmlSchemaConverter::new();
        let data = ElementData::new("root")
            .with_child("child1", json!("value1"))
            .with_child("child2", json!("value2"));

        let json = converter.decode(&data, 0);

        if let JsonValue::Object(obj) = json {
            assert_eq!(
                obj.get("child1"),
                Some(&JsonValue::String("value1".to_string()))
            );
            assert_eq!(
                obj.get("child2"),
                Some(&JsonValue::String("value2".to_string()))
            );
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_xml_schema_converter_preserve_root() {
        let config = ConverterConfig::new().with_preserve_root(true);
        let converter = XmlSchemaConverter::with_config(config);
        let data = ElementData::new("root").with_text("value");

        let json = converter.decode(&data, 0);

        if let JsonValue::Object(obj) = json {
            assert!(obj.contains_key("root"));
        } else {
            panic!("Expected object with root key");
        }
    }

    #[test]
    fn test_xml_schema_converter_encode_simple() {
        let converter = XmlSchemaConverter::new();
        let json = json!("Hello World");

        let data = converter.encode(&json, "element", 0);
        assert_eq!(data.tag(), "element");
        assert_eq!(data.text(), Some("Hello World"));
    }

    #[test]
    fn test_xml_schema_converter_encode_object() {
        let converter = XmlSchemaConverter::new();
        let json = json!({
            "@id": "123",
            "$": "content",
            "child": "child_value"
        });

        let data = converter.encode(&json, "root", 0);
        assert_eq!(data.tag(), "root");
        assert_eq!(data.text(), Some("content"));
        assert_eq!(data.attributes().get("id"), Some(&"123".to_string()));
        assert_eq!(data.content().len(), 1);
    }

    #[test]
    fn test_xml_schema_converter_is_lossy() {
        let converter = XmlSchemaConverter::new();
        // Default config has no cdata_prefix, so it's lossy
        assert!(converter.is_lossy());

        let config = ConverterConfig::new()
            .with_cdata_prefix(Some("#".to_string()));
        let converter = XmlSchemaConverter::with_config(config);
        assert!(!converter.is_lossy());
    }

    #[test]
    fn test_xmlns_processing_default() {
        assert_eq!(XmlnsProcessing::default(), XmlnsProcessing::Stacked);
    }

    #[test]
    fn test_content_item_variants() {
        let element = ContentItem::Element("child".to_string(), json!("value"));
        let cdata = ContentItem::CData(0, "text".to_string());

        if let ContentItem::Element(name, _) = element {
            assert_eq!(name, "child");
        }

        if let ContentItem::CData(index, text) = cdata {
            assert_eq!(index, 0);
            assert_eq!(text, "text");
        }
    }
}
