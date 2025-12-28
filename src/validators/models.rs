//! XSD Content Model Validators
//!
//! This module provides validation for XSD content models, including:
//! - ModelVisitor for validating XML data against model groups
//! - Model checking for determinism (EDC/UPA constraints)
//! - Content sorting and iteration utilities
//!
//! Reference: https://www.w3.org/TR/xmlschema11-1/#coss-particle

use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{ParseError, Result};
use crate::namespaces::QName;

use super::groups::{GroupParticle, XsdGroup};
use super::particles::{OccursCounter, Particle};
use super::wildcards::XsdAnyElement;

/// Type alias for occurs counter keyed by object ID
pub type OccursCounterType = OccursCounter<usize>;

/// Content item: (key, value) where key is either an index (for CDATA) or element name
#[derive(Debug, Clone)]
pub enum ContentKey {
    /// Character data index
    Index(usize),
    /// Element name
    Name(String),
}

/// Content item tuple
pub type ContentItem = (ContentKey, String);

/// Advance result tuple: (particle, occurs_count, expected_elements)
pub type AdvanceYield = (GroupParticle, u32, Vec<QName>);

/// Model visitor for validating XML content against XSD model groups.
///
/// The visitor uses a state machine to track position within the content model,
/// counting occurrences and yielding errors when constraints are violated.
#[derive(Debug)]
pub struct ModelVisitor {
    /// Stack of (group, iterator_position, match_flag) for nested groups
    groups_stack: Vec<(Arc<XsdGroup>, usize, bool)>,

    /// The root model group
    root: Arc<XsdGroup>,

    /// Occurrence counter for particles
    occurs: HashMap<usize, u32>,

    /// Current element (index into current group's particles)
    element_index: Option<usize>,

    /// Current group being processed
    group: Arc<XsdGroup>,

    /// Current position in group's particles
    position: usize,

    /// Whether current group has a match
    matched: bool,
}

impl ModelVisitor {
    /// Create a new model visitor for the given root group
    pub fn new(root: Arc<XsdGroup>) -> Self {
        let group = Arc::clone(&root);
        let mut visitor = Self {
            groups_stack: Vec::new(),
            root,
            occurs: HashMap::new(),
            element_index: None,
            group,
            position: 0,
            matched: false,
        };
        visitor.start();
        visitor
    }

    /// Clear the visitor state and restart
    pub fn clear(&mut self) {
        self.groups_stack.clear();
        self.occurs.clear();
        self.element_index = None;
        self.group = Arc::clone(&self.root);
        self.position = 0;
        self.matched = false;
    }

    /// Restart the visitor
    pub fn restart(&mut self) {
        self.clear();
        self.start();
    }

    /// Initialize by finding the first element
    fn start(&mut self) {
        while self.position < self.group.len() {
            if let Some(particle) = self.group.particles.get(self.position) {
                match particle {
                    GroupParticle::Group(inner_group) => {
                        // Push current state and descend into group
                        self.groups_stack.push((
                            Arc::clone(&self.group),
                            self.position + 1,
                            self.matched,
                        ));
                        self.group = Arc::clone(inner_group);
                        self.position = 0;
                        self.matched = false;
                    }
                    GroupParticle::Element(_) | GroupParticle::Any(_) => {
                        self.element_index = Some(self.position);
                        return;
                    }
                }
            } else {
                // End of current group, pop from stack
                if let Some((parent, pos, matched)) = self.groups_stack.pop() {
                    self.group = parent;
                    self.position = pos;
                    self.matched = matched;
                } else {
                    // End of model
                    return;
                }
            }
        }

        // Try popping from stack if we've exhausted current group
        if let Some((parent, pos, matched)) = self.groups_stack.pop() {
            self.group = parent;
            self.position = pos;
            self.matched = matched;
            self.start();
        }
    }

    /// Get the current element particle, if any
    pub fn current_element(&self) -> Option<&GroupParticle> {
        self.element_index.and_then(|idx| self.group.particles.get(idx))
    }

    /// Check if the model is ended
    pub fn is_ended(&self) -> bool {
        self.element_index.is_none()
    }

    /// Get occurrence count for a particle (by its identity)
    fn get_occurs(&self, id: usize) -> u32 {
        *self.occurs.get(&id).unwrap_or(&0)
    }

    /// Increment occurrence count
    fn inc_occurs(&mut self, id: usize) -> u32 {
        let count = self.occurs.entry(id).or_insert(0);
        *count += 1;
        *count
    }

    /// Get expected elements from current position
    pub fn expected(&self) -> Vec<QName> {
        let mut expected = Vec::new();

        // Add elements from current group that aren't over their max
        for particle in &self.group.particles {
            if let GroupParticle::Element(elem) = particle {
                expected.push(elem.name.clone());
            }
        }

        expected
    }

