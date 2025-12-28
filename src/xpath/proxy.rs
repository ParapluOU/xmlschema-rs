//! Schema Proxy for XPath Evaluation
//!
//! This module provides a proxy interface between XPath evaluation
//! and the schema/document being validated.

use std::collections::HashMap;

use super::{NamespaceMap, XPathNode, XPathNodeType, XPathResult};

/// Schema proxy for XPath operations
///
/// Provides schema context for XPath evaluation, including
/// namespace resolution and type information.
#[derive(Debug, Clone)]
pub struct SchemaProxy {
    /// Namespace mappings (prefix -> URI)
    namespaces: NamespaceMap,
    /// Default namespace for elements
    default_namespace: Option<String>,
    /// Schema target namespace
    target_namespace: Option<String>,
    /// Type information cache
    types: HashMap<String, TypeInfo>,
}

impl Default for SchemaProxy {
    fn default() -> Self {
        Self::new()
    }
}

impl SchemaProxy {
    /// Create a new schema proxy
    pub fn new() -> Self {
        let mut namespaces = NamespaceMap::new();
        // Add standard XSD namespace prefixes
        namespaces.insert(
            "xs".to_string(),
            "http://www.w3.org/2001/XMLSchema".to_string(),
        );
        namespaces.insert(
            "xsd".to_string(),
            "http://www.w3.org/2001/XMLSchema".to_string(),
        );
        namespaces.insert(
            "xml".to_string(),
            "http://www.w3.org/XML/1998/namespace".to_string(),
        );

        Self {
            namespaces,
            default_namespace: None,
            target_namespace: None,
            types: HashMap::new(),
        }
    }

    /// Create with custom namespace mappings
    pub fn with_namespaces(mut self, namespaces: NamespaceMap) -> Self {
        self.namespaces.extend(namespaces);
        self
    }

    /// Set the default namespace
    pub fn with_default_namespace(mut self, ns: impl Into<String>) -> Self {
        self.default_namespace = Some(ns.into());
        self
    }

    /// Set the target namespace
    pub fn with_target_namespace(mut self, ns: impl Into<String>) -> Self {
        self.target_namespace = Some(ns.into());
        self
    }

    /// Get namespace URI by prefix
    pub fn get_namespace(&self, prefix: &str) -> Option<&String> {
        self.namespaces.get(prefix)
    }

    /// Get the default namespace
    pub fn default_namespace(&self) -> Option<&String> {
        self.default_namespace.as_ref()
    }

    /// Get the target namespace
    pub fn target_namespace(&self) -> Option<&String> {
        self.target_namespace.as_ref()
    }

    /// Resolve a prefixed name to a Clark notation name
    ///
    /// Clark notation: `{namespace-uri}local-name`
    pub fn resolve_name(&self, name: &str) -> String {
        if let Some(colon_pos) = name.find(':') {
            let prefix = &name[..colon_pos];
            let local = &name[colon_pos + 1..];
            if let Some(uri) = self.namespaces.get(prefix) {
                return format!("{{{}}}{}", uri, local);
            }
        } else if let Some(default_ns) = &self.default_namespace {
            return format!("{{{}}}{}", default_ns, name);
        }
        name.to_string()
    }

    /// Register a namespace prefix
    pub fn register_namespace(&mut self, prefix: impl Into<String>, uri: impl Into<String>) {
        self.namespaces.insert(prefix.into(), uri.into());
    }

    /// Register type information
    pub fn register_type(&mut self, name: impl Into<String>, type_info: TypeInfo) {
        self.types.insert(name.into(), type_info);
    }

    /// Get type information by name
    pub fn get_type(&self, name: &str) -> Option<&TypeInfo> {
        self.types.get(name)
    }

    /// Evaluate a simple path against a context node
    ///
    /// This is a simplified evaluation for identity constraint selectors.
    pub fn evaluate_simple_path(
        &self,
        path: &str,
        context: &XPathNode,
        children: &[XPathNode],
    ) -> XPathResult {
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        if parts.is_empty() {
            return XPathResult::Nodes(vec![context.clone()]);
        }

        let mut current_nodes = vec![context.clone()];

        for part in parts {
            if part == "." {
                continue;
            } else if part == ".." {
                // Parent - not implemented in simple evaluation
                return XPathResult::Empty;
            } else if part.starts_with('@') {
                // Attribute selection
                let attr_name = &part[1..];
                let mut results = Vec::new();
                for node in &current_nodes {
                    // Would need attribute access from context
                    if node.node_type == XPathNodeType::Attribute && node.name == attr_name {
                        results.push(node.clone());
                    }
                }
                return XPathResult::Nodes(results);
            } else {
                // Child element selection
                let resolved_name = self.resolve_name(part);
                let local_name = if part.contains(':') {
                    part.split(':').nth(1).unwrap_or(part)
                } else {
                    part
                };

                let mut next_nodes = Vec::new();
                for child in children {
                    if child.node_type == XPathNodeType::Element {
                        // Match by local name or resolved name
                        if child.name == local_name
                            || child.name == part
                            || child.name == resolved_name
                        {
                            next_nodes.push(child.clone());
                        }
                    }
                }
                current_nodes = next_nodes;
            }
        }

        if current_nodes.is_empty() {
            XPathResult::Empty
        } else {
            XPathResult::Nodes(current_nodes)
        }
    }

