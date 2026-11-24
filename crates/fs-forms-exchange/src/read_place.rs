//! The tree of places of a page (format 3): `[[page.place]]` holds exactly one of `block` or `frame`; a frame has children
//! `[[page.place.place]]`; attributes are ONE table `set = { name = "value" }`. The name of the place is the key `at`
//! (the key `place` is the list of children: a table cannot hold a string and a list under one key).
//!
//! Pattern: Composite reader (recursion like the menu entries) with a collecting error list.

use fs_forms_layout::{AttrDoc, BlockDoc, FrameDoc, Item, PlaceNode};

use crate::fields::{Ctx, Fields};
use crate::read::{binding, items};
use crate::tree::Tree;

/// The attributes of a node: the table `set`, every value a string; names are unique by the table itself, in byte order of the name.
fn attrs(f: &mut Fields<'_>, cx: &mut Ctx<'_>) -> Vec<Item<AttrDoc>> {
    let line = f.line();
    let Some((pairs, _)) = f.table(cx, "set") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (name, v) in pairs {
        if let Tree::Str(value) = v {
            out.push(Item {
                elem: None,
                line,
                v: AttrDoc {
                    name: name.clone(),
                    value: value.clone(),
                },
            });
        } else {
            cx.bad_at(line, "type", name);
        }
    }
    out.sort_by(|a, b| a.v.name.cmp(&b.v.name)); // the order of attributes has no meaning: byte order of the name (JSON keys are sorted too)
    out
}

fn node(f: &mut Fields<'_>, cx: &mut Ctx<'_>) -> PlaceNode {
    let place = f.req_str(cx, "at");
    let (block, frame) = (f.str(cx, "block"), f.str(cx, "frame"));
    let set = attrs(f, cx);
    match (block, frame) {
        (None, Some(frame)) => PlaceNode::Frame(FrameDoc {
            place,
            frame,
            set,
            children: places(f.list(cx, "place"), cx),
        }),
        (block, other) => {
            if block.is_none() {
                cx.bad_at(f.line(), "key-missing", ""); // neither block nor frame
            } else if other.is_some() {
                cx.bad_at(f.line(), "place-block-and-frame", "frame");
            }
            PlaceNode::Block(BlockDoc {
                place,
                block: block.unwrap_or_default(),
                source: f.uint(cx, "source"),
                title_key: f.str(cx, "title"),
                text_key: f.str(cx, "text"),
                set,
                bindings: items(f.list(cx, "bind"), "bind", cx, binding),
            })
        }
    }
}

/// The places of a page or of a frame, in file order.
pub(crate) fn places(list: &[Tree], cx: &mut Ctx<'_>) -> Vec<Item<PlaceNode>> {
    items(list, "place", cx, node)
}
