//! Rows of the tree of places and of the attributes back to `PageDoc.places` and `BlockDoc.set`.
//!
//! Pattern: Adapter (rows to model), recursive over the parent column; the children of one parent are read in the order of `seq`.

use fs_forms_layout::{
    AttrDoc, BindingDoc, BlockDoc, Coded, FrameDoc, Item, LayoutCode, Limits, PlaceNode,
};

use crate::from_rows::{binding_from, each, item, same};
use crate::rows::{RowReader, Rows};
use crate::tables as t;

/// What the walk over the tree collects besides the nodes: how many it read and what it found broken.
#[derive(Default)]
pub(crate) struct Walk {
    /// Node rows reached from the page.
    pub(crate) seen: usize,
    /// Broken rows (depth over the limit).
    pub(crate) problems: Vec<Coded<LayoutCode>>,
}

fn broken(detail: &str) -> Coded<LayoutCode> {
    Coded::new(LayoutCode::StoreBroken)
        .with("file", "forms.db")
        .with("cause", "rows")
        .with("detail", detail)
}

/// The attributes of the block or node `owner`, in the order of `seq`.
pub(crate) fn attrs(rows: &Rows, id: &str, kind: &str, owner: &str) -> Vec<Item<AttrDoc>> {
    let mut list: Vec<_> = each(rows, &t::ATTR)
        .filter(|r| {
            same(r, id)
                && r.text("owner_kind").as_deref() == Some(kind)
                && r.text("owner").as_deref() == Some(owner)
        })
        .collect();
    list.sort_by_key(|r| r.int("seq"));
    list.iter()
        .map(|r| {
            let a = AttrDoc {
                name: r.text("name").unwrap_or_default(),
                value: r.text("value").unwrap_or_default(),
            };
            item(r, a)
        })
        .collect()
}

/// The children of `parent` (`None` = directly in the frame of the page), depth `depth` counted from 1.
pub(crate) fn places(
    rows: &Rows,
    id: &str,
    page: &str,
    parent: Option<&str>,
    depth: u32,
    walk: &mut Walk,
) -> Vec<Item<PlaceNode>> {
    let mut list: Vec<_> = each(rows, &t::NODE)
        .filter(|r| {
            same(r, id)
                && r.text("page").as_deref() == Some(page)
                && r.text("parent").as_deref() == parent
        })
        .collect();
    list.sort_by_key(|r| r.int("seq"));
    if !list.is_empty() && depth > Limits::declared().place_depth_max {
        walk.problems.push(broken(&format!(
            "forms_node: page {page} is deeper than the limit"
        )));
        return Vec::new();
    }
    list.iter()
        .map(|r| {
            walk.seen += 1;
            let me = r.text("elem").unwrap_or_default();
            let set = attrs(rows, id, "node", &me);
            let place = r.text("place").unwrap_or_default();
            let name = r.text("name").unwrap_or_default();
            let v = if r.text("kind").as_deref() == Some("frame") {
                PlaceNode::Frame(FrameDoc {
                    place,
                    frame: name,
                    set,
                    children: places(rows, id, page, Some(&me), depth + 1, walk),
                })
            } else {
                PlaceNode::Block(block(rows, id, r, &me, place, name, set))
            };
            item(r, v)
        })
        .collect()
}

fn block(
    rows: &Rows,
    id: &str,
    r: &RowReader<'_>,
    me: &str,
    place: String,
    name: String,
    set: Vec<Item<AttrDoc>>,
) -> BlockDoc {
    let at = |x: &RowReader<'_>| same(x, id) && x.text("node").as_deref() == Some(me);
    let mut list: Vec<_> = each(rows, &t::NODE_BINDING).filter(|x| at(x)).collect();
    list.sort_by_key(|x| (x.text("slot"), x.int("nth")));
    let bindings: Vec<Item<BindingDoc>> = list
        .iter()
        .map(|b| binding_from(rows, b, &t::NODE_SCHEME, &at))
        .collect();
    BlockDoc {
        place,
        block: name,
        source: r.uint("source"),
        title_key: r.text("title_key"),
        text_key: r.text("text_key"),
        bindings,
        set,
    }
}
