//! XSD built-in types
//!
//! This module defines the built-in primitive and derived types for XML Schema.
//! These types form the foundation of XSD validation.

use crate::error::{Error, Result, ValidationError};
use crate::validators::facets::WhiteSpace;
use crate::validators::helpers::{
    base64_binary_validator, boolean_to_rust, byte_validator, decimal_validator,
    float_to_rust, hex_binary_validator, int_validator, long_validator,
    negative_int_validator, non_negative_int_validator, non_positive_int_validator,
    positive_int_validator, short_validator, unsigned_byte_validator,
    unsigned_int_validator, unsigned_long_validator, unsigned_short_validator,
};
use rust_decimal::Decimal;
use std::collections::HashSet;
use std::fmt;

// =============================================================================
// XSD Namespace Constants
// =============================================================================

/// XSD 1.0 Namespace
pub const XSD_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema";

// Type names - String types
/// XSD string type name
pub const XSD_STRING: &str = "string";
/// XSD normalizedString type name
pub const XSD_NORMALIZED_STRING: &str = "normalizedString";
/// XSD token type name
pub const XSD_TOKEN: &str = "token";
/// XSD language type name
pub const XSD_LANGUAGE: &str = "language";
/// XSD Name type name
pub const XSD_NAME: &str = "Name";
/// XSD NCName type name
pub const XSD_NCNAME: &str = "NCName";
/// XSD ID type name
pub const XSD_ID: &str = "ID";
/// XSD IDREF type name
pub const XSD_IDREF: &str = "IDREF";
/// XSD IDREFS type name
pub const XSD_IDREFS: &str = "IDREFS";
/// XSD ENTITY type name
pub const XSD_ENTITY: &str = "ENTITY";
/// XSD ENTITIES type name
pub const XSD_ENTITIES: &str = "ENTITIES";
/// XSD NMTOKEN type name
pub const XSD_NMTOKEN: &str = "NMTOKEN";
/// XSD NMTOKENS type name
pub const XSD_NMTOKENS: &str = "NMTOKENS";

/// XSD boolean type name
pub const XSD_BOOLEAN: &str = "boolean";

// Numeric types
/// XSD decimal type name
pub const XSD_DECIMAL: &str = "decimal";
/// XSD integer type name
pub const XSD_INTEGER: &str = "integer";
/// XSD long type name
pub const XSD_LONG: &str = "long";
/// XSD int type name
pub const XSD_INT: &str = "int";
/// XSD short type name
pub const XSD_SHORT: &str = "short";
/// XSD byte type name
pub const XSD_BYTE: &str = "byte";
/// XSD nonNegativeInteger type name
pub const XSD_NON_NEGATIVE_INTEGER: &str = "nonNegativeInteger";
/// XSD positiveInteger type name
pub const XSD_POSITIVE_INTEGER: &str = "positiveInteger";
/// XSD unsignedLong type name
pub const XSD_UNSIGNED_LONG: &str = "unsignedLong";
/// XSD unsignedInt type name
pub const XSD_UNSIGNED_INT: &str = "unsignedInt";
/// XSD unsignedShort type name
pub const XSD_UNSIGNED_SHORT: &str = "unsignedShort";
/// XSD unsignedByte type name
pub const XSD_UNSIGNED_BYTE: &str = "unsignedByte";
/// XSD nonPositiveInteger type name
pub const XSD_NON_POSITIVE_INTEGER: &str = "nonPositiveInteger";
/// XSD negativeInteger type name
pub const XSD_NEGATIVE_INTEGER: &str = "negativeInteger";

/// XSD float type name
pub const XSD_FLOAT: &str = "float";
/// XSD double type name
pub const XSD_DOUBLE: &str = "double";

// Date/time types
/// XSD duration type name
pub const XSD_DURATION: &str = "duration";
/// XSD dateTime type name
pub const XSD_DATETIME: &str = "dateTime";
/// XSD time type name
pub const XSD_TIME: &str = "time";
/// XSD date type name
pub const XSD_DATE: &str = "date";
/// XSD gYearMonth type name
pub const XSD_GYEAR_MONTH: &str = "gYearMonth";
/// XSD gYear type name
pub const XSD_GYEAR: &str = "gYear";
/// XSD gMonthDay type name
pub const XSD_GMONTH_DAY: &str = "gMonthDay";
/// XSD gDay type name
pub const XSD_GDAY: &str = "gDay";
/// XSD gMonth type name
pub const XSD_GMONTH: &str = "gMonth";

// Binary types
/// XSD hexBinary type name
pub const XSD_HEX_BINARY: &str = "hexBinary";
/// XSD base64Binary type name
pub const XSD_BASE64_BINARY: &str = "base64Binary";

// Other types
/// XSD anyURI type name
pub const XSD_ANY_URI: &str = "anyURI";
/// XSD QName type name
pub const XSD_QNAME: &str = "QName";
/// XSD NOTATION type name
pub const XSD_NOTATION: &str = "NOTATION";

// Special types
/// XSD anyType type name
pub const XSD_ANY_TYPE: &str = "anyType";
/// XSD anySimpleType type name
pub const XSD_ANY_SIMPLE_TYPE: &str = "anySimpleType";
/// XSD anyAtomicType type name (XSD 1.1)
pub const XSD_ANY_ATOMIC_TYPE: &str = "anyAtomicType";
/// XSD error type name (XSD 1.1)
pub const XSD_ERROR: &str = "error";

// =============================================================================
// Facet Names
// =============================================================================

