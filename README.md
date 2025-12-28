# xmlschema-rs

A Rust implementation of XML Schema (XSD 1.0 and XSD 1.1) for validation and data conversion.

This library is a port of the Python [xmlschema](https://github.com/sissaschool/xmlschema) package, providing high-performance XML Schema validation and data conversion capabilities in Rust.

## Status

🚧 **Early Development** - This project is in active development. See [TODO.md](TODO.md) for current progress.

### Current Progress

- ✅ Project structure initialized
- ✅ Error handling infrastructure
- ✅ Limits and security checks
- ✅ Namespace handling (basic)
- ✅ XML name validation (basic)
- ✅ Resource loading (partial)
- 🚧 Validators (planned)
- 🚧 Data converters (planned)
- 🚧 XPath support (planned)

## Features (Planned)

- **Full XSD 1.0 Support** - Complete implementation of XML Schema 1.0
- **XSD 1.1 Support** - Support for XML Schema 1.1 features
- **XML Validation** - Validate XML documents against XSD schemas
- **Data Conversion** - Convert between XML, Rust structs, and JSON
- **XPath Navigation** - Schema introspection using XPath
- **Security** - Protection against XML attacks (billion laughs, XML bombs)
- **Performance** - High-performance validation leveraging Rust's speed
- **Safety** - Memory-safe implementation with compile-time guarantees

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
xmlschema = "0.1"
```

## Usage (Planned API)

```rust
use xmlschema::Schema;

// Load a schema
let schema = Schema::from_file("path/to/schema.xsd")?;

// Validate an XML document
let is_valid = schema.is_valid("path/to/document.xml")?;
println!("Valid: {}", is_valid);

// Validate and get detailed errors
if let Err(e) = schema.validate("path/to/document.xml") {
    eprintln!("Validation error: {}", e);
}

// Convert XML to Rust data structures
let data = schema.to_dict("path/to/document.xml")?;
println!("{:?}", data);

// Convert to JSON
let json = schema.to_json("path/to/document.xml")?;
println!("{}", json);
```

## Command-Line Interface

```bash
# Validate a document
xmlschema --schema schema.xsd --document data.xml --validate

# Convert to JSON
xmlschema --schema schema.xsd --document data.xml --json
```

## Architecture

The library is organized into several modules:

- **error** - Error types and handling
- **limits** - Security limits and resource constraints
- **namespaces** - XML namespace handling
- **names** - XML name validation
- **locations** - Resource location resolution
- **loaders** - Schema and document loading
- **documents** - XML document handling
- **validators** - Core validation logic (in development)
  - Simple types, complex types, elements, attributes
  - Model groups, wildcards, facets
  - Identity constraints
  - XSD 1.1 assertions
- **converters** - Data conversion (planned)
- **xpath** - XPath support (planned)

See [PYTHON_MODULES_ANALYSIS.md](PYTHON_MODULES_ANALYSIS.md) for detailed module mapping from Python.

## Testing Strategy

We use multiple testing approaches:

1. **Real-World Schemas**: DITA and NISO standard schemas for validation
2. **W3C Conformance**: XSD 1.0 and 1.1 test suites
3. **Python Comparison**: Schema dumps compared against Python reference
4. **Property-Based**: Fuzzing and random test generation
5. **Benchmarks**: Performance tracking

See [TESTING_STRATEGY.md](TESTING_STRATEGY.md) for details.

## Development Status

This is a complex, multi-stage port. See [TODO.md](TODO.md) for:
- Detailed implementation checklist
- Wave-based development plan
- Progress tracking across sessions
- Module-by-module status

### Implementation Waves

1. **Wave 1**: Foundation (error handling, limits) ✅
2. **Wave 2**: Core utilities (namespaces, names) ✅
3. **Wave 3**: Resource loading (loaders, documents) 🚧
4. **Wave 4**: Validator foundation 📋
5. **Wave 5**: Type system 📋
6. **Wave 6**: Complex structures 📋
7. **Wave 7**: Advanced validation 📋
8. **Wave 8**: XSD 1.1 features 📋
9. **Wave 9**: Data conversion 📋
10. **Wave 10**: XPath 📋
11. **Wave 11**: Utilities & extras 📋
12. **Wave 12**: Final integration 📋

## Reference Implementation

This is a port of the Python [xmlschema](https://github.com/sissaschool/xmlschema) package by Davide Brunato and contributors. The Python package is located in `python-xmlschema-reference/` for reference during development.

## Contributing

This project is in early development. Contributions are welcome once the core infrastructure is complete.

### Development Setup

```bash
# Clone the repository
git clone https://github.com/ParapluOU/xmlschema-rs
cd xmlschema-rs

# Build
cargo build

# Run tests
cargo test

# Run with CLI feature
cargo run --features cli -- --help

# Check code
cargo clippy

# Format code
cargo fmt
```

## Performance Goals

Target performance metrics (vs Python implementation):

- **Parsing**: 10-20x faster
- **Validation**: 15-30x faster
- **Conversion**: 10-15x faster
- **Memory**: 2-5x less memory usage

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

The Python reference implementation is also MIT licensed.

## Acknowledgments

- **Python xmlschema** by Davide Brunato and contributors
- **MaX (Materials design at the Exascale)** - Original project sponsor
- **W3C** - XML Schema specifications

## Resources

- [Python xmlschema Documentation](http://xmlschema.readthedocs.io/)
- [XML Schema 1.0 Specification](https://www.w3.org/TR/xmlschema-1/)
- [XML Schema 1.1 Specification](https://www.w3.org/TR/xmlschema11-1/)
- [W3C XML Schema Test Suite](https://www.w3.org/XML/Schema)

## Project Status

**Last Updated**: 2025-12-28

- **Python Reference**: Cloned and available
- **Rust Project**: Initialized with basic structure
- **Documentation**: Comprehensive TODO and planning docs
- **Testing**: Strategy defined, implementation pending
- **Implementation**: Waves 1-2 partially complete

See [TODO.md](TODO.md) for detailed progress across all modules.
