//! A5 F4: the tree of places and the attributes `set` go to rows and back (Doc to rows to Doc), through a table of example trees
//! (depth 3 and more, several children on one place, attributes on blocks and frames, bindings in the tree), and the counter-tests:
//! rows with a child, an order or an attribute missing must NOT give the same layout back (each one is the red side of the round trip).
use fs_forms_layout::*;
use fs_forms_store::*;

#[path = "tree/cases.rs"]
mod cases;
use cases::*;

mod engine_support;

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