/// XSD length facet name
pub const XSD_LENGTH: &str = "length";
/// XSD minLength facet name
pub const XSD_MIN_LENGTH: &str = "minLength";
/// XSD maxLength facet name
pub const XSD_MAX_LENGTH: &str = "maxLength";
/// XSD pattern facet name
pub const XSD_PATTERN: &str = "pattern";
/// XSD enumeration facet name
pub const XSD_ENUMERATION: &str = "enumeration";
/// XSD whiteSpace facet name
pub const XSD_WHITE_SPACE: &str = "whiteSpace";
/// XSD maxInclusive facet name
pub const XSD_MAX_INCLUSIVE: &str = "maxInclusive";
/// XSD maxExclusive facet name
pub const XSD_MAX_EXCLUSIVE: &str = "maxExclusive";
/// XSD minInclusive facet name
pub const XSD_MIN_INCLUSIVE: &str = "minInclusive";
/// XSD minExclusive facet name
pub const XSD_MIN_EXCLUSIVE: &str = "minExclusive";
/// XSD totalDigits facet name
pub const XSD_TOTAL_DIGITS: &str = "totalDigits";
/// XSD fractionDigits facet name
pub const XSD_FRACTION_DIGITS: &str = "fractionDigits";
/// XSD assertion facet name (XSD 1.1)
pub const XSD_ASSERTION: &str = "assertion";
/// XSD explicitTimezone facet name (XSD 1.1)
pub const XSD_EXPLICIT_TIMEZONE: &str = "explicitTimezone";

// =============================================================================
// Admitted Facets Sets
// =============================================================================

lazy_static::lazy_static! {
    /// Facets admitted for string types
    pub static ref STRING_FACETS: HashSet<&'static str> = {
        let mut s = HashSet::new();
        s.insert(XSD_LENGTH);
        s.insert(XSD_MIN_LENGTH);
        s.insert(XSD_MAX_LENGTH);
        s.insert(XSD_PATTERN);
        s.insert(XSD_ENUMERATION);
        s.insert(XSD_WHITE_SPACE);
        s.insert(XSD_ASSERTION);
        s
    };

    /// Facets admitted for boolean type
    pub static ref BOOLEAN_FACETS: HashSet<&'static str> = {
        let mut s = HashSet::new();
        s.insert(XSD_PATTERN);
        s.insert(XSD_WHITE_SPACE);
        s.insert(XSD_ASSERTION);
        s
    };

    /// Facets admitted for float/double types
    pub static ref FLOAT_FACETS: HashSet<&'static str> = {
        let mut s = HashSet::new();
        s.insert(XSD_PATTERN);
        s.insert(XSD_ENUMERATION);
        s.insert(XSD_WHITE_SPACE);
        s.insert(XSD_MAX_INCLUSIVE);
        s.insert(XSD_MAX_EXCLUSIVE);
        s.insert(XSD_MIN_INCLUSIVE);
        s.insert(XSD_MIN_EXCLUSIVE);
        s.insert(XSD_ASSERTION);
        s
    };

    /// Facets admitted for decimal types
    pub static ref DECIMAL_FACETS: HashSet<&'static str> = {
        let mut s = HashSet::new();
        s.insert(XSD_TOTAL_DIGITS);
        s.insert(XSD_FRACTION_DIGITS);
        s.insert(XSD_PATTERN);
        s.insert(XSD_ENUMERATION);
        s.insert(XSD_WHITE_SPACE);
        s.insert(XSD_MAX_INCLUSIVE);
        s.insert(XSD_MAX_EXCLUSIVE);
        s.insert(XSD_MIN_INCLUSIVE);
        s.insert(XSD_MIN_EXCLUSIVE);
        s.insert(XSD_ASSERTION);
        s
    };

    /// Facets admitted for datetime types
    pub static ref DATETIME_FACETS: HashSet<&'static str> = {
        let mut s = HashSet::new();
        s.insert(XSD_PATTERN);
        s.insert(XSD_ENUMERATION);
        s.insert(XSD_WHITE_SPACE);
        s.insert(XSD_MAX_INCLUSIVE);
        s.insert(XSD_MAX_EXCLUSIVE);
        s.insert(XSD_MIN_INCLUSIVE);
        s.insert(XSD_MIN_EXCLUSIVE);
        s.insert(XSD_ASSERTION);
        s.insert(XSD_EXPLICIT_TIMEZONE);
        s
    };
}

// =============================================================================
// XSD Value Representation
// =============================================================================

/// Represents any XSD atomic value
#[derive(Debug, Clone, PartialEq)]
pub enum XsdValue {
    /// String value
    String(String),
    /// Boolean value
    Boolean(bool),
    /// Decimal value
    Decimal(Decimal),
    /// Integer value
    Integer(i64),
    /// Float value
    Float(f64),
    /// Double value
    Double(f64),
    /// Binary value (hex or base64 decoded)
    Binary(Vec<u8>),
    /// Duration value (ISO 8601)
    Duration(String),
    /// DateTime value
    DateTime(String),
    /// Date value
    Date(String),
    /// Time value
    Time(String),
    /// URI value
    Uri(String),
    /// QName value (namespace, local)
    QName(Option<String>, String),
    /// Null/empty value
    Null,
}

impl fmt::Display for XsdValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            XsdValue::String(s) => write!(f, "{}", s),
            XsdValue::Boolean(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            XsdValue::Decimal(d) => write!(f, "{}", d),
            XsdValue::Integer(i) => write!(f, "{}", i),
            XsdValue::Float(v) | XsdValue::Double(v) => {
                if v.is_nan() {
                    write!(f, "NaN")
                } else if *v == f64::INFINITY {
                    write!(f, "INF")
                } else if *v == f64::NEG_INFINITY {
                    write!(f, "-INF")
                } else {
                    write!(f, "{}", v)
                }
            }
            XsdValue::Binary(b) => {
                // Hex encode for display
                for byte in b {
                    write!(f, "{:02X}", byte)?;
                }
                Ok(())
            }
            XsdValue::Duration(s)
            | XsdValue::DateTime(s)
            | XsdValue::Date(s)
            | XsdValue::Time(s)
            | XsdValue::Uri(s) => write!(f, "{}", s),
            XsdValue::QName(ns, local) => {
                if let Some(ns) = ns {
                    write!(f, "{{{}}}:{}", ns, local)
                } else {
                    write!(f, "{}", local)
                }
            }
            XsdValue::Null => write!(f, ""),
        }
    }
}

