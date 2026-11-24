//! The tree of places in the plan (A5 F6): a drawn or locked block, or a frame with children and attributes.
//!
//! Pattern: Composite (value objects; the host walks it). `leaves_of` is the flat reader (Iterator-like) over the block placements.

use crate::codes::PartLock;
use crate::model::ElemId;
use crate::plan::{PagePlan, PlacePlan};

/// One attribute of a `set` table, as the layout wrote it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttrPlan {
    /// Attribute name.
    pub name: String,
    /// Value as written.
    pub value: String,
}

/// A frame with children. The host draws only what it can build; a deeper frame is said (`host-notice-frame-not-built`), never skipped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FramePlan {
    /// Placement name.
    pub place: String,
    /// Element id.
    pub elem: ElemId,
    /// Position among the siblings.
    pub seq: u32,
    /// Frame block name.
    pub frame: String,
    /// Attributes (`set`).
    pub set: Vec<AttrPlan>,
    /// Locks of the frame itself.
    pub locks: Vec<PartLock>,
    /// Children in `seq` order.
    pub children: Vec<NodePlan>,
}

/// A node of the tree of places.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NodePlan {
    /// A block placement.
    Block(PlacePlan),
    /// A frame with children.
    Frame(FramePlan),
}

/// The block placements of a forest, depth first.
#[must_use]
pub fn leaves_of(nodes: &[NodePlan]) -> Vec<&PlacePlan> {
    let mut out = Vec::new();
    for n in nodes {
        match n {
            NodePlan::Block(p) => out.push(p),
            NodePlan::Frame(f) => out.extend(leaves_of(&f.children)),
        }
    }
    out
}

/// The same, mutable (the conformance mutants).
#[must_use]
pub fn leaves_of_mut(nodes: &mut [NodePlan]) -> Vec<&mut PlacePlan> {
    let mut out = Vec::new();
    for n in nodes {
        match n {
            NodePlan::Block(p) => out.push(p),
            NodePlan::Frame(f) => out.extend(leaves_of_mut(&mut f.children)),
        }
    }
    out
}

impl PagePlan {
    /// The block placements of the tree, depth first (the flat reader).
    #[must_use]
    pub fn leaves(&self) -> Vec<&PlacePlan> {
        leaves_of(&self.places)
    }
}

impl PagePlan {
    /// The locks of the frames of the tree, depth first.
    #[must_use]
    pub fn frame_locks(&self) -> Vec<&PartLock> {
        fn go<'a>(nodes: &'a [NodePlan], out: &mut Vec<&'a PartLock>) {
            for n in nodes {
                if let NodePlan::Frame(f) = n {
                    out.extend(&f.locks);
                    go(&f.children, out);
                }
            }
        }
        let mut out = Vec::new();
        go(&self.places, &mut out);
        out
    }
}

impl NodePlan {
    /// The block placement, if this node is one.
    #[must_use]
    pub fn block(&self) -> Option<&PlacePlan> {
        match self {
            Self::Block(p) => Some(p),
            Self::Frame(_) => None,
        }
    }
}
