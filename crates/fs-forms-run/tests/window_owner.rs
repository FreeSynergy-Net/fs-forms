//! Conformance of the `WindowOwner` contract (S6-2), run against the double `FakeWindows` (counters instead of windows).
//!
//! (W2b, W7, W7b are in `window_owner_place.rs`.) Tests: W1 the same description, only `mode` differs, gives the same child composition; W2 `open` of an id that is open returns
//! that id and makes no second frontend; W3 `close` lets records and held connection fall; W4 an unknown or refused frontend is an
//! error with its code (locked, never empty); W5 a call in the child reaches the records of the child only; W6 the parent closed
//! closes its children. What hangs on `fs_rpc::hold` (a real attach and detach on a module socket) is NOT built here: the double
//! counts it through `FakeHold`; the real one is checked when `fs_rpc::hold` exists (S6-5).
//!
//! Mutants (each makes the named test fail): M1 the mode changes the composition -> W1; M2 `open` of an open id makes a new entry ->
//! W2; M3 `close` keeps the entry or its hold -> W3; M4 an unknown key opens an empty frontend -> W4; M5 calls go to the parent
//! records -> W5; M6 `close` leaves the children -> W6.

mod window_owner_common;

use fs_forms_run::{OpenFrontend, Placement, WindowId, WindowOwner};
use window_owner_common::{child, elem, fake, key, root, settings, tasks, view};

#[test]
fn the_id_is_program_slash_elem_and_stable() {
    let owner = fake();
    let r = root(&owner);
    assert_eq!(r, WindowId("net.freesynergy.Tasks/board".into()));
    let c = child(&owner, &r, "settings-embed", Placement::Panel);
    assert_eq!(c, WindowId("net.freesynergy.Tasks/settings-embed".into()));
    // Another owner, the same request: the same id (never a counter).
    let other = fake();
    let r2 = root(&other);
    assert_eq!(child(&other, &r2, "settings-embed", Placement::Panel), c);
}

#[test]
fn w1_only_the_mode_differs_the_child_composition_is_the_same() {
    let owner = fake();
    let r = root(&owner);
    let panel = child(&owner, &r, "settings-embed", Placement::Panel);
    let window = child(&owner, &r, "settings-window", Placement::Window);
    assert_ne!(panel, window);
    let a = owner.composition_of(&panel).unwrap();
    let b = owner.composition_of(&window).unwrap();
    assert_eq!(a, b);
    let modes: Vec<_> = owner
        .open_now()
        .into_iter()
        .map(|o| (o.id, o.mode))
        .collect();
    assert!(modes.contains(&(panel, Placement::Panel)));
    assert!(modes.contains(&(window, Placement::Window)));
}

#[test]
fn w2_open_of_an_open_id_returns_it_and_makes_no_second() {
    let owner = fake();
    let r = root(&owner);
    let first = child(&owner, &r, "settings-embed", Placement::Panel);
    let before = owner.open_now().len();
    let (attached, _) = owner.holds();
    // The circle: the same node asked again, even with another mode.
    let again = child(&owner, &r, "settings-embed", Placement::Window);
    assert_eq!(first, again);
    assert_eq!(owner.open_now().len(), before);
    assert_eq!(owner.holds().0, attached, "no second connection");
    // The first mode stays.
    let kept = owner
        .open_now()
        .into_iter()
        .find(|o| o.id == first)
        .unwrap();
    assert_eq!(kept.mode, Placement::Panel);
}

#[test]
fn w3_close_lets_records_and_the_hold_fall() {
    let owner = fake();
    let r = root(&owner);
    let c = child(&owner, &r, "settings-embed", Placement::Window);
    let records = owner.records_of(&c).unwrap();
    assert!(records.upgrade().is_some());
    assert_eq!(owner.holds(), (2, 0));
    owner.close(&c).unwrap();
    assert!(records.upgrade().is_none(), "no strong holder is left");
    assert_eq!(
        owner.holds(),
        (2, 1),
        "one connection fell, the parent's stays"
    );
    assert!(owner.open_now().iter().all(|o| o.id != c));
    assert!(owner.composition_of(&c).is_none());
    // Closing again says so, with its code.
    assert_eq!(
        owner.close(&c).unwrap_err().code,
        "host-notice-window-unknown"
    );
}

