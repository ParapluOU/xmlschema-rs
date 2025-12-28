//! XPath Parsers for XML Schema
//!
//! This module provides XPath parsers for different contexts within XML Schema:
//! - Identity constraints (xs:selector, xs:field) use a restricted XPath subset
//! - XSD 1.1 assertions use XPath 2.0

use std::fmt;

use super::selectors::{is_ncname, PathStepKind};

/// XPath axis types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XPathAxis {
    /// child:: axis (default)
    Child,
    /// descendant:: axis
    Descendant,
    /// descendant-or-self:: axis
    DescendantOrSelf,
    /// self:: axis
    Self_,
    /// parent:: axis
    Parent,
    /// ancestor:: axis
    Ancestor,
    /// ancestor-or-self:: axis
    AncestorOrSelf,
    /// following-sibling:: axis
    FollowingSibling,
    /// preceding-sibling:: axis
    PrecedingSibling,
    /// following:: axis
    Following,
    /// preceding:: axis
    Preceding,
    /// attribute:: axis
    Attribute,
    /// namespace:: axis
    Namespace,
}

impl XPathAxis {
    /// Parse axis from string
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "child" => Some(Self::Child),
            "descendant" => Some(Self::Descendant),
            "descendant-or-self" => Some(Self::DescendantOrSelf),
            "self" => Some(Self::Self_),
            "parent" => Some(Self::Parent),
            "ancestor" => Some(Self::Ancestor),
            "ancestor-or-self" => Some(Self::AncestorOrSelf),
            "following-sibling" => Some(Self::FollowingSibling),
            "preceding-sibling" => Some(Self::PrecedingSibling),
            "following" => Some(Self::Following),
            "preceding" => Some(Self::Preceding),
            "attribute" => Some(Self::Attribute),
            "namespace" => Some(Self::Namespace),
            _ => None,
        }
    }

    /// Check if this axis is forward (selects nodes after context in document order)
    pub fn is_forward(&self) -> bool {
        matches!(
            self,
            Self::Child
                | Self::Descendant
                | Self::DescendantOrSelf
                | Self::Self_
                | Self::Following
                | Self::FollowingSibling
                | Self::Attribute
                | Self::Namespace
        )
    }

    /// Check if this axis is reverse
    pub fn is_reverse(&self) -> bool {
        !self.is_forward()
    }
}

impl fmt::Display for XPathAxis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Child => "child",
            Self::Descendant => "descendant",
            Self::DescendantOrSelf => "descendant-or-self",
            Self::Self_ => "self",
            Self::Parent => "parent",
            Self::Ancestor => "ancestor",
            Self::AncestorOrSelf => "ancestor-or-self",
            Self::FollowingSibling => "following-sibling",
            Self::PrecedingSibling => "preceding-sibling",
            Self::Following => "following",
            Self::Preceding => "preceding",
            Self::Attribute => "attribute",
            Self::Namespace => "namespace",
        };
        write!(f, "{}", s)
    }
}

/// XPath predicate
#[derive(Debug, Clone, PartialEq)]
pub struct XPathPredicate {
    /// The raw predicate expression
    pub expression: String,
    /// Predicate kind
    pub kind: PredicateKind,
}

impl XPathPredicate {
    /// Create a new predicate
    pub fn new(expression: impl Into<String>) -> Self {
        let expression = expression.into();
        let kind = Self::classify(&expression);
        Self { expression, kind }
    }

    /// Classify the predicate kind
    fn classify(expr: &str) -> PredicateKind {
        let trimmed = expr.trim();

        // Check for numeric predicate
        if trimmed.parse::<i64>().is_ok() {
            return PredicateKind::Position;
        }

        // Check for position() function
        if trimmed.contains("position()") || trimmed.contains("last()") {
            return PredicateKind::Position;
        }

        // Check for comparison operators
        if trimmed.contains('=')
            || trimmed.contains('<')
            || trimmed.contains('>')
            || trimmed.contains("!=")
        {
            return PredicateKind::Comparison;
        }

        // Check for function calls
        if trimmed.contains('(') && trimmed.contains(')') {
            return PredicateKind::Function;
        }

        PredicateKind::NodeTest
    }

    /// Check if this is a positional predicate
    pub fn is_positional(&self) -> bool {
        self.kind == PredicateKind::Position
    }

