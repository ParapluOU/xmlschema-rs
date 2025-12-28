# Python xmlschema Module Analysis

This document provides detailed analysis of the Python xmlschema package structure for porting to Rust.

## Directory Structure

```
xmlschema/
├── __init__.py                 # Main exports and API
├── _limits.py                  # Internal limits
├── aliases.py                  # Type aliases
├── arguments.py                # Argument validation
├── cli.py                      # Command-line interface
├── dataobjects.py              # Data binding objects
├── documents.py                # Document validation
├── exceptions.py               # Exception classes
├── exports.py                  # Export functionality
├── limits.py                   # Public limits API
├── loaders.py                  # Resource loaders
├── locations.py                # Resource location
├── names.py                    # XML name handling
├── namespaces.py               # Namespace handling
├── settings.py                 # Configuration
├── translation.py              # i18n support
├── py.typed                    # Type hints marker
│
├── converters/                 # Data converters
│   ├── __init__.py
│   ├── columnar.py
│   ├── default.py
│   └── unordered.py
│
├── extras/                     # Additional features
│   └── codegen/                # Code generation
│       ├── __init__.py
│       ├── jinja2filters.py
│       └── templates/
│
├── locale/                     # Translations
│   ├── en/
│   ├── it/
│   ├── ru/
│   └── ...
│
├── resources/                  # Resource utilities
│   ├── __init__.py
│   ├── converters.py
│   └── ...
│
├── schemas/                    # Built-in XSD schemas
│   ├── XSD_1.0/               # W3C XML Schema 1.0
│   ├── XSD_1.1/               # W3C XML Schema 1.1
│   ├── WSDL/                  # WSDL schemas
│   ├── XML/                   # XML namespace schemas
│   └── ...
│
├── testing/                    # Test utilities
│   ├── __init__.py
│   ├── builders.py
│   ├── case_class.py
│   └── helpers.py
│
├── utils/                      # Utilities
│   ├── __init__.py
│   ├── codegen.py
│   └── ...
│
├── validators/                 # Core validators
│   ├── __init__.py            # Validator exports
│   ├── assertions.py          # XSD 1.1 assertions
│   ├── attributes.py          # Attribute validators
│   ├── builders.py            # Schema builders
│   ├── builtins.py            # Built-in types
│   ├── complex_types.py       # Complex type validators
│   ├── elements.py            # Element validators
│   ├── exceptions.py          # Validator exceptions
│   ├── facets.py              # Facet constraints
│   ├── groups.py              # Model groups
│   ├── helpers.py             # Helper functions
│   ├── identities.py          # Identity constraints
│   ├── models.py              # Content models
│   ├── notations.py           # Notations
│   ├── particles.py           # Particle components
│   ├── schemas.py             # Schema validator (main)
│   ├── simple_types.py        # Simple type validators
│   ├── validation.py          # Validation logic
│   ├── wildcards.py           # Wildcards (any)
│   ├── xsd_globals.py         # Global declarations
│   └── xsdbase.py             # Base validator classes
│
└── xpath/                      # XPath support
    ├── __init__.py
    └── ...
```

## Module Dependency Analysis

### Core Dependencies (Implement First)

1. **exceptions.py** → **error.rs**
   - No internal dependencies
   - Base for all error handling
   - Priority: CRITICAL

2. **namespaces.py** → **namespaces.rs**
   - Depends on: exceptions
   - Used by: Almost everything
   - Priority: CRITICAL

3. **names.py** → **names.rs**
   - Depends on: exceptions, namespaces
   - Used by: validators, documents
   - Priority: CRITICAL

4. **_limits.py** + **limits.py** → **limits.rs**
   - Depends on: exceptions
   - Used by: validators, loaders
   - Priority: HIGH

### Resource Handling (Second Tier)

5. **locations.py** → **locations.rs**
   - Depends on: exceptions, namespaces
   - Used by: loaders, documents
   - Priority: HIGH

6. **loaders.py** → **loaders.rs**
   - Depends on: exceptions, locations, limits
   - Used by: documents, validators
   - Priority: HIGH