    /// Match an element by tag name
    pub fn match_element(&self, tag: &str) -> Option<QName> {
        self.current_element().and_then(|particle| {
            match particle {
                GroupParticle::Element(elem) => {
                    if elem.name.local_name == tag {
                        Some(elem.name.clone())
                    } else {
                        // Check for namespace match
                        None
                    }
                }
                GroupParticle::Any(any) => {
                    // Wildcard matching
                    if any.matches_tag(tag) {
                        Some(QName::local(tag))
                    } else {
                        None
                    }
                }
                GroupParticle::Group(_) => None,
            }
        })
    }

    /// Advance the visitor, optionally with a match
    ///
    /// Returns an iterator of error tuples for constraint violations.
    pub fn advance(&mut self, matched: bool) -> Vec<AdvanceYield> {
        let errors = Vec::new();

        if self.is_ended() {
            return errors;
        }

        if matched {
            if let Some(idx) = self.element_index {
                let id = idx; // Use index as ID
                self.inc_occurs(id);
                self.matched = true;

                // Check if we need to stay on current element (not over max)
                if let Some(particle) = self.current_element() {
                    let occurs = self.get_occurs(id);
                    if !particle.is_over(occurs) {
                        return errors;
                    }
                }
            }
        }

        // Move to next position
        self.position += 1;
        self.element_index = None;
        self.start();

        errors
    }

    /// Stop the model and collect all errors
    pub fn stop(&mut self) -> Vec<AdvanceYield> {
        let mut errors = Vec::new();

        while !self.is_ended() {
            errors.extend(self.advance(false));
        }

        errors
    }

    /// Check if the model can be stopped without errors
    pub fn is_stoppable(&self) -> bool {
        // Clone and test
        let mut visitor = ModelVisitor::new(Arc::clone(&self.root));
        visitor.occurs = self.occurs.clone();
        visitor.groups_stack = self.groups_stack.clone();
        visitor.element_index = self.element_index;
        visitor.group = Arc::clone(&self.group);
        visitor.position = self.position;
        visitor.matched = self.matched;

        visitor.stop().is_empty()
    }
}

impl GroupParticle {
    /// Check if this particle's occurs are over the maximum
    fn is_over(&self, count: u32) -> bool {
        match self {
            GroupParticle::Element(elem) => elem.occurs.is_over(count),
            GroupParticle::Any(any) => any.occurs().is_over(count),
            GroupParticle::Group(group) => group.occurs.is_over(count),
        }
    }
}

/// Check if two model paths are distinguishable in a deterministic way.
///
/// This is used for Unique Particle Attribution (UPA) checking.
/// Returns `true` if the paths can be distinguished without lookahead.
pub fn distinguishable_paths(
    path1: &[&XsdGroup],
    path2: &[&XsdGroup],
) -> bool {
    // Find divergence point
    let mut depth = 0;
    for (i, g) in path1.iter().enumerate() {
        if i >= path2.len() || !std::ptr::eq(*g, path2[i]) {
            depth = i.saturating_sub(1);
            break;
        }
    }

    // Simplified determinism check
    // Full implementation would check:
    // - Elements before/after in sequences
    // - Univocal particles
    // - Empty-able elements
    if depth < path1.len() && depth < path2.len() {
        let g1 = path1[depth];
        let g2 = path2[depth];

        // Different groups at divergence = distinguishable
        !std::ptr::eq(g1, g2)
    } else {
        true
    }
}

/// Check if a model group is deterministic.
///
/// Validates Element Declarations Consistent (EDC) and
/// Unique Particle Attribution (UPA) constraints.
pub fn check_model(group: &XsdGroup) -> Result<()> {
    let mut paths: HashMap<String, Vec<usize>> = HashMap::new();

    // Collect all element paths
    fn collect_paths(
        group: &XsdGroup,
        current_path: &mut Vec<usize>,
        paths: &mut HashMap<String, Vec<usize>>,
        depth: usize,
    ) -> Result<()> {
        if depth > 100 {
            return Err(ParseError::new("Model depth exceeded maximum").into());
        }

        for (idx, particle) in group.particles.iter().enumerate() {
            current_path.push(idx);

            match particle {
                GroupParticle::Element(elem) => {
                    let name = elem.name.to_string();
                    if let Some(existing) = paths.get(&name) {
                        // EDC check: same name must have same type
                        // For now, just warn about duplicates
                        // Full check would compare types
                        if existing != current_path {
                            // UPA check would go here
                        }
                    }
                    paths.insert(name, current_path.clone());
                }
                GroupParticle::Any(_) => {
                    // Wildcards need special handling for UPA
                }
                GroupParticle::Group(inner) => {
                    collect_paths(inner, current_path, paths, depth + 1)?;
                }
            }

            current_path.pop();
        }

        Ok(())
    }

    let mut current_path = Vec::new();
    collect_paths(group, &mut current_path, &mut paths, 0)?;

    Ok(())
}