    /// Get the position value if this is a numeric positional predicate
    pub fn position_value(&self) -> Option<i64> {
        self.expression.trim().parse().ok()
    }
}

/// Kind of XPath predicate
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredicateKind {
    /// Positional predicate (e.g., [1], [last()])
    Position,
    /// Comparison predicate (e.g., [@id='x'])
    Comparison,
    /// Function call predicate
    Function,
    /// Node test predicate
    NodeTest,
}

/// Parsed XPath expression
#[derive(Debug, Clone)]
pub struct ParsedXPath {
    /// Original expression
    pub expression: String,
    /// Parsed steps
    pub steps: Vec<ParsedStep>,
    /// Whether this is an absolute path
    pub is_absolute: bool,
}

impl ParsedXPath {
    /// Create from an expression string
    pub fn parse(expression: impl Into<String>) -> Result<Self, XPathParseError> {
        let expression = expression.into();
        let (is_absolute, path) = if expression.starts_with('/') {
            (true, expression[1..].to_string())
        } else {
            (false, expression.clone())
        };

        let steps = Self::parse_steps(&path)?;

        Ok(Self {
            expression,
            steps,
            is_absolute,
        })
    }

    fn parse_steps(path: &str) -> Result<Vec<ParsedStep>, XPathParseError> {
        if path.is_empty() {
            return Ok(Vec::new());
        }

        let mut steps = Vec::new();
        let mut current = String::new();
        let mut bracket_depth = 0;
        let mut chars = path.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '[' => {
                    bracket_depth += 1;
                    current.push(c);
                }
                ']' => {
                    bracket_depth -= 1;
                    current.push(c);
                }
                '/' if bracket_depth == 0 => {
                    if !current.is_empty() {
                        steps.push(ParsedStep::parse(&current)?);
                        current.clear();
                    }
                    // Check for //
                    if chars.peek() == Some(&'/') {
                        chars.next();
                        steps.push(ParsedStep {
                            axis: XPathAxis::DescendantOrSelf,
                            node_test: NodeTest::Node,
                            predicates: Vec::new(),
                        });
                    }
                }
                _ => current.push(c),
            }
        }

        if !current.is_empty() {
            steps.push(ParsedStep::parse(&current)?);
        }

        Ok(steps)
    }

    /// Get the number of steps
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Check if this path starts with descendant-or-self
    pub fn is_descendant_search(&self) -> bool {
        // Check first step
        if self
            .steps
            .first()
            .map(|s| s.axis == XPathAxis::DescendantOrSelf)
            .unwrap_or(false)
        {
            return true;
        }
        // Check second step if first is self (for .//)
        if self.steps.len() >= 2
            && self.steps[0].axis == XPathAxis::Self_
            && self.steps[1].axis == XPathAxis::DescendantOrSelf
        {
            return true;
        }
        false
    }
}

/// A parsed step in an XPath expression
#[derive(Debug, Clone)]
pub struct ParsedStep {
    /// The axis
    pub axis: XPathAxis,
    /// The node test
    pub node_test: NodeTest,
    /// Predicates
    pub predicates: Vec<XPathPredicate>,
}

impl ParsedStep {
    /// Parse a step from a string
    pub fn parse(step: &str) -> Result<Self, XPathParseError> {
        let step = step.trim();

        // Handle abbreviations
        if step == "." {
            return Ok(Self {
                axis: XPathAxis::Self_,
                node_test: NodeTest::Node,
                predicates: Vec::new(),
            });
        }

        if step == ".." {
            return Ok(Self {
                axis: XPathAxis::Parent,
                node_test: NodeTest::Node,
                predicates: Vec::new(),
            });
        }

        // Extract predicates
        let (name_part, predicates) = Self::extract_predicates(step)?;

        // Check for axis specifier
        let (axis, node_test_str) = if name_part.starts_with('@') {
            (XPathAxis::Attribute, &name_part[1..])
        } else if let Some(pos) = name_part.find("::") {
            let axis_str = &name_part[..pos];
            let axis = XPathAxis::parse(axis_str)
                .ok_or_else(|| XPathParseError::UnknownAxis(axis_str.to_string()))?;
            (axis, &name_part[pos + 2..])
        } else {
            (XPathAxis::Child, name_part.as_str())
        };

        let node_test = NodeTest::parse(node_test_str)?;

        Ok(Self {
            axis,
            node_test,
            predicates,
        })
    }

