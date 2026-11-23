//! XML Catalogs, as OASIS defines them.
//!
//! A catalog maps the identifiers a document uses — a `schemaLocation`, a
//! system identifier, a public identifier — onto where the thing actually is.
//! It is how a standard can address its own parts by URN
//! (`urn:oasis:names:tc:dita:xsd:topicMod.xsd:1.2`) and still be resolvable
//! from a directory on disk, which is how OASIS ships DITA.
//!
//! This implements **XML Catalogs 1.1**:
//! <https://www.oasis-open.org/committees/entity/spec-2001-08-06.html> (1.0)
//! and the 1.1 committee specification, which adds the suffix entries.
//!
//! # Entries
//!
//! All twelve, in both resolution paths:
//!
//! | element | what it does |
//! |---|---|
//! | `public` | an exact public identifier |
//! | `system` | an exact system identifier |
//! | `systemSuffix` | the longest matching **end** of a system identifier |
//! | `rewriteSystem` | the longest matching **start**, replaced |
//! | `delegateSystem` | hand the question to other catalogs |
//! | `delegatePublic` | the same, for a public identifier |
//! | `uri` | an exact URI |
//! | `uriSuffix` | the longest matching end of a URI |
//! | `rewriteURI` | the longest matching start, replaced |
//! | `delegateURI` | hand the question to other catalogs |
//! | `nextCatalog` | another catalog, consulted only after this one fails |
//! | `group` | entries sharing an `xml:base` and a `prefer` |
//!
//! # The order matters, and so does the length
//!
//! Resolution is not a lookup. For a URI the specification searches, **in this
//! order**: `uri`, then `rewriteURI`, then `uriSuffix`, then `delegateURI` —
//! and only once every entry in the whole catalog has failed are its
//! `nextCatalog` files consulted. Among rewrites and suffixes the **longest**
//! match wins, not the first; among exact entries the first in document order
//! does.
//!
//! Delegation is terminal: once a `delegate*` entry matches, the catalogs it
//! names are the only ones searched, and if they have no answer the resolution
//! fails rather than falling back to the rest of the delegating catalog. That
//! is what makes delegation useful — it partitions a large catalog space — and
//! getting it wrong turns a miss into a quietly wrong answer.
//!
//! # Where this stops, deliberately
//!
//! - **`prefer` defaults to `public`.** The attribute is `#IMPLIED` in the
//!   specification, which does not say what an absent one means; this follows
//!   libxml2 and the SGML Open predecessor.
//! - **Identifiers are compared as written**, after the normalisation the
//!   specification requires: whitespace collapsed in a public identifier, and
//!   the characters that must be escaped percent-encoded in a URI. No
//!   scheme-specific normalisation — the case of a host, a default port, `.`
//!   and `..` in a path — is attempted. A catalog for files on disk never
//!   needs it.
//! - **What an entry resolves to is a path, not a URI.** A relative one is
//!   joined onto the directory of the catalog that declared it, because the
//!   only consumer is a schema loader reading files.

use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use crate::documents::{Document, Element};
use crate::error::{Error, ParseError, Result};

/// The XML Catalog namespace
const CATALOG_NS: &str = "urn:oasis:names:tc:entity:xmlns:xml:catalog";

/// Which identifier is preferred when a document gives both a public and a
/// system one.
///
/// A system identifier always wins where it resolves; `prefer` decides whether
/// the public one is considered at all once it has not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Prefer {
    /// Consider `public` entries when the system identifier found nothing.
    #[default]
    Public,
    /// Never consider `public` entries where a system identifier was given.
    System,
}

/// One entry of a catalog, in the order it was written.
#[derive(Debug, Clone)]
enum Entry {
    /// `<public publicId uri>`, with the `prefer` in effect where it sits.
    Public {
        public_id: String,
        uri: String,
        prefer: Prefer,
    },
    /// `<system systemId uri>`
    System { system_id: String, uri: String },
    /// `<systemSuffix systemIdSuffix uri>`
    SystemSuffix { suffix: String, uri: String },
    /// `<rewriteSystem systemIdStartString rewritePrefix>`
    RewriteSystem { start: String, prefix: String },
    /// `<delegateSystem systemIdStartString catalog>`
    DelegateSystem {
        start: String,
        catalog: Box<XmlCatalog>,
    },
    /// `<delegatePublic publicIdStartString catalog>`
    DelegatePublic {
        start: String,
        catalog: Box<XmlCatalog>,
    },
    /// `<uri name uri>`
    Uri { name: String, uri: String },
    /// `<uriSuffix uriSuffix uri>`
    UriSuffix { suffix: String, uri: String },
    /// `<rewriteURI uriStartString rewritePrefix>`
    RewriteUri { start: String, prefix: String },
    /// `<delegateURI uriStartString catalog>`
    DelegateUri {
        start: String,
        catalog: Box<XmlCatalog>,
    },
}

