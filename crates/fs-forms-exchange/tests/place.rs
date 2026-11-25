//! A5 F3: the tree of places in the file (format 3). Reading, writing, round trips, and the counter-tests: each one must go red
//! when the rule is broken (format 2 file, block and frame on one place, unknown key, loss in the round trip).
use std::fmt::Write;
use std::path::Path;

use fs_forms_exchange::{Format, read_layout, write_layout};
use fs_forms_layout::{Coded, Layout, LayoutCode, LayoutDoc, Limits, OpenCatalog, PlaceNode};

const HEAD: &str = "schema = 3\nkind = \"frontend\"\nid = \"board\"\nextends = \"net.example.Tasks\"\nexec = \"fs-forms-run\"\ntitle = \"t\"\n\n[[page]]\nelem = \"page:board\"\npage = \"board\"\nframe = \"app-window\"\nnavigation = \"single\"\n";
const TAIL: &str =
    "\n[[message]]\nelem = \"message:t/de\"\nkey = \"t\"\nlang = \"de\"\ntext = \"T\"\n";

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
fn export(l: &Layout) -> String {
    let mut out = Vec::new();
    write_layout(l, Format::Toml, true, &mut out).expect("write");
    String::from_utf8(out).expect("utf8")
}
fn cause(text: &str) -> Vec<String> {
    read(text)
        .expect_err("must fail")
        .iter()
        .map(|c| c.param("cause").unwrap_or("-").to_owned())
        .collect()
}

const BLOCK: &str =
    "\n[[page.place]]\nelem = \"block:board/main\"\nat = \"main\"\nblock = \"board\"\n";
const TREE: &str = r#"
[[page.place]]
elem = "block:board/nav"
at = "nav"
block = "sidebar"
set = { opacity = "0.6" }

[[page.place]]
elem = "frame:board/main"
at = "main"
frame = "workbench"
set = { axis = "row", sizes = "0.3 0.7" }

[[page.place.place]]
elem = "block:board/main/side"
at = "side"
block = "contact-row"
title = "t"
set = { "min-width" = "16em", "a b" = "x\"y" }

[[page.place.place]]
elem = "frame:board/main/panel"
at = "panel"
frame = "split"

[[page.place.place.place]]
elem = "block:board/main/panel/one"
at = "pane"
block = "field-card"

[[page.place.place.place]]
elem = "block:board/main/panel/two"
at = "pane"
block = "field-card"
"#;

fn pages_tree(l: &Layout) -> Vec<fs_forms_layout::Item<PlaceNode>> {
    l.doc().pages[0].v.places.clone()
}

/// Reference text: the canonical export of `TREE` is fixed, so a writer that reorders keys or drops a table cannot pass.
#[test]
fn the_export_of_a_tree_is_the_fixed_reference_text() {
    let out = export(&layout(&file(TREE)));
    // the writer puts the scalars of a table first and the lists after: same order as the input here, `set` last of the scalars
    assert!(out.starts_with("schema = 4\n"), "{out}");
    assert!(out.contains("[[page.place.place.place]]\nelem = \"block:board/main/panel/one\"\nat = \"pane\"\nblock = \"field-card\"\n"), "{out}");
    assert!(
        out.contains("set = { axis = \"row\", sizes = \"0.3 0.7\" }\n"),
        "{out}"
    );
    assert!(
        out.contains("set = { \"a b\" = \"x\\\"y\", min-width = \"16em\" }\n"),
        "{out}"
    );
    assert_eq!(export(&layout(&out)), out, "second export is byte-equal");
}

/// Round trip over a table of example trees: TOML to doc to TOML to doc; nothing is lost, the text is stable.
#[test]
fn round_trip_over_example_trees() {
    let kids = |n: usize| -> String {
        (0..n).fold(String::new(), |mut s, i| {
            let _ = write!(s, "\n[[page.place.place]]\nelem = \"block:board/f/k{i}\"\nat = \"k\"\nblock = \"b\"\nset = {{ order = \"{i}\" }}\n");
            s
        })
    };
    let frame = "\n[[page.place]]\nelem = \"frame:board/f\"\nat = \"f\"\nframe = \"tabs\"\nset = { active = \"k1\" }\n";
    let cases = [
        String::new(),
        BLOCK.to_owned(),
        TREE.to_owned(),
        format!("{frame}{}", kids(3)),
        format!("{frame}{}{BLOCK}", kids(1)),
        "\n[[page.place]]\nelem = \"frame:board/e\"\nat = \"e\"\nframe = \"split\"\n".to_owned(),
    ];
    for (n, c) in cases.iter().enumerate() {
        let first = layout(&file(c));
        let text = export(&first);
        let second = layout(&text);
        assert_eq!(
            pages_tree(&first),
            pages_tree(&second),
            "case {n}: tree lost"
        );
        assert_eq!(export(&second), text, "case {n}: text not stable");
    }
}