    fn extract_predicates(step: &str) -> Result<(String, Vec<XPathPredicate>), XPathParseError> {
        let mut predicates = Vec::new();
        let mut name_end = step.len();

        if let Some(first_bracket) = step.find('[') {
            name_end = first_bracket;

            let pred_part = &step[first_bracket..];
            let mut current_pred = String::new();
            let mut depth = 0;

            for c in pred_part.chars() {
                match c {
                    '[' => {
                        if depth > 0 {
                            current_pred.push(c);
                        }
                        depth += 1;
                    }
                    ']' => {
                        depth -= 1;
                        if depth == 0 {
                            predicates.push(XPathPredicate::new(current_pred.clone()));
                            current_pred.clear();
                        } else {
                            current_pred.push(c);
                        }
                    }
                    _ if depth > 0 => current_pred.push(c),
                    _ => {}
                }
            }
        }

        Ok((step[..name_end].to_string(), predicates))
    }

    /// Convert to PathStepKind
    pub fn to_path_step_kind(&self) -> PathStepKind {
        match self.axis {
            XPathAxis::Attribute => PathStepKind::Attribute,
            XPathAxis::Self_ => PathStepKind::Self_,
            XPathAxis::Parent => PathStepKind::Parent,
            XPathAxis::DescendantOrSelf => PathStepKind::DescendantOrSelf,
            _ => PathStepKind::Child,
        }
    }
}

/// Node test in an XPath step
#[derive(Debug, Clone, PartialEq)]
pub enum NodeTest {
    /// Name test (element or attribute name)
    Name {
        /// Namespace prefix
        prefix: Option<String>,
        /// Local name
        local: String,
    },
    /// Wildcard test (*)
    Wildcard,
    /// Namespace wildcard (prefix:*)
    NamespaceWildcard(String),
    /// node() test
    Node,
    /// text() test
    Text,
    /// comment() test
    Comment,
    /// processing-instruction() test
    ProcessingInstruction(Option<String>),
}

impl NodeTest {
    /// Parse a node test from a string
    pub fn parse(s: &str) -> Result<Self, XPathParseError> {
        let s = s.trim();

        if s == "*" {
            return Ok(Self::Wildcard);
        }

        if s == "node()" {
            return Ok(Self::Node);
        }

        if s == "text()" {
            return Ok(Self::Text);
        }

        if s == "comment()" {
            return Ok(Self::Comment);
        }

        if s.starts_with("processing-instruction(") {
            let inner = s
                .strip_prefix("processing-instruction(")
                .and_then(|s| s.strip_suffix(')'))
                .unwrap_or("");
            let target = if inner.is_empty() {
                None
            } else {
                // Strip quotes
                let target = inner.trim_matches(|c| c == '\'' || c == '"');
                Some(target.to_string())
            };
            return Ok(Self::ProcessingInstruction(target));
        }

        // Check for namespace wildcard (prefix:*)
        if s.ends_with(":*") {
            let prefix = &s[..s.len() - 2];
            return Ok(Self::NamespaceWildcard(prefix.to_string()));
        }

        // Name test
        let (prefix, local) = if let Some(colon_pos) = s.find(':') {
            let prefix = &s[..colon_pos];
            let local = &s[colon_pos + 1..];
            (Some(prefix.to_string()), local.to_string())
        } else {
            (None, s.to_string())
        };

        Ok(Self::Name { prefix, local })
    }

    /// Check if this test matches any node
    pub fn matches_any(&self) -> bool {
        matches!(self, Self::Wildcard | Self::Node)
    }

    /// Get the local name if this is a name test
    pub fn local_name(&self) -> Option<&str> {
        match self {
            Self::Name { local, .. } => Some(local),
            _ => None,
        }
    }
}

/// XPath parse error
#[derive(Debug, Clone)]
pub enum XPathParseError {
    /// Unknown axis name
    UnknownAxis(String),
    /// Invalid syntax
    InvalidSyntax(String),
    /// Unexpected end of expression
    UnexpectedEnd,
}

impl fmt::Display for XPathParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownAxis(axis) => write!(f, "Unknown XPath axis: {}", axis),
            Self::InvalidSyntax(msg) => write!(f, "Invalid XPath syntax: {}", msg),
            Self::UnexpectedEnd => write!(f, "Unexpected end of XPath expression"),
        }
    }
}

