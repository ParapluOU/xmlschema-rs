//! XSD Wildcard validators
//!
//! This module implements wildcards for XSD element and attribute content:
//! - xs:any - allows any element from specified namespaces
//! - xs:anyAttribute - allows any attribute from specified namespaces
//!
//! Reference: https://www.w3.org/TR/xmlschema11-1/#Wildcards

use crate::error::ParseError;
use crate::namespaces::QName;
use std::collections::HashSet;
use std::sync::Arc;

use super::particles::{Occurs, Particle};

/// Process contents mode for wildcards
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProcessContents {
    /// Validate strictly - element/attribute must be declared
    #[default]
    Strict,
    /// Validate if declaration found, otherwise accept
    Lax,
    /// Skip validation entirely
    Skip,
}

impl ProcessContents {
    /// Parse from string value
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "strict" => Some(Self::Strict),
            "lax" => Some(Self::Lax),
            "skip" => Some(Self::Skip),
            _ => None,
        }
    }

    /// Check if this is a valid restriction of another process contents
    pub fn is_restriction_of(&self, other: &Self) -> bool {
        match (self, other) {
            // Same is always valid
            (a, b) if a == b => true,
            // strict restricts everything
            (Self::Strict, _) => true,
            // lax restricts skip
            (Self::Lax, Self::Skip) => true,
            // skip doesn't restrict strict or lax
            (Self::Skip, Self::Strict | Self::Lax) => false,
            _ => false,
        }
    }
}

impl std::fmt::Display for ProcessContents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Strict => write!(f, "strict"),
            Self::Lax => write!(f, "lax"),
            Self::Skip => write!(f, "skip"),
        }
    }
}

/// Namespace constraint for wildcards
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamespaceConstraint {
    /// Any namespace is allowed (##any)
    Any,
    /// Any namespace except target namespace and no namespace (##other)
    Other {
        /// The target namespace to exclude
        target_namespace: Option<String>,
    },
    /// Specific set of allowed namespaces
    Enumeration(HashSet<String>),
    /// XSD 1.1: Set of disallowed namespaces (notNamespace)
    Not(HashSet<String>),
}

impl Default for NamespaceConstraint {
    fn default() -> Self {
        Self::Any
    }
}

impl NamespaceConstraint {
    /// Create from namespace attribute value
    pub fn from_namespace_attr(
        value: &str,
        target_namespace: Option<&str>,
    ) -> Result<Self, ParseError> {
        let value = value.trim();

        match value {
            "##any" => Ok(Self::Any),
            "##other" => Ok(Self::Other {
                target_namespace: target_namespace.map(String::from),
            }),
            "" => Ok(Self::Enumeration(HashSet::new())),
            _ => {
                let mut namespaces = HashSet::new();
                for ns in value.split_whitespace() {
                    match ns {
                        "##local" => {
                            namespaces.insert(String::new());
                        }
                        "##targetNamespace" => {
                            if let Some(tns) = target_namespace {
                                namespaces.insert(tns.to_string());
                            } else {
                                namespaces.insert(String::new());
                            }
                        }
                        s if s.starts_with("##") => {
                            return Err(ParseError::new(format!(
                                "wrong value '{}' in 'namespace' attribute",
                                s
                            )));
                        }
                        uri => {
                            namespaces.insert(uri.to_string());
                        }
                    }
                }
                Ok(Self::Enumeration(namespaces))
            }
        }
    }

    /// Create from notNamespace attribute (XSD 1.1)
    pub fn from_not_namespace_attr(
        value: &str,
        target_namespace: Option<&str>,
    ) -> Result<Self, ParseError> {
        let mut namespaces = HashSet::new();
        for ns in value.trim().split_whitespace() {
            match ns {
                "##local" => {
                    namespaces.insert(String::new());
                }
                "##targetNamespace" => {
                    if let Some(tns) = target_namespace {
                        namespaces.insert(tns.to_string());
                    } else {
                        namespaces.insert(String::new());
                    }
                }
                s if s.starts_with("##") => {
                    return Err(ParseError::new(format!(
                        "wrong value '{}' in 'notNamespace' attribute",
                        s
                    )));
                }
                uri => {
                    namespaces.insert(uri.to_string());
                }
            }
        }
        Ok(Self::Not(namespaces))
    }