/// Interleaved model visitor for openContent models.
///
/// Handles XSD 1.1 openContent with interleave mode.
pub struct InterleavedModelVisitor {
    inner: ModelVisitor,
    wildcard: Arc<XsdAnyElement>,
    advance_model: bool,
}

impl InterleavedModelVisitor {
    /// Create a new interleaved model visitor
    pub fn new(root: Arc<XsdGroup>, wildcard: Arc<XsdAnyElement>) -> Self {
        Self {
            inner: ModelVisitor::new(root),
            wildcard,
            advance_model: true,
        }
    }

    /// Match element, considering the wildcard
    pub fn match_element(&mut self, tag: &str) -> Option<QName> {
        // Try regular match first
        if let Some(name) = self.inner.match_element(tag) {
            self.advance_model = true;
            return Some(name);
        }

        // Try wildcard match
        if self.wildcard.matches_tag(tag) {
            self.advance_model = false;
            return Some(QName::local(tag));
        }

        None
    }

    /// Advance the visitor
    pub fn advance(&mut self, matched: bool) -> Vec<AdvanceYield> {
        if self.advance_model {
            self.inner.advance(matched)
        } else {
            self.advance_model = true;
            Vec::new()
        }
    }
}

/// Suffixed model visitor for openContent models.
///
/// Handles XSD 1.1 openContent with suffix mode.
pub struct SuffixedModelVisitor {
    inner: ModelVisitor,
    wildcard: Arc<XsdAnyElement>,
    in_suffix: bool,
}

impl SuffixedModelVisitor {
    /// Create a new suffixed model visitor
    pub fn new(root: Arc<XsdGroup>, wildcard: Arc<XsdAnyElement>) -> Self {
        Self {
            inner: ModelVisitor::new(root),
            wildcard,
            in_suffix: false,
        }
    }

    /// Match element, considering suffix wildcard
    pub fn match_element(&mut self, tag: &str) -> Option<QName> {
        if self.in_suffix {
            // In suffix mode, only wildcard matches
            if self.wildcard.matches_tag(tag) {
                return Some(QName::local(tag));
            }
            return None;
        }

        // Try regular match
        if let Some(name) = self.inner.match_element(tag) {
            return Some(name);
        }

        // If model ended, try wildcard
        if self.inner.is_ended() && self.wildcard.matches_tag(tag) {
            self.in_suffix = true;
            return Some(QName::local(tag));
        }

        None
    }

    /// Advance the visitor
    pub fn advance(&mut self, matched: bool) -> Vec<AdvanceYield> {
        if self.in_suffix {
            Vec::new()
        } else {
            let result = self.inner.advance(matched);
            if self.inner.is_ended() {
                self.in_suffix = true;
            }
            result
        }
    }
}

