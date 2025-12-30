//! XSD Model Group validators
//!
//! This module implements model groups for XSD content models:
//! - xs:sequence - ordered content
//! - xs:choice - alternative content
//! - xs:all - unordered content (XSD 1.0: elements only, XSD 1.1: any particles)
//!
//! Reference: https://www.w3.org/TR/xmlschema11-1/#Model_Groups

use crate::error::ParseError;
use crate::namespaces::QName;
use std::sync::Arc;

use super::elements::XsdElement;
use super::particles::{Occurs, OccursCalculator, Particle};
use super::wildcards::XsdAnyElement;

/// Model group compositor type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModelType {
    /// Ordered sequence of particles
    #[default]
    Sequence,
    /// One of multiple alternatives
    Choice,
    /// Unordered set of particles (XSD 1.0: elements only)
    All,
}

impl ModelType {
    /// Parse from element tag name
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "sequence" | "{http://www.w3.org/2001/XMLSchema}sequence" => Some(Self::Sequence),
            "choice" | "{http://www.w3.org/2001/XMLSchema}choice" => Some(Self::Choice),
            "all" | "{http://www.w3.org/2001/XMLSchema}all" => Some(Self::All),
            _ => None,
        }
    }
}

impl std::fmt::Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sequence => write!(f, "sequence"),
            Self::Choice => write!(f, "choice"),
            Self::All => write!(f, "all"),
        }
    }
}

/// A particle in a model group (element, wildcard, or nested group)
#[derive(Debug, Clone)]
pub enum GroupParticle {
    /// Element reference
    Element(Arc<ElementParticle>),
    /// Wildcard (xs:any)
    Any(Arc<XsdAnyElement>),
    /// Nested model group
    Group(Arc<XsdGroup>),
}

impl GroupParticle {
    /// Get the occurrence constraints
    pub fn occurs(&self) -> Occurs {
        match self {
            Self::Element(e) => e.occurs,
            Self::Any(a) => a.occurs(),
            Self::Group(g) => g.occurs,
        }
    }

    /// Check if this particle is emptiable
    pub fn is_emptiable(&self) -> bool {
        match self {
            Self::Element(e) => e.occurs.is_emptiable(),
            Self::Any(a) => a.is_emptiable(),
            Self::Group(g) => g.is_emptiable(),
        }
    }

    /// Get effective minimum occurs
    pub fn effective_min_occurs(&self) -> u32 {
        match self {
            Self::Element(e) => e.occurs.min,
            Self::Any(a) => a.min_occurs(),
            Self::Group(g) => g.effective_min_occurs(),
        }
    }

    /// Get effective maximum occurs (None = unbounded)
    pub fn effective_max_occurs(&self) -> Option<u32> {
        match self {
            Self::Element(e) => e.occurs.max,
            Self::Any(a) => a.max_occurs(),
            Self::Group(g) => g.effective_max_occurs(),
        }
    }
}

/// Element particle in a model group
#[derive(Debug, Clone)]
pub struct ElementParticle {
    /// Element name
    pub name: QName,
    /// Occurrence constraints
    pub occurs: Occurs,
    /// Reference to the actual element declaration (if available)
    pub element_ref: Option<QName>,
    /// Actual element declaration for local elements
    pub element_decl: Option<Arc<XsdElement>>,
}

impl ElementParticle {
    /// Create a new element particle
    pub fn new(name: QName, occurs: Occurs) -> Self {
        Self {
            name,
            occurs,
            element_ref: None,
            element_decl: None,
        }
    }

    /// Create with element reference
    pub fn with_ref(name: QName, occurs: Occurs, element_ref: QName) -> Self {
        Self {
            name,
            occurs,
            element_ref: Some(element_ref),
            element_decl: None,
        }
    }

    /// Create with local element declaration
    pub fn with_decl(name: QName, occurs: Occurs, element_decl: Arc<XsdElement>) -> Self {
        Self {
            name,
            occurs,
            element_ref: None,
            element_decl: Some(element_decl),
        }
    }

    /// Get the element declaration if available
    pub fn element(&self) -> Option<&Arc<XsdElement>> {
        self.element_decl.as_ref()
    }
}

impl Particle for ElementParticle {
    fn occurs(&self) -> Occurs {
        self.occurs
    }
}

/// XSD Model Group (sequence, choice, all)
#[derive(Debug, Clone)]
pub struct XsdGroup {
    /// Optional name for named model groups
    pub name: Option<QName>,
    /// Model type (sequence, choice, all)
    pub model: ModelType,
    /// Particles in this group
    pub particles: Vec<GroupParticle>,
    /// Occurrence constraints
    pub occurs: Occurs,
    /// Whether content is mixed (text + elements)
    pub mixed: bool,
    /// Reference to another group (for xs:group ref="...")
    pub group_ref: Option<QName>,
    /// Back-reference to original group when this is a redefinition (xs:redefine)
    pub redefine: Option<Arc<XsdGroup>>,
    /// Parse errors
    errors: Vec<ParseError>,
}

