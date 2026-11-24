//! A5 F4: the tree of places and the attributes `set` go to rows and back (Doc to rows to Doc), through a table of example trees
//! (depth 3 and more, several children on one place, attributes on blocks and frames, bindings in the tree), and the counter-tests:
//! rows with a child, an order or an attribute missing must NOT give the same layout back (each one is the red side of the round trip).
use fs_forms_layout::*;
use fs_forms_store::*;

mod engine_support;

fn it<T>(e: &str, v: T) -> Item<T> {
    Item {
        elem: Some(ElemId::parse(e).unwrap()),
        line: 0,
        v,
    }
}
fn set(e: &str, list: &[(&str, &str)]) -> Vec<Item<AttrDoc>> {
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
fn block(e: &str, place: &str, name: &str, attrs: &[(&str, &str)]) -> Item<PlaceNode> {
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
fn frame(
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
fn full_block(e: &str, place: &str) -> Item<PlaceNode> {
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
                },
            ),
        ];
    }
    b
}

fn layout(places: Vec<Item<PlaceNode>>, flat: Vec<Item<BlockDoc>>) -> Layout {
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
        format: 3,
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
    };
    doc.check(&OpenCatalog, &Limits::declared()).expect("check")
}

fn meta() -> LayoutMeta {
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

/// The table of example trees: (name, tree, flat blocks).
fn cases() -> Vec<(&'static str, Vec<Item<PlaceNode>>, Vec<Item<BlockDoc>>)> {
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

#[test]
fn doc_to_rows_to_doc_is_the_same_layout_for_every_example_tree() {
    for (name, places, flat) in cases() {
        let l = layout(places, flat);
        let rows = to_rows(&l, &meta()).expect(name);
        assert_eq!(validate(&rows), Vec::<String>::new(), "{name}");
        let back = from_rows(&rows, &meta().id).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(back, l, "{name}");
        assert_eq!(back.digest(), l.digest(), "{name}");
    }
}

#[test]
fn the_rows_have_the_shape_the_tables_say() {
    let l = layout(cases().remove(2).1, vec![]);
    let rows = to_rows(&l, &meta()).unwrap();
    let nodes = &rows.tables["forms_node"];
    assert_eq!(nodes.len(), 7); // main, side, a, b, deep, leaf, c
}

fn tree_layout() -> Layout {
    layout(cases().remove(2).1, vec![])
}
fn col(table: &str, name: &str) -> usize {
    ALL.iter()
        .find(|t| t.name == table)
        .unwrap()
        .cols
        .iter()
        .position(|c| c.name == name)
        .unwrap()
}
fn find(rows: &Rows, table: &str, column: &str, value: &str) -> usize {
    let c = col(table, column);
    rows.tables[table]
        .iter()
        .position(|r| r[c] == Cell::Text(value.into()))
        .unwrap_or_else(|| panic!("{table}.{column} = {value}"))
}

/// Counter-test 1: a round trip that loses a child must be red. Rows without the leaf `c` give a different layout.
#[test]
fn a_round_trip_that_loses_a_child_is_not_the_same_layout() {
    let l = tree_layout();
    let mut rows = to_rows(&l, &meta()).unwrap();
    let at = find(&rows, "forms_node", "elem", "block:c");
    rows.tables.get_mut("forms_node").unwrap().remove(at);
    // the attribute of the lost child goes with it, as a foreign key would force
    let owner = find(&rows, "forms_attr", "owner", "block:c");
    rows.tables.get_mut("forms_attr").unwrap().remove(owner);
    let back = from_rows(&rows, &meta().id).expect("the rest is still a layout");
    assert_ne!(back, l);
    assert_eq!(back.doc().pages[0].v.leaves().count(), 3);
}

/// A node whose parent is gone is not silently dropped: the rows are broken.
#[test]
fn a_node_that_no_page_reaches_is_reported() {
    let mut rows = to_rows(&tree_layout(), &meta()).unwrap();
    let at = find(&rows, "forms_node", "elem", "frame:side");
    rows.tables.get_mut("forms_node").unwrap().remove(at);
    let e = from_rows(&rows, &meta().id).unwrap_err();
    assert_eq!(e[0].code, LayoutCode::StoreBroken);
    assert!(e[0].param("detail").unwrap().contains("not reachable"));
}

/// Counter-test 2: the order of the children is the column `seq`, not the order of the rows.
#[test]
fn the_seq_column_is_the_order_of_the_children() {
    let l = tree_layout();
    let mut rows = to_rows(&l, &meta()).unwrap();
    let seq = col("forms_node", "seq");
    // 1. rows in another order, same seq: same layout (the order of rows is not the order of the tree)
    rows.tables.get_mut("forms_node").unwrap().reverse();
    assert_eq!(from_rows(&rows, &meta().id).unwrap(), l);
    // 2. the two children of the frame `main` swap their seq: the layout changes, and `c` now comes first
    let (side, c) = (
        find(&rows, "forms_node", "elem", "frame:side"),
        find(&rows, "forms_node", "elem", "block:c"),
    );
    let t = rows.tables.get_mut("forms_node").unwrap();
    t[side][seq] = Cell::Int(1);
    t[c][seq] = Cell::Int(0);
    let back = from_rows(&rows, &meta().id).unwrap();
    assert_ne!(back, l);
    let PlaceNode::Frame(main) = &back.doc().pages[0].v.places[0].v else {
        panic!("frame")
    };
    assert_eq!(main.children[0].v.place(), "right");
    assert_eq!(main.children[1].v.place(), "left");
}

/// Counter-test 3: an attribute that is lost in the rows is a different layout; its order is `seq`.
#[test]
fn a_lost_attribute_is_not_the_same_layout() {
    let l = tree_layout();
    let mut rows = to_rows(&l, &meta()).unwrap();
    let at = find(&rows, "forms_attr", "name", "sizes");
    rows.tables.get_mut("forms_attr").unwrap().remove(at);
    let back = from_rows(&rows, &meta().id).unwrap();
    assert_ne!(back, l);
    let PlaceNode::Frame(main) = &back.doc().pages[0].v.places[0].v else {
        panic!("frame")
    };
    assert_eq!(main.set.len(), 1);
}

#[test]
fn attributes_keep_their_order_and_flat_blocks_keep_theirs() {
    let l = layout(vec![], cases().remove(5).2);
    let mut rows = to_rows(&l, &meta()).unwrap();
    rows.tables.get_mut("forms_attr").unwrap().reverse(); // the order is `seq`, not the order of the rows
    let back = from_rows(&rows, &meta().id).unwrap();
    assert_eq!(back, l);
    let b = &back.doc().pages[0].v.blocks[0].v;
    let names: Vec<&str> = b.set.iter().map(|a| a.v.name.as_str()).collect();
    assert_eq!(names, ["hidden", "opacity"]);
}

/// The real engine (turso, foreign keys on): the rows of the tree go in, parents first, and a node under a parent that does not exist is refused.
#[test]
fn the_engine_takes_the_tree_and_refuses_a_node_without_its_parent() {
    use engine_support::{insert, open, run};
    run(async {
        let rows = to_rows(&tree_layout(), &meta()).unwrap();
        insert(&open().await, &rows).await.unwrap();
        let mut orphan = rows.clone();
        let (at, parent) = (
            find(&orphan, "forms_node", "elem", "block:a"),
            col("forms_node", "parent"),
        );
        orphan.tables.get_mut("forms_node").unwrap()[at][parent] =
            Cell::Text("frame:nowhere".into());
        assert!(insert(&open().await, &orphan).await.is_err());
        // an attribute name twice on one owner is refused by the primary key
        let mut twice = rows;
        let first = twice.tables["forms_attr"][0].clone();
        twice.tables.get_mut("forms_attr").unwrap().push(first);
        assert!(insert(&open().await, &twice).await.is_err());
    });
}
