//! The G7 matcher: its suite (nine rules, nine mutants). The match against the REAL form catalog is tested in `fs-forms-run` (`form_check`).
#![cfg(feature = "conformance")]
use fs_forms_bind::suite::FormsSuite;
use fs_forms_bind::*;

#[test]
fn the_suite_proves_itself_nine_of_nine() {
    let (base_green, caught) = FormsSuite::prove();
    assert!(
        base_green,
        "{:?}",
        FormsSuite::run(&|f, m, c| match_form(f, m, c, None))
    );
    let mut want = FormsSuite::rules();
    want.sort_unstable();
    let mut got = caught;
    got.sort_unstable();
    assert_eq!(got, want, "every rule has its mutant red");
    assert_eq!(want.len(), 9);
}

#[test]
fn every_rule_has_exactly_one_mutant() {
    let mut flaws: Vec<String> = FormsSuite::mutants()
        .iter()
        .map(|m| format!("{:?}", m.0))
        .collect();
    flaws.sort_unstable();
    flaws.dedup();
    assert_eq!(flaws.len(), 9);
}
