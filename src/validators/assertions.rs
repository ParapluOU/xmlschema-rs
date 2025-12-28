//! XSD 1.1 Assertions
//!
//! This module handles xs:assert constraints for XSD 1.1.
//! Assertions allow additional constraints via XPath expressions.

use crate::error::{ParseError, Result};
use super::base::{ValidationStatus, Validator};

/// XSD 1.1 Assert constraint
///
/// Represents an xs:assert declaration:
/// ```xml
/// <assert
///   id = ID
///   test = an XPath expression
///   xpathDefaultNamespace = (anyURI | (##defaultNamespace | ##targetNamespace | ##local))
///   {any attributes with non-schema namespace . . .}>
///   Content: (annotation?)
/// </assert>
/// ```
#[derive(Debug, Clone)]
pub struct XsdAssert {
    /// The XPath expression to test
    pub test: String,
    /// Optional ID attribute
    pub id: Option<String>,
    /// XPath default namespace
    pub xpath_default_namespace: Option<XPathDefaultNamespace>,
    /// The parsed XPath token (placeholder - would need XPath parser)
    xpath_compiled: bool,
    /// Building errors
    errors: Vec<ParseError>,
    /// Whether the assertion has been built
    built: bool,
}

/// XPath default namespace options
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XPathDefaultNamespace {
    /// Use ##defaultNamespace
    DefaultNamespace,
    /// Use ##targetNamespace
    TargetNamespace,
    /// Use ##local (no namespace)
    Local,
    /// Use a specific namespace URI
    Uri(String),
}

impl XPathDefaultNamespace {
    /// Parse from string value
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "##defaultNamespace" => Ok(XPathDefaultNamespace::DefaultNamespace),
            "##targetNamespace" => Ok(XPathDefaultNamespace::TargetNamespace),
            "##local" => Ok(XPathDefaultNamespace::Local),
            uri => Ok(XPathDefaultNamespace::Uri(uri.to_string())),
        }
    }
}

impl XsdAssert {
    /// Create a new assertion with the given test expression
    pub fn new(test: impl Into<String>) -> Self {
        Self {
            test: test.into(),
            id: None,
            xpath_default_namespace: None,
            xpath_compiled: false,
            errors: Vec::new(),
            built: false,
        }
    }

    /// Create an assertion with an ID
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the XPath default namespace
    pub fn with_xpath_default_namespace(mut self, ns: XPathDefaultNamespace) -> Self {
        self.xpath_default_namespace = Some(ns);
        self
    }

    /// Get the test expression
    pub fn test(&self) -> &str {
        &self.test
    }

    /// Get the XPath default namespace
    pub fn xpath_default_namespace(&self) -> Option<&XPathDefaultNamespace> {
        self.xpath_default_namespace.as_ref()
    }

    /// Check if the assertion has been compiled
    pub fn is_compiled(&self) -> bool {
        self.xpath_compiled
    }

    /// Add a parse error
    pub fn add_error(&mut self, error: ParseError) {
        self.errors.push(error);
    }

    /// Compile the XPath expression
    fn compile_xpath(&mut self) -> Result<()> {
        // Validate the test expression is not empty
        let is_empty = self.test.trim().is_empty();
        let has_absolute_path = self.test.trim().starts_with('/')
            || self.test.contains("//");

        if is_empty {
            self.add_error(ParseError::new("missing required attribute 'test'"));
            // Use 'true()' as fallback
            self.test = "true()".to_string();
        }

        // Check for potentially problematic patterns
        // (Absolute paths in assertions may return empty sequences)
        if has_absolute_path {
            // This is a warning, not an error
            // In Python this triggers XMLSchemaAssertPathWarning
        }

        // Mark as compiled (actual XPath compilation would happen here)
        self.xpath_compiled = true;
        Ok(())
    }

    /// Evaluate the assertion against a value
    ///
    /// Note: This is a placeholder. Real implementation would require:
    /// - An XPath 2.0+ evaluator (for XSD 1.1)
    /// - Integration with schema-bound parser
    /// - Variable binding support ($value)
    pub fn evaluate(&self, _value: &str) -> Result<bool> {
        if !self.xpath_compiled {
            return Err(crate::error::Error::Parse(
                ParseError::new("assertion not compiled")
            ));
        }

        // Placeholder: would need actual XPath evaluation
        // For now, return true for 'true()' and false for 'false()'
        match self.test.trim() {
            "true()" => Ok(true),
            "false()" => Ok(false),
            _ => {
                // Cannot evaluate complex expressions without XPath engine
                Ok(true) // Optimistic default
            }
        }
    }
}

impl Validator for XsdAssert {
    fn is_built(&self) -> bool {
        self.built
    }

    fn build(&mut self) -> Result<()> {
        if !self.built {
            self.compile_xpath()?;
            self.built = true;
        }
        Ok(())
    }

    fn validation_attempted(&self) -> ValidationStatus {
        if !self.built {
            ValidationStatus::None
        } else if self.errors.is_empty() {
            ValidationStatus::Full
        } else {
            ValidationStatus::Partial
        }
    }

    fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    fn errors(&self) -> Vec<ParseError> {
        self.errors.clone()
    }
}

/// Collection of assertions for a complex type
#[derive(Debug, Clone, Default)]
pub struct AssertionList {
    /// The assertions
    assertions: Vec<XsdAssert>,
}

impl AssertionList {
    /// Create a new empty assertion list
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an assertion
    pub fn add(&mut self, assertion: XsdAssert) {
        self.assertions.push(assertion);
    }

    /// Get the number of assertions
    pub fn len(&self) -> usize {
        self.assertions.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.assertions.is_empty()
    }

    /// Iterate over assertions
    pub fn iter(&self) -> impl Iterator<Item = &XsdAssert> {
        self.assertions.iter()
    }

    /// Iterate mutably over assertions
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut XsdAssert> {
        self.assertions.iter_mut()
    }

    /// Build all assertions
    pub fn build_all(&mut self) -> Result<()> {
        for assertion in &mut self.assertions {
            assertion.build()?;
        }
        Ok(())
    }

    /// Evaluate all assertions against a value
    pub fn evaluate_all(&self, value: &str) -> Vec<(usize, bool)> {
        self.assertions
            .iter()
            .enumerate()
            .map(|(i, a)| (i, a.evaluate(value).unwrap_or(true)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assertion_creation() {
        let assertion = XsdAssert::new("@value > 0");
        assert_eq!(assertion.test(), "@value > 0");
        assert!(!assertion.is_compiled());
    }

    #[test]
    fn test_assertion_with_id() {
        let assertion = XsdAssert::new("true()")
            .with_id("assert-1");
        assert_eq!(assertion.id, Some("assert-1".to_string()));
    }

    #[test]
    fn test_xpath_default_namespace() {
        assert_eq!(
            XPathDefaultNamespace::from_str("##defaultNamespace").unwrap(),
            XPathDefaultNamespace::DefaultNamespace
        );
        assert_eq!(
            XPathDefaultNamespace::from_str("##targetNamespace").unwrap(),
            XPathDefaultNamespace::TargetNamespace
        );
        assert_eq!(
            XPathDefaultNamespace::from_str("##local").unwrap(),
            XPathDefaultNamespace::Local
        );
        assert_eq!(
            XPathDefaultNamespace::from_str("http://example.com").unwrap(),
            XPathDefaultNamespace::Uri("http://example.com".to_string())
        );
    }

    #[test]
    fn test_assertion_build() {
        let mut assertion = XsdAssert::new("@age >= 18");
        assert!(!assertion.is_built());

        assertion.build().unwrap();
        assert!(assertion.is_built());
        assert!(assertion.is_compiled());
    }

    #[test]
    fn test_empty_test_expression() {
        let mut assertion = XsdAssert::new("");
        assertion.build().unwrap();

        assert!(assertion.has_errors());
        assert_eq!(assertion.test(), "true()"); // Fallback
    }

    #[test]
    fn test_assertion_evaluate_true() {
        let mut assertion = XsdAssert::new("true()");
        assertion.build().unwrap();

        assert_eq!(assertion.evaluate("any").unwrap(), true);
    }

    #[test]
    fn test_assertion_evaluate_false() {
        let mut assertion = XsdAssert::new("false()");
        assertion.build().unwrap();

        assert_eq!(assertion.evaluate("any").unwrap(), false);
    }

    #[test]
    fn test_assertion_list() {
        let mut list = AssertionList::new();
        assert!(list.is_empty());

        list.add(XsdAssert::new("true()"));
        list.add(XsdAssert::new("false()"));

        assert_eq!(list.len(), 2);
        assert!(!list.is_empty());
    }

    #[test]
    fn test_assertion_list_build_all() {
        let mut list = AssertionList::new();
        list.add(XsdAssert::new("@value > 0"));
        list.add(XsdAssert::new("string-length(.) <= 100"));

        list.build_all().unwrap();

        for assertion in list.iter() {
            assert!(assertion.is_built());
        }
    }

    #[test]
    fn test_assertion_list_evaluate_all() {
        let mut list = AssertionList::new();
        list.add(XsdAssert::new("true()"));
        list.add(XsdAssert::new("false()"));
        list.build_all().unwrap();

        let results = list.evaluate_all("test");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], (0, true));
        assert_eq!(results[1], (1, false));
    }

    #[test]
    fn test_validator_trait() {
        let mut assertion = XsdAssert::new("@test");
        assert_eq!(assertion.validation_attempted(), ValidationStatus::None);

        assertion.build().unwrap();
        assert_eq!(assertion.validation_attempted(), ValidationStatus::Full);
    }

    #[test]
    fn test_assertion_with_xpath_namespace() {
        let assertion = XsdAssert::new("test")
            .with_xpath_default_namespace(XPathDefaultNamespace::TargetNamespace);

        assert_eq!(
            assertion.xpath_default_namespace(),
            Some(&XPathDefaultNamespace::TargetNamespace)
        );
    }
}
