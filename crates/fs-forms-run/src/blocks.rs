//! The blocks this build of the host draws, all from `fs-components`: the registry that says which block is built and builds it.
//! The workbench blocks (`board`, `table`, ...) join when the adapter that feeds them is released; until then a placement with
//! such a block is locked with `host-error-block-not-built`, visibly.
//!
//! Pattern: Abstract Factory over `fs-components` (one table, block name to builder; the host has no other).

use crate::catalog::Catalog;

/// The frames (the screens of the shell) and the blocks of the state surfaces.
const BUILT: &[&str] = &[
    "workbench",
    "detail-page",
    "sidebar",
    "empty-state",
    "partial-notice",
    "skeleton",
];

/// The registry of this build.
#[derive(Clone, Copy, Debug, Default)]
pub struct Blocks;

impl Blocks {
    /// Whether the block is a frame: a screen of the shell, not an element inside it.
    #[must_use]
    pub fn is_frame(block: &str) -> bool {
        matches!(block, "workbench" | "detail-page" | "sidebar")
    }
}

impl Catalog for Blocks {
    fn built(&self, block: &str) -> bool {
        BUILT.contains(&block)
    }
}
