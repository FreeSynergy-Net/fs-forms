//! The place of an action (`at`): read from the file, written back and kept in the round trip; an `at` that is no element id is an
//! error of the file, never dropped.
//!
//! Pattern: none (tests).

use std::path::Path;

use fs_forms_exchange::{Format, read_layout, write_layout};
use fs_forms_layout::{LayoutCode, Limits, OpenCatalog, Trigger};

fn file(at: &str) -> String {
    format!(
        r#"schema = 3
kind = "frontend"
id = "board"
extends = "net.example.Tasks"
exec = "fs-forms-run"
title = "board-title"

[[source]]
elem = "source:1"
n = 1
contract = "tasks@>=0.0.1, <0.1.0"
type = "Task"

[[page]]
elem = "page:board"
page = "board"
frame = "workbench"

[[page.place]]
elem = "block:board/main"
at = "main"
block = "board"
source = 1

[[action]]
elem = "action:1"
{at}
trigger = "drop"
source = 1
command = "card.move"

[[message]]
elem = "message:board-title/de"
key = "board-title"
lang = "de"
text = "Aufgaben"
"#
    )
}

fn read(text: &str) -> Result<fs_forms_layout::LayoutDoc, Vec<LayoutCode>> {
    read_layout(&mut text.as_bytes(), Path::new("t"), Format::Toml)
        .map_err(|e| e.iter().map(|c| c.code).collect())
}

#[test]
fn the_place_of_an_action_is_read_written_and_kept_in_the_round_trip() {
    let doc = read(&file("at = \"block:board/main\"")).expect("read");
    assert_eq!(doc.actions[0].v.trigger, Trigger::Drop);
    assert_eq!(
        doc.actions[0]
            .v
            .at
            .as_ref()
            .map(fs_forms_layout::ElemId::as_str),
        Some("block:board/main")
    );
    let layout = doc.check(&OpenCatalog, &Limits::declared()).expect("check");
    let mut first = Vec::new();
    write_layout(&layout, Format::Toml, true, &mut first).expect("write");
    let text = String::from_utf8(first.clone()).expect("utf8");
    assert!(text.contains("at = \"block:board/main\""), "{text}");
    let again = read(&text).expect("read again");
    let layout2 = again
        .check(&OpenCatalog, &Limits::declared())
        .expect("check");
    let mut second = Vec::new();
    write_layout(&layout2, Format::Toml, true, &mut second).expect("write");
    assert_eq!(first, second);
}

#[test]
fn an_action_without_a_place_writes_no_at() {
    let doc = read(&file("")).expect("read");
    assert!(doc.actions[0].v.at.is_none());
}

#[test]
fn an_at_that_is_no_element_id_is_an_error_of_the_file() {
    assert!(read(&file("at = \"not an id\"")).is_err());
}
