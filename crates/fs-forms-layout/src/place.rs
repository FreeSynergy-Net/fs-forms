//! The tree of places on a page: a place holds a block (leaf) or a frame that has places of its own.
//!
//! Pattern: Composite (see `PlaceNode`): leaf or composite; the types are Value Objects, the
//! behaviour (walk, check, draw) lives outside. [`PageDoc::leaves`] is an Iterator over the leaves in document order.
//! Deviation from the pattern book: no `List` node; several children on one place ARE the list (their order is the file order).

use crate::doc::{BlockDoc, Item, PageDoc};
use crate::ids::ElemId;

/// Pattern: Composite: one node of the place tree, a leaf (`Block`) or a composite (`Frame`) with children.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlaceNode {
    /// A block (leaf).
    Block(BlockDoc),
    /// A frame with places of its own.
    Frame(FrameDoc),
}

/// A frame placed on a place; its parameters (`axis`, `sizes`, `active` ...) are attributes in `set`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameDoc {
    /// Place in the parent, several children may share one place.
    pub place: String,
    /// Frame name in the component catalog.
    pub frame: String,
    /// Attributes, unique by name.
    pub set: Vec<Item<AttrDoc>>,
    /// Children, in order.
    pub children: Vec<Item<PlaceNode>>,
}

/// One attribute: a name and its value as text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttrDoc {
    /// Attribute name.
    pub name: String,
    /// Value.
    pub value: String,
}

impl PlaceNode {
    /// The place the node sits on.
    #[must_use]
    pub fn place(&self) -> &str {
        match self {
            Self::Block(b) => &b.place,
            Self::Frame(f) => &f.place,
        }
    }
}

/// A leaf: the block with the id and line of the item it sits in.
#[derive(Clone, Copy, Debug)]
pub struct Leaf<'a> {
    /// Element id of the item.
    pub elem: &'a Option<ElemId>,
    /// Line of the item.
    pub line: u32,
    /// The block.
    pub v: &'a BlockDoc,
}

/// A leaf to change.
#[derive(Debug)]
pub struct LeafMut<'a> {
    /// Element id of the item.
    pub elem: &'a mut Option<ElemId>,
    /// Line of the item.
    pub line: &'a mut u32,
    /// The block.
    pub v: &'a mut BlockDoc,
}

/// Path segment of the child `i` of `siblings`: its place; a second child on the same place gets `place.1`, a third `place.2` ...
/// (the first keeps the plain name, so adding a sibling never changes an id).
pub(crate) fn segment(siblings: &[Item<PlaceNode>], i: usize) -> String {
    let place = siblings[i].v.place();
    let nth = siblings[..i]
        .iter()
        .filter(|s| s.v.place() == place)
        .count();
    if nth == 0 {
        place.to_owned()
    } else {
        format!("{place}.{nth}")
    }
}

fn collect<'a>(items: &'a [Item<PlaceNode>], out: &mut Vec<Leaf<'a>>) {
    for it in items {
        match &it.v {
            PlaceNode::Block(b) => out.push(Leaf {
                elem: &it.elem,
                line: it.line,
                v: b,
            }),
            PlaceNode::Frame(f) => collect(&f.children, out),
        }
    }
}

fn collect_mut<'a>(items: &'a mut [Item<PlaceNode>], out: &mut Vec<LeafMut<'a>>) {
    for it in items {
        match &mut it.v {
            PlaceNode::Block(b) => out.push(LeafMut {
                elem: &mut it.elem,
                line: &mut it.line,
                v: b,
            }),
            PlaceNode::Frame(f) => collect_mut(&mut f.children, out),
        }
    }
}

impl PageDoc {
    /// All blocks of the page in document order: first the flat list `blocks` (format 2, readable until the exchange reads
    /// format 3), then the leaves of the tree `places`, depth first.
    pub fn leaves(&self) -> impl Iterator<Item = Leaf<'_>> {
        let mut v: Vec<Leaf<'_>> = self
            .blocks
            .iter()
            .map(|b| Leaf {
                elem: &b.elem,
                line: b.line,
                v: &b.v,
            })
            .collect();
        collect(&self.places, &mut v);
        v.into_iter()
    }

    /// The same, to change.
    pub fn leaves_mut(&mut self) -> impl Iterator<Item = LeafMut<'_>> {
        let mut v: Vec<LeafMut<'_>> = self
            .blocks
            .iter_mut()
            .map(|b| LeafMut {
                elem: &mut b.elem,
                line: &mut b.line,
                v: &mut b.v,
            })
            .collect();
        collect_mut(&mut self.places, &mut v);
        v.into_iter()
    }
}
