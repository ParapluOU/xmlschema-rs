use xmlschema::comparison::{
    format_qualified_name, AttributeInfo, ChildElementInfo, ElementInfo, RestrictionInfo,
    SchemaDump, SimpleTypeInfo, TypeInfo,
};
use xmlschema::validators::{
    ComplexContent, GlobalType, GroupParticle, SimpleType, XsdSchema,
};

fn main() {
    let xsd_path = "tests/comparison/schemas/book.xsd";
    
    // Load and dump Rust result
    let rust_schema = dump_schema_rust(xsd_path).expect("Failed to parse XSD with Rust");
    
    // Print as JSON
    println!("{}", serde_json::to_string_pretty(&rust_schema).unwrap());
}

fn dump_schema_rust(xsd_path: &str) -> Result<SchemaDump, String> {
    use xmlschema::validators::{ElementType, FormDefault};

    // Parse the XSD file
    let schema = XsdSchema::from_file(xsd_path)
        .map_err(|e| format!("Failed to parse XSD: {}", e))?;

    let target_ns = schema.target_namespace.clone();
    let maps = &schema.maps.global_maps;

    // Build dump structure
    let mut dump = SchemaDump {
        target_namespace: target_ns.clone(),
        schema_location: Some(xsd_path.to_string()),
        element_form_default: match schema.element_form_default {
            FormDefault::Qualified => Some("qualified".to_string()),
            FormDefault::Unqualified => Some("unqualified".to_string()),
        },
        root_elements: Vec::new(),
        complex_types: Vec::new(),
        simple_types: Vec::new(),
    };

    // Convert elements
    for (qname, elem) in &maps.elements {
        let type_info = match &elem.element_type {
            ElementType::Complex(ct) => {
                // Get type name
                let (type_name, type_qname) = if let Some(ref name) = ct.name {
                    let n = format_qualified_name(name.namespace.as_deref(), &name.local_name);
                    (Some(n.clone()), Some(n))
                } else {
                    (None, None)
                };

                // Get attributes
                let attrs: Vec<AttributeInfo> = ct
                    .attributes
                    .iter_attributes()
                    .map(|a| {
                        let attr_type = a
                            .simple_type()
                            .and_then(|st| st.qualified_name_string())
                            .unwrap_or_else(|| "{http://www.w3.org/2001/XMLSchema}string".to_string());
                        AttributeInfo {
                            name: a.name().local_name.clone(),
                            attr_type,
                            use_mode: format!("{:?}", a.use_mode()).to_lowercase(),
                            default: a.default().map(|s| s.to_string()),
                        }
                    })
                    .collect();

                // Get content model type
                let content_model = if ct.content.is_empty() {
                    None
                } else {
                    Some("XsdGroup".to_string())
                };

                // Extract child elements from content model
                let child_elements = if let ComplexContent::Group(ref group) = ct.content {
                    let mut children = Vec::new();
                    extract_child_elements(&group.particles, &mut children, &schema);
                    if children.is_empty() {
                        None
                    } else {
                        Some(children)
                    }
                } else {
                    None
                };

                Some(TypeInfo {
                    name: type_name,
                    qualified_name: type_qname,
                    category: "XsdComplexType".to_string(),
                    is_complex: true,
                    is_simple: false,
                    content_model,
                    attributes: if attrs.is_empty() { None } else { Some(attrs) },
                    child_elements,
                })
            },
            ElementType::Simple(st) => {
                let type_name = st.qualified_name_string();
                Some(TypeInfo {
                    name: type_name.clone(),
                    qualified_name: type_name,
                    category: "XsdAtomicType".to_string(),
                    is_complex: false,
                    is_simple: true,
                    content_model: None,
                    attributes: None,
                    child_elements: None,
                })
            },
            ElementType::Any => None,
        };

        let elem_name = format_qualified_name(qname.namespace.as_deref(), &qname.local_name);
        dump.root_elements.push(ElementInfo {
            name: elem_name.clone(),
            qualified_name: elem_name,
            element_type: type_info,
            min_occurs: elem.occurs.min,
            max_occurs: elem.occurs.max,
            nillable: elem.nillable,
            default: elem.default.clone(),
        });
    }

    // Convert types
    for (qname, global_type) in &maps.types {
        let type_name = format_qualified_name(qname.namespace.as_deref(), &qname.local_name);

        match global_type {
            GlobalType::Complex(ct) => {
                // Collect attributes with proper types
                let attrs: Vec<AttributeInfo> = ct
                    .attributes
                    .iter_attributes()
                    .map(|a| {
                        let attr_type = a
                            .simple_type()
                            .and_then(|st| st.qualified_name_string())
                            .unwrap_or_else(|| "{http://www.w3.org/2001/XMLSchema}string".to_string());
                        AttributeInfo {
                            name: a.name().local_name.clone(),
                            attr_type,
                            use_mode: format!("{:?}", a.use_mode()).to_lowercase(),
                            default: a.default().map(|s| s.to_string()),
                        }
                    })
                    .collect();

                let content_model = if ct.content.is_empty() {
                    None
                } else {
                    Some("XsdGroup".to_string())
                };

                // Extract child elements from content model
                let child_elements = if let ComplexContent::Group(ref group) = ct.content {
                    let mut children = Vec::new();
                    extract_child_elements(&group.particles, &mut children, &schema);
                    if children.is_empty() {
                        None
                    } else {
                        Some(children)
                    }
                } else {
                    None
                };

                dump.complex_types.push(TypeInfo {
                    name: Some(type_name.clone()),
                    qualified_name: Some(type_name),
                    category: "XsdComplexType".to_string(),
                    is_complex: true,
                    is_simple: false,
                    content_model,
                    attributes: if attrs.is_empty() { None } else { Some(attrs) },
                    child_elements,
                });
            }
            GlobalType::Simple(st) => {
                // Get facets for restrictions
                let facets = st.facets();
                let mut restrictions = Vec::new();

                // Check enumeration
                if let Some(ref enums) = facets.enumeration {
                    restrictions.push(RestrictionInfo {
                        kind: "Enumeration".to_string(),
                        value: None,
                        values: Some(enums.values.clone()),
                    });
                }

                // Get base type using the SimpleType trait
                let base_type = SimpleType::base_type(st.as_ref())
                    .and_then(|bt| bt.qualified_name_string());

                dump.simple_types.push(SimpleTypeInfo {
                    name: type_name.clone(),
                    qualified_name: type_name,
                    category: "XsdAtomicRestriction".to_string(),
                    base_type,
                    restrictions: if restrictions.is_empty() {
                        None
                    } else {
                        Some(restrictions)
                    },
                });
            }
        }
    }

    Ok(dump)
}

