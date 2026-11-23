//! Contract ranges (`name@<req>`, the `semver::VersionReq` grammar of the host): both bounds, comma form, three parts.
#![allow(clippy::assert_is_empty, clippy::many_single_char_names)] // tests: short names and plain asserts read best
mod support;

use fs_forms_layout::*;
use support::*;

fn codes(r: Result<Layout, Vec<Coded<LayoutCode>>>) -> Vec<LayoutCode> {
    r.expect_err("must fail").iter().map(|c| c.code).collect()
}
fn check(d: LayoutDoc) -> Result<Layout, Vec<Coded<LayoutCode>>> {
    d.check(&OpenCatalog, &Limits::declared())
}

#[test]
fn n17_a_range_needs_both_bounds() {
    for text in [
        "tasks >=0.2",
        "tasks <0.3",
        "tasks",
        "tasks@>=0.3, <0.2",
        "tasks >0.2",
    ] {
        let mut d = doc("board");
        d.requires = vec![item("requires:t", RequiresDoc { range: text.into() })];
        assert_eq!(codes(check(d)), vec![LayoutCode::RangeInvalid], "{text}");
        let mut d = doc("board");
        d.sources = vec![item(
            "source:1",
            SourceDoc {
                range: text.into(),
                data_type: "T".into(),
                scope: None,
            },
        )];
        assert_eq!(codes(check(d)), vec![LayoutCode::RangeInvalid], "{text}");
    }
}

#[test]
fn n17b_a_range_is_written_like_the_host_does_with_a_comma_and_three_parts() {
    for text in [
        "tasks@>=0.0.1, <0.1",
        "tasks@>=0.2, <0.3",
        "tasks@>=1.2.3, <2",
        "tasks@>=0.0.1, <0.1.0",
    ] {
        let mut d = doc("board");
        d.requires = vec![item("requires:t", RequiresDoc { range: text.into() })];
        assert!(check(d).is_ok(), "{text}");
    }
    for text in [
        "tasks >=0.2, <0.3",
        "tasks@>=0.2 <0.3",
        "tasks@>=0.3, <0.2",
        "tasks@>=zz, <0.3",
        "tasks@>=0.0.1, >=0.0.2",
        "bad name@>=0.1, <0.2",
    ] {
        let mut d = doc("board");
        d.requires = vec![item("requires:t", RequiresDoc { range: text.into() })];
        assert_eq!(codes(check(d)), vec![LayoutCode::RangeInvalid], "{text}");
    }
}

#[test]
fn n17c_one_spelling_no_blank_after_an_operator_and_no_pre_release() {
    for text in [
        "tasks@>= 1.2, < 2.0",
        "tasks@>=1.2,<2.0",
        "tasks@>=0.2.0-rc.1, <0.3",
        "tasks@>=0.2, <0.3.0-rc.1",
    ] {
        let mut d = doc("board");
        d.requires = vec![item("requires:t", RequiresDoc { range: text.into() })];
        assert_eq!(codes(check(d)), vec![LayoutCode::RangeInvalid], "{text}");
    }
    assert_eq!(
        range_bounds("tasks@>=1.2, <2.0").map(|b| (b.low, b.below)),
        Some(((1, 2, 0), (2, 0, 0)))
    );
}
