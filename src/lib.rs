//! # xmlschema-rs
//!
//! A Rust implementation of XML Schema (XSD 1.0 and XSD 1.1) for validation and data conversion.
//!
//! This library is a port of the Python [xmlschema](https://github.com/sissaschool/xmlschema) package.
//!
//! ## Features
//!
//! - Full XSD 1.0 support (in progress)
//! - XSD 1.1 support (planned)
//! - XML validation against XSD schemas
//! - XML to Rust data structure conversion
//! - Rust data structure to XML conversion
//! - JSON conversion support
//! - XPath-based schema navigation
//! - Protection against XML attacks
//! - Resource caching
//!
//! ## Example
//!
//! ```rust,ignore
//! use xmlschema::Schema;
//!
//! // Load a schema
//! let schema = Schema::from_file("path/to/schema.xsd")?;
//!
//! // Validate an XML document
//! let is_valid = schema.is_valid("path/to/document.xml")?;
//!
//! // Convert XML to a dictionary
//! let data = schema.to_dict("path/to/document.xml")?;
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![allow(dead_code)] // During development

// Core modules - Wave 1: Foundation
pub mod error;
pub mod limits;

// Core modules - Wave 2: Utilities
pub mod namespaces;
pub mod names;
pub mod locations;

// Core modules - Wave 3: Resource Loading
pub mod loaders;
pub mod documents;

// Validators - Wave 4+
pub mod validators;

// Data conversion - Wave 9
pub mod converters;
pub mod exports;
// pub mod dataobjects;  // Later

// Testing support
pub mod comparison;

// XPath support - Wave 10
// pub mod xpath;

// Utilities - Wave 11
// pub mod arguments;
// pub mod settings;
// pub mod aliases;

// Re-exports for convenience
pub use error::{Error, Result};
// pub use validators::Schema;  // Will be implemented later

/// Version of the xmlschema-rs library
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// XSD 1.0 namespace
pub const XSD_1_0_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema";

/// XSD 1.1 namespace
pub const XSD_1_1_NAMESPACE: &str = "http://www.w3.org/2009/XMLSchema";

/// XML namespace
pub const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";

/// XMLNS namespace
pub const XMLNS_NAMESPACE: &str = "http://www.w3.org/2000/xmlns/";
