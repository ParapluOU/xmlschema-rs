//! XSD Validation Exceptions
//!
//! This module contains error types used during XML validation against XSD schemas.

use std::fmt;
use crate::error::ParseError;

/// Base trait for XSD validator errors
pub trait XsdValidatorError: std::error::Error + fmt::Debug {
    /// Get the error message
    fn message(&self) -> &str;

    /// Get the XPath to the error location (if available)
    fn path(&self) -> Option<&str>;

    /// Get the schema URL (if available)
    fn schema_url(&self) -> Option<&str>;

    /// Get the source line number (if available)
    fn source_line(&self) -> Option<usize>;
}

/// Validation error when XML data doesn't conform to the schema
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// The error message
    message: String,
    /// The reason for the validation failure
    pub reason: Option<String>,
    /// The XPath to the element that failed validation
    path: Option<String>,
    /// The schema URL
    schema_url: Option<String>,
    /// The source line number
    source_line: Option<usize>,
    /// The tag of the element
    pub element_tag: Option<String>,
    /// The expected type or element
    pub expected: Option<String>,
    /// The actual value found
    pub actual: Option<String>,
}

impl ValidationError {
    /// Create a new validation error
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            reason: None,
            path: None,
            schema_url: None,
            source_line: None,
            element_tag: None,
            expected: None,
            actual: None,
        }
    }

    /// Set the reason
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Set the path
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Set the schema URL
    pub fn with_schema_url(mut self, url: impl Into<String>) -> Self {
        self.schema_url = Some(url.into());
        self
    }

    /// Set the source line
    pub fn with_source_line(mut self, line: usize) -> Self {
        self.source_line = Some(line);
        self
    }

    /// Set the element tag
    pub fn with_element(mut self, tag: impl Into<String>) -> Self {
        self.element_tag = Some(tag.into());
        self
    }

    /// Set expected value
    pub fn with_expected(mut self, expected: impl Into<String>) -> Self {
        self.expected = Some(expected.into());
        self
    }

    /// Set actual value
    pub fn with_actual(mut self, actual: impl Into<String>) -> Self {
        self.actual = Some(actual.into());
        self
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(ref reason) = self.reason {
            write!(f, "\nReason: {}", reason)?;
        }
        if let Some(ref path) = self.path {
            write!(f, "\nPath: {}", path)?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationError {}

impl XsdValidatorError for ValidationError {
    fn message(&self) -> &str {
        &self.message
    }

    fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    fn schema_url(&self) -> Option<&str> {
        self.schema_url.as_deref()
    }

    fn source_line(&self) -> Option<usize> {
        self.source_line
    }
}

/// Error when a child element fails validation
#[derive(Debug, Clone)]
pub struct ChildrenValidationError {
    /// Base validation error
    base: ValidationError,
    /// Index of the invalid child
    pub index: usize,
    /// The invalid tag (if any)
    pub invalid_tag: Option<String>,
    /// Expected tags
    pub expected_tags: Vec<String>,
    /// Number of occurrences
    pub occurs: usize,
    /// Minimum occurrences required
    pub min_occurs: usize,
    /// Maximum occurrences allowed
    pub max_occurs: Option<usize>,
}

impl ChildrenValidationError {
    /// Create a new children validation error for unexpected child
    pub fn unexpected_child(
        parent_tag: impl Into<String>,
        child_tag: impl Into<String>,
        index: usize,
    ) -> Self {
        let parent = parent_tag.into();
        let child = child_tag.into();
        let message = format!(
            "Unexpected child with tag '{}' at position {} in '{}'",
            child, index + 1, parent
        );
        Self {
            base: ValidationError::new(message),
            index,
            invalid_tag: Some(child),
            expected_tags: Vec::new(),
            occurs: 0,
            min_occurs: 0,
            max_occurs: None,
        }
    }

    /// Create a new children validation error for incomplete content
    pub fn incomplete_content(parent_tag: impl Into<String>) -> Self {
        let parent = parent_tag.into();
        let message = format!("The content of element '{}' is not complete", parent);
        Self {
            base: ValidationError::new(message),
            index: 0,
            invalid_tag: None,
            expected_tags: Vec::new(),
            occurs: 0,
            min_occurs: 0,
            max_occurs: None,
        }
    }

    /// Set expected tags
    pub fn with_expected_tags(mut self, tags: Vec<String>) -> Self {
        self.expected_tags = tags;
        self
    }

    /// Set occurrence information
    pub fn with_occurs(mut self, occurs: usize, min: usize, max: Option<usize>) -> Self {
        self.occurs = occurs;
        self.min_occurs = min;
        self.max_occurs = max;
        self
    }

    /// Set the path
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.base.path = Some(path.into());
        self
    }
}

impl fmt::Display for ChildrenValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.base.message)?;

        if self.occurs > 0 && self.min_occurs > self.occurs {
            write!(
                f, " The element occurs {} times but the minimum is {}.",
                self.occurs, self.min_occurs
            )?;
        } else if let Some(max) = self.max_occurs {
            if max < self.occurs {
                write!(
                    f, " The element occurs {} times but the maximum is {}.",
                    self.occurs, max
                )?;
            }
        }

        if !self.expected_tags.is_empty() {
            if self.expected_tags.len() == 1 {
                write!(f, " Tag '{}' expected.", self.expected_tags[0])?;
            } else {
                let tags: Vec<_> = self.expected_tags.iter()
                    .map(|t| format!("'{}'", t))
                    .collect();
                write!(f, " Tag ({}) expected.", tags.join(" | "))?;
            }
        }

        if let Some(ref path) = self.base.path {
            write!(f, "\nPath: {}", path)?;
        }

        Ok(())
    }
}