impl XsdGroup {
    /// Create a new model group
    pub fn new(model: ModelType) -> Self {
        Self {
            name: None,
            model,
            particles: Vec::new(),
            occurs: Occurs::once(),
            mixed: false,
            group_ref: None,
            redefine: None,
            errors: Vec::new(),
        }
    }

    /// Create a named model group
    pub fn named(name: QName, model: ModelType) -> Self {
        Self {
            name: Some(name),
            model,
            particles: Vec::new(),
            occurs: Occurs::once(),
            mixed: false,
            group_ref: None,
            redefine: None,
            errors: Vec::new(),
        }
    }

    /// Create a group reference
    pub fn reference(ref_name: QName, occurs: Occurs) -> Self {
        Self {
            name: None,
            model: ModelType::Sequence, // Will be resolved from referenced group
            particles: Vec::new(),
            occurs,
            mixed: false,
            group_ref: Some(ref_name),
            redefine: None,
            errors: Vec::new(),
        }
    }

    /// Add a particle to the group
    pub fn add_particle(&mut self, particle: GroupParticle) {
        self.particles.push(particle);
    }

    /// Add an element particle
    pub fn add_element(&mut self, name: QName, occurs: Occurs) {
        self.particles.push(GroupParticle::Element(Arc::new(
            ElementParticle::new(name, occurs),
        )));
    }

    /// Add a wildcard particle
    pub fn add_any(&mut self, any: XsdAnyElement) {
        self.particles.push(GroupParticle::Any(Arc::new(any)));
    }

    /// Add a nested group
    pub fn add_group(&mut self, group: XsdGroup) {
        self.particles.push(GroupParticle::Group(Arc::new(group)));
    }