    /// Check if a namespace is allowed by this constraint
    pub fn is_allowed(&self, namespace: &str, target_namespace: Option<&str>) -> bool {
        match self {
            Self::Any => true,
            Self::Other { target_namespace: tns } => {
                // ##other: any namespace except target and no-namespace
                if namespace.is_empty() {
                    return false;
                }
                match (tns, target_namespace) {
                    (Some(tns), _) => namespace != tns,
                    (None, Some(tns)) => namespace != tns,
                    (None, None) => true,
                }
            }
            Self::Enumeration(set) => set.contains(namespace),
            Self::Not(set) => !set.contains(namespace),
        }
    }

    /// Check if this constraint is a valid restriction of another
    pub fn is_restriction_of(&self, other: &Self, target_namespace: Option<&str>) -> bool {
        match (self, other) {
            // Same is always valid
            (a, b) if a == b => true,

            // Anything restricts Any
            (_, Self::Any) => true,

            // Any doesn't restrict anything else
            (Self::Any, _) => false,

            // Enumeration restricts Other if it doesn't include target or empty
            (Self::Enumeration(set), Self::Other { target_namespace: tns }) => {
                let excluded_ns = tns.as_deref().or(target_namespace);
                !set.contains("") && excluded_ns.is_some_and(|ns| !set.contains(ns))
            }

            // Other doesn't restrict Enumeration
            (Self::Other { .. }, Self::Enumeration(_)) => false,

            // Enumeration restricts Enumeration if subset
            (Self::Enumeration(a), Self::Enumeration(b)) => a.is_subset(b),

            // Not constraints (XSD 1.1)
            (Self::Not(a), Self::Not(b)) => b.is_subset(a),
            (Self::Enumeration(set), Self::Not(not_set)) => {
                set.iter().all(|ns| !not_set.contains(ns))
            }
            (Self::Not(_), Self::Enumeration(_)) => false,

            // Other cases
            _ => false,
        }
    }

    /// Compute union with another constraint
    pub fn union(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::Any, _) | (_, Self::Any) => Self::Any,

            (Self::Other { target_namespace: tns1 }, Self::Other { target_namespace: tns2 }) => {
                if tns1 == tns2 {
                    Self::Other {
                        target_namespace: tns1.clone(),
                    }
                } else {
                    Self::Any
                }
            }

            (Self::Enumeration(a), Self::Enumeration(b)) => {
                Self::Enumeration(a.union(b).cloned().collect())
            }

            (Self::Not(a), Self::Not(b)) => Self::Not(a.intersection(b).cloned().collect()),

            // Mixed cases become Any for simplicity
            _ => Self::Any,
        }
    }

    /// Compute intersection with another constraint
    pub fn intersection(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::Any, x) | (x, Self::Any) => x.clone(),

            (Self::Other { target_namespace: tns }, Self::Enumeration(set))
            | (Self::Enumeration(set), Self::Other { target_namespace: tns }) => {
                let excluded = tns.as_ref().map(|s| s.as_str());
                let filtered: HashSet<_> = set
                    .iter()
                    .filter(|ns| !ns.is_empty() && Some(ns.as_str()) != excluded)
                    .cloned()
                    .collect();
                Self::Enumeration(filtered)
            }

            (Self::Enumeration(a), Self::Enumeration(b)) => {
                Self::Enumeration(a.intersection(b).cloned().collect())
            }

            (Self::Not(a), Self::Not(b)) => Self::Not(a.union(b).cloned().collect()),

            (Self::Other { target_namespace: tns1 }, Self::Other { target_namespace: tns2 }) => {
                // Intersection of two ##other
                let mut not_set = HashSet::new();
                not_set.insert(String::new());
                if let Some(tns) = tns1 {
                    not_set.insert(tns.clone());
                }
                if let Some(tns) = tns2 {
                    not_set.insert(tns.clone());
                }
                Self::Not(not_set)
            }

            _ => self.clone(),
        }
    }
}

/// Base wildcard component
#[derive(Debug, Clone)]
pub struct XsdWildcard {
    /// Process contents mode
    pub process_contents: ProcessContents,
    /// Namespace constraint
    pub namespace: NamespaceConstraint,
    /// Disallowed QNames (XSD 1.1)
    pub not_qname: HashSet<QName>,
    /// Target namespace of the schema
    target_namespace: Option<String>,
    /// Parse errors
    errors: Vec<ParseError>,
}

impl XsdWildcard {
    /// Create a new wildcard
    pub fn new(target_namespace: Option<&str>) -> Self {
        Self {
            process_contents: ProcessContents::Strict,
            namespace: NamespaceConstraint::Any,
            not_qname: HashSet::new(),
            target_namespace: target_namespace.map(String::from),
            errors: Vec::new(),
        }
    }

