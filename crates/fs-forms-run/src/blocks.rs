//! The blocks this build of the host draws: the CATALOG says which blocks exist (`fs-components-catalog`, read from the block
//! contract of the very tag the host is built with); the host adds only what it can stand behind: the frames it shows as screens,
//! the state blocks the window draws itself, and the functions of `draw` that put a component behind a name. A block the contract
//! does not know is `host-error-block-unknown`; one it knows and this build cannot draw yet is `host-error-block-not-built`;
//! both are visible in the window, never dropped.
//!
//! Pattern: Abstract Factory seen from the host: it asks the catalog, then calls only the function of the block named.

use std::sync::OnceLock;

use fs_components_catalog::{BlockCatalog, FormRef};

use crate::catalog::Catalog;
use crate::draw;

mod contract {
    include!(concat!(env!("OUT_DIR"), "/contract.rs"));
}

/// The frames: the screens of the shell.
const FRAMES: &[&str] = &["workbench", "detail-page", "sidebar"];

/// The blocks of the states the window draws itself (a notice, an empty page, a placeholder).
const STATE_BLOCKS: &[&str] = &["empty-state", "partial-notice", "skeleton"];

/// The catalog of the contract, read once. A contract that cannot be read leaves it empty: then every block is unknown and says so.
fn catalog() -> &'static BlockCatalog {
    static CATALOG: OnceLock<BlockCatalog> = OnceLock::new();
    CATALOG.get_or_init(|| read().unwrap_or_default())
}

fn read() -> Result<BlockCatalog, usize> {
    BlockCatalog::read(contract::CONTRACT, contract::BLOCK_FILES).map_err(|problems| problems.len())
}

/// The forms the component of `block` says it draws (`forms=` of the block contract); none for a block that draws no whole form.
pub(crate) fn forms_of(block: &str) -> Vec<FormRef> {
    catalog()
        .block(block)
        .map_or_else(Vec::new, |b| b.forms.clone())
}

/// Every block of the contract that draws a form, with its forms.
pub(crate) fn forms_of_blocks() -> Vec<(String, Vec<FormRef>)> {
    catalog()
        .iter()
        .filter(|b| !b.forms.is_empty())
        .map(|b| (b.name.clone(), b.forms.clone()))
        .collect()
}

/// The registry of this build.
#[derive(Clone, Copy, Debug, Default)]
pub struct Blocks;

impl Blocks {
    /// Whether the block is a frame: a screen of the shell, not an element inside it.
    #[must_use]
    pub fn is_frame(block: &str) -> bool {
        FRAMES.contains(&block)
    }
}

impl Catalog for Blocks {
    fn known(&self, block: &str) -> bool {
        catalog().block(block).is_some()
    }

    fn built(&self, block: &str) -> bool {
        self.known(block) && (Self::is_frame(block) || self.draws(block))
    }

    fn draws(&self, block: &str) -> bool {
        self.known(block) && (STATE_BLOCKS.contains(&block) || draw::has(block))
    }
}

#[cfg(test)]
mod tests {
    use super::{FRAMES, STATE_BLOCKS, catalog, read};
    use crate::catalog::Catalog;

    #[test]
    fn the_contract_of_the_catalog_reads_without_a_problem() {
        assert_eq!(read().map(|c| c.len() > 50), Ok(true));
    }

    #[test]
    fn every_frame_and_state_block_of_the_host_is_a_block_of_the_contract() {
        for name in FRAMES.iter().chain(STATE_BLOCKS) {
            assert!(
                catalog().block(name).is_some(),
                "{name} is not in the contract"
            );
        }
    }

    #[test]
    fn a_name_the_contract_does_not_have_is_unknown_and_never_built() {
        let b = super::Blocks;
        assert!(
            !b.known("no-such-block") && !b.built("no-such-block") && !b.draws("no-such-block")
        );
        assert!(
            b.known("board-detail") && !b.built("board-detail"),
            "known, not drawn yet"
        );
    }
}