// =============================================================================
// Built-in Type Definition
// =============================================================================

/// Category of XSD type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeCategory {
    /// Primitive type (directly defined in XSD spec)
    Primitive,
    /// Derived type (derived from another type)
    Derived,
    /// Special type (anyType, anySimpleType, etc.)
    Special,
}

/// Definition of a built-in XSD type
#[derive(Debug, Clone)]
pub struct BuiltinType {
    /// Type name (local name without namespace)
    pub name: &'static str,
    /// Type category
    pub category: TypeCategory,
    /// Base type name (for derived types)
    pub base_type: Option<&'static str>,
    /// White space handling
    pub white_space: WhiteSpace,
    /// Admitted facets for this type
    pub admitted_facets: &'static HashSet<&'static str>,
    /// Validator function
    validator: fn(&str) -> Result<XsdValue>,
}

impl BuiltinType {
    /// Validate a string value against this type
    pub fn validate(&self, value: &str) -> Result<XsdValue> {
        // Apply white space normalization
        let normalized = self.white_space.normalize(value);
        (self.validator)(&normalized)
    }

    /// Check if this type is a numeric type
    pub fn is_numeric(&self) -> bool {
        matches!(
            self.name,
            XSD_DECIMAL
                | XSD_INTEGER
                | XSD_LONG
                | XSD_INT
                | XSD_SHORT
                | XSD_BYTE
                | XSD_NON_NEGATIVE_INTEGER
                | XSD_POSITIVE_INTEGER
                | XSD_UNSIGNED_LONG
                | XSD_UNSIGNED_INT
                | XSD_UNSIGNED_SHORT
                | XSD_UNSIGNED_BYTE
                | XSD_NON_POSITIVE_INTEGER
                | XSD_NEGATIVE_INTEGER
                | XSD_FLOAT
                | XSD_DOUBLE
        )
    }

    /// Check if this type is a string type
    pub fn is_string(&self) -> bool {
        matches!(
            self.name,
            XSD_STRING
                | XSD_NORMALIZED_STRING
                | XSD_TOKEN
                | XSD_LANGUAGE
                | XSD_NAME
                | XSD_NCNAME
                | XSD_ID
                | XSD_IDREF
                | XSD_ENTITY
                | XSD_NMTOKEN
        )
    }

    /// Check if this type is a date/time type
    pub fn is_datetime(&self) -> bool {
        matches!(
            self.name,
            XSD_DURATION
                | XSD_DATETIME
                | XSD_TIME
                | XSD_DATE
                | XSD_GYEAR_MONTH
                | XSD_GYEAR
                | XSD_GMONTH_DAY
                | XSD_GDAY
                | XSD_GMONTH
        )
    }
}

// =============================================================================
// Validator Functions
// =============================================================================

fn validate_string(value: &str) -> Result<XsdValue> {
    Ok(XsdValue::String(value.to_string()))
}

fn validate_normalized_string(value: &str) -> Result<XsdValue> {
    // Should not contain \r, \n, \t after normalization
    if value.contains(['\r', '\n', '\t']) {
        return Err(Error::Validation(ValidationError::new(
            "normalizedString cannot contain CR, LF, or TAB characters",
        )));
    }
    Ok(XsdValue::String(value.to_string()))
}

fn validate_token(value: &str) -> Result<XsdValue> {
    // Should be normalized and not have leading/trailing spaces or multiple consecutive spaces
    if value.starts_with(' ') || value.ends_with(' ') || value.contains("  ") {
        return Err(Error::Validation(ValidationError::new(
            "token cannot have leading/trailing spaces or consecutive spaces",
        )));
    }
    Ok(XsdValue::String(value.to_string()))
}

fn validate_language(value: &str) -> Result<XsdValue> {
    // Language code pattern: [a-zA-Z]{1,8}(-[a-zA-Z0-9]{1,8})*
    let re = regex::Regex::new(r"^[a-zA-Z]{1,8}(-[a-zA-Z0-9]{1,8})*$").unwrap();
    if !re.is_match(value) {
        return Err(Error::Validation(ValidationError::new(
            "invalid language code format",
        )));
    }
    Ok(XsdValue::String(value.to_string()))
}

fn validate_name(value: &str) -> Result<XsdValue> {
    if value.is_empty() {
        return Err(Error::Validation(ValidationError::new(
            "Name cannot be empty",
        )));
    }
    // Name must start with letter or underscore
    let first = value.chars().next().unwrap();
    if !first.is_alphabetic() && first != '_' && first != ':' {
        return Err(Error::Validation(ValidationError::new(
            "Name must start with a letter, underscore, or colon",
        )));
    }
    Ok(XsdValue::String(value.to_string()))
}

fn validate_ncname(value: &str) -> Result<XsdValue> {
    if value.is_empty() {
        return Err(Error::Validation(ValidationError::new(
            "NCName cannot be empty",
        )));
    }
    // NCName cannot contain colons
    if value.contains(':') {
        return Err(Error::Validation(ValidationError::new(
            "NCName cannot contain colons",
        )));
    }
    let first = value.chars().next().unwrap();
    if !first.is_alphabetic() && first != '_' {
        return Err(Error::Validation(ValidationError::new(
            "NCName must start with a letter or underscore",
        )));
    }
    Ok(XsdValue::String(value.to_string()))
}

fn validate_nmtoken(value: &str) -> Result<XsdValue> {
    if value.is_empty() {
        return Err(Error::Validation(ValidationError::new(
            "NMTOKEN cannot be empty",
        )));
    }
    // NMTOKEN can contain letters, digits, hyphens, underscores, periods, colons
    for c in value.chars() {
        if !c.is_alphanumeric() && c != '-' && c != '_' && c != '.' && c != ':' {
            return Err(Error::Validation(ValidationError::new(format!(
                "NMTOKEN contains invalid character: '{}'",
                c
            ))));
        }
    }
    Ok(XsdValue::String(value.to_string()))
}

