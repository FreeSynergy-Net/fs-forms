//! The catalog the host asks: which block does this build of `fs-components` draw? The block was already chosen inside the plan;
//! the host never picks a block by type (H-K2).
//!
//! Pattern: Port (`Catalog`) with a test double. The real catalog (`renders` and `choose`, G7) replaces `TestCatalog`.

use std::collections::BTreeSet;

/// What the host needs from the catalog.
pub trait Catalog {
    /// Whether this build draws the block.
    fn built(&self, block: &str) -> bool;

    /// Whether this build draws the block as an element of a page. A frame is built (it is a screen) but is no element; the
    /// default is `built`.
    fn draws(&self, block: &str) -> bool {
        self.built(block)
    }
}

/// A catalog that knows the blocks it is told (tests and the first frame, until G7).
#[derive(Clone, Debug, Default)]
pub struct TestCatalog(BTreeSet<String>);

impl TestCatalog {
    /// A catalog that builds exactly these blocks.
    #[must_use]
    pub fn building(blocks: &[&str]) -> Self {
        Self(blocks.iter().map(|b| (*b).to_owned()).collect())
    }
}

impl Catalog for TestCatalog {
    fn built(&self, block: &str) -> bool {
        self.0.contains(block)
    }
}