/// An XML catalog: its entries in document order, and the catalogs its
/// `nextCatalog` entries name.
///
/// Those are held apart from the entries rather than merged into them, because
/// the specification searches this whole catalog before any of them. Merging
/// would make precedence a matter of document order, which is how a nested
/// catalog comes to quietly outrank the catalog that named it.
#[derive(Debug, Clone, Default)]
pub struct XmlCatalog {
    entries: Vec<Entry>,
    next: Vec<XmlCatalog>,
    /// Catalogs this one names that could not be read, and why.
    ///
    /// Kept rather than raised. A bundle commonly ships a catalog naming
    /// modules it does not carry — DITA-LCE's names `machineryIndustry`,
    /// `subjectScheme` and `xnal`, none of which are in it — and refusing the
    /// whole catalog for that would make a standard unusable over parts that
    /// nothing it is asked about needs. (Checked: tolerating them, DITA-LCE
    /// resolves to the same 265 elements as DITA 1.3.)
    ///
    /// Nor is it swallowed: anything that needed the missing catalog fails to
    /// resolve, and [`missing`](Self::missing) says what was not there, so a
    /// caller can report it beside whatever went wrong afterwards.
    missing: Vec<String>,
}

/// What one catalog had to say about a question.
enum Step<'a> {
    /// It resolved.
    Found(String),
    /// A `delegate*` entry matched: these catalogs, and nothing else, answer
    /// the question now.
    Delegated(Vec<&'a XmlCatalog>),
    /// Nothing here.
    Missed,
}

