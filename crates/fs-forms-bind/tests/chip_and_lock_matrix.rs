//! Chip and lock matrix: every combination of placement, field, layout, source, component and chip locks stays visible, and reserved menu keys follow the platform (Primary).
mod common;
use common::*;
use fs_forms_bind::testing::*;
use fs_forms_bind::*;

fn count(p: &Plan, code: &str) -> usize {
    all_codes(p).iter().filter(|c| *c == code).count()
}

/// B5: placement lock x field lock x layout lock x source missing x component missing x chip, all 32 combinations: nothing silent.
#[test]
fn b5_every_combination_is_visible() {
    for bits in 0u32..32 {
        let (place, field_lock, layout, no_source, chip) = (
            bits & 1 != 0,
            bits & 2 != 0,
            bits & 4 != 0,
            bits & 8 != 0,
            bits & 16 != 0,
        );
        let mut r = tasks_resolved();
        r.pages[0].places[0].block_mut().unwrap().bindings[1].kind = BindKind::Condition {
            path: vec!["done".into()],
            basic: Basic::Bool,
        };
        if field_lock {
            r.pages[0].places[0].block_mut().unwrap().bindings[2].kind = BindKind::Field {
                field: field("due", Basic::Unknown),
                role: Role::Compact,
                schemes: None,
            };
        }
        if layout {
            r.problems
                .push(problem("b-card-title-0", "layout-error-b5"));
        }
        let mut cat = TestCatalog::new();
        if place {
            cat.unbuilt.push("board".into());
        }
        if chip {
            cat.unbuilt.push("field-bool".into());
        }
        let facts = if no_source {
            FakeFacts(vec![])
        } else {
            tasks_facts()
        };
        let p = build_with(&r, &cat, &facts, host());
        let c = all_codes(&p);
        let want = usize::from(field_lock) + usize::from(chip);
        assert_eq!(
            count(&p, "bind-error-field-component-missing"),
            want,
            "bits {bits:05b}: {c:?}"
        );
        assert_eq!(
            c.contains(&"layout-error-b5".to_owned()),
            layout,
            "bits {bits:05b}: {c:?}"
        );
        assert_eq!(
            c.contains(&"bind-error-block-not-built".to_owned()),
            place,
            "bits {bits:05b}: {c:?}"
        );
        if no_source {
            assert!(
                c.contains(&"accepts-error-contract-missing".to_owned()),
                "bits {bits:05b}: {c:?}"
            );
        }
        if chip {
            assert!(
                p.locks()
                    .iter()
                    .any(|l| l.cause.coded().param("block") == Some("field-bool")
                        && l.cause.coded().param("role") == Some("chip")),
                "bits {bits:05b}: {c:?}"
            );
        }
        if bits == 0 {
            assert!(
                c.is_empty() || !c.iter().any(|x| x.starts_with("bind-error-field")),
                "{c:?}"
            );
        }
    }
}

/// B5: a missing chip alone locks the SLOT, not the whole placement.
#[test]
fn b5_chip_alone_locks_the_slot_only() {
    let mut r = tasks_resolved();
    r.pages[0].places[0].block_mut().unwrap().bindings[1].kind = BindKind::Condition {
        path: vec!["done".into()],
        basic: Basic::Bool,
    };
    let mut cat = TestCatalog::new();
    cat.unbuilt.push("field-bool".into());
    let p = build_with(&r, &cat, &tasks_facts(), host());
    let st = &p.pages[0].places[0].block().unwrap().status;
    assert!(matches!(st, PlaceStatus::Ready(_)), "{st:?}");
}

fn menu_key(key: &str, platform: Platform) -> Vec<String> {
    let mut r = tasks_resolved();
    r.menu = vec![MenuIn {
        elem: "m-k".into(),
        kind: MenuKind::Command {
            command: cmd("alpha", true, false, true),
            on_selected: true,
            shortcut: Some(key.into()),
        },
        children: vec![],
    }];
    let p = Bind::build(
        &r,
        &TestCatalog::new(),
        &Providers {
            records: &tasks_facts(),
            settings: &settings(),
            host: host(),
            platform,
            standard: &standard(),
        },
    );
    p.menu.locks.iter().map(code_of).collect()
}

/// K10 through the menu: the platform of the build decides.
#[test]
fn k10_menu_reserved_per_platform() {
    let res = |k: &str, p: Platform| menu_key(k, p).iter().any(|c| c.ends_with("key-reserved"));
    for p in [Platform::Linux, Platform::Windows, Platform::Mac] {
        assert!(res("Primary+Q", p), "{p:?}");
        assert!(res("Shift+Tab", p), "{p:?}");
        assert!(res("Shift+F6", p), "{p:?}");
        assert!(!res("Shift+F1", p), "{p:?}");
        assert!(!res("Ctrl+Meta+Q", p), "{p:?}");
    }
    assert!(res("Ctrl+Q", Platform::Linux));
    assert!(res("Ctrl+Q", Platform::Windows));
    assert!(!res("Ctrl+Q", Platform::Mac));
    assert!(res("Meta+Q", Platform::Mac));
    assert!(!res("Meta+Q", Platform::Linux));
    assert!(!res("Meta+Q", Platform::Windows));
}