impl std::error::Error for ChildrenValidationError {}

/// Decode error when XML data cannot be decoded to a value
#[derive(Debug, Clone)]
pub struct DecodeError {
    /// Base validation error
    base: ValidationError,
    /// The value that failed to decode
    pub value: String,
    /// The target type
    pub target_type: Option<String>,
}

impl DecodeError {
    /// Create a new decode error
    pub fn new(value: impl Into<String>, reason: impl Into<String>) -> Self {
        let val = value.into();
        let message = format!("failed decoding '{}'", val);
        Self {
            base: ValidationError::new(message).with_reason(reason),
            value: val,
            target_type: None,
        }
    }

    /// Set the target type
    pub fn with_target_type(mut self, type_name: impl Into<String>) -> Self {
        self.target_type = Some(type_name.into());
        self
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.base)?;
        if let Some(ref target) = self.target_type {
            write!(f, " (target type: {})", target)?;
        }
        Ok(())
    }
}

impl std::error::Error for DecodeError {}

/// Encode error when data cannot be encoded to XML
#[derive(Debug, Clone)]
pub struct EncodeError {
    /// Base validation error
    base: ValidationError,
    /// The value that failed to encode
    pub value: String,
}

impl EncodeError {
    /// Create a new encode error
    pub fn new(value: impl Into<String>, reason: impl Into<String>) -> Self {
        let val = value.into();
        let message = format!("failed encoding '{}'", val);
        Self {
            base: ValidationError::new(message).with_reason(reason),
            value: val,
        }
    }
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.base)
    }
}

impl std::error::Error for EncodeError {}

/// Error for circular definitions in schema
#[derive(Debug, Clone)]
pub struct CircularityError {
    /// The component type (element, complexType, etc.)
    pub component_type: String,
    /// The component name
    pub component_name: String,
    /// The path of components in the cycle
    pub cycle_path: Vec<String>,
}

impl CircularityError {
    /// Create a new circularity error
    pub fn new(
        component_type: impl Into<String>,
        component_name: impl Into<String>,
    ) -> Self {
        Self {
            component_type: component_type.into(),
            component_name: component_name.into(),
            cycle_path: Vec::new(),
        }
    }

    /// Add to the cycle path
    pub fn with_cycle_path(mut self, path: Vec<String>) -> Self {
        self.cycle_path = path;
        self
    }
}

impl fmt::Display for CircularityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f, "Circular definition detected for xs:{} '{}'",
            self.component_type, self.component_name
        )?;
        if !self.cycle_path.is_empty() {
            write!(f, " (cycle: {})", self.cycle_path.join(" -> "))?;
        }
        Ok(())
    }
}

impl std::error::Error for CircularityError {}

/// Error when using a schema component that hasn't been built
#[derive(Debug, Clone)]
pub struct NotBuiltError {
    /// The component that isn't built
    pub component: String,
    /// Additional message
    pub message: String,
}

impl NotBuiltError {
    /// Create a new not-built error
    pub fn new(component: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            component: component.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for NotBuiltError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Schema component '{}' is not built: {}", self.component, self.message)
    }
}

impl std::error::Error for NotBuiltError {}

/// Model error when checking content models
#[derive(Debug, Clone)]
pub struct ModelError {
    /// The error message
    message: String,
    /// The group or type that has the error
    pub component: Option<String>,
}

impl ModelError {
    /// Create a new model error
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            component: None,
        }
    }

    /// Set the component
    pub fn with_component(mut self, component: impl Into<String>) -> Self {
        self.component = Some(component.into());
        self
    }
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(ref comp) = self.component {
            write!(f, " in {}", comp)?;
        }
        Ok(())
    }
}

impl std::error::Error for ModelError {}

/// Error when maximum model recursion depth is exceeded
#[derive(Debug, Clone)]
pub struct ModelDepthError {
    /// The group where depth was exceeded
    pub group: Option<String>,
    /// The current depth
    pub depth: usize,
    /// The maximum allowed depth
    pub max_depth: usize,
}

