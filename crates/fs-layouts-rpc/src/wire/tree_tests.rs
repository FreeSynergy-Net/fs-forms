//! Tests of the tree of places on the wire: round trips doc -> capnp -> doc over example trees (the guard has its own file,
//! `tree_guard_tests`). The wire is built with the real writer and read with the real reader, no model shortcut.
//!
//! Pattern: Composite (tests of the tree over the wire); the cases are a table, one function checks them all.

use fs_forms_layout::{
    AttrDoc, BlockDoc, ElemId, FrameDoc, FrontendKey, Item, LayoutCode, LayoutDoc, Navigation,
    PageDoc, PlaceNode,
};

use super::doc_in::read_doc;
use super::doc_out::write_doc;
use super::tree_guard::TreeLimits;
use crate::layouts_capnp as w;

pub(super) fn it<T>(e: Option<&str>, v: T) -> Item<T> {
    Item {
        elem: e.map(|e| ElemId::parse(e).unwrap()),
        line: 0,
        v,
    }
}

pub(super) fn attr(e: Option<&str>, name: &str, value: &str) -> Item<AttrDoc> {
    it(
        e,
        AttrDoc {
            name: name.into(),
            value: value.into(),
        },
    )
}

pub(super) fn block(e: Option<&str>, place: &str, set: Vec<Item<AttrDoc>>) -> Item<PlaceNode> {
    let b = BlockDoc {
        place: place.into(),
        block: "list".into(),
        source: Some(2),
        title_key: Some("t".into()),
        text_key: None,
        bindings: vec![],
        set,
    };
    it(e, PlaceNode::Block(b))
}

pub(super) fn frame(
    e: Option<&str>,
    place: &str,
    set: Vec<Item<AttrDoc>>,
    children: Vec<Item<PlaceNode>>,
) -> Item<PlaceNode> {
    it(
        e,
        PlaceNode::Frame(FrameDoc {
            place: place.into(),
            frame: "split".into(),
            set,
            children,
        }),
    )
}

/// A chain of `depth` nodes: frames inside frames, the deepest one a block (depth 1 = a block on the page root).
pub(super) fn chain(depth: u32) -> Vec<Item<PlaceNode>> {
    let mut node = block(None, "main", vec![attr(None, "k", "v")]);
    for _ in 1..depth {
        node = frame(None, "main", vec![], vec![node]);
    }
    vec![node]
}

pub(super) fn doc(places: Vec<Item<PlaceNode>>) -> LayoutDoc {
    let page = PageDoc {
        page: "main".into(),
        frame: "app-window".into(),
        navigation: Navigation::Single,
        blocks: vec![],
        places,
    };
    LayoutDoc {
        format: 3,
        key: FrontendKey {
            extends: "people".into(),
            frontend: "desktop".into(),
        },
        exec: String::new(),
        title_key: String::new(),
        style: None,
        requires: vec![],
        sources: vec![],
        menu: vec![],
        pages: vec![it(Some("page:main"), page)],
        actions: vec![],
        views: vec![],
        messages: vec![],
        sample: None,
    }
}

pub(super) fn wire(d: &LayoutDoc) -> capnp::message::Builder<capnp::message::HeapAllocator> {
    let mut m = capnp::message::Builder::new_default();
    write_doc(m.init_root::<w::layout_doc::Builder<'_>>(), d).unwrap();
    m
}

pub(super) fn read(
    m: &capnp::message::Builder<capnp::message::HeapAllocator>,
    lim: &TreeLimits,
) -> Result<LayoutDoc, fs_forms_layout::Coded<LayoutCode>> {
    read_doc(
        m.get_root_as_reader::<w::layout_doc::Reader<'_>>().unwrap(),
        lim,
    )
}

fn trips(d: &LayoutDoc) -> LayoutDoc {
    read(&wire(d), &TreeLimits::declared()).unwrap()
}

/// The example trees: depth 4 with several children on one place and on several, attributes on frames and blocks, an empty frame.
fn trees() -> Vec<(&'static str, Vec<Item<PlaceNode>>)> {
    let deep = frame(
        Some("frame:main/root"),
        "main",
        vec![
            attr(Some("attr:main/root/axis"), "axis", "row"),
            attr(None, "sizes", "30,70"),
        ],
        vec![
            block(
                Some("block:main/root/side"),
                "side",
                vec![attr(None, "mode", "tree")],
            ),
            frame(
                None,
                "body",
                vec![attr(None, "active", "1")],
                vec![
                    block(None, "tab", vec![]),
                    block(None, "tab", vec![attr(None, "label", "two")]),
                    frame(
                        None,
                        "tab",
                        vec![],
                        vec![block(
                            None,
                            "panel",
                            vec![attr(None, "a", "b"), attr(None, "c", "d")],
                        )],
                    ),
                ],
            ),
            frame(None, "foot", vec![], vec![]),
        ],
    );
    vec![
        ("empty page", vec![]),
        (
            "flat blocks on the root",
            vec![
                block(None, "a", vec![]),
                block(None, "a", vec![attr(None, "x", "1")]),
            ],
        ),
        ("depth 4, several children", vec![deep]),
        ("chain of 8", chain(8)),
    ]
}

#[test]
fn every_tree_comes_back_equal() {
    for (name, t) in trees() {
        let d = doc(t);
        assert_eq!(trips(&d), d, "tree `{name}`");
    }
}

#[test]
fn flat_blocks_travel_with_their_attributes() {
    let mut d = doc(vec![]);
    let PlaceNode::Block(b) = block(None, "main", vec![attr(None, "k", "v")]).v else {
        unreachable!()
    };
    d.pages[0].v.blocks.push(it(Some("block:main/flat"), b));
    assert_eq!(trips(&d), d);
}

/// The red side of the round trip: a loss would be seen. One child, one attribute, one value less is another description.
#[test]
fn a_lost_child_or_attribute_would_be_seen() {
    let d = doc(trees().remove(2).1);
    let back = trips(&d);
    let PlaceNode::Frame(root) = &back.pages[0].v.places[0].v else {
        panic!("frame")
    };
    let (mut less_child, mut less_attr, mut other_value) =
        (back.clone(), back.clone(), back.clone());
    let fr = |d: &mut LayoutDoc| match &mut d.pages[0].v.places[0].v {
        PlaceNode::Frame(f) => f.clone(),
        _ => unreachable!(),
    };
    let mut f = fr(&mut less_child);
    f.children.pop();
    less_child.pages[0].v.places[0].v = PlaceNode::Frame(f);
    let mut f = fr(&mut less_attr);
    f.set.pop();
    less_attr.pages[0].v.places[0].v = PlaceNode::Frame(f);
    let mut f = fr(&mut other_value);
    f.set[0].v.value = "column".into();
    other_value.pages[0].v.places[0].v = PlaceNode::Frame(f);
    for lost in [less_child, less_attr, other_value] {
        assert_ne!(lost, d);
    }
    assert_eq!(root.children.len(), 3);
}

#[test]
fn the_order_of_the_children_is_kept() {
    let kids = (0..6)
        .map(|i| block(None, "tab", vec![attr(None, "n", &i.to_string())]))
        .collect();
    let d = doc(vec![frame(None, "main", vec![], kids)]);
    let back = trips(&d);
    let PlaceNode::Frame(f) = &back.pages[0].v.places[0].v else {
        panic!("frame")
    };
    let order: Vec<_> = f
        .children
        .iter()
        .map(|c| match &c.v {
            PlaceNode::Block(b) => b.set[0].v.value.clone(),
            _ => String::new(),
        })
        .collect();
    assert_eq!(order, ["0", "1", "2", "3", "4", "5"]);
}
