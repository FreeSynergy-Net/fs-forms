//! The tree of places (F1): leaves in order, recursive walk and removal, ids from the path.

mod support;
use fs_forms_layout::*;
use support::*;

fn check(d: LayoutDoc) -> Result<Layout, Vec<Coded<LayoutCode>>> {
    d.check(&OpenCatalog, &Limits::declared())
}
fn codes<T>(r: Result<T, Vec<Coded<LayoutCode>>>) -> Vec<LayoutCode> {
    r.err().unwrap_or_default().iter().map(|c| c.code).collect()
}

fn blk(place: &str, name: &str) -> Item<PlaceNode> {
    let b = BlockDoc {
        place: place.into(),
        block: name.into(),
        source: None,
        title_key: None,
        text_key: None,
        set: vec![],
        bindings: vec![],
    };
    Item {
        elem: None,
        line: 0,
        v: PlaceNode::Block(b),
    }
}

/// main, side{ axis=row; panel, panel }, status; format 1 so the ids are made from the path.
fn tree_doc() -> LayoutDoc {
    let mut d = doc("board");
    d.format = 1;
    d.pages[0].elem = None;
    d.messages[0].elem = None;
    let page = &mut d.pages[0].v;
    page.blocks.clear();
    let axis = AttrDoc {
        name: "axis".into(),
        value: "row".into(),
    };
    let side = FrameDoc {
        place: "side".into(),
        frame: "split".into(),
        set: vec![Item {
            elem: None,
            line: 0,
            v: axis,
        }],
        children: vec![blk("panel", "board"), blk("panel", "board")],
    };
    page.places = vec![
        blk("main", "board"),
        Item {
            elem: None,
            line: 0,
            v: PlaceNode::Frame(side),
        },
        blk("status", "board"),
    ];
    d
}

#[test]
fn leaves_come_in_document_order_through_frames() {
    let d = tree_doc();
    let places: Vec<&str> = d.pages[0].v.leaves().map(|l| l.v.place.as_str()).collect();
    assert_eq!(places, ["main", "panel", "panel", "status"]);
}

#[test]
fn flat_blocks_stay_readable_and_come_first() {
    let mut d = tree_doc();
    d.pages[0].v.blocks = doc("board").pages[0].v.blocks.clone();
    d.pages[0].v.places.truncate(1);
    d.pages[0].v.places[0] = blk("side", "board");
    let places: Vec<&str> = d.pages[0].v.leaves().map(|l| l.v.place.as_str()).collect();
    assert_eq!(places, ["main", "side"]);
}

#[test]
fn walk_is_recursive_and_ids_are_the_path() {
    let l = check(tree_doc()).unwrap();
    let mut ids = Vec::new();
    for it in &l.doc().pages[0].v.places {
        ids.push(it.elem.clone().unwrap().as_str().to_owned());
        if let PlaceNode::Frame(f) = &it.v {
            ids.push(f.set[0].elem.clone().unwrap().as_str().to_owned());
            ids.extend(
                f.children
                    .iter()
                    .map(|c| c.elem.clone().unwrap().as_str().to_owned()),
            );
        }
    }
    assert_eq!(
        ids,
        [
            "block:board/main",
            "frame:board/side",
            "attr:board/side/axis",
            "block:board/side/panel",
            "block:board/side/panel.1",
            "block:board/status"
        ]
    );
    let again = check(l.to_doc()).unwrap();
    assert_eq!(l.doc().pages[0].v.places, again.doc().pages[0].v.places);
}

#[test]
fn ids_are_unique_and_a_clash_is_found() {
    let mut d = tree_doc();
    assert!(check(d.clone()).is_ok());
    d.format = 2;
    let l = check({
        let mut x = d.clone();
        x.format = 1;
        x
    })
    .unwrap();
    let mut d = l.to_doc();
    d.format = 2;
    if let PlaceNode::Frame(f) = &mut d.pages[0].v.places[1].v {
        f.children[1].elem = f.children[0].elem.clone();
    }
    assert!(codes(check(d)).contains(&LayoutCode::ElemTwice));
}