impl ModelDepthError {
    /// Create a new model depth error
    pub fn new(depth: usize, max_depth: usize) -> Self {
        Self {
            group: None,
            depth,
            max_depth,
        }
    }

    /// Set the group
    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }
}

impl fmt::Display for ModelDepthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f, "Maximum model recursion depth ({}) exceeded (at depth {})",
            self.max_depth, self.depth
        )?;
        if let Some(ref group) = self.group {
            write!(f, " while iterating {}", group)?;
        }
        Ok(())
    }
}

impl std::error::Error for ModelDepthError {}

/// Exception to stop the validation process
#[derive(Debug, Clone)]
pub struct StopValidation {
    /// Optional message
    pub message: Option<String>,
}

impl StopValidation {
    /// Create a stop validation exception
    pub fn new() -> Self {
        Self { message: None }
    }

    /// With a message
    pub fn with_message(message: impl Into<String>) -> Self {
        Self { message: Some(message.into()) }
    }
}

impl Default for StopValidation {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for StopValidation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref msg) = self.message {
            write!(f, "Validation stopped: {}", msg)
        } else {
            write!(f, "Validation stopped")
        }
    }
}

impl std::error::Error for StopValidation {}

/// Convert from ParseError to ValidationError
impl From<ParseError> for ValidationError {
    fn from(err: ParseError) -> Self {
        ValidationError::new(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error() {
        let error = ValidationError::new("Invalid element")
            .with_reason("element 'foo' not expected")
            .with_path("/root/child")
            .with_source_line(42);

        assert_eq!(error.message(), "Invalid element");
        assert_eq!(error.path(), Some("/root/child"));
        assert_eq!(error.source_line(), Some(42));
    }

    #[test]
    fn test_validation_error_display() {
        let error = ValidationError::new("Invalid element")
            .with_reason("element 'foo' not expected")
            .with_path("/root/child");

        let display = error.to_string();
        assert!(display.contains("Invalid element"));
        assert!(display.contains("Reason:"));
        assert!(display.contains("Path:"));
    }

    #[test]
    fn test_children_validation_error_unexpected() {
        let error = ChildrenValidationError::unexpected_child("parent", "child", 2)
            .with_expected_tags(vec!["expected1".to_string(), "expected2".to_string()]);

        let display = error.to_string();
        assert!(display.contains("Unexpected child"));
        assert!(display.contains("child"));
        assert!(display.contains("position 3"));
        assert!(display.contains("expected1"));
    }

    #[test]
    fn test_children_validation_error_incomplete() {
        let error = ChildrenValidationError::incomplete_content("parent");
        let display = error.to_string();
        assert!(display.contains("not complete"));
    }

    #[test]
    fn test_children_validation_error_occurs() {
        let error = ChildrenValidationError::unexpected_child("parent", "child", 0)
            .with_occurs(1, 2, None);

        let display = error.to_string();
        assert!(display.contains("1 times"));
        assert!(display.contains("minimum is 2"));
    }

    #[test]
    fn test_decode_error() {
        let error = DecodeError::new("abc", "not a valid integer")
            .with_target_type("xs:integer");

        let display = error.to_string();
        assert!(display.contains("abc"));
        assert!(display.contains("not a valid integer"));
        assert!(display.contains("xs:integer"));
    }

    #[test]
    fn test_encode_error() {
        let error = EncodeError::new("invalid", "cannot encode value");
        let display = error.to_string();
        assert!(display.contains("invalid"));
    }

    #[test]
    fn test_circularity_error() {
        let error = CircularityError::new("complexType", "MyType")
            .with_cycle_path(vec![
                "MyType".to_string(),
                "OtherType".to_string(),
                "MyType".to_string(),
            ]);

        let display = error.to_string();
        assert!(display.contains("Circular definition"));
        assert!(display.contains("MyType"));
        assert!(display.contains("cycle:"));
    }

    #[test]
    fn test_not_built_error() {
        let error = NotBuiltError::new("element 'foo'", "call build() first");
        let display = error.to_string();
        assert!(display.contains("not built"));
        assert!(display.contains("foo"));
    }

    #[test]
    fn test_model_error() {
        let error = ModelError::new("Non-deterministic content model")
            .with_component("MyGroup");

        let display = error.to_string();
        assert!(display.contains("Non-deterministic"));
        assert!(display.contains("MyGroup"));
    }

    #[test]
    fn test_model_depth_error() {
        let error = ModelDepthError::new(100, 50)
            .with_group("RecursiveGroup");

        let display = error.to_string();
        assert!(display.contains("100"));
        assert!(display.contains("50"));
        assert!(display.contains("RecursiveGroup"));
    }

    #[test]
    fn test_stop_validation() {
        let stop = StopValidation::new();
        assert!(stop.message.is_none());

        let stop = StopValidation::with_message("user requested stop");
        assert_eq!(stop.message.as_deref(), Some("user requested stop"));
    }
}