7. **documents.py** → **documents.rs**
   - Depends on: exceptions, namespaces, loaders
   - Used by: Main API
   - Priority: HIGH

### Validator Infrastructure (Third Tier)

8. **validators/xsdbase.py** → **validators/base.rs**
   - Depends on: exceptions, namespaces, names
   - Used by: All validators
   - Priority: CRITICAL

9. **validators/exceptions.py** → **validators/exceptions.rs**
   - Depends on: main exceptions
   - Used by: All validators
   - Priority: CRITICAL

10. **validators/helpers.py** → **validators/helpers.rs**
    - Depends on: base, exceptions
    - Used by: All validators
    - Priority: HIGH

### Type System (Fourth Tier)

11. **validators/builtins.py** → **validators/builtins.rs**
    - Depends on: base, exceptions, facets
    - Defines primitive XSD types
    - Priority: CRITICAL

12. **validators/facets.py** → **validators/facets.rs**
    - Depends on: base, exceptions
    - Used by: simple_types, complex_types
    - Priority: CRITICAL

13. **validators/simple_types.py** → **validators/simple_types.rs**
    - Depends on: base, builtins, facets
    - Used by: elements, attributes, complex_types
    - Priority: CRITICAL

14. **validators/attributes.py** → **validators/attributes.rs**
    - Depends on: base, simple_types
    - Used by: complex_types, elements
    - Priority: HIGH

### Structural Components (Fifth Tier)

15. **validators/particles.py** → **validators/particles.rs**
    - Depends on: base
    - Used by: groups, elements
    - Priority: HIGH

16. **validators/wildcards.py** → **validators/wildcards.rs**
    - Depends on: base, particles
    - Used by: groups, complex_types
    - Priority: MEDIUM

17. **validators/groups.py** → **validators/groups.rs**
    - Depends on: base, particles, wildcards
    - Used by: complex_types, elements
    - Priority: HIGH

18. **validators/models.py** → **validators/models.rs**
    - Depends on: base, groups, particles
    - Used by: complex_types
    - Priority: HIGH

19. **validators/complex_types.py** → **validators/complex_types.rs**
    - Depends on: base, simple_types, attributes, groups, models
    - Used by: elements
    - Priority: HIGH

20. **validators/elements.py** → **validators/elements.rs**
    - Depends on: base, complex_types, simple_types
    - Used by: schemas
    - Priority: HIGH

### Advanced Features (Sixth Tier)

21. **validators/identities.py** → **validators/identities.rs**
    - Depends on: base, elements
    - Used by: schemas, validation
    - Priority: MEDIUM

22. **validators/assertions.py** → **validators/assertions.rs**
    - Depends on: base, xpath
    - XSD 1.1 only
    - Priority: LOW

23. **validators/notations.py** → **validators/notations.rs**
    - Depends on: base
    - Priority: LOW

### Schema Components (Seventh Tier)

24. **validators/xsd_globals.py** → **validators/globals.rs**
    - Depends on: base, elements, attributes, groups
    - Used by: schemas
    - Priority: HIGH

25. **validators/builders.py** → **validators/builders.rs**
    - Depends on: base, all component validators
    - Used by: schemas
    - Priority: HIGH

26. **validators/schemas.py** → **validators/schemas.rs**
    - Depends on: Everything in validators/
    - Main schema validator
    - Priority: CRITICAL (but implement last)

27. **validators/validation.py** → **validators/validation.rs**
    - Depends on: schemas, all validators
    - Validation orchestration
    - Priority: HIGH

### Data Conversion (Eighth Tier)

