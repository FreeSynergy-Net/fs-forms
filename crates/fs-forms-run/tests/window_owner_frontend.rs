//! `WindowOwner::use_frontend` (P2, NACHTRAG-2 C, GC1-GC3 and the two guards), against `FakeWindows`.
//!
//! Tests: GC1 the surface keeps id, record, records (the same `Weak`) and hold, the composition and the key change; GC2 a frontend of
//! another program and an unknown one are refused with their code and nothing changes; GC3 after the change the place is found by
//! its key (`open(list, A)` is `Raised`, not `New`; `decide` agrees) and a second open of the old name does not overwrite the id;
//! GC5 a frontend that already shows this record elsewhere is refused.
//!
//! Mutants (each run red, see BERICHT-s6-p12): M1 `use_frontend` makes a new hold -> GC1, `gc3c`; M2 it forgets to set the key -> GC1, GC3, `gc3b`, `gc3c`; M3 other program
//! is answered as unknown -> GC2; M4 roots are found by the id text -> GC3, `gc3c`; M5 no `id-taken` guard -> `gc3c`; M6 no `open-elsewhere` guard -> GC5.

mod window_owner_common;

use std::sync::Weak;

use fs_forms_run::{
    Decision, FakeWindows, OpenFrontend, Placement, WhenOpen, WindowId, WindowOpened, WindowOwner,
    decide,
};
use window_owner_common::{fake, key, tasks, view};

const A: &str = "urn:record:a";

fn two() -> FakeWindows {
    fake().knowing(
        key("net.freesynergy.Tasks", "list"),
        view("List", "field-card"),
    )
}

fn open_a(owner: &FakeWindows, frontend: &str) -> WindowId {
    let req = OpenFrontend::new(key("net.freesynergy.Tasks", frontend), Placement::Window)
        .with_scope(A)
        .with_when_open(WhenOpen::Window);
    match owner.open(req).unwrap() {
        WindowOpened::New(id) => id,
        o => panic!("not new: {o:?}"),
    }
}

fn code(e: &fs_forms_run::Coded) -> &str {
    &e.code
}

#[test]
fn gc1_the_surface_stays_only_the_frontend_changes() {
    let owner = two();
    let id = open_a(&owner, "board");
    let records: Weak<_> = owner.records_of(&id).unwrap();
    owner.invoke(&id, "values", "value.set").unwrap();
    let (holds, before) = (owner.holds(), owner.composition_of(&id).unwrap());
    owner.use_frontend(&id, "list").unwrap();
    let now = owner.open_now();
    assert_eq!(now.len(), 1);
    assert_eq!((&now[0].id, now[0].scope.as_deref()), (&id, Some(A)));
    assert_eq!(now[0].key, key("net.freesynergy.Tasks", "list"));
    assert!(Weak::ptr_eq(&records, &owner.records_of(&id).unwrap()));
    assert_eq!(records.upgrade().unwrap().calls(), 1);
    assert_eq!(owner.holds(), holds);
    assert_ne!(owner.composition_of(&id).unwrap(), before);
    // and back again: still the same surface
    owner.use_frontend(&id, "board").unwrap();
    assert_eq!(owner.composition_of(&id).unwrap(), before);
    assert_eq!(owner.holds(), holds);
}

#[test]
fn gc2_another_program_and_an_unknown_name_are_refused_and_nothing_changes() {
    let owner = two();
    let id = open_a(&owner, "board");
    let (holds, comp, now) = (
        owner.holds(),
        owner.composition_of(&id).unwrap(),
        owner.open_now(),
    );
    let e = owner.use_frontend(&id, "settings").unwrap_err();
    assert_eq!(code(&e), "host-notice-frontend-other-program");
    assert!(e.params.contains(&("frontend".into(), "settings".into())));
    let e = owner.use_frontend(&id, "nope").unwrap_err();
    assert_eq!(code(&e), "host-notice-frontend-unknown");
    assert!(
        e.params
            .contains(&("available".into(), "[board, list]".into())),
        "{e:?}"
    );
    let e = owner
        .use_frontend(&WindowId("a/b".into()), "list")
        .unwrap_err();
    assert_eq!(code(&e), "host-notice-window-unknown");
    assert_eq!(
        (
            owner.holds(),
            owner.composition_of(&id).unwrap(),
            owner.open_now()
        ),
        (holds, comp, now)
    );
}

#[test]
fn gc3_after_the_change_the_place_is_found_by_its_key() {
    let owner = two();
    let id = open_a(&owner, "board");
    owner.use_frontend(&id, "list").unwrap();
    let req =
        OpenFrontend::new(key("net.freesynergy.Tasks", "list"), Placement::Window).with_scope(A);
    assert_eq!(
        decide(&owner.open_now(), &req),
        Decision::Open(WhenOpen::Switch)
    );
    match owner.open(req).unwrap() {
        WindowOpened::Raised(r) => assert_eq!(r, id, "the id stays the name of the place"),
        o => panic!("not raised: {o:?}"),
    }
    assert_eq!(owner.raised(&id), 1);
    assert_eq!(owner.open_now().len(), 1);
}

#[test]
fn gc3b_another_record_at_the_changed_place_switches_and_keeps_the_name_of_the_place() {
    let owner = two();
    let id = open_a(&owner, "board");
    owner.use_frontend(&id, "list").unwrap();
    let req = OpenFrontend::new(key("net.freesynergy.Tasks", "list"), Placement::Window)
        .with_scope("urn:record:b")
        .with_when_open(WhenOpen::Switch);
    match owner.open(req).unwrap() {
        WindowOpened::Switched { id: now, from } => {
            assert_eq!(
                now,
                WindowId::of_record("net.freesynergy.Tasks", "board", Some("urn:record:b"))
            );
            assert_eq!(from.as_deref(), Some(A));
        }
        o => panic!("not switched: {o:?}"),
    }
    assert_eq!(owner.open_now().len(), 1);
}

#[test]
fn gc3c_the_old_name_is_never_taken_over_a_surface_that_still_carries_it() {
    let owner = two();
    let id = open_a(&owner, "board");
    owner.use_frontend(&id, "list").unwrap();
    // `board` + A would be the id of the surface that now shows `list`: refused, nothing is overwritten.
    let req = OpenFrontend::new(tasks(), Placement::Window)
        .with_scope(A)
        .with_when_open(WhenOpen::Window);
    let e = owner.open(req).unwrap_err();
    assert_eq!(code(&e), "host-notice-window-id-taken");
    assert_eq!(owner.open_now().len(), 1);
    assert_eq!(owner.holds(), (1, 0));
}

#[test]
fn gc5_a_frontend_that_already_shows_this_record_elsewhere_is_refused() {
    let owner = two();
    let board = open_a(&owner, "board");
    let list = open_a(&owner, "list");
    let e = owner.use_frontend(&board, "list").unwrap_err();
    assert_eq!(code(&e), "host-notice-frontend-open-elsewhere");
    assert!(e.params.contains(&("other".into(), list.as_str().into())));
    let now = owner.open_now();
    assert!(
        now.iter()
            .any(|o| o.id == board && o.key.frontend == "board")
    );
}