    /// Check if group is empty
    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }

    /// Check if group can produce empty content
    pub fn is_emptiable(&self) -> bool {
        if self.occurs.min == 0 {
            return true;
        }
        if self.particles.is_empty() {
            return true;
        }

        match self.model {
            // Choice is emptiable if any branch is emptiable
            ModelType::Choice => self.particles.iter().any(|p| p.is_emptiable()),
            // Sequence/All is emptiable only if all particles are emptiable
            ModelType::Sequence | ModelType::All => {
                self.particles.iter().all(|p| p.is_emptiable())
            }
        }
    }

    /// Check if this is a single-occurrence group
    pub fn is_single(&self) -> bool {
        if self.occurs.max != Some(1) || self.particles.is_empty() {
            return false;
        }

        if self.particles.len() > 1 {
            return true;
        }

        // Single nested group - delegate
        match &self.particles[0] {
            GroupParticle::Group(g) => g.is_single(),
            _ => true,
        }
    }

    /// Check if this group is "pointless" and can be eliminated
    pub fn is_pointless(&self, parent_model: ModelType) -> bool {
        if self.particles.is_empty() {
            return true;
        }
        if self.occurs != Occurs::once() {
            return false;
        }
        if self.particles.len() == 1 {
            return true;
        }

        // Same model type as parent can be flattened
        self.model == parent_model
    }

    /// Calculate effective minimum occurs
    pub fn effective_min_occurs(&self) -> u32 {
        if self.occurs.min == 0 || self.particles.is_empty() {
            return 0;
        }

        let effective_items: Vec<_> = self
            .particles
            .iter()
            .filter(|p| p.effective_max_occurs() != Some(0))
            .collect();

        if effective_items.is_empty() {
            return 0;
        }

        match self.model {
            ModelType::Choice => {
                let min = effective_items
                    .iter()
                    .map(|p| p.effective_min_occurs())
                    .min()
                    .unwrap_or(0);
                self.occurs.min * min
            }
            ModelType::All => {
                effective_items
                    .iter()
                    .map(|p| p.effective_min_occurs())
                    .max()
                    .unwrap_or(0)
            }
            ModelType::Sequence => {
                let not_emptiable: Vec<_> = effective_items
                    .iter()
                    .filter(|p| p.effective_min_occurs() > 0)
                    .collect();

                if not_emptiable.is_empty() {
                    0
                } else if not_emptiable.len() > 1 {
                    self.occurs.min
                } else {
                    self.occurs.min * not_emptiable[0].effective_min_occurs()
                }
            }
        }
    }

    /// Calculate effective maximum occurs
    pub fn effective_max_occurs(&self) -> Option<u32> {
        if self.occurs.max == Some(0) || self.particles.is_empty() {
            return Some(0);
        }

        let effective_items: Vec<_> = self
            .particles
            .iter()
            .filter(|p| p.effective_max_occurs() != Some(0))
            .collect();

        if effective_items.is_empty() {
            return Some(0);
        }

        if self.occurs.max.is_none() {
            return None;
        }

        match self.model {
            ModelType::Choice => {
                if effective_items
                    .iter()
                    .any(|p| p.effective_max_occurs().is_none())
                {
                    None
                } else {
                    let max = effective_items
                        .iter()
                        .filter_map(|p| p.effective_max_occurs())
                        .max()
                        .unwrap_or(0);
                    Some(self.occurs.max.unwrap() * max)
                }
            }
            ModelType::All | ModelType::Sequence => {
                let not_emptiable: Vec<_> = effective_items
                    .iter()
                    .filter(|p| p.effective_min_occurs() > 0)
                    .collect();

                if not_emptiable.is_empty() {
                    if effective_items
                        .iter()
                        .any(|p| p.effective_max_occurs().is_none())
                    {
                        None
                    } else {
                        let max = effective_items
                            .iter()
                            .filter_map(|p| p.effective_max_occurs())
                            .max()
                            .unwrap_or(0);
                        Some(self.occurs.max.unwrap() * max)
                    }
                } else if not_emptiable.len() > 1 {
                    if self.model == ModelType::Sequence {
                        self.occurs.max
                    } else if not_emptiable
                        .iter()
                        .all(|p| p.effective_max_occurs().is_none())
                    {
                        None
                    } else {
                        not_emptiable
                            .iter()
                            .filter_map(|p| p.effective_max_occurs())
                            .min()
                    }
                } else {
                    match not_emptiable[0].effective_max_occurs() {
                        None => None,
                        Some(max) => Some(self.occurs.max.unwrap() * max),
                    }
                }
            }
        }
    }

    /// Calculate combined occurs using OccursCalculator
    pub fn calculate_occurs(&self) -> OccursCalculator {
        let mut calc = OccursCalculator::new();
        let mut first = true;

        for particle in &self.particles {
            let particle_occurs = Occurs::new(
                particle.effective_min_occurs(),
                particle.effective_max_occurs(),
            );

            match self.model {
                ModelType::Sequence | ModelType::All => {
                    calc.add(particle_occurs);
                }
                ModelType::Choice => {
                    // For choice: first particle initializes, rest use max_with
                    if first {
                        calc.add(particle_occurs);
                        first = false;
                    } else {
                        calc.max_with(particle_occurs);
                    }
                }
            }
        }

        calc.multiply(self.occurs);
        calc
    }

    /// Get parse errors
    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }

    /// Add a parse error
    pub fn add_error(&mut self, error: ParseError) {
        self.errors.push(error);
    }

    /// Iterate over direct particles
    pub fn iter(&self) -> impl Iterator<Item = &GroupParticle> {
        self.particles.iter()
    }

    /// Get number of particles
    pub fn len(&self) -> usize {
        self.particles.len()
    }
}

