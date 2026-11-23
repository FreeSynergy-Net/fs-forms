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

/// The block filter of `match_form`: the frontend names a block; a block that does not carry the form is not taken.
#[test]
fn the_frontend_names_the_block_and_a_block_without_the_form_is_not_taken() {
    use fs_forms_bind::form_fixtures::{fixture_catalog, form_ref, tree_component, tree_module};
    let forms = fixture_catalog();
    let module = tree_module();
    let comps = [
        tree_component(),
        ComponentForms {
            block: "tree2".into(),
            forms: vec![form_ref("tree", 1, 1)],
        },
    ];
    assert_eq!(
        match_form(&forms, &module, &comps, None),
        Matched::Blocks(vec!["tree".into(), "tree2".into()])
    );
    assert_eq!(
        match_form(&forms, &module, &comps, Some("tree2")),
        Matched::Blocks(vec!["tree2".into()])
    );
    assert!(matches!(
        match_form(&forms, &module, &comps, Some("table")),
        Matched::Fallback(_)
    ));
}
