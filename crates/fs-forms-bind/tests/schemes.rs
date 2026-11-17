//! H-K7: the per-use scheme list is never defaulted; a missing list locks the placement.
mod common;
use common::*;
use fs_forms_bind::testing::*;
use fs_forms_bind::*;

fn with_field(basic: Basic, schemes: Option<Vec<&str>>, problems: Vec<Problem>) -> Plan {
    let mut r = tasks_resolved();
    r.pages = vec![PageIn {
        elem: "pg".into(),
        page: "p".into(),
        seq: 0,
        frame: "form".into(),
        places: vec![PlaceIn {
            elem: "pl".into(),
            place: "f".into(),
            seq: 0,
            block: "board-detail".into(),
            source: None,
            bindings: vec![BindingIn {
                elem: "b1".into(),
                slot: "properties".into(),
                nth: 0,
                kind: BindKind::Field {
                    field: field("homepage", basic),
                    role: Role::Input,
                    schemes: schemes.map(|v| v.into_iter().map(str::to_owned).collect()),
                },
            }],
        }],
    }];
    r.problems = problems;
    build_with(&r, &TestCatalog::new(), &tasks_facts(), host())
}

fn problem(code: &str) -> Problem {
    Problem {
        elem: Some("b1".into()),
        code: code.into(),
        params: vec![("path".into(), "homepage".into())],
    }
}

#[test]
fn missing_list_found_by_f1b_locks_the_place_with_its_code() {
    let p = with_field(
        Basic::Link,
        None,
        vec![problem("layout-error-schemes-missing")],
    );
    let l = locks_of(&p, "p", "f");
    assert_eq!(l.len(), 1);
    assert_eq!(code_of(&l[0]), "layout-error-schemes-missing");
}

#[test]
fn provider_unchecked_is_a_notice_and_the_one_rule_still_locks() {
    let p = with_field(
        Basic::Link,
        None,
        vec![problem("layout-error-provider-unchecked")],
    );
    let l = locks_of(&p, "p", "f");
    assert_eq!(
        code_of(&l[0]),
        "layout-error-schemes-missing",
        "one rule source, one code"
    );
    assert_eq!(l[0].cause.coded().param("path"), Some("homepage"));
    assert!(
        p.notices
            .iter()
            .any(|n| n.cause.coded().code == "layout-error-provider-unchecked")
    );
}

#[test]
fn no_problem_and_no_list_still_locks_never_defaults() {
    let p = with_field(Basic::Image, None, vec![]);
    assert_eq!(
        code_of(&locks_of(&p, "p", "f")[0]),
        "layout-error-schemes-missing"
    );
    assert_eq!(locks_of(&p, "p", "f").len(), 1);
}

#[test]
fn list_is_kept_in_order() {
    let p = with_field(Basic::Image, Some(vec!["ssh", "https"]), vec![]);
    let PlaceStatus::Ready(r) = &p.place("p", "f").unwrap().status else {
        panic!("locked")
    };
    let Fill::Field(f) = &r.slots[0].fill else {
        panic!()
    };
    assert_eq!(f.schemes, Some(vec!["ssh".to_owned(), "https".to_owned()]));
}

#[test]
fn unexpected_list_on_text_comes_from_f1b() {
    let p = with_field(
        Basic::Text,
        Some(vec!["https"]),
        vec![problem("layout-error-schemes-unexpected")],
    );
    assert_eq!(
        code_of(&locks_of(&p, "p", "f")[0]),
        "layout-error-schemes-unexpected"
    );
}
