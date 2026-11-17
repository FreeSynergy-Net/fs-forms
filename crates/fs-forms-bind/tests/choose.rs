//! H-K2 (choose) and H-K19 (style / determinism).
mod common;
use common::*;
use fs_forms_bind::testing::*;
use fs_forms_bind::*;

fn one_field_resolved(basic: Basic, target: Option<&str>, role: Role) -> Resolved {
    let mut r = tasks_resolved();
    let mut f = field("x", basic);
    f.target = target.map(str::to_owned);
    r.pages = vec![PageIn {
        elem: "pg".into(),
        page: "p".into(),
        seq: 0,
        frame: "form".into(),
        places: vec![PlaceIn {
            elem: "pl".into(),
            place: "f".into(),
            seq: 0,
            block: "probe-field".into(),
            source: None,
            bindings: vec![],
        }],
    }];
    r.pages[0].places[0].bindings.push(BindingIn {
        elem: "b".into(),
        slot: "any".into(),
        nth: 0,
        kind: BindKind::Field {
            field: f,
            role,
            schemes: Some(vec!["https".into()]),
        },
    });
    r
}

fn block_of(plan: &Plan) -> (String, InputState) {
    // probe-field has no slots: the slot check locks the place, so read the fill from a catalog that accepts the slot.
    match &plan.place("p", "f").unwrap().status {
        PlaceStatus::Ready(r) => match &r.slots[0].fill {
            Fill::Field(f) => (f.block.clone(), f.input.clone()),
            _ => panic!("not a field"),
        },
        PlaceStatus::Locked(l) => panic!("locked: {l:?}"),
    }
}

fn catalog_with_any() -> TestCatalog {
    let mut c = TestCatalog::new();
    c.blocks
        .iter_mut()
        .find(|b| b.name == "probe-field")
        .unwrap()
        .slots
        .push(SlotSpec {
            name: "any".into(),
            min: 0,
            max: None,
        });
    c
}

#[test]
fn table_per_basic_and_role() {
    let table = [
        (Basic::Text, None, "field-text"),
        (Basic::Email, None, "field-email"),
        (Basic::DateTime, None, "field-date-time"),
        (Basic::Date, None, "field-date"),
        (Basic::Choice, None, "field-choice"),
        (Basic::Keywords, None, "field-keywords"),
        (Basic::Reference, Some("Person"), "person-picker"),
        (Basic::Reference, Some("Task"), "field-reference"),
        (Basic::Unknown, None, "field-text"),
    ];
    for role in [Role::Input, Role::Display, Role::Compact, Role::Chip] {
        for (basic, target, want) in table {
            let r = one_field_resolved(basic, target, role);
            let plan = build_with(&r, &catalog_with_any(), &tasks_facts(), host());
            let (block, input) = block_of(&plan);
            assert_eq!(block, want, "{basic:?} {role:?}");
            if basic == Basic::Unknown && role == Role::Input {
                assert!(
                    matches!(input, InputState::Locked(_)),
                    "unknown input stays locked"
                );
            }
        }
    }
}

#[test]
fn tasks_board_choices() {
    let plan = tasks_plan();
    let PlaceStatus::Ready(r) = &plan.place("board", "main").unwrap().status else {
        panic!("board locked")
    };
    let blocks: Vec<String> = r
        .slots
        .iter()
        .filter_map(|s| {
            if let Fill::Field(f) = &s.fill {
                Some(f.block.clone())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        blocks,
        [
            "field-text",
            "field-date-time",
            "field-choice",
            "person-picker",
            "field-keywords"
        ]
    );
}

#[test]
fn unbuilt_field_block_locks_the_slot_only() {
    let mut cat = TestCatalog::new();
    cat.unbuilt.push("field-keywords".into());
    let plan = build_with(&tasks_resolved(), &cat, &tasks_facts(), host());
    let PlaceStatus::Ready(r) = &plan.place("board", "main").unwrap().status else {
        panic!("place must stay drawn")
    };
    let l = r.slot_locks();
    assert_eq!(l.len(), 1);
    assert_eq!(code_of(l[0]), "bind-error-field-component-missing");
}

#[test]
fn style_changes_nothing_but_the_echo_and_builds_are_equal() {
    let r = tasks_resolved();
    let cat = TestCatalog::new();
    let base = build_with(&r, &cat, &tasks_facts(), host());
    assert_eq!(base, build_with(&r, &cat, &tasks_facts(), host()));
    for style in ["fiori", "apple", "windows", "simple"] {
        let mut s = settings();
        s.style = Some(style.into());
        let p = Bind::build(
            &r,
            &cat,
            &Providers {
                records: &tasks_facts(),
                settings: &s,
                host: host(),
            },
        );
        assert_eq!(p.canonical_without_style(), base.canonical_without_style());
        assert_eq!(p.style.as_deref(), Some(style));
    }
}
