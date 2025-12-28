//! XPath Selectors for XML Schema
//!
//! This module provides XPath selector types and utilities for processing
//! identity constraints (xs:selector, xs:field) in XML Schema.

/// Element selector for identity constraints
#[derive(Debug, Clone)]
pub struct ElementSelector {
    /// The raw XPath expression
    pub xpath: String,
    /// Parsed path steps
    pub steps: Vec<PathStep>,
}

impl ElementSelector {
    /// Create a new element selector from an XPath expression
    pub fn new(xpath: impl Into<String>) -> Self {
        let xpath = xpath.into();
        let steps = split_path(&xpath)
            .into_iter()
            .map(PathStep::parse)
            .collect();
        Self { xpath, steps }
    }

    /// Get the XPath expression
    pub fn xpath(&self) -> &str {
        &self.xpath
    }

    /// Get the parsed steps
    pub fn steps(&self) -> &[PathStep] {
        &self.steps
    }

    /// Check if this selector matches any descendant
    pub fn is_descendant(&self) -> bool {
        self.steps
            .first()
            .map(|s| s.kind == PathStepKind::DescendantOrSelf)
            .unwrap_or(false)
    }
}

/// A single step in an XPath path expression
#[derive(Debug, Clone, PartialEq)]
pub struct PathStep {
    /// The kind of step
    pub kind: PathStepKind,
    /// The local name (may include prefix)
    pub name: String,
    /// Optional namespace prefix
    pub prefix: Option<String>,
    /// Optional predicate
    pub predicate: Option<String>,
}

impl PathStep {
    /// Parse a step from a string
    pub fn parse(step: &str) -> Self {
        let step = step.trim();

        // Check for descendant-or-self axis
        if step == "." || step == "self::node()" {
            return Self {
                kind: PathStepKind::Self_,
                name: String::new(),
                prefix: None,
                predicate: None,
            };
        }

        if step == ".." || step == "parent::node()" {
            return Self {
                kind: PathStepKind::Parent,
                name: String::new(),
                prefix: None,
                predicate: None,
            };
        }

        // Check for attribute axis
        let (kind, rest) = if step.starts_with('@') {
            (PathStepKind::Attribute, &step[1..])
        } else if step.starts_with("attribute::") {
            (PathStepKind::Attribute, &step[11..])
        } else if step.starts_with("child::") {
            (PathStepKind::Child, &step[7..])
        } else {
            (PathStepKind::Child, step)
        };

        // Extract predicate if present
        let (name_part, predicate) = if let Some(bracket_pos) = rest.find('[') {
            let name = &rest[..bracket_pos];
            let pred_end = rest.rfind(']').unwrap_or(rest.len());
            let pred = &rest[bracket_pos + 1..pred_end];
            (name, Some(pred.to_string()))
        } else {
            (rest, None)
        };

        // Extract prefix and local name
        let (prefix, name) = if let Some(colon_pos) = name_part.find(':') {
            (
                Some(name_part[..colon_pos].to_string()),
                name_part[colon_pos + 1..].to_string(),
            )
        } else {
            (None, name_part.to_string())
        };

        Self {
            kind,
            name,
            prefix,
            predicate,
        }
    }

    /// Create a child step
    pub fn child(name: impl Into<String>) -> Self {
        Self {
            kind: PathStepKind::Child,
            name: name.into(),
            prefix: None,
            predicate: None,
        }
    }

    /// Create an attribute step
    pub fn attribute(name: impl Into<String>) -> Self {
        Self {
            kind: PathStepKind::Attribute,
            name: name.into(),
            prefix: None,
            predicate: None,
        }
    }

    /// Set the prefix
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Get the qualified name (prefix:local)
    pub fn qname(&self) -> String {
        if let Some(prefix) = &self.prefix {
            format!("{}:{}", prefix, self.name)
        } else {
            self.name.clone()
        }
    }

    /// Check if this step matches any element (*)
    pub fn is_wildcard(&self) -> bool {
        self.name == "*"
    }
}

/// Kind of path step
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathStepKind {
    /// Child axis (default)
    Child,
    /// Attribute axis (@)
    Attribute,
    /// Descendant-or-self axis (//)
    DescendantOrSelf,
    /// Self axis (.)
    Self_,
    /// Parent axis (..)
    Parent,
}

