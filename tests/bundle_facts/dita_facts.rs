//! Static facts about DITA 1.2 schemas for assertion testing.
//!
//! These facts are derived from the OASIS DITA 1.2 specification and XSD schemas.
//! Reference: https://www.oasis-open.org/committees/dita/

/// Known facts about DITA 1.2 schemas
pub struct DitaFacts;

impl DitaFacts {
    /// DITA Architecture namespace
    pub const NAMESPACE: &'static str = "http://dita.oasis-open.org/architecture/2005/";

    /// Entry point XSD files for technicalContent package
    pub const ENTRY_POINTS: &'static [&'static str] = &[
        "topic.xsd",
        "concept.xsd",
        "reference.xsd",
        "task.xsd",
        "map.xsd",
        "ditabase.xsd",
        "generalTask.xsd",
        "glossary.xsd",
        "glossentry.xsd",
        "bookmap.xsd",
    ];

    /// Number of domains in the technicalContent/topic.xsd
    pub const DOMAIN_COUNT: usize = 8;

    /// Domain names expected in topic.xsd
    pub const DOMAINS: &'static [&'static str] = &[
        "abbrev-d",
        "hazard-d",
        "hi-d",
        "indexing-d",
        "pr-d",
        "sw-d",
        "ui-d",
        "ut-d",
    ];

    /// Topic element's required attributes
    pub const TOPIC_REQUIRED_ATTRS: &'static [&'static str] = &["id"];

    /// Topic element's optional attributes (subset)
    pub const TOPIC_OPTIONAL_ATTRS: &'static [&'static str] = &[
        "outputclass",
        "conref",
        "conrefend",
        "conaction",
        "conkeyref",
    ];

    /// Topic element's child elements in sequence order
    /// (name, min_occurs, max_occurs) - None means unbounded
    pub const TOPIC_CHILDREN: &'static [(&'static str, u32, Option<u32>)] = &[
        ("title", 1, Some(1)),        // required
        ("titlealts", 0, Some(1)),    // optional
        // shortdesc|abstract is a choice, handled separately
        ("prolog", 0, Some(1)),       // optional
        ("body", 0, Some(1)),         // optional
        ("related-links", 0, Some(1)), // optional
        // topic-info-types (nested topics) is unbounded
    ];

    /// topicreftypes enumeration values
    pub const TOPICREFTYPES_VALUES: &'static [&'static str] = &[
        "topic",
        "concept",
        "task",
        "reference",
        "external",
        "local",
        "-dita-use-conref-target",
    ];

    /// frame attribute enumeration values (tables)
    pub const FRAME_VALUES: &'static [&'static str] = &[
        "top",
        "bottom",
        "topbot",
        "all",
        "sides",
        "none",
        "-dita-use-conref-target",
    ];

    /// expanse attribute enumeration values
    pub const EXPANSE_VALUES: &'static [&'static str] = &[
        "page",
        "column",
        "textline",
        "-dita-use-conref-target",
    ];

    /// conaction attribute enumeration values
    pub const CONACTION_VALUES: &'static [&'static str] = &[
        "mark",
        "pushafter",
        "pushbefore",
        "pushreplace",
        "-dita-use-conref-target",
    ];

    /// importance attribute enumeration values
    pub const IMPORTANCE_VALUES: &'static [&'static str] = &[
        "obsolete",
        "deprecated",
        "optional",
        "default",
        "low",
        "normal",
        "high",
        "recommended",
        "required",
        "urgent",
        "-dita-use-conref-target",
    ];

    /// scale attribute enumeration values
    pub const SCALE_VALUES: &'static [&'static str] = &[
        "50",
        "60",
        "70",
        "80",
        "90",
        "100",
        "110",
        "120",
        "140",
        "160",
        "180",
        "200",
        "-dita-use-conref-target",
    ];

    /// status attribute enumeration values
    pub const STATUS_VALUES: &'static [&'static str] = &[
        "new",
        "changed",
        "deleted",
        "unchanged",
        "-dita-use-conref-target",
    ];

    /// Attribute groups defined in DITA base
    pub const ATTRIBUTE_GROUPS: &'static [&'static str] = &[
        "domains-att",
        "select-atts",
        "localization-atts",
        "global-atts",
        "conref-atts",
    ];
}

/// Child element information for assertions
#[derive(Debug, Clone)]
pub struct ChildElementFact {
    pub name: &'static str,
    pub min_occurs: u32,
    pub max_occurs: Option<u32>, // None = unbounded
}

/// Attribute fact for assertions
#[derive(Debug, Clone)]
pub struct AttributeFact {
    pub name: &'static str,
    pub type_name: &'static str,
    pub required: bool,
    pub default: Option<&'static str>,
}

impl DitaFacts {
    /// Get expected topic attributes
    pub fn topic_id_attribute() -> AttributeFact {
        AttributeFact {
            name: "id",
            type_name: "{http://www.w3.org/2001/XMLSchema}ID",
            required: true,
            default: None,
        }
    }

    /// Check if a value is a valid topicreftypes enumeration value
    pub fn is_valid_topicreftype(value: &str) -> bool {
        Self::TOPICREFTYPES_VALUES.contains(&value)
    }

    /// Check if a value is a valid importance enumeration value
    pub fn is_valid_importance(value: &str) -> bool {
        Self::IMPORTANCE_VALUES.contains(&value)
    }

    /// Check if a value is a valid scale enumeration value
    pub fn is_valid_scale(value: &str) -> bool {
        Self::SCALE_VALUES.contains(&value)
    }

    /// Check if a domain name is expected
    pub fn is_valid_domain(domain: &str) -> bool {
        Self::DOMAINS.contains(&domain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topicreftypes_values() {
        assert!(DitaFacts::is_valid_topicreftype("topic"));
        assert!(DitaFacts::is_valid_topicreftype("concept"));
        assert!(DitaFacts::is_valid_topicreftype("-dita-use-conref-target"));
        assert!(!DitaFacts::is_valid_topicreftype("invalid"));
    }

    #[test]
    fn test_importance_values() {
        assert!(DitaFacts::is_valid_importance("high"));
        assert!(DitaFacts::is_valid_importance("normal"));
        assert!(DitaFacts::is_valid_importance("-dita-use-conref-target"));
        assert!(!DitaFacts::is_valid_importance("invalid"));
    }

    #[test]
    fn test_domain_count() {
        assert_eq!(DitaFacts::DOMAINS.len(), DitaFacts::DOMAIN_COUNT);
    }

    #[test]
    fn test_entry_points_not_empty() {
        assert!(!DitaFacts::ENTRY_POINTS.is_empty());
        assert!(DitaFacts::ENTRY_POINTS.contains(&"topic.xsd"));
    }
}
