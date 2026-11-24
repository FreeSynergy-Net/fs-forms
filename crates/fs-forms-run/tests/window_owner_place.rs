//! Conformance of `WindowOwner::place` and the repeated `open` (S6-9, W2b, W7, W7b), against `FakeWindows`. Helpers: `window_owner_common`.

mod window_owner_common;

use fs_forms_run::{OpenFrontend, Placement, WindowId, WindowOwner};
use window_owner_common::{child, elem, fake, id, root, settings};

#[test]
fn w2b_opening_an_open_id_brings_it_forward_and_keeps_the_mode() {
    let owner = fake();
    let r = root(&owner);
    let c = child(&owner, &r, "settings-embed", Placement::InFrame);
    assert_eq!(owner.raised(&c), 0);
    child(&owner, &r, "settings-embed", Placement::Window);
    assert_eq!(owner.raised(&c), 1);
}

#[test]
fn w7_place_switches_only_the_place_the_form_state_stays() {
    let owner = fake();
    let r = root(&owner);
    let c = child(&owner, &r, "settings-embed", Placement::InFrame);
    let records = owner.records_of(&c).unwrap();
    owner.invoke(&c, "values", "value.set").unwrap();
    let before = owner.composition_of(&c).unwrap();
    let holds = owner.holds();
    owner.place(&c, Placement::Window).unwrap();
    // The same description, the same content; only the placement differs.
    assert_eq!(owner.composition_of(&c).unwrap(), before);
    let now = owner.open_now().into_iter().find(|o| o.id == c).unwrap();
    assert_eq!(now.mode, Placement::Window);
    // The state stays: the same records (with their call), no new connection, none dropped.
    assert_eq!(records.upgrade().unwrap().calls(), 1);
    assert_eq!(owner.holds(), holds);
    // And back; placing where it is already is no error.
    owner.place(&c, Placement::InFrame).unwrap();
    owner.place(&c, Placement::InFrame).unwrap();
    assert_eq!(owner.holds(), holds);
    assert_eq!(
        owner
            .place(&WindowId("a/b".into()), Placement::Window)
            .unwrap_err()
            .code,
        "host-notice-window-unknown"
    );
}

#[test]
fn w7b_a_locked_switch_is_refused_with_a_code_and_nothing_changes() {
    let owner = fake();
    let r = root(&owner);
    let c = id(owner
        .open(OpenFrontend::new(settings(), Placement::InFrame).with_parent(r, elem("fixed")))
        .unwrap());
    let e = owner.place(&c, Placement::Window).unwrap_err();
    assert_eq!(e.code, "host-notice-placement-locked");
    assert!(e.params.contains(&("id".into(), c.as_str().into())));
    let now = owner.open_now().into_iter().find(|o| o.id == c).unwrap();
    assert_eq!(now.mode, Placement::InFrame);
}
