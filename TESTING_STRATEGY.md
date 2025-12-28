# Testing Strategy for xmlschema-rs

## Overview

This document outlines the comprehensive testing strategy for validating the Rust port against the Python reference implementation.

## Test Categories

### 1. Real-World Schema Validation Tests

#### DITA (Darwin Information Typing Architecture)
- **Purpose**: Industry-standard technical documentation schema
- **Location**: `tests/schemas/dita/`
- **Strategy**:
  - Parse DITA XSD schemas with both Python and Rust implementations
  - Generate dictionary dumps of the parsed schema structure
  - Compare Python vs Rust dumps for exact equivalence
  - Validate sample DITA documents against the schema

#### NISO (National Information Standards Organization)
- **Purpose**: Publishing and library standards schemas
- **Location**: `tests/schemas/niso/`
- **Strategy**:
  - Parse NISO XSD schemas with both Python and Rust implementations
  - Generate dictionary dumps of the parsed schema structure
  - Compare Python vs Rust dumps for exact equivalence
  - Validate sample NISO documents against the schema

**Comparison Method**:
```rust
// Generate schema dumps for comparison
let python_dump = run_python_xmlschema_dump("schema.xsd");
let rust_dump = generate_rust_schema_dump("schema.xsd");
assert_eq!(python_dump, rust_dump, "Schema dumps must match exactly");
```

### 2. Python Test Suite Port

#### Unit Tests
- [ ] Port all Python unit tests from `tests/` directory
- [ ] Maintain 1:1 mapping where possible
- [ ] Adapt for Rust idioms where necessary
- [ ] Target: 100% test coverage of ported functionality

#### Test Organization
```
tests/
├── validation/          # Validation tests
│   ├── test_validation.rs
│   ├── test_decoding.rs
│   └── test_encoding.rs
├── test_documents.rs    # Document handling
├── test_namespaces.rs   # Namespace tests
├── test_converters.rs   # Converter tests
├── test_locations.rs    # Location resolution
├── test_loaders.rs      # Resource loading
└── schemas/             # Real-world schemas
    ├── dita/            # DITA schemas
    ├── niso/            # NISO schemas
    ├── vehicles/        # Example schemas from Python
    └── collection/      # Collection example
```

### 3. W3C XSD Conformance Tests

#### XSD 1.0 Test Suite
- **Source**: W3C XML Schema 1.0 Test Suite
- **Location**: `tests/w3c/xsd10/`
- **Goal**: Maximum conformance with spec
- **Tracking**: Maintain conformance percentage metrics

#### XSD 1.1 Test Suite
- **Source**: W3C XML Schema 1.1 Test Suite
- **Location**: `tests/w3c/xsd11/`
- **Goal**: Maximum conformance with spec
- **Tracking**: Maintain conformance percentage metrics

### 4. Property-Based Testing

Use `proptest` or `quickcheck` for:
- [ ] Random schema generation
- [ ] Random XML instance generation
- [ ] Fuzzing schema validation
- [ ] Roundtrip testing (encode → decode → encode)

### 5. Benchmark Tests

#### Performance Comparison
- [ ] Parse time: Python vs Rust
- [ ] Validation time: Python vs Rust
- [ ] Memory usage: Python vs Rust
- [ ] Conversion time: Python vs Rust

#### Benchmark Suites
```
benches/
├── parse_schema.rs      # Schema parsing benchmarks
├── validate_xml.rs      # Validation benchmarks
├── convert_data.rs      # Conversion benchmarks
└── large_documents.rs   # Large document handling
```

### 6. Integration Tests

#### End-to-End Scenarios
- [ ] Load schema from URL
- [ ] Validate document
- [ ] Convert to JSON
- [ ] Convert back to XML
- [ ] Verify equivalence

#### Example-Based Tests
Port examples from Python package:
- [ ] vehicles.xsd example
- [ ] collection.xsd example
- [ ] Custom user examples

## Test Data Organization

```
tests/
├── test_cases/          # Ported from Python
│   ├── examples/
│   │   ├── vehicles/
│   │   │   ├── vehicles.xsd
│   │   │   ├── vehicles.xml
│   │   │   └── vehicles-1_error.xml
│   │   └── collection/
│   │       ├── collection.xsd
│   │       └── collection.xml
│   └── schemas/         # Test schemas
│
├── schemas/             # Real-world schemas
│   ├── dita/
│   │   ├── *.xsd        # DITA schema files
│   │   ├── samples/     # Sample DITA documents
│   │   └── dumps/       # Schema dumps for comparison
│   │       ├── python/  # Python-generated dumps
│   │       └── rust/    # Rust-generated dumps
│   └── niso/
│       ├── *.xsd        # NISO schema files
│       ├── samples/     # Sample NISO documents
│       └── dumps/       # Schema dumps for comparison
│           ├── python/  # Python-generated dumps
│           └── rust/    # Rust-generated dumps
│
└── w3c/                 # W3C conformance tests
    ├── xsd10/
    └── xsd11/
```

