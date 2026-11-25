//! The places of one page as the host draws them today: ONE level (A5 F6, "the picture stays the same").
//!
//! A block at depth 1 is a [`PlaceView`] like before. A frame at depth 1 is not built yet: it becomes a place that carries the
//! lock `host-notice-frame-not-built{frame}` (the window draws a `partial-notice`, state `partial`); the blocks inside it are
//! not drawn, and the notice names the frame. Nothing is dropped without a word.
//!
//! Pattern: Adapter over the Composite (`PlaceNode` in, the flat `PlaceView` out) with an explicit "not built" arm.

use fs_forms_layout::{EmbedDoc, Layout, PageDoc, PlaceNode, Placement, Shown};

use crate::view::{Coded, EmbedView, PlaceView};

fn text(layout: &Layout, key: &str, lang: &str) -> String {
    let find = |l: &str| {
        layout
            .doc()
            .messages
            .iter()
            .find(|m| m.v.key == key && m.v.lang == l)
            .map(|m| m.v.message.clone())
    };
    find(lang)
        .or_else(|| find("en"))
        .unwrap_or_else(|| key.to_owned())
}

/// The depth-1 places of `page`: first the flat list of format 2 (stores and wire still fill it), then the tree.
pub(crate) fn place_views(layout: &Layout, page: &PageDoc, lang: &str) -> Vec<PlaceView> {
    let mut out = Vec::new();
    let mut seq = 0u32;
    let mut next = || {
        seq += 1;
        seq - 1
    };
    for b in &page.blocks {
        out.push(block_view(layout, &b.v, next(), lang));
    }
    for n in &page.places {
        out.push(match &n.v {
            PlaceNode::Block(b) => block_view(layout, b, next(), lang),
            PlaceNode::Frame(f) => PlaceView {
                place: f.place.clone(),
                seq: next(),
                block: f.frame.clone(),
                title: None,
                locks: vec![Coded::new("host-notice-frame-not-built").with("frame", &f.frame)],
                slot_locks: Vec::new(),
                embed: None,
            },
            PlaceNode::Frontend(e) => embed_view(
                e,
                n.elem.as_ref().map(fs_forms_layout::ElemId::as_str),
                next(),
            ),
            // a kind of node this host does not know: said, never skipped
            _ => PlaceView {
                place: String::new(),
                seq: next(),
                block: String::new(),
                title: None,
                locks: vec![Coded::new("host-notice-frame-not-built").with("frame", "?")],
                slot_locks: Vec::new(),
                embed: None,
            },
        });
    }
    out
}

fn block_view(layout: &Layout, b: &fs_forms_layout::BlockDoc, seq: u32, lang: &str) -> PlaceView {
    PlaceView {
        place: b.place.clone(),
        seq,
        block: b.block.clone(),
        title: b.title_key.as_deref().map(|k| text(layout, k, lang)),
        locks: Vec::new(),
        slot_locks: Vec::new(),
        embed: None,
    }
}

/// A frontend node: the host does not draw it, the owner of the open frontends opens it (panel here, or a window of its own).
fn embed_view(e: &EmbedDoc, elem: Option<&str>, seq: u32) -> PlaceView {
    PlaceView {
        place: e.place.clone(),
        seq,
        block: "frontend".to_owned(),
        title: None,
        locks: Vec::new(),
        slot_locks: Vec::new(),
        embed: Some(EmbedView {
            elem: elem.unwrap_or(&e.place).to_owned(),
            key: e.key.clone(),
            panel: e.mode == Placement::Panel,
            on_open: e.shown == Shown::OnOpen,
            scope_from: e.scope_from.as_ref().map(|i| i.as_str().to_owned()),
        }),
    }
}
