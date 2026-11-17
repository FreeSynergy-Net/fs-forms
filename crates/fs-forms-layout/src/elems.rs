//! Finding, reading, setting and removing one element by kind and id. First cut: fields of layout, menu, page, block,
//! binding, scheme and message; other kinds can be found and removed but their fields are not settable yet.
//!
//! Pattern: Visitor-like lookup over the tree; the borrow of the found node is handed to the caller.

use crate::doc::{BindFrom, BindingDoc, BlockDoc, Item, LayoutDoc, MenuDoc, MessageDoc, PageDoc};
use crate::ids::{ElemId, ElemKind, ElemRef};
use crate::overlay::{Field, Value};

/// A found element.
pub(crate) enum Node<'a> {
    Layout(&'a mut LayoutDoc),
    Menu(&'a mut MenuDoc),
    Page(&'a mut PageDoc),
    Block(&'a mut BlockDoc),
    Binding(&'a mut BindingDoc),
    Scheme(&'a mut String),
    Message(&'a mut MessageDoc),
    /// Exists, but its fields are not settable yet.
    Other,
}

fn is(e: &Option<ElemId>, id: &ElemId) -> bool {
    e.as_ref() == Some(id)
}
fn has<T>(items: &[Item<T>], id: &ElemId) -> bool {
    items.iter().any(|i| is(&i.elem, id))
}
fn find_menu<'a>(items: &'a mut [Item<MenuDoc>], id: &ElemId) -> Option<&'a mut MenuDoc> {
    for m in items {
        if is(&m.elem, id) {
            return Some(&mut m.v);
        }
        if let Some(x) = find_menu(&mut m.v.children, id) {
            return Some(x);
        }
    }
    None
}

/// Finds an element.
pub(crate) fn find<'a>(doc: &'a mut LayoutDoc, r: &ElemRef) -> Option<Node<'a>> {
    let id = &r.elem;
    match r.kind {
        ElemKind::Layout => Some(Node::Layout(doc)),
        ElemKind::Menu => find_menu(&mut doc.menu, id).map(Node::Menu),
        ElemKind::Page => doc
            .pages
            .iter_mut()
            .find(|p| is(&p.elem, id))
            .map(|p| Node::Page(&mut p.v)),
        ElemKind::Block => doc
            .pages
            .iter_mut()
            .flat_map(|p| p.v.blocks.iter_mut())
            .find(|b| is(&b.elem, id))
            .map(|b| Node::Block(&mut b.v)),
        ElemKind::Binding => bindings(doc)
            .find(|b| is(&b.elem, id))
            .map(|b| Node::Binding(&mut b.v)),
        ElemKind::BindingScheme => bindings(doc)
            .flat_map(|b| b.v.schemes.iter_mut().flatten())
            .find(|s| is(&s.elem, id))
            .map(|s| Node::Scheme(&mut s.v)),
        ElemKind::Message => doc
            .messages
            .iter_mut()
            .find(|m| is(&m.elem, id))
            .map(|m| Node::Message(&mut m.v)),
        ElemKind::Requires => has(&doc.requires, id).then_some(Node::Other),
        ElemKind::Source => has(&doc.sources, id).then_some(Node::Other),
        ElemKind::Action => has(&doc.actions, id).then_some(Node::Other),
        ElemKind::View | ElemKind::ViewOrder | ElemKind::FilterNode | ElemKind::FilterValue => {
            has(&doc.views, id).then_some(Node::Other)
        }
    }
}

fn bindings(doc: &mut LayoutDoc) -> impl Iterator<Item = &mut Item<BindingDoc>> {
    doc.pages
        .iter_mut()
        .flat_map(|p| p.v.blocks.iter_mut())
        .flat_map(|b| b.v.bindings.iter_mut())
}

fn opt(v: Option<&String>) -> Value {
    v.map_or(Value::None, |s| Value::Text(s.clone()))
}
fn text(v: &Value) -> Option<Option<String>> {
    match v {
        Value::None => Some(None),
        Value::Text(s) => Some(Some(s.clone())),
        _ => None,
    }
}

/// Reads a field; `None` when the node does not support it yet.
pub(crate) fn get(n: &Node<'_>, f: Field) -> Option<Value> {
    Some(match (n, f) {
        (Node::Layout(d), Field::TitleKey) => Value::Text(d.title_key.clone()),
        (Node::Layout(d), Field::Style) => opt(d.style.as_ref()),
        (Node::Menu(m), Field::LabelKey) => opt(m.label_key.as_ref()),
        (Node::Menu(m), Field::Shortcut) => opt(m.shortcut.as_ref()),
        (Node::Page(p), Field::Frame) => Value::Text(p.frame.clone()),
        (Node::Block(b), Field::TitleKey) => opt(b.title_key.as_ref()),
        (Node::Block(b), Field::TextKey) => opt(b.text_key.as_ref()),
        (Node::Block(b), Field::Block) => Value::Text(b.block.clone()),
        (Node::Binding(b), Field::Path) => match &b.from {
            BindFrom::Field { path, .. } => Value::Text(path.clone()),
            _ => Value::None,
        },
        (Node::Scheme(s), Field::Scheme) => Value::Text((*s).clone()),
        (Node::Message(m), Field::Message) => Value::Text(m.message.clone()),
        _ => return None,
    })
}

/// Sets a field; `false` when the value has the wrong type for it.
pub(crate) fn set(n: &mut Node<'_>, f: Field, v: &Value) -> bool {
    let Some(t) = text(v) else { return false };
    match (n, f, t) {
        (Node::Layout(d), Field::TitleKey, Some(s)) => d.title_key = s,
        (Node::Layout(d), Field::Style, s) => d.style = s,
        (Node::Menu(m), Field::LabelKey, s) => m.label_key = s,
        (Node::Menu(m), Field::Shortcut, s) => m.shortcut = s,
        (Node::Page(p), Field::Frame, Some(s)) => p.frame = s,
        (Node::Block(b), Field::TitleKey, s) => b.title_key = s,
        (Node::Block(b), Field::TextKey, s) => b.text_key = s,
        (Node::Block(b), Field::Block, Some(s)) => b.block = s,
        (Node::Binding(b), Field::Path, Some(s)) => match &mut b.from {
            BindFrom::Field { path, .. } => *path = s,
            _ => return false,
        },
        (Node::Scheme(x), Field::Scheme, Some(s)) => **x = s,
        (Node::Message(m), Field::Message, Some(s)) => m.message = s,
        _ => return false,
    }
    true
}

fn drop_menu(items: &mut Vec<Item<MenuDoc>>, id: &ElemId) {
    items.retain(|m| !is(&m.elem, id));
    items
        .iter_mut()
        .for_each(|m| drop_menu(&mut m.v.children, id));
}

fn count(doc: &LayoutDoc) -> usize {
    let mut d = doc.clone();
    let mut n = 0;
    crate::walk::walk(&mut d, &mut |_| n += 1);
    n
}

/// Removes an element (with everything below it); `false` when it was not there.
pub(crate) fn remove(doc: &mut LayoutDoc, r: &ElemRef) -> bool {
    let (id, before) = (&r.elem, count(doc));
    match r.kind {
        ElemKind::Layout => return false,
        ElemKind::Menu => drop_menu(&mut doc.menu, id),
        ElemKind::Page => doc.pages.retain(|p| !is(&p.elem, id)),
        ElemKind::Block => doc
            .pages
            .iter_mut()
            .for_each(|p| p.v.blocks.retain(|b| !is(&b.elem, id))),
        ElemKind::Binding => bindings_parents(doc, |l| l.retain(|b| !is(&b.elem, id))),
        ElemKind::BindingScheme => {
            bindings(doc).for_each(|b| {
                if let Some(s) = b.v.schemes.as_mut() {
                    s.retain(|x| !is(&x.elem, id));
                }
            });
        }
        ElemKind::Message => doc.messages.retain(|m| !is(&m.elem, id)),
        ElemKind::Requires => doc.requires.retain(|x| !is(&x.elem, id)),
        ElemKind::Source => doc.sources.retain(|x| !is(&x.elem, id)),
        ElemKind::Action => doc.actions.retain(|x| !is(&x.elem, id)),
        ElemKind::View => doc.views.retain(|x| !is(&x.elem, id)),
        ElemKind::ViewOrder | ElemKind::FilterNode | ElemKind::FilterValue => return false,
    }
    count(doc) < before
}

fn bindings_parents(doc: &mut LayoutDoc, mut f: impl FnMut(&mut Vec<Item<BindingDoc>>)) {
    doc.pages
        .iter_mut()
        .flat_map(|p| p.v.blocks.iter_mut())
        .for_each(|b| f(&mut b.v.bindings));
}
