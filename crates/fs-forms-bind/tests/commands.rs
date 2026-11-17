//! H-K5: one decision for menu, context menu, button and key.
mod common;
use common::*;
use fs_forms_bind::testing::*;
use fs_forms_bind::*;

fn menu_with(names: &[(&str, bool, bool, bool, Option<&str>)]) -> Resolved {
    let mut r = tasks_resolved();
    r.menu = names
        .iter()
        .enumerate()
        .map(|(i, (n, supported, writes, on_record, key))| MenuIn {
            elem: format!("m{i}"),
            kind: MenuKind::Command {
                command: cmd(n, *supported, *writes, *on_record),
                on_selected: *on_record,
                shortcut: key.map(str::to_owned),
            },
            children: vec![],
        })
        .collect();
    r
}

fn surfaces(
    plan: &Plan,
    name: &str,
    sel: Option<&RecordView>,
    key: &str,
) -> (Option<Shown>, Shown, Outcome) {
    let arr = plan.menu.arrangement(sel);
    let id = format!("0/{name}");
    let menu = arr.iter().find(|a| a.id == id).map(|a| a.shown.clone());
    let c = plan
        .menu
        .commands()
        .into_iter()
        .find(|c| c.name == name)
        .unwrap();
    let direct = command_state(c, sel);
    let out = plan
        .keys
        .keymap_for(&plan.menu, sel, Platform::Linux)
        .dispatch(&press(key), FocusContext::List);
    (menu, direct, out)
}

#[test]
fn table_on_all_surfaces() {
    let r = menu_with(&[
        ("claim", true, false, true, Some("f2")),
        ("release", true, false, true, Some("f3")),
        ("setColumns", false, false, false, Some("f4")),
        ("note", true, false, true, Some("f5")),
        ("delete", true, true, true, Some("f7")),
    ]);
    let plan = build_with(
        &r,
        &TestCatalog::new(),
        &tasks_facts(),
        HostCaps {
            write: false,
            workflow: false,
        },
    );
    let sel = view(&["claim", "delete"]);
    // claim: enabled everywhere.
    let (m, d, k) = surfaces(&plan, "claim", Some(&sel), "f2");
    assert_eq!(
        (m, d, k),
        (
            Some(Shown::Enabled),
            Shown::Enabled,
            Outcome::Run(CommandId::new("0/claim"))
        )
    );
    // release: not in may -> absent on every surface, key does nothing.
    let (m, d, k) = surfaces(&plan, "release", Some(&sel), "f3");
    assert_eq!((m, d, k), (None, Shown::Hidden, Outcome::NoMatch));
    // setColumns: visible "coming", key disabled.
    let (m, d, k) = surfaces(&plan, "setColumns", Some(&sel), "f4");
    assert_eq!((m, d), (Some(Shown::Coming), Shown::Coming));
    assert!(matches!(k, Outcome::Disabled { .. }));
    // note: no selection -> locked, visible.
    let (m, d, _) = surfaces(&plan, "note", None, "f5");
    assert!(matches!(&d, Shown::Locked(l) if code_of(l) == "bind-error-no-selection"));
    assert_eq!(m, Some(d));
    // delete with write = false -> write-later.
    let (m, d, _) = surfaces(&plan, "delete", Some(&sel), "f7");
    assert!(matches!(&d, Shown::Locked(l) if code_of(l) == "bind-error-write-later"));
    assert_eq!(m, Some(d));
}

#[test]
fn right_comes_first_then_coming() {
    let r = menu_with(&[("setColumns", false, false, true, None)]);
    let plan = build_with(&r, &TestCatalog::new(), &tasks_facts(), host());
    let c = plan.menu.commands()[0];
    assert_eq!(
        command_state(c, Some(&view(&[]))),
        Shown::Hidden,
        "no right: invisible even though it is coming"
    );
    assert_eq!(
        command_state(c, Some(&view(&["setColumns"]))),
        Shown::Coming,
        "right but unsupported: coming"
    );
}

#[test]
fn menu_of_only_hidden_entries_goes_but_source_locked_stays() {
    let plan = tasks_plan();
    let sel = view(&["claim"]);
    assert!(
        plan.menu
            .visible_roots(Some(&sel))
            .contains(&"menu-task".to_owned())
    );
    let mut r = tasks_resolved();
    r.menu[1].children.retain(|c| c.elem != "m-create");
    let plan = build_with(&r, &TestCatalog::new(), &tasks_facts(), host());
    assert!(
        !plan
            .menu
            .visible_roots(Some(&view(&[])))
            .contains(&"menu-task".to_owned()),
        "all hidden by right"
    );
    let locked = build_with(&r, &TestCatalog::new(), &FakeFacts(vec![]), host());
    assert!(
        locked
            .menu
            .visible_roots(None)
            .contains(&"menu-task".to_owned()),
        "no selection: entries locked, menu stays"
    );
}

#[test]
fn workflow_target_is_locked_until_the_runtime_exists() {
    let mut c = cmd("remind", true, false, false);
    c.target = Target::Workflow("remind-all".into());
    let p = command_plan(
        &c,
        HostCaps {
            write: true,
            workflow: false,
        },
        None,
    );
    assert_eq!(
        code_of(p.fixed.as_ref().unwrap()),
        "bind-error-workflow-runtime-missing"
    );
    let p = command_plan(
        &c,
        HostCaps {
            write: true,
            workflow: true,
        },
        None,
    );
    assert!(p.fixed.is_none());
}

#[test]
fn type_command_is_hidden_only_when_the_type_right_says_no() {
    let mut c = cmd("create", true, true, false);
    let mut mk = |t: Option<bool>| {
        c.desc.type_may = t;
        command_plan(&c, host(), None)
    };
    assert_eq!(command_state(&mk(Some(false)), None), Shown::Hidden);
    assert_eq!(command_state(&mk(Some(true)), None), Shown::Enabled);
    assert_eq!(
        command_state(&mk(None), None),
        Shown::Enabled,
        "unknown: shown, the provider answers forbidden"
    );
}
