use xmlschema::validators::XsdSchema;
use xmlschema::namespaces::QName;

#[test]
fn debug_type_lookup() {
    use xmlschema::validators::globals::GlobalType;
    
    let schema = XsdSchema::from_file("tests/comparison/schemas/book.xsd")
        .expect("Failed to parse XSD");
    
    // Check if isbnType is in global types
    let isbn_qname = QName::namespaced("http://example.com/book", "isbnType");
    eprintln!("Looking up: {:?}", isbn_qname);
    
    let found = schema.maps.global_maps.types.get(&isbn_qname);
    eprintln!("Found isbnType: {:?}", found.is_some());
    
    // List all simple types
    eprintln!("\nAll types in global_maps:");
    for (qname, gt) in &schema.maps.global_maps.types {
        match gt {
            GlobalType::Simple(st) => {
                eprintln!("  Simple: {:?} = {:?}", qname, st.qualified_name_string());
            }
            GlobalType::Complex(ct) => {
                eprintln!("  Complex: {:?}", qname);
            }
        }
    }
}
