//! The tree of places to the wire: layer 1 out. Written as the model has it, depth first; a node of a kind this version does not
//! know is `export-failed{what}`, never left out (the same rule as the tables).
//!
//! Pattern: Adapter (model -> wire) over a Composite, recursive like the menu.

use fs_forms_layout::{AttrDoc, BlockDoc, Coded, Item, LayoutCode, PlaceNode};

use super::doc_out::{elem_text, write_binding};
use super::text::{R, n};
use crate::layouts_capnp as w;

pub(crate) fn write_attrs(
    mut l: capnp::struct_list::Builder<'_, w::attr::Owned>,
    set: &[Item<AttrDoc>],
) {
    for (i, a) in (0..).zip(set) {
        let mut e = l.reborrow().get(i);
        e.set_elem(elem_text(a));
        e.set_name(&a.v.name);
        e.init_value().set_text(&a.v.value);
    }
}

pub(crate) fn write_block(mut x: w::block::Builder<'_>, elem: &str, b: &BlockDoc) {
    x.set_elem(elem);
    x.set_place(&b.place);
    x.set_block(&b.block);
    x.set_source(b.source.unwrap_or(0));
    x.set_title_key(b.title_key.as_deref().unwrap_or(""));
    x.set_text_key(b.text_key.as_deref().unwrap_or(""));
    let mut bl = x.reborrow().init_binds(n(&b.bindings));
    for (j, bi) in (0..).zip(&b.bindings) {
        write_binding(bl.reborrow().get(j), bi);
    }
    write_attrs(x.init_set(n(&b.set)), &b.set);
}

pub(crate) fn write_places(
    mut l: capnp::struct_list::Builder<'_, w::place_node::Owned>,
    nodes: &[Item<PlaceNode>],
) -> R<()> {
    for (i, node) in (0..).zip(nodes) {
        let e = l.reborrow().get(i);
        match &node.v {
            PlaceNode::Block(b) => write_block(e.init_block(), elem_text(node), b),
            PlaceNode::Frame(f) => {
                let mut x = e.init_frame();
                x.set_elem(elem_text(node));
                x.set_place(&f.place);
                x.set_frame(&f.frame);
                write_attrs(x.reborrow().init_set(n(&f.set)), &f.set);
                write_places(x.init_children(n(&f.children)), &f.children)?;
            }
            _ => return Err(Coded::new(LayoutCode::ExportFailed).with("what", "place-node")),
        }
    }
    Ok(())
}
