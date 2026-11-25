//! The checks of the frontend nodes (S6-3): the id is there and unique, the key names a program and a frontend, user flags are known,
//! `scope-from` names a node of the same page, and an action `open` names a frontend node. All of them block (red), none is a warning.
//!
//! Pattern: Template method (like `check_places`): `check_frontends` walks the trees once and only appends codes.

use std::collections::HashMap;

use crate::codes::LayoutCode;
use crate::doc::{ActionTarget, Item, LayoutDoc};
use crate::ids::ElemId;
use crate::place::{EmbedDoc, PlaceNode};
use crate::shim::Coded;

type Problems = Vec<Coded<LayoutCode>>;

/// Every node of the trees: (id, is frontend), in document order.
fn nodes<'a>(
    items: &'a [Item<PlaceNode>],
    out: &mut Vec<(&'a Item<PlaceNode>, Option<&'a EmbedDoc>)>,
) {
    for i in items {
        match &i.v {
            PlaceNode::Frontend(e) => out.push((i, Some(e))),
            PlaceNode::Frame(f) => {
                out.push((i, None));
                nodes(&f.children, out);
            }
            PlaceNode::Block(_) => out.push((i, None)),
        }
    }
}

fn bad(elem: &str, cause: &str) -> Coded<LayoutCode> {
    Coded::new(LayoutCode::FrontendInvalid)
        .with("elem", elem)
        .with("cause", cause)
}

/// The checks; a layout without frontend nodes costs one walk and says nothing.
pub(crate) fn check_frontends(doc: &LayoutDoc, out: &mut Problems) {
    let mut ids: HashMap<&str, u32> = HashMap::new(); // tree-node id -> how often (all pages: one window id per program)
    let mut frontends: Vec<ElemId> = Vec::new();
    for p in &doc.pages {
        let mut all = Vec::new();
        nodes(&p.v.places, &mut all);
        for (i, _) in &all {
            if let Some(id) = &i.elem {
                *ids.entry(id.as_str()).or_default() += 1;
            }
        }
        for (i, e) in all.iter().filter_map(|(i, e)| e.map(|e| (*i, e))) {
            let Some(id) = &i.elem else {
                out.push(Coded::new(LayoutCode::ElemMissing).with("line", i.line));
                continue;
            };
            frontends.push(id.clone());
            if e.key.extends.is_empty() {
                out.push(bad(id.as_str(), "extends-empty"));
            }
            if e.key.frontend.is_empty() {
                out.push(bad(id.as_str(), "frontend-empty"));
            }
            for flag in e
                .user
                .iter()
                .filter(|f| !EmbedDoc::USER_FLAGS.contains(&f.as_str()))
            {
                out.push(bad(id.as_str(), "user-unknown").with("flag", flag));
            }
            if let Some(from) = &e.scope_from
                && (from == id || !all.iter().any(|(n, _)| n.elem.as_ref() == Some(from)))
            {
                out.push(bad(id.as_str(), "scope-from-unknown").with("scope-from", from.as_str()));
            }
        }
    }
    for id in &frontends {
        if ids.get(id.as_str()).copied().unwrap_or(0) > 1 {
            out.push(
                Coded::new(LayoutCode::ElemTwice)
                    .with("elem", id.as_str())
                    .with("kind", "frontend"),
            );
        }
    }
    for a in &doc.actions {
        if let ActionTarget::Open { elem } = &a.v.target
            && !frontends.contains(elem)
        {
            out.push(
                Coded::new(LayoutCode::ElemUnknown)
                    .with("elem", elem.as_str())
                    .with("kind", "frontend"),
            );
        }
    }
}