fn validate_boolean(value: &str) -> Result<XsdValue> {
    let b = boolean_to_rust(value)?;
    Ok(XsdValue::Boolean(b))
}

fn validate_decimal(value: &str) -> Result<XsdValue> {
    let d = decimal_validator(value)?;
    Ok(XsdValue::Decimal(d))
}

fn validate_integer(value: &str) -> Result<XsdValue> {
    let i: i64 = value.trim().parse().map_err(|_| {
        Error::Validation(ValidationError::new("invalid integer value"))
    })?;
    Ok(XsdValue::Integer(i))
}

fn validate_long(value: &str) -> Result<XsdValue> {
    let i: i64 = value.trim().parse().map_err(|_| {
        Error::Validation(ValidationError::new("invalid long value"))
    })?;
    long_validator(i)?;
    Ok(XsdValue::Integer(i))
}

fn validate_int(value: &str) -> Result<XsdValue> {
    let i: i64 = value.trim().parse().map_err(|_| {
        Error::Validation(ValidationError::new("invalid int value"))
    })?;
    int_validator(i)?;
    Ok(XsdValue::Integer(i))
}

fn validate_short(value: &str) -> Result<XsdValue> {
    let i: i64 = value.trim().parse().map_err(|_| {
        Error::Validation(ValidationError::new("invalid short value"))
    })?;
    short_validator(i)?;
    Ok(XsdValue::Integer(i))
}

fn validate_byte(value: &str) -> Result<XsdValue> {
    let i: i64 = value.trim().parse().map_err(|_| {
        Error::Validation(ValidationError::new("invalid byte value"))
    })?;
    byte_validator(i)?;
    Ok(XsdValue::Integer(i))
}

fn validate_non_negative_integer(value: &str) -> Result<XsdValue> {
    let i: i64 = value.trim().parse().map_err(|_| {
        Error::Validation(ValidationError::new("invalid nonNegativeInteger value"))
    })?;
    non_negative_int_validator(i)?;
    Ok(XsdValue::Integer(i))
}

fn validate_positive_integer(value: &str) -> Result<XsdValue> {
    let i: i64 = value.trim().parse().map_err(|_| {
        Error::Validation(ValidationError::new("invalid positiveInteger value"))
    })?;
    positive_int_validator(i)?;
    Ok(XsdValue::Integer(i))
}

fn validate_unsigned_long(value: &str) -> Result<XsdValue> {
    let u: u64 = value.trim().parse().map_err(|_| {
        Error::Validation(ValidationError::new("invalid unsignedLong value"))
    })?;
    unsigned_long_validator(u)?;
    Ok(XsdValue::Integer(u as i64))
}

fn validate_unsigned_int(value: &str) -> Result<XsdValue> {
    let i: i64 = value.trim().parse().map_err(|_| {
        Error::Validation(ValidationError::new("invalid unsignedInt value"))
    })?;
    unsigned_int_validator(i)?;
    Ok(XsdValue::Integer(i))
}

fn validate_unsigned_short(value: &str) -> Result<XsdValue> {
    let i: i64 = value.trim().parse().map_err(|_| {
        Error::Validation(ValidationError::new("invalid unsignedShort value"))
    })?;
    unsigned_short_validator(i)?;
    Ok(XsdValue::Integer(i))
}

fn validate_unsigned_byte(value: &str) -> Result<XsdValue> {
    let i: i64 = value.trim().parse().map_err(|_| {
        Error::Validation(ValidationError::new("invalid unsignedByte value"))
    })?;
    unsigned_byte_validator(i)?;
    Ok(XsdValue::Integer(i))
}

fn validate_non_positive_integer(value: &str) -> Result<XsdValue> {
    let i: i64 = value.trim().parse().map_err(|_| {
        Error::Validation(ValidationError::new("invalid nonPositiveInteger value"))
    })?;
    non_positive_int_validator(i)?;
    Ok(XsdValue::Integer(i))
}

fn validate_negative_integer(value: &str) -> Result<XsdValue> {
    let i: i64 = value.trim().parse().map_err(|_| {
        Error::Validation(ValidationError::new("invalid negativeInteger value"))
    })?;
    negative_int_validator(i)?;
    Ok(XsdValue::Integer(i))
}

fn validate_float(value: &str) -> Result<XsdValue> {
    let f = float_to_rust(value)?;
    Ok(XsdValue::Float(f))
}

fn validate_double(value: &str) -> Result<XsdValue> {
    let f = float_to_rust(value)?;
    Ok(XsdValue::Double(f))
}

fn validate_hex_binary(value: &str) -> Result<XsdValue> {
    let bytes = hex_binary_validator(value)?;
    Ok(XsdValue::Binary(bytes))
}

fn validate_base64_binary(value: &str) -> Result<XsdValue> {
    let bytes = base64_binary_validator(value)?;
    Ok(XsdValue::Binary(bytes))
}

fn validate_any_uri(value: &str) -> Result<XsdValue> {
    // Basic URI validation - allow relative and absolute URIs
    // Just check for obviously invalid characters
    if value.contains(['\n', '\r', '\t']) {
        return Err(Error::Validation(ValidationError::new(
            "anyURI cannot contain newline or tab characters",
        )));
    }
    Ok(XsdValue::Uri(value.to_string()))
}

fn validate_qname(value: &str) -> Result<XsdValue> {
    if value.is_empty() {
        return Err(Error::Validation(ValidationError::new(
            "QName cannot be empty",
        )));
    }
    if let Some((prefix, local)) = value.split_once(':') {
        // Validate both parts as NCNames
        validate_ncname(prefix)?;
        validate_ncname(local)?;
        Ok(XsdValue::QName(Some(prefix.to_string()), local.to_string()))
    } else {
        validate_ncname(value)?;
        Ok(XsdValue::QName(None, value.to_string()))
    }
}

