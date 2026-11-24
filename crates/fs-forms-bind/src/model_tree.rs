//! The tree of places of one page as `Bind::build` reads it (A5 F6): a block or a frame with children and attributes.
//!
//! Pattern: Composite (value objects; the behaviour lives in `build` and in the host). Mirror of `fs_forms_layout::PlaceNode`.

use crate::model::{ElemId, PlaceIn};

/// One attribute of a `set` table (name and text value; the catalog says which names a block takes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttrIn {
    /// Attribute name.
    pub name: String,
    /// Value as written.
    pub value: String,
}

/// A frame placed on a page or in another frame: it holds more places.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameIn {
    /// Element id.
    pub elem: ElemId,
    /// Placement name (the place of the parent this frame stands in).
    pub place: String,
    /// Position among the siblings.
    pub seq: u32,
    /// Frame block name.
    pub frame: String,
    /// Attributes (`set`).
    pub set: Vec<AttrIn>,
    /// Children in file order.
    pub children: Vec<NodeIn>,
}

/// A node of the tree of places: a block (leaf) or a frame with children.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NodeIn {
    /// A block placement.
    Block(PlaceIn),
    /// A frame with children.
    Frame(FrameIn),
}

impl From<PlaceIn> for NodeIn {
    fn from(p: PlaceIn) -> Self {
        Self::Block(p)
    }
}

impl NodeIn {
    /// Element id.
    #[must_use]
    pub fn elem(&self) -> &ElemId {
        match self {
            Self::Block(p) => &p.elem,
            Self::Frame(f) => &f.elem,
        }
    }

    /// Position among the siblings.
    #[must_use]
    pub fn seq(&self) -> u32 {
        match self {
            Self::Block(p) => p.seq,
            Self::Frame(f) => f.seq,
        }
    }
}

/// The block placements of a forest, depth first, in file order (the flat reader of the tree).
#[must_use]
pub fn leaves_in(nodes: &[NodeIn]) -> Vec<&PlaceIn> {
    let mut out = Vec::new();
    for n in nodes {
        match n {
            NodeIn::Block(p) => out.push(p),
            NodeIn::Frame(f) => out.extend(leaves_in(&f.children)),
        }
    }
    out
}

impl NodeIn {
    /// The block placement, if this node is one.
    #[must_use]
    pub fn block(&self) -> Option<&PlaceIn> {
        match self {
            Self::Block(p) => Some(p),
            Self::Frame(_) => None,
        }
    }

    /// The block placement, mutable.
    pub fn block_mut(&mut self) -> Option<&mut PlaceIn> {
        match self {
            Self::Block(p) => Some(p),
            Self::Frame(_) => None,
        }
    }
}
