//! XSD Identity Constraints
//!
//! This module implements identity constraints for XML Schema:
//! - xs:unique - Ensures values are unique within scope
//! - xs:key - Like unique, but all field values must be present
//! - xs:keyref - References a key/unique constraint (foreign key)
//!
//! Based on Python xmlschema/validators/identities.py

use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{ParseError, Result};
use crate::namespaces::QName;

use super::base::{ValidationStatus, Validator};

/// Type for identity field values.
/// In XPath terms, this is the typed or untyped atomic value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FieldValue {
    /// String value
    String(String),
    /// Integer value
    Integer(i64),
    /// Boolean value
    Boolean(bool),
    /// QName value
    QName(QName),
    /// Null/missing value
    Null,
}

impl FieldValue {
    /// Check if this is a null/missing value
    pub fn is_null(&self) -> bool {
        matches!(self, FieldValue::Null)
    }
}

impl From<String> for FieldValue {
    fn from(s: String) -> Self {
        FieldValue::String(s)
    }
}

impl From<&str> for FieldValue {
    fn from(s: &str) -> Self {
        FieldValue::String(s.to_string())
    }
}

impl From<i64> for FieldValue {
    fn from(i: i64) -> Self {
        FieldValue::Integer(i)
    }
}

impl From<bool> for FieldValue {
    fn from(b: bool) -> Self {
        FieldValue::Boolean(b)
    }
}

impl From<QName> for FieldValue {
    fn from(q: QName) -> Self {
        FieldValue::QName(q)
    }
}

/// A tuple of field values forming a composite key
pub type FieldTuple = Vec<FieldValue>;

/// XPath selector for identity constraints.
/// The selector identifies which elements are subject to the constraint.
#[derive(Debug, Clone)]
pub struct XsdSelector {
    /// The XPath expression
    pub xpath: String,
    /// XPath default namespace (XSD 1.1)
    pub xpath_default_namespace: Option<String>,
    /// Parse errors
    errors: Vec<ParseError>,
}

impl XsdSelector {
    /// Create a new selector with the given XPath expression
    pub fn new(xpath: impl Into<String>) -> Self {
        Self {
            xpath: xpath.into(),
            xpath_default_namespace: None,
            errors: Vec::new(),
        }
    }

    /// Create a selector with default namespace
    pub fn with_default_namespace(xpath: impl Into<String>, ns: impl Into<String>) -> Self {
        Self {
            xpath: xpath.into(),
            xpath_default_namespace: Some(ns.into()),
            errors: Vec::new(),
        }
    }

    /// Validate the selector XPath expression
    pub fn validate(&mut self) -> bool {
        // Simplified validation - real implementation would parse XPath
        // Selectors must match the restricted pattern for identity constraints
        let path = self.xpath.replace(' ', "");

        // Basic validation - must start with . or child::
        if path.is_empty() {
            self.errors.push(ParseError::new(
                "selector xpath expression cannot be empty",
            ));
            return false;
        }

        true
    }

    /// Get parse errors
    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }
}

/// XPath field selector for identity constraints.
/// Fields identify which values form the key within selected elements.
#[derive(Debug, Clone)]
pub struct XsdField {
    /// The XPath expression
    pub xpath: String,
    /// XPath default namespace (XSD 1.1)
    pub xpath_default_namespace: Option<String>,
    /// Parse errors
    errors: Vec<ParseError>,
}

impl XsdField {
    /// Create a new field with the given XPath expression
    pub fn new(xpath: impl Into<String>) -> Self {
        Self {
            xpath: xpath.into(),
            xpath_default_namespace: None,
            errors: Vec::new(),
        }
    }

    /// Create a field with default namespace
    pub fn with_default_namespace(xpath: impl Into<String>, ns: impl Into<String>) -> Self {
        Self {
            xpath: xpath.into(),
            xpath_default_namespace: Some(ns.into()),
            errors: Vec::new(),
        }
    }

    /// Validate the field XPath expression
    pub fn validate(&mut self) -> bool {
        // Simplified validation - real implementation would parse XPath
        let path = self.xpath.replace(' ', "");

        if path.is_empty() {
            self.errors.push(ParseError::new(
                "field xpath expression cannot be empty",
            ));
            return false;
        }

        true
    }

    /// Get parse errors
    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }
}

/// Type of identity constraint
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityConstraintKind {
    /// xs:unique - values must be unique, but fields can be missing
    Unique,
    /// xs:key - values must be unique AND all fields must be present
    Key,
    /// xs:keyref - references a key or unique constraint
    Keyref,
}

