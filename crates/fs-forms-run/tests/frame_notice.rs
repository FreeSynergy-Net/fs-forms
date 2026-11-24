//! A5 F6: the host draws ONE level (the picture stays the same); a frame deeper than that is said, never skipped.
//! Counter-proofs: the same layout without the frame has no notice; the notice has words in both languages.
use std::path::Path;

use fs_forms_exchange::{Format, read_layout};
use fs_forms_layout::{Layout, Limits, OpenCatalog};
use fs_forms_run::{HostState, Node, TestCatalog, compose, view_of, words};

const HEAD: &str = "schema = 3\nkind = \"frontend\"\nid = \"board\"\nextends = \"net.example.Tasks\"\nexec = \"fs-forms-run\"\ntitle = \"t\"\n\n[[page]]\nelem = \"page:board\"\npage = \"board\"\nframe = \"workbench\"\nnavigation = \"single\"\n";
const TAIL: &str =
    "\n[[message]]\nelem = \"message:t/de\"\nkey = \"t\"\nlang = \"de\"\ntext = \"T\"\n";

const BLOCK: &str =
    "\n[[page.place]]\nelem = \"block:board/main\"\nat = \"main\"\nblock = \"board\"\n";
const FRAME: &str = "\n[[page.place]]\nelem = \"frame:board/side\"\nat = \"side\"\nframe = \"split\"\nset = { axis = \"row\" }\n\n[[page.place.place]]\nelem = \"block:board/side/one\"\nat = \"pane\"\nblock = \"table\"\n";

fn layout(places: &str) -> Layout {
    let text = format!("{HEAD}{places}{TAIL}");
    read_layout(&mut text.as_bytes(), Path::new("t.toml"), Format::Toml)
        .expect("read")
        .check(&OpenCatalog, &Limits::declared())
        .expect("check")
}

fn catalog() -> TestCatalog {
    TestCatalog::building(&["workbench", "board", "table", "partial-notice"])
}

#[test]
fn a_frame_at_depth_one_is_a_partial_notice_with_its_name() {
    let c = compose(
        &view_of(&layout(&format!("{BLOCK}{FRAME}")), "en"),
        &catalog(),
    );
    assert_eq!(c.state, HostState::Partial);
    let nodes = &c.screens[0].nodes;
    let said = nodes.iter().any(|n| {
        matches!(n, Node::Locked { place: Some(p), codes }
            if p == "side" && codes[0].code == "host-notice-frame-not-built"
                && codes[0].params == [("frame".to_owned(), "split".to_owned())])
    });
    assert!(said, "{nodes:?}");
    // the block at depth one is drawn as before; the block INSIDE the frame is not drawn (the frame says so)
    assert!(
        nodes
            .iter()
            .any(|n| matches!(n, Node::Block { place, .. } if place == "main"))
    );
    assert!(
        !nodes
            .iter()
            .any(|n| matches!(n, Node::Block { block, .. } if block == "table"))
    );
}

#[test]
fn counter_proof_without_the_frame_nothing_is_said_and_the_page_is_ready() {
    let c = compose(&view_of(&layout(BLOCK), "en"), &catalog());
    assert!(
        !c.screens[0]
            .nodes
            .iter()
            .any(|n| matches!(n, Node::Locked { .. })),
        "{:?}",
        c.screens[0].nodes
    );
    assert_eq!(c.screens[0].nodes.len(), 1);
}

#[test]
fn the_notice_has_words_in_both_languages() {
    let en = words(&fs_components_words::Locale::english()).plain("host-notice-frame-not-built");
    let de = words(&fs_components_words::Locale::parse("de").unwrap())
        .plain("host-notice-frame-not-built");
    assert!(!en.is_empty() && !de.is_empty() && en != de, "{en} / {de}");
}
