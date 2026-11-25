//! The block catalog of this build as the port the test data asks (`fs_forms_binames("board") == ["cards-in-columns"],d::BlockForms`): the same knowledge as `feed::feeds_of`
//! and `feed::wants_of` have of the description, here of the blocks; nothing is decided, the contract answers.
//!
//! Pattern: Adapter (the contract of the catalog in, the port of `fs-forms-bind` out).

use fs_forms_bind::{BlockForms, BlockSpec, FormRef};

use crate::bind_spec::block_spec;
use crate::blocks::forms_of;
use crate::form_check::drawn;

/// The adapter over the block contract this host is built with.
#[derive(Clone, Copy, Debug, Default)]
pub struct CatalogBlockForms;

impl BlockForms for CatalogBlockForms {
    fn forms(&self, block: &str) -> Vec<FormRef> {
        forms_of(block).iter().filter_map(drawn).collect()
    }

    fn spec(&self, block: &str) -> Option<BlockSpec> {
        block_spec(block)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(block: &str) -> Vec<String> {
        CatalogBlockForms
            .forms(block)
            .into_iter()
            .map(|f| f.name)
            .collect()
    }

    #[test]
    fn the_blocks_that_draw_a_whole_form_say_which_over_the_real_catalog() {
        for block in ["board", "tree", "table", "calendar-grid"] {
            assert!(!names(block).is_empty(), "{block} says no form");
            assert!(CatalogBlockForms.spec(block).is_some(), "{block} unknown");
        }
        assert_eq!(names("board"), ["cards-in-columns"]);
    }

    #[test]
    fn a_block_the_contract_does_not_have_has_no_forms_and_no_spec() {
        assert_eq!(names("no-such-block"), Vec::<String>::new());
        assert!(CatalogBlockForms.spec("no-such-block").is_none());
    }
}