/// Base structure for identity constraints
#[derive(Debug, Clone)]
pub struct XsdIdentity {
    /// Constraint name
    pub name: QName,
    /// Kind of constraint
    pub kind: IdentityConstraintKind,
    /// XPath selector
    pub selector: XsdSelector,
    /// XPath fields
    pub fields: Vec<XsdField>,
    /// Reference to another constraint (for keyref)
    pub refer: Option<QName>,
    /// Target namespace
    pub target_namespace: Option<String>,
    /// Reference to identity constraint (XSD 1.1)
    pub ref_identity: Option<QName>,
    /// Parse errors
    errors: Vec<ParseError>,
    /// Whether the constraint has been built
    built: bool,
}

impl XsdIdentity {
    /// Create a new identity constraint
    pub fn new(name: QName, kind: IdentityConstraintKind, selector: XsdSelector) -> Self {
        Self {
            name,
            kind,
            selector,
            fields: Vec::new(),
            refer: None,
            target_namespace: None,
            ref_identity: None,
            errors: Vec::new(),
            built: false,
        }
    }

    /// Create a unique constraint
    pub fn unique(name: QName, selector: XsdSelector) -> Self {
        Self::new(name, IdentityConstraintKind::Unique, selector)
    }

    /// Create a key constraint
    pub fn key(name: QName, selector: XsdSelector) -> Self {
        Self::new(name, IdentityConstraintKind::Key, selector)
    }

    /// Create a keyref constraint
    pub fn keyref(name: QName, selector: XsdSelector, refer: QName) -> Self {
        let mut identity = Self::new(name, IdentityConstraintKind::Keyref, selector);
        identity.refer = Some(refer);
        identity
    }

    /// Add a field to this constraint
    pub fn add_field(&mut self, field: XsdField) {
        self.fields.push(field);
    }

    /// Add multiple fields
    pub fn with_fields(mut self, fields: impl IntoIterator<Item = XsdField>) -> Self {
        self.fields.extend(fields);
        self
    }

    /// Set the refer attribute (for keyref)
    pub fn with_refer(mut self, refer: QName) -> Self {
        self.refer = Some(refer);
        self
    }

    /// Set target namespace
    pub fn with_target_namespace(mut self, ns: impl Into<String>) -> Self {
        self.target_namespace = Some(ns.into());
        self
    }

    /// Check if this is a unique constraint
    pub fn is_unique(&self) -> bool {
        matches!(self.kind, IdentityConstraintKind::Unique)
    }

    /// Check if this is a key constraint
    pub fn is_key(&self) -> bool {
        matches!(self.kind, IdentityConstraintKind::Key)
    }

    /// Check if this is a keyref constraint
    pub fn is_keyref(&self) -> bool {
        matches!(self.kind, IdentityConstraintKind::Keyref)
    }

    /// Validate the identity constraint
    pub fn validate(&mut self) -> bool {
        let mut valid = true;

        // Validate selector
        if !self.selector.validate() {
            valid = false;
            self.errors.extend(self.selector.errors.iter().cloned());
        }

        // Must have at least one field
        if self.fields.is_empty() {
            self.errors.push(ParseError::new(format!(
                "identity constraint '{}' must have at least one field",
                self.name.to_string()
            )));
            valid = false;
        }

        // Validate fields
        for field in &mut self.fields {
            if !field.validate() {
                valid = false;
                self.errors.extend(field.errors.iter().cloned());
            }
        }

        // Keyref must have refer attribute
        if self.is_keyref() && self.refer.is_none() {
            self.errors.push(ParseError::new(format!(
                "keyref '{}' must have a 'refer' attribute",
                self.name.to_string()
            )));
            valid = false;
        }

        valid
    }

    /// Add a parse error
    pub fn add_error(&mut self, error: ParseError) {
        self.errors.push(error);
    }

    /// Get parse errors (internal method)
    pub fn get_errors(&self) -> &[ParseError] {
        &self.errors
    }

    /// Check if there are errors (internal method)
    pub fn check_has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

impl Validator for XsdIdentity {
    fn is_built(&self) -> bool {
        self.built
    }

