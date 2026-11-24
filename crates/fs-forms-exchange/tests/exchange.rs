//! K51 (round trip byte-equal), K63/K64 (problems with codes), K66 (scheme order kept), K70 (JSON), K71 (CRLF), K72 (own views).
use std::path::Path;

use fs_forms_exchange::{Format, read_layout, write_layout};
use fs_forms_layout::{Layout, LayoutCode, Limits, OpenCatalog};

const FILE: &str = r#"schema = 3
kind = "frontend"
id = "board"
extends = "net.example.Tasks"
exec = "fs-forms-run"
title = "board-title"

[[source]]
elem = "source:1"
n = 1
contract = "tasks@>=0.2, <0.3"
type = "Task"
scope = "selected-list"

[[menu]]
elem = "menu:1"
kind = "menu"
label = "menu-task"

[[menu.entries]]
elem = "menu:1.0"
kind = "command"
source = 1
command = "create"
key = "ctrl+n"

[[page]]
elem = "page:board"
page = "board"
frame = "workbench"
navigation = "list-detail"

[[page.place]]
elem = "block:board/main"
at = "main"
block = "board"
source = 1

[[page.place.bind]]
elem = "binding:board/main/link/0"
slot = "value"
nth = 0
field = "homepage"
role = "input"

[[page.place.bind.schemes]]
elem = "scheme:board/main/link/0/0"
scheme = "ssh"

[[page.place.bind.schemes]]
elem = "scheme:board/main/link/0/1"
scheme = "https"

[[view]]
elem = "view:mine"
view = "mine"
owner = "https://example.org/people/test"

[[message]]
elem = "message:board-title/de"
key = "board-title"
lang = "de"
text = "Aufgaben"
"#;

fn read(
    text: &str,
    f: Format,
) -> Result<fs_forms_layout::LayoutDoc, Vec<fs_forms_layout::Coded<LayoutCode>>> {
    read_layout(&mut text.as_bytes(), Path::new("t.toml"), f)
}
fn layout(text: &str) -> Layout {
    read(text, Format::Toml)
        .expect("read")
        .check(&OpenCatalog, &Limits::declared())
        .expect("check")
}
fn export(l: &Layout, f: Format, own: bool) -> String {
    let mut out = Vec::new();
    write_layout(l, f, own, &mut out).expect("write");
    String::from_utf8(out).expect("utf8")
}
fn codes(text: &str) -> Vec<LayoutCode> {
    read(text, Format::Toml)
        .expect_err("must fail")
        .iter()
        .map(|c| c.code)
        .collect()
}

#[test]
fn k51_round_trip_is_byte_equal_from_the_first_export() {
    let first = export(&layout(FILE), Format::Toml, true);
    let second = export(&layout(&first), Format::Toml, true);
    assert_eq!(first, second);
    assert!(first.ends_with("\"\n") && !first.contains('\r') && !first.contains('#'));
    assert!(
        first.find("\"ssh\"").unwrap() < first.find("\"https\"").unwrap(),
        "scheme order is the suggestion order"
    );
}

#[test]
fn k72_own_views_only_on_request() {
    let l = layout(FILE);
    assert!(!export(&l, Format::Toml, false).contains("view:mine"));
    assert!(export(&l, Format::Toml, true).contains("view:mine"));
}

#[test]
fn k71_crlf_is_read_and_export_is_lf() {
    let crlf = FILE.replace('\n', "\r\n");
    assert_eq!(
        export(&layout(&crlf), Format::Toml, true),
        export(&layout(FILE), Format::Toml, true)
    );
}

#[test]
fn k64_problems_have_codes_and_lines() {
    let e = read(&FILE.replace("exec = ", "wat = 1\nexec = "), Format::Toml).unwrap_err();
    assert_eq!(
        (e[0].code, e[0].param("cause"), e[0].param("line")),
        (LayoutCode::ImportInvalid, Some("key-unknown"), Some("5"))
    );
    let e = read(&FILE.replace("nth = 0", "nth = \"zero\""), Format::Toml).unwrap_err();
    assert_eq!(e[0].param("cause"), Some("type"));
    let e = read(&format!("\u{feff}{FILE}"), Format::Toml).unwrap_err();
    assert_eq!(e[0].param("cause"), Some("utf8"));
    let e = read("schema = [", Format::Toml).unwrap_err();
    assert_eq!(e[0].param("cause"), Some("toml"));
    let three = FILE.replace("exec = ", "a = 1\nb = 2\nc = 3\nexec = ");
    assert_eq!(codes(&three).len(), 3);
}

#[test]
fn missing_key_and_wrong_format_are_visible() {
    assert!(codes(&FILE.replace("id = \"board\"\n", "")).contains(&LayoutCode::ImportInvalid));
    let d = read(&FILE.replace("schema = 3", "schema = 4"), Format::Toml).unwrap();
    let e = d.check(&OpenCatalog, &Limits::declared()).unwrap_err();
    assert_eq!(e[0].code, LayoutCode::FormatNewer);
}

#[test]
fn format_one_and_two_are_refused_as_old() {
    for old in ["schema = 1", "schema = 2"] {
        let e = read(&FILE.replace("schema = 3", old), Format::Toml).unwrap_err();
        assert_eq!(e.len(), 1, "only the format problem is reported");
        assert_eq!(e[0].code, LayoutCode::FormatOld);
        assert_eq!(
            (e[0].param("found"), e[0].param("known")),
            (Some(&old[9..]), Some("3"))
        );
    }
}

#[cfg(feature = "json")]
#[test]
fn k70_json_is_canonical_and_round_trips() {
    let l = layout(FILE);
    let j = export(&l, Format::Json, true);
    assert!(
        j.starts_with("{\"action\":[]")
            || j.starts_with("{\"actions\"")
            || j.starts_with("{\"action"),
        "{j}"
    );
    let back = read_layout(&mut j.as_bytes(), Path::new("t.json"), Format::Json).unwrap();
    let again = back.check(&OpenCatalog, &Limits::declared()).unwrap();
    assert_eq!(export(&again, Format::Json, true), j);
}

#[test]
fn elem_missing_names_the_line_of_its_table() {
    let text = FILE.replace("elem = \"page:board\"\n", "");
    let page_line = 1 + text.lines().position(|l| l == "[[page]]").unwrap();
    let doc = read(&text, Format::Toml).unwrap();
    let e = doc.check(&OpenCatalog, &Limits::declared()).unwrap_err();
    assert_eq!(e[0].code, LayoutCode::ElemMissing);
    assert_eq!(e[0].param("line"), Some(page_line.to_string().as_str()));
}
