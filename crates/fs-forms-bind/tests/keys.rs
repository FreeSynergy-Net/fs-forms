//! Reserved keys (c0 18.11.: Primary+Q, F1, F6, F10, Escape, Tab), `delete` as an ordinary list-only command (R4), conflicts
//! with the standard shortcuts of the shell.
mod common;
use common::*;
use fs_forms_bind::testing::*;
use fs_forms_bind::*;

fn menu_cmd(elem: &str, name: &str, key: &str) -> MenuIn {
    MenuIn {
        elem: elem.into(),
        kind: MenuKind::Command {
            command: cmd(name, true, false, false),
            on_selected: false,
            shortcut: Some(key.into()),
        },
        children: vec![],
    }
}

fn plan_with_key(key: &str) -> Plan {
    let mut r = tasks_resolved();
    r.menu = vec![menu_cmd("m-x", "other", key)];
    build_with(&r, &TestCatalog::new(), &tasks_facts(), host())
}

#[test]
fn every_global_key_is_reserved_and_the_entry_loses_its_key() {
    for key in [
        "F6",
        "Shift+F6",
        "F1",
        "F10",
        "Escape",
        "Primary+Q",
        "Tab",
        "Shift+Tab",
    ] {
        let p = plan_with_key(key);
        let codes: Vec<String> = p.menu.locks.iter().map(code_of).collect();
        assert_eq!(codes, ["bind-error-key-reserved"], "{key}");
        let MenuEntry::Command { shortcut, .. } = &p.menu.roots[0].entry else {
            panic!()
        };
        assert!(shortcut.is_none(), "{key} dropped");
    }
    assert_eq!(plan_with_key("F7").menu.locks, vec![]);
    assert_eq!(
        plan_with_key("Delete").menu.locks,
        vec![],
        "delete is an ordinary command, not reserved"
    );
}

/// R4: `delete` runs only in a list: not global, not in a tree, not in a text field (kills H-M13e).
#[test]
fn r4_delete_runs_only_in_a_list() {
    let p = tasks_plan();
    let km = p
        .keys
        .keymap_for(&p.menu, Some(&view(&["delete"])), Platform::Linux);
    assert_eq!(
        km.dispatch(&press("Delete"), FocusContext::Global),
        Outcome::NoMatch
    );
    assert_eq!(
        km.dispatch(&press("Delete"), FocusContext::Tree),
        Outcome::NoMatch
    );
    assert_eq!(
        km.dispatch(&press("Delete"), FocusContext::TextField),
        Outcome::NoMatch
    );
    assert_eq!(
        km.dispatch(&press("Delete"), FocusContext::List),
        Outcome::Run(CommandId::new("0/delete"))
    );
}

/// O19: a command bound to a shortcut of a standard entry of the shell is a visible conflict.
#[test]
fn conflict_with_a_standard_shortcut_is_visible() {
    let std_menus = StandardMenus {
        menus: vec![(
            "program".into(),
            vec![StdEntry {
                name: "settings".into(),
                shortcut: Some("Ctrl+,".into()),
            }],
        )],
    };
    let mut r = tasks_resolved();
    r.menu = vec![
        MenuIn {
            elem: "m-prog".into(),
            kind: MenuKind::Standard("program".into()),
            children: vec![],
        },
        menu_cmd("m-x", "other", "Ctrl+,"),
    ];
    let p = build_custom(&r, &settings(), &std_menus);
    let l: Vec<_> = p
        .menu
        .locks
        .iter()
        .filter(|l| code_of(l) == "bind-error-shortcut-conflict")
        .collect();
    assert_eq!(l.len(), 1, "{:?}", p.menu.locks);
    assert!(
        l[0].cause
            .coded()
            .param("commands")
            .unwrap()
            .contains("app.settings")
    );
}

/// K6: two commands on one key, one of them hidden for this selection: NO key runs, the clash stays a coded problem.
#[test]
fn a_clash_with_a_hidden_command_runs_neither() {
    let mut r = tasks_resolved();
    let rec = |elem: &str, name: &str| MenuIn {
        elem: elem.into(),
        kind: MenuKind::Command {
            command: cmd(name, true, false, true),
            on_selected: true,
            shortcut: Some("ctrl+k".into()),
        },
        children: vec![],
    };
    r.menu = vec![rec("m-a", "alpha"), rec("m-b", "beta")];
    let p = build_with(&r, &TestCatalog::new(), &tasks_facts(), host());
    assert!(
        p.menu
            .locks
            .iter()
            .any(|l| code_of(l) == "bind-error-shortcut-conflict")
    );
    let sel = view(&["alpha"]);
    let out = p
        .keys
        .keymap_for(&p.menu, Some(&sel), Platform::Linux)
        .dispatch(&press("ctrl+k"), FocusContext::Global);
    assert!(!matches!(out, Outcome::Run(_)), "{out:?}");
    // beta alone does not clash with anything: its key stays NoMatch when hidden.
    r.menu.remove(0);
    let p = build_with(&r, &TestCatalog::new(), &tasks_facts(), host());
    let out = p
        .keys
        .keymap_for(&p.menu, Some(&view(&[])), Platform::Linux)
        .dispatch(&press("ctrl+k"), FocusContext::Global);
    assert_eq!(out, Outcome::NoMatch);
}

fn reserved_text(t: &str) -> bool {
    reserved(&Shortcut::parse(t).expect("a readable key"))
}

/// P3 + Q11 (c0): exactly the listed keys without further modifiers; Shift+Tab and Shift+F6 too; Shift+F1, Ctrl+F6 and the
/// like are free.
#[test]
fn p3_reserved_spellings_and_free_modified_keys() {
    for t in [
        "ctrl+q",
        "Primary+Q",
        "f1",
        "F10",
        "escape",
        "F6",
        "Shift+F6",
        "tab",
        "Shift+Tab",
    ] {
        assert!(reserved_text(t), "{t}");
    }
    for t in [
        "Ctrl+Escape",
        "Ctrl+F1",
        "Ctrl+F10",
        "Ctrl+Tab",
        "Ctrl+F6",
        "Shift+F1",
        "Shift+F10",
        "Shift+Escape",
        "Alt+F1",
        "Ctrl+Shift+Q",
        "Alt+Q",
        "F7",
        "Delete",
        "ctrl+n",
    ] {
        assert!(!reserved_text(t), "{t}");
    }
}

/// P4: the stored list and the function say the same (one list); every stored key parses and is reserved.
#[test]
fn p4_stored_reserved_list_equals_the_function() {
    let names = KeyPlan::default().reserved;
    assert_eq!(names.len(), 8);
    assert_eq!(
        names,
        RESERVED.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>()
    );
    for n in &names {
        assert!(reserved_text(n), "{n}");
    }
}