    /// Create with specific namespace constraint
    pub fn with_namespace(
        namespace: NamespaceConstraint,
        process_contents: ProcessContents,
        target_namespace: Option<&str>,
    ) -> Self {
        Self {
            process_contents,
            namespace,
            not_qname: HashSet::new(),
            target_namespace: target_namespace.map(String::from),
            errors: Vec::new(),
        }
    }

    /// Check if a namespace is allowed
    pub fn is_namespace_allowed(&self, namespace: &str) -> bool {
        self.namespace
            .is_allowed(namespace, self.target_namespace.as_deref())
    }

    /// Check if a name matches this wildcard
    pub fn is_matching(&self, name: &str, default_namespace: Option<&str>) -> bool {
        if name.is_empty() {
            return false;
        }

        // Extract namespace from qualified name
        let namespace = if name.starts_with('{') {
            // {namespace}localName format
            if let Some(end) = name.find('}') {
                &name[1..end]
            } else {
                ""
            }
        } else if let Some(ns) = default_namespace {
            ns
        } else {
            ""
        };

        self.is_namespace_allowed(namespace)
    }

    /// Check if this wildcard is a valid restriction of another
    pub fn is_restriction_of(&self, other: &XsdWildcard) -> bool {
        // Process contents must be valid restriction
        if !self.process_contents.is_restriction_of(&other.process_contents) {
            return false;
        }

        // Namespace constraint must be valid restriction
        self.namespace
            .is_restriction_of(&other.namespace, self.target_namespace.as_deref())
    }

    /// Get parse errors
    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }

    /// Add a parse error
    pub fn add_error(&mut self, error: ParseError) {
        self.errors.push(error);
    }
}

/// XSD any element wildcard (xs:any)
#[derive(Debug, Clone)]
pub struct XsdAnyElement {
    /// Base wildcard
    pub wildcard: XsdWildcard,
    /// Occurrence constraints
    occurs: Occurs,
    /// Whether to skip validation for matching elements
    pub skip: bool,
}

impl XsdAnyElement {
    /// Create a new any element wildcard
    pub fn new(target_namespace: Option<&str>) -> Self {
        Self {
            wildcard: XsdWildcard::new(target_namespace),
            occurs: Occurs::once(),
            skip: false,
        }
    }

    /// Create a wildcard that allows any element (##any)
    pub fn any() -> Self {
        Self {
            wildcard: XsdWildcard::with_namespace(
                NamespaceConstraint::Any,
                ProcessContents::Lax,
                None,
            ),
            occurs: Occurs::zero_or_more(),
            skip: false,
        }
    }

    /// Simple tag matching (without namespace qualification)
    pub fn matches_tag(&self, tag: &str) -> bool {
        self.is_matching(tag, None)
    }

    /// Create with specific settings
    pub fn with_settings(
        namespace: NamespaceConstraint,
        process_contents: ProcessContents,
        occurs: Occurs,
        target_namespace: Option<&str>,
    ) -> Self {
        let skip = process_contents == ProcessContents::Skip;
        Self {
            wildcard: XsdWildcard::with_namespace(namespace, process_contents, target_namespace),
            occurs,
            skip,
        }
    }

    /// Check if element name matches
    pub fn is_matching(&self, name: &str, default_namespace: Option<&str>) -> bool {
        self.wildcard.is_matching(name, default_namespace)
    }

    /// Check if this is a valid restriction of another any element
    pub fn is_restriction_of(&self, other: &XsdAnyElement) -> bool {
        // Check occurs restriction
        if !self.occurs.has_occurs_restriction(&other.occurs) {
            return false;
        }

        // Check wildcard restriction
        self.wildcard.is_restriction_of(&other.wildcard)
    }

    /// Get process contents mode
    pub fn process_contents(&self) -> ProcessContents {
        self.wildcard.process_contents
    }
}

impl Particle for XsdAnyElement {
    fn occurs(&self) -> Occurs {
        self.occurs
    }
}

/// XSD any attribute wildcard (xs:anyAttribute)
#[derive(Debug, Clone)]
pub struct XsdAnyAttribute {
    /// Base wildcard
    pub wildcard: XsdWildcard,
}

impl XsdAnyAttribute {
    /// Create a new any attribute wildcard
    pub fn new(target_namespace: Option<&str>) -> Self {
        Self {
            wildcard: XsdWildcard::new(target_namespace),
        }
    }

    /// Create with specific settings
    pub fn with_settings(
        namespace: NamespaceConstraint,
        process_contents: ProcessContents,
        target_namespace: Option<&str>,
    ) -> Self {
        Self {
            wildcard: XsdWildcard::with_namespace(namespace, process_contents, target_namespace),
        }
    }

