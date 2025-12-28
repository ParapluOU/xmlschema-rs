//! BadgerFish Convention Converter
//!
//! Implements the BadgerFish convention for XML to JSON conversion.
//!
//! References:
//! - http://www.sklar.com/badgerfish/
//! - https://badgerfish.ning.com/
//!
//! The BadgerFish convention preserves more XML structure:
//! - Element content is in "$" key
//! - Attributes are prefixed with "@"
//! - Namespace declarations in "@xmlns"
//! - Each element is wrapped in an object with its tag name

use serde_json::{Map, Value as JsonValue};

use super::base::{ContentItem, ConverterConfig, ElementData};
use super::JsonConverter;

/// BadgerFish convention converter
///
/// A lossless converter that preserves:
/// - All attributes (prefixed with @)
/// - Namespace declarations (in @xmlns object)
/// - Character data indices (prefixed with $)
/// - Element structure (wrapped in tag name objects)
#[derive(Debug, Clone)]
pub struct BadgerFishConverter {
    config: ConverterConfig,
}

impl Default for BadgerFishConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl BadgerFishConverter {
    /// Create a new BadgerFish converter
    pub fn new() -> Self {
        let config = ConverterConfig::new()
            .with_attr_prefix(Some("@".to_string()))
            .with_text_key(Some("$".to_string()))
            .with_cdata_prefix(Some("$".to_string()));

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

impl JsonConverter for BadgerFishConverter {
    fn decode(&self, data: &ElementData, level: usize) -> JsonValue {
        let tag = self.map_qname(data.tag());
        let mut result = Map::new();

        // Add attributes
        for (name, value) in data.attributes() {
            let key = format!("@{}", name);
            result.insert(key, JsonValue::String(value.clone()));
        }

        // Add xmlns declarations if present
        if level == 0 && !data.xmlns().is_empty() {
            let mut xmlns_obj = Map::new();
            for (prefix, uri) in data.xmlns() {
                let key = if prefix.is_empty() {
                    "$".to_string()
                } else {
                    prefix.clone()
                };
                xmlns_obj.insert(key, JsonValue::String(uri.clone()));
            }
            result.insert("@xmlns".to_string(), JsonValue::Object(xmlns_obj));
        }

        // Handle content
        if data.content().is_empty() {
            // Simple content
            if let Some(text) = data.text() {
                result.insert("$".to_string(), JsonValue::String(text.to_string()));
            }
        } else {
            // Complex content
            if let Some(text) = data.text() {
                result.insert("$".to_string(), JsonValue::String(text.to_string()));
            }

            for item in data.content() {
                match item {
                    ContentItem::Element(name, value) => {
                        let key = self.map_qname(name);

                        // BadgerFish wraps each element in its tag name
                        // So the value should be unwrapped if it's already wrapped
                        let inner_value = if let JsonValue::Object(obj) = value {
                            if obj.len() == 1 {
                                if let Some((inner_key, inner_val)) = obj.iter().next() {
                                    if inner_key == &key {
                                        inner_val.clone()
                                    } else {
                                        value.clone()
                                    }
                                } else {
                                    value.clone()
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
                                arr.push(inner_value);
                            } else {
                                let old = existing.take();
                                *existing = JsonValue::Array(vec![old, inner_value]);
                            }
                        } else {
                            result.insert(key, inner_value);
                        }
                    }
                    ContentItem::CData(index, text) => {
                        let key = format!("${}", index);
                        result.insert(key, JsonValue::String(text.clone()));
                    }
                }
            }
        }

        // Wrap in tag name object
        let mut wrapper = Map::new();
        wrapper.insert(tag, JsonValue::Object(result));
        JsonValue::Object(wrapper)
    }

    fn encode(&self, value: &JsonValue, tag: &str, _level: usize) -> ElementData {
        let mut data = ElementData::new(tag);

        // BadgerFish values are wrapped in tag name objects
        let obj = if let JsonValue::Object(outer) = value {
            if outer.len() == 1 {
                if let Some((key, inner)) = outer.iter().next() {
                    // Check if this is the tag wrapper
                    if key == tag {
                        if let JsonValue::Object(inner_obj) = inner {
                            inner_obj
                        } else {
                            outer
                        }
                    } else {
                        outer
                    }
                } else {
                    outer
                }
            } else {
                outer
            }
        } else {
            return data;
        };

        for (key, val) in obj {
            if key == "$" {
                // Text content
                if let JsonValue::String(s) = val {
                    data.text = Some(s.clone());
                }
            } else if key == "@xmlns" {
                // Namespace declarations
                if let JsonValue::Object(xmlns_obj) = val {
                    for (prefix, uri) in xmlns_obj {
                        let prefix = if prefix == "$" {
                            String::new()
                        } else {
                            prefix.clone()
                        };
                        if let JsonValue::String(uri_str) = uri {
                            data.xmlns.push((prefix, uri_str.clone()));
                        }
                    }
                }
            } else if key.starts_with('@') {
                // Attribute
                let attr_name = &key[1..];
                if let JsonValue::String(s) = val {
                    data.attributes.insert(attr_name.to_string(), s.clone());
                } else {
                    data.attributes.insert(attr_name.to_string(), val.to_string());
                }
            } else if key.starts_with('$') && key.len() > 1 && key[1..].chars().all(|c| c.is_ascii_digit()) {
                // Character data ($0, $1, etc.)
                if let Ok(index) = key[1..].parse::<usize>() {
                    if let JsonValue::String(s) = val {
                        data.content.push(ContentItem::CData(index, s.clone()));
                    }
                }
            } else {
                // Child element
                match val {
                    JsonValue::Array(arr) => {
                        for item in arr {
                            // Wrap in tag name for proper BadgerFish format
                            let mut wrapper = Map::new();
                            wrapper.insert(key.clone(), item.clone());
                            data.content.push(ContentItem::Element(
                                key.clone(),
                                JsonValue::Object(wrapper),
                            ));
                        }
                    }
                    _ => {
                        let mut wrapper = Map::new();
                        wrapper.insert(key.clone(), val.clone());
                        data.content.push(ContentItem::Element(
                            key.clone(),
                            JsonValue::Object(wrapper),
                        ));
                    }
                }
            }
        }

        data
    }

    fn is_lossy(&self) -> bool {
        false // BadgerFish preserves all information
    }

    fn loses_xmlns(&self) -> bool {
        false // BadgerFish preserves namespace declarations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_badgerfish_simple_text() {
        let converter = BadgerFishConverter::new();
        let data = ElementData::new("element").with_text("value");

        let json = converter.decode(&data, 0);

        if let JsonValue::Object(obj) = &json {
            assert!(obj.contains_key("element"));
            if let Some(JsonValue::Object(inner)) = obj.get("element") {
                assert_eq!(inner.get("$"), Some(&JsonValue::String("value".to_string())));
            } else {
                panic!("Expected inner object");
            }
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_badgerfish_with_attributes() {
        let converter = BadgerFishConverter::new();
        let data = ElementData::new("element")
            .with_text("value")
            .with_attribute("id", "123");

        let json = converter.decode(&data, 0);

        if let JsonValue::Object(obj) = &json {
            if let Some(JsonValue::Object(inner)) = obj.get("element") {
                assert_eq!(inner.get("@id"), Some(&JsonValue::String("123".to_string())));
                assert_eq!(inner.get("$"), Some(&JsonValue::String("value".to_string())));
            } else {
                panic!("Expected inner object");
            }
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_badgerfish_with_xmlns() {
        let converter = BadgerFishConverter::new();
        let data = ElementData::new("root")
            .with_xmlns("", "http://example.com")
            .with_xmlns("ns", "http://example.com/ns");

        let json = converter.decode(&data, 0);

        if let JsonValue::Object(obj) = &json {
            if let Some(JsonValue::Object(inner)) = obj.get("root") {
                if let Some(JsonValue::Object(xmlns)) = inner.get("@xmlns") {
                    assert_eq!(
                        xmlns.get("$"),
                        Some(&JsonValue::String("http://example.com".to_string()))
                    );
                    assert_eq!(
                        xmlns.get("ns"),
                        Some(&JsonValue::String("http://example.com/ns".to_string()))
                    );
                } else {
                    panic!("Expected @xmlns object");
                }
            } else {
                panic!("Expected inner object");
            }
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_badgerfish_with_cdata() {
        let converter = BadgerFishConverter::new();
        let data = ElementData::new("mixed")
            .with_cdata(0, "Hello ")
            .with_child("name", json!({"name": {"$": "World"}}))
            .with_cdata(1, "!");

        let json = converter.decode(&data, 0);

        if let JsonValue::Object(obj) = &json {
            if let Some(JsonValue::Object(inner)) = obj.get("mixed") {
                assert_eq!(
                    inner.get("$0"),
                    Some(&JsonValue::String("Hello ".to_string()))
                );
                assert_eq!(inner.get("$1"), Some(&JsonValue::String("!".to_string())));
            } else {
                panic!("Expected inner object");
            }
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_badgerfish_encode_simple() {
        let converter = BadgerFishConverter::new();
        let json = json!({
            "element": {
                "$": "value",
                "@id": "123"
            }
        });

        let data = converter.encode(&json, "element", 0);
        assert_eq!(data.tag(), "element");
        assert_eq!(data.text(), Some("value"));
        assert_eq!(data.attributes().get("id"), Some(&"123".to_string()));
    }

    #[test]
    fn test_badgerfish_encode_with_xmlns() {
        let converter = BadgerFishConverter::new();
        let json = json!({
            "root": {
                "@xmlns": {
                    "$": "http://example.com",
                    "ns": "http://example.com/ns"
                }
            }
        });

        let data = converter.encode(&json, "root", 0);
        assert_eq!(data.xmlns().len(), 2);
    }

    #[test]
    fn test_badgerfish_is_lossless() {
        let converter = BadgerFishConverter::new();
        assert!(!converter.is_lossy());
        assert!(!converter.loses_xmlns());
        assert!(converter.is_lossless());
    }

    #[test]
    fn test_badgerfish_multiple_children() {
        let converter = BadgerFishConverter::new();
        let data = ElementData::new("root")
            .with_child("item", json!({"item": {"$": "first"}}))
            .with_child("item", json!({"item": {"$": "second"}}));

        let json = converter.decode(&data, 0);

        if let JsonValue::Object(obj) = &json {
            if let Some(JsonValue::Object(inner)) = obj.get("root") {
                if let Some(JsonValue::Array(arr)) = inner.get("item") {
                    assert_eq!(arr.len(), 2);
                } else {
                    panic!("Expected item array");
                }
            }
        }
    }
}