impl Particle for XsdGroup {
    fn occurs(&self) -> Occurs {
        self.occurs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_type_from_tag() {
        assert_eq!(ModelType::from_tag("sequence"), Some(ModelType::Sequence));
        assert_eq!(ModelType::from_tag("choice"), Some(ModelType::Choice));
        assert_eq!(ModelType::from_tag("all"), Some(ModelType::All));
        assert_eq!(ModelType::from_tag("invalid"), None);
    }

    #[test]
    fn test_group_creation() {
        let group = XsdGroup::new(ModelType::Sequence);
        assert_eq!(group.model, ModelType::Sequence);
        assert!(group.particles.is_empty());
        assert_eq!(group.occurs, Occurs::once());
    }

    #[test]
    fn test_named_group() {
        let group = XsdGroup::named(QName::local("myGroup"), ModelType::Choice);
        assert_eq!(group.name, Some(QName::local("myGroup")));
        assert_eq!(group.model, ModelType::Choice);
    }

    #[test]
    fn test_add_elements() {
        let mut group = XsdGroup::new(ModelType::Sequence);
        group.add_element(QName::local("first"), Occurs::once());
        group.add_element(QName::local("second"), Occurs::optional());
        group.add_element(QName::local("third"), Occurs::zero_or_more());

        assert_eq!(group.len(), 3);
    }

    #[test]
    fn test_is_emptiable_sequence() {
        let mut group = XsdGroup::new(ModelType::Sequence);

        // Empty sequence is emptiable
        assert!(group.is_emptiable());

        // Add required element - no longer emptiable
        group.add_element(QName::local("required"), Occurs::once());
        assert!(!group.is_emptiable());

        // Add optional element - still not emptiable due to required
        group.add_element(QName::local("optional"), Occurs::optional());
        assert!(!group.is_emptiable());
    }

    #[test]
    fn test_is_emptiable_choice() {
        let mut group = XsdGroup::new(ModelType::Choice);

        // Add required and optional alternatives
        group.add_element(QName::local("required"), Occurs::once());
        group.add_element(QName::local("optional"), Occurs::optional());

        // Choice is emptiable if any branch is emptiable
        assert!(group.is_emptiable());
    }

    #[test]
    fn test_effective_min_occurs_sequence() {
        let mut group = XsdGroup::new(ModelType::Sequence);
        group.add_element(QName::local("a"), Occurs::new(2, Some(5)));
        group.add_element(QName::local("b"), Occurs::optional());

        // Only 'a' is required (min=2), 'b' is optional
        assert_eq!(group.effective_min_occurs(), 2);
    }

    #[test]
    fn test_effective_min_occurs_choice() {
        let mut group = XsdGroup::new(ModelType::Choice);
        group.add_element(QName::local("a"), Occurs::new(2, Some(5)));
        group.add_element(QName::local("b"), Occurs::new(1, Some(3)));

        // Choice: minimum of all branches
        assert_eq!(group.effective_min_occurs(), 1);
    }

    #[test]
    fn test_effective_max_occurs() {
        let mut group = XsdGroup::new(ModelType::Choice);
        group.add_element(QName::local("a"), Occurs::new(1, Some(5)));
        group.add_element(QName::local("b"), Occurs::new(1, Some(3)));

        // Choice: maximum of all branches
        assert_eq!(group.effective_max_occurs(), Some(5));
    }

    #[test]
    fn test_effective_max_occurs_unbounded() {
        let mut group = XsdGroup::new(ModelType::Choice);
        group.add_element(QName::local("a"), Occurs::new(1, Some(5)));
        group.add_element(QName::local("b"), Occurs::one_or_more());

        // Choice with unbounded branch
        assert_eq!(group.effective_max_occurs(), None);
    }

    #[test]
    fn test_is_pointless() {
        // Empty group is pointless
        let empty = XsdGroup::new(ModelType::Sequence);
        assert!(empty.is_pointless(ModelType::Sequence));

        // Single element with occurs (1,1) is pointless
        let mut single = XsdGroup::new(ModelType::Sequence);
        single.add_element(QName::local("elem"), Occurs::once());
        assert!(single.is_pointless(ModelType::Sequence));

        // Multiple elements with occurs (1,1) and same model is pointless
        let mut multi = XsdGroup::new(ModelType::Sequence);
        multi.add_element(QName::local("a"), Occurs::once());
        multi.add_element(QName::local("b"), Occurs::once());
        assert!(multi.is_pointless(ModelType::Sequence));
        assert!(!multi.is_pointless(ModelType::Choice)); // Different model

        // Group with different occurs is not pointless
        let mut repeated = XsdGroup::new(ModelType::Sequence);
        repeated.occurs = Occurs::zero_or_more();
        repeated.add_element(QName::local("elem"), Occurs::once());
        assert!(!repeated.is_pointless(ModelType::Sequence));
    }

    #[test]
    fn test_nested_group() {
        let mut inner = XsdGroup::new(ModelType::Sequence);
        inner.add_element(QName::local("a"), Occurs::once());
        inner.add_element(QName::local("b"), Occurs::once());

        let mut outer = XsdGroup::new(ModelType::Choice);
        outer.add_group(inner);
        outer.add_element(QName::local("c"), Occurs::once());

        assert_eq!(outer.len(), 2);
    }

    #[test]
    fn test_calculate_occurs_sequence() {
        let mut group = XsdGroup::new(ModelType::Sequence);
        group.add_element(QName::local("a"), Occurs::new(1, Some(2)));
        group.add_element(QName::local("b"), Occurs::new(2, Some(3)));

        let calc = group.calculate_occurs();
        assert_eq!(calc.min_occurs, 3); // 1 + 2
        assert_eq!(calc.max_occurs, Some(5)); // 2 + 3
    }

    #[test]
    fn test_calculate_occurs_choice() {
        let mut group = XsdGroup::new(ModelType::Choice);
        group.add_element(QName::local("a"), Occurs::new(1, Some(2)));
        group.add_element(QName::local("b"), Occurs::new(3, Some(5)));

        let calc = group.calculate_occurs();
        assert_eq!(calc.min_occurs, 1); // min(1, 3)
        assert_eq!(calc.max_occurs, Some(5)); // max(2, 5)
    }

    #[test]
    fn test_group_reference() {
        let group = XsdGroup::reference(
            QName::namespaced("http://example.com", "myGroup"),
            Occurs::zero_or_more(),
        );

        assert!(group.group_ref.is_some());
        assert_eq!(group.occurs, Occurs::zero_or_more());
    }
}
