//! XSD Particle Schema Components
//!
//! This module implements the particle model for XSD elements, groups, and wildcards.
//! Particles define occurrence constraints (minOccurs, maxOccurs) for schema components.
//!
//! Reference: https://www.w3.org/TR/xmlschema11-1/#p

use crate::error::{ParseError, Result};
use std::collections::HashMap;
use std::hash::Hash;

/// Occurrence bounds for a particle (minOccurs, maxOccurs)
/// None for max_occurs means unbounded
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Occurs {
    /// Minimum number of occurrences (default 1)
    pub min: u32,
    /// Maximum number of occurrences (None = unbounded, default 1)
    pub max: Option<u32>,
}

impl Occurs {
    /// Create new occurrence bounds
    pub fn new(min: u32, max: Option<u32>) -> Self {
        Self { min, max }
    }

    /// Default occurrence (1, 1)
    pub fn once() -> Self {
        Self { min: 1, max: Some(1) }
    }

    /// Optional occurrence (0, 1)
    pub fn optional() -> Self {
        Self { min: 0, max: Some(1) }
    }

    /// Zero or more (0, unbounded)
    pub fn zero_or_more() -> Self {
        Self { min: 0, max: None }
    }

    /// One or more (1, unbounded)
    pub fn one_or_more() -> Self {
        Self { min: 1, max: None }
    }

    /// Empty (0, 0)
    pub fn empty() -> Self {
        Self { min: 0, max: Some(0) }
    }

    /// Check if this particle can be empty (minOccurs == 0)
    pub fn is_emptiable(&self) -> bool {
        self.min == 0
    }

    /// Check if this particle is empty (maxOccurs == 0)
    pub fn is_empty(&self) -> bool {
        self.max == Some(0)
    }

    /// Check if particle has maxOccurs == 1
    pub fn is_single(&self) -> bool {
        self.max == Some(1)
    }

    /// Check if particle can have multiple occurrences
    pub fn is_multiple(&self) -> bool {
        !self.is_empty() && !self.is_single()
    }

    /// Check if minOccurs != maxOccurs
    pub fn is_ambiguous(&self) -> bool {
        match self.max {
            Some(max) => self.min != max,
            None => true,
        }
    }

    /// Check if minOccurs == maxOccurs
    pub fn is_univocal(&self) -> bool {
        !self.is_ambiguous()
    }

    /// Check if occurrence count is under the minimum
    pub fn is_missing(&self, count: u32) -> bool {
        count < self.min
    }

    /// Check if occurrence count is at or over the maximum
    pub fn is_over(&self, count: u32) -> bool {
        match self.max {
            Some(max) => count >= max,
            None => false,
        }
    }

    /// Check if occurrence count exceeds the maximum
    pub fn is_exceeded(&self, count: u32) -> bool {
        match self.max {
            Some(max) => count > max,
            None => false,
        }
    }

    /// Check if this particle has valid occurs restriction compared to another
    pub fn has_occurs_restriction(&self, other: &Occurs) -> bool {
        // Self must have >= min_occurs than other
        if self.min < other.min {
            return false;
        }

        // If self is empty, it's always a valid restriction
        if self.max == Some(0) {
            return true;
        }

        // If other is unbounded, self can be anything
        if other.max.is_none() {
            return true;
        }

        // If self is unbounded but other isn't, not a valid restriction
        if self.max.is_none() {
            return false;
        }

        // Both have bounds - self must have <= max_occurs
        self.max.unwrap() <= other.max.unwrap()
    }
}

impl Default for Occurs {
    fn default() -> Self {
        Self::once()
    }
}

/// Trait for XSD components that have particle semantics
pub trait Particle {
    /// Get the occurrence bounds
    fn occurs(&self) -> Occurs;

    /// Get minimum occurrences
    fn min_occurs(&self) -> u32 {
        self.occurs().min
    }

