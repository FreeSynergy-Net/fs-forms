//! The checks of the tree of places (A5 F2): K4, K6, K8 in depth, K20, and the depth limit.

mod support;
use fs_forms_layout::*;
use support::*;

/// `app` declares `side` (takes `panel` blocks, 1 to 2) and `main` (takes anything, exactly 1); `split` declares `pane` (2 to 3).
struct Cat;
impl CatalogRules for Cat {
    fn block_known(&self, _: &str) -> bool {
        true
    }
    fn slot_known(&self, _: &str, _: &str) -> bool {
        true
    }
    fn slot_repeat(&self, _: &str, _: &str) -> u32 {
        16
    }
    fn places_of(&self, frame: &str) -> Option<Vec<PlaceRule>> {
        let rule = |place: &str, takes: &[&str], min, max| PlaceRule {
            place: place.into(),
            takes: takes.iter().map(|t| (*t).to_owned()).collect(),
            min,
            max,
        };
        match frame {
            "app" => Some(vec![
                rule("main", &[], 1, 1),
                rule("side", &["panel"], 1, 2),
            ]),
            "split" => Some(vec![rule("pane", &[], 2, 3)]),
            _ => None,
        }
    }
    fn sets_of(&self, name: &str) -> Option<Vec<String>> {
        match name {
            "panel" => Some(vec!["opacity".into()]),
            "split" => Some(vec!["axis".into(), "sizes".into()]),
            _ => None,
        }
    }
}

fn run(d: LayoutDoc) -> Result<Layout, Vec<Coded<LayoutCode>>> {
    d.check(&Cat, &Limits::declared())
}
fn codes(r: Result<Layout, Vec<Coded<LayoutCode>>>) -> Vec<LayoutCode> {
    r.err().unwrap_or_default().iter().map(|c| c.code).collect()
}

fn blk(place: &str, name: &str) -> Item<PlaceNode> {
    bare(PlaceNode::Block(BlockDoc {
        place: place.into(),
        block: name.into(),
        source: None,
        title_key: None,
        text_key: None,
        set: vec![],
        bindings: vec![],
    }))
}
fn attr(name: &str) -> Item<AttrDoc> {
    bare(AttrDoc {
        name: name.into(),
        value: "x".into(),
    })
}
fn frame(place: &str, name: &str, kids: Vec<Item<PlaceNode>>) -> Item<PlaceNode> {
    bare(PlaceNode::Frame(FrameDoc {
        place: place.into(),
        frame: name.into(),
        set: vec![],
        children: kids,
    }))
}
fn page(places: Vec<Item<PlaceNode>>) -> LayoutDoc {
    let mut d = doc("board");
    d.format = 1;
    d.pages[0].elem = None;
    d.messages[0].elem = None;
    d.pages[0].v.frame = "app".into();
    d.pages[0].v.blocks.clear();
    d.pages[0].v.places = places;
    d
}
fn good() -> Vec<Item<PlaceNode>> {
    vec![
        blk("main", "board"),
        frame(
            "side",
            "split",
            vec![blk("pane", "panel"), blk("pane", "panel")],
        ),
    ]
}

#[test]
fn a_declared_tree_passes() {
    // side takes `panel` blocks only: a split frame there is wrong, so the good tree puts panels directly
    let ok = page(vec![
        blk("main", "board"),
        blk("side", "panel"),
        blk("side", "panel"),
    ]);
    assert_eq!(codes(run(ok)), []);
    // the open catalog declares nothing: nothing is checked
    assert!(
        page(good())
            .check(&OpenCatalog, &Limits::declared())
            .is_ok()
    );
}

#[test]
fn k4_a_place_the_frame_does_not_declare() {
    let d = page(vec![
        blk("main", "board"),
        blk("side", "panel"),
        blk("nowhere", "board"),
    ]);
    assert_eq!(codes(run(d)), [LayoutCode::PlaceUnknown]);
}

#[test]
fn k6_takes_is_checked() {
    let d = page(vec![blk("main", "board"), blk("side", "board")]);
    assert_eq!(codes(run(d)), [LayoutCode::PlaceTakes]);
}