impl XmlCatalog {
    /// An empty catalog.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load a catalog, and every catalog it names.
    ///
    /// A catalog that names itself, directly or in a ring, is read once: the
    /// specification says a catalog entry file is processed at most once in a
    /// catalog list, and without that a cycle would not terminate.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let mut seen = HashSet::new();
        Self::load(path.as_ref(), &mut seen)
    }

    fn load(path: &Path, seen: &mut HashSet<PathBuf>) -> Result<Self> {
        let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        if !seen.insert(canonical) {
            // Already in this catalog list: it contributes nothing a second time.
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path).map_err(|e| {
            Error::Resource(format!(
                "Failed to read catalog '{}': {}",
                path.display(),
                e
            ))
        })?;
        let base = path
            .parent()
            .map(|parent| directory_prefix(&parent.to_string_lossy()))
            .unwrap_or_default();
        Self::parse(&content, &base, seen)
    }

    /// Parse catalog XML whose relative entries resolve against `base`.
    fn parse(xml: &str, base: &str, seen: &mut HashSet<PathBuf>) -> Result<Self> {
        let doc = Document::from_string(xml)?;
        let root = doc
            .root()
            .ok_or_else(|| Error::Parse(ParseError::new("Empty catalog document")))?;
        if root.local_name() != "catalog" {
            return Err(Error::Parse(ParseError::new(format!(
                "Expected catalog root element, got {}",
                root.local_name()
            ))));
        }
        // The declared default namespace, which is where a catalog's elements
        // live. Read from what the document declared rather than from the
        // element's own qualified name, which the parser leaves unresolved.
        if let Some(namespace) = root.namespaces.get_default_namespace() {
            if namespace != CATALOG_NS {
                return Err(Error::Parse(ParseError::new(format!(
                    "Expected a catalog in {CATALOG_NS}, got one in {namespace}"
                ))));
            }
        }
        let mut catalog = Self::default();
        // The root is a group: its `xml:base` and `prefer` apply to everything.
        let base = base_of(root, base);
        let prefer = prefer_of(root, Prefer::default())?;
        catalog.read(&root.children, &base, prefer, seen)?;
        Ok(catalog)
    }

    /// Read the children of a `catalog` or a `group`.
    fn read(
        &mut self,
        children: &[Element],
        base: &str,
        prefer: Prefer,
        seen: &mut HashSet<PathBuf>,
    ) -> Result<()> {
        for child in children {
            // Each entry may move its own base; a group moves it for everything
            // inside, and may change `prefer` for them too.
            let base = base_of(child, base);
            let prefer = prefer_of(child, prefer)?;
            let attribute = |name: &str| child.get_attribute(name);
            match child.local_name() {
                "public" => {
                    if let (Some(id), Some(uri)) = (attribute("publicId"), attribute("uri")) {
                        self.entries.push(Entry::Public {
                            public_id: normalized_public_id(id),
                            uri: against(&base, uri),
                            prefer,
                        });
                    }
                }
                "system" => {
                    if let (Some(id), Some(uri)) = (attribute("systemId"), attribute("uri")) {
                        self.entries.push(Entry::System {
                            system_id: normalized_uri(id),
                            uri: against(&base, uri),
                        });
                    }
                }
                "systemSuffix" => {
                    if let (Some(suffix), Some(uri)) =
                        (attribute("systemIdSuffix"), attribute("uri"))
                    {
                        self.entries.push(Entry::SystemSuffix {
                            suffix: normalized_uri(suffix),
                            uri: against(&base, uri),
                        });
                    }
                }
                "rewriteSystem" => {
                    if let (Some(start), Some(prefix)) =
                        (attribute("systemIdStartString"), attribute("rewritePrefix"))
                    {
                        self.entries.push(Entry::RewriteSystem {
                            start: normalized_uri(start),
                            prefix: against(&base, prefix),
                        });
                    }
                }
                "uri" => {
                    if let (Some(name), Some(uri)) = (attribute("name"), attribute("uri")) {
                        self.entries.push(Entry::Uri {
                            name: normalized_uri(name),
                            uri: against(&base, uri),
                        });
                    }
                }
                "uriSuffix" => {
                    if let (Some(suffix), Some(uri)) = (attribute("uriSuffix"), attribute("uri")) {
                        self.entries.push(Entry::UriSuffix {
                            suffix: normalized_uri(suffix),
                            uri: against(&base, uri),
                        });
                    }
                }
                "rewriteURI" => {
                    if let (Some(start), Some(prefix)) =
                        (attribute("uriStartString"), attribute("rewritePrefix"))
                    {
                        self.entries.push(Entry::RewriteUri {
                            start: normalized_uri(start),
                            prefix: against(&base, prefix),
                        });
                    }
                }
                kind @ ("delegateSystem" | "delegatePublic" | "delegateURI") => {
                    let start = match kind {
                        "delegateSystem" => attribute("systemIdStartString"),
                        "delegatePublic" => attribute("publicIdStartString"),
                        _ => attribute("uriStartString"),
                    };
                    if let (Some(start), Some(named)) = (start, attribute("catalog")) {
                        // A delegate's catalogs are a list of their own, so a
                        // file already read for *this* list may be read again
                        // for theirs.
                        let mut theirs = HashSet::new();
                        let catalog =
                            Box::new(Self::load(Path::new(&against(&base, named)), &mut theirs)?);
                        self.entries.push(match kind {
                            "delegateSystem" => Entry::DelegateSystem {
                                start: normalized_uri(start),
                                catalog,
                            },
                            "delegatePublic" => Entry::DelegatePublic {
                                start: normalized_public_id(start),
                                catalog,
                            },
                            _ => Entry::DelegateUri {
                                start: normalized_uri(start),
                                catalog,
                            },
                        });
                    }
                }
                "nextCatalog" => {
                    if let Some(named) = attribute("catalog") {
                        let at = against(&base, named);
                        match Self::load(Path::new(&at), seen) {
                            Ok(loaded) => self.next.push(loaded),
                            // Remembered, not raised: see the note on `missing`.
                            Err(why) => self.missing.push(format!("{at}: {why}")),
                        }
                    }
                }
                "group" => self.read(&child.children, &base, prefer, seen)?,
                // Annotations, and whatever a later version adds.
                _ => {}
            }
        }
        Ok(())
    }

    /// Resolve a URI: `uri`, then `rewriteURI`, then `uriSuffix`, then
    /// `delegateURI`, then the `nextCatalog` files.
    ///
    /// A URI in the `urn:publicid:` form is a public identifier in disguise
    /// (RFC 3151) and is resolved as one.
    pub fn resolve_uri(&self, uri: &str) -> Option<String> {
        let uri = normalized_uri(uri);
        if let Some(public_id) = public_id_of_urn(&uri) {
            return self.resolve_external_identifier(Some(&public_id), None);
        }
        over(self, &|catalog: &XmlCatalog| catalog.uri_step(&uri))
    }

    /// Resolve an external identifier: the system identifier wherever it
    /// resolves, and the public one after it where `prefer` allows.
    pub fn resolve_external_identifier(
        &self,
        public_id: Option<&str>,
        system_id: Option<&str>,
    ) -> Option<String> {
        // A system identifier written as a `urn:publicid:` URN *is* the public
        // identifier (RFC 3151), and the specification says to unwrap it.
        let (public_id, system_id) = match system_id.map(normalized_uri) {
            Some(system) => match public_id_of_urn(&system) {
                Some(unwrapped) => (public_id.map(normalized_public_id).or(Some(unwrapped)), None),
                None => (public_id.map(normalized_public_id), Some(system)),
            },
            None => (public_id.map(normalized_public_id), None),
        };
        if let Some(system) = &system_id {
            if let Some(found) = over(self, &|catalog: &XmlCatalog| catalog.system_step(system)) {
                return Some(found);
            }
        }
        let public = public_id?;
        // With both identifiers, `prefer="system"` means the public one is not
        // consulted at all — which is the point of saying so.
        let with_system = system_id.is_some();
        over(self, &|catalog: &XmlCatalog| {
            catalog.public_step(&public, with_system)
        })
    }

    /// Resolve the location a document names, by whichever path answers.
    ///
    /// A `schemaLocation` is a URI and is resolved as one; where that finds
    /// nothing it is tried as a system identifier, because a catalog may
    /// describe the same file either way and some do both. DITA's says as much
    /// in a comment: its `system` entries are there for resolvers that treat a
    /// schema location as a system identifier, its `uri` entries for those
    /// that follow the specification.
    pub fn resolve(&self, location: &str) -> Option<String> {
        self.resolve_uri(location)
            .or_else(|| self.resolve_external_identifier(None, Some(location)))
    }

    /// What this one catalog says about a URI, in the order the specification
    /// gives.
    fn uri_step(&self, uri: &str) -> Step<'_> {
        for entry in &self.entries {
            if let Entry::Uri { name, uri: to } = entry {
                if name == uri {
                    return Step::Found(to.clone());
                }
            }
        }
        if let Some(found) = self.longest_rewrite(uri, |entry| match entry {
            Entry::RewriteUri { start, prefix } => Some((start, prefix)),
            _ => None,
        }) {
            return Step::Found(found);
        }
        if let Some(found) = self.longest_suffix(uri, |entry| match entry {
            Entry::UriSuffix { suffix, uri } => Some((suffix, uri)),
            _ => None,
        }) {
            return Step::Found(found);
        }
        match self.delegates(uri, |entry| match entry {
            Entry::DelegateUri { start, catalog } => Some((start, catalog.as_ref())),
            _ => None,
        }) {
            Some(catalogs) => Step::Delegated(catalogs),
            None => Step::Missed,
        }
    }

    /// What this one catalog says about a system identifier.
    fn system_step(&self, system_id: &str) -> Step<'_> {
        for entry in &self.entries {
            if let Entry::System { system_id: id, uri } = entry {
                if id == system_id {
                    return Step::Found(uri.clone());
                }
            }
        }
        if let Some(found) = self.longest_rewrite(system_id, |entry| match entry {
            Entry::RewriteSystem { start, prefix } => Some((start, prefix)),
            _ => None,
        }) {
            return Step::Found(found);
        }
        if let Some(found) = self.longest_suffix(system_id, |entry| match entry {
            Entry::SystemSuffix { suffix, uri } => Some((suffix, uri)),
            _ => None,
        }) {
            return Step::Found(found);
        }
        match self.delegates(system_id, |entry| match entry {
            Entry::DelegateSystem { start, catalog } => Some((start, catalog.as_ref())),
            _ => None,
        }) {
            Some(catalogs) => Step::Delegated(catalogs),
            None => Step::Missed,
        }
    }

    /// What this one catalog says about a public identifier. `with_system` is
    /// whether a system identifier was given beside it, which is when `prefer`
    /// has anything to decide.
    fn public_step(&self, public_id: &str, with_system: bool) -> Step<'_> {
        for entry in &self.entries {
            if let Entry::Public {
                public_id: id,
                uri,
                prefer,
            } = entry
            {
                if id == public_id && !(with_system && *prefer == Prefer::System) {
                    return Step::Found(uri.clone());
                }
            }
        }
        match self.delegates(public_id, |entry| match entry {
            Entry::DelegatePublic { start, catalog } => Some((start, catalog.as_ref())),
            _ => None,
        }) {
            Some(catalogs) => Step::Delegated(catalogs),
            None => Step::Missed,
        }
    }

    /// The longest `rewrite*` whose start matches, with the rest of the
    /// identifier carried onto its prefix.
    fn longest_rewrite<'a>(
        &'a self,
        id: &str,
        of: impl Fn(&'a Entry) -> Option<(&'a String, &'a String)>,
    ) -> Option<String> {
        self.entries
            .iter()
            .filter_map(of)
            .filter(|(start, _)| id.starts_with(start.as_str()))
            .max_by_key(|(start, _)| start.len())
            .map(|(start, prefix)| format!("{prefix}{}", &id[start.len()..]))
    }

    /// The longest `*Suffix` whose end matches.
    fn longest_suffix<'a>(
        &'a self,
        id: &str,
        of: impl Fn(&'a Entry) -> Option<(&'a String, &'a String)>,
    ) -> Option<String> {
        self.entries
            .iter()
            .filter_map(of)
            .filter(|(suffix, _)| id.ends_with(suffix.as_str()))
            .max_by_key(|(suffix, _)| suffix.len())
            .map(|(_, uri)| uri.clone())
    }

    /// Every `delegate*` whose start matches, longest first: the catalog list
    /// that answers the question now, and the only one that will.
    fn delegates<'a>(
        &'a self,
        id: &str,
        of: impl Fn(&'a Entry) -> Option<(&'a String, &'a XmlCatalog)>,
    ) -> Option<Vec<&'a XmlCatalog>> {
        let mut matching: Vec<(&String, &XmlCatalog)> = self
            .entries
            .iter()
            .filter_map(of)
            .filter(|(start, _)| id.starts_with(start.as_str()))
            .collect();
        if matching.is_empty() {
            return None;
        }
        matching.sort_by_key(|(start, _)| std::cmp::Reverse(start.len()));
        Some(matching.into_iter().map(|(_, catalog)| catalog).collect())
    }

    /// Append another catalog's entries after this one's, so that this
    /// catalog's keep their precedence.
    pub fn merge(&mut self, other: &XmlCatalog) {
        self.entries.extend(other.entries.iter().cloned());
        self.next.extend(other.next.iter().cloned());
        self.missing.extend(other.missing.iter().cloned());
    }

    /// The catalogs this one, or any it names, could not read — and why.
    ///
    /// Empty for a catalog whose every part was there. Anything else is worth
    /// repeating to whoever handed the bundle over: it is the likeliest reason
    /// an identifier that should have resolved did not.
    pub fn missing(&self) -> Vec<String> {
        let mut all = self.missing.clone();
        for next in &self.next {
            all.extend(next.missing());
        }
        for entry in &self.entries {
            match entry {
                Entry::DelegateSystem { catalog, .. }
                | Entry::DelegatePublic { catalog, .. }
                | Entry::DelegateUri { catalog, .. } => all.extend(catalog.missing()),
                _ => {}
            }
        }
        all
    }

    /// Whether the catalog has anything to say.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty() && self.next.iter().all(Self::is_empty)
    }

    /// How many entries it has, its `nextCatalog` files included.
    pub fn len(&self) -> usize {
        self.entries.len() + self.next.iter().map(Self::len).sum::<usize>()
    }
}

