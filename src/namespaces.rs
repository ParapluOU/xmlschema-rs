//! XML namespace handling
//!
//! This module provides utilities for working with XML namespaces,
//! qualified names (QNames), and namespace prefix mappings.

use crate::error::{Error, Result};
use std::collections::HashMap;

/// XML Namespace URI
pub type NamespaceUri = String;

/// Namespace prefix
pub type Prefix = String;

/// The namespace the `xml` prefix is always bound to, declared or not
/// (XML Namespaces, section 3).
pub const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";

/// Qualified name (QName) - combination of namespace and local name
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QName {
    /// Namespace URI (None for no namespace)
    pub namespace: Option<NamespaceUri>,
    /// Local name
    pub local_name: String,
}

impl QName {
    /// Create a new QName
    pub fn new(namespace: Option<impl Into<String>>, local_name: impl Into<String>) -> Self {
        Self {
            namespace: namespace.map(|s| s.into()),
            local_name: local_name.into(),
        }
    }

    /// Create a QName without a namespace
    pub fn local(local_name: impl Into<String>) -> Self {
        Self {
            namespace: None,
            local_name: local_name.into(),
        }
    }

    /// Create a QName with a namespace
    pub fn namespaced(namespace: impl Into<String>, local_name: impl Into<String>) -> Self {
        Self {
            namespace: Some(namespace.into()),
            local_name: local_name.into(),
        }
    }

    /// Get the fully qualified name as a string
    pub fn to_string(&self) -> String {
        match &self.namespace {
            Some(ns) => format!("{{{}}}{}", ns, self.local_name),
            None => self.local_name.clone(),
        }
    }
}

/// Namespace context for resolving prefixes
#[derive(Debug, Clone)]
pub struct NamespaceContext {
    /// Mapping from prefix to namespace URI
    prefixes: HashMap<Prefix, NamespaceUri>,
    /// Default namespace (no prefix)
    default_namespace: Option<NamespaceUri>,
}

impl NamespaceContext {
    /// Create a new empty namespace context
    pub fn new() -> Self {
        Self {
            prefixes: HashMap::new(),
            default_namespace: None,
        }
    }

    /// Add a namespace prefix mapping
    pub fn add_prefix(&mut self, prefix: impl Into<String>, namespace: impl Into<String>) {
        self.prefixes.insert(prefix.into(), namespace.into());
    }

    /// Set the default namespace
    pub fn set_default_namespace(&mut self, namespace: impl Into<String>) {
        self.default_namespace = Some(namespace.into());
    }

    /// Get the namespace for a prefix
    pub fn get_namespace(&self, prefix: &str) -> Option<&str> {
        if let Some(found) = self.prefixes.get(prefix) {
            return Some(found.as_str());
        }
        // `xml` is bound by the XML specification itself and is almost never
        // declared: a document may use `xml:base`, `xml:lang` or `xml:space`
        // without saying anything. Without this an `xml:…` attribute keeps the
        // prefix in its local name and is invisible to anything asking for it
        // by name — which is how an OASIS catalog's `xml:base` came to be
        // ignored, and with it every URN mapping in DITA.
        if prefix == "xml" {
            return Some(XML_NAMESPACE);
        }
        None
    }

    /// Get the default namespace
    pub fn get_default_namespace(&self) -> Option<&str> {
        self.default_namespace.as_deref()
    }

    /// Iterate over all prefix-namespace pairs
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.prefixes.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Resolve a prefixed name to a QName
    pub fn resolve(&self, prefixed_name: &str) -> Result<QName> {
        if let Some((prefix, local)) = prefixed_name.split_once(':') {
            let namespace = self
                .get_namespace(prefix)
                .ok_or_else(|| Error::Namespace(format!("Unknown prefix: {}", prefix)))?;
            Ok(QName::namespaced(namespace, local))
        } else {
            Ok(QName::new(self.default_namespace.clone(), prefixed_name))
        }
    }
}

impl Default for NamespaceContext {
    fn default() -> Self {
        Self::new()
    }
}

// TODO: Implement more namespace utilities
// - Namespace inheritance
// - Namespace merging
// - Well-known namespace constants

#[cfg(test)]
mod tests {
    use super::*;

    /// The `xml` prefix is bound by the specification, declared or not: a
    /// document may say `xml:base`, `xml:lang` or `xml:space` without
    /// declaring anything. Without this such an attribute keeps its prefix in
    /// its local name and is invisible to anything asking for it by name.
    #[test]
    fn the_xml_prefix_is_bound_without_being_declared() {
        let namespaces = NamespaceContext::new();
        assert_eq!(namespaces.get_namespace("xml"), Some(XML_NAMESPACE));
        assert_eq!(namespaces.get_namespace("nope"), None);
    }

    #[test]
    fn test_qname_creation() {
        let qname = QName::namespaced("http://example.com", "element");
        assert_eq!(qname.namespace, Some("http://example.com".to_string()));
        assert_eq!(qname.local_name, "element");
    }

    #[test]
    fn test_qname_to_string() {
        let qname = QName::namespaced("http://example.com", "element");
        assert_eq!(qname.to_string(), "{http://example.com}element");

        let qname_local = QName::local("element");
        assert_eq!(qname_local.to_string(), "element");
    }

    #[test]
    fn test_namespace_context() {
        let mut ctx = NamespaceContext::new();
        ctx.add_prefix("xs", "http://www.w3.org/2001/XMLSchema");
        ctx.set_default_namespace("http://example.com");

        assert_eq!(
            ctx.get_namespace("xs"),
            Some("http://www.w3.org/2001/XMLSchema")
        );
        assert_eq!(ctx.get_default_namespace(), Some("http://example.com"));
    }

    #[test]
    fn test_resolve_prefixed_name() {
        let mut ctx = NamespaceContext::new();
        ctx.add_prefix("xs", "http://www.w3.org/2001/XMLSchema");

        let qname = ctx.resolve("xs:element").unwrap();
        assert_eq!(
            qname.namespace,
            Some("http://www.w3.org/2001/XMLSchema".to_string())
        );
        assert_eq!(qname.local_name, "element");
    }
}
