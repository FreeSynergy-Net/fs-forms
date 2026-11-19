//! The plan becomes a tree of nodes: a block that is drawn, a part that is locked with its codes, an empty page. Pure: no window,
//! no clock, no I/O, so every rule of the host is testable without Blinc.
//!
//! Pattern: Interpreter (plan in, tree out) with the catalog asked for every block (Abstract Factory seen from the host: it never
//! picks, it only asks whether the chosen block is built). Nothing is dropped: every lock of the plan is in the tree.

use crate::catalog::Catalog;
use crate::view::{Coded, PageView, View};

/// How the window stands (the host's own states of the plan, `Teilweise` and so on).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostState {
    /// Everything is drawn.
    Ready,
    /// At least one part is locked or noted; the rest runs.
    Partial,
    /// Nothing can be drawn; the window shows the notices only.
    Refused,
}

/// One part of a page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    /// A block that is built, in its place.
    Block {
        /// The placement.
        place: String,
        /// The block the catalog chose.
        block: String,
        /// Locks of single slots (the block shows them itself).
        slot_locks: Vec<Coded>,
        /// The words of the placement's title, if there are any.
        title: Option<String>,
    },
    /// A part that is not drawn: a `partial-notice` with every code. `place` is `None` for the page itself.
    Locked {
        /// The placement, if the lock is of one.
        place: Option<String>,
        /// Every code.
        codes: Vec<Coded>,
    },
    /// A page without placements.
    Empty {
        /// The page.
        page: String,
    },
}

/// One screen: a page and its parts, in Tab order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScreenTree {
    /// The page.
    pub page: String,
    /// The frame block.
    pub frame: String,
    /// The parts in `seq` order.
    pub nodes: Vec<Node>,
}

/// The whole window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Composition {
    /// The state.
    pub state: HostState,
    /// The screens in `seq` order.
    pub screens: Vec<ScreenTree>,
    /// Notices of the window (from the plan, and the host's own).
    pub notices: Vec<Coded>,
}

/// Why a block is not drawn: the catalog does not know it, or this build has no drawing for it yet.
fn not_built(block: &str, catalog: &dyn Catalog) -> Coded {
    let code = if catalog.known(block) {
        "host-error-block-not-built"
    } else {
        "host-error-block-unknown"
    };
    Coded::new(code).with("block", block)
}

fn screen(page: &PageView, catalog: &dyn Catalog) -> ScreenTree {
    let mut nodes = Vec::new();
    if !page.locks.is_empty() {
        nodes.push(Node::Locked {
            place: None,
            codes: page.locks.clone(),
        });
    }
    if !catalog.built(&page.frame) {
        nodes.push(Node::Locked {
            place: None,
            codes: vec![not_built(&page.frame, catalog)],
        });
    }
    let mut places: Vec<_> = page.places.iter().collect();
    places.sort_by_key(|p| p.seq); // stable: equal positions keep the order of the plan; Tab order = `seq`
    for p in places {
        if !p.locks.is_empty() {
            nodes.push(Node::Locked {
                place: Some(p.place.clone()),
                codes: p.locks.clone(),
            });
        } else if catalog.draws(&p.block) {
            nodes.push(Node::Block {
                place: p.place.clone(),
                block: p.block.clone(),
                slot_locks: p.slot_locks.clone(),
                title: p.title.clone(),
            });
        } else {
            nodes.push(Node::Locked {
                place: Some(p.place.clone()),
                codes: vec![not_built(&p.block, catalog)],
            });
        }
    }
    if nodes.is_empty() {
        nodes.push(Node::Empty {
            page: page.page.clone(),
        });
    }
    ScreenTree {
        page: page.page.clone(),
        frame: page.frame.clone(),
        nodes,
    }
}

/// Put the plan together. Never fails, never panics; a part that cannot be drawn is a `Locked` node with the reason.
#[must_use]
pub fn compose(view: &View, catalog: &dyn Catalog) -> Composition {
    let mut pages: Vec<_> = view.pages.iter().collect();
    pages.sort_by_key(|p| p.seq);
    let screens: Vec<ScreenTree> = pages.into_iter().map(|p| screen(p, catalog)).collect();
    let mut notices = view.notices.clone();
    if screens.is_empty() {
        notices.push(Coded::new("host-error-no-pages"));
    }
    let drawn = screens
        .iter()
        .flat_map(|s| &s.nodes)
        .any(|n| matches!(n, Node::Block { .. }));
    let problem = !notices.is_empty()
        || screens.iter().flat_map(|s| &s.nodes).any(|n| match n {
            Node::Block { slot_locks, .. } => !slot_locks.is_empty(),
            _ => true,
        });
    let state = match (drawn, problem) {
        (true, false) => HostState::Ready,
        (true, true) => HostState::Partial,
        (false, _) => HostState::Refused,
    };
    Composition {
        state,
        screens,
        notices,
    }
}