/// Walk a catalog list: this catalog, then what it names, in order — and if a
/// delegation matches, that list alone.
fn over(start: &XmlCatalog, attempt: &dyn Fn(&XmlCatalog) -> Step<'_>) -> Option<String> {
    let mut list: VecDeque<&XmlCatalog> = VecDeque::new();
    list.push_back(start);
    while let Some(catalog) = list.pop_front() {
        match attempt(catalog) {
            Step::Found(uri) => return Some(uri),
            // Terminal by specification: the delegating catalog's remaining
            // entries, and everything after it in the list, are not consulted.
            Step::Delegated(catalogs) => {
                let mut theirs: VecDeque<&XmlCatalog> = catalogs.into_iter().collect();
                while let Some(catalog) = theirs.pop_front() {
                    match attempt(catalog) {
                        Step::Found(uri) => return Some(uri),
                        Step::Delegated(further) => theirs = further.into_iter().collect(),
                        Step::Missed => theirs.extend(catalog.next.iter()),
                    }
                }
                return None;
            }
            // Only once this whole catalog has failed.
            Step::Missed => list.extend(catalog.next.iter()),
        }
    }
    None
}

/// `xml:base` where an element carries one, joined onto what it inherited.
///
/// Catalogs lean on this. OASIS ships DITA with one catalog at the root of the
/// bundle and the directory of each entry in its `xml:base`; ignoring it points
/// every mapping at a file beside the catalog that is not there.
fn base_of(element: &Element, inherited: &str) -> String {
    // `xml:base`'s local name is `base`, which is how it is read here.
    match element.get_attribute("base") {
        Some(given) => directory_prefix(&against(inherited, given)),
        None => inherited.to_string(),
    }
}