impl std::error::Error for XPathParseError {}

/// Parser for identity constraint XPath (xs:selector, xs:field)
///
/// Identity constraints use a restricted subset of XPath.
#[derive(Debug, Clone)]
pub struct IdentityXPathParser {
    /// Whether to allow attribute axis
    allow_attributes: bool,
    /// Whether to allow descendant axis
    allow_descendant: bool,
}

impl Default for IdentityXPathParser {
    fn default() -> Self {
        Self::new()
    }
}

impl IdentityXPathParser {
    /// Create a new parser for selector expressions
    pub fn new() -> Self {
        Self {
            allow_attributes: false,
            allow_descendant: true,
        }
    }

    /// Create a parser for field expressions (allows attributes)
    pub fn for_field() -> Self {
        Self {
            allow_attributes: true,
            allow_descendant: true,
        }
    }

    /// Parse an identity constraint XPath expression
    pub fn parse(&self, xpath: &str) -> Result<ParsedXPath, XPathParseError> {
        let parsed = ParsedXPath::parse(xpath)?;

        // Validate the expression
        self.validate(&parsed)?;

        Ok(parsed)
    }

    fn validate(&self, parsed: &ParsedXPath) -> Result<(), XPathParseError> {
        for step in &parsed.steps {
            // Check axis restrictions
            match step.axis {
                XPathAxis::Attribute if !self.allow_attributes => {
                    return Err(XPathParseError::InvalidSyntax(
                        "Attribute axis not allowed in selector".to_string(),
                    ));
                }
                XPathAxis::Descendant | XPathAxis::DescendantOrSelf if !self.allow_descendant => {
                    return Err(XPathParseError::InvalidSyntax(
                        "Descendant axis not allowed".to_string(),
                    ));
                }
                XPathAxis::Ancestor
                | XPathAxis::AncestorOrSelf
                | XPathAxis::Following
                | XPathAxis::FollowingSibling
                | XPathAxis::Preceding
                | XPathAxis::PrecedingSibling => {
                    return Err(XPathParseError::InvalidSyntax(format!(
                        "Axis {} not allowed in identity constraint",
                        step.axis
                    )));
                }
                _ => {}
            }

            // Check node test (only names and wildcards allowed, except for . and ..)
            match &step.node_test {
                NodeTest::Name { local, .. } => {
                    // Validate the name is an NCName
                    if !is_ncname(local) && local != "*" {
                        return Err(XPathParseError::InvalidSyntax(format!(
                            "Invalid NCName: {}",
                            local
                        )));
                    }
                }
                NodeTest::Wildcard | NodeTest::NamespaceWildcard(_) => {}
                // node() is allowed for self and parent axes (. and ..)
                NodeTest::Node
                    if step.axis == XPathAxis::Self_
                        || step.axis == XPathAxis::Parent
                        || step.axis == XPathAxis::DescendantOrSelf => {}
                _ => {
                    return Err(XPathParseError::InvalidSyntax(
                        "Only name tests allowed in identity constraint".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }
}

/// Parser for XSD 1.1 assertion XPath expressions
///
/// Assertions use XPath 2.0, which is more expressive than the identity constraint subset.
#[derive(Debug, Clone)]
pub struct AssertionXPathParser {
    /// Default namespace for unprefixed element names
    default_element_namespace: Option<String>,
    /// Whether to allow extension functions
    allow_extensions: bool,
}

impl Default for AssertionXPathParser {
    fn default() -> Self {
        Self::new()
    }
}

impl AssertionXPathParser {
    /// Create a new assertion XPath parser
    pub fn new() -> Self {
        Self {
            default_element_namespace: None,
            allow_extensions: false,
        }
    }

    /// Set the default element namespace
    pub fn with_default_namespace(mut self, ns: impl Into<String>) -> Self {
        self.default_element_namespace = Some(ns.into());
        self
    }

    /// Allow extension functions
    pub fn with_extensions(mut self, allow: bool) -> Self {
        self.allow_extensions = allow;
        self
    }

    /// Parse an assertion XPath expression
    pub fn parse(&self, xpath: &str) -> Result<ParsedXPath, XPathParseError> {
        // For now, use the basic parser
        // Full XPath 2.0 would require a more sophisticated parser
        ParsedXPath::parse(xpath)
    }

    /// Get the default element namespace
    pub fn default_namespace(&self) -> Option<&String> {
        self.default_element_namespace.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xpath_axis_parse() {
        assert_eq!(XPathAxis::parse("child"), Some(XPathAxis::Child));
        assert_eq!(XPathAxis::parse("attribute"), Some(XPathAxis::Attribute));
        assert_eq!(
            XPathAxis::parse("descendant-or-self"),
            Some(XPathAxis::DescendantOrSelf)
        );
        assert_eq!(XPathAxis::parse("invalid"), None);
    }

    #[test]
    fn test_xpath_axis_forward_reverse() {
        assert!(XPathAxis::Child.is_forward());
        assert!(!XPathAxis::Child.is_reverse());
        assert!(XPathAxis::Parent.is_reverse());
        assert!(!XPathAxis::Parent.is_forward());
    }

    #[test]
    fn test_xpath_predicate() {
        let pred = XPathPredicate::new("1");
        assert!(pred.is_positional());
        assert_eq!(pred.position_value(), Some(1));

        let pred2 = XPathPredicate::new("@id='test'");
        assert!(!pred2.is_positional());
        assert_eq!(pred2.kind, PredicateKind::Comparison);
    }

    #[test]
    fn test_node_test_parse() {
        let test = NodeTest::parse("element").unwrap();
        assert_eq!(
            test,
            NodeTest::Name {
                prefix: None,
                local: "element".to_string()
            }
        );

        let test2 = NodeTest::parse("ns:element").unwrap();
        assert_eq!(
            test2,
            NodeTest::Name {
                prefix: Some("ns".to_string()),
                local: "element".to_string()
            }
        );

        let test3 = NodeTest::parse("*").unwrap();
        assert_eq!(test3, NodeTest::Wildcard);

        let test4 = NodeTest::parse("node()").unwrap();
        assert_eq!(test4, NodeTest::Node);
    }

    #[test]
    fn test_parsed_xpath_simple() {
        let parsed = ParsedXPath::parse("a/b/c").unwrap();
        assert_eq!(parsed.step_count(), 3);
        assert!(!parsed.is_absolute);
    }

    #[test]
    fn test_parsed_xpath_absolute() {
        let parsed = ParsedXPath::parse("/root/child").unwrap();
        assert!(parsed.is_absolute);
        assert_eq!(parsed.step_count(), 2);
    }

    #[test]
    fn test_parsed_xpath_with_predicates() {
        let parsed = ParsedXPath::parse("a[1]/b[@id='x']").unwrap();
        assert_eq!(parsed.step_count(), 2);
        assert_eq!(parsed.steps[0].predicates.len(), 1);
        assert_eq!(parsed.steps[1].predicates.len(), 1);
    }

    #[test]
    fn test_parsed_xpath_descendant() {
        let parsed = ParsedXPath::parse(".//element").unwrap();
        assert!(parsed.is_descendant_search());
    }

    #[test]
    fn test_parsed_step_dot() {
        let step = ParsedStep::parse(".").unwrap();
        assert_eq!(step.axis, XPathAxis::Self_);
    }

    #[test]
    fn test_parsed_step_double_dot() {
        let step = ParsedStep::parse("..").unwrap();
        assert_eq!(step.axis, XPathAxis::Parent);
    }

    #[test]
    fn test_parsed_step_attribute() {
        let step = ParsedStep::parse("@id").unwrap();
        assert_eq!(step.axis, XPathAxis::Attribute);
        assert_eq!(step.node_test.local_name(), Some("id"));
    }

    #[test]
    fn test_identity_parser_selector() {
        let parser = IdentityXPathParser::new();
        let result = parser.parse("./person/name");
        assert!(result.is_ok());
    }

    #[test]
    fn test_identity_parser_field() {
        let parser = IdentityXPathParser::for_field();
        let result = parser.parse("@id");
        assert!(result.is_ok());
    }

    #[test]
    fn test_identity_parser_rejects_attribute_in_selector() {
        let parser = IdentityXPathParser::new();
        let result = parser.parse("@id");
        assert!(result.is_err());
    }

    #[test]
    fn test_assertion_parser() {
        let parser = AssertionXPathParser::new()
            .with_default_namespace("http://example.com");

        assert_eq!(
            parser.default_namespace(),
            Some(&"http://example.com".to_string())
        );

        let result = parser.parse("@value > 0");
        assert!(result.is_ok());
    }
}
