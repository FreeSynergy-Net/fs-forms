//! H-K13 (menu tree and shortcuts) and H-K15 (end to end on the hand-made tasks board).
mod common;
use common::*;
use fs_forms_bind::testing::*;
use fs_forms_bind::*;

fn plan_with_menu(extra: Vec<MenuIn>) -> Plan {
    let mut r = tasks_resolved();
    r.menu.extend(extra);
    build_with(&r, &TestCatalog::new(), &tasks_facts(), host())
}

fn entry(elem: &str, name: &str, key: &str, on_record: bool) -> MenuIn {
    MenuIn {
        elem: elem.into(),
        kind: MenuKind::Command {
            command: cmd(name, true, false, on_record),
            on_selected: on_record,
            shortcut: Some(key.into()),
        },
        children: vec![],
    }
}

#[test]
fn outline_follows_the_description() {
    let want = [
        "standard:program",
        "  settings",
        "  language",
        "  quit [ctrl+q]",
        "menu-task",
        "  create [ctrl+n]",
        "  claim",
        "  release",
        "  note",
        "  delete [delete]",
        "menu-view",
        "  view:board",
        "standard:help",
        "  help [f1]",
        "  about",
    ];
    assert_eq!(tasks_plan().menu.outline(), want);
}

#[test]
fn duplicate_shortcut_conflicts_and_keeps_both() {
    let p = plan_with_menu(vec![entry("m-x", "other", "Ctrl+N", false)]);
    let l: Vec<_> = p
        .menu
        .locks
        .iter()
        .filter(|l| code_of(l) == "bind-error-shortcut-conflict")
        .collect();
    assert_eq!(l.len(), 1);
    assert_eq!(l[0].cause.coded().param("shortcut"), Some("ctrl+n"));
    assert_eq!(
        p.menu
            .commands()
            .iter()
            .filter(|c| c.name == "create" || c.name == "other")
            .count(),
        2
    );
    assert_eq!(
        p.keys
            .keymap_for(&p.menu, None)
            .dispatch("ctrl+n", Focus::Global),
        Outcome::Conflict
    );
}

#[test]
fn reserved_and_invalid_keys_are_refused_visibly() {
    let p = plan_with_menu(vec![
        entry("m-t", "t1", "tab", false),
        entry("m-i", "t2", "ctrl+", false),
    ]);
    let codes: Vec<String> = p.menu.locks.iter().map(code_of).collect();
    assert!(codes.contains(&"bind-error-key-reserved".to_owned()));
    assert!(codes.contains(&"bind-error-shortcut-invalid".to_owned()));
    let km = p.keys.keymap_for(&p.menu, None);
    assert_eq!(km.dispatch("tab", Focus::Global), Outcome::NoMatch);
}

#[test]
fn delete_runs_in_a_list_never_in_a_text_field() {
    let p = tasks_plan();
    let km = p.keys.keymap_for(&p.menu, Some(&view(&["delete"])));
    assert_eq!(km.dispatch("delete", Focus::TextField), Outcome::NoMatch);
    assert_eq!(
        km.dispatch("delete", Focus::List),
        Outcome::Run("0/delete".into())
    );
}

#[test]
fn standard_menu_twice_is_a_lock() {
    let p = plan_with_menu(vec![MenuIn {
        elem: "m-p2".into(),
        kind: MenuKind::Standard("program".into()),
        children: vec![],
    }]);
    assert!(
        p.menu
            .locks
            .iter()
            .any(|l| code_of(l) == "bind-error-standard-twice")
    );
}

#[test]
fn end_to_end_board_is_ready_and_ordered() {
    let r = tasks_resolved();
    let p = tasks_plan();
    assert_eq!(p.state, PlanState::Ready, "{:?}", p.locks());
    let PlaceStatus::Ready(ready) = &p.place("board", "main").unwrap().status else {
        panic!()
    };
    let want: Vec<(String, u32)> = r.pages[0].places[0]
        .bindings
        .iter()
        .map(|b| (b.slot.clone(), b.nth))
        .collect();
    let got: Vec<(String, u32)> = ready
        .bindings()
        .into_iter()
        .map(|b| (b.slot, b.nth))
        .collect();
    assert_eq!(got, want);
    assert!(got.contains(&("card-facts".to_owned(), 3)));
    assert!(got.contains(&("card-menu".to_owned(), 3)));
}

#[test]
fn problems_are_never_swallowed() {
    let mut r = tasks_resolved();
    r.problems.push(Problem {
        elem: Some("gone".into()),
        code: "layout-error-x".into(),
        params: vec![],
    });
    r.problems.push(Problem {
        elem: None,
        code: "layout-error-y".into(),
        params: vec![],
    });
    r.problems.push(Problem {
        elem: Some("pl-board-main".into()),
        code: "layout-error-z".into(),
        params: vec![],
    });
    let p = build_with(&r, &TestCatalog::new(), &tasks_facts(), host());
    let n: Vec<String> = p
        .notices
        .iter()
        .map(|n| n.cause.coded().code.clone())
        .collect();
    assert!(
        n.contains(&"bind-error-elem-orphan".to_owned())
            && n.contains(&"layout-error-y".to_owned())
    );
    assert_eq!(code_of(&locks_of(&p, "board", "main")[0]), "layout-error-z");
}