/// Helper to extract child elements from content model particles
fn extract_child_elements(
    particles: &[GroupParticle],
    children: &mut Vec<ChildElementInfo>,
    schema: &XsdSchema,
) {
    for particle in particles {
        match particle {
            GroupParticle::Element(ep) => {
                // Get element type from the particle's element declaration or schema lookup
                let element_type = if let Some(elem_decl) = ep.element() {
                    get_element_type_name(elem_decl, schema)
                } else if let Some(ref elem_ref) = ep.element_ref {
                    if let Some(elem) = schema.lookup_element(elem_ref) {
                        get_element_type_name(&elem, schema)
                    } else {
                        "unknown".to_string()
                    }
                } else {
                    if let Some(elem) = schema.lookup_element(&ep.name) {
                        get_element_type_name(&elem, schema)
                    } else {
                        "unknown".to_string()
                    }
                };

                children.push(ChildElementInfo {
                    name: format_qualified_name(ep.name.namespace.as_deref(), &ep.name.local_name),
                    element_type,
                    min_occurs: ep.occurs.min,
                    max_occurs: ep.occurs.max,
                });
            }
            GroupParticle::Group(nested) => {
                extract_child_elements(&nested.particles, children, schema);
            }
            GroupParticle::Any(_) => {}
        }
    }
}

fn get_element_type_name(elem: &xmlschema::validators::XsdElement, _schema: &XsdSchema) -> String {
    use xmlschema::validators::ElementType;

    match &elem.element_type {
        ElementType::Simple(st) => st.qualified_name_string().unwrap_or_else(|| "unknown".to_string()),
        ElementType::Complex(ct) => {
            if let Some(ref name) = ct.name {
                format_qualified_name(name.namespace.as_deref(), &name.local_name)
            } else {
                "XsdComplexType".to_string()
            }
        }
        ElementType::Any => "anyType".to_string(),
    }
}
