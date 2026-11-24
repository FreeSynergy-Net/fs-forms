//! Rows of the tree of places (`forms_node`, `forms_node_binding`, `forms_node_scheme`) and of the attributes `set`
//! (`forms_attr`) of flat blocks and tree nodes.
//!
//! Pattern: Adapter (model to rows), recursive over the Composite `PlaceNode`; pre-order, so a parent row always comes before its children.

use fs_forms_layout::{AttrDoc, Coded, Item, LayoutCode, PlaceNode};

use crate::rows::{Cell, RowBuilder, Rows, int, opt_int, opt_text, text};
use crate::tables as t;
use crate::to_rows::elem;
use crate::to_rows_parts::binding_rows;

fn seq(n: usize) -> Cell {
    int(u32::try_from(n).unwrap_or(u32::MAX))
}

/// The attributes of one owner (`block` of the flat list or `node` of the tree), in the order of the description.
pub(crate) fn attr_rows(
    set: &[Item<AttrDoc>],
    owner_kind: &str,
    owner: &Cell,
    id: &Cell,
    rows: &mut Rows,
) {
    for (n, a) in set.iter().enumerate() {
        RowBuilder::new(&t::ATTR)
            .set("layout", id.clone())
            .set("elem", elem(a))
            .set("owner_kind", text(owner_kind))
            .set("owner", owner.clone())
            .set("seq", seq(n))
            .set("name", text(&a.v.name))
            .set("value", text(&a.v.value))
            .push_into(rows);
    }
}

/// The children of one parent (`None` = the frame of the page), then their children, depth first.
pub(crate) fn node_rows(
    items: &[Item<PlaceNode>],
    parent: Option<&Cell>,
    id: &Cell,
    page: &Cell,
    rows: &mut Rows,
) -> Result<(), Coded<LayoutCode>> {
    for (n, it) in items.iter().enumerate() {
        let me = elem(it);
        let (kind, place, name, set, block) = match &it.v {
            PlaceNode::Block(b) => ("block", &b.place, &b.block, &b.set, Some(b)),
            PlaceNode::Frame(f) => ("frame", &f.place, &f.frame, &f.set, None),
            // a variant of a later cut that the tables do not hold: refused, never dropped
            _ => {
                return Err(Coded::new(LayoutCode::ExportFailed)
                    .with("file", "")
                    .with("kind", "place-node-not-supported-yet"));
            }
        };
        let (source, title, body) = block.map_or((None, None, None), |b| {
            (b.source, b.title_key.as_ref(), b.text_key.as_ref())
        });
        RowBuilder::new(&t::NODE)
            .set("layout", id.clone())
            .set("elem", me.clone())
            .set("page", page.clone())
            .set("parent", parent.cloned().unwrap_or(Cell::Null))
            .set("place", text(place))
            .set("seq", seq(n))
            .set("kind", text(kind))
            .set("name", text(name))
            .set("source", opt_int(source))
            .set("title_key", opt_text(title))
            .set("text_key", opt_text(body))
            .push_into(rows);
        attr_rows(set, "node", &me, id, rows);
        match &it.v {
            PlaceNode::Frame(f) => node_rows(&f.children, Some(&me), id, page, rows)?,
            _ => {
                for bd in block.iter().flat_map(|b| &b.bindings) {
                    let key = |r: RowBuilder<'static>| {
                        r.set("layout", id.clone())
                            .set("node", me.clone())
                            .set("slot", text(&bd.v.slot))
                            .set("nth", int(bd.v.nth))
                    };
                    binding_rows(bd, (&t::NODE_BINDING, &t::NODE_SCHEME), &key, rows);
                }
            }
        }
    }
    Ok(())
}
