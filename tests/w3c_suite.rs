//! The W3C XML Schema Test Suite, run against this parser.
//!
//! The suite is 235 MB and is not vendored here. Check it out beside the crate
//! and point this at it:
//!
//! ```sh
//! git clone --depth 1 https://github.com/w3c/xsdtests.git
//! XSDTESTS=../xsdtests cargo test --test w3c_suite -- --ignored --nocapture
//! ```
//!
//! It is the W3C's own copy (`00COPYRIGHT`: W3C Document Notice and License),
//! taken from the working copy of the XSD 1.1 working group.
//!
//! # What is asserted, and what is only reported
//!
//! The suite says, for each schema document, whether it is **valid** — a
//! conforming processor must accept it — or **invalid**, which a conforming
//! processor must reject. Those are two very different claims about a parser
//! that is deliberately lenient:
//!
//! - A valid schema this parser **rejects** is a bug, and the only thing here
//!   that fails a test. A schema the W3C says is well formed and we cannot read
//!   is a schema some customer cannot import.
//! - An invalid schema this parser **accepts** is a gap, counted and printed
//!   but not failed. This crate reads schemas in order to compile them; it does
//!   not set out to be the arbiter of which are legal, and making every one of
//!   those fail would be asserting an ambition the crate does not have.
//!
//! So the number to watch is the first, and the second is a measurement of how
//! far from a conformance-grade validator this is.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use xmlschema::documents::{Document, Element};
use xmlschema::validators::XsdSchema;

/// One schema the suite has an opinion about.
struct SchemaTest {
    /// The test group it belongs to, for reporting.
    group: String,
    /// The schema document itself.
    at: PathBuf,
    /// Whether the suite says a processor must accept it.
    valid: bool,
}

/// Where the suite is checked out, if it is.
fn suite() -> Option<PathBuf> {
    let at = PathBuf::from(std::env::var("XSDTESTS").ok()?);
    at.is_dir().then_some(at)
}

/// Every `testGroup`'s schema test, from every test set in the suite.
fn schema_tests(root: &Path) -> Vec<SchemaTest> {
    let mut tests = Vec::new();
    let mut walking = vec![root.to_path_buf()];
    while let Some(directory) = walking.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walking.push(path);
                continue;
            }
            // The sets are `.testSet` in the W3C's own layout and `.xml` in the
            // GitHub copy; what says one is a test set is its root element.
            if !matches!(path.extension().and_then(|e| e.to_str()), Some("testSet" | "xml")) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            if !text.contains("<testSet") {
                continue;
            }
            let Ok(document) = Document::from_string(&text) else { continue };
            let Some(root_element) = document.root() else { continue };
            let beside = path.parent().unwrap_or(Path::new("."));
            collect(root_element, beside, &mut tests);
        }
    }
    tests
}

/// Walk a test set for `schemaTest` elements.
fn collect(element: &Element, beside: &Path, tests: &mut Vec<SchemaTest>) {
    if element.local_name() == "testGroup" {
        let group = element.get_attribute("name").unwrap_or("?").to_string();
        for child in &element.children {
            if child.local_name() != "schemaTest" {
                continue;
            }
            // A schema test may name several documents; the first is the one
            // compiled, the rest are what it includes.
            let Some(document) = child.children.iter().find(|c| c.local_name() == "schemaDocument") else { continue };
            let Some(href) = document.get_attribute("href") else { continue };
            if href.is_empty() {
                continue;
            }
            let Some(expected) = child.children.iter().find(|c| c.local_name() == "expected") else { continue };
            let valid = match expected.get_attribute("validity") {
                Some("valid") => true,
                Some("invalid") => false,
                // `notKnown`, and anything a later version adds: no claim to test.
                _ => continue,
            };
            tests.push(SchemaTest { group: group.clone(), at: beside.join(href), valid });
        }
    }
    for child in &element.children {
        collect(child, beside, tests);
    }
}

/// Whether this schema, or anything it includes a level down, uses
/// `xs:redefine` — which is how the redefine corpus is picked out.
fn uses_redefine(at: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(at) else { return false };
    if text.contains("redefine") {
        return true;
    }
    // One level is enough to catch a test whose entry includes the redefining
    // document, which is the usual shape.
    let beside = at.parent().unwrap_or(Path::new("."));
    text.match_indices("schemaLocation=\"")
        .filter_map(|(at, _)| text[at + 16..].split('"').next())
        .any(|location| {
            std::fs::read_to_string(beside.join(location))
                .map(|included| included.contains("redefine"))
                .unwrap_or(false)
        })
}

