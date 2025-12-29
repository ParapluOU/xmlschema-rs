//! Document Validation
//!
//! This module implements XML document validation against XSD schemas.
//! It provides the core logic for validating XML documents, elements,
//! attributes, and content models.

use std::sync::Arc;

use crate::documents::{Document, Element};
use crate::error::Result;
use crate::namespaces::QName;

use super::base::AttributeValidator;
use super::complex_types::{ComplexContent, ContentTypeLabel, XsdComplexType};
use super::elements::{ElementType, XsdElement};
use super::groups::GroupParticle;
use super::models::ModelVisitor;
use super::schemas::{XsdSchema, XSI_NAMESPACE};
use super::simple_types::SimpleType;
use super::validation::ValidationContext;

/// Validate an XML document against the schema
pub fn validate_document(
    schema: &XsdSchema,
    doc: &Document,
    context: &mut ValidationContext,
) -> Result<()> {
    let root = match &doc.root {
        Some(r) => r,
        None => {
            return context.validation_error("Document has no root element", None);
        }
    };

    // Find the root element declaration
    let root_qname = resolve_element_qname(root, schema);

    let element_decl = match schema.lookup_element(&root_qname) {
        Some(decl) => Arc::clone(decl),
        None => {
            return context.validation_error(
                format!("Unknown root element: {}", root.local_name()),
                Some(format!("No global element declaration found for '{}:{}'",
                    root_qname.namespace.as_deref().unwrap_or(""),
                    root_qname.local_name)),
            );
        }
    };

    // Validate the root element
    validate_element(schema, root, &element_decl, context)
}

/// Validate an XML element against its declaration
pub fn validate_element(
    schema: &XsdSchema,
    elem: &Element,
    decl: &XsdElement,
    context: &mut ValidationContext,
) -> Result<()> {
    context.current_element = Some(elem.local_name().to_string());
    context.enter_level();

    // Check max depth
    if context.is_max_depth_exceeded() {
        context.exit_level();
        return Ok(()); // Skip validation at this depth
    }

    // Check for xsi:nil
    if let Some(nil_value) = get_xsi_attribute(elem, "nil") {
        if nil_value == "true" {
            if !decl.nillable {
                context.validation_error(
                    format!("Element '{}' is not nillable", elem.local_name()),
                    Some("xsi:nil='true' used on non-nillable element".to_string()),
                )?;
            } else {
                // Nilled element should be empty
                if !elem.children.is_empty() || elem.text.is_some() {
                    context.validation_error(
                        format!("Nilled element '{}' must be empty", elem.local_name()),
                        None,
                    )?;
                }
                context.exit_level();
                return Ok(());
            }
        }
    }

    // Validate based on element type
    match &decl.element_type {
        ElementType::Simple(simple_type) => {
            validate_simple_element(elem, simple_type.as_ref(), decl, context)?;
        }
        ElementType::Complex(complex_type) => {
            validate_complex_element(schema, elem, complex_type, decl, context)?;
        }
        ElementType::Any => {
            // Any type allows any content - skip validation
        }
    }

    context.exit_level();
    context.current_element = None;
    Ok(())
}

/// Validate an element with simple type content
fn validate_simple_element(
    elem: &Element,
    simple_type: &(dyn SimpleType + Send + Sync),
    decl: &XsdElement,
    context: &mut ValidationContext,
) -> Result<()> {
    // Simple elements should not have child elements
    if !elem.children.is_empty() {
        context.validation_error(
            format!("Element '{}' has simple type but contains child elements", elem.local_name()),
            None,
        )?;
    }

    // Get text content
    let text = elem.text.as_deref().unwrap_or("");

    // Check for fixed value
    if let Some(ref fixed) = decl.fixed {
        if text != fixed {
            context.validation_error(
                format!(
                    "Element '{}' has fixed value '{}' but contains '{}'",
                    elem.local_name(),
                    fixed,
                    text
                ),
                None,
            )?;
        }
    }

    // Validate the text content against the simple type
    if let Err(e) = simple_type.validate_value(text) {
        context.validation_error(
            format!(
                "Invalid value for element '{}': {}",
                elem.local_name(),
                text
            ),
            Some(e.to_string()),
        )?;
    }

    Ok(())
}

/// Validate an element with complex type content
fn validate_complex_element(
    schema: &XsdSchema,
    elem: &Element,
    complex_type: &Arc<XsdComplexType>,
    decl: &XsdElement,
    context: &mut ValidationContext,
) -> Result<()> {
    // Validate attributes
    validate_attributes(elem, complex_type, context)?;

    // Validate content based on content type
    match complex_type.content_type_label() {
        ContentTypeLabel::Empty => {
            validate_empty_content(elem, context)?;
        }
        ContentTypeLabel::Simple => {
            if let Some(simple_type) = complex_type.simple_type() {
                validate_simple_content(elem, simple_type.as_ref(), decl, context)?;
            }
        }
        ContentTypeLabel::Mixed | ContentTypeLabel::ElementOnly => {
            validate_element_content(schema, elem, complex_type, context)?;
        }
    }

    Ok(())
}

