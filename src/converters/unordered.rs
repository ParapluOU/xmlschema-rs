//! Unordered Converter
//!
//! A converter that handles unordered XML content, allowing elements
//! to appear in any order during encoding.

use serde_json::{Map, Value as JsonValue};
use std::collections::HashMap;

use super::base::{ContentItem, ConverterConfig, ElementData};
use super::JsonConverter;

/// Unordered content converter
///
/// Similar to the default converter but allows unordered content
/// during encoding. This is useful when the input data may not
/// match the schema-defined order.
#[derive(Debug, Clone)]
pub struct UnorderedConverter {
    config: ConverterConfig,
}

impl Default for UnorderedConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl UnorderedConverter {
    /// Create a new unordered converter
    pub fn new() -> Self {
        Self {
            config: ConverterConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: ConverterConfig) -> Self {
        Self { config }
    }

    /// Get the configuration
    pub fn config(&self) -> &ConverterConfig {
        &self.config
    }

    /// Map a qname
    fn map_qname(&self, name: &str) -> String {
        name.to_string()
    }
}

impl JsonConverter for UnorderedConverter {
    fn decode(&self, data: &ElementData, level: usize) -> JsonValue {
        let mut result = Map::new();
        let attr_prefix = self.config.attr_prefix();
        let text_key = self.config.text_key();

        // Add xmlns declarations
        if level == 0 && !data.xmlns().is_empty() {
            for (prefix, uri) in data.xmlns() {
                let key = if prefix.is_empty() {
                    format!("{}xmlns", attr_prefix)
                } else {
                    format!("{}xmlns:{}", attr_prefix, prefix)
                };
                result.insert(key, JsonValue::String(uri.clone()));
            }
        }

        // Add attributes
        for (name, value) in data.attributes() {
            let key = format!("{}{}", attr_prefix, self.map_qname(name));
            result.insert(key, JsonValue::String(value.clone()));
        }

        // Handle content
        if data.content().is_empty() {
            if let Some(text) = data.text() {
                if result.is_empty() && !self.config.force_dict() {
                    if level == 0 && self.config.preserve_root() {
                        let mut wrapper = Map::new();
                        wrapper.insert(
                            self.map_qname(data.tag()),
                            JsonValue::String(text.to_string()),
                        );
                        return JsonValue::Object(wrapper);
                    }
                    return JsonValue::String(text.to_string());
                }
                result.insert(text_key.to_string(), JsonValue::String(text.to_string()));
            }
        } else {
            if let Some(text) = data.text() {
                result.insert(text_key.to_string(), JsonValue::String(text.to_string()));
            }

            // Group content by name for unordered handling
            let mut content_groups: HashMap<String, Vec<JsonValue>> = HashMap::new();

            for item in data.content() {
                match item {
                    ContentItem::Element(name, value) => {
                        let key = self.map_qname(name);
                        content_groups
                            .entry(key)
                            .or_insert_with(Vec::new)
                            .push(value.clone());
                    }
                    ContentItem::CData(index, text) => {
                        if let Some(prefix) = self.config.cdata_prefix() {
                            let key = format!("{}{}", prefix, index);
                            result.insert(key, JsonValue::String(text.clone()));
                        }
                    }
                }
            }

            // Add grouped content
            for (key, values) in content_groups {
                if values.len() == 1 && !self.config.force_list() {
                    result.insert(key, values.into_iter().next().unwrap());
                } else {
                    result.insert(key, JsonValue::Array(values));
                }
            }
        }

        if level == 0 && self.config.preserve_root() {
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
                // Handle root wrapper
                let obj = if level == 0 && self.config.preserve_root() && obj.len() == 1 {
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

                // Collect all content items (we don't enforce order)
                for (key, val) in obj {
                    if key == text_key {
                        if let JsonValue::String(s) = val {
                            data.text = Some(s.clone());
                        }
                    } else if key.starts_with(attr_prefix) && key != attr_prefix {
                        let attr_name = &key[attr_prefix.len()..];
                        if let JsonValue::String(s) = val {
                            data.attributes.insert(attr_name.to_string(), s.clone());
                        } else {
                            data.attributes.insert(attr_name.to_string(), val.to_string());
                        }
                    } else if let Some(prefix) = self.config.cdata_prefix() {
                        if key.starts_with(prefix) {
                            if let Ok(index) = key[prefix.len()..].parse::<usize>() {
                                if let JsonValue::String(s) = val {
                                    data.content.push(ContentItem::CData(index, s.clone()));
                                }
                            }
                        } else {
                            self.add_content_items(&mut data, key, val);
                        }
                    } else {
                        self.add_content_items(&mut data, key, val);
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
        self.config.cdata_prefix().is_none()
    }

    fn loses_xmlns(&self) -> bool {
        false
    }
}

impl UnorderedConverter {
    fn add_content_items(&self, data: &mut ElementData, key: &str, val: &JsonValue) {
        match val {
            JsonValue::Array(arr) => {
                for item in arr {
                    data.content.push(ContentItem::Element(key.to_string(), item.clone()));
                }
            }
            JsonValue::Object(obj) => {
                // For unordered converter, object values may represent
                // element content in a different format
                data.content.push(ContentItem::Element(key.to_string(), val.clone()));
            }
            _ => {
                data.content.push(ContentItem::Element(key.to_string(), val.clone()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_unordered_simple_text() {
        let converter = UnorderedConverter::new();
        let data = ElementData::new("element").with_text("value");

        let json = converter.decode(&data, 0);
        assert_eq!(json, JsonValue::String("value".to_string()));
    }

    #[test]
    fn test_unordered_with_attributes() {
        let converter = UnorderedConverter::new();
        let data = ElementData::new("element")
            .with_text("value")
            .with_attribute("id", "123");

        let json = converter.decode(&data, 0);

        if let JsonValue::Object(obj) = json {
            assert_eq!(obj.get("@id"), Some(&JsonValue::String("123".to_string())));
            assert_eq!(obj.get("$"), Some(&JsonValue::String("value".to_string())));
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_unordered_groups_same_elements() {
        let converter = UnorderedConverter::new();
        let data = ElementData::new("root")
            .with_child("item", json!("first"))
            .with_child("other", json!("middle"))
            .with_child("item", json!("second"));

        let json = converter.decode(&data, 0);

        if let JsonValue::Object(obj) = json {
            // Items should be grouped into an array
            if let Some(JsonValue::Array(items)) = obj.get("item") {
                assert_eq!(items.len(), 2);
            } else {
                panic!("Expected items array");
            }
            // Other should be a single value
            assert_eq!(obj.get("other"), Some(&JsonValue::String("middle".to_string())));
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_unordered_encode() {
        let converter = UnorderedConverter::new();
        let json = json!({
            "@id": "123",
            "$": "content",
            "child1": "value1",
            "child2": ["a", "b", "c"]
        });

        let data = converter.encode(&json, "root", 0);
        assert_eq!(data.tag(), "root");
        assert_eq!(data.text(), Some("content"));
        assert_eq!(data.attributes().get("id"), Some(&"123".to_string()));
        // child1 = 1 element, child2 = 3 elements
        assert_eq!(data.content().len(), 4);
    }

    #[test]
    fn test_unordered_force_list() {
        let config = ConverterConfig::default().with_force_list(true);
        let converter = UnorderedConverter::with_config(config);
        let data = ElementData::new("root").with_child("item", json!("value"));

        let json = converter.decode(&data, 0);

        if let JsonValue::Object(obj) = json {
            if let Some(JsonValue::Array(arr)) = obj.get("item") {
                assert_eq!(arr.len(), 1);
            } else {
                panic!("Expected array even for single item");
            }
        }
    }

    #[test]
    fn test_unordered_is_lossy() {
        let converter = UnorderedConverter::new();
        // Default config has no cdata_prefix, so lossy
        assert!(converter.is_lossy());
        assert!(!converter.loses_xmlns());
    }

    #[test]
    fn test_unordered_with_cdata_prefix() {
        let config = ConverterConfig::default().with_cdata_prefix(Some("#".to_string()));
        let converter = UnorderedConverter::with_config(config);
        assert!(!converter.is_lossy());
    }
}