## Schema Dump Comparison

### Python Dump Generator
```python
# tests/dump_schema.py
import xmlschema
import json
import sys

def dump_schema(schema_path):
    """Generate a normalized dictionary dump of a schema."""
    schema = xmlschema.XMLSchema(schema_path)

    dump = {
        'target_namespace': schema.target_namespace,
        'elements': {name: dump_element(elem)
                     for name, elem in schema.elements.items()},
        'types': {name: dump_type(typ)
                  for name, typ in schema.types.items()},
        'attributes': {name: dump_attribute(attr)
                       for name, attr in schema.attributes.items()},
        'groups': {name: dump_group(grp)
                   for name, grp in schema.groups.items()},
        # ... more components
    }

    return json.dumps(dump, sort_keys=True, indent=2)

if __name__ == '__main__':
    schema_path = sys.argv[1]
    print(dump_schema(schema_path))
```

### Rust Dump Generator
```rust
// tests/dump_schema.rs
use xmlschema::Schema;
use serde_json;

pub fn dump_schema(schema_path: &str) -> String {
    let schema = Schema::from_file(schema_path).unwrap();

    let dump = SchemaDump {
        target_namespace: schema.target_namespace().map(|s| s.to_string()),
        elements: schema.elements().iter()
            .map(|(name, elem)| (name.clone(), dump_element(elem)))
            .collect(),
        types: schema.types().iter()
            .map(|(name, typ)| (name.clone(), dump_type(typ)))
            .collect(),
        // ... more components
    };

    serde_json::to_string_pretty(&dump).unwrap()
}
```

### Comparison Test
```rust
#[test]
fn test_dita_schema_equivalence() {
    let schema_path = "tests/schemas/dita/technicalContent/dtd/topic.xsd";

    // Generate Python dump
    let python_dump = std::process::Command::new("python")
        .args(&["tests/dump_schema.py", schema_path])
        .output()
        .expect("Failed to run Python dump");

    // Generate Rust dump
    let rust_dump = dump_schema(schema_path);

    // Compare
    let python_json: serde_json::Value =
        serde_json::from_slice(&python_dump.stdout).unwrap();
    let rust_json: serde_json::Value =
        serde_json::from_str(&rust_dump).unwrap();

    assert_eq!(python_json, rust_json,
               "DITA schema dumps must match between Python and Rust");
}
```

## Continuous Integration

### CI Pipeline
```yaml
# .github/workflows/test.yml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Install Rust
        uses: actions-rs/toolchain@v1
      - name: Install Python
        uses: actions/setup-python@v2
        with:
          python-version: '3.9'
      - name: Install Python xmlschema
        run: pip install xmlschema
      - name: Run tests
        run: cargo test --all-features
      - name: Run schema comparison tests
        run: cargo test --test schema_comparison
      - name: Run W3C conformance tests
        run: cargo test --test w3c_conformance
```

## Conformance Tracking

### Metrics to Track
- [ ] W3C XSD 1.0 conformance percentage
- [ ] W3C XSD 1.1 conformance percentage
- [ ] Schema dump match rate (DITA, NISO)
- [ ] Test coverage percentage
- [ ] Performance vs Python (speedup factor)

### Reporting
Generate conformance reports:
```
XSD 1.0 Conformance: 95.2% (1234/1296 tests passing)
XSD 1.1 Conformance: 87.3% (892/1021 tests passing)
DITA Schema Match: 100%
NISO Schema Match: 100%
Test Coverage: 92.1%
Performance: 15.3x faster than Python
```

## Testing Checklist

### Per Module
- [ ] Unit tests for all public functions
- [ ] Integration tests for module interactions
- [ ] Error case testing
- [ ] Edge case testing
- [ ] Documentation examples as tests (doctest equivalent)

### Per Feature
- [ ] Happy path tests
- [ ] Error handling tests
- [ ] Performance benchmarks
- [ ] Comparison with Python behavior
- [ ] Real-world schema tests (DITA/NISO)

### Pre-Release
- [ ] All unit tests passing
- [ ] All integration tests passing
- [ ] W3C conformance > 90%
- [ ] DITA/NISO schema dumps match 100%
- [ ] Performance benchmarks documented
- [ ] Memory leak tests passing
- [ ] Fuzzing completed without crashes
- [ ] Documentation examples working

## Test Development Workflow

1. **Before implementing a feature**:
   - Write failing tests based on Python behavior
   - Document expected behavior from XSD spec

2. **During implementation**:
   - Run tests frequently
   - Compare behavior with Python reference
   - Generate schema dumps for complex cases

3. **After implementation**:
   - Ensure all tests pass
   - Add edge case tests
   - Verify performance benchmarks
   - Update conformance metrics

## Future Enhancements

- [ ] Automated test generation from XSD spec
- [ ] Mutation testing for test quality
- [ ] Continuous fuzzing integration
- [ ] Performance regression detection
- [ ] Automated conformance reporting
