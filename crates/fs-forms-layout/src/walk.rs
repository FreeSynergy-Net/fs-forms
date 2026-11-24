//! One walk over every element of a description, in document order, with the natural key parts of each.
//!
//! Pattern: Visitor: a closure sees every element; the walk knows the shape, the closure knows the purpose.

use crate::doc::{BindingDoc, FilterDoc, Item, LayoutDoc, MenuDoc};
use crate::ids::{ElemId, ElemKind};
use crate::place::{PlaceNode, segment};

/// What the visitor gets for one element.
pub(crate) struct Visit<'a> {
    pub kind: ElemKind,
    /// Natural key parts (joined by `/` for the natural id).
    pub parts: Vec<String>,
    pub elem: &'a mut Option<ElemId>,
    pub line: &'a mut u32,
}

pub(crate) type Visitor<'v> = dyn FnMut(Visit<'_>) + 'v;

macro_rules! emit {
    ($f:expr, $k:expr, $parts:expr, $item:expr) => {
        $f(Visit {
            kind: $k,
            parts: $parts,
            elem: &mut $item.elem,
            line: &mut $item.line,
        })
    };
}

/// Visits every element of the description.
pub(crate) fn walk(doc: &mut LayoutDoc, f: &mut Visitor<'_>) {
    for (i, r) in doc.requires.iter_mut().enumerate() {
        emit!(f, ElemKind::Requires, vec![(i + 1).to_string()], r);
    }
    for (i, s) in doc.sources.iter_mut().enumerate() {
        emit!(f, ElemKind::Source, vec![(i + 1).to_string()], s);
    }
    menu(&mut doc.menu, "", f);
    for p in &mut doc.pages {
        let page = p.v.page.clone();
        emit!(f, ElemKind::Page, vec![page.clone()], p);
        for b in &mut p.v.blocks {
            let place = b.v.place.clone();
            emit!(f, ElemKind::Block, vec![page.clone(), place.clone()], b);
            for a in &mut b.v.set {
                let p = vec![page.clone(), place.clone(), a.v.name.clone()];
                emit!(f, ElemKind::Attr, p, a);
            }
            for bd in &mut b.v.bindings {
                binding(bd, &page, &place, f);
            }
        }
        places(&mut p.v.places, &page, &[], f);
    }
    for (i, a) in doc.actions.iter_mut().enumerate() {
        emit!(f, ElemKind::Action, vec![(i + 1).to_string()], a);
    }
    for v in &mut doc.views {
        let name = v.v.view.clone();
        emit!(f, ElemKind::View, vec![name.clone()], v);
        for (i, o) in v.v.orders.iter_mut().enumerate() {
            emit!(
                f,
                ElemKind::ViewOrder,
                vec![name.clone(), (i + 1).to_string()],
                o
            );
        }
        if let Some(fl) = v.v.filter.as_mut() {
            filter(fl, &name, "1", f);
        }
    }
    for m in &mut doc.messages {
        let parts = vec![m.v.key.clone(), m.v.lang.clone()];
        emit!(f, ElemKind::Message, parts, m);
    }
}

fn binding(bd: &mut Item<BindingDoc>, page: &str, place: &str, f: &mut Visitor<'_>) {
    let (slot, nth) = (bd.v.slot.clone(), bd.v.nth.to_string());
    let base = vec![page.to_owned(), place.to_owned(), slot, nth];
    emit!(f, ElemKind::Binding, base.clone(), bd);
    for (i, s) in bd.v.schemes.iter_mut().flatten().enumerate() {
        let mut parts = base.clone();
        parts.push(i.to_string());
        emit!(f, ElemKind::BindingScheme, parts, s);
    }
}

fn menu(items: &mut [Item<MenuDoc>], path: &str, f: &mut Visitor<'_>) {
    for (i, m) in items.iter_mut().enumerate() {
        let here = if path.is_empty() {
            i.to_string()
        } else {
            format!("{path}.{i}")
        };
        emit!(f, ElemKind::Menu, vec![here.clone()], m);
        menu(&mut m.v.children, &here, f);
    }
}

fn filter(fl: &mut Item<FilterDoc>, view: &str, path: &str, f: &mut Visitor<'_>) {
    emit!(
        f,
        ElemKind::FilterNode,
        vec![view.to_owned(), path.to_owned()],
        fl
    );
    match &mut fl.v {
        FilterDoc::Op { children, .. } => {
            for (i, c) in children.iter_mut().enumerate() {
                filter(c, view, &format!("{path}.{i}"), f);
            }
        }
        FilterDoc::Is { values, .. } => {
            for (i, v) in values.iter_mut().enumerate() {
                emit!(
                    f,
                    ElemKind::FilterValue,
                    vec![view.to_owned(), path.to_owned(), i.to_string()],
                    v
                );
            }
        }
    }
}

/// Depth of the deepest menu entry (top level = 1).
pub(crate) fn menu_depth(items: &[Item<MenuDoc>]) -> u32 {
    items
        .iter()
        .map(|m| 1 + menu_depth(&m.v.children))
        .max()
        .unwrap_or(0)
}

/// The tree of places of a page, depth first. The natural id of a node is its path: page plus the segments of the places from
/// the root (see `segment`), so ids are unique as long as the tree is.
fn places(items: &mut [Item<PlaceNode>], page: &str, path: &[String], f: &mut Visitor<'_>) {
    let segs: Vec<String> = (0..items.len()).map(|i| segment(items, i)).collect();
    for (it, seg) in items.iter_mut().zip(segs) {
        let mut here = path.to_vec();
        here.push(seg);
        let mut parts = vec![page.to_owned()];
        parts.extend(here.iter().cloned());
        match &mut it.v {
            PlaceNode::Block(b) => {
                emit!(f, ElemKind::Block, parts.clone(), it);
                let joined = here.join("/");
                for a in &mut b.set {
                    let mut p = parts.clone();
                    p.push(a.v.name.clone());
                    emit!(f, ElemKind::Attr, p, a);
                }
                for bd in &mut b.bindings {
                    binding(bd, page, &joined, f);
                }
            }
            PlaceNode::Frame(fr) => {
                emit!(f, ElemKind::Frame, parts.clone(), it);
                for a in &mut fr.set {
                    let mut p = parts.clone();
                    p.push(a.v.name.clone());
                    emit!(f, ElemKind::Attr, p, a);
                }
                places(&mut fr.children, page, &here, f);
            }
        }
    }
}
