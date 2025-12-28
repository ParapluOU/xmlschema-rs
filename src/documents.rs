//! XML document handling and validation
//!
//! This module provides functionality for working with XML documents.

use crate::error::{Error, Result};
use crate::namespaces::{NamespaceContext, QName};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use std::collections::HashMap;

/// XML Element in the document tree
#[derive(Debug, Clone)]
pub struct Element {
    /// Element qualified name
    pub qname: QName,
    /// Element attributes
    pub attributes: HashMap<QName, String>,
    /// Text content (if any)
    pub text: Option<String>,
    /// Child elements
    pub children: Vec<Element>,
    /// Namespace context for this element
    pub namespaces: NamespaceContext,
}

impl Element {
    /// Create a new element
    pub fn new(qname: QName) -> Self {
        Self {
            qname,
            attributes: HashMap::new(),
            text: None,
            children: Vec::new(),
            namespaces: NamespaceContext::new(),
        }
    }

    /// Get the local name of the element
    pub fn local_name(&self) -> &str {
        &self.qname.local_name
    }

    /// Get the namespace of the element
    pub fn namespace(&self) -> Option<&str> {
        self.qname.namespace.as_deref()
    }

    /// Get an attribute value by name
    pub fn get_attribute(&self, name: &str) -> Option<&str> {
        // Try local name first
        for (qname, value) in &self.attributes {
            if qname.local_name == name {
                return Some(value);
            }
        }
        None
    }

    /// Get an attribute value by qualified name
    pub fn get_attribute_qname(&self, qname: &QName) -> Option<&str> {
        self.attributes.get(qname).map(|s| s.as_str())
    }

    /// Add a child element
    pub fn add_child(&mut self, child: Element) {
        self.children.push(child);
    }

    /// Set text content
    pub fn set_text(&mut self, text: String) {
        self.text = Some(text);
    }

    /// Find child elements by local name
    pub fn find_children(&self, local_name: &str) -> Vec<&Element> {
        self.children
            .iter()
            .filter(|e| e.local_name() == local_name)
            .collect()
    }
}

/// XML Document representation
#[derive(Debug)]
pub struct Document {
    /// Root element of the document
    pub root: Option<Element>,
    /// Document namespace context
    pub namespaces: NamespaceContext,
}

impl Document {
    /// Create a new empty document
    pub fn new() -> Self {
        Self {
            root: None,
            namespaces: NamespaceContext::new(),
        }
    }

    /// Parse an XML document from a string
    pub fn from_string(xml: &str) -> Result<Self> {
        Self::parse(xml.as_bytes())
    }

    /// Parse an XML document from bytes
    pub fn parse(xml: &[u8]) -> Result<Self> {
        let mut reader = Reader::from_reader(xml);
        reader.trim_text(true);

        let mut doc = Document::new();
        let mut element_stack: Vec<Element> = Vec::new();
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let element = Self::parse_element(&e, &reader)?;
                    element_stack.push(element);
                }
                Ok(Event::End(_)) => {
                    if let Some(current) = element_stack.pop() {
                        if let Some(parent) = element_stack.last_mut() {
                            parent.add_child(current);
                        } else {
                            // This is the root element
                            doc.root = Some(current);
                        }
                    }
                }
                Ok(Event::Empty(e)) => {
                    let element = Self::parse_element(&e, &reader)?;
                    if let Some(parent) = element_stack.last_mut() {
                        parent.add_child(element);
                    } else {
                        // Empty root element
                        doc.root = Some(element);
                    }
                }
                Ok(Event::Text(e)) => {
                    if let Some(current) = element_stack.last_mut() {
                        let text = e
                            .unescape()
                            .map_err(|e| Error::Xml(format!("Failed to unescape text: {}", e)))?
                            .to_string();
                        if !text.trim().is_empty() {
                            current.set_text(text);
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(Error::Xml(format!(
                        "Error parsing XML at position {}: {}",
                        reader.buffer_position(),
                        e
                    )))
                }
                _ => {} // Ignore other events (comments, processing instructions, etc.)
            }
            buf.clear();
        }

