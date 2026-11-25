//! The tree of places to the canonical tree (format 3): see `read_place` for the shape. The flat list `blocks` of a page
//! (a reader of the stores that does not fill the tree yet) is written first, as block places, so nothing is lost.
//!
//! Pattern: Composite writer (recursion like the menu entries).

use fs_forms_layout::{
    AttrDoc, BlockDoc, Coded, EmbedDoc, Item, LayoutCode, PageDoc, PlaceNode, Words,
};

use crate::tree::Tree;
use crate::write::{Pairs, binding, elem, int, list, obj, opt};

fn set(attrs: &[Item<AttrDoc>], q: &mut Pairs) {
    if !attrs.is_empty() {
        let mut t: Vec<(String, Tree)> = attrs
            .iter()
            .map(|a| (a.v.name.clone(), Tree::s(&a.v.value)))
            .collect();
        t.sort_by(|a, b| a.0.cmp(&b.0)); // byte order of the name: same as the reader and as JSON
        q.push(("set", Tree::Obj(t, 0)));
    }
}

fn block(e: (&'static str, Tree), b: &BlockDoc) -> Tree {
    let mut q = vec![e, ("at", Tree::s(&b.place)), ("block", Tree::s(&b.block))];
    if let Some(s) = b.source {
        q.push(("source", int(s)));
    }
    opt(&mut q, "title", b.title_key.as_ref());
    opt(&mut q, "text", b.text_key.as_ref());
    set(&b.set, &mut q);
    // order of bindings: (slot, nth), kept by the invariant of `Layout`
    q.push(("bind", list(&b.bindings, binding)));
    obj(q)
}

/// A frontend node: the key as a table, `mode` and `shown` always written (a file says what it does).
fn embed(e: (&'static str, Tree), d: &EmbedDoc) -> Tree {
    let key = Tree::obj(vec![
        ("extends", Tree::s(&d.key.extends)),
        ("frontend", Tree::s(&d.key.frontend)),
    ]);
    let mut q = vec![
        e,
        ("at", Tree::s(&d.place)),
        ("frontend", key),
        ("mode", Tree::s(d.mode.word())),
        ("shown", Tree::s(d.shown.word())),
    ];
    if let Some(s) = &d.scope_from {
        q.push(("scope-from", Tree::s(s.as_str())));
    }
    if !d.user.is_empty() {
        q.push((
            "user",
            Tree::Arr(d.user.iter().map(|u| Tree::s(u)).collect()),
        ));
    }
    obj(q)
}

fn node(i: &Item<PlaceNode>) -> Result<Tree, Coded<LayoutCode>> {
    match &i.v {
        PlaceNode::Block(b) => Ok(block(elem(i), b)),
        PlaceNode::Frontend(e) => Ok(embed(elem(i), e)),
        PlaceNode::Frame(f) => {
            let mut q = vec![
                elem(i),
                ("at", Tree::s(&f.place)),
                ("frame", Tree::s(&f.frame)),
            ];
            set(&f.set, &mut q);
            let kids = f.children.iter().map(node).collect::<Result<_, _>>()?;
            q.push(("place", Tree::Arr(kids)));
            Ok(obj(q))
        }
        _ => Err(Coded::new(LayoutCode::ExportFailed)
            .with("file", "")
            .with("kind", "place-kind-unknown")),
    }
}

/// The `place` list of a page.
pub(crate) fn places(p: &PageDoc) -> Result<Tree, Coded<LayoutCode>> {
    let flat = p.blocks.iter().map(|b| block(elem(b), &b.v));
    let tree = p.places.iter().map(node);
    let all = flat.map(Ok).chain(tree).collect::<Result<_, _>>()?;
    Ok(Tree::Arr(all))
}
