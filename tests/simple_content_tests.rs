//! A `simpleContent` type's content is the text its base types, and the parser
//! has to record that.
//!
//! It did not: the `simpleContent` branch kept the derivation's `base` and its
//! attributes and left the content an empty model group, so
//! `<xs:simpleContent><xs:extension base="xs:string">…` — an element with text
//! and attributes, the commonest shape there is — read back with the content
//! type label `Empty`. Nothing refused such a document, because the validator
//! only looks for stray text in an `ElementOnly` type, so the mistake was
//! invisible to validation and plain to every other reader of the schema: an
//! editor asking "may text stand here?" was told no.

use xmlschema::validators::{ContentTypeLabel, GlobalType, XsdSchema};

/// The type of the element of that name, as a complex type.
fn complex_type_of(schema: &XsdSchema, element: &str) -> std::sync::Arc<xmlschema::validators::XsdComplexType> {
    let (_, declaration) = schema
        .elements()
        .find(|(qname, _)| qname.local_name == element)
        .unwrap_or_else(|| panic!("the schema declares {element}"));
    match &declaration.element_type {
        xmlschema::validators::ElementType::Complex(ct) => std::sync::Arc::clone(&ct),
        other => panic!("{element} is not of a complex type: {other:?}"),
    }
}

#[test]
fn an_anonymous_simple_content_extension_holds_text() {
    let schema = XsdSchema::from_string(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="step">
    <xs:complexType>
      <xs:simpleContent>
        <xs:extension base="xs:string">
          <xs:attribute name="minutes" type="xs:string"/>
        </xs:extension>
      </xs:simpleContent>
    </xs:complexType>
  </xs:element>
</xs:schema>"#,
    )
    .expect("the schema parses");
    let step = complex_type_of(&schema, "step");
    assert_eq!(step.content_type_label(), ContentTypeLabel::Simple, "step's content is its base's text");
    assert!(step.content.as_simple().is_some(), "and the simple type that types it is there");
    assert_eq!(step.attributes.len(), 1, "the extension's attribute is kept beside it");
}

#[test]
fn a_named_simple_content_extension_holds_text() {
    let schema = XsdSchema::from_string(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="word" type="WordType"/>
  <xs:complexType name="WordType">
    <xs:simpleContent>
      <xs:extension base="xs:string">
        <xs:attribute name="id" type="xs:string" use="required"/>
      </xs:extension>
    </xs:simpleContent>
  </xs:complexType>
</xs:schema>"#,
    )
    .expect("the schema parses");
    let word = complex_type_of(&schema, "word");
    assert_eq!(word.content_type_label(), ContentTypeLabel::Simple, "word's content is its base's text");
    assert!(word.content.as_simple().is_some(), "and the simple type that types it is there");
}

#[test]
fn a_complex_content_extension_is_left_alone() {
    // The base of a `complexContent` derivation is a complex type, and its
    // content stays a model group: this is the case the fix must not reach.
    let schema = XsdSchema::from_string(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="dish" type="DishType"/>
  <xs:complexType name="BaseType">
    <xs:sequence><xs:element name="title" type="xs:string"/></xs:sequence>
  </xs:complexType>
  <xs:complexType name="DishType">
    <xs:complexContent>
      <xs:extension base="BaseType">
        <xs:sequence><xs:element name="step" type="xs:string"/></xs:sequence>
      </xs:extension>
    </xs:complexContent>
  </xs:complexType>
</xs:schema>"#,
    )
    .expect("the schema parses");
    let dish = complex_type_of(&schema, "dish");
    assert_eq!(dish.content_type_label(), ContentTypeLabel::ElementOnly, "a complexContent extension holds elements");
    assert!(dish.content.as_simple().is_none(), "and no simple type types it");
}

#[test]
fn a_simple_content_restriction_of_a_complex_type_is_left_alone() {
    // Legal XSD: `simpleContent` may restrict a complex type that itself has
    // simple content. Its base is no simple type, so nothing is recorded here
    // rather than a guess at `xs:string`.
    let schema = XsdSchema::from_string(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="minutes" type="ShortType"/>
  <xs:complexType name="MeasureType">
    <xs:simpleContent>
      <xs:extension base="xs:string">
        <xs:attribute name="unit" type="xs:string"/>
      </xs:extension>
    </xs:simpleContent>
  </xs:complexType>
  <xs:complexType name="ShortType">
    <xs:simpleContent>
      <xs:restriction base="MeasureType">
        <xs:maxLength value="3"/>
      </xs:restriction>
    </xs:simpleContent>
  </xs:complexType>
</xs:schema>"#,
    )
    .expect("the schema parses");
    // Whatever label it carries, parsing it must not fail and must not claim a
    // simple type the schema never named.
    let minutes = complex_type_of(&schema, "minutes");
    assert!(minutes.content.as_simple().is_none(), "a complex base names no simple type: {:?}", minutes.content_type_label());
}