#[test]
fn w4_an_unknown_or_refused_frontend_is_an_error_with_a_code_never_empty() {
    let owner = fake().refusing(key("net.freesynergy.Calendar", "month"));
    let r = root(&owner);
    let missing = owner
        .open(OpenFrontend {
            key: key("net.freesynergy.Nothing", "none"),
            mode: Placement::Panel,
            parent: Some((r.clone(), elem("nothing"))),
            switchable: true,
            scope: None,
        })
        .unwrap_err();
    assert_eq!(missing.code, "host-notice-frontend-missing");
    assert!(
        missing
            .params
            .contains(&("extends".into(), "net.freesynergy.Nothing".into()))
    );
    assert!(missing.params.contains(&("frontend".into(), "none".into())));
    let unknown_parent = owner
        .open(OpenFrontend {
            key: settings(),
            mode: Placement::Panel,
            parent: Some((WindowId("net.freesynergy.Tasks/gone".into()), elem("x"))),
            switchable: true,
            scope: None,
        })
        .unwrap_err();
    assert_eq!(unknown_parent.code, "host-notice-window-unknown");
    // Nothing was opened by the failures, and no connection was made for them.
    assert_eq!(owner.open_now().len(), 1);
    assert_eq!(owner.holds(), (1, 0));
}

#[test]
fn w4b_a_refusing_module_is_said_as_refused() {
    let calendar = key("net.freesynergy.Calendar", "month");
    let owner = fake()
        .knowing(calendar.clone(), view("Calendar", "board"))
        .refusing(calendar.clone());
    let r = root(&owner);
    let e = owner
        .open(OpenFrontend {
            key: calendar,
            mode: Placement::Panel,
            parent: Some((r, elem("cal"))),
            switchable: true,
            scope: None,
        })
        .unwrap_err();
    assert_eq!(e.code, "host-notice-frontend-refused");
    assert_eq!(owner.holds(), (1, 0), "a refused module is never attached");
}

#[test]
fn w5_a_call_in_the_child_reaches_the_child_records_only() {
    let owner = fake();
    let r = root(&owner);
    let c = child(&owner, &r, "settings-embed", Placement::Panel);
    let parent_records = owner.records_of(&r).unwrap().upgrade().unwrap();
    let child_records = owner.records_of(&c).unwrap().upgrade().unwrap();
    assert!(owner.invoke(&c, "values", "value.set").is_some());
    assert_eq!(child_records.calls(), 1);
    assert_eq!(parent_records.calls(), 0, "the parent never sees the call");
    assert!(owner.invoke(&WindowId("x/y".into()), "f", "c").is_none());
}

#[test]
fn w6_the_parent_closed_closes_its_children() {
    let owner = fake();
    let r = root(&owner);
    let a = child(&owner, &r, "settings-embed", Placement::Panel);
    let b = child(&owner, &r, "settings-window", Placement::Window);
    // A grandchild: a frontend inside the child.
    let g = owner
        .open(OpenFrontend {
            key: tasks(),
            mode: Placement::Panel,
            parent: Some((a.clone(), elem("tasks-again"))),
            switchable: true,
            scope: Some("urn:uuid:1".into()),
        })
        .unwrap();
    let weak: Vec<_> = [&r, &a, &b, &g]
        .iter()
        .map(|i| owner.records_of(i).unwrap())
        .collect();
    assert_eq!(owner.holds(), (4, 0));
    owner.close(&r).unwrap();
    assert_eq!(owner.open_now(), Vec::new());
    assert!(weak.iter().all(|w| w.upgrade().is_none()));
    assert_eq!(owner.holds(), (4, 4), "every connection fell");
}