    fn build(&mut self) -> Result<()> {
        if self.built {
            return Ok(());
        }

        // Validate the constraint
        self.validate();
        self.built = true;
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

/// Counter for tracking identity constraint values during validation.
/// Used to detect duplicates and verify referential integrity.
#[derive(Debug, Clone)]
pub struct IdentityCounter {
    /// The identity constraint being tracked
    pub identity: Arc<XsdIdentity>,
    /// Counted field tuples with their counts
    counter: HashMap<FieldTuple, usize>,
    /// Whether this counter is enabled
    pub enabled: bool,
}

impl IdentityCounter {
    /// Create a new counter for an identity constraint
    pub fn new(identity: Arc<XsdIdentity>) -> Self {
        Self {
            identity,
            counter: HashMap::new(),
            enabled: true,
        }
    }

    /// Reset the counter
    pub fn reset(&mut self) {
        self.counter.clear();
        self.enabled = true;
    }

    /// Increase the count for a field tuple
    /// Returns an error if a duplicate is detected (for unique/key)
    pub fn increase(&mut self, fields: FieldTuple) -> Result<()> {
        let count = self.counter.entry(fields.clone()).or_insert(0);
        *count += 1;

        // For unique and key, duplicates are errors
        if !self.identity.is_keyref() && *count == 2 {
            return Err(crate::error::Error::Parse(ParseError::new(format!(
                "duplicated value {:?} for '{}'",
                fields, self.identity.name.to_string()
            ))));
        }

        Ok(())
    }

    /// Get the count for a field tuple
    pub fn get_count(&self, fields: &FieldTuple) -> usize {
        self.counter.get(fields).copied().unwrap_or(0)
    }

    /// Check if a field tuple exists
    pub fn contains(&self, fields: &FieldTuple) -> bool {
        self.counter.contains_key(fields)
    }

    /// Get all field tuples
    pub fn keys(&self) -> impl Iterator<Item = &FieldTuple> {
        self.counter.keys()
    }

    /// Get the number of unique field tuples
    pub fn len(&self) -> usize {
        self.counter.len()
    }

    /// Check if counter is empty
    pub fn is_empty(&self) -> bool {
        self.counter.is_empty()
    }
}

/// Counter for keyref constraints.
/// Tracks values and validates they exist in the referenced key/unique.
#[derive(Debug, Clone)]
pub struct KeyrefCounter {
    /// Base counter
    counter: IdentityCounter,
    /// Reference to the key/unique constraint
    pub refer: Option<Arc<XsdIdentity>>,
    /// Path to the referred constraint's scope
    pub refer_path: String,
}

impl KeyrefCounter {
    /// Create a new keyref counter
    pub fn new(identity: Arc<XsdIdentity>) -> Self {
        Self {
            counter: IdentityCounter::new(identity),
            refer: None,
            refer_path: ".".to_string(),
        }
    }

    /// Set the referenced constraint
    pub fn with_refer(mut self, refer: Arc<XsdIdentity>) -> Self {
        self.refer = Some(refer);
        self
    }

    /// Increase the count for a field tuple
    pub fn increase(&mut self, fields: FieldTuple) {
        // For keyref, we don't check for duplicates immediately
        let count = self.counter.counter.entry(fields).or_insert(0);
        *count += 1;
    }

    /// Validate that all keyref values exist in the referenced constraint
    pub fn validate_references(
        &self,
        refer_counter: &IdentityCounter,
    ) -> Vec<ParseError> {
        let mut errors = Vec::new();

        // Check if this identity has a refer (i.e., is a keyref)
        if self.counter.identity.refer.is_none() {
            return errors; // Not a keyref, can't validate
        }

        for (fields, count) in &self.counter.counter {
            if !refer_counter.contains(fields) {
                // Handle single-field special case
                if fields.len() == 1 {
                    let single = vec![fields[0].clone()];
                    if refer_counter.contains(&single) {
                        continue;
                    }
                }

                let refer_name = self.counter.identity.refer
                    .as_ref()
                    .map(|q: &QName| q.to_string())
                    .unwrap_or_default();

                let msg = if *count > 1 {
                    format!(
                        "value {:?} not found for '{}' ({} times)",
                        fields,
                        refer_name,
                        count
                    )
                } else {
                    format!(
                        "value {:?} not found for '{}'",
                        fields,
                        refer_name
                    )
                };

                errors.push(ParseError::new(msg));
            }
        }

        errors
    }

    /// Get the underlying counter
    pub fn counter(&self) -> &IdentityCounter {
        &self.counter
    }
}

/// Map of identity constraints to their counters
pub type IdentityMap = HashMap<QName, IdentityCounter>;

/// Manager for identity constraints during validation
#[derive(Debug, Default)]
pub struct IdentityManager {
    /// Registered identity constraints
    constraints: HashMap<QName, Arc<XsdIdentity>>,
    /// Active counters during validation
    counters: HashMap<QName, IdentityCounter>,
    /// Active keyref counters
    keyref_counters: HashMap<QName, KeyrefCounter>,
}

impl IdentityManager {
    /// Create a new identity manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an identity constraint
    pub fn register(&mut self, identity: XsdIdentity) -> Arc<XsdIdentity> {
        let arc = Arc::new(identity);
        self.constraints.insert(arc.name.clone(), arc.clone());
        arc
    }

    /// Get a registered constraint by name
    pub fn get(&self, name: &QName) -> Option<Arc<XsdIdentity>> {
        self.constraints.get(name).cloned()
    }

    /// Start tracking a constraint during validation
    pub fn start_tracking(&mut self, name: &QName) -> Option<&mut IdentityCounter> {
        let identity: Arc<XsdIdentity> = self.constraints.get(name)?.clone();
        if identity.is_keyref() {
            return None; // Use start_keyref_tracking instead
        }
        self.counters
            .entry(name.clone())
            .or_insert_with(|| IdentityCounter::new(identity));
        self.counters.get_mut(name)
    }

    /// Start tracking a keyref constraint
    pub fn start_keyref_tracking(&mut self, name: &QName) -> Option<&mut KeyrefCounter> {
        let identity: Arc<XsdIdentity> = self.constraints.get(name)?.clone();
        if !identity.is_keyref() {
            return None;
        }

        // Try to resolve the refer
        let refer: Option<Arc<XsdIdentity>> = identity.refer
            .as_ref()
            .and_then(|refer_name| self.constraints.get(refer_name).cloned());

        self.keyref_counters
            .entry(name.clone())
            .or_insert_with(|| {
                let mut counter = KeyrefCounter::new(identity);
                counter.refer = refer;
                counter
            });
        self.keyref_counters.get_mut(name)
    }

    /// Get a counter
    pub fn get_counter(&self, name: &QName) -> Option<&IdentityCounter> {
        self.counters.get(name)
    }

    /// Get a keyref counter
    pub fn get_keyref_counter(&self, name: &QName) -> Option<&KeyrefCounter> {
        self.keyref_counters.get(name)
    }

    /// Validate all keyref constraints
    pub fn validate_keyrefs(&self) -> Vec<ParseError> {
        let mut errors = Vec::new();

        for (name, keyref_counter) in &self.keyref_counters {
            if let Some(refer_name) = &keyref_counter.counter.identity.refer {
                if let Some(refer_counter) = self.counters.get(refer_name) {
                    errors.extend(keyref_counter.validate_references(refer_counter));
                } else {
                    errors.push(ParseError::new(format!(
                        "referenced constraint '{}' not found for keyref '{}'",
                        refer_name.to_string(), name.to_string()
                    )));
                }
            }
        }

        errors
    }

    /// Reset all counters
    pub fn reset(&mut self) {
        self.counters.clear();
        self.keyref_counters.clear();
    }

    /// Get all registered constraints
    pub fn constraints(&self) -> impl Iterator<Item = &Arc<XsdIdentity>> {
        self.constraints.values()
    }
}

/// Builder for identity constraints
#[derive(Debug)]
pub struct IdentityBuilder {
    name: Option<QName>,
    kind: IdentityConstraintKind,
    selector: Option<XsdSelector>,
    fields: Vec<XsdField>,
    refer: Option<QName>,
    target_namespace: Option<String>,
}

impl IdentityBuilder {
    /// Create a builder for a unique constraint
    pub fn unique() -> Self {
        Self {
            name: None,
            kind: IdentityConstraintKind::Unique,
            selector: None,
            fields: Vec::new(),
            refer: None,
            target_namespace: None,
        }
    }

    /// Create a builder for a key constraint
    pub fn key() -> Self {
        Self {
            name: None,
            kind: IdentityConstraintKind::Key,
            selector: None,
            fields: Vec::new(),
            refer: None,
            target_namespace: None,
        }
    }

    /// Create a builder for a keyref constraint
    pub fn keyref() -> Self {
        Self {
            name: None,
            kind: IdentityConstraintKind::Keyref,
            selector: None,
            fields: Vec::new(),
            refer: None,
            target_namespace: None,
        }
    }

    /// Set the constraint name
    pub fn name(mut self, name: QName) -> Self {
        self.name = Some(name);
        self
    }

    /// Set the selector
    pub fn selector(mut self, xpath: impl Into<String>) -> Self {
        self.selector = Some(XsdSelector::new(xpath));
        self
    }

    /// Add a field
    pub fn field(mut self, xpath: impl Into<String>) -> Self {
        self.fields.push(XsdField::new(xpath));
        self
    }

    /// Set the refer attribute (for keyref)
    pub fn refer(mut self, refer: QName) -> Self {
        self.refer = Some(refer);
        self
    }

    /// Set target namespace
    pub fn target_namespace(mut self, ns: impl Into<String>) -> Self {
        self.target_namespace = Some(ns.into());
        self
    }

    /// Build the identity constraint
    pub fn build(self) -> std::result::Result<XsdIdentity, ParseError> {
        let name = self.name.ok_or_else(|| ParseError::new(
            "identity constraint must have a name",
        ))?;

        let selector = self.selector.ok_or_else(|| ParseError::new(
            "identity constraint must have a selector",
        ))?;

        if self.fields.is_empty() {
            return Err(ParseError::new(
                "identity constraint must have at least one field",
            ));
        }

        if self.kind == IdentityConstraintKind::Keyref && self.refer.is_none() {
            return Err(ParseError::new(
                "keyref must have a 'refer' attribute",
            ));
        }

        let mut identity = XsdIdentity::new(name, self.kind, selector);
        identity.fields = self.fields;
        identity.refer = self.refer;
        identity.target_namespace = self.target_namespace;

        Ok(identity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_value_creation() {
        let s = FieldValue::from("test");
        assert!(matches!(s, FieldValue::String(_)));

        let i = FieldValue::from(42i64);
        assert!(matches!(i, FieldValue::Integer(42)));

        let b = FieldValue::from(true);
        assert!(matches!(b, FieldValue::Boolean(true)));

        let null = FieldValue::Null;
        assert!(null.is_null());
    }

    #[test]
    fn test_selector_creation() {
        let selector = XsdSelector::new(".//item");
        assert_eq!(selector.xpath, ".//item");
        assert!(selector.xpath_default_namespace.is_none());

        let selector_ns = XsdSelector::with_default_namespace(
            ".//item",
            "http://example.com",
        );
        assert_eq!(selector_ns.xpath_default_namespace.as_deref(), Some("http://example.com"));
    }

    #[test]
    fn test_field_creation() {
        let field = XsdField::new("@id");
        assert_eq!(field.xpath, "@id");
    }

    #[test]
    fn test_identity_unique_creation() {
        let identity = XsdIdentity::unique(
            QName::local("productKey"),
            XsdSelector::new(".//product"),
        );

        assert!(identity.is_unique());
        assert!(!identity.is_key());
        assert!(!identity.is_keyref());
        assert!(identity.refer.is_none());
    }

    #[test]
    fn test_identity_key_creation() {
        let mut identity = XsdIdentity::key(
            QName::local("productKey"),
            XsdSelector::new(".//product"),
        );
        identity.add_field(XsdField::new("@id"));

        assert!(identity.is_key());
        assert_eq!(identity.fields.len(), 1);
    }

    #[test]
    fn test_identity_keyref_creation() {
        let identity = XsdIdentity::keyref(
            QName::local("orderProductRef"),
            XsdSelector::new(".//orderItem"),
            QName::local("productKey"),
        );

        assert!(identity.is_keyref());
        assert_eq!(identity.refer.as_ref().unwrap().local_name, "productKey");
    }

    #[test]
    fn test_identity_builder() {
        let identity = IdentityBuilder::key()
            .name(QName::local("bookKey"))
            .selector(".//book")
            .field("@isbn")
            .target_namespace("http://example.com/books")
            .build()
            .unwrap();

        assert!(identity.is_key());
        assert_eq!(identity.name.local_name, "bookKey");
        assert_eq!(identity.selector.xpath, ".//book");
        assert_eq!(identity.fields.len(), 1);
        assert_eq!(identity.fields[0].xpath, "@isbn");
    }

    #[test]
    fn test_keyref_builder() {
        let identity = IdentityBuilder::keyref()
            .name(QName::local("orderBookRef"))
            .selector(".//orderItem")
            .field("bookId")
            .refer(QName::local("bookKey"))
            .build()
            .unwrap();

        assert!(identity.is_keyref());
        assert_eq!(identity.refer.as_ref().unwrap().local_name, "bookKey");
    }

    #[test]
    fn test_builder_validation_errors() {
        // Missing name
        let result = IdentityBuilder::unique()
            .selector(".")
            .field("@id")
            .build();
        assert!(result.is_err());

        // Missing selector
        let result = IdentityBuilder::unique()
            .name(QName::local("test"))
            .field("@id")
            .build();
        assert!(result.is_err());

        // Missing field
        let result = IdentityBuilder::unique()
            .name(QName::local("test"))
            .selector(".")
            .build();
        assert!(result.is_err());

        // Keyref missing refer
        let result = IdentityBuilder::keyref()
            .name(QName::local("test"))
            .selector(".")
            .field("@id")
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_identity_counter() {
        let identity = Arc::new(
            XsdIdentity::unique(
                QName::local("testKey"),
                XsdSelector::new(".//item"),
            ).with_fields([XsdField::new("@id")])
        );

        let mut counter = IdentityCounter::new(identity);

        // First value should succeed
        let result = counter.increase(vec![FieldValue::from("1")]);
        assert!(result.is_ok());

        // Second different value should succeed
        let result = counter.increase(vec![FieldValue::from("2")]);
        assert!(result.is_ok());

        // Duplicate should fail
        let result = counter.increase(vec![FieldValue::from("1")]);
        assert!(result.is_err());
    }

    #[test]
    fn test_keyref_counter() {
        let key_identity = Arc::new(
            XsdIdentity::key(
                QName::local("productKey"),
                XsdSelector::new(".//product"),
            ).with_fields([XsdField::new("@id")])
        );

        let keyref_identity = Arc::new(
            XsdIdentity::keyref(
                QName::local("orderRef"),
                XsdSelector::new(".//order"),
                QName::local("productKey"),
            ).with_fields([XsdField::new("productId")])
        );

        // Set up key counter with some values
        let mut key_counter = IdentityCounter::new(key_identity);
        key_counter.increase(vec![FieldValue::from("P1")]).unwrap();
        key_counter.increase(vec![FieldValue::from("P2")]).unwrap();

        // Set up keyref counter
        let mut keyref_counter = KeyrefCounter::new(keyref_identity);
        keyref_counter.increase(vec![FieldValue::from("P1")]); // Valid ref
        keyref_counter.increase(vec![FieldValue::from("P3")]); // Invalid ref

        // Validate references
        let errors = keyref_counter.validate_references(&key_counter);
        assert_eq!(errors.len(), 1); // P3 not found
    }

    #[test]
    fn test_identity_manager() {
        let mut manager = IdentityManager::new();

        // Register a key constraint
        let _key = manager.register(
            XsdIdentity::key(
                QName::local("productKey"),
                XsdSelector::new(".//product"),
            ).with_fields([XsdField::new("@id")])
        );

        // Register a keyref constraint
        let _keyref = manager.register(
            XsdIdentity::keyref(
                QName::local("orderRef"),
                XsdSelector::new(".//order"),
                QName::local("productKey"),
            ).with_fields([XsdField::new("productId")])
        );

        // Start tracking
        let counter = manager.start_tracking(&QName::local("productKey"));
        assert!(counter.is_some());

        // Get constraint
        let retrieved = manager.get(&QName::local("productKey"));
        assert!(retrieved.is_some());
        assert!(retrieved.unwrap().is_key());
    }

    #[test]
    fn test_composite_key() {
        let identity = Arc::new(
            XsdIdentity::key(
                QName::local("compositeKey"),
                XsdSelector::new(".//item"),
            ).with_fields([
                XsdField::new("@type"),
                XsdField::new("@id"),
            ])
        );

        let mut counter = IdentityCounter::new(identity);

        // Different composite keys
        counter.increase(vec![
            FieldValue::from("book"),
            FieldValue::from("1"),
        ]).unwrap();

        counter.increase(vec![
            FieldValue::from("dvd"),
            FieldValue::from("1"),
        ]).unwrap();

        // Same composite key should fail
        let result = counter.increase(vec![
            FieldValue::from("book"),
            FieldValue::from("1"),
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn test_identity_validation() {
        let mut identity = XsdIdentity::unique(
            QName::local("test"),
            XsdSelector::new(".//item"),
        );

        // No fields - should fail
        assert!(!identity.validate());
        assert!(identity.check_has_errors());

        // Add field - should succeed
        identity.fields.push(XsdField::new("@id"));
        identity.errors.clear();
        assert!(identity.validate());
    }
}
