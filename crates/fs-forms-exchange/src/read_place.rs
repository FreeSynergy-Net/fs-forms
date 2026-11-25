//! The tree of places of a page (format 3): `[[page.place]]` holds exactly one of `block` or `frame`; a frame has children
//! `[[page.place.place]]`; attributes are ONE table `set = { name = "value" }`. The name of the place is the key `at`
//! (the key `place` is the list of children: a table cannot hold a string and a list under one key).
//!
//! Pattern: Composite reader (recursion like the menu entries) with a collecting error list.

use fs_forms_layout::{
    AttrDoc, BlockDoc, ElemId, EmbedDoc, FrameDoc, FrontendKey, Item, PlaceNode, Placement, Shown,
    Words,
};

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
    if f.has_list("frontend") {
        return PlaceNode::Frontend(embed(f, cx, place));
    }
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

/// The places of a page or of a frame, in file order. A frontend node needs its `elem` (the window id is built from it).
pub(crate) fn places(list: &[Tree], cx: &mut Ctx<'_>) -> Vec<Item<PlaceNode>> {
    let out = items(list, "place", cx, node);
    for i in out
        .iter()
        .filter(|i| i.elem.is_none() && matches!(i.v, PlaceNode::Frontend(_)))
    {
        cx.bad_at(i.line, "key-missing", "elem");
    }
    out
}

/// A frontend node: `frontend = { extends = "...", frontend = "..." }`, `mode` (panel, window), `shown` (start, on-open),
/// `scope-from` (id of a neighbour), `user` (list of flags). `block`, `frame` and `set` do not belong here: `block` and `frame`
/// are reported at once, `set` as an unknown key by `done`.
fn embed(f: &mut Fields<'_>, cx: &mut Ctx<'_>, place: String) -> EmbedDoc {
    let line = f.line();
    let key = match f.table(cx, "frontend") {
        Some((pairs, tline)) => {
            let find = |k: &str| {
                pairs.iter().find_map(|(n, v)| match v {
                    Tree::Str(s) if n == k => Some(s.clone()),
                    _ => None,
                })
            };
            for (n, _) in pairs
                .iter()
                .filter(|(n, _)| n != "extends" && n != "frontend")
            {
                cx.bad_at(tline, "key-unknown", n);
            }
            let (extends, name) = (find("extends"), find("frontend"));
            if extends.is_none() || name.is_none() {
                cx.bad_at(tline, "key-missing", "frontend");
            }
            FrontendKey {
                extends: extends.unwrap_or_default(),
                frontend: name.unwrap_or_default(),
            }
        }
        None => FrontendKey {
            extends: String::new(),
            frontend: String::new(),
        },
    };
    EmbedDoc {
        place,
        key,
        mode: word_or::<Placement>(f, cx, "mode", "mode-unknown").unwrap_or_default(),
        shown: word_or::<Shown>(f, cx, "shown", "shown-unknown").unwrap_or_default(),
        scope_from: f
            .str(cx, "scope-from")
            .and_then(|s| match ElemId::parse(&s) {
                Ok(e) => Some(e),
                Err(c) => {
                    cx.errs.push(c.with("line", line));
                    None
                }
            }),
        user: user_flags(f, cx),
    }
}

/// A closed word of the table; an unknown one is `import-invalid` with the `cause` (never a silent default).
fn word_or<W: Words>(f: &mut Fields<'_>, cx: &mut Ctx<'_>, key: &str, cause: &str) -> Option<W> {
    let line = f.line();
    let w = f.str(cx, key)?;
    let got = W::from_word(&w);
    if got.is_none() {
        cx.bad_at(line, cause, &w);
    }
    got
}

fn user_flags(f: &mut Fields<'_>, cx: &mut Ctx<'_>) -> Vec<String> {
    let line = f.line();
    f.list(cx, "user")
        .iter()
        .filter_map(|t| {
            if let Tree::Str(s) = t {
                Some(s.clone())
            } else {
                cx.bad_at(line, "type", "user");
                None
            }
        })
        .collect()
}