/// Split an XPath expression into path steps
///
/// Handles both `/` and `//` separators.
pub fn split_path(path: &str) -> Vec<&str> {
    let path = path.trim();

    if path.is_empty() {
        return Vec::new();
    }

    let mut steps = Vec::new();
    let mut current_start = 0;

    // Handle .// (self + descendant-or-self)
    if path.starts_with(".//") {
        steps.push(".");
        steps.push(".//");
        current_start = 3;
    }
    // Handle ./ (self + child)
    else if path.starts_with("./") {
        steps.push(".");
        current_start = 2;
    }
    // Handle single .
    else if path == "." {
        return vec!["."];
    }
    // Handle leading //
    else if path.starts_with("//") {
        steps.push(".//");
        current_start = 2;
    }
    // Handle leading /
    else if path.starts_with('/') {
        current_start = 1;
    }

    let mut in_predicate = 0;
    let bytes = path.as_bytes();
    let len = bytes.len();
    let mut i = current_start;

    while i < len {
        let c = bytes[i] as char;

        match c {
            '[' => {
                in_predicate += 1;
                i += 1;
            }
            ']' => {
                in_predicate -= 1;
                i += 1;
            }
            '/' if in_predicate == 0 => {
                // Check for //
                let is_double = i + 1 < len && bytes[i + 1] == b'/';

                if i > current_start {
                    let step = &path[current_start..i];
                    if !step.is_empty() {
                        steps.push(step);
                    }
                }

                if is_double {
                    steps.push(".//");
                    current_start = i + 2;
                    i += 2;
                } else {
                    current_start = i + 1;
                    i += 1;
                }
            }
            _ => {
                i += 1;
            }
        }
    }

    // Add final step
    if current_start < path.len() {
        let step = &path[current_start..];
        if !step.is_empty() {
            steps.push(step);
        }
    }

    steps
}

/// Check if a string is a valid NCName (non-colonized name)
///
/// NCName is defined in XML Namespaces as a Name that does not contain colons.
pub fn is_ncname(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let mut chars = s.chars();

    // First character must be a letter or underscore
    match chars.next() {
        Some(c) if is_ncname_start_char(c) => {}
        _ => return false,
    }

    // Remaining characters
    chars.all(is_ncname_char)
}

/// Check if a character is valid as the start of an NCName
fn is_ncname_start_char(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

/// Check if a character is valid in an NCName (not at start)
pub fn is_ncname_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-' || c == '.'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_path_simple() {
        assert_eq!(split_path("a/b/c"), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_split_path_with_leading_slash() {
        assert_eq!(split_path("/a/b"), vec!["a", "b"]);
    }

    #[test]
    fn test_split_path_with_dot() {
        assert_eq!(split_path("./a/b"), vec![".", "a", "b"]);
    }

    #[test]
    fn test_split_path_with_descendant() {
        assert_eq!(split_path(".//a/b"), vec![".", ".//", "a", "b"]);
    }

    #[test]
    fn test_split_path_with_predicate() {
        assert_eq!(split_path("a[1]/b"), vec!["a[1]", "b"]);
    }

    #[test]
    fn test_split_path_single() {
        assert_eq!(split_path("."), vec!["."]);
    }

    #[test]
    fn test_path_step_parse_simple() {
        let step = PathStep::parse("element");
        assert_eq!(step.kind, PathStepKind::Child);
        assert_eq!(step.name, "element");
        assert!(step.prefix.is_none());
    }

    #[test]
    fn test_path_step_parse_prefixed() {
        let step = PathStep::parse("ns:element");
        assert_eq!(step.kind, PathStepKind::Child);
        assert_eq!(step.name, "element");
        assert_eq!(step.prefix, Some("ns".to_string()));
    }

    #[test]
    fn test_path_step_parse_attribute() {
        let step = PathStep::parse("@id");
        assert_eq!(step.kind, PathStepKind::Attribute);
        assert_eq!(step.name, "id");
    }

    #[test]
    fn test_path_step_parse_with_predicate() {
        let step = PathStep::parse("element[1]");
        assert_eq!(step.name, "element");
        assert_eq!(step.predicate, Some("1".to_string()));
    }

    #[test]
    fn test_path_step_parse_self() {
        let step = PathStep::parse(".");
        assert_eq!(step.kind, PathStepKind::Self_);
    }

    #[test]
    fn test_path_step_parse_parent() {
        let step = PathStep::parse("..");
        assert_eq!(step.kind, PathStepKind::Parent);
    }

    #[test]
    fn test_element_selector() {
        let selector = ElementSelector::new("./person/name");
        assert_eq!(selector.steps().len(), 3);
    }

    #[test]
    fn test_is_ncname_valid() {
        assert!(is_ncname("element"));
        assert!(is_ncname("_private"));
        assert!(is_ncname("my-element"));
        assert!(is_ncname("element123"));
    }

    #[test]
    fn test_is_ncname_invalid() {
        assert!(!is_ncname("")); // Empty
        assert!(!is_ncname("123start")); // Starts with digit
        assert!(!is_ncname("ns:element")); // Contains colon
        assert!(!is_ncname("-hyphen")); // Starts with hyphen
    }

    #[test]
    fn test_path_step_qname() {
        let step = PathStep::child("element").with_prefix("ns");
        assert_eq!(step.qname(), "ns:element");

        let step2 = PathStep::child("element");
        assert_eq!(step2.qname(), "element");
    }

    #[test]
    fn test_path_step_wildcard() {
        let step = PathStep::child("*");
        assert!(step.is_wildcard());

        let step2 = PathStep::child("element");
        assert!(!step2.is_wildcard());
    }
}
