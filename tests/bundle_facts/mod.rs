//! Static facts about schema bundles for assertion in tests.
//!
//! These modules define known facts about the DITA and NISO STS standards
//! that can be asserted after parsing schemas.

pub mod dita_facts;
pub mod niso_facts;

pub use dita_facts::DitaFacts;
pub use niso_facts::NisoFacts;
