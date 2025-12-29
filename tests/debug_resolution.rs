use xmlschema::validators::XsdSchema;
use xmlschema::validators::globals::GlobalType;

#[test]
fn debug_resolution_after_build() {
    // Re-run after build to see if resolve worked
    let schema = XsdSchema::from_file("tests/comparison/schemas/book.xsd")
        .expect("Failed to parse XSD");
    
    // At this point, build() has been called
    // Check bookType attributes
    for (qname, global_type) in &schema.maps.global_maps.types {
        if qname.local_name == "bookType" {
            if let GlobalType::Complex(ct) = global_type {
                eprintln!("After build(), bookType has {} attributes", ct.attributes.iter_attributes().count());
                for attr in ct.attributes.iter_attributes() {
                    let type_info = attr.simple_type()
                        .map(|st| format!("{:?}", st.qualified_name_string()))
                        .unwrap_or_else(|| "None".to_string());
                    eprintln!("  Attribute '{}': type_name={:?}, simple_type={}", 
                        attr.name().local_name,
                        attr.type_name,
                        type_info
                    );
                }
            }
        }
    }
}
