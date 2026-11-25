//! Unit tests of the tree: removal and lookup reach into frames.
//!
//! Pattern: Composite (tests of the tree of `PlaceNode`).

use crate::doc::{BlockDoc, Item, LayoutDoc, PageDoc};
use crate::elems::remove;
use crate::ids::{ElemId, ElemKind, ElemRef};
use crate::place::{AttrDoc, FrameDoc, PlaceNode};

fn it<T>(e: &str, v: T) -> Item<T> {
    Item {
        elem: Some(ElemId::parse(e).unwrap()),
        line: 0,
        v,
    }
}
fn blk(e: &str) -> Item<PlaceNode> {
    let b = BlockDoc {
        place: "panel".into(),
        block: "b".into(),
        source: None,
        title_key: None,
        text_key: None,
        set: vec![],
        bindings: vec![],
    };
    it(e, PlaceNode::Block(b))
}
fn page() -> PageDoc {
    let frame = FrameDoc {
        place: "side".into(),
        frame: "split".into(),
        set: vec![it(
            "attr:p/side/axis",
            AttrDoc {
                name: "axis".into(),
                value: "row".into(),
            },
        )],
        children: vec![blk("block:p/side/panel"), blk("block:p/side/panel.1")],
    };
    PageDoc {
        page: "p".into(),
        frame: "workbench".into(),
        navigation: crate::doc::Navigation::Single,
        blocks: vec![],
        places: vec![it("frame:p/side", PlaceNode::Frame(frame))],
    }
}
fn layout(p: PageDoc) -> LayoutDoc {
    crate::doc::LayoutDoc {
        format: 4,
        key: crate::ids::FrontendKey {
            extends: "x".into(),
            frontend: "y".into(),
        },
        exec: String::new(),
        title_key: String::new(),
        style: None,
        requires: vec![],
        sources: vec![],
        menu: vec![],
        pages: vec![it("page:p", p)],
        actions: vec![],
        views: vec![],
        messages: vec![],
        sample: None,
    }
}
fn r(kind: ElemKind, e: &str) -> ElemRef {
    ElemRef {
        kind,
        elem: ElemId::parse(e).unwrap(),
    }
}

#[test]
fn remove_block_in_frame_keeps_the_others() {
    let mut d = layout(page());
    assert!(remove(&mut d, &r(ElemKind::Block, "block:p/side/panel.1")));
    assert_eq!(d.pages[0].v.leaves().count(), 1);
}

#[test]
fn remove_frame_takes_children_and_attr_goes_alone() {
    let mut d = layout(page());
    assert!(remove(&mut d, &r(ElemKind::Attr, "attr:p/side/axis")));
    assert_eq!(d.pages[0].v.leaves().count(), 2);
    assert!(remove(&mut d, &r(ElemKind::Frame, "frame:p/side")));
    assert_eq!(d.pages[0].v.leaves().count(), 0);
    assert!(!remove(&mut d, &r(ElemKind::Frame, "frame:p/side")));
}
