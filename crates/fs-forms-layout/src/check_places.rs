//! The checks of the tree of places (A5 F2): K4 a place the frame does not declare, K6 what a place takes and how many children,
//! K8 ids of one page, the depth limit, and K20 (`set` keys against the catalog, a warning that never blocks).
//!
//! Pattern: Template method: `check` calls these in a fixed order; each function only appends codes. The catalog is a Strategy
//! ([`CatalogRules`]); `OpenCatalog` is its Null Object and declares nothing, so nothing is checked against it.

use std::collections::HashMap;

use crate::check::CatalogRules;
use crate::codes::LayoutCode;
use crate::doc::{Item, LayoutDoc, PageDoc};
use crate::ids::{ElemId, ElemKind};
use crate::limits::Limits;
use crate::place::{AttrDoc, PlaceNode};
use crate::shim::Coded;

type Problems = Vec<Coded<LayoutCode>>;

fn elem<T>(i: &Item<T>) -> &str {
    i.elem.as_ref().map_or("", ElemId::as_str)
}

/// The catalog name of a node: its block or its frame.
fn name(n: &PlaceNode) -> &str {
    match n {
        PlaceNode::Block(b) => &b.block,
        PlaceNode::Frame(f) => &f.frame,
    }
}

/// Depth of the tree: a node on the page root is 1, a child of a frame there is 2 ...
fn depth(items: &[Item<PlaceNode>]) -> u32 {
    items
        .iter()
        .map(|i| match &i.v {
            PlaceNode::Frame(f) => 1 + depth(&f.children),
            PlaceNode::Block(_) => 1,
        })
        .max()
        .unwrap_or(0)
}

/// Blocking checks of the tree of one page. A page without a tree (format 2, flat list) is not checked here.
pub(crate) fn check_tree(
    page: &Item<PageDoc>,
    catalog: &dyn CatalogRules,
    limits: &Limits,
    out: &mut Problems,
) {
    let p = &page.v;
    if p.places.is_empty() {
        return;
    }
    if depth(&p.places) > limits.place_depth_max {
        out.push(
            Coded::new(LayoutCode::TooMany)
                .with("what", "place-depth")
                .with("max", limits.place_depth_max)
                .with("by", limits.by.place_depth.as_str())
                .with("elem", elem(page)),
        );
    }
    level(&p.frame, elem(page), &p.places, catalog, out);
    page_ids(p, out);
}

/// K4 and K6 for the children of one frame (or of the page root, whose frame is `PageDoc.frame`), then for the frames below.
fn level(
    frame: &str,
    owner: &str,
    kids: &[Item<PlaceNode>],
    catalog: &dyn CatalogRules,
    out: &mut Problems,
) {
    if let Some(rules) = catalog.places_of(frame) {
        for k in kids {
            let place = k.v.place();
            let Some(rule) = rules.iter().find(|r| r.place == place) else {
                out.push(
                    Coded::new(LayoutCode::PlaceUnknown)
                        .with("elem", elem(k))
                        .with("frame", frame)
                        .with("place", place),
                );
                continue;
            };
            let n = name(&k.v);
            if !rule.takes.is_empty() && !rule.takes.iter().any(|t| t == n) {
                out.push(
                    Coded::new(LayoutCode::PlaceTakes)
                        .with("elem", elem(k))
                        .with("place", place)
                        .with("name", n),
                );
            }
        }
        for r in &rules {
            let count = kids.iter().filter(|k| k.v.place() == r.place).count();
            let count = u32::try_from(count).unwrap_or(u32::MAX);
            if count < r.min || count > r.max {
                out.push(
                    Coded::new(LayoutCode::PlaceCount)
                        .with("elem", owner)
                        .with("place", &r.place)
                        .with("count", count)
                        .with("min", r.min)
                        .with("max", r.max),
                );
            }
        }
    }
    for k in kids {
        if let PlaceNode::Frame(f) = &k.v {
            level(&f.frame, elem(k), &f.children, catalog, out);
        }
    }
}

/// K8: one id names one element of a page, whatever its kind and depth. The same id twice in one kind is already `elem-twice` from
/// the walk; here the same id on different kinds (a block and a frame, a frame and an attribute) is reported.
fn page_ids(p: &PageDoc, out: &mut Problems) {
    fn go(items: &[Item<PlaceNode>], seen: &mut HashMap<String, ElemKind>, out: &mut Problems) {
        for i in items {
            note(i.elem.as_ref(), kind_of(&i.v), seen, out);
            match &i.v {
                PlaceNode::Block(b) => attr_ids(&b.set, seen, out),
                PlaceNode::Frame(f) => {
                    attr_ids(&f.set, seen, out);
                    go(&f.children, seen, out);
                }
            }
        }
    }
    let mut seen = HashMap::new();
    go(&p.places, &mut seen, out);
}

fn kind_of(n: &PlaceNode) -> ElemKind {
    match n {
        PlaceNode::Block(_) => ElemKind::Block,
        PlaceNode::Frame(_) => ElemKind::Frame,
    }
}

fn attr_ids(set: &[Item<AttrDoc>], seen: &mut HashMap<String, ElemKind>, out: &mut Problems) {
    for a in set {
        note(a.elem.as_ref(), ElemKind::Attr, seen, out);
    }
}

fn note(
    id: Option<&ElemId>,
    kind: ElemKind,
    seen: &mut HashMap<String, ElemKind>,
    out: &mut Problems,
) {
    let Some(id) = id else { return };
    if let Some(first) = seen.insert(id.as_str().to_owned(), kind)
        && first != kind
    {
        out.push(
            Coded::new(LayoutCode::ElemTwice)
                .with("elem", id.as_str())
                .with("kind", kind.as_str()),
        );
    }
}

/// K20, non-blocking: `set` keys of blocks and frames that the catalog does not list in their `sets`. Yellow, never red: the
/// host still shows the page; the notice says which key was ignored. The catalog says nothing about a node (`None`): no notice.
pub(crate) fn set_warnings(doc: &LayoutDoc, catalog: &dyn CatalogRules) -> Problems {
    fn check(name: &str, set: &[Item<AttrDoc>], c: &dyn CatalogRules, out: &mut Problems) {
        let Some(allowed) = c.sets_of(name) else {
            return;
        };
        for a in set.iter().filter(|a| !allowed.contains(&a.v.name)) {
            out.push(
                Coded::new(LayoutCode::SetUnknown)
                    .with("elem", elem(a))
                    .with("name", &a.v.name)
                    .with("owner", name),
            );
        }
    }
    fn go(items: &[Item<PlaceNode>], c: &dyn CatalogRules, out: &mut Problems) {
        for i in items {
            match &i.v {
                PlaceNode::Block(b) => check(&b.block, &b.set, c, out),
                PlaceNode::Frame(f) => {
                    check(&f.frame, &f.set, c, out);
                    go(&f.children, c, out);
                }
            }
        }
    }
    let mut out = Vec::new();
    for p in &doc.pages {
        for b in &p.v.blocks {
            check(&b.v.block, &b.v.set, catalog, &mut out);
        }
        go(&p.v.places, catalog, &mut out);
    }
    out
}
