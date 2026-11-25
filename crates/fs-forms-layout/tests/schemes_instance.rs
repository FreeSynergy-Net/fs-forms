//! The scheme policy comes from the instance (`CeilingSet.schemes` through `Limits::effective`), reaches `check`, and has a fixed floor.
//!
//! Pattern: Specification asked through the real path (`check`), with the instance as the only source of a wider list.
mod support;

use fs_forms_layout::*;
use support::*;

fn check_with(list: &[&str], limits: &Limits) -> Result<Layout, Vec<Coded<LayoutCode>>> {
    let mut d = doc("board");
    d.pages[0].v.blocks[0].v.bindings[0].v.schemes = Some(
        list.iter()
            .enumerate()
            .map(|(i, s)| item(&format!("scheme:b/{i}"), (*s).to_owned()))
            .collect(),
    );
    d.check(&OpenCatalog, limits)
}

fn codes(r: Result<Layout, Vec<Coded<LayoutCode>>>) -> Vec<LayoutCode> {
    r.err().unwrap_or_default().iter().map(|c| c.code).collect()
}

fn instance(list: &'static [&'static str]) -> Limits {
    let ceil = CeilingSet {
        schemes: Some(list),
        ..CeilingSet::default()
    };
    Limits::effective(&Limits::declared(), &ceil)
}

#[test]
fn the_default_is_the_closed_list() {
    assert_eq!(Limits::declared().schemes, DEFAULT_ALLOWED);
    assert_eq!(
        codes(check_with(&["mailto"], &Limits::declared())),
        vec![LayoutCode::SchemeNotAllowed]
    );
}

#[test]
fn the_instance_can_allow_mailto() {
    let l = instance(&["https", "mailto"]);
    assert!(check_with(&["https", "mailto"], &l).is_ok());
}

#[test]
fn a_person_can_not_widen_it() {
    let person = Limits {
        schemes: &["https", "mailto"],
        ..Limits::declared()
    };
    let eff = Limits::effective(&person, &CeilingSet::default());
    assert_eq!(eff.schemes, DEFAULT_ALLOWED);
    assert_eq!(
        codes(check_with(&["mailto"], &eff)),
        vec![LayoutCode::SchemeNotAllowed]
    );
}

#[test]
fn a_wide_instance_list_does_not_open_the_floor() {
    let l = instance(&["https", "javascript", "vbscript", "data", "file", "blob"]);
    for s in NEVER_ALLOWED {
        assert_eq!(
            codes(check_with(&[s], &l)),
            vec![LayoutCode::SchemeNotAllowed],
            "{s}"
        );
    }
    assert!(check_with(&["https"], &l).is_ok());
}

/// The old block list (`javascript`, `vbscript` only) let `data:` and `file:` pass; they are refused now, by default and by floor.
#[test]
fn the_old_block_list_hole_is_closed() {
    for s in ["data", "file"] {
        assert_eq!(
            codes(check_with(&[s], &Limits::declared())),
            vec![LayoutCode::SchemeNotAllowed]
        );
    }
}
