//! Parker Convention Converter
//!
//! Implements the Parker convention for XML to JSON conversion.
//!
//! References:
//! - http://wiki.open311.org/JSON_and_XML_Conversion/#the-parker-convention
//! - https://developer.mozilla.org/en-US/docs/Archive/JXON#The_Parker_Convention
//!
//! The Parker convention is a simplified, lossy conversion that:
//! - Ignores attributes
//! - Ignores namespaces
//! - Removes the document root element by default
//! - Uses element names as object keys

use serde_json::{Map, Value as JsonValue};

use super::base::{ContentItem, ConverterConfig, ElementData};
use super::JsonConverter;

/// Parker convention converter
///
/// A simplified converter that produces more compact JSON but loses:
/// - All attribute information
/// - Namespace information
/// - CDATA structure
#[derive(Debug, Clone)]
pub struct ParkerConverter {
    config: ConverterConfig,
}

impl Default for ParkerConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl ParkerConverter {
    /// Create a new Parker converter
    pub fn new() -> Self {
        // Parker convention has no attributes, text key is empty, no cdata
        let config = ConverterConfig::new()
            .with_attr_prefix(None)
            .with_text_key(Some(String::new()))
            .with_cdata_prefix(None);

        Self { config }
    }

    /// Create with preserve_root option
    pub fn with_preserve_root(preserve: bool) -> Self {
        let config = ConverterConfig::new()
            .with_attr_prefix(None)
            .with_text_key(Some(String::new()))
            .with_cdata_prefix(None)
            .with_preserve_root(preserve);

        Self { config }
    }

    /// Get the configuration
    pub fn config(&self) -> &ConverterConfig {
        &self.config
    }

    /// Map a qname (simplified - just returns the local name)
    fn map_qname(&self, name: &str) -> String {
        // Check for Clark notation first {uri}local
        if name.starts_with('{') {
            if let Some(pos) = name.find('}') {
                return name[pos + 1..].to_string();
            }
        }
        // Strip namespace prefix if present (ns:local)
        if let Some(pos) = name.find(':') {
            name[pos + 1..].to_string()
        } else {
            name.to_string()
        }
    }
}

impl JsonConverter for ParkerConverter {
    fn decode(&self, data: &ElementData, level: usize) -> JsonValue {
        let preserve_root = self.config.preserve_root();

        // If no content, just return the text value
        if data.content().is_empty() {
            if preserve_root {
                let mut obj = Map::new();
                obj.insert(
                    self.map_qname(data.tag()),
                    data.text()
                        .map(|s| JsonValue::String(s.to_string()))
                        .unwrap_or(JsonValue::Null),
                );
                return JsonValue::Object(obj);
            }
            return data
                .text()
                .map(|s| JsonValue::String(s.to_string()))
                .unwrap_or(JsonValue::Null);
        }

        // Build result object from content
        let mut result = Map::new();

        for item in data.content() {
            if let ContentItem::Element(name, value) = item {
                let key = self.map_qname(name);

                // If preserve_root and value is a single-key object, unwrap it
                let value = if preserve_root {
                    if let JsonValue::Object(obj) = value {
                        if obj.len() == 1 {
                            if let Some((_, inner)) = obj.iter().next() {
                                inner.clone()
                            } else {
                                JsonValue::Object(obj.clone())
                            }
                        } else {
                            JsonValue::Object(obj.clone())
                        }
                    } else {
                        value.clone()
                    }
                } else {
                    value.clone()
                };

                if let Some(existing) = result.get_mut(&key) {
                    // Already exists - convert to array
                    if let JsonValue::Array(arr) = existing {
                        if let JsonValue::Array(new_arr) = &value {
                            // Value is also an array - wrap both
                            arr.push(value.clone());
                        } else {
                            arr.push(value.clone());
                        }
                    } else {
                        let old = existing.take();
                        *existing = JsonValue::Array(vec![old, value.clone()]);
                    }
                } else {
                    // First occurrence
                    if let JsonValue::Array(_) = &value {
                        result.insert(key, JsonValue::Array(vec![value.clone()]));
                    } else {
                        result.insert(key, value.clone());
                    }
                }
            }
            // CDATA is ignored in Parker convention
        }

        // Flatten single-element arrays
        for (_, v) in result.iter_mut() {
            if let JsonValue::Array(arr) = v {
                if arr.len() == 1 {
                    if let JsonValue::Array(inner) = &arr[0] {
                        *v = JsonValue::Array(inner.clone());
                    }
                }
            }
        }

        if preserve_root {
            let mut wrapper = Map::new();
            wrapper.insert(self.map_qname(data.tag()), JsonValue::Object(result));
            JsonValue::Object(wrapper)
        } else if result.is_empty() {
            JsonValue::Null
        } else {
            JsonValue::Object(result)
        }
    }

