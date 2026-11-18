//! "Nothing silent" (Opus probes R1-R3 and more): every layout problem stays visible somewhere, warnings never lock,
//! the state counts notices, the highest version is chosen, style and settings notices are kept.
mod common;
use common::*;
use fs_forms_bind::testing::*;
use fs_forms_bind::*;

fn has(p: &Plan, code: &str) -> bool {
    all_codes(p).contains(&code.to_owned())
}

/// R1: a layout problem on a source whose provider is missing is still visible.
#[test]
fn r1_problem_on_missing_source_is_not_swallowed() {
    let mut r = tasks_resolved();
    r.problems.push(problem("s0", "layout-error-r1"));
    let p = build_with(&r, &TestCatalog::new(), &FakeFacts(vec![]), host());
    assert!(has(&p, "layout-error-r1"), "{:?}", all_codes(&p));
}

/// R2: two problems on one reached source: both visible.
#[test]
fn r2_second_problem_on_source_is_not_swallowed() {
    let mut r = tasks_resolved();
    r.problems.push(problem("s0", "layout-error-r2a"));
    r.problems.push(problem("s0", "layout-error-r2b"));
    let p = build_with(&r, &TestCatalog::new(), &tasks_facts(), host());
    assert!(
        has(&p, "layout-error-r2a") && has(&p, "layout-error-r2b"),
        "{:?}",
        all_codes(&p)
    );
}

/// R3: a binding problem of a placement that is locked for another reason stays visible.
#[test]
fn r3_binding_problem_on_locked_place_is_not_swallowed() {
    let mut r = tasks_resolved();
    r.problems.push(problem("b-cards-0", "layout-error-r3"));
    r.problems.push(problem("b-cards", "layout-error-r3b"));
    let mut cat = TestCatalog::new();
    cat.unbuilt.push("board".into());
    let p = build_with(&r, &cat, &tasks_facts(), host());
    assert!(has(&p, "layout-error-r3b"), "{:?}", all_codes(&p));
    assert!(has(&p, "bind-error-block-not-built"));
}

/// R3 (field): a missing field component of a slot of a locked placement stays visible, with schema names, not Debug text.
#[test]
fn r3_field_lock_on_locked_place_is_kept_with_schema_names() {
    let mut cat = TestCatalog::new();
    cat.unbuilt.push("board".into());
    cat.unbuilt.push("field-date-time".into());
    let p = build_with(&tasks_resolved(), &cat, &tasks_facts(), host());
    let l = p
        .locks()
        .into_iter()
        .find(|l| code_of(l) == "bind-error-field-component-missing")
        .expect("component lock kept");
    assert_eq!(l.cause.coded().param("basic"), Some("date-time"));
    assert_eq!(l.cause.coded().param("role"), Some("compact"));
}

/// R3 (command): a locked command of a slot of a locked placement stays visible.
#[test]
fn r3_command_lock_on_locked_place_is_kept() {
    let mut cat = TestCatalog::new();
    cat.unbuilt.push("board".into());
    let p = build_with(
        &tasks_resolved(),
        &cat,
        &tasks_facts(),
        HostCaps {
            write: false,
            workflow: false,
        },
    );
    assert!(has(&p, "bind-error-write-later"), "{:?}", all_codes(&p));
}

/// O21: a problem on a reached source locks the source (not only a notice).
#[test]
fn problem_on_reached_source_locks_it() {
    let mut r = tasks_resolved();
    r.problems.push(problem("s0", "layout-error-x"));
    let p = build_with(&r, &TestCatalog::new(), &tasks_facts(), host());
    let lock = p.sources[0].status.as_ref().expect_err("source locked");
    assert_eq!(code_of(lock), "layout-error-x");
    assert!(matches!(lock.cause, Cause::Layout(_)));
}

/// O2 + O22: a warning is a notice only: it locks nothing, but the plan is no longer `Ready`.
#[test]
fn warning_is_a_notice_and_makes_the_plan_partial() {
    let mut r = tasks_resolved();
    assert_eq!(
        build_with(&r, &TestCatalog::new(), &tasks_facts(), host()).state,
        PlanState::Ready
    );
    r.warnings
        .push(problem("pl-board-main", "layout-error-warn"));
    let p = build_with(&r, &TestCatalog::new(), &tasks_facts(), host());
    assert!(
        matches!(
            p.place("board", "main").unwrap().status,
            PlaceStatus::Ready(_)
        ),
        "a warning must not lock"
    );
    assert_eq!(p.locks(), Vec::<&PartLock>::new());
    assert!(
        p.notices
            .iter()
            .any(|n| n.cause.coded().code == "layout-error-warn")
    );
    assert_eq!(p.state, PlanState::Partial);
}

/// O12: the HIGHEST version of the range wins, whatever the order of the offers.
#[test]
fn highest_version_in_range_wins() {
    let mut r = tasks_resolved();
    r.sources[0].range = Range {
        min: (0, 1),
        max: (0, 3),
    };
    for offers in [
        vec![provider(0, 1), provider(0, 2)],
        vec![provider(0, 2), provider(0, 1)],
    ] {
        let f = FakeFacts(vec![("tasks".into(), Offer::Reached(offers))]);
        let p = build_with(&r, &TestCatalog::new(), &f, host());
        assert_eq!(p.sources[0].status.as_ref().unwrap().version, (0, 2));
    }
}

/// O15: settings that fell back to defaults are announced.
#[test]
fn settings_defaults_are_announced() {
    let mut s = settings();
    s.health = SettingsHealth::Defaults {
        cause: "setting-error-unreadable".into(),
    };
    let p = build_custom(&tasks_resolved(), &s, &standard());
    let n = p
        .notices
        .iter()
        .find(|n| n.cause.coded().code == "bind-error-settings-defaults")
        .expect("notice");
    assert_eq!(
        n.cause.coded().param("setting"),
        Some("setting-error-unreadable")
    );
}

/// O16: the style of the person (settings) wins over the style of the description; both are only echoed.
#[test]
fn style_of_the_person_wins() {
    let mut r = tasks_resolved();
    r.style = Some("from-layout".into());
    let mut s = settings();
    assert_eq!(
        build_custom(&r, &s, &standard()).style.as_deref(),
        Some("from-layout")
    );
    s.style = Some("from-person".into());
    assert_eq!(
        build_custom(&r, &s, &standard()).style.as_deref(),
        Some("from-person")
    );
}

/// O17: two placements with the same `seq` on one page are both refused.
#[test]
fn same_seq_on_a_page_locks_both_placements() {
    let mut r = tasks_resolved();
    r.pages[1].places[1].seq = 0;
    let p = build_with(&r, &TestCatalog::new(), &tasks_facts(), host());
    for pl in ["props", "empty"] {
        let codes: Vec<String> = locks_of(&p, "detail", pl).iter().map(code_of).collect();
        assert!(
            codes.contains(&"bind-error-seq-twice".to_owned()),
            "{pl}: {codes:?}"
        );
    }
}
