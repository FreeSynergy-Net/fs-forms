//! `fixed = "<name>"` in `[[sample.set]]` of a description: the same road as the generator (spec -> builder -> set), a code for every misuse.

mod described_support;

use described_support::{Table, code, describe, read, specs};
use fs_forms_testdata::{SampleCode, SampleModule, SampleSpec, read_sample};

const KEY: &str = "bars-in-time@1.1.0";

fn description(source_type: &str, set: &str) -> (fs_forms_layout::Layout, String) {
    let text = describe(&[source_type], &[("calendar-grid", 1)]);
    let sample = format!(
        "[sample]\nanchor = {{ week = \"2026-W47\", zone = \"Europe/Berlin\" }}\n[[sample.set]]\nsource = 1\nform = \"bars-in-time@^1.1.0\"\n{set}\n"
    );
    (read(&text), sample)
}

#[test]
fn a_description_asks_for_the_fixed_events_and_gets_them_for_its_week() {
    let (l, sample) = description("Event", "fixed = \"events.sample\"");
    let got = specs(&l, Some(&sample));
    assert_eq!(got[0].spec.sets[0].fixed.as_deref(), Some("events.sample"));
    assert_eq!(
        got[0].spec.sets[0].count, 0,
        "no count: `fixed` excludes it"
    );
    let set = got[0].build().expect("builds");
    let rows = set.rows(KEY).expect("rows");
    assert_eq!(rows.len(), 11, "the week of 16 November 2026 of the sample");
    assert!(set.now.is_some());
    let module = SampleModule::from_description(&l, &Table, Some(&sample), 1).expect("module");
    drop(module);
}

#[test]
fn the_wishes_of_the_generator_next_to_fixed_are_fixed_conflict() {
    for extra in [
        "count = 3",
        "distinct = { lane = 2 }",
        "empty = { lane = 1 }",
        "invalid = 1",
        "cases = [ { case = \"all-day\", times = 1 } ]",
    ] {
        let (l, sample) = description("Event", &format!("fixed = \"events.sample\"\n{extra}"));
        assert_eq!(code(&l, &sample), SampleCode::FixedConflict, "{extra}");
    }
}

#[test]
fn a_name_that_is_not_in_the_registry_is_fixed_unknown() {
    let (l, sample) = description("Event", "fixed = \"nothing.here\"");
    let spec = &SampleSpec::from_description(&l, &Table, Some(&sample)).expect("reads")[0];
    assert_eq!(
        spec.build().expect_err("unknown").code(),
        SampleCode::FixedUnknown
    );
}

#[test]
fn a_source_of_another_type_than_the_fixed_one_is_fixed_type() {
    let (l, sample) = description("Task", "fixed = \"events.sample\"");
    let spec = &SampleSpec::from_description(&l, &Table, Some(&sample)).expect("reads")[0];
    assert_eq!(
        spec.build().expect_err("type").code(),
        SampleCode::FixedType
    );
}

#[test]
fn read_sample_keeps_fixed_and_a_typo_stays_an_error() {
    let ok = read_sample("[[sample.set]]\nsource = 1\nform = \"x@1\"\nfixed = \"a.b\"\n")
        .expect("reads")
        .expect("section");
    assert_eq!(ok.sets[0].wish.fixed.as_deref(), Some("a.b"));
    let typo = read_sample("[[sample.set]]\nsource = 1\nform = \"x@1\"\nfixd = \"a.b\"\n")
        .expect_err("unknown key");
    assert_eq!(typo.code(), SampleCode::SectionBad);
}
