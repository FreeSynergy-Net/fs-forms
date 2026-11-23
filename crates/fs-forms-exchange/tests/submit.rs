//! SW-3: the trigger `submit` (the fields of a form sent to a command channel) is read, written and kept in the round trip; a
//! trigger that is in no list stays an error.
use std::path::Path;

use fs_forms_exchange::{Format, read_layout, write_layout};
use fs_forms_layout::{ActionDoc, LayoutCode, Limits, OpenCatalog, Trigger};

fn file(trigger: &str) -> String {
    format!(
        r#"schema = 2
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

[[action]]
elem = "action:1"
trigger = "{trigger}"
source = 1
command = "task.create"

[[message]]
elem = "message:board-title/de"
key = "board-title"
lang = "de"
text = "Aufgaben"
"#
    )
}

fn read(text: &str, f: Format) -> Result<fs_forms_layout::LayoutDoc, Vec<LayoutCode>> {
    read_layout(&mut text.as_bytes(), Path::new("t"), f)
        .map_err(|e| e.iter().map(|c| c.code).collect())
}

#[test]
fn a_submit_action_is_read_and_written_in_format_two() {
    let doc = read(&file("submit"), Format::Toml).expect("read");
    let actions: Vec<&ActionDoc> = doc.actions.iter().map(|a| &a.v).collect();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].trigger, Trigger::Submit);
    let layout = doc.check(&OpenCatalog, &Limits::declared()).expect("check");
    let mut first = Vec::new();
    write_layout(&layout, Format::Toml, true, &mut first).unwrap();
    let first = String::from_utf8(first).unwrap();
    assert!(first.contains("trigger = \"submit\""), "{first}");
    let again = read(&first, Format::Toml)
        .unwrap()
        .check(&OpenCatalog, &Limits::declared())
        .unwrap();
    let mut second = Vec::new();
    write_layout(&again, Format::Toml, true, &mut second).unwrap();
    assert_eq!(first, String::from_utf8(second).unwrap());
}

#[test]
fn an_unknown_trigger_is_still_an_error() {
    let err = read(&file("shake"), Format::Toml).expect_err("must fail");
    assert_ne!(err, Vec::<LayoutCode>::new());
}

#[test]
fn a_submit_action_in_format_one_is_read_and_written_on_as_format_two() {
    // SW-3 r1: format 1 has no `elem` lines; the trigger is read the same, and the file written from it is format 2 with the trigger kept
    let one = file("submit")
        .lines()
        .filter(|l| !l.starts_with("elem = "))
        .collect::<Vec<_>>()
        .join("\n")
        .replace("schema = 2", "schema = 1");
    let doc = read(&one, Format::Toml).expect("read format 1");
    assert_eq!(doc.actions[0].v.trigger, Trigger::Submit);
    let layout = doc.check(&OpenCatalog, &Limits::declared()).expect("check");
    let mut out = Vec::new();
    write_layout(&layout, Format::Toml, true, &mut out).unwrap();
    let out = String::from_utf8(out).unwrap();
    assert!(
        out.starts_with("schema = 2") && out.contains("trigger = \"submit\""),
        "{out}"
    );
    let again = read(&out, Format::Toml).unwrap();
    assert_eq!(again.actions[0].v.trigger, Trigger::Submit);
}
