//! XML Schema validators
//!
//! This module contains the core validation logic for XML Schema.

// Wave 4: Foundation modules
pub mod base;
pub mod helpers;         // Helper utilities ✅
pub mod particles;       // Particle components ✅

// Wave 5: Type system
pub mod facets;       // Facet validators ✅
pub mod builtins;     // Built-in types ✅
pub mod simple_types; // Simple type validators ✅
pub mod attributes;   // Attribute validators ✅

// Wave 6: Complex structures
pub mod wildcards;       // Wildcard validators ✅
pub mod groups;          // Model group validators ✅
pub mod models;          // Content model validators ✅
pub mod complex_types;   // Complex type validators ✅
pub mod elements;        // Element validators ✅

// Wave 7: Advanced validation
pub mod identities;      // Identity constraints ✅
pub mod globals;         // Global declarations ✅
pub mod builders;        // Schema builders ✅
pub mod schemas;         // Schema validator (main) ✅

// Wave 8: XSD 1.1 and exceptions
pub mod assertions;      // XSD 1.1 assertions ✅
pub mod exceptions;      // Validation exceptions ✅

// Wave 9: XML validation
pub mod validation;      // Validation context and traits ✅

// Re-exports
pub use base::{
    AttributeValidator, ElementValidator, TypeValidator, ValidationMode, ValidationStatus,
    ValidityStatus, Validator, XsdValidator,
};
pub use facets::{
    EnumerationFacet, LengthFacet, MaxInclusiveFacet, MaxLengthFacet, MinInclusiveFacet,
    MinLengthFacet, NumericBound, PatternFacet, WhiteSpace,
};
pub use helpers::{
    base64_binary_validator, boolean_to_rust, byte_validator, decimal_validator,
    float_to_rust, hex_binary_validator, int_to_rust, int_validator, long_validator,
    negative_int_validator, non_negative_int_validator, non_positive_int_validator,
    positive_int_validator, qname_validator, rust_to_boolean, rust_to_float, rust_to_int,
    short_validator, unsigned_byte_validator, unsigned_int_validator, unsigned_long_validator,
    unsigned_short_validator, XSD_BOOLEAN_MAP,
};
pub use builtins::{
    get_builtin_type, validate_builtin, BuiltinType, TypeCategory, XsdValue,
    // Type constants
    XSD_NAMESPACE, XSD_STRING, XSD_NORMALIZED_STRING, XSD_TOKEN, XSD_LANGUAGE,
    XSD_NAME, XSD_NCNAME, XSD_ID, XSD_IDREF, XSD_IDREFS, XSD_ENTITY, XSD_ENTITIES,
    XSD_NMTOKEN, XSD_NMTOKENS, XSD_BOOLEAN, XSD_DECIMAL, XSD_INTEGER, XSD_LONG,
    XSD_INT, XSD_SHORT, XSD_BYTE, XSD_NON_NEGATIVE_INTEGER, XSD_POSITIVE_INTEGER,
    XSD_UNSIGNED_LONG, XSD_UNSIGNED_INT, XSD_UNSIGNED_SHORT, XSD_UNSIGNED_BYTE,
    XSD_NON_POSITIVE_INTEGER, XSD_NEGATIVE_INTEGER, XSD_FLOAT, XSD_DOUBLE,
    XSD_DURATION, XSD_DATETIME, XSD_TIME, XSD_DATE, XSD_GYEAR_MONTH, XSD_GYEAR,
    XSD_GMONTH_DAY, XSD_GDAY, XSD_GMONTH, XSD_HEX_BINARY, XSD_BASE64_BINARY,
    XSD_ANY_URI, XSD_QNAME, XSD_NOTATION, XSD_ANY_TYPE, XSD_ANY_SIMPLE_TYPE,
    XSD_ANY_ATOMIC_TYPE, XSD_ERROR,
    // Facet constants
    XSD_LENGTH, XSD_MIN_LENGTH, XSD_MAX_LENGTH, XSD_PATTERN, XSD_ENUMERATION,
    XSD_WHITE_SPACE, XSD_MAX_INCLUSIVE, XSD_MAX_EXCLUSIVE, XSD_MIN_INCLUSIVE,
    XSD_MIN_EXCLUSIVE, XSD_TOTAL_DIGITS, XSD_FRACTION_DIGITS, XSD_ASSERTION,
    XSD_EXPLICIT_TIMEZONE,
};
pub use simple_types::{
    FacetSet, SimpleType, SimpleTypeVariety, XsdAtomicType, XsdListType,
    XsdRestrictedType, XsdUnionType,
};
pub use attributes::{
    AttributeForm, AttributeScope, AttributeUse, XsdAttribute, XsdAttributeGroup,
};
pub use particles::{
    Occurs, OccursCalculator, OccursCounter, Particle, parse_occurs,
};
pub use wildcards::{
    NamespaceConstraint, ProcessContents, WildcardRef, XsdAnyAttribute, XsdAnyElement, XsdWildcard,
};
pub use groups::{
    ElementParticle, GroupParticle, ModelType, XsdGroup,
};
pub use models::{
    AdvanceYield, ContentItem, ContentKey, InterleavedModelVisitor, ModelVisitor,
    SuffixedModelVisitor, check_model, distinguishable_paths, sort_content,
};
pub use complex_types::{
    ComplexContent, ComplexTypeBuilder, ContentTypeLabel, DerivationFlags,
    DerivationMethod, OpenContentMode, XsdComplexType, XsdOpenContent,
};
pub use elements::{
    ElementForm, ElementScope, ElementType, XsdElement, XsdElementBuilder,
};
pub use identities::{
    FieldTuple, FieldValue, IdentityBuilder, IdentityConstraintKind, IdentityCounter,
    IdentityManager, IdentityMap, KeyrefCounter, XsdField, XsdIdentity, XsdSelector,
};
pub use globals::{
    AttributeGroupMap, AttributeMap, ElementMap, GlobalMaps, GlobalType, GroupMap,
    NotationMap, SubstitutionGroupMap, TypeMap, XsdGlobals, XsdNotation,
};
pub use builders::{
    BuildContext, StagedItem, StagedMap, XsdBuilders, XsdVersion,
};
pub use schemas::{
    DerivationDefault, FormDefault, NamespaceView, SchemaCollection, SchemaImport,
    SchemaInclude, SchemaSource, ValidationResult, XsdSchema,
    XML_NAMESPACE, XSI_NAMESPACE, VC_NAMESPACE,
};
pub use assertions::{
    AssertionList, XPathDefaultNamespace, XsdAssert,
};
pub use exceptions::{
    ChildrenValidationError, CircularityError, DecodeError, EncodeError,
    ModelDepthError, ModelError, NotBuiltError, StopValidation, ValidationError,
    XsdValidatorError,
};
pub use validation::{
    DecimalTypePreference, DecodeContext, EncodeContext, ValidationContext,
    ValidationOutcome, XmlDecoder, XmlEncoder, XmlValidator,
};

/// Type alias for backward compatibility
pub type Schema = XsdSchema;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_creation() {
        let _schema = Schema::new();
        assert!(!_schema.is_built());
    }
}