    fn encode(&self, value: &JsonValue, tag: &str, _level: usize) -> ElementData {
        let mut data = ElementData::new(tag);

        match value {
            JsonValue::Object(obj) => {
                if obj.is_empty() {
                    return data;
                }

                // Handle preserve_root
                if self.config.preserve_root() {
                    if let Some((key, inner)) = obj.iter().next() {
                        if obj.len() == 1 && key == tag {
                            if let JsonValue::Object(inner_obj) = inner {
                                for (k, v) in inner_obj {
                                    match v {
                                        JsonValue::Array(arr) => {
                                            for item in arr {
                                                data.content
                                                    .push(ContentItem::Element(k.clone(), item.clone()));
                                            }
                                        }
                                        _ => {
                                            data.content
                                                .push(ContentItem::Element(k.clone(), v.clone()));
                                        }
                                    }
                                }
                                return data;
                            }
                        }
                    }
                }

                // Normal object processing
                for (name, val) in obj {
                    match val {
                        JsonValue::Array(arr) => {
                            for item in arr {
                                data.content
                                    .push(ContentItem::Element(name.clone(), item.clone()));
                            }
                        }
                        _ => {
                            data.content
                                .push(ContentItem::Element(name.clone(), val.clone()));
                        }
                    }
                }
            }
            JsonValue::String(s) if s.is_empty() => {
                // Empty string -> null content
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
        true // Parker convention always loses attribute information
    }

    fn loses_xmlns(&self) -> bool {
        true // Parker convention ignores namespaces
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parker_simple_text() {
        let converter = ParkerConverter::new();
        let data = ElementData::new("element").with_text("value");

        let json = converter.decode(&data, 0);
        assert_eq!(json, JsonValue::String("value".to_string()));
    }

    #[test]
    fn test_parker_ignores_attributes() {
        let converter = ParkerConverter::new();
        let data = ElementData::new("element")
            .with_text("value")
            .with_attribute("id", "123");

        let json = converter.decode(&data, 0);
        // Attributes are ignored, only text is returned
        assert_eq!(json, JsonValue::String("value".to_string()));
    }

    #[test]
    fn test_parker_with_children() {
        let converter = ParkerConverter::new();
        let data = ElementData::new("root")
            .with_child("name", json!("Alice"))
            .with_child("age", json!("30"));

        let json = converter.decode(&data, 0);

        if let JsonValue::Object(obj) = json {
            assert_eq!(obj.get("name"), Some(&JsonValue::String("Alice".to_string())));
            assert_eq!(obj.get("age"), Some(&JsonValue::String("30".to_string())));
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_parker_preserve_root() {
        let converter = ParkerConverter::with_preserve_root(true);
        let data = ElementData::new("person").with_text("Alice");

        let json = converter.decode(&data, 0);

        if let JsonValue::Object(obj) = json {
            assert!(obj.contains_key("person"));
            assert_eq!(
                obj.get("person"),
                Some(&JsonValue::String("Alice".to_string()))
            );
        } else {
            panic!("Expected object with person key");
        }
    }

    #[test]
    fn test_parker_multiple_same_children() {
        let converter = ParkerConverter::new();
        let data = ElementData::new("root")
            .with_child("item", json!("first"))
            .with_child("item", json!("second"))
            .with_child("item", json!("third"));

        let json = converter.decode(&data, 0);

        if let JsonValue::Object(obj) = json {
            if let Some(JsonValue::Array(arr)) = obj.get("item") {
                assert_eq!(arr.len(), 3);
            } else {
                panic!("Expected item to be an array");
            }
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_parker_encode_simple() {
        let converter = ParkerConverter::new();
        let json = json!("Hello");

        let data = converter.encode(&json, "element", 0);
        assert_eq!(data.tag(), "element");
        assert_eq!(data.text(), Some("Hello"));
    }

    #[test]
    fn test_parker_encode_object() {
        let converter = ParkerConverter::new();
        let json = json!({
            "name": "Alice",
            "age": "30"
        });

        let data = converter.encode(&json, "person", 0);
        assert_eq!(data.tag(), "person");
        assert_eq!(data.content().len(), 2);
    }

    #[test]
    fn test_parker_is_lossy() {
        let converter = ParkerConverter::new();
        assert!(converter.is_lossy());
        assert!(converter.loses_xmlns());
        assert!(!converter.is_lossless());
    }

    #[test]
    fn test_parker_strips_namespace_prefix() {
        let converter = ParkerConverter::new();
        assert_eq!(converter.map_qname("ns:element"), "element");
        assert_eq!(converter.map_qname("element"), "element");
    }

    #[test]
    fn test_parker_strips_clark_notation() {
        let converter = ParkerConverter::new();
        assert_eq!(
            converter.map_qname("{http://example.com}element"),
            "element"
        );
    }
}
