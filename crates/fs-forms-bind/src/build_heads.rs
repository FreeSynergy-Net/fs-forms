//! The heads rule of a grouping slot (`bind-error-heads-form-missing`): the records of the head source are the groups (the
//! lanes); the module that serves them must declare a form entry for exactly the data type of that source, in a form version
//! the slot takes (`heads=` of the block contract).
//!
//! Pattern: Specification. A pure predicate over (slot, source, module declarations); the result is a lock on the SLOT only,
//! never on the placement: the block lives, only the lanes slot is locked.

use crate::build_place::Env;
use crate::coded::{Coded, Family, SlotRole};
use crate::codes::{BindCode, Cause, Part, PartLock};
use crate::forms::FormRef;
use crate::model::{BindingIn, PlaceIn};

/// `None` = the heads are fine; otherwise the one lock of the slot (source locked, source unknown, form entry missing).
pub(crate) fn heads_lock(
    env: &Env<'_>,
    wanted: &[FormRef],
    heads: u32,
    slot: &Part,
) -> Option<PartLock> {
    let Some(src) = crate::plan_query::source_by_number(env.sources, heads) else {
        let c = Coded::new("layout-error-source-unknown")
            .of(Family::Layout)
            .for_slot(SlotRole::Heads)
            .with("index", heads.to_string());
        return Some(PartLock::locked(slot.clone(), c));
    };
    let ready = match &src.status {
        Ok(r) => r,
        Err(l) => {
            return Some(PartLock {
                part: slot.clone(),
                severity: l.severity,
                cause: Cause::from_coded(l.cause.coded().clone().for_slot(SlotRole::Heads)),
            });
        }
    };
    let declared = env.facts.forms(&ready.capability);
    let fits = declared.iter().any(|m| {
        m.data_type == src.data_type
            && wanted.iter().any(|w| {
                w.name == m.form.name && w.major == m.form.major && m.form.minor >= w.minor
            })
    });
    if fits {
        return None;
    }
    let form = wanted
        .iter()
        .map(|w| format!("{}@{}.{}", w.name, w.major, w.minor))
        .collect::<Vec<_>>()
        .join(",");
    let c = BindCode::HeadsFormMissing
        .coded()
        .with("form", form)
        .with("type", src.data_type.clone());
    let c = c.for_slot(SlotRole::Heads);
    Some(PartLock::locked(slot.clone(), c))
}

/// The rule for one grouping binding: the heads forms come from the slot of the block (`heads=` of the contract).
pub(crate) fn heads_of_slot(
    env: &Env<'_>,
    p: &PlaceIn,
    b: &BindingIn,
    heads: u32,
    slot: &Part,
) -> Option<PartLock> {
    let wanted = env
        .catalog
        .need(&p.block)
        .ok()
        .and_then(|s| s.slots.iter().find(|s| s.name == b.slot))
        .map_or(&[][..], |s| s.heads.as_slice());
    heads_lock(env, wanted, heads, slot)
}