    /// Get maximum occurrences (None = unbounded)
    fn max_occurs(&self) -> Option<u32> {
        self.occurs().max
    }

    /// Check if this particle can be empty
    fn is_emptiable(&self) -> bool {
        self.occurs().is_emptiable()
    }

    /// Check if this particle is empty (max = 0)
    fn is_empty(&self) -> bool {
        self.occurs().is_empty()
    }

    /// Check if this particle is single occurrence
    fn is_single(&self) -> bool {
        self.occurs().is_single()
    }

    /// Check if this particle can have multiple occurrences
    fn is_multiple(&self) -> bool {
        self.occurs().is_multiple()
    }
}

/// Parse minOccurs/maxOccurs from XML attribute values
pub fn parse_occurs(
    min_occurs: Option<&str>,
    max_occurs: Option<&str>,
) -> Result<Occurs> {
    let mut occurs = Occurs::once();

    // Parse minOccurs
    if let Some(min_str) = min_occurs {
        match min_str.parse::<u32>() {
            Ok(min) => occurs.min = min,
            Err(_) => {
                return Err(ParseError::new(
                    "minOccurs value is not a valid non-negative integer",
                )
                .into())
            }
        }
    }

    // Parse maxOccurs
    if let Some(max_str) = max_occurs {
        if max_str == "unbounded" {
            occurs.max = None;
        } else {
            match max_str.parse::<u32>() {
                Ok(max) => {
                    if occurs.min > max {
                        return Err(ParseError::new(
                            "maxOccurs must be 'unbounded' or greater than minOccurs",
                        )
                        .into());
                    }
                    occurs.max = Some(max);
                }
                Err(_) => {
                    return Err(ParseError::new(
                        "maxOccurs value must be a non-negative integer or 'unbounded'",
                    )
                    .into())
                }
            }
        }
    } else {
        // Default maxOccurs is 1, but must be >= minOccurs
        if occurs.min > 1 {
            return Err(ParseError::new(
                "minOccurs must be lesser or equal than maxOccurs",
            )
            .into());
        }
    }

    Ok(occurs)
}

/// Counter for tracking particle occurrences during validation
#[derive(Debug, Clone, Default)]
pub struct OccursCounter<K: Hash + Eq> {
    counts: HashMap<K, u32>,
}

impl<K: Hash + Eq> OccursCounter<K> {
    /// Create a new counter
    pub fn new() -> Self {
        Self {
            counts: HashMap::new(),
        }
    }

    /// Get the count for a key
    pub fn get(&self, key: &K) -> u32 {
        *self.counts.get(key).unwrap_or(&0)
    }

    /// Increment the count for a key
    pub fn increment(&mut self, key: K) -> u32 {
        let count = self.counts.entry(key).or_insert(0);
        *count += 1;
        *count
    }

    /// Reset the counter
    pub fn reset(&mut self) {
        self.counts.clear();
    }
}

/// Helper for calculating combined min/max occurs for model groups
#[derive(Debug, Clone, Copy, Default)]
pub struct OccursCalculator {
    /// Calculated minimum occurrences
    pub min_occurs: u32,
    /// Calculated maximum occurrences (None = unbounded)
    pub max_occurs: Option<u32>,
}

impl OccursCalculator {
    /// Create a new calculator initialized to (0, 0)
    pub fn new() -> Self {
        Self {
            min_occurs: 0,
            max_occurs: Some(0),
        }
    }

    /// Get as Occurs
    pub fn occurs(&self) -> Occurs {
        Occurs::new(self.min_occurs, self.max_occurs)
    }

    /// Add another particle's occurs (for sequence)
    pub fn add(&mut self, other: Occurs) {
        self.min_occurs += other.min;
        match (self.max_occurs, other.max) {
            (Some(a), Some(b)) => self.max_occurs = Some(a + b),
            _ => self.max_occurs = None,
        }
    }