    /// Check if attribute name matches
    pub fn is_matching(&self, name: &str, default_namespace: Option<&str>) -> bool {
        self.wildcard.is_matching(name, default_namespace)
    }

    /// Check if this is a valid restriction of another any attribute
    pub fn is_restriction_of(&self, other: &XsdAnyAttribute) -> bool {
        self.wildcard.is_restriction_of(&other.wildcard)
    }

    /// Get process contents mode
    pub fn process_contents(&self) -> ProcessContents {
        self.wildcard.process_contents
    }
}

/// Reference to a wildcard (either element or attribute)
#[derive(Debug, Clone)]
pub enum WildcardRef {
    /// Element wildcard (xs:any)
    Element(Arc<XsdAnyElement>),
    /// Attribute wildcard (xs:anyAttribute)
    Attribute(Arc<XsdAnyAttribute>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_contents_from_str() {
        assert_eq!(ProcessContents::from_str("strict"), Some(ProcessContents::Strict));
        assert_eq!(ProcessContents::from_str("lax"), Some(ProcessContents::Lax));
        assert_eq!(ProcessContents::from_str("skip"), Some(ProcessContents::Skip));
        assert_eq!(ProcessContents::from_str("invalid"), None);
    }

    #[test]
    fn test_process_contents_restriction() {
        assert!(ProcessContents::Strict.is_restriction_of(&ProcessContents::Strict));
        assert!(ProcessContents::Strict.is_restriction_of(&ProcessContents::Lax));
        assert!(ProcessContents::Strict.is_restriction_of(&ProcessContents::Skip));

        assert!(!ProcessContents::Lax.is_restriction_of(&ProcessContents::Strict));
        assert!(ProcessContents::Lax.is_restriction_of(&ProcessContents::Lax));
        assert!(ProcessContents::Lax.is_restriction_of(&ProcessContents::Skip));

        assert!(!ProcessContents::Skip.is_restriction_of(&ProcessContents::Strict));
        assert!(!ProcessContents::Skip.is_restriction_of(&ProcessContents::Lax));
        assert!(ProcessContents::Skip.is_restriction_of(&ProcessContents::Skip));
    }

    #[test]
    fn test_namespace_constraint_any() {
        let constraint = NamespaceConstraint::from_namespace_attr("##any", None).unwrap();
        assert_eq!(constraint, NamespaceConstraint::Any);
        assert!(constraint.is_allowed("http://example.com", None));
        assert!(constraint.is_allowed("", None));
    }

    #[test]
    fn test_namespace_constraint_other() {
        let constraint =
            NamespaceConstraint::from_namespace_attr("##other", Some("http://target.com")).unwrap();

        match &constraint {
            NamespaceConstraint::Other { target_namespace } => {
                assert_eq!(target_namespace.as_deref(), Some("http://target.com"));
            }
            _ => panic!("Expected Other constraint"),
        }

        assert!(constraint.is_allowed("http://example.com", Some("http://target.com")));
        assert!(!constraint.is_allowed("http://target.com", Some("http://target.com")));
        assert!(!constraint.is_allowed("", Some("http://target.com")));
    }

    #[test]
    fn test_namespace_constraint_enumeration() {
        let constraint = NamespaceConstraint::from_namespace_attr(
            "http://ns1.com http://ns2.com ##local",
            None,
        )
        .unwrap();

        match &constraint {
            NamespaceConstraint::Enumeration(set) => {
                assert!(set.contains("http://ns1.com"));
                assert!(set.contains("http://ns2.com"));
                assert!(set.contains("")); // ##local
                assert!(!set.contains("http://other.com"));
            }
            _ => panic!("Expected Enumeration constraint"),
        }
    }

    #[test]
    fn test_namespace_constraint_target_namespace() {
        let constraint = NamespaceConstraint::from_namespace_attr(
            "##targetNamespace ##local",
            Some("http://target.com"),
        )
        .unwrap();

        match &constraint {
            NamespaceConstraint::Enumeration(set) => {
                assert!(set.contains("http://target.com"));
                assert!(set.contains(""));
            }
            _ => panic!("Expected Enumeration constraint"),
        }
    }

    #[test]
    fn test_namespace_constraint_not() {
        let constraint = NamespaceConstraint::from_not_namespace_attr(
            "http://excluded.com ##local",
            None,
        )
        .unwrap();

        match &constraint {
            NamespaceConstraint::Not(set) => {
                assert!(set.contains("http://excluded.com"));
                assert!(set.contains(""));
            }
            _ => panic!("Expected Not constraint"),
        }

        assert!(constraint.is_allowed("http://example.com", None));
        assert!(!constraint.is_allowed("http://excluded.com", None));
        assert!(!constraint.is_allowed("", None));
    }

    #[test]
    fn test_namespace_constraint_restriction() {
        let any = NamespaceConstraint::Any;
        let other = NamespaceConstraint::Other {
            target_namespace: Some("http://target.com".to_string()),
        };

        let mut set = HashSet::new();
        set.insert("http://example.com".to_string());
        let enum_constraint = NamespaceConstraint::Enumeration(set);

        // Everything restricts Any
        assert!(enum_constraint.is_restriction_of(&any, None));
        assert!(other.is_restriction_of(&any, None));

        // Any doesn't restrict anything else
        assert!(!any.is_restriction_of(&other, None));
        assert!(!any.is_restriction_of(&enum_constraint, None));

        // Enumeration without target/empty restricts Other
        assert!(enum_constraint.is_restriction_of(&other, Some("http://target.com")));
    }

    #[test]
    fn test_namespace_constraint_union() {
        let mut set1 = HashSet::new();
        set1.insert("http://ns1.com".to_string());
        let enum1 = NamespaceConstraint::Enumeration(set1);

        let mut set2 = HashSet::new();
        set2.insert("http://ns2.com".to_string());
        let enum2 = NamespaceConstraint::Enumeration(set2);

        let union = enum1.union(&enum2);
        match union {
            NamespaceConstraint::Enumeration(set) => {
                assert!(set.contains("http://ns1.com"));
                assert!(set.contains("http://ns2.com"));
            }
            _ => panic!("Expected Enumeration"),
        }
    }

    #[test]
    fn test_namespace_constraint_intersection() {
        let mut set1 = HashSet::new();
        set1.insert("http://ns1.com".to_string());
        set1.insert("http://ns2.com".to_string());
        let enum1 = NamespaceConstraint::Enumeration(set1);

        let mut set2 = HashSet::new();
        set2.insert("http://ns2.com".to_string());
        set2.insert("http://ns3.com".to_string());
        let enum2 = NamespaceConstraint::Enumeration(set2);

        let intersection = enum1.intersection(&enum2);
        match intersection {
            NamespaceConstraint::Enumeration(set) => {
                assert!(!set.contains("http://ns1.com"));
                assert!(set.contains("http://ns2.com"));
                assert!(!set.contains("http://ns3.com"));
            }
            _ => panic!("Expected Enumeration"),
        }
    }

    #[test]
    fn test_wildcard_matching() {
        let wildcard = XsdWildcard::with_namespace(
            NamespaceConstraint::Any,
            ProcessContents::Lax,
            None,
        );

        assert!(wildcard.is_matching("{http://example.com}element", None));
        assert!(wildcard.is_matching("element", Some("http://example.com")));
        assert!(wildcard.is_matching("element", None)); // empty namespace
        assert!(!wildcard.is_matching("", None)); // empty name
    }

    #[test]
    fn test_any_element_creation() {
        let any = XsdAnyElement::with_settings(
            NamespaceConstraint::Any,
            ProcessContents::Skip,
            Occurs::zero_or_more(),
            Some("http://target.com"),
        );

        assert!(any.skip);
        assert_eq!(any.process_contents(), ProcessContents::Skip);
        assert_eq!(any.min_occurs(), 0);
        assert_eq!(any.max_occurs(), None);
    }

    #[test]
    fn test_any_element_restriction() {
        let base = XsdAnyElement::with_settings(
            NamespaceConstraint::Any,
            ProcessContents::Lax,
            Occurs::new(0, Some(5)),
            None,
        );

        // Valid restriction: stricter process, fewer occurrences
        let valid = XsdAnyElement::with_settings(
            NamespaceConstraint::Any,
            ProcessContents::Strict,
            Occurs::new(1, Some(3)),
            None,
        );
        assert!(valid.is_restriction_of(&base));

        // Invalid: less strict process
        let invalid = XsdAnyElement::with_settings(
            NamespaceConstraint::Any,
            ProcessContents::Skip,
            Occurs::new(1, Some(3)),
            None,
        );
        assert!(!invalid.is_restriction_of(&base));
    }

    #[test]
    fn test_any_attribute_creation() {
        let any = XsdAnyAttribute::with_settings(
            NamespaceConstraint::Other {
                target_namespace: Some("http://target.com".to_string()),
            },
            ProcessContents::Strict,
            Some("http://target.com"),
        );

        assert!(any.is_matching("{http://example.com}attr", None));
        assert!(!any.is_matching("{http://target.com}attr", None));
    }
}
