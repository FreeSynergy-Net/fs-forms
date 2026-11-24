//! The block catalog of the contract (`fs-components-catalog`, A5 C2) as the rules the layout check asks (`CatalogRules`):
//! which blocks and slots exist, which places a frame declares, which attribute names a block takes.
//!
//! Pattern: Adapter (the catalog's `BlockCatalog` in, the `CatalogRules` Strategy of `fs-forms-layout` out). The contract names
//! CLASSES a place takes (`command`, `form`, `frame`, `value`, `any`), the layout check compares NAMES; the adapter turns each class
//! into the names of the blocks of that class, read from the catalog (no table of block names here):
//! `frame` = the block holds places; `value` = it draws a field (`renders`); `command` = it has a command slot and no records slot;
//! `form` = it draws forms or has a records slot; `any` = no limit (an empty list).

use fs_components_catalog::{BlockCatalog, BlockSpec, Max, SlotKind, Takes};
use fs_forms_layout::{CatalogRules, PlaceRule};

/// The rules of one catalog.
#[derive(Clone, Copy, Debug)]
pub struct ContractRules<'a>(pub &'a BlockCatalog);

fn is_of_class(b: &BlockSpec, class: Takes) -> bool {
    let kind = |k: SlotKind| b.accepts.iter().any(|s| s.kind == k);
    match class {
        Takes::Frame => !b.places.is_empty(),
        Takes::Value => b.renders.is_some(),
        Takes::Command => kind(SlotKind::Command) && !kind(SlotKind::Records),
        Takes::Form => !b.forms.is_empty() || kind(SlotKind::Records),
        Takes::Any => true,
    }
}

impl ContractRules<'_> {
    /// The names a place takes: empty = anything (`any`, or a class this reader does not know).
    fn names(self, takes: &[Takes]) -> Vec<String> {
        if takes.iter().any(|t| matches!(t, Takes::Any)) {
            return Vec::new();
        }
        let mut out: Vec<String> = self
            .0
            .iter()
            .filter(|b| takes.iter().any(|t| is_of_class(b, *t)))
            .map(|b| b.name.clone())
            .collect();
        out.sort();
        out.dedup();
        out
    }
}

impl CatalogRules for ContractRules<'_> {
    fn block_known(&self, block: &str) -> bool {
        self.0.block(block).is_some()
    }

    fn slot_known(&self, block: &str, slot: &str) -> bool {
        self.0.block(block).is_some_and(|b| b.slot(slot).is_some())
    }

    fn slot_repeat(&self, block: &str, slot: &str) -> u32 {
        self.0
            .block(block)
            .and_then(|b| b.slot(slot))
            .map_or(0, |s| s.repeat)
    }

    fn places_of(&self, frame: &str) -> Option<Vec<PlaceRule>> {
        let b = self.0.block(frame)?;
        // a block without `places` says nothing: no place is checked (the flat pages of before)
        if b.places.is_empty() {
            return None;
        }
        Some(
            b.places
                .iter()
                .map(|p| PlaceRule {
                    place: p.place.clone(),
                    takes: self.names(&p.takes),
                    min: p.min,
                    max: match p.max {
                        Max::Count(n) => n,
                        Max::Many => u32::MAX,
                    },
                })
                .collect(),
        )
    }

    fn sets_of(&self, name: &str) -> Option<Vec<String>> {
        let b = self.0.block(name)?;
        Some(b.sets.clone())
    }
}

impl crate::blocks::Blocks {
    /// The rules of the contract this build is made with (what a layout check against the real catalog asks).
    #[must_use]
    pub fn rules() -> ContractRules<'static> {
        ContractRules(crate::blocks::contract())
    }
}
