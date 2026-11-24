//! The tree of places from the wire (`PlaceNode`, `Frame`, `Attr`, `Block.set`): layer 1 in. The caller has run the guard
//! (`tree_guard`) first, so the recursion here is as deep as the limit allows. A value that is not text, a node of a kind this
//! version does not know: `format-invalid{found}`, never left out.
//!
//! Pattern: Adapter (wire -> model) over a Composite, recursive like the menu.

use fs_forms_layout::{AttrDoc, BlockDoc, FrameDoc, Item, PlaceNode};

use super::doc_in::{item, read_binding};
use super::text::{R, bad, elem, opt, txt};
use crate::layouts_capnp as w;

pub(crate) fn read_attrs(
    l: capnp::Result<capnp::struct_list::Reader<'_, w::attr::Owned>>,
) -> R<Vec<Item<AttrDoc>>> {
    l.map_err(|_| bad("set"))?
        .iter()
        .map(|a| {
            let value = match a.get_value().map_err(|_| bad("attr-value"))?.which() {
                Ok(w::value::Which::Text(t)) => txt(t)?,
                _ => return Err(bad("attr-value")),
            };
            Ok(item(
                elem(a.get_elem())?,
                AttrDoc {
                    name: txt(a.get_name())?,
                    value,
                },
            ))
        })
        .collect()
}

pub(crate) fn read_block(b: w::block::Reader<'_>) -> R<BlockDoc> {
    let bindings = b
        .get_binds()
        .map_err(|_| bad("binds"))?
        .iter()
        .map(read_binding)
        .collect::<R<Vec<_>>>()?;
    Ok(BlockDoc {
        place: txt(b.get_place())?,
        block: txt(b.get_block())?,
        source: (b.get_source() != 0).then(|| b.get_source()),
        title_key: opt(b.get_title_key())?,
        text_key: opt(b.get_text_key())?,
        bindings,
        set: read_attrs(b.get_set())?,
    })
}

pub(crate) fn read_places(
    l: capnp::Result<capnp::struct_list::Reader<'_, w::place_node::Owned>>,
) -> R<Vec<Item<PlaceNode>>> {
    l.map_err(|_| bad("places"))?
        .iter()
        .map(|n| match n.which().map_err(|_| bad("place"))? {
            w::place_node::Which::Block(b) => {
                let b = b.map_err(|_| bad("block"))?;
                Ok(item(elem(b.get_elem())?, PlaceNode::Block(read_block(b)?)))
            }
            w::place_node::Which::Frame(f) => {
                let f = f.map_err(|_| bad("frame"))?;
                Ok(item(
                    elem(f.get_elem())?,
                    PlaceNode::Frame(FrameDoc {
                        place: txt(f.get_place())?,
                        frame: txt(f.get_frame())?,
                        set: read_attrs(f.get_set())?,
                        children: read_places(f.get_children())?,
                    }),
                ))
            }
        })
        .collect()
}
