//! The `PlanSuite` proves itself: green base, 12 of 12 mutants caught, codes unique.
#![cfg(feature = "conformance")]
use fs_forms_bind::suite::*;
use fs_forms_bind::*;

#[test]
fn proven() {
    let proof = PlanSuite::prove();
    assert!(
        proof.base_green,
        "{:?}",
        PlanSuite::run(&BindBench::new(None))
    );
    assert_eq!(proof.caught.len(), 12, "{proof:?}");
    assert!(proof.is_proven());
}

#[test]
fn every_rule_has_exactly_one_mutant() {
    let mut rules: Vec<&str> = PlanSuite::rules().iter().map(|r| r.0).collect();
    let mut named: Vec<&str> = bench_mutants().iter().map(|m| m.1).collect();
    rules.sort_unstable();
    named.sort_unstable();
    assert_eq!(rules, named);
    assert_eq!(rules.len(), 12);
}

#[test]
fn suite_codes_are_once_and_match_the_rules() {
    let mut all = SuiteCode::ALL.to_vec();
    all.sort_unstable();
    all.dedup();
    assert_eq!(all.len(), 13);
    for (rule, _) in PlanSuite::rules() {
        assert!(SuiteCode::ALL.contains(&rule.trim_start_matches("plan.")));
    }
}

#[test]
fn bind_codes_are_eighteen_unique_kebab_names() {
    let mut names: Vec<&str> = BindCode::ALL.iter().map(|c| c.name()).collect();
    assert_eq!(names.len(), 18);
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 18);
    for c in BindCode::ALL {
        assert_eq!(c.key(), format!("bind-error-{}", c.name()));
        assert!(
            c.name()
                .chars()
                .all(|ch| ch.is_ascii_lowercase() || ch == '-')
        );
    }
}
