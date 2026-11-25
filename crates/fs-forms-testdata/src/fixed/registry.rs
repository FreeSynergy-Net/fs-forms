//! The names of the fixed sources.
//!
//! Pattern: Registry – name -> source; `standard_fixed()` like `standard_makers()`. An unknown name is `fixed-unknown`.

use super::{EventsSample, FixedSource};
use crate::error::{Failure, SampleCode, fail};

/// Name -> fixed source.
pub struct FixedRegistry {
    sources: Vec<Box<dyn FixedSource>>,
}

impl FixedRegistry {
    /// An empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Adds a source; one with the same name is replaced.
    #[must_use]
    pub fn with(mut self, source: Box<dyn FixedSource>) -> Self {
        self.sources.retain(|s| s.name() != source.name());
        self.sources.push(source);
        self
    }

    /// The source named `name`.
    ///
    /// # Errors
    /// `fixed-unknown`.
    pub fn get(&self, name: &str) -> Result<&dyn FixedSource, Failure> {
        self.sources
            .iter()
            .find(|s| s.name() == name)
            .map(AsRef::as_ref)
            .ok_or_else(|| fail(SampleCode::FixedUnknown, "fixed", name))
    }

    /// The names, sorted.
    #[must_use]
    pub fn names(&self) -> Vec<&str> {
        let mut out: Vec<&str> = self.sources.iter().map(|s| s.name()).collect();
        out.sort_unstable();
        out
    }
}

impl Default for FixedRegistry {
    fn default() -> Self {
        standard_fixed()
    }
}

/// The fixed sources this crate brings: the adapters on the fixed samples of the contracts.
#[must_use]
pub fn standard_fixed() -> FixedRegistry {
    FixedRegistry::new().with(Box::new(EventsSample))
}