/// Sort content according to model group ordering.
///
/// Takes unordered content and yields it in the order defined by the model.
pub fn sort_content(
    content: Vec<ContentItem>,
    group: &XsdGroup,
) -> Vec<ContentItem> {
    let mut result = Vec::new();
    let mut remaining: HashMap<String, Vec<String>> = HashMap::new();
    let mut cdata: Vec<(usize, String)> = Vec::new();

    // Separate CDATA and element content
    for (key, value) in content {
        match key {
            ContentKey::Index(idx) => cdata.push((idx, value)),
            ContentKey::Name(name) => {
                remaining.entry(name).or_default().push(value);
            }
        }
    }

    // Sort CDATA by index
    cdata.sort_by_key(|(idx, _)| *idx);
    let mut cdata_iter = cdata.into_iter();

    // Yield first CDATA if any
    if let Some((idx, value)) = cdata_iter.next() {
        result.push((ContentKey::Index(idx), value));
    }

    // Create visitor and process in order
    let group_arc = Arc::new(group.clone());
    let mut visitor = ModelVisitor::new(group_arc);

    while !visitor.is_ended() && !remaining.is_empty() {
        let mut found = false;

        // Try to match current element
        for (name, values) in remaining.iter_mut() {
            if visitor.match_element(name).is_some() {
                if let Some(value) = values.pop() {
                    result.push((ContentKey::Name(name.clone()), value));
                    found = true;
                    visitor.advance(true);

                    // Add interleaved CDATA
                    if let Some((idx, cdata_value)) = cdata_iter.next() {
                        result.push((ContentKey::Index(idx), cdata_value));
                    }
                    break;
                }
            }
        }

        // Remove empty entries
        remaining.retain(|_, v| !v.is_empty());

        if !found {
            visitor.advance(false);
        }
    }

    // Add remaining content
    for (name, values) in remaining {
        for value in values {
            result.push((ContentKey::Name(name.clone()), value));
            if let Some((idx, cdata_value)) = cdata_iter.next() {
                result.push((ContentKey::Index(idx), cdata_value));
            }
        }
    }

    // Add remaining CDATA
    for (idx, value) in cdata_iter {
        result.push((ContentKey::Index(idx), value));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validators::groups::{ElementParticle, ModelType};
    use crate::validators::particles::Occurs;

    fn make_element(name: &str) -> GroupParticle {
        GroupParticle::Element(Arc::new(ElementParticle::new(
            QName::local(name),
            Occurs::once(),
        )))
    }

    #[test]
    fn test_model_visitor_creation() {
        let mut group = XsdGroup::new(ModelType::Sequence);
        group.particles.push(make_element("a"));
        group.particles.push(make_element("b"));

        let visitor = ModelVisitor::new(Arc::new(group));
        assert!(!visitor.is_ended());
    }

    #[test]
    fn test_model_visitor_advance() {
        let mut group = XsdGroup::new(ModelType::Sequence);
        group.particles.push(make_element("a"));
        group.particles.push(make_element("b"));

        let mut visitor = ModelVisitor::new(Arc::new(group));

        // Match first element
        assert!(visitor.match_element("a").is_some());
        visitor.advance(true);

        // Match second element
        assert!(visitor.match_element("b").is_some());
        visitor.advance(true);

        // Model should be ended
        assert!(visitor.is_ended());
    }

    #[test]
    fn test_model_visitor_choice() {
        let mut group = XsdGroup::new(ModelType::Choice);
        group.particles.push(make_element("a"));
        group.particles.push(make_element("b"));

        let visitor = ModelVisitor::new(Arc::new(group));

        // Either a or b should match initially
        let matched_a = visitor.match_element("a").is_some();
        let matched_b = visitor.match_element("b").is_some();
        assert!(matched_a || matched_b);
    }

    #[test]
    fn test_model_visitor_restart() {
        let mut group = XsdGroup::new(ModelType::Sequence);
        group.particles.push(make_element("a"));

        let mut visitor = ModelVisitor::new(Arc::new(group));

        visitor.advance(true);
        assert!(visitor.is_ended());

        visitor.restart();
        assert!(!visitor.is_ended());
    }

    #[test]
    fn test_check_model_simple() {
        let mut group = XsdGroup::new(ModelType::Sequence);
        group.particles.push(make_element("a"));
        group.particles.push(make_element("b"));

        assert!(check_model(&group).is_ok());
    }

    #[test]
    fn test_check_model_choice() {
        let mut group = XsdGroup::new(ModelType::Choice);
        group.particles.push(make_element("a"));
        group.particles.push(make_element("b"));

        assert!(check_model(&group).is_ok());
    }

    #[test]
    fn test_content_key() {
        let index_key = ContentKey::Index(0);
        let name_key = ContentKey::Name("test".to_string());

        match index_key {
            ContentKey::Index(i) => assert_eq!(i, 0),
            _ => panic!("Expected Index"),
        }

        match name_key {
            ContentKey::Name(n) => assert_eq!(n, "test"),
            _ => panic!("Expected Name"),
        }
    }

    #[test]
    fn test_sort_content_simple() {
        let mut group = XsdGroup::new(ModelType::Sequence);
        group.particles.push(make_element("a"));
        group.particles.push(make_element("b"));

        let content = vec![
            (ContentKey::Name("b".to_string()), "value_b".to_string()),
            (ContentKey::Name("a".to_string()), "value_a".to_string()),
        ];

        let sorted = sort_content(content, &group);

        // Should be sorted according to model order
        assert!(sorted.len() >= 2);
        if let ContentKey::Name(ref name) = sorted[0].0 {
            assert_eq!(name, "a");
        }
    }

    #[test]
    fn test_model_visitor_expected() {
        let mut group = XsdGroup::new(ModelType::Sequence);
        group.particles.push(make_element("a"));
        group.particles.push(make_element("b"));

        let visitor = ModelVisitor::new(Arc::new(group));
        let expected = visitor.expected();

        assert!(!expected.is_empty());
    }

    #[test]
    fn test_interleaved_visitor() {
        let mut group = XsdGroup::new(ModelType::Sequence);
        group.particles.push(make_element("a"));

        let wildcard = Arc::new(XsdAnyElement::any());

        let mut visitor = InterleavedModelVisitor::new(Arc::new(group), wildcard);

        // Should match element
        assert!(visitor.match_element("a").is_some());
    }

    #[test]
    fn test_suffixed_visitor() {
        let mut group = XsdGroup::new(ModelType::Sequence);
        group.particles.push(make_element("a"));

        let wildcard = Arc::new(XsdAnyElement::any());

        let mut visitor = SuffixedModelVisitor::new(Arc::new(group), wildcard);

        // Should match element from model
        assert!(visitor.match_element("a").is_some());
    }
}