/// `prefer` where an element carries one, or what it inherited.
fn prefer_of(element: &Element, inherited: Prefer) -> Result<Prefer> {
    match element.get_attribute("prefer") {
        Some("public") => Ok(Prefer::Public),
        Some("system") => Ok(Prefer::System),
        Some(other) => Err(Error::Parse(ParseError::new(format!(
            "`prefer` is `public` or `system`, not `{other}`"
        )))),
        None => Ok(inherited),
    }
}

/// A relative value joined onto the base in effect; an absolute one as it is.
fn against(base: &str, value: &str) -> String {
    if value.starts_with('/') || value.contains("://") || base.is_empty() {
        value.to_string()
    } else {
        format!("{base}{value}")
    }
}

/// A directory as a prefix other paths hang off: with one trailing separator.
fn directory_prefix(path: &str) -> String {
    if path.is_empty() || path.ends_with('/') {
        path.to_string()
    } else {
        format!("{path}/")
    }
}

/// A public identifier with its whitespace normalised — runs of whitespace
/// become one space and the ends are trimmed — as the specification requires
/// before comparison.
fn normalized_public_id(id: &str) -> String {
    id.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A URI with the characters that must be escaped percent-encoded: space, the
/// control characters, the delimiters, and anything outside printable ASCII.
/// One that is already escaped is left as it is.
fn normalized_uri(uri: &str) -> String {
    let mut out = String::with_capacity(uri.len());
    for byte in uri.bytes() {
        match byte {
            b' ' | b'"' | b'<' | b'>' | b'\\' | b'^' | b'`' | b'{' | b'|' | b'}' => {
                out.push_str(&format!("%{byte:02X}"))
            }
            byte if byte <= 0x20 || byte >= 0x7F => out.push_str(&format!("%{byte:02X}")),
            byte => out.push(byte as char),
        }
    }
    out
}

/// The public identifier a `urn:publicid:` URN stands for (RFC 3151), or
/// `None` for a URI that is not one.
fn public_id_of_urn(uri: &str) -> Option<String> {
    let rest = uri.strip_prefix("urn:publicid:")?;
    let mut out = String::with_capacity(rest.len());
    let mut characters = rest.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '+' => out.push(' '),
            ':' => out.push_str("//"),
            ';' => out.push_str("::"),
            '%' => {
                // `%xx` for whatever could not be written plainly.
                let hex: String = characters.by_ref().take(2).collect();
                match u8::from_str_radix(&hex, 16) {
                    Ok(byte) => out.push(byte as char),
                    // Not an escape after all: keep it as it was written.
                    Err(_) => {
                        out.push('%');
                        out.push_str(&hex);
                    }
                }
            }
            character => out.push(character),
        }
    }
    Some(normalized_public_id(&out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    /// Write a catalog into `directory` and answer with its path.
    fn write(directory: &TempDir, name: &str, xml: &str) -> PathBuf {
        let at = directory.path().join(name);
        if let Some(parent) = at.parent() {
            std::fs::create_dir_all(parent).expect("a directory");
        }
        let mut file = std::fs::File::create(&at).expect("a file");
        file.write_all(xml.as_bytes()).expect("written");
        at
    }

    fn catalog(xml: &str) -> (TempDir, XmlCatalog) {
        let directory = TempDir::new().expect("a directory");
        let at = write(&directory, "catalog.xml", xml);
        let catalog = XmlCatalog::from_file(&at).expect("the catalog loads");
        (directory, catalog)
    }

    fn here(directory: &TempDir, path: &str) -> String {
        directory.path().join(path).to_string_lossy().to_string()
    }

    #[test]
    fn an_exact_entry_resolves_and_the_first_of_them_wins() {
        let (directory, catalog) = catalog(
            r#"<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
                 <uri name="urn:example:types" uri="types.xsd"/>
                 <uri name="urn:example:types" uri="other.xsd"/>
                 <system systemId="urn:example:topic" uri="topic.xsd"/>
               </catalog>"#,
        );
        assert_eq!(
            catalog.resolve_uri("urn:example:types"),
            Some(here(&directory, "types.xsd"))
        );
        assert_eq!(
            catalog.resolve_external_identifier(None, Some("urn:example:topic")),
            Some(here(&directory, "topic.xsd"))
        );
        assert_eq!(catalog.resolve_uri("urn:example:nothing"), None);
    }

    /// Among rewrites the longest start wins, not the first — and the rest of
    /// the identifier is carried over.
    #[test]
    fn the_longest_rewrite_wins() {
        let (directory, catalog) = catalog(
            r#"<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
                 <rewriteURI uriStartString="http://example.com/" rewritePrefix="all/"/>
                 <rewriteURI uriStartString="http://example.com/deep/" rewritePrefix="deep/"/>
               </catalog>"#,
        );
        assert_eq!(
            catalog.resolve_uri("http://example.com/deep/one.xsd"),
            Some(here(&directory, "deep/one.xsd"))
        );
        assert_eq!(
            catalog.resolve_uri("http://example.com/two.xsd"),
            Some(here(&directory, "all/two.xsd"))
        );
    }

    #[test]
    fn the_longest_suffix_wins() {
        let (directory, catalog) = catalog(
            r#"<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
                 <uriSuffix uriSuffix="topic.xsd" uri="short.xsd"/>
                 <uriSuffix uriSuffix="base/topic.xsd" uri="long.xsd"/>
                 <systemSuffix systemIdSuffix="map.xsd" uri="map.xsd"/>
               </catalog>"#,
        );
        assert_eq!(
            catalog.resolve_uri("http://example.com/base/topic.xsd"),
            Some(here(&directory, "long.xsd"))
        );
        assert_eq!(
            catalog.resolve_uri("http://example.com/other/topic.xsd"),
            Some(here(&directory, "short.xsd"))
        );
        assert_eq!(
            catalog.resolve_external_identifier(None, Some("x/map.xsd")),
            Some(here(&directory, "map.xsd"))
        );
    }

    /// An exact entry is tried before a rewrite, and a rewrite before a suffix.
    #[test]
    fn the_entry_types_are_tried_in_order() {
        let (directory, catalog) = catalog(
            r#"<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
                 <uriSuffix uriSuffix="one.xsd" uri="by-suffix.xsd"/>
                 <rewriteURI uriStartString="urn:example:" rewritePrefix="by-rewrite/"/>
                 <uri name="urn:example:one.xsd" uri="by-name.xsd"/>
               </catalog>"#,
        );
        // Exact wins over both, though it is written last.
        assert_eq!(
            catalog.resolve_uri("urn:example:one.xsd"),
            Some(here(&directory, "by-name.xsd"))
        );
        // And a rewrite wins over a suffix.
        assert_eq!(
            catalog.resolve_uri("urn:example:two/one.xsd"),
            Some(here(&directory, "by-rewrite/two/one.xsd"))
        );
    }

    /// `nextCatalog` is consulted only after the whole catalog has failed,
    /// whatever order the entries were written in.
    #[test]
    fn a_next_catalog_comes_after_everything_here() {
        let directory = TempDir::new().expect("a directory");
        write(
            &directory,
            "other.xml",
            r#"<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
                 <uri name="urn:example:shared" uri="theirs.xsd"/>
                 <uri name="urn:example:only-theirs" uri="only-theirs.xsd"/>
               </catalog>"#,
        );
        let at = write(
            &directory,
            "catalog.xml",
            r#"<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
                 <nextCatalog catalog="other.xml"/>
                 <uri name="urn:example:shared" uri="ours.xsd"/>
               </catalog>"#,
        );
        let catalog = XmlCatalog::from_file(&at).expect("the catalog loads");
        assert_eq!(
            catalog.resolve_uri("urn:example:shared"),
            Some(here(&directory, "ours.xsd")),
            "ours wins though the next catalog was named first"
        );
        assert_eq!(
            catalog.resolve_uri("urn:example:only-theirs"),
            Some(here(&directory, "only-theirs.xsd"))
        );
    }

    /// Delegation is terminal: once it matches, the catalogs it names are the
    /// only ones asked, and a miss is a miss — the `nextCatalog` that would
    /// have answered is never reached.
    ///
    /// Within one catalog there is nothing else for delegation to shadow,
    /// because every other entry type is tried before it (`uri`, then
    /// `rewriteURI`, then `uriSuffix`, then `delegateURI`). What delegation
    /// cuts off is the rest of the *list*.
    #[test]
    fn a_delegation_answers_alone() {
        let directory = TempDir::new().expect("a directory");
        write(
            &directory,
            "delegated.xml",
            r#"<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
                 <uri name="urn:example:known" uri="theirs.xsd"/>
               </catalog>"#,
        );
        write(
            &directory,
            "next.xml",
            r#"<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
                 <uri name="urn:example:unknown" uri="would-have.xsd"/>
                 <uri name="urn:other:thing" uri="other.xsd"/>
               </catalog>"#,
        );
        let at = write(
            &directory,
            "catalog.xml",
            r#"<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
                 <delegateURI uriStartString="urn:example:" catalog="delegated.xml"/>
                 <nextCatalog catalog="next.xml"/>
               </catalog>"#,
        );
        let catalog = XmlCatalog::from_file(&at).expect("the catalog loads");
        assert_eq!(
            catalog.resolve_uri("urn:example:known"),
            Some(here(&directory, "theirs.xsd"))
        );
        // Delegated and not found: the next catalog would have resolved it and
        // is not asked.
        assert_eq!(catalog.resolve_uri("urn:example:unknown"), None);
        // What the delegation does not cover reaches the next catalog as usual.
        assert_eq!(
            catalog.resolve_uri("urn:other:thing"),
            Some(here(&directory, "other.xsd"))
        );
    }

    /// A system identifier is tried first; `prefer` decides whether the public
    /// one is considered once it has failed.
    #[test]
    fn prefer_decides_whether_a_public_identifier_is_consulted() {
        let (directory, public_first) = catalog(
            r#"<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog" prefer="public">
                 <public publicId="-//EXAMPLE//DTD Thing//EN" uri="by-public.xsd"/>
                 <system systemId="http://example.com/thing.dtd" uri="by-system.xsd"/>
               </catalog>"#,
        );
        // The system identifier resolves, so it wins whatever `prefer` says.
        assert_eq!(
            public_first.resolve_external_identifier(
                Some("-//EXAMPLE//DTD Thing//EN"),
                Some("http://example.com/thing.dtd")
            ),
            Some(here(&directory, "by-system.xsd"))
        );
        // It does not resolve, and `prefer="public"` lets the public one answer.
        assert_eq!(
            public_first.resolve_external_identifier(
                Some("-//EXAMPLE//DTD Thing//EN"),
                Some("http://example.com/elsewhere.dtd")
            ),
            Some(here(&directory, "by-public.xsd"))
        );

        let (directory, system_only) = catalog(
            r#"<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog" prefer="system">
                 <public publicId="-//EXAMPLE//DTD Thing//EN" uri="by-public.xsd"/>
               </catalog>"#,
        );
        assert_eq!(
            system_only.resolve_external_identifier(
                Some("-//EXAMPLE//DTD Thing//EN"),
                Some("http://example.com/thing.dtd")
            ),
            None,
            "with a system identifier given, `prefer=system` leaves public entries out"
        );
        // With no system identifier there is nothing to prefer, so it answers.
        assert_eq!(
            system_only.resolve_external_identifier(Some("-//EXAMPLE//DTD Thing//EN"), None),
            Some(here(&directory, "by-public.xsd"))
        );
    }

    /// `xml:base` on an entry moves where its `uri` points, and a group's moves
    /// it for everything inside — nesting as it goes.
    ///
    /// This is how OASIS ships DITA: one catalog at the root of the bundle,
    /// every entry naming its own directory in `xml:base`.
    #[test]
    fn an_entry_resolves_against_its_xml_base() {
        let (directory, catalog) = catalog(
            r#"<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
                 <system systemId="urn:example:topicMod.xsd:1.2" uri="topicMod.xsd" xml:base="base/xsd/"/>
                 <uri name="urn:example:types.xsd" uri="types.xsd" xml:base="common/"/>
                 <system systemId="urn:example:plain.xsd" uri="plain.xsd"/>
                 <group xml:base="technicalContent/xsd/">
                   <system systemId="urn:example:uiDomain.xsd:1.2" uri="uiDomain.xsd"/>
                   <group xml:base="deeper/">
                     <system systemId="urn:example:nested.xsd" uri="nested.xsd"/>
                   </group>
                 </group>
               </catalog>"#,
        );
        let system = |id: &str| catalog.resolve_external_identifier(None, Some(id));
        assert_eq!(
            system("urn:example:topicMod.xsd:1.2"),
            Some(here(&directory, "base/xsd/topicMod.xsd"))
        );
        assert_eq!(
            catalog.resolve_uri("urn:example:types.xsd"),
            Some(here(&directory, "common/types.xsd"))
        );
        assert_eq!(
            system("urn:example:plain.xsd"),
            Some(here(&directory, "plain.xsd")),
            "an entry without one resolves beside the catalog"
        );
        assert_eq!(
            system("urn:example:uiDomain.xsd:1.2"),
            Some(here(&directory, "technicalContent/xsd/uiDomain.xsd"))
        );
        assert_eq!(
            system("urn:example:nested.xsd"),
            Some(here(&directory, "technicalContent/xsd/deeper/nested.xsd"))
        );
    }

    /// A `urn:publicid:` URI is a public identifier written as a URN
    /// (RFC 3151), and resolves as one from either side.
    #[test]
    fn a_publicid_urn_is_a_public_identifier() {
        let (directory, catalog) = catalog(
            r#"<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
                 <public publicId="-//EXAMPLE//DTD Thing 1.0//EN" uri="thing.dtd"/>
               </catalog>"#,
        );
        let urn = "urn:publicid:-:EXAMPLE:DTD+Thing+1.0:EN";
        assert_eq!(catalog.resolve_uri(urn), Some(here(&directory, "thing.dtd")));
        assert_eq!(
            catalog.resolve_external_identifier(None, Some(urn)),
            Some(here(&directory, "thing.dtd"))
        );
    }

    /// Whitespace in a public identifier is normalised before comparison, so a
    /// catalog and a document may lay it out differently.
    #[test]
    fn a_public_identifier_is_compared_with_its_whitespace_normalised() {
        let (directory, catalog) = catalog(
            r#"<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
                 <public publicId="-//EXAMPLE//DTD   Thing//EN" uri="thing.dtd"/>
               </catalog>"#,
        );
        assert_eq!(
            catalog.resolve_external_identifier(Some("-//EXAMPLE//DTD Thing//EN"), None),
            Some(here(&directory, "thing.dtd"))
        );
    }

    /// A URI is compared escaped, so a space in a document matches `%20` in
    /// the catalog.
    #[test]
    fn a_uri_is_compared_escaped() {
        let (directory, catalog) = catalog(
            r#"<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
                 <uri name="http://example.com/a%20name.xsd" uri="spaced.xsd"/>
               </catalog>"#,
        );
        assert_eq!(
            catalog.resolve_uri("http://example.com/a name.xsd"),
            Some(here(&directory, "spaced.xsd"))
        );
    }

    /// A catalog that names itself, directly or in a ring, is read once rather
    /// than for ever.
    #[test]
    fn a_catalog_that_names_itself_terminates() {
        let directory = TempDir::new().expect("a directory");
        write(
            &directory,
            "b.xml",
            r#"<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
                 <nextCatalog catalog="catalog.xml"/>
                 <uri name="urn:example:b" uri="b.xsd"/>
               </catalog>"#,
        );
        let at = write(
            &directory,
            "catalog.xml",
            r#"<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
                 <nextCatalog catalog="b.xml"/>
                 <uri name="urn:example:a" uri="a.xsd"/>
               </catalog>"#,
        );
        let catalog = XmlCatalog::from_file(&at).expect("the catalog loads");
        assert_eq!(
            catalog.resolve_uri("urn:example:a"),
            Some(here(&directory, "a.xsd"))
        );
        assert_eq!(
            catalog.resolve_uri("urn:example:b"),
            Some(here(&directory, "b.xsd"))
        );
        assert_eq!(catalog.resolve_uri("urn:example:neither"), None);
    }

    /// A `nextCatalog` that is not there does not sink the catalog that names
    /// it — but it is remembered and can be reported.
    ///
    /// A bundle commonly ships a catalog naming modules it does not carry:
    /// DITA-LCE's names three directories that are not in it, and tolerating
    /// them is what lets it resolve at all. What must not happen is silence,
    /// and `missing` is how it is broken.
    #[test]
    fn a_catalog_that_cannot_be_read_is_remembered() {
        let directory = TempDir::new().expect("a directory");
        let at = write(
            &directory,
            "catalog.xml",
            r#"<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
                 <nextCatalog catalog="absent.xml"/>
                 <uri name="urn:example:here" uri="here.xsd"/>
               </catalog>"#,
        );
        let catalog = XmlCatalog::from_file(&at).expect("the catalog still loads");
        assert_eq!(
            catalog.resolve_uri("urn:example:here"),
            Some(here(&directory, "here.xsd")),
            "what it does carry still resolves"
        );
        let missing = catalog.missing();
        assert_eq!(missing.len(), 1, "{missing:?}");
        assert!(missing[0].contains("absent.xml"), "it says which: {}", missing[0]);
        // And one with everything it names has nothing to report.
        let whole = write(&directory, "whole.xml", r#"<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog"/>"#);
        assert!(XmlCatalog::from_file(&whole).expect("it loads").missing().is_empty());
    }

    #[test]
    fn something_that_is_not_a_catalog_is_refused() {
        let directory = TempDir::new().expect("a directory");
        let at = write(&directory, "catalog.xml", "<notacatalog/>");
        assert!(XmlCatalog::from_file(&at).is_err());
        let at = write(
            &directory,
            "wrong-ns.xml",
            r#"<catalog xmlns="urn:example:not-a-catalog"/>"#,
        );
        assert!(XmlCatalog::from_file(&at).is_err());
    }

    #[test]
    fn a_bad_prefer_is_refused() {
        let directory = TempDir::new().expect("a directory");
        let at = write(
            &directory,
            "catalog.xml",
            r#"<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog" prefer="neither"/>"#,
        );
        assert!(XmlCatalog::from_file(&at).is_err());
    }

    #[test]
    fn a_location_is_resolved_as_a_uri_and_then_as_a_system_identifier() {
        let (directory, catalog) = catalog(
            r#"<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
                 <uri name="urn:example:by-uri" uri="by-uri.xsd"/>
                 <system systemId="urn:example:by-system" uri="by-system.xsd"/>
               </catalog>"#,
        );
        assert_eq!(
            catalog.resolve("urn:example:by-uri"),
            Some(here(&directory, "by-uri.xsd"))
        );
        assert_eq!(
            catalog.resolve("urn:example:by-system"),
            Some(here(&directory, "by-system.xsd"))
        );
    }

    #[test]
    fn merging_keeps_the_first_catalogs_precedence() {
        let (directory, mut ours) = catalog(
            r#"<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
                 <uri name="urn:example:shared" uri="ours.xsd"/>
               </catalog>"#,
        );
        let (theirs_directory, theirs) = catalog(
            r#"<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
                 <uri name="urn:example:shared" uri="theirs.xsd"/>
                 <uri name="urn:example:only-theirs" uri="only-theirs.xsd"/>
               </catalog>"#,
        );
        ours.merge(&theirs);
        assert_eq!(
            ours.resolve_uri("urn:example:shared"),
            Some(here(&directory, "ours.xsd"))
        );
        assert_eq!(
            ours.resolve_uri("urn:example:only-theirs"),
            Some(here(&theirs_directory, "only-theirs.xsd"))
        );
        assert_eq!(ours.len(), 3);
        assert!(!ours.is_empty());
    }
}