#[test]
fn k6_the_number_of_children_min_and_max() {
    let few = page(vec![blk("main", "board")]); // side needs 1
    assert_eq!(codes(run(few)), [LayoutCode::PlaceCount]);
    let many = page(vec![
        blk("main", "board"),
        blk("side", "panel"),
        blk("side", "panel"),
        blk("side", "panel"),
    ]); // side takes at most 2
    assert_eq!(codes(run(many)), [LayoutCode::PlaceCount]);
    // inside a frame: a split with one pane is too few
    let d = page(vec![
        blk("main", "board"),
        frame("side", "split", vec![blk("pane", "panel")]),
    ]);
    let c = codes(run(d));
    assert!(c.contains(&LayoutCode::PlaceCount), "{c:?}");
}

#[test]
fn k8_one_id_names_one_element_in_every_depth() {
    let deep = frame(
        "main",
        "split",
        vec![frame(
            "pane",
            "split",
            vec![blk("pane", "panel"), blk("pane", "panel")],
        )],
    );
    let l = page(vec![deep])
        .check(&OpenCatalog, &Limits::declared())
        .unwrap();
    let mut d = l.to_doc();
    d.format = 2;
    assert!(d.clone().check(&OpenCatalog, &Limits::declared()).is_ok());
    // two blocks on depth 3 with the same id
    let PlaceNode::Frame(a) = &mut d.pages[0].v.places[0].v else {
        panic!()
    };
    let PlaceNode::Frame(b) = &mut a.children[0].v else {
        panic!()
    };
    b.children[1].elem = b.children[0].elem.clone();
    assert!(
        codes(d.clone().check(&OpenCatalog, &Limits::declared())).contains(&LayoutCode::ElemTwice)
    );
    // a block and a frame with one id on a page (different kinds)
    let PlaceNode::Frame(a) = &mut d.pages[0].v.places[0].v else {
        panic!()
    };
    let PlaceNode::Frame(b) = &mut a.children[0].v else {
        panic!()
    };
    b.children[1].elem = Some(elem("block:board/main/pane/panel.1"));
    a.children[0].elem = b.children[1].elem.clone();
    assert!(codes(d.check(&OpenCatalog, &Limits::declared())).contains(&LayoutCode::ElemTwice));
}

#[test]
fn k20_a_set_key_the_catalog_does_not_list_is_yellow() {
    let mut p = frame(
        "side",
        "split",
        vec![blk("pane", "panel"), blk("pane", "panel")],
    );
    if let PlaceNode::Frame(f) = &mut p.v {
        f.set = vec![attr("axis"), attr("colour")];
        if let PlaceNode::Block(b) = &mut f.children[0].v {
            b.set = vec![attr("opacity"), attr("glow")];
        }
    }
    // checking passes (never blocking); the warnings say which keys are ignored
    let l = page(vec![blk("main", "board"), p])
        .check(&OpenCatalog, &Limits::declared())
        .unwrap();
    let w = l.warnings(&Cat);
    let names: Vec<&str> = w.iter().filter_map(|c| c.param("name")).collect();
    assert_eq!(names, ["colour", "glow"]);
    assert!(w.iter().all(|c| c.code == LayoutCode::SetUnknown));
    assert_eq!(l.warnings(&OpenCatalog), vec![]);
}

#[test]
fn the_depth_limit_by_default_and_one_more_is_refused() {
    let nest = |n: usize| {
        let mut node = blk("pane", "panel");
        for _ in 1..n {
            node = frame("pane", "x", vec![node]);
        }
        node
    };
    let lim = Limits {
        place_depth_max: 3,
        ..Limits::declared()
    };
    let ok = page(vec![nest(3)]).check(&OpenCatalog, &lim);
    assert!(ok.is_ok());
    let err = page(vec![nest(4)]).check(&OpenCatalog, &lim).unwrap_err();
    assert_eq!(err[0].param("what"), Some("place-depth"));
    assert_eq!(err[0].param("max"), Some("3"));
    assert_eq!(err[0].param("by"), Some("default"));
    // the ceiling clamps
    let ceil = CeilingSet {
        place_depth_max: Some(2),
        ..CeilingSet::default()
    };
    let eff = Limits::effective(&lim, &ceil);
    assert_eq!(
        (eff.place_depth_max, eff.by.place_depth),
        (2, LimitSource::Ceiling)
    );
}