        Ok(doc)
    }

    /// Parse element from BytesStart event
    fn parse_element(start: &BytesStart, _reader: &Reader<&[u8]>) -> Result<Element> {
        let name_bytes = start.name();
        let name = std::str::from_utf8(name_bytes.as_ref())
            .map_err(|e| Error::Xml(format!("Invalid element name: {}", e)))?
            .to_string();

        // Parse namespace and local name
        let qname = if let Some((_prefix, local)) = name.split_once(':') {
            QName::local(local) // Namespace will be resolved later
        } else {
            QName::local(&name)
        };

        let mut element = Element::new(qname);

        // Parse attributes
        for attr_result in start.attributes() {
            let attr = attr_result
                .map_err(|e| Error::Xml(format!("Failed to parse attribute: {}", e)))?;

            let attr_name = std::str::from_utf8(attr.key.as_ref())
                .map_err(|e| Error::Xml(format!("Invalid attribute name: {}", e)))?;

            let attr_value = attr
                .unescape_value()
                .map_err(|e| Error::Xml(format!("Failed to unescape attribute value: {}", e)))?
                .to_string();

            // Handle namespace declarations
            if attr_name == "xmlns" {
                element.namespaces.set_default_namespace(&attr_value);
            } else if let Some(prefix) = attr_name.strip_prefix("xmlns:") {
                element.namespaces.add_prefix(prefix, &attr_value);
            } else {
                // Regular attribute
                let attr_qname = if let Some((_prefix, local)) = attr_name.split_once(':') {
                    QName::local(local) // Namespace will be resolved later
                } else {
                    QName::local(attr_name)
                };
                element.attributes.insert(attr_qname, attr_value);
            }
        }

        Ok(element)
    }

    /// Get the root element
    pub fn root(&self) -> Option<&Element> {
        self.root.as_ref()
    }

    /// Get the root element mutably
    pub fn root_mut(&mut self) -> Option<&mut Element> {
        self.root.as_mut()
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_creation() {
        let doc = Document::new();
        assert!(doc.root.is_none());
    }

    #[test]
    fn test_parse_simple_xml() {
        let xml = r#"<root><child>text</child></root>"#;
        let doc = Document::from_string(xml).unwrap();

        assert!(doc.root.is_some());
        let root = doc.root.unwrap();
        assert_eq!(root.local_name(), "root");
        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].local_name(), "child");
        assert_eq!(root.children[0].text.as_deref(), Some("text"));
    }

    #[test]
    fn test_parse_with_attributes() {
        let xml = r#"<root attr1="value1" attr2="value2"><child/></root>"#;
        let doc = Document::from_string(xml).unwrap();

        let root = doc.root.unwrap();
        assert_eq!(root.get_attribute("attr1"), Some("value1"));
        assert_eq!(root.get_attribute("attr2"), Some("value2"));
    }

    #[test]
    fn test_parse_with_namespaces() {
        let xml = r#"<root xmlns="http://example.com" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"/>"#;
        let doc = Document::from_string(xml).unwrap();

        let root = doc.root.unwrap();
        assert_eq!(
            root.namespaces.get_default_namespace(),
            Some("http://example.com")
        );
        assert_eq!(
            root.namespaces.get_namespace("xsi"),
            Some("http://www.w3.org/2001/XMLSchema-instance")
        );
    }

    #[test]
    fn test_find_children() {
        let xml = r#"<root><child1/><child2/><child1/></root>"#;
        let doc = Document::from_string(xml).unwrap();

        let root = doc.root.unwrap();
        let children = root.find_children("child1");
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn test_element_creation() {
        let qname = QName::local("test");
        let mut elem = Element::new(qname);
        elem.set_text("content".to_string());

        assert_eq!(elem.local_name(), "test");
        assert_eq!(elem.text.as_deref(), Some("content"));
    }
}