28. **converters/** → **converters/**
    - Depends on: validators, documents
    - Priority: MEDIUM

29. **dataobjects.py** → **dataobjects.rs**
    - Depends on: validators, converters
    - Priority: LOW

### XPath Support

30. **xpath/** → **xpath/**
    - Depends on: validators
    - Priority: MEDIUM (or use external crate)

### Utilities & Extras

31. **arguments.py** → **arguments.rs**
    - Argument validation utilities
    - Priority: MEDIUM

32. **settings.py** → **settings.rs**
    - Configuration management
    - Priority: MEDIUM

33. **aliases.py** → **aliases.rs**
    - Type aliases
    - Priority: LOW

34. **translation.py** → **translation.rs**
    - i18n support
    - Priority: LOW

35. **cli.py** → **cli.rs**
    - Command-line interface
    - Priority: LOW (implement last)

36. **exports.py** → **exports.rs**
    - Export utilities
    - Priority: LOW

## Implementation Order

Based on dependency analysis, implement in this order:

### Wave 1: Foundation (No dependencies)
1. error.rs (exceptions.py)
2. limits.rs (_limits.py + limits.py)

### Wave 2: Core Utilities (Depends on Wave 1)
3. namespaces.rs (namespaces.py)
4. names.rs (names.py)
5. locations.rs (locations.py)

### Wave 3: Resource Loading (Depends on Waves 1-2)
6. loaders.rs (loaders.py)
7. documents.rs (documents.py)

### Wave 4: Validator Foundation (Depends on Waves 1-3)
8. validators/mod.rs (validators/__init__.py)
9. validators/exceptions.rs (validators/exceptions.py)
10. validators/base.rs (validators/xsdbase.py)
11. validators/helpers.rs (validators/helpers.py)
12. validators/particles.rs (validators/particles.py)

### Wave 5: Type System (Depends on Wave 4)
13. validators/facets.rs (validators/facets.py)
14. validators/builtins.rs (validators/builtins.py)
15. validators/simple_types.rs (validators/simple_types.py)
16. validators/attributes.rs (validators/attributes.py)
17. validators/notations.rs (validators/notations.py)

### Wave 6: Complex Structures (Depends on Wave 5)
18. validators/wildcards.rs (validators/wildcards.py)
19. validators/groups.rs (validators/groups.py)
20. validators/models.rs (validators/models.py)
21. validators/complex_types.rs (validators/complex_types.py)
22. validators/elements.rs (validators/elements.py)

### Wave 7: Advanced Validation (Depends on Wave 6)
23. validators/identities.rs (validators/identities.py)
24. validators/globals.rs (validators/xsd_globals.py)
25. validators/builders.rs (validators/builders.py)
26. validators/schemas.rs (validators/schemas.py)
27. validators/validation.rs (validators/validation.py)

### Wave 8: XSD 1.1 Features (Optional)
28. validators/assertions.rs (validators/assertions.py)

### Wave 9: Data Conversion
29. converters/ (converters/)
30. dataobjects.rs (dataobjects.py)

### Wave 10: XPath
31. xpath/ (xpath/)

### Wave 11: Utilities & Extras
32. arguments.rs (arguments.py)
33. settings.rs (settings.py)
34. aliases.rs (aliases.py)
35. cli.rs (cli.py)

### Wave 12: Final Integration
36. lib.rs (__init__.py) - Main public API

## File Size Analysis

Large files that need careful planning:

1. **validators/schemas.py** - 86KB - Schema validator core
2. **validators/elements.py** - 66KB - Element validation
3. **validators/simple_types.py** - 63KB - Simple type validation
4. **validators/groups.py** - 62KB - Model group validation
5. **validators/complex_types.py** - 46KB - Complex type validation
6. **validators/wildcards.py** - 37KB - Wildcard validation
7. **validators/models.py** - 37KB - Content model validation
8. **documents.py** - 32KB - Document handling
9. **validators/builders.py** - 32KB - Schema building
10. **validators/facets.py** - 33KB - Facet validation

These large files should be split into multiple Rust modules for maintainability.

## External Dependencies (Python)

The Python package depends on:
- **elementpath** - XPath 2.0/3.0/3.1 implementation
- **elementTree** - Standard library XML parser

For Rust, we need equivalents:
- XML parsing: quick-xml, roxmltree, or xml-rs
- XPath: sxd-xpath or custom implementation

## Testing Strategy

The Python package has extensive tests:
- Unit tests for each module
- Integration tests
- W3C XSD test suite
- Example-based tests

We should port these incrementally as we implement each module.
