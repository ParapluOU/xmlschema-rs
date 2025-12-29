//! Static facts about NISO STS schemas for assertion testing.
//!
//! These facts are derived from the NISO STS (Z39.102-2017) specification.
//! NISO STS = NISO Standards Tag Suite for encoding standards documents.
//! Reference: https://www.niso-sts.org/

/// Known facts about NISO STS schemas
pub struct NisoFacts;

impl NisoFacts {
    /// Entry point XSD files
    pub const ENTRY_POINTS: &'static [&'static str] = &[
        "NISO-STS-extended-1-mathml3.xsd",
        "NISO-STS-interchange-1-mathml3.xsd",
    ];

    /// Number of namespace imports expected (7 total)
    pub const IMPORT_COUNT: usize = 7;

    /// Namespaces imported by NISO STS
    pub const IMPORTED_NAMESPACES: &'static [&'static str] = &[
        "http://www.w3.org/1999/xlink",                               // xlink
        "http://www.w3.org/1998/Math/MathML",                         // mml (MathML)
        "http://www.w3.org/2001/XInclude",                            // xi (XInclude)
        "urn:iso:std:iso:30042:ed-1",                                 // tbx (TBX terminology)
        "http://www.niso.org/standards/z39-96/ns/oasis-exchange/table", // oasis (OASIS table model)
        "http://www.niso.org/schemas/ali/1.0/",                       // ali (Access and License indicators)
        "http://www.w3.org/XML/1998/namespace",                       // xml
    ];

    /// Number of elements defined in NISO STS extended
    pub const ELEMENT_COUNT: usize = 347;

    /// Total enumeration values across all simple types
    pub const ENUMERATION_VALUE_COUNT: usize = 338;

    /// Key elements that should exist in NISO STS
    pub const KEY_ELEMENTS: &'static [&'static str] = &[
        "standard",
        "front",
        "body",
        "back",
        "sec",
        "title",
        "p",
        "table-wrap",
        "fig",
    ];

    /// Metadata elements specific to different standard types
    pub const METADATA_ELEMENTS: &'static [&'static str] = &[
        "std-meta",
        "iso-meta",
        "nat-meta",
        "reg-meta",
    ];

    /// orientation attribute enumeration values
    pub const ORIENTATION_VALUES: &'static [&'static str] = &[
        "landscape",
        "portrait",
    ];

    /// yes-no type enumeration values
    pub const YES_NO_VALUES: &'static [&'static str] = &[
        "yes",
        "no",
    ];

    /// pub-id-type attribute enumeration values (subset)
    pub const PUB_ID_TYPE_VALUES: &'static [&'static str] = &[
        "accession",
        "ark",
        "art-access-id",
        "arxiv",
        "coden",
        "doaj",
        "doi",
        "handle",
        "isbn",
        "manuscript",
        "medline",
        "other",
        "pii",
        "pmcid",
        "pmid",
        "publisher-id",
        "sici",
        "std-designation",
    ];

    /// standard type enumeration values
    pub const STANDARD_TYPE_VALUES: &'static [&'static str] = &[
        "implementation",
        "specification",
        "guide",
        "terminology",
        "test-method",
        "standard",
        "is",
        "tr",
        "ts",
        "pas",
    ];

    /// std-org-type enumeration values
    pub const STD_ORG_TYPE_VALUES: &'static [&'static str] = &[
        "sdo",           // Standards Developing Organization
        "consortium",
        "reg-auth",      // Registration Authority
    ];
}

impl NisoFacts {
    /// Check if a namespace is an expected import
    pub fn is_imported_namespace(ns: &str) -> bool {
        Self::IMPORTED_NAMESPACES.contains(&ns)
    }

    /// Check if an element is a key element
    pub fn is_key_element(name: &str) -> bool {
        Self::KEY_ELEMENTS.contains(&name)
    }

    /// Check if an element is a metadata element
    pub fn is_metadata_element(name: &str) -> bool {
        Self::METADATA_ELEMENTS.contains(&name)
    }

    /// Check if a value is a valid pub-id-type
    pub fn is_valid_pub_id_type(value: &str) -> bool {
        Self::PUB_ID_TYPE_VALUES.contains(&value)
    }

    /// Check if a value is a valid standard-type
    pub fn is_valid_standard_type(value: &str) -> bool {
        Self::STANDARD_TYPE_VALUES.contains(&value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namespace_count() {
        assert_eq!(NisoFacts::IMPORTED_NAMESPACES.len(), NisoFacts::IMPORT_COUNT);
    }

    #[test]
    fn test_key_elements() {
        assert!(NisoFacts::is_key_element("standard"));
        assert!(NisoFacts::is_key_element("front"));
        assert!(NisoFacts::is_key_element("body"));
        assert!(!NisoFacts::is_key_element("invalid-element"));
    }

    #[test]
    fn test_metadata_elements() {
        assert!(NisoFacts::is_metadata_element("std-meta"));
        assert!(NisoFacts::is_metadata_element("iso-meta"));
        assert!(!NisoFacts::is_metadata_element("invalid"));
    }

    #[test]
    fn test_pub_id_types() {
        assert!(NisoFacts::is_valid_pub_id_type("doi"));
        assert!(NisoFacts::is_valid_pub_id_type("isbn"));
        assert!(NisoFacts::is_valid_pub_id_type("pmid"));
        assert!(!NisoFacts::is_valid_pub_id_type("invalid"));
    }

    #[test]
    fn test_mathml_namespace() {
        assert!(NisoFacts::is_imported_namespace("http://www.w3.org/1998/Math/MathML"));
    }

    #[test]
    fn test_xlink_namespace() {
        assert!(NisoFacts::is_imported_namespace("http://www.w3.org/1999/xlink"));
    }
}
