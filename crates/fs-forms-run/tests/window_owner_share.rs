//! GA1 to GA6 on the double `FakeWindows`: the frontends of one record share the read cache, each has its OWN hold and watches itself; the
//! cache falls with the last frontend; a neighbour window learns of a change only through its own `watch`; the second window writing with an
//! old revision gets `conflict`. (The hub itself is checked in `record_hub.rs`.)
//!
//! Mutants (each makes the named test fail): F3 the switch keeps the old key -> `GA4f`; F4 the switch opens a new connection instead of
//! keeping the hold -> `GA4f`; the mutants of `record_hub.rs` (key without scope, `attach` always opens, leaked `Arc`, no broadcast,
//! revision ignored) also turn the tests here red.

mod window_owner_common;

use std::sync::Weak;

use fs_components_kanban::Answer;
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
fn ga1f_two_views_of_one_record_share_the_cache_and_have_two_holds() {
    let owner = fake();
    let a = id(at(&owner, "urn:a", WhenOpen::Window));
    let b = second_view(&owner, &a, "urn:a");
    assert_ne!(a, b);
    assert_eq!(owner.holds(), (2, 0), "two views, two holds");
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
            OpenFrontend::new(tasks(), Placement::InFrame)
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
    assert_eq!(owner.holds(), (2, 1), "the other view is still attached");
    owner.close(&a).unwrap();
    assert!(weak.upgrade().is_none(), "no strong holder is left");
    assert_eq!(owner.holds(), (2, 2));
    assert_eq!(owner.records_alive(), 0);
}

#[test]
fn ga4f_switched_keeps_the_hold_and_moves_the_view_to_the_new_record() {
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
    assert_eq!(owner.holds(), (1, 0), "the hold stays: the grip stays");
}

#[test]
fn ga5f_the_neighbour_window_is_told_by_its_own_watch_and_both_are_told() {
    let owner = fake();
    let a = id(at(&owner, "urn:a", WhenOpen::Window));
    let b = second_view(&owner, &a, "urn:a");
    owner.invoke(&a, "values", "value.set").unwrap();
    assert_eq!(
        (owner.generation_of(&a), owner.generation_of(&b)),
        (Some(1), Some(1))
    );
    let rec = owner.records_of(&a).unwrap().upgrade().unwrap();
    assert_eq!(rec.rereads(), 1, "two holds were told, the cache read once");
}

#[test]
fn ga6f_two_windows_one_record_the_second_writes_with_an_old_revision() {
    let owner = fake();
    let a = id(at(&owner, "urn:a", WhenOpen::Window));
    let b = second_view(&owner, &a, "urn:a");
    assert_eq!(
        owner.invoke(&a, "values", "value.set"),
        Some(Answer::Accepted)
    );
    assert_eq!(
        owner.invoke(&b, "values", "value.set"),
        Some(Answer::Refused("conflict".into())),
        "two instances of one person are two participants"
    );
    let _ = owner.grip_of(&b).unwrap().records().forms(); // b saw the new state
    assert_eq!(
        owner.invoke(&b, "values", "value.set"),
        Some(Answer::Accepted)
    );
}