fn validate_duration(value: &str) -> Result<XsdValue> {
    // ISO 8601 duration: P[n]Y[n]M[n]DT[n]H[n]M[n]S
    let re = regex::Regex::new(
        r"^-?P(\d+Y)?(\d+M)?(\d+D)?(T(\d+H)?(\d+M)?(\d+(\.\d+)?S)?)?$"
    ).unwrap();
    if !re.is_match(value) || value == "P" || value == "-P" {
        return Err(Error::Validation(ValidationError::new(
            "invalid duration format",
        )));
    }
    Ok(XsdValue::Duration(value.to_string()))
}

fn validate_datetime(value: &str) -> Result<XsdValue> {
    // Basic dateTime validation: YYYY-MM-DDThh:mm:ss[.sss][Z|(+|-)hh:mm]
    let re = regex::Regex::new(
        r"^-?\d{4,}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})?$"
    ).unwrap();
    if !re.is_match(value) {
        return Err(Error::Validation(ValidationError::new(
            "invalid dateTime format",
        )));
    }
    Ok(XsdValue::DateTime(value.to_string()))
}

fn validate_date(value: &str) -> Result<XsdValue> {
    // Basic date validation: YYYY-MM-DD[Z|(+|-)hh:mm]
    let re = regex::Regex::new(
        r"^-?\d{4,}-\d{2}-\d{2}(Z|[+-]\d{2}:\d{2})?$"
    ).unwrap();
    if !re.is_match(value) {
        return Err(Error::Validation(ValidationError::new(
            "invalid date format",
        )));
    }
    Ok(XsdValue::Date(value.to_string()))
}

fn validate_time(value: &str) -> Result<XsdValue> {
    // Basic time validation: hh:mm:ss[.sss][Z|(+|-)hh:mm]
    let re = regex::Regex::new(
        r"^\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})?$"
    ).unwrap();
    if !re.is_match(value) {
        return Err(Error::Validation(ValidationError::new(
            "invalid time format",
        )));
    }
    Ok(XsdValue::Time(value.to_string()))
}

fn validate_gyear(value: &str) -> Result<XsdValue> {
    let re = regex::Regex::new(r"^-?\d{4,}(Z|[+-]\d{2}:\d{2})?$").unwrap();
    if !re.is_match(value) {
        return Err(Error::Validation(ValidationError::new(
            "invalid gYear format",
        )));
    }
    Ok(XsdValue::String(value.to_string()))
}

fn validate_gyear_month(value: &str) -> Result<XsdValue> {
    let re = regex::Regex::new(r"^-?\d{4,}-\d{2}(Z|[+-]\d{2}:\d{2})?$").unwrap();
    if !re.is_match(value) {
        return Err(Error::Validation(ValidationError::new(
            "invalid gYearMonth format",
        )));
    }
    Ok(XsdValue::String(value.to_string()))
}

fn validate_gmonth(value: &str) -> Result<XsdValue> {
    let re = regex::Regex::new(r"^--\d{2}(Z|[+-]\d{2}:\d{2})?$").unwrap();
    if !re.is_match(value) {
        return Err(Error::Validation(ValidationError::new(
            "invalid gMonth format",
        )));
    }
    Ok(XsdValue::String(value.to_string()))
}

fn validate_gday(value: &str) -> Result<XsdValue> {
    let re = regex::Regex::new(r"^---\d{2}(Z|[+-]\d{2}:\d{2})?$").unwrap();
    if !re.is_match(value) {
        return Err(Error::Validation(ValidationError::new(
            "invalid gDay format",
        )));
    }
    Ok(XsdValue::String(value.to_string()))
}

fn validate_gmonth_day(value: &str) -> Result<XsdValue> {
    let re = regex::Regex::new(r"^--\d{2}-\d{2}(Z|[+-]\d{2}:\d{2})?$").unwrap();
    if !re.is_match(value) {
        return Err(Error::Validation(ValidationError::new(
            "invalid gMonthDay format",
        )));
    }
    Ok(XsdValue::String(value.to_string()))
}

fn validate_any_type(_value: &str) -> Result<XsdValue> {
    // anyType accepts any content
    Ok(XsdValue::Null)
}

fn validate_error(_value: &str) -> Result<XsdValue> {
    Err(Error::Validation(ValidationError::new(
        "no value is allowed for xs:error type",
    )))
}

// =============================================================================
// Built-in Type Registry
// =============================================================================

