//! Slot-level locks (Opus probes P1, P2, P5, Q3): a layout problem next to a missing component, input locks, the type right.
mod common;
use common::*;
use fs_forms_bind::testing::*;
use fs_forms_bind::*;

fn has(p: &Plan, code: &str) -> bool {
    all_codes(p).contains(&code.to_owned())
}

fn unknown_title_field(r: &mut Resolved) {
    r.pages[0].places[0].block_mut().unwrap().bindings[1].kind = BindKind::Field {
        field: field("title", Basic::Unknown),
        role: Role::Display,
        schemes: None,
    };
}

/// P1 (B4): a layout problem AND a missing component on one field slot: both stay visible (ready placement).
#[test]
fn p1_component_missing_next_to_a_slot_problem() {
    let mut r = tasks_resolved();
    unknown_title_field(&mut r);
    r.problems
        .push(problem("b-card-title-0", "layout-error-p1"));
    let p = build_with(&r, &TestCatalog::new(), &tasks_facts(), host());
    assert!(has(&p, "layout-error-p1"), "{:?}", all_codes(&p));
    assert!(
        has(&p, "bind-error-field-component-missing"),
        "{:?}",
        all_codes(&p)
    );
}

/// P2 (B4): the same on a placement that is locked for another reason (block not built).
#[test]
fn p2_component_missing_next_to_a_slot_problem_on_a_locked_place() {
    let mut r = tasks_resolved();
    unknown_title_field(&mut r);
    r.problems
        .push(problem("b-card-title-0", "layout-error-p2"));
    let mut cat = TestCatalog::new();
    cat.unbuilt.push("board".into());
    let p = build_with(&r, &cat, &tasks_facts(), host());
    assert!(has(&p, "layout-error-p2"), "{:?}", all_codes(&p));
    assert!(
        has(&p, "bind-error-field-component-missing"),
        "{:?}",
        all_codes(&p)
    );
    assert!(has(&p, "bind-error-block-not-built"));
}

/// Q3: the input lock of a field in a READY placement is part of `Plan::locks()`.
#[test]
fn q3_input_lock_of_a_ready_place_is_in_plan_locks() {
    let mut r = tasks_resolved();
    r.pages[0].places[0].block_mut().unwrap().bindings[1].kind = BindKind::Field {
        field: field("title", Basic::Text),
        role: Role::Input,
        schemes: None,
    };
    let h = HostCaps {
        write: false,
        workflow: false,
    };
    let p = build_with(&r, &TestCatalog::new(), &tasks_facts(), h);
    assert!(matches!(
        p.place("board", "main").unwrap().status,
        PlaceStatus::Ready(_)
    ));
    assert!(
        p.locks()
            .iter()
            .any(|l| code_of(l) == "bind-error-write-later" && matches!(l.part, Part::Slot { .. })),
        "{:?}",
        all_codes(&p)
    );
}

/// P5: the type right beats "coming", also when the source is locked.
#[test]
fn p5_type_right_beats_coming_with_a_locked_source() {
    let mut c = cmd("setColumns", false, false, false);
    c.desc.type_may = false;
    let mut r = tasks_resolved();
    r.menu = vec![MenuIn {
        elem: "m-c".into(),
        kind: MenuKind::Command {
            command: c,
            on_selected: false,
            shortcut: None,
        },
        children: vec![],
    }];
    let p = build_with(&r, &TestCatalog::new(), &FakeFacts(vec![]), host());
    assert_eq!(command_state(p.menu.commands()[0], None), Shown::Hidden);
}

/// C4 (B5): a condition chip whose component is not built is visible, with the chip block and the schema names.
#[test]
fn c4_condition_chip_not_built_is_visible() {
    let mut r = tasks_resolved();
    r.pages[0].places[0].block_mut().unwrap().bindings[1].kind = BindKind::Condition {
        path: vec!["done".into()],
        basic: Basic::Bool,
    };
    let mut cat = TestCatalog::new();
    cat.unbuilt.push("field-bool".into());
    let p = build_with(&r, &cat, &tasks_facts(), host());
    let l = p
        .locks()
        .into_iter()
        .find(|l| l.cause.coded().param("block") == Some("field-bool"))
        .unwrap_or_else(|| panic!("{:?}", all_codes(&p)));
    assert_eq!(code_of(l), "bind-error-field-component-missing");
    assert_eq!(l.cause.coded().param("basic"), Some("bool"));
    assert_eq!(l.cause.coded().param("role"), Some("chip"));
    // A built chip stays silent.
    let p = build_with(&r, &TestCatalog::new(), &tasks_facts(), host());
    assert!(!has(&p, "bind-error-field-component-missing"));
}
