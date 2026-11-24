//! GA1 to GA3 on the double `FakeWindows`: the frontends of one record share the records and the hold; the records fall with the last
//! frontend. (The hub itself is checked in `record_hub.rs`.)
//!
//! Mutants (each makes the named test fail): F3 the switch keeps the old grip -> `GA4f`; the mutants H1-H4 of `record_hub.rs` (key without
//! scope, `attach` always opens, the hub leaks a strong `Arc`, no generation count) also turn `GA1f`-`GA3f` red.

mod window_owner_common;

use std::sync::Weak;

use fs_forms_run::{OpenFrontend, Placement, WhenOpen, WindowId, WindowOpened, WindowOwner};
use window_owner_common::{elem, fake, id, tasks};

fn at(owner: &impl WindowOwner, scope: &str, when: WhenOpen) -> WindowOpened {
    owner
        .open(
            OpenFrontend::new(tasks(), Placement::Window)
                .with_scope(scope)
                .with_when_open(when),
        )
        .unwrap()
}

#[test]
fn ga1f_two_views_of_one_record_are_one_records_one_hold() {
    let owner = fake();
    let a = id(at(&owner, "urn:a", WhenOpen::Window));
    let b = second_view(&owner, &a, "urn:a");
    assert_ne!(a, b);
    assert_eq!(owner.holds(), (1, 0), "one hold for the one record");
    let ra = owner.records_of(&a).unwrap();
    assert!(Weak::ptr_eq(&ra, &owner.records_of(&b).unwrap()));
    let before = owner.generation_of(&b).unwrap();
    owner.invoke(&a, "values", "value.set").unwrap();
    assert_eq!(owner.generation_of(&b), Some(before + 1));
    assert_eq!(ra.upgrade().unwrap().rereads(), 1, "read again once");
}

/// A second view of the record `scope`: the same frontend as a child of `parent`, at a place of another name.
fn second_view(owner: &impl WindowOwner, parent: &WindowId, scope: &str) -> WindowId {
    id(owner
        .open(
            OpenFrontend::new(tasks(), Placement::Panel)
                .with_scope(scope)
                .with_parent(parent.clone(), elem("second-view")),
        )
        .unwrap())
}

#[test]
fn ga2f_another_scope_is_another_records() {
    let owner = fake();
    let a = id(at(&owner, "urn:a", WhenOpen::Window));
    let b = id(at(&owner, "urn:b", WhenOpen::Window));
    assert_ne!(a, b);
    assert!(!Weak::ptr_eq(
        &owner.records_of(&a).unwrap(),
        &owner.records_of(&b).unwrap()
    ));
    let gb = owner.generation_of(&b).unwrap();
    owner.invoke(&a, "values", "value.set").unwrap();
    assert_eq!(owner.generation_of(&b), Some(gb), "B is untouched");
    assert_eq!(owner.records_alive(), 2);
}

#[test]
fn ga3f_the_records_fall_with_the_last_view() {
    let owner = fake();
    let a = id(at(&owner, "urn:a", WhenOpen::Window));
    let b = second_view(&owner, &a, "urn:a");
    let weak = owner.records_of(&a).unwrap();
    owner.close(&b).unwrap();
    assert!(weak.upgrade().is_some());
    assert_eq!(owner.holds(), (1, 0), "still attached");
    owner.close(&a).unwrap();
    assert!(weak.upgrade().is_none(), "no strong holder is left");
    assert_eq!(owner.holds(), (1, 1));
    assert_eq!(owner.records_alive(), 0);
}

#[test]
fn ga4f_switched_moves_the_grip_to_the_records_of_the_new_record() {
    let owner = fake();
    let a = id(at(&owner, "urn:a", WhenOpen::Window));
    let old = owner.records_of(&a).unwrap();
    let r = at(&owner, "urn:b", WhenOpen::Switch);
    let b = id(r);
    let new = owner.records_of(&b).unwrap();
    assert!(!Weak::ptr_eq(&old, &new));
    assert!(
        old.upgrade().is_none(),
        "the old record is shown nowhere now"
    );
    assert_eq!(owner.records_alive(), 1);
    assert_eq!(
        owner.holds(),
        (2, 1),
        "the new hold is taken before the old one falls"
    );
}
