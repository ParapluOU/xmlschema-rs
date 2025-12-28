//! Error types for xmlschema-rs
//!
//! This module defines all error types used throughout the library.
//! It mirrors the exception hierarchy from the Python xmlschema package.

use std::fmt;
use thiserror::Error;

/// Result type alias using xmlschema Error
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for xmlschema operations
#[derive(Error, Debug)]
pub enum Error {
    /// XML Schema validation error
    #[error("validation error: {0}")]
    Validation(#[from] ValidationError),

    /// XML Schema parsing/building error
    #[error("parse error: {0}")]
    Parse(#[from] ParseError),

    /// Type error in schema or data
    #[error("type error: {0}")]
    Type(String),

    /// Value error (invalid value for a type)
    #[error("value error: {0}")]
    Value(String),

    /// Key error (missing required key/element)
    #[error("key error: {0}")]
    Key(String),

    /// Encoding error (data to XML conversion)
    #[error("encoding error: {0}")]
    Encode(String),

    /// Decoding error (XML to data conversion)
    #[error("decoding error: {0}")]
    Decode(String),

    /// Resource loading error
    #[error("resource error: {0}")]
    Resource(String),

    /// Namespace error
    #[error("namespace error: {0}")]
    Namespace(String),

    /// Name error (invalid XML name)
    #[error("name error: {0}")]
    Name(String),

    /// Limit exceeded error
    #[error("limit exceeded: {0}")]
    LimitExceeded(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// XML parsing error
    #[error("XML error: {0}")]
    Xml(String),

    /// URL parsing error
    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

/// XML Schema validation error with context
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Error message
    pub message: String,
    /// Path to the element that failed validation
    pub path: Option<String>,
    /// Schema component that caused the error
    pub schema_component: Option<String>,
    /// XML instance snippet
    pub instance: Option<String>,
    /// Original exception reason
    pub reason: Option<String>,
}

impl ValidationError {
    /// Create a new validation error
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            path: None,
            schema_component: None,
            instance: None,
            reason: None,
        }
    }

    /// Set the path where validation failed
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Set the schema component
    pub fn with_schema_component(mut self, component: impl Into<String>) -> Self {
        self.schema_component = Some(component.into());
        self
    }

    /// Set the instance snippet
    pub fn with_instance(mut self, instance: impl Into<String>) -> Self {
        self.instance = Some(instance.into());
        self
    }

    /// Set the reason
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;

        if let Some(ref reason) = self.reason {
            write!(f, "\n\nReason: {}", reason)?;
        }

        if let Some(ref path) = self.path {
            write!(f, "\n\nPath: {}", path)?;
        }

        if let Some(ref schema) = self.schema_component {
            write!(f, "\n\nSchema:\n{}", schema)?;
        }

        if let Some(ref instance) = self.instance {
            write!(f, "\n\nInstance:\n{}", instance)?;
        }

        Ok(())
    }
}

impl std::error::Error for ValidationError {}

/// XML Schema parsing error
#[derive(Debug, Clone)]
pub struct ParseError {
    /// Error message
    pub message: String,
    /// Location in the schema file
    pub location: Option<String>,
    /// Schema source that caused the error
    pub source: Option<String>,
}

impl ParseError {
    /// Create a new parse error
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            location: None,
            source: None,
        }
    }

    /// Set the location
    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    /// Set the source
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;

        if let Some(ref loc) = self.location {
            write!(f, "\n\nLocation: {}", loc)?;
        }

        if let Some(ref src) = self.source {
            write!(f, "\n\nSource:\n{}", src)?;
        }

        Ok(())
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError::new("Element 'foo' is not valid")
            .with_reason("Required element 'bar' is missing")
            .with_path("/root/foo")
            .with_schema_component("<xs:element name='foo'>...</xs:element>");

        let msg = format!("{}", err);
        assert!(msg.contains("Element 'foo' is not valid"));
        assert!(msg.contains("Reason:"));
        assert!(msg.contains("Path:"));
        assert!(msg.contains("Schema:"));
    }

    #[test]
    fn test_parse_error_display() {
        let err = ParseError::new("Invalid schema syntax")
            .with_location("schema.xsd:42:10")
            .with_source("<xs:element name='invalid'/>");

        let msg = format!("{}", err);
        assert!(msg.contains("Invalid schema syntax"));
        assert!(msg.contains("Location:"));
        assert!(msg.contains("Source:"));
    }

    #[test]
    fn test_error_conversion() {
        let val_err = ValidationError::new("test");
        let err: Error = val_err.into();
        assert!(matches!(err, Error::Validation(_)));
    }
}
