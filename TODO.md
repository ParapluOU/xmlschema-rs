# xmlschema-rs: Rust Port of Python xmlschema Package

This document tracks the progress of porting the Python xmlschema package to Rust.

## Reference
- **Python Source**: [sissaschool/xmlschema](https://github.com/sissaschool/xmlschema)
- **License**: MIT
- **Python Package Stats**: 79 Python source files, ~50k+ lines of code

## Project Overview

The xmlschema library is an implementation of XML Schema (XSD 1.0 and XSD 1.1) that provides:
- XML Schema validation
- XML data decoding to native data structures and JSON
- XML data encoding from native data structures and JSON
- XPath-based schema navigation
- Protection against XML attacks
- URL/filesystem access control
- Offline schema caching

---

## Phase 1: Project Setup & Infrastructure ✓

### Project Initialization
- [x] Clone Python reference code to `python-xmlschema-reference/`
- [x] Initialize Rust cargo project
- [ ] Set up Cargo.toml with metadata and dependencies
- [ ] Create module structure matching Python architecture
- [ ] Set up CI/CD configuration
- [ ] Create comprehensive README.md
- [ ] Set up documentation infrastructure (rustdoc)

### Dependencies to Evaluate
- [ ] XML parsing: `quick-xml`, `roxmltree`, or `xml-rs`
- [ ] XPath support: `sxd-xpath` or custom implementation
- [ ] URL handling: `url` crate
- [ ] Date/time handling: `chrono`
- [ ] Decimal numbers: `rust_decimal`
- [ ] Error handling: `thiserror` or `anyhow`
- [ ] Regex: `regex`
- [ ] JSON support: `serde_json`

---

## Phase 2: Core Architecture Analysis

### Python Module Mapping (79 files total)

#### Top-Level Modules (xmlschema/)
- [ ] `__init__.py` → `lib.rs` - Main entry point and public API
- [ ] `exceptions.py` → `error.rs` - Error types and exception handling
- [ ] `namespaces.py` → `namespaces.rs` - XML namespace handling
- [ ] `names.py` → `names.rs` - XML name validation and utilities
- [ ] `aliases.py` → `aliases.rs` - Type aliases and shortcuts
- [ ] `arguments.py` → `arguments.rs` - Argument validation and processing
- [ ] `limits.py` / `_limits.py` → `limits.rs` - Schema limits and constraints
- [ ] `settings.py` → `settings.rs` - Global settings and configuration
- [ ] `translation.py` → `translation.rs` - Internationalization support

#### Document Handling
- [ ] `documents.py` → `documents.rs` - XML document validation and processing
- [ ] `loaders.py` → `loaders.rs` - Schema and document loading
- [ ] `locations.py` → `locations.rs` - Resource location resolution

#### Data Conversion
- [ ] `converters/` directory (needs analysis)
  - [ ] Converter base classes
  - [ ] Dictionary converters
  - [ ] JSON converters
  - [ ] Custom converter support
- [ ] `dataobjects.py` → `dataobjects.rs` - Data binding objects

#### XPath Support
- [ ] `xpath/` directory (needs analysis)
  - [ ] XPath expression evaluation
  - [ ] Schema navigation API
  - [ ] Element/attribute selection

#### Validators (validators/)
This is the core of the library - 21 Python files:

- [ ] `__init__.py` → `validators/mod.rs` - Validator module exports
- [ ] `xsdbase.py` → `validators/base.rs` - Base validator classes
- [ ] `schemas.py` → `validators/schemas.rs` - Schema validator (86KB, complex)
- [ ] `elements.py` → `validators/elements.rs` - Element validators (66KB)
- [ ] `attributes.py` → `validators/attributes.rs` - Attribute validators
- [ ] `simple_types.py` → `validators/simple_types.rs` - Simple type validators (63KB)
- [ ] `complex_types.py` → `validators/complex_types.rs` - Complex type validators (46KB)
- [ ] `groups.py` → `validators/groups.rs` - Model group validators (62KB)
- [ ] `facets.py` → `validators/facets.rs` - Facet validators (33KB)
- [ ] `models.py` → `validators/models.rs` - Content model validators (37KB)
- [ ] `wildcards.py` → `validators/wildcards.rs` - Wildcard validators (37KB)
- [ ] `identities.py` → `validators/identities.rs` - Identity constraints (21KB)
- [ ] `assertions.py` → `validators/assertions.rs` - XSD 1.1 assertions
- [ ] `builtins.py` → `validators/builtins.rs` - Built-in types
- [ ] `builders.py` → `validators/builders.rs` - Schema builders (32KB)
- [ ] `notations.py` → `validators/notations.rs` - Notation declarations
- [ ] `particles.py` → `validators/particles.rs` - Particle components
- [ ] `validation.py` → `validators/validation.rs` - Validation logic (28KB)
- [ ] `exceptions.py` → `validators/exceptions.rs` - Validator exceptions
- [ ] `helpers.py` → `validators/helpers.rs` - Validator utilities
- [ ] `xsd_globals.py` → `validators/globals.rs` - Global declarations (27KB)

#### Resources & Schemas
- [ ] `resources/` directory - Resource management
- [ ] `schemas/` directory - Built-in XSD schemas (13 subdirectories)
  - [ ] Bundle XSD 1.0 base schemas
  - [ ] Bundle XSD 1.1 base schemas
  - [ ] Include WSDL schemas
  - [ ] Include XML namespace schemas

#### Testing & Extras
- [ ] `testing/` directory - Test utilities
- [ ] `extras/` directory - Additional features
- [ ] `cli.py` → `cli.rs` - Command-line interface
- [ ] `exports.py` → `exports.rs` - Export functionality

#### Utilities
- [ ] `utils/` directory (needs analysis)
  - [ ] Common utilities
  - [ ] Helper functions

---

## Phase 3: Implementation Strategy

### Stage 1: Foundation (Weeks 1-4)
Priority: Critical path items

- [ ] **Error Types** - Complete error hierarchy
  - [ ] XMLSchemaException base
  - [ ] XMLSchemaValidationError
  - [ ] XMLSchemaParseError
  - [ ] XMLSchemaValueError
  - [ ] XMLSchemaTypeError
  - [ ] XMLSchemaKeyError
  - [ ] XMLSchemaEncodeError
  - [ ] XMLSchemaDecodeError

- [ ] **Namespace Handling**
  - [ ] Namespace struct
  - [ ] Qualified name (QName) handling
  - [ ] Namespace prefix mapping
  - [ ] Default namespace resolution

- [ ] **XML Name Validation**
  - [ ] NCName validation
  - [ ] QName validation
  - [ ] Name normalization

- [ ] **Resource Loading**
  - [ ] File loader
  - [ ] URL loader
  - [ ] String/bytes loader
  - [ ] Caching mechanism
  - [ ] Access control

### Stage 2: Core Validators (Weeks 5-12)
Priority: Core functionality

- [ ] **Base Validator Infrastructure**
  - [ ] Validator trait/base class
  - [ ] Validation context
  - [ ] Validation modes (strict/lax/skip)

- [ ] **Simple Type Validators**
  - [ ] String types
  - [ ] Numeric types (integer, decimal, float, double)
  - [ ] Boolean type
  - [ ] Date/time types
  - [ ] Binary types (base64, hex)
  - [ ] URI types
  - [ ] QName type
  - [ ] List types
  - [ ] Union types

- [ ] **Facet Validators**
  - [ ] length, minLength, maxLength
  - [ ] pattern (regex)
  - [ ] enumeration
  - [ ] whiteSpace
  - [ ] maxInclusive, maxExclusive
  - [ ] minInclusive, minExclusive
  - [ ] totalDigits, fractionDigits
  - [ ] assertions (XSD 1.1)

- [ ] **Complex Type Validators**
  - [ ] Simple content
  - [ ] Complex content
  - [ ] Mixed content
  - [ ] Extension/restriction derivation

- [ ] **Element Validators**
  - [ ] Element declarations
  - [ ] Element references
  - [ ] Substitution groups
  - [ ] Nillable elements
  - [ ] Default/fixed values

- [ ] **Attribute Validators**
  - [ ] Attribute declarations
  - [ ] Attribute references
  - [ ] Attribute groups
  - [ ] Default/fixed values
  - [ ] Attribute use (required/optional/prohibited)

### Stage 3: Schema Structure (Weeks 13-18)
Priority: Schema building

- [ ] **Model Groups**
  - [ ] Sequence compositor
  - [ ] Choice compositor
  - [ ] All compositor
  - [ ] Group references
  - [ ] Occurrence constraints (minOccurs/maxOccurs)

- [ ] **Content Models**
  - [ ] Deterministic finite automaton (DFA)
  - [ ] Model validation
  - [ ] Particle iteration
  - [ ] Emptiable checking

- [ ] **Wildcards**
  - [ ] Any element wildcard
  - [ ] Any attribute wildcard
  - [ ] Namespace constraints
  - [ ] Process contents (strict/lax/skip)

- [ ] **Schema Component**
  - [ ] Schema parsing
  - [ ] Component resolution
  - [ ] Include/import/redefine
  - [ ] Schema composition
  - [ ] Circular import detection

### Stage 4: Advanced Features (Weeks 19-24)
Priority: Advanced validation

- [ ] **Identity Constraints**
  - [ ] Unique constraints
  - [ ] Key constraints
  - [ ] Keyref constraints
  - [ ] Selector/field XPath evaluation

- [ ] **XSD 1.1 Features**
  - [ ] Assertions
  - [ ] Conditional type assignment
  - [ ] Open content
  - [ ] All compositor extensions
  - [ ] Override mechanism

- [ ] **Notations**
  - [ ] Notation declarations
  - [ ] Notation references

### Stage 5: Data Conversion (Weeks 25-30)
Priority: Usability features

- [ ] **Converters**
  - [ ] Converter trait
  - [ ] UnorderedConverter (HashMap-based)
  - [ ] OrderedConverter (Vec-based)
  - [ ] ColumnarConverter
  - [ ] JSONConverter
  - [ ] Custom converter support

- [ ] **Encoding**
  - [ ] Rust structs to XML
  - [ ] JSON to XML
  - [ ] Type coercion
  - [ ] Attribute handling
  - [ ] Namespace serialization

- [ ] **Decoding**
  - [ ] XML to Rust structs
  - [ ] XML to JSON
  - [ ] Type conversion
  - [ ] Attribute extraction
  - [ ] Namespace handling

- [ ] **Data Objects**
  - [ ] DataElement class equivalent
  - [ ] Dynamic data binding
  - [ ] Attribute access patterns

### Stage 6: XPath & Navigation (Weeks 31-35)
Priority: Schema introspection

- [ ] **XPath Integration**
  - [ ] XPath expression compilation
  - [ ] Schema context evaluation
  - [ ] Element/attribute selection
  - [ ] findall() implementation
  - [ ] find() implementation
  - [ ] iterfind() implementation

### Stage 7: Additional Features (Weeks 36-40)
Priority: Production readiness

- [ ] **Security**
  - [ ] XML bomb protection
  - [ ] Entity expansion limits
  - [ ] Billion laughs attack prevention
  - [ ] DTD restrictions
  - [ ] External entity restrictions

- [ ] **Settings & Configuration**
  - [ ] Global settings
  - [ ] Schema-level settings
  - [ ] Validation settings
  - [ ] Converter settings
  - [ ] Security settings

- [ ] **CLI Tool**
  - [ ] Validate command
  - [ ] Convert command
  - [ ] Inspect command
  - [ ] Download schemas command

- [ ] **Caching & Performance**
  - [ ] Schema caching
  - [ ] Built-in schema preloading
  - [ ] Validation optimization
  - [ ] Lazy loading strategies

---

## Phase 4: Testing & Validation

### Test Infrastructure
- [ ] Set up test framework
- [ ] Port Python unit tests
- [ ] Create Rust-specific tests
- [ ] Integration tests
- [ ] Benchmark suite

### W3C Conformance Tests
- [ ] XSD 1.0 test suite
- [ ] XSD 1.1 test suite
- [ ] Run conformance tests
- [ ] Track conformance percentage

### Example Cases
- [ ] Port example schemas from Python package
- [ ] vehicles.xsd example
- [ ] collection.xsd example
- [ ] Create Rust usage examples

---

## Phase 5: Documentation & Polish

### Documentation
- [ ] API documentation (rustdoc)
- [ ] User guide
- [ ] Migration guide from Python
- [ ] Examples and tutorials
- [ ] Architecture documentation

### Polish
- [ ] Ergonomic API design
- [ ] Builder patterns
- [ ] Error message quality
- [ ] Performance optimization
- [ ] Memory optimization

### Release Preparation
- [ ] Versioning strategy
- [ ] CHANGELOG.md
- [ ] Crates.io metadata
- [ ] License compliance
- [ ] Security audit

---

## Current Progress

**Overall**: ~0% complete
**Current Stage**: Phase 1 - Project Setup

### Recently Completed
- ✓ Cloned Python reference code
- ✓ Initialized Rust cargo project
- ✓ Created TODO tracking document

### In Progress
- Creating comprehensive TODO documentation

### Next Steps
1. Analyze Python module dependencies
2. Set up Cargo.toml dependencies
3. Create initial module structure
4. Implement error types
5. Implement namespace handling

---

## Implementation Notes

### Key Design Decisions

1. **XML Parser Choice**: TBD - evaluate quick-xml vs roxmltree
2. **Error Handling**: Use thiserror for error types
3. **Async Support**: Consider async/await for network operations
4. **API Style**: Builder patterns for complex objects
5. **Memory**: Minimize copying, use Arc/Rc where appropriate
6. **Compatibility**: Aim for similar API to Python where idiomatic

### Challenges to Address

1. **Python ElementTree → Rust**: Need efficient XML tree representation
2. **Dynamic typing**: Python's dynamic features vs Rust's static typing
3. **XPath implementation**: Complex feature, may need external crate
4. **Regex patterns**: XSD regex syntax differs from standard regex
5. **Content model validation**: DFA construction and validation
6. **Performance**: Ensure Rust version is faster than Python

### Rust Advantages to Leverage

1. **Type safety**: Catch errors at compile time
2. **Performance**: Much faster validation and conversion
3. **Memory safety**: No segfaults or memory leaks
4. **Concurrency**: Safe parallel validation
5. **Zero-cost abstractions**: Clean API without runtime overhead

---

## Resources

### Documentation
- XSD 1.0 Specification: https://www.w3.org/TR/xmlschema-1/
- XSD 1.1 Specification: https://www.w3.org/TR/xmlschema11-1/
- Python xmlschema docs: http://xmlschema.readthedocs.io/

### Similar Rust Projects
- serde-xml-rs: XML serialization/deserialization
- quick-xml: Fast XML parser
- roxmltree: DOM-like XML tree

---

## Session Notes

### Session 1 (2025-12-28)
- Cloned Python reference repository
- Initialized Rust project
- Created comprehensive TODO documentation
- Identified 79 Python source files to port
- Mapped Python modules to Rust architecture
