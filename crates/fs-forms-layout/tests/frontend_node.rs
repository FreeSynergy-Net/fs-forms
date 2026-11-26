//! S6-3: the layout node `PlaceNode::Frontend` and the action `open`: what passes, and one counter-test per rule (each must go
//! red when its rule is removed from `check_frontend.rs`).

mod support;
use fs_forms_layout::*;
use support::*;

fn eid(s: &str) -> ElemId {
    ElemId::parse(s).expect("id")
}
fn embed(place: &str, extends: &str, name: &str) -> EmbedDoc {
    EmbedDoc {
        place: place.into(),
        key: FrontendKey {
            extends: extends.into(),
            frontend: name.into(),
        },
        mode: Placement::Panel,
        shown: Shown::Start,
        scope_from: None,
        user: vec![],
    }
}
fn node(id: Option<&str>, e: EmbedDoc) -> Item<PlaceNode> {
    Item {
        elem: id.map(eid),
        line: 0,
        v: PlaceNode::Frontend(e),
    }
}
fn page(places: Vec<Item<PlaceNode>>) -> LayoutDoc {
    let mut d = doc("board");
    d.format = 1;
    d.pages[0].elem = None;
    d.messages[0].elem = None;
    d.pages[0].v.blocks.clear();
    d.pages[0].v.places = places;
    d
}
fn good() -> Item<PlaceNode> {
    node(
        Some("settings-embed"),
        embed("panel", "net.freesynergy.Settings", "settings"),
    )
}
fn open(id: &str) -> Item<ActionDoc> {
    bare(ActionDoc {
        trigger: Trigger::Button,
        at: None,
        target: ActionTarget::Open { elem: eid(id) },
        confirm: None,
    })
}
fn codes(d: LayoutDoc) -> Vec<(LayoutCode, String)> {
    d.check(&OpenCatalog, &Limits::declared())
        .err()
        .unwrap_or_default()
        .iter()
        .map(|c| (c.code, c.param("cause").unwrap_or("").to_owned()))
        .collect()
}

#[test]
fn a_frontend_node_with_an_id_passes_and_is_a_leaf() {
    let d = page(vec![good()]);
    let l = d.check(&OpenCatalog, &Limits::declared()).expect("check");
    assert_eq!(l.doc().pages[0].v.places.len(), 1);
    assert_eq!(l.doc().pages[0].v.places[0].v.place(), "panel");
    assert_eq!(
        l.doc().pages[0].v.leaves().count(),
        0,
        "a frontend is no block"
    );
}

#[test]
fn without_elem_it_is_red() {
    let d = page(vec![node(
        None,
        embed("panel", "net.freesynergy.Settings", "settings"),
    )]);
    assert_eq!(codes(d), [(LayoutCode::ElemMissing, String::new())]);
}

#[test]
fn an_empty_key_or_unknown_flag_or_unknown_scope_from_is_red() {
    let mut e = embed("panel", "", "");
    e.user = vec!["fly".into()];
    e.scope_from = Some(eid("nowhere"));
    let got: Vec<String> = codes(page(vec![node(Some("x"), e)]))
        .into_iter()
        .map(|(c, cause)| {
            assert_eq!(c, LayoutCode::FrontendInvalid);
            cause
        })
        .collect();
    assert_eq!(
        got,
        [
            "extends-empty",
            "frontend-empty",
            "user-unknown",
            "scope-from-unknown"
        ]
    );
}

#[test]
fn scope_from_may_name_a_neighbour_and_the_user_flag_window_is_known() {
    let mut e = embed("panel", "net.freesynergy.Settings", "settings");
    e.scope_from = Some(eid("list"));
    e.user = vec!["window".into()];
    let list = frame_node("list");
    assert_eq!(codes(page(vec![list, node(Some("x"), e)])), []);
}

fn frame_node(id: &str) -> Item<PlaceNode> {
    Item {
        elem: Some(eid(id)),
        line: 0,
        v: PlaceNode::Frame(FrameDoc {
            place: "main".into(),
            frame: "workbench".into(),
            set: vec![],
            children: vec![],
        }),
    }
}

#[test]
fn the_same_id_twice_is_red_even_across_kinds() {
    let two = page(vec![good(), good()]);
    assert_eq!(codes(two).len(), 2);
    let clash = page(vec![frame_node("settings-embed"), good()]);
    assert!(
        codes(clash)
            .iter()
            .any(|(c, _)| *c == LayoutCode::ElemTwice)
    );
}

#[test]
fn an_action_open_must_name_a_frontend_node() {
    let mut d = page(vec![good()]);
    d.actions.push(open("settings-embed"));
    assert_eq!(codes(d), []);
    let mut d = page(vec![good(), frame_node("f")]);
    d.actions.push(open("f")); // a frame is no frontend
    assert_eq!(codes(d), [(LayoutCode::ElemUnknown, String::new())]);
    let mut d = page(vec![good()]);
    d.actions.push(open("nowhere"));
    assert_eq!(codes(d), [(LayoutCode::ElemUnknown, String::new())]);
}

#[test]
fn the_words_of_mode_and_shown_round_trip() {
    for (w, p) in [("panel", Placement::Panel), ("window", Placement::Window)] {
        assert_eq!(Placement::from_word(w), Some(p));
        assert_eq!(p.word(), w);
    }
    for (w, s) in [("start", Shown::Start), ("on-open", Shown::OnOpen)] {
        assert_eq!(Shown::from_word(w), Some(s));
        assert_eq!(s.word(), w);
    }
    assert_eq!(
        Placement::from_word("tab"),
        None,
        "a tab is a panel in a frame"
    );
}