lazy_static::lazy_static! {
    /// Registry of all built-in XSD types
    pub static ref BUILTIN_TYPES: Vec<BuiltinType> = vec![
        // Special types
        BuiltinType {
            name: XSD_ANY_TYPE,
            category: TypeCategory::Special,
            base_type: None,
            white_space: WhiteSpace::Preserve,
            admitted_facets: &STRING_FACETS,
            validator: validate_any_type,
        },
        BuiltinType {
            name: XSD_ANY_SIMPLE_TYPE,
            category: TypeCategory::Special,
            base_type: Some(XSD_ANY_TYPE),
            white_space: WhiteSpace::Preserve,
            admitted_facets: &STRING_FACETS,
            validator: validate_string,
        },
        BuiltinType {
            name: XSD_ERROR,
            category: TypeCategory::Special,
            base_type: None,
            white_space: WhiteSpace::Preserve,
            admitted_facets: &STRING_FACETS,
            validator: validate_error,
        },

        // Primitive string types
        BuiltinType {
            name: XSD_STRING,
            category: TypeCategory::Primitive,
            base_type: Some(XSD_ANY_SIMPLE_TYPE),
            white_space: WhiteSpace::Preserve,
            admitted_facets: &STRING_FACETS,
            validator: validate_string,
        },

        // Derived string types
        BuiltinType {
            name: XSD_NORMALIZED_STRING,
            category: TypeCategory::Derived,
            base_type: Some(XSD_STRING),
            white_space: WhiteSpace::Replace,
            admitted_facets: &STRING_FACETS,
            validator: validate_normalized_string,
        },
        BuiltinType {
            name: XSD_TOKEN,
            category: TypeCategory::Derived,
            base_type: Some(XSD_NORMALIZED_STRING),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &STRING_FACETS,
            validator: validate_token,
        },
        BuiltinType {
            name: XSD_LANGUAGE,
            category: TypeCategory::Derived,
            base_type: Some(XSD_TOKEN),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &STRING_FACETS,
            validator: validate_language,
        },
        BuiltinType {
            name: XSD_NAME,
            category: TypeCategory::Derived,
            base_type: Some(XSD_TOKEN),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &STRING_FACETS,
            validator: validate_name,
        },
        BuiltinType {
            name: XSD_NCNAME,
            category: TypeCategory::Derived,
            base_type: Some(XSD_NAME),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &STRING_FACETS,
            validator: validate_ncname,
        },
        BuiltinType {
            name: XSD_ID,
            category: TypeCategory::Derived,
            base_type: Some(XSD_NCNAME),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &STRING_FACETS,
            validator: validate_ncname,
        },
        BuiltinType {
            name: XSD_IDREF,
            category: TypeCategory::Derived,
            base_type: Some(XSD_NCNAME),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &STRING_FACETS,
            validator: validate_ncname,
        },
        BuiltinType {
            name: XSD_ENTITY,
            category: TypeCategory::Derived,
            base_type: Some(XSD_NCNAME),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &STRING_FACETS,
            validator: validate_ncname,
        },
        BuiltinType {
            name: XSD_NMTOKEN,
            category: TypeCategory::Derived,
            base_type: Some(XSD_TOKEN),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &STRING_FACETS,
            validator: validate_nmtoken,
        },

        // Boolean
        BuiltinType {
            name: XSD_BOOLEAN,
            category: TypeCategory::Primitive,
            base_type: Some(XSD_ANY_SIMPLE_TYPE),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &BOOLEAN_FACETS,
            validator: validate_boolean,
        },

        // Decimal types
        BuiltinType {
            name: XSD_DECIMAL,
            category: TypeCategory::Primitive,
            base_type: Some(XSD_ANY_SIMPLE_TYPE),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &DECIMAL_FACETS,
            validator: validate_decimal,
        },
        BuiltinType {
            name: XSD_INTEGER,
            category: TypeCategory::Derived,
            base_type: Some(XSD_DECIMAL),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &DECIMAL_FACETS,
            validator: validate_integer,
        },
        BuiltinType {
            name: XSD_LONG,
            category: TypeCategory::Derived,
            base_type: Some(XSD_INTEGER),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &DECIMAL_FACETS,
            validator: validate_long,
        },
        BuiltinType {
            name: XSD_INT,
            category: TypeCategory::Derived,
            base_type: Some(XSD_LONG),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &DECIMAL_FACETS,
            validator: validate_int,
        },
        BuiltinType {
            name: XSD_SHORT,
            category: TypeCategory::Derived,
            base_type: Some(XSD_INT),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &DECIMAL_FACETS,
            validator: validate_short,
        },
        BuiltinType {
            name: XSD_BYTE,
            category: TypeCategory::Derived,
            base_type: Some(XSD_SHORT),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &DECIMAL_FACETS,
            validator: validate_byte,
        },
        BuiltinType {
            name: XSD_NON_NEGATIVE_INTEGER,
            category: TypeCategory::Derived,
            base_type: Some(XSD_INTEGER),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &DECIMAL_FACETS,
            validator: validate_non_negative_integer,
        },
        BuiltinType {
            name: XSD_POSITIVE_INTEGER,
            category: TypeCategory::Derived,
            base_type: Some(XSD_NON_NEGATIVE_INTEGER),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &DECIMAL_FACETS,
            validator: validate_positive_integer,
        },
        BuiltinType {
            name: XSD_UNSIGNED_LONG,
            category: TypeCategory::Derived,
            base_type: Some(XSD_NON_NEGATIVE_INTEGER),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &DECIMAL_FACETS,
            validator: validate_unsigned_long,
        },
        BuiltinType {
            name: XSD_UNSIGNED_INT,
            category: TypeCategory::Derived,
            base_type: Some(XSD_UNSIGNED_LONG),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &DECIMAL_FACETS,
            validator: validate_unsigned_int,
        },
        BuiltinType {
            name: XSD_UNSIGNED_SHORT,
            category: TypeCategory::Derived,
            base_type: Some(XSD_UNSIGNED_INT),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &DECIMAL_FACETS,
            validator: validate_unsigned_short,
        },
        BuiltinType {
            name: XSD_UNSIGNED_BYTE,
            category: TypeCategory::Derived,
            base_type: Some(XSD_UNSIGNED_SHORT),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &DECIMAL_FACETS,
            validator: validate_unsigned_byte,
        },
        BuiltinType {
            name: XSD_NON_POSITIVE_INTEGER,
            category: TypeCategory::Derived,
            base_type: Some(XSD_INTEGER),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &DECIMAL_FACETS,
            validator: validate_non_positive_integer,
        },
        BuiltinType {
            name: XSD_NEGATIVE_INTEGER,
            category: TypeCategory::Derived,
            base_type: Some(XSD_NON_POSITIVE_INTEGER),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &DECIMAL_FACETS,
            validator: validate_negative_integer,
        },

        // Float types
        BuiltinType {
            name: XSD_FLOAT,
            category: TypeCategory::Primitive,
            base_type: Some(XSD_ANY_SIMPLE_TYPE),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &FLOAT_FACETS,
            validator: validate_float,
        },
        BuiltinType {
            name: XSD_DOUBLE,
            category: TypeCategory::Primitive,
            base_type: Some(XSD_ANY_SIMPLE_TYPE),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &FLOAT_FACETS,
            validator: validate_double,
        },

        // Binary types
        BuiltinType {
            name: XSD_HEX_BINARY,
            category: TypeCategory::Primitive,
            base_type: Some(XSD_ANY_SIMPLE_TYPE),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &STRING_FACETS,
            validator: validate_hex_binary,
        },
        BuiltinType {
            name: XSD_BASE64_BINARY,
            category: TypeCategory::Primitive,
            base_type: Some(XSD_ANY_SIMPLE_TYPE),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &STRING_FACETS,
            validator: validate_base64_binary,
        },

        // URI and QName types
        BuiltinType {
            name: XSD_ANY_URI,
            category: TypeCategory::Primitive,
            base_type: Some(XSD_ANY_SIMPLE_TYPE),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &STRING_FACETS,
            validator: validate_any_uri,
        },
        BuiltinType {
            name: XSD_QNAME,
            category: TypeCategory::Primitive,
            base_type: Some(XSD_ANY_SIMPLE_TYPE),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &STRING_FACETS,
            validator: validate_qname,
        },

        // Date/time types
        BuiltinType {
            name: XSD_DURATION,
            category: TypeCategory::Primitive,
            base_type: Some(XSD_ANY_SIMPLE_TYPE),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &DATETIME_FACETS,
            validator: validate_duration,
        },
        BuiltinType {
            name: XSD_DATETIME,
            category: TypeCategory::Primitive,
            base_type: Some(XSD_ANY_SIMPLE_TYPE),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &DATETIME_FACETS,
            validator: validate_datetime,
        },
        BuiltinType {
            name: XSD_DATE,
            category: TypeCategory::Primitive,
            base_type: Some(XSD_ANY_SIMPLE_TYPE),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &DATETIME_FACETS,
            validator: validate_date,
        },
        BuiltinType {
            name: XSD_TIME,
            category: TypeCategory::Primitive,
            base_type: Some(XSD_ANY_SIMPLE_TYPE),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &DATETIME_FACETS,
            validator: validate_time,
        },
        BuiltinType {
            name: XSD_GYEAR,
            category: TypeCategory::Primitive,
            base_type: Some(XSD_ANY_SIMPLE_TYPE),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &DATETIME_FACETS,
            validator: validate_gyear,
        },
        BuiltinType {
            name: XSD_GYEAR_MONTH,
            category: TypeCategory::Primitive,
            base_type: Some(XSD_ANY_SIMPLE_TYPE),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &DATETIME_FACETS,
            validator: validate_gyear_month,
        },
        BuiltinType {
            name: XSD_GMONTH,
            category: TypeCategory::Primitive,
            base_type: Some(XSD_ANY_SIMPLE_TYPE),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &DATETIME_FACETS,
            validator: validate_gmonth,
        },
        BuiltinType {
            name: XSD_GDAY,
            category: TypeCategory::Primitive,
            base_type: Some(XSD_ANY_SIMPLE_TYPE),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &DATETIME_FACETS,
            validator: validate_gday,
        },
        BuiltinType {
            name: XSD_GMONTH_DAY,
            category: TypeCategory::Primitive,
            base_type: Some(XSD_ANY_SIMPLE_TYPE),
            white_space: WhiteSpace::Collapse,
            admitted_facets: &DATETIME_FACETS,
            validator: validate_gmonth_day,
        },
    ];
}

