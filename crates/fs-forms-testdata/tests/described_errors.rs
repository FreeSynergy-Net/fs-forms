//! Errors of description -> wishes: a broken `[sample]` and a section that does not fit the description are codes, never ignored.

mod described_support;

use described_support::*;
use fs_forms_testdata::{SampleCode, SampleModule, SampleSpec};

#[test]
fn a_broken_section_sample_is_a_code_never_ignored() {
    let l = read(&describe(&["Task"], &[("board", 1)]));
    for (name, text) in [
        ("not toml", "[sample\nseed = 1"),
        ("unknown key", "[sample]\nseeed = 1\n"),
        ("wrong kind", "[sample]\nseed = \"one\"\n"),
        (
            "unknown set key",
            "[[sample.set]]\nsource = 1\nform = \"rows@^1.0.0\"\nrelation = 1\n",
        ),
        (
            "anchor twice",
            "[sample]\nanchor = { month = \"2026-05\", day = \"2026-05-01\", zone = \"Europe/Berlin\" }\n",
        ),
        (
            "anchor text",
            "[sample]\nanchor = { month = \"May\", zone = \"Europe/Berlin\" }\n",
        ),
        (
            "unknown case",
            "[[sample.set]]\nsource = 1\nform = \"rows@^1.0.0\"\ncases = [ { case = \"nope\" } ]\n",
        ),
        (
            "case key",
            "[[sample.set]]\nsource = 1\nform = \"rows@^1.0.0\"\ncases = [ { case = \"all-day\" } ]\n",
        ),
    ] {
        assert_eq!(code(&l, text), SampleCode::SectionBad, "{name}");
    }
}

#[test]
fn a_section_that_does_not_fit_the_description_is_a_code() {
    let l = read(&describe(&["Task"], &[("board", 1)]));
    let set = |extra: &str| {
        format!("[[sample.set]]\nsource = 1\nform = \"cards-in-columns@^1.0.0\"\n{extra}\n")
    };
    assert_eq!(
        code(&l, "[[sample.set]]\nsource = 2\nform = \"rows@^1.0.0\"\n"),
        SampleCode::SourceUnknown
    );
    assert_eq!(
        code(&l, "[[sample.set]]\nsource = 0\nform = \"rows@^1.0.0\"\n"),
        SampleCode::SourceUnknown
    );
    assert_eq!(code(&l, &set("type = \"Event\"")), SampleCode::TypeMismatch);
    assert_eq!(
        code(&l, &format!("{}{}", set(""), set(""))),
        SampleCode::WishTwice
    );
    assert_eq!(
        code(&l, "[[sample.set]]\nsource = 1\nform = \"nope@^1.0.0\"\n"),
        SampleCode::FormUnknown
    );
    assert_eq!(
        code(
            &l,
            "[sample]\nanchor = { month = \"2026-05\", zone = \"Mars/Base\" }\n"
        ),
        SampleCode::ZoneUnknown
    );
    assert_eq!(
        code(
            &l,
            "[sample]\nanchor = { month = \"2026-13\", zone = \"Europe/Berlin\" }\n"
        ),
        SampleCode::AnchorBad
    );
    // a wish the form cannot take is the builder's code, not dropped
    let built = SampleSpec::from_description(
        &l,
        &Table,
        Some(&set("cases = [ { case = \"all-day\", times = 1 } ]")),
    )
    .unwrap()[0]
        .build()
        .expect_err("no interval in a board");
    assert_eq!(built.code(), SampleCode::CaseImpossible);
}

#[test]
fn a_module_for_an_unknown_source_is_a_code() {
    let l = read(&describe(&["Task"], &[("board", 1)]));
    let err = SampleModule::from_description(&l, &Table, None, 3)
        .err()
        .expect("fails");
    assert_eq!(err.code(), SampleCode::SourceUnknown);
}

#[test]
fn a_description_without_sample_and_the_same_with_an_empty_section_agree() {
    let l = read(&describe(&["Task"], &[("board", 1)]));
    assert_eq!(specs(&l, None), specs(&l, Some("[sample]\n")));
    assert_eq!(
        specs(&l, None),
        specs(&l, Some("title = \"nothing about samples\"\n"))
    );
}
