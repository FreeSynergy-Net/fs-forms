//! The port `Catalog` (consumer side) and the small `accepts` check bind needs.
//!
//! Pattern: Port. The adapter over the real block catalog follows when G7 (`renders` + `choose`) exists.

use crate::coded::{Coded, Family};
use crate::model::BindingIn;
use crate::types::{FieldDesc, Role};

/// One slot a block offers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotSpec {
    /// Slot name.
    pub name: String,
    /// Fewest bindings.
    pub min: u32,
    /// Most bindings (`None` = unbounded).
    pub max: Option<u32>,
    /// Forms the heads of a grouping slot come as (`heads=` of the slot, e.g. `rows.0`); empty = the slot takes no heads.
    pub heads: Vec<crate::forms::FormRef>,
}

/// A block of the block contract with its `accepts`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockSpec {
    /// Block name.
    pub name: String,
    /// Slots.
    pub slots: Vec<SlotSpec>,
}

/// What bind needs from the block catalog.
pub trait Catalog {
    /// The block with its `accepts`, or `accepts-error-block-unknown`.
    ///
    /// # Errors
    /// The coded `accepts-error-block-unknown` when the catalog does not know the block.
    fn need(&self, block: &str) -> Result<&BlockSpec, Coded>;
    /// The ONE block for a field in a role. The host never picks (H-K2); never fails for a known basic.
    fn choose(&self, field: &FieldDesc, role: Role) -> String;
    /// Does this fs-components version build the block?
    fn built(&self, block: &str) -> bool;
}

/// Checks the bindings of one placement against the block's slots; returns every code, never only the first.
#[must_use]
pub fn check_bindings(spec: &BlockSpec, bindings: &[BindingIn]) -> Vec<Coded> {
    let mut out = Vec::new();
    for b in bindings {
        if !spec.slots.iter().any(|s| s.name == b.slot) {
            out.push(
                Coded::new("accepts-error-slot-unknown")
                    .of(Family::Accepts)
                    .with("block", spec.name.clone())
                    .with("slot", b.slot.clone()),
            );
        }
    }
    for s in &spec.slots {
        let n =
            u32::try_from(bindings.iter().filter(|b| b.slot == s.name).count()).unwrap_or(u32::MAX);
        if n < s.min {
            out.push(
                Coded::new("accepts-error-slot-missing")
                    .of(Family::Accepts)
                    .with("slot", s.name.clone())
                    .with("min", s.min.to_string()),
            );
        }
        if s.max.is_some_and(|m| n > m) {
            out.push(
                Coded::new("accepts-error-slot-too-many")
                    .of(Family::Accepts)
                    .with("slot", s.name.clone())
                    .with("got", n.to_string()),
            );
        }
    }
    out
}