    /// Multiply by another particle's occurs (for nested groups)
    pub fn multiply(&mut self, other: Occurs) {
        self.min_occurs *= other.min;
        match (self.max_occurs, other.max) {
            (None, Some(0)) => self.max_occurs = Some(0),
            (Some(0), _) => self.max_occurs = Some(0),
            (Some(_), None) => self.max_occurs = None,
            (None, _) => {}
            (Some(a), Some(b)) => self.max_occurs = Some(a * b),
        }
    }

    /// Subtract another particle's occurs
    pub fn subtract(&mut self, other: Occurs) {
        self.min_occurs = self.min_occurs.saturating_sub(other.min);
        match (self.max_occurs, other.max) {
            (Some(a), Some(b)) => self.max_occurs = Some(a.saturating_sub(b)),
            (Some(_), None) => self.max_occurs = Some(0),
            (None, _) => {}
        }
    }

    /// Reset to (0, 0)
    pub fn reset(&mut self) {
        self.min_occurs = 0;
        self.max_occurs = Some(0);
    }

    /// Take the max of this and another (for choice)
    pub fn max_with(&mut self, other: Occurs) {
        // For choice: min is the min of all branches, max is unbounded if any is
        self.min_occurs = self.min_occurs.min(other.min);
        match (self.max_occurs, other.max) {
            (None, _) | (_, None) => self.max_occurs = None,
            (Some(a), Some(b)) => self.max_occurs = Some(a.max(b)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_occurs_creation() {
        let occurs = Occurs::new(1, Some(5));
        assert_eq!(occurs.min, 1);
        assert_eq!(occurs.max, Some(5));
    }

    #[test]
    fn test_occurs_presets() {
        assert_eq!(Occurs::once(), Occurs::new(1, Some(1)));
        assert_eq!(Occurs::optional(), Occurs::new(0, Some(1)));
        assert_eq!(Occurs::zero_or_more(), Occurs::new(0, None));
        assert_eq!(Occurs::one_or_more(), Occurs::new(1, None));
        assert_eq!(Occurs::empty(), Occurs::new(0, Some(0)));
    }

    #[test]
    fn test_occurs_predicates() {
        let optional = Occurs::optional();
        assert!(optional.is_emptiable());
        assert!(!optional.is_empty());
        assert!(optional.is_single());
        assert!(!optional.is_multiple());

        let unbounded = Occurs::zero_or_more();
        assert!(unbounded.is_emptiable());
        assert!(!unbounded.is_empty());
        assert!(!unbounded.is_single());
        assert!(unbounded.is_multiple());

        let empty = Occurs::empty();
        assert!(empty.is_emptiable());
        assert!(empty.is_empty());
    }

    #[test]
    fn test_occurs_ambiguous() {
        assert!(!Occurs::once().is_ambiguous());
        assert!(Occurs::optional().is_ambiguous());
        assert!(Occurs::zero_or_more().is_ambiguous());
        assert!(!Occurs::new(5, Some(5)).is_ambiguous());
    }

    #[test]
    fn test_occurs_counting() {
        let occurs = Occurs::new(2, Some(5));
        assert!(occurs.is_missing(0));
        assert!(occurs.is_missing(1));
        assert!(!occurs.is_missing(2));
        assert!(!occurs.is_missing(3));

        assert!(!occurs.is_over(4));
        assert!(occurs.is_over(5));
        assert!(occurs.is_over(6));

        assert!(!occurs.is_exceeded(5));
        assert!(occurs.is_exceeded(6));
    }

    #[test]
    fn test_occurs_restriction() {
        let base = Occurs::new(1, Some(3));

        // Valid restrictions
        assert!(Occurs::new(1, Some(3)).has_occurs_restriction(&base));
        assert!(Occurs::new(2, Some(2)).has_occurs_restriction(&base));
        assert!(Occurs::new(1, Some(1)).has_occurs_restriction(&base));

        // Invalid restrictions
        assert!(!Occurs::new(0, Some(3)).has_occurs_restriction(&base)); // min too low
        assert!(!Occurs::new(1, Some(5)).has_occurs_restriction(&base)); // max too high
        assert!(!Occurs::new(1, None).has_occurs_restriction(&base)); // unbounded not valid
        assert!(!Occurs::empty().has_occurs_restriction(&base)); // min too low (0 < 1)

        // Unbounded base allows anything
        let unbounded_base = Occurs::new(1, None);
        assert!(Occurs::new(1, Some(100)).has_occurs_restriction(&unbounded_base));
        assert!(Occurs::new(1, None).has_occurs_restriction(&unbounded_base));

        // Optional base (0, 1)
        let optional_base = Occurs::optional();
        assert!(Occurs::empty().has_occurs_restriction(&optional_base)); // (0,0) is valid restriction of (0,1)
        assert!(Occurs::optional().has_occurs_restriction(&optional_base));
        assert!(Occurs::new(1, Some(1)).has_occurs_restriction(&optional_base)); // more restrictive min
    }

    #[test]
    fn test_parse_occurs_default() {
        let occurs = parse_occurs(None, None).unwrap();
        assert_eq!(occurs, Occurs::once());
    }

    #[test]
    fn test_parse_occurs_values() {
        let occurs = parse_occurs(Some("0"), Some("5")).unwrap();
        assert_eq!(occurs, Occurs::new(0, Some(5)));

        let occurs = parse_occurs(Some("1"), Some("unbounded")).unwrap();
        assert_eq!(occurs, Occurs::new(1, None));
    }

    #[test]
    fn test_parse_occurs_errors() {
        // Invalid minOccurs
        assert!(parse_occurs(Some("abc"), None).is_err());

        // Invalid maxOccurs
        assert!(parse_occurs(None, Some("abc")).is_err());

        // minOccurs > maxOccurs
        assert!(parse_occurs(Some("5"), Some("3")).is_err());

        // minOccurs > default maxOccurs (1)
        assert!(parse_occurs(Some("5"), None).is_err());
    }

    #[test]
    fn test_occurs_counter() {
        let mut counter: OccursCounter<&str> = OccursCounter::new();
        assert_eq!(counter.get(&"foo"), 0);

        assert_eq!(counter.increment("foo"), 1);
        assert_eq!(counter.increment("foo"), 2);
        assert_eq!(counter.get(&"foo"), 2);

        counter.reset();
        assert_eq!(counter.get(&"foo"), 0);
    }

    #[test]
    fn test_occurs_calculator_add() {
        let mut calc = OccursCalculator::new();
        calc.add(Occurs::new(1, Some(2)));
        assert_eq!(calc.min_occurs, 1);
        assert_eq!(calc.max_occurs, Some(2));

        calc.add(Occurs::new(2, Some(3)));
        assert_eq!(calc.min_occurs, 3);
        assert_eq!(calc.max_occurs, Some(5));

        calc.add(Occurs::new(1, None)); // unbounded
        assert_eq!(calc.min_occurs, 4);
        assert_eq!(calc.max_occurs, None);
    }

    #[test]
    fn test_occurs_calculator_multiply() {
        let mut calc = OccursCalculator::new();
        calc.add(Occurs::new(2, Some(3)));

        calc.multiply(Occurs::new(2, Some(4)));
        assert_eq!(calc.min_occurs, 4);
        assert_eq!(calc.max_occurs, Some(12));
    }

    #[test]
    fn test_occurs_calculator_max_with() {
        let mut calc = OccursCalculator::new();
        calc.add(Occurs::new(2, Some(3)));

        // For choice, take min of mins and max of maxes
        calc.max_with(Occurs::new(1, Some(5)));
        assert_eq!(calc.min_occurs, 1); // min(2, 1)
        assert_eq!(calc.max_occurs, Some(5)); // max(3, 5)
    }
}
