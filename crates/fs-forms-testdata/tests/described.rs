//! Description -> wishes (package 4a): the forms the blocks need, the defaults of the declaration, `[sample]`, errors, determinism.
//! The block catalog is a small table here (the host gives the real adapter; this crate links no component crate).

use fs_forms_testdata::{Case, SampleModule, SetWish, SourceSpec, needs};

mod common;
mod described_support;

use described_support::*;

fn forms_of(spec: &SourceSpec) -> Vec<(String, String, u32)> {
    spec.spec
        .sets
        .iter()
        .map(|w| (w.form.clone(), w.data_type.clone(), w.count))
        .collect()
}

#[test]
fn a_board_needs_its_form_and_the_heads_of_its_lane_slot_come_from_the_other_source() {
    let (l, text) = file(&format!("{FORMS}/board-swimlanes.toml"));
    let got = specs(&l, Some(&text));
    assert_eq!(got.len(), 2);
    assert_eq!(
        forms_of(&got[0]),
        [("cards-in-columns@^1.0.0".into(), "Task".into(), 8)]
    );
    assert_eq!(
        forms_of(&got[1]),
        [("rows@^1.0.0".into(), "Lane".into(), 8)]
    );
}

#[test]
fn every_board_description_of_the_tasks_repo_reads_the_same_way() {
    for name in ["board", "board-plain", "board-wip"] {
        let (l, text) = file(&format!("{FORMS}/{name}.toml"));
        let got = specs(&l, Some(&text));
        assert_eq!(got.len(), 1, "{name}");
        assert_eq!(
            forms_of(&got[0]),
            [("cards-in-columns@^1.0.0".into(), "Task".into(), 8)],
            "{name}"
        );
    }
}

#[test]
fn a_description_without_a_block_that_draws_a_form_asks_for_nothing() {
    for name in ["day", "list"] {
        let (l, text) = file(&format!("{FORMS}/{name}.toml"));
        let got = specs(&l, Some(&text));
        assert!(got.iter().all(|s| s.spec.sets.is_empty()), "{name}");
        // and the module of such a source is empty but there
        assert!(SampleModule::from_description(&l, &Table, Some(&text), 1).is_ok());
    }
}

#[test]
fn the_groups_description_of_the_host_gets_a_tree_and_rows_for_its_one_source() {
    let (l, text) = file(&here("groups.toml"));
    let got = specs(&l, Some(&text));
    assert_eq!(got.len(), 1);
    assert_eq!(
        forms_of(&got[0]),
        [
            ("tree@^1.0.0".into(), "Group".into(), 8),
            ("rows@^1.0.0".into(), "Group".into(), 8)
        ]
    );
    let set = got[0].build().expect("builds");
    let rows = set.rows("tree@1.0.0").expect("tree rows");
    assert_eq!(rows.len(), 8);
    assert!(
        rows.iter().filter(|r| r.get("parent").is_some()).count() >= 4,
        "chains of depth 2: four edges"
    );
}

#[test]
fn the_default_follows_the_declaration_never_a_name() {
    let (l, _) = file(&here("board.toml"));
    let set = specs(&l, None)[0].build().expect("builds");
    let rows = set.rows(&set.slices[0].form).expect("rows");
    assert_eq!(rows.len(), 8);
    let distinct = |c: &str| {
        rows.iter()
            .filter_map(|r| r.get(c))
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    };
    assert_eq!(distinct("column"), 3, "a category: three values");
    assert!(distinct("title") > 3, "the naming channel stays free");
    for c in ["column", "title"] {
        assert!(rows.iter().all(|r| r.get(c).is_some()), "{c} is required");
    }
    // an optional text channel with values: filled in every second row, and with values left to see
    let lane = rows.iter().filter(|r| r.get("lane").is_some()).count();
    assert_eq!(lane, 4);
}

#[test]
fn the_needs_keep_the_highest_minor_of_one_form_per_source() {
    let l = read(&describe(
        &["Task"],
        &[("board", 1), ("board", 1), ("tree", 1)],
    ));
    let got = needs(&l, &Table).unwrap();
    assert_eq!(got.len(), 2);
    assert_eq!(
        (got[0].source, got[0].form.name.as_str()),
        (1, "cards-in-columns")
    );
    assert_eq!(got[0].data_type, "Task");
}

const SAMPLE: &str = r#"
[sample]
seed = 5
anchor = { week = "2026-W22", zone = "Europe/Berlin" }
forbid = [ { command = "move" } ]

[[sample.set]]
source = 1
form = "cards-in-columns@^1.0.0"
type = "Task"
count = 5
distinct = { column = 2 }
cases = [ { case = "long-text", n = 1 } ]
"#;

#[test]
fn the_section_sample_replaces_the_default_of_its_form() {
    let l = read(&describe(&["Task"], &[("board", 1)]));
    let got = specs(&l, Some(SAMPLE));
    let s = &got[0].spec;
    assert_eq!(s.seed, 5);
    assert_eq!(s.forbid.len(), 1);
    assert_eq!(s.sets.len(), 1);
    let w: &SetWish = &s.sets[0];
    assert_eq!((w.count, w.data_type.as_str()), (5, "Task"));
    assert_eq!(w.distinct, [("column".to_owned(), 2)]);
    assert_eq!(w.cases, [Case::LongText(1)]);
    let set = got[0].build().unwrap();
    assert_eq!(set.rows(&set.slices[0].form).unwrap().len(), 5);
}

#[test]
fn a_wish_of_a_form_no_block_needs_is_added_not_lost() {
    let l = read(&describe(&["Task"], &[("table", 1)]));
    let text = SAMPLE;
    let got = specs(&l, Some(text));
    let names: Vec<_> = got[0].spec.sets.iter().map(|w| w.form.as_str()).collect();
    assert_eq!(names, ["rows@^1.0.0", "cards-in-columns@^1.0.0"]);
}

#[test]
fn the_goldens_are_frozen() {
    let (l, _) = file(&here("groups.toml"));
    let mut text = String::new();
    for s in specs(&l, None) {
        text += &s.build().unwrap().render();
    }
    common::golden("description-groups", &text);
    let l = read(&describe(&["Task"], &[("board", 1)]));
    common::golden(
        "description-board",
        &specs(&l, None)[0].build().unwrap().render(),
    );
}

#[test]
fn the_same_description_gives_the_same_set_and_a_seed_changes_it() {
    let (l, _) = file(&here("groups.toml"));
    let render = |text: Option<&str>| specs(&l, text)[0].build().unwrap().render();
    assert_eq!(render(None), render(None));
    let other = "[sample]\nseed = 2\n";
    assert_ne!(render(None), render(Some(other)));
    assert_eq!(render(Some(other)), render(Some(other)));
}
