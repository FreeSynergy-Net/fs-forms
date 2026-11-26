//! The check of the gestures an `[[action]]` binds: with `at` the trigger is the NAME of a command slot of the block at that place
//! (`drop`, `menu`, `quick-add`, `activate`), so the place must exist and hold a block, the block must have the slot, the slot must be a
//! command slot, and no more actions may be bound to it than its `repeat`. Without `at` only the triggers of the page remain (`button`,
//! `key`, `submit`); a gesture has no place then and is refused. Offline: the catalog is enough, no provider is asked.
//!
//! Pattern: Specification (one pure rule set over actions and catalog); the catalog is the Strategy [`CatalogRules`].

use std::collections::HashMap;

use crate::check::CatalogRules;
use crate::codes::LayoutCode;
use crate::doc::{LayoutDoc, Trigger};
use crate::ids::ElemId;
use crate::shim::Coded;
use crate::words::Words;

type Problems = Vec<Coded<LayoutCode>>;

fn bad(elem: &str, cause: &str) -> Coded<LayoutCode> {
    Coded::new(LayoutCode::ActionInvalid)
        .with("elem", elem)
        .with("cause", cause)
}

/// The triggers a page has itself (no command slot behind them).
fn of_the_page(t: Trigger) -> bool {
    matches!(t, Trigger::Button | Trigger::Key | Trigger::Submit)
}

/// Runs the rule on every action; only appends problems.
pub(crate) fn check_actions(doc: &LayoutDoc, catalog: &dyn CatalogRules, out: &mut Problems) {
    // block name by element id of its place, all pages
    let blocks: HashMap<&ElemId, &str> = doc
        .pages
        .iter()
        .flat_map(|p| p.v.leaves())
        .filter_map(|l| l.elem.as_ref().map(|e| (e, l.v.block.as_str())))
        .collect();
    let mut used: HashMap<(&ElemId, Trigger), u32> = HashMap::new();
    for a in &doc.actions {
        let elem = a.elem.as_ref().map_or("", ElemId::as_str);
        let Some(at) = &a.v.at else {
            if !of_the_page(a.v.trigger) {
                out.push(bad(elem, "at-missing").with("trigger", a.v.trigger.word()));
            }
            continue;
        };
        let slot = a.v.trigger.word();
        let Some(block) = blocks.get(at) else {
            out.push(bad(elem, "at-unknown").with("at", at.as_str()));
            continue;
        };
        if !catalog.block_known(block) {
            continue; // reported as `block-unknown` by the names check
        }
        if !catalog.slot_known(block, slot) {
            out.push(
                bad(elem, "slot-unknown")
                    .with("block", *block)
                    .with("slot", slot),
            );
        } else if catalog.command_args(block, slot).is_none() {
            out.push(
                bad(elem, "slot-not-command")
                    .with("block", *block)
                    .with("slot", slot),
            );
        } else {
            let n = used.entry((at, a.v.trigger)).or_default();
            *n += 1;
            if *n > catalog.slot_repeat(block, slot) {
                out.push(
                    bad(elem, "repeat-exceeded")
                        .with("block", *block)
                        .with("slot", slot),
                );
            }
        }
    }
}
