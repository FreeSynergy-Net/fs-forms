//! The tree of places in the plan (A5 F6): build keeps the shape, every level is in `seq` order, a frame can be locked, and the
//! digest sees the tree and the attributes.
mod common;
use common::*;
use fs_forms_bind::testing::*;
use fs_forms_bind::*;

fn block(elem: &str, place: &str, seq: u32) -> NodeIn {
    NodeIn::Block(PlaceIn {
        elem: elem.into(),
        place: place.into(),
        seq,
        block: "empty-state".into(),
        source: None,
        bindings: vec![],
        set: vec![AttrIn {
            name: "opacity".into(),
            value: "0.6".into(),
        }],
    })
}

fn frame(elem: &str, place: &str, seq: u32, children: Vec<NodeIn>) -> NodeIn {
    NodeIn::Frame(FrameIn {
        elem: elem.into(),
        place: place.into(),
        seq,
        frame: "split".into(),
        set: vec![AttrIn {
            name: "axis".into(),
            value: "row".into(),
        }],
        children,
    })
}

/// Page 0 of the tasks layout, replaced by a tree: split { side (seq 1), split { b, a } (seq 0 -> first) }.
fn tree_resolved() -> Resolved {
    let mut r = tasks_resolved();
    r.pages[0].places = vec![frame(
        "f-root",
        "main",
        0,
        vec![
            block("pl-side", "side", 1),
            frame(
                "f-inner",
                "panel",
                0,
                vec![block("pl-b", "b", 1), block("pl-a", "a", 0)],
            ),
        ],
    )];
    r
}

fn plan_of(r: &Resolved) -> Plan {
    build_with(r, &TestCatalog::new(), &FakeFacts(vec![]), host())
}

#[test]
fn the_plan_keeps_the_tree_and_sorts_every_level_by_seq() {
    let p = plan_of(&tree_resolved());
    let page = &p.pages[0];
    let NodePlan::Frame(root) = &page.places[0] else {
        panic!("root is a frame")
    };
    assert_eq!(root.frame, "split");
    assert_eq!(root.set[0].name, "axis");
    assert_eq!(root.set[0].value, "row");
    // seq 0 (the inner frame) comes before seq 1 (the block `side`)
    assert!(matches!(&root.children[0], NodePlan::Frame(f) if f.place == "panel"));
    assert!(matches!(&root.children[1], NodePlan::Block(b) if b.place == "side"));
    let leaves: Vec<&str> = page.leaves().iter().map(|b| b.place.as_str()).collect();
    assert_eq!(leaves, ["a", "b", "side"], "depth first, seq order");
    assert_eq!(page.leaves()[0].set[0].name, "opacity");
}

#[test]
fn a_problem_on_a_frame_locks_the_frame_not_the_page() {
    let mut r = tree_resolved();
    r.problems
        .push(problem("f-inner", "layout-error-place-takes"));
    let p = plan_of(&r);
    let page = &p.pages[0];
    assert_eq!(page.frame_locks().len(), 1);
    assert_eq!(page.locks.len(), 0);
    assert!(p.locks().iter().any(|l| l.part.text().contains("panel")));
    assert_eq!(p.state, PlanState::Partial);
}

#[test]
fn a_frame_with_blocks_inside_draws_the_blocks() {
    let p = plan_of(&tree_resolved());
    let name = p.pages[0].page.clone();
    assert!(p.place(&name, "a").is_some(), "found through the tree");
}

#[test]
fn the_digest_sees_the_tree_and_the_attributes() {
    let base = plan_of(&tree_resolved());
    let d = base.digest_without_style();
    assert_eq!(d, plan_of(&tree_resolved()).digest_without_style());

    // counter-proofs: each change of the tree or of an attribute gives another digest
    let mut r = tree_resolved();
    if let NodeIn::Frame(f) = &mut r.pages[0].places[0] {
        f.set[0].value = "column".into();
    }
    assert_ne!(plan_of(&r).digest_without_style(), d, "frame attribute");

    let mut r = tree_resolved();
    if let NodeIn::Frame(f) = &mut r.pages[0].places[0]
        && let NodeIn::Block(b) = &mut f.children[0]
    {
        b.set[0].value = "1".into();
    }
    assert_ne!(plan_of(&r).digest_without_style(), d, "block attribute");

    let mut r = tree_resolved();
    if let NodeIn::Frame(f) = &mut r.pages[0].places[0] {
        f.children.pop();
    }
    assert_ne!(plan_of(&r).digest_without_style(), d, "a child less");

    // the same blocks, one level up (flat) is another plan than the nested one
    let mut r = tree_resolved();
    r.pages[0].places = vec![
        block("pl-side", "side", 1),
        block("pl-b", "b", 2),
        block("pl-a", "a", 0),
    ];
    assert_ne!(
        plan_of(&r).digest_without_style(),
        d,
        "flat instead of nested"
    );
}
