//! S6-3: reading, checking and writing the frontend node (`frontend = { extends, frontend }`, `mode`, `shown`) and the action
//! `open`. Gate counter-tests: a node without `elem`, or with an unknown `mode`, must be red.
use std::path::Path;

use fs_forms_exchange::{Format, read_layout, write_layout};
use fs_forms_layout::{
    ActionTarget, Coded, Layout, LayoutCode, LayoutDoc, Limits, OpenCatalog, PlaceNode, Placement,
    Shown,
};

const HEAD: &str = "schema = 3\nkind = \"frontend\"\nid = \"board\"\nextends = \"net.example.Tasks\"\nexec = \"fs-forms-run\"\ntitle = \"t\"\n\n[[page]]\nelem = \"page:board\"\npage = \"board\"\nframe = \"app-window\"\nnavigation = \"single\"\n";
const TAIL: &str =
    "\n[[message]]\nelem = \"message:t/de\"\nkey = \"t\"\nlang = \"de\"\ntext = \"T\"\n";
const NODE: &str = "\n[[page.place]]\nelem = \"settings-embed\"\nat = \"panel\"\nfrontend = { extends = \"net.freesynergy.Settings\", frontend = \"settings\" }\nmode = \"panel\"\nshown = \"start\"\n";

fn file(places: &str) -> String {
    format!("{HEAD}{places}{TAIL}")
}
fn read(text: &str) -> Result<LayoutDoc, Vec<Coded<LayoutCode>>> {
    read_layout(&mut text.as_bytes(), Path::new("t.toml"), Format::Toml)
}
fn layout(text: &str) -> Layout {
    read(text)
        .expect("read")
        .check(&OpenCatalog, &Limits::declared())
        .expect("check")
}
fn causes(text: &str) -> Vec<String> {
    read(text)
        .expect_err("must be red")
        .iter()
        .map(|c| c.param("cause").unwrap_or("-").to_owned())
        .collect()
}

#[test]
fn the_node_is_read_with_every_key() {
    let l = layout(&file(NODE));
    let PlaceNode::Frontend(e) = &l.doc().pages[0].v.places[0].v else {
        panic!("a frontend node");
    };
    assert_eq!(
        (
            e.place.as_str(),
            e.key.extends.as_str(),
            e.key.frontend.as_str()
        ),
        ("panel", "net.freesynergy.Settings", "settings")
    );
    assert_eq!((e.mode, e.shown), (Placement::Panel, Shown::Start));
    assert_eq!(
        l.doc().pages[0].v.places[0]
            .elem
            .as_ref()
            .map(fs_forms_layout::ElemId::as_str),
        Some("settings-embed")
    );
}

#[test]
fn mode_window_and_shown_on_open_and_the_defaults() {
    let t = NODE
        .replace("mode = \"panel\"", "mode = \"window\"")
        .replace("\"start\"", "\"on-open\"");
    let l = layout(&file(&t));
    let PlaceNode::Frontend(e) = &l.doc().pages[0].v.places[0].v else {
        panic!("a frontend node");
    };
    assert_eq!((e.mode, e.shown), (Placement::Window, Shown::OnOpen));
    let bare = "\n[[page.place]]\nelem = \"e\"\nat = \"panel\"\nfrontend = { extends = \"a.B\", frontend = \"c\" }\n";
    let l = layout(&file(bare));
    let PlaceNode::Frontend(e) = &l.doc().pages[0].v.places[0].v else {
        panic!("a frontend node");
    };
    assert_eq!((e.mode, e.shown), (Placement::Panel, Shown::Start));
}

#[test]
fn gate_a_frontend_node_without_elem_is_red() {
    let t = NODE.replace("elem = \"settings-embed\"\n", "");
    assert_eq!(causes(&file(&t)), ["key-missing"]);
}

#[test]
fn gate_an_unknown_mode_is_red() {
    let t = NODE.replace("mode = \"panel\"", "mode = \"tab\"");
    assert_eq!(causes(&file(&t)), ["mode-unknown"]);
    let t = NODE.replace("\"start\"", "\"later\"");
    assert_eq!(causes(&file(&t)), ["shown-unknown"]);
}

#[test]
fn a_broken_key_or_a_stray_key_is_red() {
    let t = NODE.replace(", frontend = \"settings\"", "");
    assert_eq!(causes(&file(&t)), ["key-missing"]);
    let t = NODE.replace("mode = ", "set = { a = \"b\" }\nmode = ");
    assert_eq!(causes(&file(&t)), ["key-unknown"]);
    let t = NODE.replace("mode = ", "block = \"x\"\nmode = ");
    assert_eq!(causes(&file(&t)), ["key-unknown"]);
}

#[test]
fn write_and_read_again_gives_the_same_layout() {
    let t = NODE.replace("mode = \"panel\"", "mode = \"window\"")
        + "scope-from = \"settings-embed2\"\nuser = [\"window\"]\n[[page.place]]\nelem = \"settings-embed2\"\nat = \"main\"\nfrontend = { extends = \"a.B\", frontend = \"c\" }\n";
    let l = layout(&file(&t));
    let mut out = Vec::new();
    write_layout(&l, Format::Toml, true, &mut out).expect("write");
    let text = String::from_utf8(out).expect("utf8");
    assert!(text.contains("mode = \"window\""), "{text}");
    let again = layout(&text);
    let mut out2 = Vec::new();
    write_layout(&again, Format::Toml, true, &mut out2).expect("write");
    assert_eq!(String::from_utf8(out2).expect("utf8"), text); // lines differ by nature, the text does not
}

#[test]
fn the_action_open_is_read_checked_and_written() {
    let act =
        "\n[[action]]\nelem = \"action:1\"\ntrigger = \"button\"\nopen = \"settings-embed\"\n";
    let text = format!("{HEAD}{NODE}{act}{TAIL}");
    let l = layout(&text);
    assert!(
        matches!(&l.doc().actions[0].v.target, ActionTarget::Open { elem } if elem.as_str() == "settings-embed")
    );
    let mut out = Vec::new();
    write_layout(&l, Format::Toml, true, &mut out).expect("write");
    assert!(
        String::from_utf8(out)
            .expect("utf8")
            .contains("open = \"settings-embed\"")
    );
    // a target that is no frontend node is red at the check
    let text = format!(
        "{HEAD}{NODE}{}{TAIL}",
        act.replace("\"settings-embed\"", "\"nowhere\"")
    );
    let e = read(&text)
        .expect("read")
        .check(&OpenCatalog, &Limits::declared())
        .expect_err("red");
    assert_eq!(e[0].code, LayoutCode::ElemUnknown);
    // two targets on one action are red at the read
    let text = format!(
        "{HEAD}{NODE}{}{TAIL}",
        act.replace("trigger", "workflow = \"w\"\ntrigger")
    );
    assert_eq!(causes(&text), ["key-twice"]);
}