/// What a run of the suite came to.
#[derive(Default)]
struct Tally {
    /// Valid, and accepted: right.
    accepted: usize,
    /// Valid, and refused: wrong, and the only thing that fails.
    refused: Vec<String>,
    /// Invalid, and refused: right.
    caught: usize,
    /// Invalid, and accepted: lenient, counted but not failed.
    missed: usize,
}

impl Tally {
    fn of(tests: &[SchemaTest]) -> Self {
        let mut tally = Tally::default();
        for test in tests {
            if !test.at.is_file() {
                continue;
            }
            // A schema that sends the parser into a panic is a refusal, not a
            // crashed test run.
            let parsed = std::panic::catch_unwind(|| XsdSchema::from_file(&test.at)).unwrap_or_else(|_| {
                Err(xmlschema::error::Error::Parse(xmlschema::error::ParseError::new("panicked")))
            });
            match (test.valid, parsed.is_ok()) {
                (true, true) => tally.accepted += 1,
                (true, false) => tally.refused.push(format!("{} ({})", test.group, test.at.display())),
                (false, false) => tally.caught += 1,
                (false, true) => tally.missed += 1,
            }
        }
        tally
    }

    fn say(&self, what: &str) {
        let valid = self.accepted + self.refused.len();
        let invalid = self.caught + self.missed;
        println!("\n{what}");
        println!("  the suite says valid:   {valid:5}  accepted {:5}  refused {:5}", self.accepted, self.refused.len());
        println!("  the suite says invalid: {invalid:5}  refused  {:5}  accepted {:5} (lenient)", self.caught, self.missed);
        for refused in self.refused.iter().take(20) {
            println!("    refused a valid schema: {refused}");
        }
        if self.refused.len() > 20 {
            println!("    … and {} more", self.refused.len() - 20);
        }
    }
}

/// Every schema in the suite that uses `xs:redefine`.
///
/// This is the corpus for the feature the crate had been quietly dropping: a
/// redefinition was parsed and thrown away, so every component it changed kept
/// the definition the included schema gave it.
#[test]
#[ignore = "needs the W3C suite: XSDTESTS=/path/to/xsdtests"]
fn the_w3c_suite_on_redefine() {
    let Some(root) = suite() else {
        panic!("XSDTESTS is not a directory; see this file's documentation");
    };
    let all = schema_tests(&root);
    let redefine: Vec<SchemaTest> = all.into_iter().filter(|test| uses_redefine(&test.at)).collect();
    assert!(!redefine.is_empty(), "no redefine tests found under {}", root.display());
    let tally = Tally::of(&redefine);
    tally.say(&format!("W3C XSD test suite — the {} schema tests that use xs:redefine", redefine.len()));
    assert!(
        tally.refused.is_empty(),
        "{} schemas the suite says are valid were refused",
        tally.refused.len()
    );
}

/// The whole suite, as a measurement rather than a gate.
///
/// Printed, never failed: this crate reads schemas in order to compile them and
/// does not set out to decide which are legal, so the count of invalid schemas
/// it accepts is a measure of distance from a conformance validator rather than
/// a list of bugs. What *is* worth reading is how many valid ones it refuses.
#[test]
#[ignore = "a measurement over 235 MB of test suite; XSDTESTS=/path/to/xsdtests"]
fn the_w3c_suite_whole() {
    let Some(root) = suite() else {
        panic!("XSDTESTS is not a directory; see this file's documentation");
    };
    let all = schema_tests(&root);
    assert!(!all.is_empty(), "no tests found under {}", root.display());
    let tally = Tally::of(&all);
    tally.say(&format!("W3C XSD test suite — all {} schema tests", all.len()));

    // Which part of the suite the refusals come from, which says where to look:
    // the two directories below the vendor's data directory.
    let mut by_set: BTreeMap<String, usize> = BTreeMap::new();
    for refused in &tally.refused {
        let parts: Vec<&str> = refused.split('/').collect();
        let set = parts
            .iter()
            .position(|part| part.ends_with("Data"))
            .map(|at| parts[at..].iter().take(2).copied().collect::<Vec<_>>().join("/"))
            .unwrap_or_else(|| "?".to_string());
        *by_set.entry(set).or_default() += 1;
    }
    println!("  refusals by part of the suite: {by_set:?}");
}