/// Validate empty content (no text, no children)
fn validate_empty_content(elem: &Element, context: &mut ValidationContext) -> Result<()> {
    if !elem.children.is_empty() {
        context.validation_error(
            format!("Element '{}' should be empty but contains child elements", elem.local_name()),
            None,
        )?;
    }

    if let Some(ref text) = elem.text {
        if !text.trim().is_empty() {
            context.validation_error(
                format!("Element '{}' should be empty but contains text", elem.local_name()),
                None,
            )?;
        }
    }

    Ok(())
}

/// Validate simple content (text only, with possible attributes)
fn validate_simple_content(
    elem: &Element,
    simple_type: &(dyn SimpleType + Send + Sync),
    decl: &XsdElement,
    context: &mut ValidationContext,
) -> Result<()> {
    // Should not have child elements
    if !elem.children.is_empty() {
        context.validation_error(
            format!("Element '{}' has simple content but contains child elements", elem.local_name()),
            None,
        )?;
    }

    let text = elem.text.as_deref().unwrap_or("");

    // Check for fixed value
    if let Some(ref fixed) = decl.fixed {
        if text != fixed {
            context.validation_error(
                format!(
                    "Element '{}' has fixed value '{}' but contains '{}'",
                    elem.local_name(),
                    fixed,
                    text
                ),
                None,
            )?;
        }
    }

    // Validate the text content
    if let Err(e) = simple_type.validate_value(text) {
        context.validation_error(
            format!(
                "Invalid simple content for element '{}': {}",
                elem.local_name(),
                text
            ),
            Some(e.to_string()),
        )?;
    }

    Ok(())
}

/// Validate element content (children) using the content model
fn validate_element_content(
    schema: &XsdSchema,
    elem: &Element,
    complex_type: &Arc<XsdComplexType>,
    context: &mut ValidationContext,
) -> Result<()> {
    // Check for text content in element-only mode
    if complex_type.content_type_label() == ContentTypeLabel::ElementOnly {
        if let Some(ref text) = elem.text {
            if !text.trim().is_empty() {
                context.validation_error(
                    format!(
                        "Element '{}' has element-only content but contains text: '{}'",
                        elem.local_name(),
                        text.trim()
                    ),
                    None,
                )?;
            }
        }
    }

    // Get the model group
    let model_group = match &complex_type.content {
        ComplexContent::Group(group) => Arc::clone(group),
        ComplexContent::Simple(_) => {
            // This shouldn't happen for element content, but handle gracefully
            return Ok(());
        }
    };

    // If group is empty and no children, that's valid
    if model_group.is_empty() {
        if !elem.children.is_empty() {
            context.validation_error(
                format!(
                    "Element '{}' should have no child elements but contains {}",
                    elem.local_name(),
                    elem.children.len()
                ),
                None,
            )?;
        }
        return Ok(());
    }

    // Create model visitor and validate children
    let mut visitor = ModelVisitor::new(model_group);

    for child in &elem.children {
        let child_name = child.local_name();

        // Try to match the child element
        let matched = visitor.match_element(child_name);

        if matched.is_some() {
            // Advance the visitor (with match)
            visitor.advance(true);

            // Find the element declaration for the child
            let child_qname = resolve_element_qname(child, schema);

            // Look up the element declaration
            if let Some(child_decl) = schema.lookup_element(&child_qname) {
                validate_element(schema, child, child_decl, context)?;
            } else {
                // Try to find in the content model's particles
                if let Some(child_decl) = find_element_in_visitor(&visitor, &child_qname) {
                    validate_element(schema, child, &child_decl, context)?;
                }
                // Unknown elements are often allowed by wildcards - don't error here
            }
        } else {
            // No match - check if it's allowed by wildcards or report error
            let expected = visitor.expected();
            context.validation_error(
                format!(
                    "Unexpected child element '{}' in '{}'",
                    child_name,
                    elem.local_name()
                ),
                Some(format!("Expected one of: {:?}", expected)),
            )?;

            // Try to advance anyway for error recovery
            visitor.advance(false);
        }
    }

    // Check that model is complete (all required elements present)
    let remaining_errors = visitor.stop();
    for (particle, _count, expected) in remaining_errors {
        if let GroupParticle::Element(missing_elem) = particle {
            if missing_elem.occurs.min > 0 {
                context.validation_error(
                    format!(
                        "Missing required element '{}' in '{}'",
                        missing_elem.name.local_name,
                        elem.local_name()
                    ),
                    Some(format!("Expected: {:?}", expected)),
                )?;
            }
        }
    }

    Ok(())
}

