//! The guard of the tree of places on the wire: depth and number of nodes are measured BEFORE anything is unpacked. The socket
//! takes a message of up to 64 MiB (`fs_rpc::MAX_MESSAGE_BYTES`); a tree of frames inside frames (or one list of a million empty
//! nodes) must not become a million `Item`s, nor an endless recursion. The walk here builds nothing, only counts, and stops at the first
//! count over a limit; a list is counted by its length BEFORE its entries are visited, so the walk never goes deeper than
//! `depth_max + 1` and never takes more than `nodes_max + (one list)` steps.
//!
//! Pattern: Guard Clause over a Visitor (a read-only walk of the Composite on the wire); the limits are a Value Object, passed in.
//! Deviation: the walk is its own pass over the reader (the data is read twice), the price of "refuse before unpacking".

use fs_forms_layout::{Coded, LayoutCode, LimitSource, Limits};

use super::text::R;
use crate::layouts_capnp as w;

/// What one description may hold in its trees of places: SETTINGS (rule 57), passed in with the source of each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TreeLimits {
    /// Deepest nesting of places (a node on the page root = 1); the same meaning as `Limits::place_depth_max`.
    pub depth_max: u32,
    /// All nodes and attributes of all pages of one description, flat blocks included.
    pub nodes_max: u32,
    /// Where the values came from.
    pub by: LimitSource,
}

impl TreeLimits {
    /// The declared defaults: depth = `Limits::declared().place_depth_max`, nodes = `Limits::declared().elements_max`.
    #[must_use]
    pub fn declared() -> Self {
        let l = Limits::declared();
        Self {
            depth_max: l.place_depth_max,
            nodes_max: l.elements_max,
            by: LimitSource::Default,
        }
    }

    fn refuse(&self, what: &str, max: u32) -> Coded<LayoutCode> {
        Coded::new(LayoutCode::TooMany)
            .with("what", what)
            .with("max", max)
            .with("by", self.by.as_str())
    }
}

/// The nodes still allowed.
struct Budget<'a> {
    left: u32,
    lim: &'a TreeLimits,
}

impl Budget<'_> {
    /// Takes `len` entries of a list at once; over the limit = refused with nothing visited.
    fn take(&mut self, len: u32) -> R<()> {
        self.left = self
            .left
            .checked_sub(len)
            .ok_or_else(|| self.lim.refuse("place-nodes", self.lim.nodes_max))?;
        Ok(())
    }
}

fn attrs(
    l: capnp::Result<capnp::struct_list::Reader<'_, w::attr::Owned>>,
    b: &mut Budget<'_>,
) -> R<()> {
    // A list that cannot be read is left to the reader (`format-invalid`); the guard only measures.
    l.map_or(Ok(()), |l| b.take(l.len()))
}

fn level(
    nodes: capnp::struct_list::Reader<'_, w::place_node::Owned>,
    depth: u32,
    b: &mut Budget<'_>,
) -> R<()> {
    if depth > b.lim.depth_max {
        return Err(b.lim.refuse("place-depth", b.lim.depth_max));
    }
    b.take(nodes.len())?;
    for n in nodes {
        match n.which() {
            Ok(w::place_node::Which::Block(Ok(x))) => attrs(x.get_set(), b)?,
            Ok(w::place_node::Which::Frame(Ok(f))) => {
                attrs(f.get_set(), b)?;
                if let Ok(kids) = f.get_children() {
                    level(kids, depth + 1, b)?;
                }
            }
            _ => {} // not readable: the reader reports it
        }
    }
    Ok(())
}

/// Measures all pages of a description; `Err(too-many{what, max, by})` at the first excess, nothing is built.
pub(crate) fn guard(
    pages: capnp::struct_list::Reader<'_, w::page::Owned>,
    lim: &TreeLimits,
) -> R<()> {
    let mut b = Budget {
        left: lim.nodes_max,
        lim,
    };
    b.take(pages.len())?;
    for p in pages {
        if let Ok(flat) = p.get_blocks() {
            b.take(flat.len())?;
            for x in flat {
                attrs(x.get_set(), &mut b)?;
            }
        }
        if let Ok(places) = p.get_places() {
            level(places, 1, &mut b)?;
        }
    }
    Ok(())
}
