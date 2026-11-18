//! Layer 3: the result of applying overlays, and what `resolve` hands the host.
//!
//! Pattern: Value Object: plain results of a pure function.

use crate::codes::LayoutCode;
use crate::ids::{ElemRef, FrontendKey, LayoutId, OverlayId, Rev, Stamp};
use crate::layout::Layout;
use crate::overlay::{Cause, Conflict, ElemLock, OverlayScope, OverlayState};
use crate::shim::Coded;

/// The result of [`apply`](crate::apply).
#[derive(Clone, Debug)]
pub struct Applied {
    /// The effective layout.
    pub layout: Layout,
    /// One entry per op that was looked at.
    pub effects: Vec<Effect>,
    /// Which layer last changed an element.
    pub provenance: Vec<(ElemRef, Layer)>,
}
/// What happened to one op.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Effect {
    /// The overlay.
    pub overlay: OverlayId,
    /// The op.
    pub seq: u32,
    /// Acted, or why not.
    pub result: Result<(), Cause>,
}
/// The layer an element comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Layer {
    /// The original.
    Original,
    /// An instance overlay.
    Instance(OverlayId),
    /// A person overlay.
    Person(OverlayId),
}
/// What the host gets from `resolve`.
#[derive(Clone, Debug)]
pub struct Resolved {
    /// The layout's id.
    pub id: LayoutId,
    /// Its key.
    pub key: FrontendKey,
    /// Its counter.
    pub rev: Rev,
    /// The store counter at the time.
    pub stamp: Stamp,
    /// Original plus overlays, computed on read, never written back.
    pub layout: Layout,
    /// The overlays that were looked at.
    pub layers: Vec<LayerInfo>,
    /// Open conflicts.
    pub conflicts: Vec<Conflict>,
    /// Locks.
    pub locks: Vec<ElemLock>,
    /// Where each changed element comes from.
    pub provenance: Vec<(ElemRef, Layer)>,
    /// Digest of the effective layout: the host rebuilds only when it changed.
    pub effective_digest: [u8; 32],
    /// Blocking live checks (`schemes-missing`, `schemes-unexpected`): the host locks the part.
    pub problems: Vec<Coded<LayoutCode>>,
    /// Not blocking (`provider-unchecked`): a notice only.
    pub warnings: Vec<Coded<LayoutCode>>,
}
/// One overlay in a [`Resolved`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayerInfo {
    /// The overlay.
    pub overlay: OverlayId,
    /// Whose.
    pub scope: OverlayScope,
    /// State.
    pub state: OverlayState,
    /// Counter.
    pub rev: Rev,
}