/// Get a built-in type by name
pub fn get_builtin_type(name: &str) -> Option<&'static BuiltinType> {
    BUILTIN_TYPES.iter().find(|t| t.name == name)
}

/// Validate a value against a built-in type by name
pub fn validate_builtin(type_name: &str, value: &str) -> Result<XsdValue> {
    match get_builtin_type(type_name) {
        Some(builtin) => builtin.validate(value),
        None => Err(Error::Type(format!("Unknown built-in type: {}", type_name))),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_types() {
        assert!(validate_builtin(XSD_STRING, "Hello World").is_ok());
        assert!(validate_builtin(XSD_TOKEN, "Hello World").is_ok());
        assert!(validate_builtin(XSD_NCNAME, "validName").is_ok());
        assert!(validate_builtin(XSD_NCNAME, "invalid:name").is_err());
    }

    #[test]
    fn test_boolean_type() {
        assert_eq!(
            validate_builtin(XSD_BOOLEAN, "true").unwrap(),
            XsdValue::Boolean(true)
        );
        assert_eq!(
            validate_builtin(XSD_BOOLEAN, "false").unwrap(),
            XsdValue::Boolean(false)
        );
        assert_eq!(
            validate_builtin(XSD_BOOLEAN, "1").unwrap(),
            XsdValue::Boolean(true)
        );
        assert_eq!(
            validate_builtin(XSD_BOOLEAN, "0").unwrap(),
            XsdValue::Boolean(false)
        );
        assert!(validate_builtin(XSD_BOOLEAN, "yes").is_err());
    }

    #[test]
    fn test_numeric_types() {
        assert!(validate_builtin(XSD_INTEGER, "123").is_ok());
        assert!(validate_builtin(XSD_INTEGER, "-456").is_ok());
        assert!(validate_builtin(XSD_INTEGER, "abc").is_err());

        assert!(validate_builtin(XSD_BYTE, "127").is_ok());
        assert!(validate_builtin(XSD_BYTE, "128").is_err());
        assert!(validate_builtin(XSD_BYTE, "-128").is_ok());
        assert!(validate_builtin(XSD_BYTE, "-129").is_err());

        assert!(validate_builtin(XSD_UNSIGNED_BYTE, "255").is_ok());
        assert!(validate_builtin(XSD_UNSIGNED_BYTE, "256").is_err());
        assert!(validate_builtin(XSD_UNSIGNED_BYTE, "-1").is_err());

        assert!(validate_builtin(XSD_POSITIVE_INTEGER, "1").is_ok());
        assert!(validate_builtin(XSD_POSITIVE_INTEGER, "0").is_err());
        assert!(validate_builtin(XSD_POSITIVE_INTEGER, "-1").is_err());

        assert!(validate_builtin(XSD_NEGATIVE_INTEGER, "-1").is_ok());
        assert!(validate_builtin(XSD_NEGATIVE_INTEGER, "0").is_err());
        assert!(validate_builtin(XSD_NEGATIVE_INTEGER, "1").is_err());
    }

    #[test]
    fn test_float_types() {
        assert!(validate_builtin(XSD_FLOAT, "123.456").is_ok());
        assert!(validate_builtin(XSD_FLOAT, "NaN").is_ok());
        assert!(validate_builtin(XSD_FLOAT, "INF").is_ok());
        assert!(validate_builtin(XSD_FLOAT, "-INF").is_ok());

        assert!(validate_builtin(XSD_DOUBLE, "123.456").is_ok());
        assert!(validate_builtin(XSD_DOUBLE, "1.23e10").is_ok());
    }

    #[test]
    fn test_decimal_type() {
        assert!(validate_builtin(XSD_DECIMAL, "123.456").is_ok());
        assert!(validate_builtin(XSD_DECIMAL, "-789.012").is_ok());
        assert!(validate_builtin(XSD_DECIMAL, "abc").is_err());
    }

    #[test]
    fn test_binary_types() {
        assert!(validate_builtin(XSD_HEX_BINARY, "0A1B2C").is_ok());
        assert!(validate_builtin(XSD_HEX_BINARY, "").is_ok());
        assert!(validate_builtin(XSD_HEX_BINARY, "GH").is_err());

        assert!(validate_builtin(XSD_BASE64_BINARY, "SGVsbG8=").is_ok());
        assert!(validate_builtin(XSD_BASE64_BINARY, "").is_ok());
    }

    #[test]
    fn test_datetime_types() {
        assert!(validate_builtin(XSD_DATETIME, "2024-01-15T10:30:00").is_ok());
        assert!(validate_builtin(XSD_DATETIME, "2024-01-15T10:30:00Z").is_ok());
        assert!(validate_builtin(XSD_DATETIME, "2024-01-15T10:30:00+05:30").is_ok());
        assert!(validate_builtin(XSD_DATETIME, "invalid").is_err());

        assert!(validate_builtin(XSD_DATE, "2024-01-15").is_ok());
        assert!(validate_builtin(XSD_DATE, "2024-01-15Z").is_ok());
        assert!(validate_builtin(XSD_DATE, "invalid").is_err());

        assert!(validate_builtin(XSD_TIME, "10:30:00").is_ok());
        assert!(validate_builtin(XSD_TIME, "10:30:00.123").is_ok());
        assert!(validate_builtin(XSD_TIME, "invalid").is_err());

        assert!(validate_builtin(XSD_DURATION, "P1Y2M3DT4H5M6S").is_ok());
        assert!(validate_builtin(XSD_DURATION, "PT1H").is_ok());
        assert!(validate_builtin(XSD_DURATION, "P").is_err());
    }

    #[test]
    fn test_uri_types() {
        assert!(validate_builtin(XSD_ANY_URI, "http://example.com").is_ok());
        assert!(validate_builtin(XSD_ANY_URI, "relative/path").is_ok());
        assert!(validate_builtin(XSD_ANY_URI, "#fragment").is_ok());
    }

    #[test]
    fn test_qname_type() {
        assert!(validate_builtin(XSD_QNAME, "localName").is_ok());
        assert!(validate_builtin(XSD_QNAME, "prefix:localName").is_ok());
        assert!(validate_builtin(XSD_QNAME, "").is_err());
    }

    #[test]
    fn test_language_type() {
        assert!(validate_builtin(XSD_LANGUAGE, "en").is_ok());
        assert!(validate_builtin(XSD_LANGUAGE, "en-US").is_ok());
        assert!(validate_builtin(XSD_LANGUAGE, "zh-Hans-CN").is_ok());
        assert!(validate_builtin(XSD_LANGUAGE, "123").is_err());
    }

    #[test]
    fn test_error_type() {
        assert!(validate_builtin(XSD_ERROR, "anything").is_err());
    }

    #[test]
    fn test_get_builtin_type() {
        let string_type = get_builtin_type(XSD_STRING).unwrap();
        assert_eq!(string_type.name, XSD_STRING);
        assert!(string_type.is_string());
        assert!(!string_type.is_numeric());

        let int_type = get_builtin_type(XSD_INTEGER).unwrap();
        assert!(int_type.is_numeric());
        assert!(!int_type.is_string());

        let dt_type = get_builtin_type(XSD_DATETIME).unwrap();
        assert!(dt_type.is_datetime());

        assert!(get_builtin_type("unknownType").is_none());
    }

    #[test]
    fn test_xsd_value_display() {
        assert_eq!(XsdValue::String("test".to_string()).to_string(), "test");
        assert_eq!(XsdValue::Boolean(true).to_string(), "true");
        assert_eq!(XsdValue::Integer(42).to_string(), "42");
        assert_eq!(XsdValue::Float(f64::NAN).to_string(), "NaN");
        assert_eq!(XsdValue::Float(f64::INFINITY).to_string(), "INF");
    }

    #[test]
    fn test_type_category() {
        let string_type = get_builtin_type(XSD_STRING).unwrap();
        assert_eq!(string_type.category, TypeCategory::Primitive);

        let token_type = get_builtin_type(XSD_TOKEN).unwrap();
        assert_eq!(token_type.category, TypeCategory::Derived);

        let any_type = get_builtin_type(XSD_ANY_TYPE).unwrap();
        assert_eq!(any_type.category, TypeCategory::Special);
    }
}
