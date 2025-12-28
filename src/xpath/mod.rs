//! XPath Support for XML Schema
//!
//! This module provides XPath-related functionality for XML Schema validation.
//!
//! ## Overview
//!
//! XPath is used in XSD for:
//! - Identity constraints (xs:selector, xs:field)
//! - XSD 1.1 assertions (xs:assert)
//! - Type alternatives
//!
//! ## Limitations
//!
//! This is a basic implementation that supports:
//! - Path splitting and analysis
//! - Simple step-based selectors
//! - NCName validation
//!
//! For full XPath 2.0/3.0 support, integration with a dedicated
//! XPath engine would be required.

mod selectors;
mod proxy;
mod parsers;

pub use selectors::{
    ElementSelector, PathStep, PathStepKind, split_path, is_ncname, is_ncname_char,
};
pub use proxy::SchemaProxy;
pub use parsers::{
    IdentityXPathParser, AssertionXPathParser, ParsedXPath, XPathAxis, XPathPredicate,
};

use std::collections::HashMap;

/// Namespace mapping type
pub type NamespaceMap = HashMap<String, String>;

/// Result of XPath evaluation
#[derive(Debug, Clone)]
pub enum XPathResult {
    /// A node set result
    Nodes(Vec<XPathNode>),
    /// A boolean result
    Boolean(bool),
    /// A number result
    Number(f64),
    /// A string result
    String(String),
    /// An empty result
    Empty,
}

impl XPathResult {
    /// Check if the result is true (for boolean or non-empty nodes)
    pub fn is_truthy(&self) -> bool {
        match self {
            XPathResult::Boolean(b) => *b,
            XPathResult::Nodes(nodes) => !nodes.is_empty(),
            XPathResult::Number(n) => *n != 0.0 && !n.is_nan(),
            XPathResult::String(s) => !s.is_empty(),
            XPathResult::Empty => false,
        }
    }

    /// Get as nodes if applicable
    pub fn as_nodes(&self) -> Option<&Vec<XPathNode>> {
        if let XPathResult::Nodes(nodes) = self {
            Some(nodes)
        } else {
            None
        }
    }

    /// Get as boolean
    pub fn as_bool(&self) -> bool {
        self.is_truthy()
    }

    /// Get as string
    pub fn as_string(&self) -> String {
        match self {
            XPathResult::String(s) => s.clone(),
            XPathResult::Number(n) => n.to_string(),
            XPathResult::Boolean(b) => b.to_string(),
            XPathResult::Nodes(nodes) => {
                nodes.first().map(|n| n.value.clone()).unwrap_or_default()
            }
            XPathResult::Empty => String::new(),
        }
    }
}

/// Represents a node in XPath result
#[derive(Debug, Clone)]
pub struct XPathNode {
    /// Node type
    pub node_type: XPathNodeType,
    /// Node name (for elements/attributes)
    pub name: String,
    /// Node namespace
    pub namespace: Option<String>,
    /// Node value/text content
    pub value: String,
    /// Position in the result set
    pub position: usize,
}

impl XPathNode {
    /// Create a new element node
    pub fn element(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            node_type: XPathNodeType::Element,
            name: name.into(),
            namespace: None,
            value: value.into(),
            position: 0,
        }
    }

    /// Create an attribute node
    pub fn attribute(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            node_type: XPathNodeType::Attribute,
            name: name.into(),
            namespace: None,
            value: value.into(),
            position: 0,
        }
    }

    /// Set the namespace
    pub fn with_namespace(mut self, ns: impl Into<String>) -> Self {
        self.namespace = Some(ns.into());
        self
    }

    /// Set the position
    pub fn with_position(mut self, pos: usize) -> Self {
        self.position = pos;
        self
    }
}

/// Node types in XPath
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XPathNodeType {
    /// Root node
    Root,
    /// Element node
    Element,
    /// Attribute node
    Attribute,
    /// Text node
    Text,
    /// Comment node
    Comment,
    /// Processing instruction
    ProcessingInstruction,
    /// Namespace node
    Namespace,
}

/// XPath context for evaluation
#[derive(Debug, Clone)]
pub struct XPathContext {
    /// Namespace mappings
    pub namespaces: NamespaceMap,
    /// Current context node
    pub context_node: Option<XPathNode>,
    /// Context position
    pub position: usize,
    /// Context size
    pub size: usize,
    /// Variable bindings
    pub variables: HashMap<String, XPathResult>,
}

impl Default for XPathContext {
    fn default() -> Self {
        Self::new()
    }
}

impl XPathContext {
    /// Create a new context
    pub fn new() -> Self {
        Self {
            namespaces: NamespaceMap::new(),
            context_node: None,
            position: 1,
            size: 1,
            variables: HashMap::new(),
        }
    }

    /// Set namespace mappings
    pub fn with_namespaces(mut self, namespaces: NamespaceMap) -> Self {
        self.namespaces = namespaces;
        self
    }

    /// Set a variable
    pub fn with_variable(mut self, name: impl Into<String>, value: XPathResult) -> Self {
        self.variables.insert(name.into(), value);
        self
    }

    /// Set the context node
    pub fn with_context_node(mut self, node: XPathNode) -> Self {
        self.context_node = Some(node);
        self
    }

    /// Get a namespace URI by prefix
    pub fn get_namespace(&self, prefix: &str) -> Option<&String> {
        self.namespaces.get(prefix)
    }

    /// Expand a prefixed name to Clark notation
    pub fn expand_name(&self, name: &str) -> String {
        if let Some(pos) = name.find(':') {
            let prefix = &name[..pos];
            let local = &name[pos + 1..];
            if let Some(uri) = self.namespaces.get(prefix) {
                return format!("{{{}}}{}", uri, local);
            }
        }
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xpath_result_truthy() {
        assert!(XPathResult::Boolean(true).is_truthy());
        assert!(!XPathResult::Boolean(false).is_truthy());
        assert!(!XPathResult::Empty.is_truthy());
        assert!(XPathResult::Number(1.0).is_truthy());
        assert!(!XPathResult::Number(0.0).is_truthy());
        assert!(XPathResult::String("test".to_string()).is_truthy());
        assert!(!XPathResult::String("".to_string()).is_truthy());
    }

    #[test]
    fn test_xpath_node_creation() {
        let elem = XPathNode::element("test", "value")
            .with_namespace("http://example.com")
            .with_position(1);

        assert_eq!(elem.name, "test");
        assert_eq!(elem.value, "value");
        assert_eq!(elem.namespace, Some("http://example.com".to_string()));
        assert_eq!(elem.position, 1);
    }

    #[test]
    fn test_xpath_context() {
        let mut namespaces = NamespaceMap::new();
        namespaces.insert("xs".to_string(), "http://www.w3.org/2001/XMLSchema".to_string());

        let ctx = XPathContext::new()
            .with_namespaces(namespaces)
            .with_variable("test", XPathResult::Number(42.0));

        assert_eq!(
            ctx.get_namespace("xs"),
            Some(&"http://www.w3.org/2001/XMLSchema".to_string())
        );
        assert!(ctx.variables.contains_key("test"));
    }

    #[test]
    fn test_expand_name() {
        let mut namespaces = NamespaceMap::new();
        namespaces.insert("ns".to_string(), "http://example.com".to_string());

        let ctx = XPathContext::new().with_namespaces(namespaces);

        assert_eq!(ctx.expand_name("ns:element"), "{http://example.com}element");
        assert_eq!(ctx.expand_name("local"), "local");
    }
}
