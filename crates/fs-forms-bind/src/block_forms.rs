//! The question "which forms does a block draw" as a port, for every consumer that must not link the block catalog.
//!
//! Pattern: Port (consumer side; the host gives the adapter over the block catalog, this crate links no component crate).

use crate::{BlockSpec, FormRef};

/// Pattern: Port (consumer side) – what the block catalog says; the host implements it over its catalog of blocks.
pub trait BlockForms {
    /// The forms the component of `block` draws (`forms=` of the block contract); none for a block that draws no whole form.
    fn forms(&self, block: &str) -> Vec<FormRef>;

    /// The block with its slots (the `heads` of a grouping slot are the forms its heads come as), or `None` for an unknown block.
    fn spec(&self, block: &str) -> Option<BlockSpec>;
}
