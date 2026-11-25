//! Example trees and builders for the `tree` round-trip tests (shared helpers, no tests).
use fs_forms_layout::*;
use fs_forms_store::*;

pub fn it<T>(e: &str, v: T) -> Item<T> {
    Item {
        elem: Some(ElemId::parse(e).unwrap()),
        line: 0,
        v,
    }
}
pub fn set(e: &str, list: &[(&str, &str)]) -> Vec<Item<AttrDoc>> {
    list.iter()
        .enumerate()
        .map(|(n, (k, v))| {
            it(
                &format!("attr:{e}/{n}"),
                AttrDoc {
                    name: (*k).into(),
                    value: (*v).into(),
                },
            )
        })
        .collect()
}
pub fn block(e: &str, place: &str, name: &str, attrs: &[(&str, &str)]) -> Item<PlaceNode> {
    it(
        &format!("block:{e}"),
        PlaceNode::Block(BlockDoc {
            place: place.into(),
            block: name.into(),
            source: None,
            title_key: None,
            text_key: None,
            bindings: Vec::new(),
            set: set(e, attrs),
        }),
    )
}
pub fn frame(
    e: &str,
    place: &str,
    name: &str,
    attrs: &[(&str, &str)],
    children: Vec<Item<PlaceNode>>,
) -> Item<PlaceNode> {
    it(
        &format!("frame:{e}"),
        PlaceNode::Frame(FrameDoc {
            place: place.into(),
            frame: name.into(),
            set: set(e, attrs),
            children,
        }),
    )
}
/// A block with a title key, a source and two bindings (one with a scheme list): the columns of a leaf in the tree.
pub fn full_block(e: &str, place: &str) -> Item<PlaceNode> {
    let mut b = block(e, place, "field-card", &[("order", "2")]);
    if let PlaceNode::Block(x) = &mut b.v {
        x.source = Some(1);
        x.title_key = Some("t".into());
        x.text_key = Some("tx".into());
        x.bindings = vec![
            it(
                &format!("binding:{e}/a"),
                BindingDoc {
                    slot: "group".into(),
                    nth: 0,
                    from: BindFrom::Grouping("status".into()),
                    schemes: None,
                    source: None,
                },
            ),
            it(
                &format!("binding:{e}/b"),
                BindingDoc {
                    slot: "value".into(),
                    nth: 0,
                    from: BindFrom::Field {
                        path: "homepage".into(),
                        role: "input".into(),
                    },
                    schemes: Some(vec![
                        it(&format!("scheme:{e}/0"), "ssh".to_owned()),
                        it(&format!("scheme:{e}/1"), "https".to_owned()),
                    ]),
                    source: None,
                },
            ),
        ];
    }
    b
}

pub fn layout(places: Vec<Item<PlaceNode>>, flat: Vec<Item<BlockDoc>>) -> Layout {
    let msg = |lang: &str| {
        it(
            &format!("message:t/{lang}"),
            MessageDoc {
                key: "t".into(),
                lang: lang.into(),
                message: "T".into(),
            },
        )
    };
    let doc = LayoutDoc {
        format: 4,
        key: FrontendKey {
            extends: "net.example.Tasks".into(),
            frontend: "board".into(),
        },
        exec: "fs-forms-run".into(),
        title_key: "t".into(),
        style: None,
        requires: Vec::new(),
        sources: vec![it(
            "source:1",
            SourceDoc {
                range: "tasks@>=0.0.1, <0.1.0".into(),
                data_type: "Task".into(),
                scope: None,
            },
        )],
        menu: Vec::new(),
        pages: vec![it(
            "page:board",
            PageDoc {
                page: "board".into(),
                frame: "app-window".into(),
                navigation: Navigation::Single,
                blocks: flat,
                places,
            },
        )],
        actions: Vec::new(),
        views: Vec::new(),
        messages: vec![msg("de"), msg("en")],
        sample: None,
    };
    doc.check(&OpenCatalog, &Limits::declared()).expect("check")
}

pub fn meta() -> LayoutMeta {
    LayoutMeta {
        id: LayoutId(ObjectId("layout-1".into())),
        origin: Origin::Package,
        package: Some(PackageRef {
            package: "net.example.pkg".into(),
            version: "1.0".into(),
        }),
        owner: None,
        rev: Rev(1),
        created: Utc(10),
        updated: Utc(10),
        updated_by: Reference("https://example.org/p".into()),
    }
}

pub type Case = (&'static str, Vec<Item<PlaceNode>>, Vec<Item<BlockDoc>>);

/// The table of example trees: (name, tree, flat blocks).
pub fn cases() -> Vec<Case> {
    let deep = vec![frame(
        "main",
        "main",
        "split",
        &[("axis", "row"), ("sizes", "0.3 0.7")],
        vec![
            frame(
                "side",
                "left",
                "tabs",
                &[("active", "0")],
                vec![
                    block("a", "tab", "contact-row", &[("opacity", "0.6")]),
                    block("b", "tab", "contact-row", &[]),
                    frame(
                        "deep",
                        "tab",
                        "split",
                        &[("axis", "column")],
                        vec![full_block("leaf", "pane")],
                    ),
                ],
            ),
            block("c", "right", "table", &[("min-width", "16em")]),
        ],
    )];
    let flat = |e: &str, place: &str| {
        it(
            &format!("block:{e}"),
            BlockDoc {
                place: place.into(),
                block: "sidebar".into(),
                source: None,
                title_key: None,
                text_key: None,
                bindings: Vec::new(),
                set: set(e, &[("hidden", "keep"), ("opacity", "0.5")]),
            },
        )
    };
    vec![
        ("one block", vec![block("a", "main", "table", &[])], vec![]),
        (
            "a leaf with all columns",
            vec![full_block("a", "main")],
            vec![],
        ),
        ("depth 4, attributes, bindings", deep, vec![]),
        (
            "several children on one place",
            vec![
                block("a", "nav", "sidebar", &[]),
                block("b", "nav", "sidebar", &[("order", "2")]),
                block("c", "nav", "table", &[("order", "1")]),
            ],
            vec![],
        ),
        (
            "attributes in a frame only, empty frame",
            vec![frame(
                "f",
                "main",
                "split",
                &[("axis", "row"), ("sizes", "1")],
                vec![],
            )],
            vec![],
        ),
        (
            "flat blocks with set beside a tree",
            vec![frame(
                "f",
                "main",
                "split",
                &[],
                vec![block("a", "x", "table", &[])],
            )],
            vec![flat("flat1", "nav"), flat("flat2", "aside")],
        ),
        ("an empty page", vec![], vec![]),
    ]
}
