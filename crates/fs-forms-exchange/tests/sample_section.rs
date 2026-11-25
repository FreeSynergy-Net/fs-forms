//! The section `[sample]` (the test data a description asks for): the reader hands it on as text, it does not read its keys (its one
//! reader is `fs-forms-testdata`, strict); the writer gives it back, so import and export lose nothing.
use std::path::Path;

use fs_forms_exchange::{Format, read_layout, write_layout};
use fs_forms_layout::{Layout, Limits, OpenCatalog};

const HEAD: &str = r#"schema = 3
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
"#;

const SAMPLE: &str = r#"
[sample]
seed = 5
anchor = { week = "2026-W22", zone = "Europe/Berlin" }

[[sample.set]]
source = 1
form = "cards-in-columns@^1.0.0"
count = 5
distinct = { column = 2 }
cases = [ { case = "long-text", n = 1 } ]
"#;

fn layout(text: &str) -> Layout {
    read_layout(&mut text.as_bytes(), Path::new("t.toml"), Format::Toml)
        .expect("a description with `[sample]` is importable")
        .check(&OpenCatalog, &Limits::declared())
        .expect("checks")
}

fn export(l: &Layout, f: Format) -> String {
    let mut out = Vec::new();
    write_layout(l, f, false, &mut out).expect("write");
    String::from_utf8(out).expect("utf8")
}

/// The section as a TOML value, for comparing what was said with what came back (text may differ, the meaning may not).
fn value(text: &str) -> toml_edit::DocumentMut {
    text.parse().expect("toml")
}

#[test]
fn a_description_with_a_sample_section_is_importable_and_the_text_arrives_unchanged_in_meaning() {
    let l = layout(&format!("{HEAD}{SAMPLE}"));
    let got = l.doc().sample.clone().expect("the section is there");
    let (said, came) = (value(SAMPLE), value(&got));
    assert_eq!(
        said["sample"]["seed"].as_integer(),
        came["sample"]["seed"].as_integer()
    );
    let set = came["sample"]["set"]
        .as_array_of_tables()
        .and_then(|a| a.get(0))
        .expect("set");
    assert_eq!(set["count"].as_integer(), Some(5));
    assert_eq!(set["form"].as_str(), Some("cards-in-columns@^1.0.0"));
    assert_eq!(
        came["sample"]["anchor"]["week"].as_str(),
        Some("2026-W22"),
        "{got}"
    );
    // a list of tables may come back in the other (equal) notation: look at what is there, not at how it is written
    assert!(got.contains("long-text") && got.contains("case"), "{got}");
}

#[test]
fn without_the_section_there_is_no_text() {
    assert_eq!(layout(HEAD).doc().sample, None);
}

#[test]
fn the_section_survives_export_and_import_byte_equal_from_the_first_export() {
    let first = export(&layout(&format!("{HEAD}{SAMPLE}")), Format::Toml);
    assert!(
        first.contains("[sample]") && first.contains("[[sample.set]]"),
        "{first}"
    );
    let again = layout(&first);
    assert_eq!(export(&again, Format::Toml), first);
    assert_eq!(
        layout(&format!("{HEAD}{SAMPLE}")).doc().sample,
        again.doc().sample
    );
}

#[test]
fn a_section_without_tables_inside_is_kept_too() {
    let l = layout(&format!("{HEAD}\n[sample]\nseed = 9\n"));
    let first = export(&l, Format::Toml);
    assert_eq!(layout(&first).doc().sample, l.doc().sample);
    assert!(l.doc().sample.as_deref().unwrap().contains("seed = 9"));
}

#[test]
fn the_keys_of_the_section_are_not_checked_here_that_is_the_job_of_its_reader() {
    // `seeed` is no key of `[sample]`: the exchange reader takes the section whole, `read_sample` of fs-forms-testdata refuses it
    let l = layout(&format!("{HEAD}\n[sample]\nseeed = 1\n"));
    assert!(l.doc().sample.as_deref().unwrap().contains("seeed = 1"));
}

#[test]
fn a_section_that_is_no_table_is_a_type_problem_and_the_other_top_level_keys_stay_unknown() {
    let bad = |extra: &str| {
        read_layout(
            &mut format!("{HEAD}{extra}").as_bytes(),
            Path::new("t.toml"),
            Format::Toml,
        )
        .expect_err("refused")
        .iter()
        .map(|c| c.param("cause").map(str::to_owned))
        .collect::<Vec<_>>()
    };
    // a value that is no table: put before the first table header so it is a top-level key
    let text = HEAD.replacen("title =", "sample = 3\ntitle =", 1);
    let e = read_layout(&mut text.as_bytes(), Path::new("t.toml"), Format::Toml).expect_err("type");
    assert_eq!(e[0].param("cause"), Some("type"));
    assert_eq!(
        bad("\n[sampel]\nseed = 1\n"),
        [Some("key-unknown".to_owned())]
    );
}

#[test]
fn a_key_with_a_dot_in_the_section_is_quoted_never_split() {
    let l = layout(&format!(
        "{HEAD}\n[sample]\nseed = 1\n\n[[sample.set]]\nsource = 1\n\n[sample.set.distinct]\n\"a.b\" = 2\n"
    ));
    let got = l.doc().sample.clone().expect("section");
    assert!(got.contains("\"a.b\""), "{got}");
    assert!(value(&got)["sample"]["set"].as_array_of_tables().is_some());
}
