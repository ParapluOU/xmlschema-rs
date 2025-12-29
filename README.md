# xmlschema-rs

A Rust implementation of XML Schema (XSD 1.0 and XSD 1.1) for validation and data conversion.

This library is a port of the Python [xmlschema](https://github.com/sissaschool/xmlschema) package, providing high-performance XML Schema validation and data conversion capabilities in Rust.

## Status

**Active Development** - Core XSD parsing and validation infrastructure is complete. The library can parse complex XSD schemas and resolve forward references with Python parity.

### Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| XSD Parsing | ✅ Complete | Full XSD 1.0/1.1 schema parsing |
| Type System | ✅ Complete | Simple types, complex types, restrictions |
| Forward References | ✅ Complete | Full resolution of type/element references |
| Attributes | ✅ Complete | Attribute declarations and groups |
| Elements | ✅ Complete | Element declarations with type resolution |
| Content Models | ✅ Complete | Sequence, choice, all, groups |
| Facets | ✅ Complete | Enumeration, pattern, length, etc. |
| Document Validation | ✅ Complete | Validate XML against XSD |
| Data Converters | ✅ Complete | Parker, BadgerFish, Unordered |
| Schema Export | ✅ Complete | JSON export of schema structure |
| XPath Support | ✅ Complete | XPath for identity constraints |
| XSD 1.1 Assertions | ✅ Complete | Assert and report elements |
| Identity Constraints | ✅ Complete | Key, keyref, unique |
| HTTP/HTTPS Loading | 🚧 Partial | Local files work, HTTP pending |
| CLI Tool | 🚧 Partial | Basic structure, commands pending |

## Features

- **Full XSD 1.0 Support** - Complete implementation of XML Schema 1.0
- **XSD 1.1 Support** - Assertions, conditional type assignment
- **XML Validation** - Validate XML documents against XSD schemas
- **Data Conversion** - Convert between XML and JSON using multiple conventions
- **XPath Navigation** - Schema introspection and identity constraints
- **Security** - Protection against XML attacks (entity expansion limits)
- **Performance** - High-performance validation leveraging Rust's speed
- **Python Parity** - Schema introspection matches Python xmlschema output

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
xmlschema = { git = "https://github.com/ParapluOU/xmlschema-rs" }
```

## Usage

### Basic Schema Parsing

```rust
use xmlschema::validators::XsdSchema;

// Load a schema from file
let schema = XsdSchema::from_file("path/to/schema.xsd")?;

// Or from string
let schema = XsdSchema::from_string(xsd_content)?;

// Access schema information
println!("Target namespace: {:?}", schema.target_namespace);
println!("Elements: {}", schema.maps.global_maps.elements.len());
println!("Types: {}", schema.maps.global_maps.types.len());
```

### Document Validation

```rust
use xmlschema::validators::XsdSchema;
use xmlschema::documents::Document;

let schema = XsdSchema::from_file("schema.xsd")?;
let xml_content = std::fs::read_to_string("document.xml")?;
let doc = Document::from_string(&xml_content)?;

// Quick validity check
if schema.is_valid(&doc) {
    println!("Document is valid");
}

// Detailed validation with errors
let result = schema.validate(&doc);
if result.valid {
    println!("Valid!");
} else {
    for error in &result.errors {
        eprintln!("Validation error: {}", error);
    }
}
```

### Data Conversion

```rust
use xmlschema::converters::{ParkerConverter, BadgerFishConverter};

// Convert XML to JSON using Parker convention
let parker = ParkerConverter::new();
let json = parker.decode(&element_data)?;

// Or using BadgerFish convention
let badgerfish = BadgerFishConverter::new();
let json = badgerfish.decode(&element_data)?;
```

## Examples

The `examples/` directory contains runnable demonstrations of the library's features.

### Validation Example

Demonstrates validating XML documents against an XSD schema:

```bash
cargo run --example validate
```

Output:
```
Loading schema: examples/data/book.xsd
Schema loaded successfully!

Validating: examples/data/book_valid.xml
  Result: Document is valid!

Validating: examples/data/book_invalid.xml
  Result: Document is invalid!

Detailed validation of invalid document:
  - validation error: Invalid value for attribute 'isbn': invalid-isbn-format
```

### Schema Inspection Example

Demonstrates parsing and inspecting an XSD schema structure:

```bash
cargo run --example inspect_schema
```

Output:
```
=== Schema Inspection Example ===

Loading: examples/data/book.xsd

--- Schema Metadata ---
Target Namespace: Some("http://example.com/book")
Element Form Default: qualified

--- Component Counts ---
Global Elements: 1
Global Types: 4
Model Groups: 0

--- Global Elements ---
  - book

--- Global Types ---
  - isbnType (simple)
  - bookType (complex)
  - personType (complex)
  - emailType (simple)
```

### XML to JSON Conversion Example

Demonstrates converting XML to JSON using different conventions:

```bash
cargo run --example xml_to_json
```

Output (truncated):
```
=== XML to JSON Conversion Example ===

--- Default Convention ---
{
  "book": {
    "@isbn": "978-0-13-468599-1",
    "author": [
      { "email": "steve@example.com", "firstName": "Steve", "lastName": "Klabnik" },
      { "firstName": "Carol", "lastName": "Nichols" }
    ],
    "pages": "552",
    "title": "The Rust Programming Language"
  }
}

--- Parker Convention ---
(Simple element-to-value mapping, attributes may be lost)
...

--- BadgerFish Convention ---
(Preserves attributes with @ prefix, text with $ key)
...
```

### Example Data Files

The `examples/data/` directory contains sample files:

- `book.xsd` - XSD schema defining a book document structure
- `book_valid.xml` - Valid XML document conforming to the schema
- `book_invalid.xml` - Invalid XML document (invalid ISBN format)

## Architecture

The library is organized into these modules:

### Core Modules
- **error** - Error types and handling
- **limits** - Security limits and resource constraints
- **namespaces** - XML namespace handling with QName support
- **names** - XML name validation
- **documents** - XML document parsing and representation

### Validators (`validators/`)
- **schemas** - Main XsdSchema type with parsing and validation
- **simple_types** - Atomic, list, union, and restriction types
- **complex_types** - Complex type definitions with content models
- **elements** - Element declarations and particles
- **attributes** - Attribute declarations and groups
- **groups** - Model groups (sequence, choice, all)
- **facets** - Type facets (enumeration, pattern, length, etc.)
- **builtins** - Built-in XSD types (string, integer, date, etc.)
- **identities** - Identity constraints (key, keyref, unique)
- **assertions** - XSD 1.1 assertions
- **wildcards** - Any and anyAttribute wildcards
- **document_validation** - XML document validation logic
- **parsing** - XSD parsing from XML

### Converters (`converters/`)
- **parker** - Parker convention (simple element-to-value mapping)
- **badgerfish** - BadgerFish convention (preserves attributes)
- **unordered** - Unordered element handling

### XPath (`xpath/`)
- XPath expression evaluation for identity constraints

### Export (`exports.rs`)
- Schema export to JSON for comparison testing

## Testing

```bash
# Run all tests
cargo test

# Run comparison tests against Python
cargo test comparison

# Run with output
cargo test -- --nocapture
```

### Testing Strategy

1. **Comparison Testing** - Schema dumps compared against Python xmlschema
2. **Real-World Schemas** - DITA and NISO standard schema bundles
3. **Unit Tests** - Per-module functionality testing
4. **Integration Tests** - End-to-end validation scenarios

## Development

```bash
# Clone the repository
git clone https://github.com/ParapluOU/xmlschema-rs
cd xmlschema-rs

# Build
cargo build

# Run tests
cargo test

# Run examples
cargo run --example validate         # Document validation
cargo run --example inspect_schema   # Schema inspection
cargo run --example xml_to_json      # XML to JSON conversion
cargo run --example compare          # Compare with Python xmlschema

# Check code
cargo clippy

# Format code
cargo fmt
```

## Remaining Work

The following items are not yet complete:

- [ ] HTTP/HTTPS schema loading
- [ ] Full CLI implementation (validate, convert commands)
- [ ] xs:include/xs:import resolution across files
- [ ] Substitution groups
- [ ] Default/fixed value application during validation
- [ ] Full XSD 1.1 conditional type assignment

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- **Python xmlschema** by Davide Brunato and contributors
- **W3C** - XML Schema specifications

## Resources

- [Python xmlschema Documentation](http://xmlschema.readthedocs.io/)
- [XML Schema 1.0 Specification](https://www.w3.org/TR/xmlschema-1/)
- [XML Schema 1.1 Specification](https://www.w3.org/TR/xmlschema11-1/)

---

**Last Updated**: 2025-12-29
