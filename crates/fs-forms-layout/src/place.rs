//! The tree of places on a page: a place holds a block (leaf) or a frame that has places of its own.
//!
//! Pattern: Composite (see `PlaceNode`): leaf (block, frontend) or composite (frame); the types are Value Objects, the
//! behaviour (walk, check, draw) lives outside. [`PageDoc::leaves`] is an Iterator over the leaves in document order.
//! Deviation from the pattern book: no `List` node; several children on one place ARE the list (their order is the file order).

use crate::doc::{BlockDoc, Item, PageDoc, Placement, Shown};
use crate::ids::{ElemId, FrontendKey};

/// Pattern: Composite: one node of the place tree, a leaf (`Block`) or a composite (`Frame`) with children.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlaceNode {
    /// A block (leaf).
    Block(BlockDoc),
    /// A frame with places of its own.
    Frame(FrameDoc),
    /// A frontend of another (or the same) program on this place (leaf; Stage 6).
    Frontend(EmbedDoc),
}

/// A frontend embedded on a place: which one, how it appears, when it is opened. Pure description (Value Object); opening it is
/// the job of the window owner (`fs-forms-run`), never of this crate. The `elem` of the item is the stable id the window id is
/// built from, so it is required (the natural ids of the walk are never made for this node).
///
/// Pattern: Composite leaf (see `PlaceNode`); `mode` is a Strategy word (see `Placement`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmbedDoc {
    /// Place in the parent.
    pub place: String,
    /// The embedded frontend: `AppStream` id of its program and its name.
    pub key: FrontendKey,
    /// Panel on the place or window of its own.
    pub mode: Placement,
    /// At the start of the parent or only by an action `open`.
    pub shown: Shown,
    /// Element id of a neighbour node whose selection is the `scope` of the child.
    pub scope_from: Option<ElemId>,
    /// Flags the person may use; known: `window` (switch panel and window).
    pub user: Vec<String>,
}

impl EmbedDoc {
    /// The only user flag that exists so far.
    pub const USER_FLAGS: &'static [&'static str] = &["window"];
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
            Self::Frontend(e) => &e.place,
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
            PlaceNode::Frontend(_) => {}
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
            PlaceNode::Frontend(_) => {}
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