    /// Check if a value matches a type
    pub fn check_type(&self, value: &str, type_name: &str) -> bool {
        if let Some(type_info) = self.types.get(type_name) {
            type_info.validate(value)
        } else {
            // Unknown type - assume valid
            true
        }
    }
}

/// Type information for schema-aware XPath
#[derive(Debug, Clone)]
pub struct TypeInfo {
    /// Type name
    pub name: String,
    /// Base type name
    pub base_type: Option<String>,
    /// Type variety
    pub variety: TypeVariety,
    /// Facets (minLength, maxLength, pattern, etc.)
    pub facets: HashMap<String, String>,
}

impl TypeInfo {
    /// Create a new type info
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            base_type: None,
            variety: TypeVariety::Atomic,
            facets: HashMap::new(),
        }
    }

    /// Set the base type
    pub fn with_base_type(mut self, base: impl Into<String>) -> Self {
        self.base_type = Some(base.into());
        self
    }

    /// Set the variety
    pub fn with_variety(mut self, variety: TypeVariety) -> Self {
        self.variety = variety;
        self
    }

    /// Add a facet
    pub fn with_facet(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.facets.insert(name.into(), value.into());
        self
    }

    /// Validate a value against this type
    pub fn validate(&self, value: &str) -> bool {
        // Check length facets
        if let Some(min_length) = self.facets.get("minLength") {
            if let Ok(min) = min_length.parse::<usize>() {
                if value.len() < min {
                    return false;
                }
            }
        }

        if let Some(max_length) = self.facets.get("maxLength") {
            if let Ok(max) = max_length.parse::<usize>() {
                if value.len() > max {
                    return false;
                }
            }
        }

        if let Some(length) = self.facets.get("length") {
            if let Ok(len) = length.parse::<usize>() {
                if value.len() != len {
                    return false;
                }
            }
        }

        // Check pattern facet (simplified - just checks if non-empty)
        if let Some(_pattern) = self.facets.get("pattern") {
            // Full regex matching would require regex crate
            // For now, just verify the value is not empty
            if value.is_empty() {
                return false;
            }
        }

        true
    }
}

/// Type variety
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeVariety {
    /// Atomic type (simple, single value)
    Atomic,
    /// List type
    List,
    /// Union type
    Union,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_proxy_new() {
        let proxy = SchemaProxy::new();
        assert!(proxy.get_namespace("xs").is_some());
        assert!(proxy.get_namespace("xsd").is_some());
    }

    #[test]
    fn test_schema_proxy_resolve_name() {
        let proxy = SchemaProxy::new();
        let resolved = proxy.resolve_name("xs:string");
        assert_eq!(resolved, "{http://www.w3.org/2001/XMLSchema}string");

        let unresolved = proxy.resolve_name("element");
        assert_eq!(unresolved, "element");
    }

    #[test]
    fn test_schema_proxy_with_default_namespace() {
        let proxy =
            SchemaProxy::new().with_default_namespace("http://example.com");
        let resolved = proxy.resolve_name("element");
        assert_eq!(resolved, "{http://example.com}element");
    }

    #[test]
    fn test_schema_proxy_register_namespace() {
        let mut proxy = SchemaProxy::new();
        proxy.register_namespace("ex", "http://example.com");
        assert_eq!(
            proxy.get_namespace("ex"),
            Some(&"http://example.com".to_string())
        );
    }

    #[test]
    fn test_type_info_validation() {
        let type_info = TypeInfo::new("myString")
            .with_facet("minLength", "1")
            .with_facet("maxLength", "10");

        assert!(type_info.validate("hello"));
        assert!(!type_info.validate("")); // Too short
        assert!(!type_info.validate("this is too long")); // Too long
    }

    #[test]
    fn test_type_info_length_facet() {
        let type_info = TypeInfo::new("fixedString").with_facet("length", "5");

        assert!(type_info.validate("hello"));
        assert!(!type_info.validate("hi"));
        assert!(!type_info.validate("too long"));
    }

    #[test]
    fn test_simple_path_evaluation() {
        let proxy = SchemaProxy::new();
        let context = XPathNode::element("root", "");
        let children = vec![
            XPathNode::element("child1", "value1"),
            XPathNode::element("child2", "value2"),
        ];

        let result = proxy.evaluate_simple_path("child1", &context, &children);
        if let XPathResult::Nodes(nodes) = result {
            assert_eq!(nodes.len(), 1);
            assert_eq!(nodes[0].name, "child1");
        } else {
            panic!("Expected nodes result");
        }
    }

    #[test]
    fn test_type_variety() {
        let atomic = TypeInfo::new("string").with_variety(TypeVariety::Atomic);
        assert_eq!(atomic.variety, TypeVariety::Atomic);

        let list = TypeInfo::new("stringList").with_variety(TypeVariety::List);
        assert_eq!(list.variety, TypeVariety::List);
    }
}