#[test]
fn the_tree_is_read_with_children_and_attributes() {
    let l = layout(&file(TREE));
    let top = pages_tree(&l);
    assert_eq!(top.len(), 2);
    let PlaceNode::Frame(main) = &top[1].v else {
        panic!("frame expected")
    };
    assert_eq!(main.children.len(), 2);
    assert_eq!(main.set.len(), 2);
    assert_eq!(main.set[1].v.name, "sizes");
    let PlaceNode::Frame(panel) = &main.children[1].v else {
        panic!("frame expected")
    };
    assert_eq!(
        panel.children.len(),
        2,
        "two children on one place are the list"
    );
    assert!(
        l.doc().pages[0].v.blocks.is_empty(),
        "no flat list any more"
    );
    // the attribute gets its id from the path
    assert!(
        main.set[0]
            .elem
            .as_ref()
            .is_some_and(|e| e.as_str() == "attr:board/main/axis")
    );
}

// ---- counter-tests: each must be red

#[test]
fn a_format_two_file_is_refused_with_format_old() {
    let old = "schema = 2\nkind = \"frontend\"\nid = \"b\"\nextends = \"x.Y\"\nexec = \"e\"\ntitle = \"t\"\n\n[[page]]\nelem = \"page:b\"\npage = \"b\"\nframe = \"workbench\"\n\n[[page.block]]\nelem = \"block:b/main\"\nplace = \"main\"\nblock = \"b\"\n";
    let e = read(old).unwrap_err();
    assert_eq!(e.len(), 1);
    assert_eq!(e[0].code, LayoutCode::FormatOld);
    // a format 3 file that still uses `[[page.block]]` is a closed-schema error, not a second reading path
    let mixed = file("\n[[page.block]]\nelem = \"block:board/x\"\nplace = \"x\"\nblock = \"b\"\n");
    assert_eq!(cause(&mixed), ["key-unknown"]);
}

#[test]
fn a_place_with_block_and_frame_is_refused() {
    let both = "\n[[page.place]]\nelem = \"block:board/x\"\nat = \"x\"\nblock = \"b\"\nframe = \"split\"\n";
    assert_eq!(cause(&file(both)), ["place-block-and-frame"]);
    let neither = "\n[[page.place]]\nelem = \"block:board/x\"\nat = \"x\"\n";
    assert_eq!(cause(&file(neither)), ["key-missing"]);
}

#[test]
fn an_unknown_key_in_a_place_is_refused() {
    let unknown =
        "\n[[page.place]]\nelem = \"block:board/x\"\nat = \"x\"\nblock = \"b\"\nwat = \"1\"\n";
    assert_eq!(cause(&file(unknown)), ["key-unknown"]);
    // block keys on a frame are unknown too, and a block has no children
    let on_frame =
        "\n[[page.place]]\nelem = \"frame:board/x\"\nat = \"x\"\nframe = \"split\"\nsource = 1\n";
    assert_eq!(cause(&file(on_frame)), ["key-unknown"]);
    let kids = "\n[[page.place]]\nelem = \"block:board/x\"\nat = \"x\"\nblock = \"b\"\n\n[[page.place.place]]\nelem = \"block:board/y\"\nat = \"y\"\nblock = \"b\"\n";
    assert_eq!(cause(&file(kids)), ["key-unknown"]);
    // an attribute value must be text
    let num = "\n[[page.place]]\nelem = \"block:board/x\"\nat = \"x\"\nblock = \"b\"\nset = { opacity = 1 }\n";
    assert_eq!(cause(&file(num)), ["type"]);
}

#[test]
fn a_round_trip_that_loses_a_node_is_seen() {
    let text = export(&layout(&file(TREE)));
    // drop one child from the exported text: the trees must differ (the comparison of the round trip has teeth)
    let cut = text.replace("[[page.place.place.place]]\nelem = \"block:board/main/panel/two\"\nat = \"pane\"\nblock = \"field-card\"\n", "");
    assert_ne!(cut, text);
    assert_ne!(pages_tree(&layout(&cut)), pages_tree(&layout(&text)));
    let no_attr = text.replace("set = { opacity = \"0.6\" }\n", "");
    assert_ne!(pages_tree(&layout(&no_attr)), pages_tree(&layout(&text)));
}

#[cfg(feature = "json")]
#[test]
fn the_tree_round_trips_through_json() {
    let l = layout(&file(TREE));
    let mut out = Vec::new();
    write_layout(&l, Format::Json, true, &mut out).unwrap();
    let back = read_layout(&mut out.as_slice(), Path::new("t.json"), Format::Json).unwrap();
    let again = back.check(&OpenCatalog, &Limits::declared()).unwrap();
    assert_eq!(pages_tree(&again), pages_tree(&l));
}