/// Validate element attributes
fn validate_attributes(
    elem: &Element,
    complex_type: &Arc<XsdComplexType>,
    context: &mut ValidationContext,
) -> Result<()> {
    let attr_group = &complex_type.attributes;

    // Track which attributes we've validated
    let mut validated_attrs: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Validate declared attributes
    for attr_decl in attr_group.iter_attributes() {
        let attr_name = attr_decl.name();
        let attr_name_str = &attr_name.local_name;
        validated_attrs.insert(attr_name_str.clone());

        // Get attribute value from element
        let value = elem.get_attribute(attr_name_str);

        // Check use mode
        if attr_decl.is_required() && value.is_none() {
            context.validation_error(
                format!(
                    "Missing required attribute '{}' on element '{}'",
                    attr_name_str,
                    elem.local_name()
                ),
                None,
            )?;
        }

        if attr_decl.is_prohibited() && value.is_some() {
            context.validation_error(
                format!(
                    "Prohibited attribute '{}' present on element '{}'",
                    attr_name_str,
                    elem.local_name()
                ),
                None,
            )?;
        }

        // Validate attribute value if present
        if let Some(val) = value {
            // Check for fixed value
            if let Some(fixed) = attr_decl.fixed_value() {
                if val != fixed {
                    context.validation_error(
                        format!(
                            "Attribute '{}' has fixed value '{}' but contains '{}'",
                            attr_name_str, fixed, val
                        ),
                        None,
                    )?;
                }
            }

            // Validate against type
            if let Some(simple_type) = attr_decl.simple_type() {
                if let Err(e) = simple_type.validate_value(val) {
                    context.validation_error(
                        format!("Invalid value for attribute '{}': {}", attr_name_str, val),
                        Some(e.to_string()),
                    )?;
                }
            }
        }
    }

    // Check for unknown attributes (unless wildcard allows them)
    for (attr_qname, _value) in &elem.attributes {
        let attr_name = &attr_qname.local_name;

        // Skip xsi: namespace attributes
        if attr_qname.namespace.as_deref() == Some(XSI_NAMESPACE) {
            continue;
        }

        // Skip xmlns declarations
        if attr_name == "xmlns" || attr_name.starts_with("xmlns:") {
            continue;
        }

        if !validated_attrs.contains(attr_name) {
            // For now, just warn about unknown attributes in strict mode
            // A full implementation would check anyAttribute wildcards
            context.validation_error(
                format!(
                    "Unknown attribute '{}' on element '{}'",
                    attr_name,
                    elem.local_name()
                ),
                None,
            )?;
        }
    }

    Ok(())
}

/// Helper to resolve element QName from an XML element
fn resolve_element_qname(elem: &Element, schema: &XsdSchema) -> QName {
    let local_name = elem.local_name();

    // Check for namespace on element
    if let Some(ns) = elem.namespace() {
        QName::namespaced(ns, local_name)
    } else if let Some(default_ns) = elem.namespaces.get_default_namespace() {
        QName::namespaced(default_ns, local_name)
    } else if let Some(target_ns) = &schema.target_namespace {
        QName::namespaced(target_ns, local_name)
    } else {
        QName::local(local_name)
    }
}

/// Get xsi: attribute value
fn get_xsi_attribute<'a>(elem: &'a Element, local_name: &str) -> Option<&'a str> {
    for (qname, value) in &elem.attributes {
        if qname.local_name == local_name {
            if let Some(ref ns) = qname.namespace {
                if ns == XSI_NAMESPACE {
                    return Some(value);
                }
            }
        }
    }

    // Also check without namespace (common in documents)
    elem.get_attribute(local_name)
}

/// Find element declaration in the current content model
fn find_element_in_visitor(visitor: &ModelVisitor, qname: &QName) -> Option<Arc<XsdElement>> {
    visitor.find_element_decl(qname)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validators::base::ValidationMode;
    use crate::validators::simple_types::XsdAtomicType;
    use crate::validators::builtins::XSD_STRING;
    use std::sync::Arc;

    fn create_test_schema() -> XsdSchema {
        let mut schema = XsdSchema::new();
        schema.register_builtins().unwrap();
        schema
    }

    #[test]
    fn test_validate_empty_document() {
        let schema = create_test_schema();
        let doc = Document::new();
        let mut context = ValidationContext::new().with_mode(ValidationMode::Lax);

        let result = validate_document(&schema, &doc, &mut context);
        assert!(result.is_err() || context.has_errors());
    }

    #[test]
    fn test_validate_simple_element() {
        let mut schema = create_test_schema();

        // Create a simple string element
        let string_type = Arc::new(XsdAtomicType::new(XSD_STRING).unwrap());
        let elem_decl = Arc::new(XsdElement::simple(
            QName::local("name"),
            string_type,
        ));
        schema.maps.global_maps.elements.insert(QName::local("name"), elem_decl);

        // Create document
        let xml = r#"<name>John</name>"#;
        let doc = Document::from_string(xml).unwrap();

        let mut context = ValidationContext::new().with_mode(ValidationMode::Lax);
        let result = validate_document(&schema, &doc, &mut context);

        assert!(result.is_ok());
        assert!(!context.has_errors(), "Errors: {:?}", context.errors);
    }

    #[test]
    fn test_validate_unknown_root() {
        let schema = create_test_schema();

        let xml = r#"<unknown>test</unknown>"#;
        let doc = Document::from_string(xml).unwrap();

        let mut context = ValidationContext::new().with_mode(ValidationMode::Lax);
        let _result = validate_document(&schema, &doc, &mut context);

        assert!(context.has_errors());
    }
}
