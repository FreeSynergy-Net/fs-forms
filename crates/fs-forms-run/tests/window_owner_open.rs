//! Conformance of the record in the id and of `open` for a place that is open already (O1, W8-W10), against `FakeWindows`.
//! Helpers: `window_owner_common`. The pure decision is in `window_owner_decide.rs`.
//!
//! Tests: W8 `of_record` (no scope = the old id; scope percent-encoded; two scopes never one id; reversible; proptest, G3);
//! W9 other record at an open place: `Window` = second id and second hold, `Switch` = one entry, the grip moves to the records of the new record (G1, P3);
//! W10 no scope twice = the old id, `Raised` (G2, the old ids never change).
//!
//! Mutants (each run red, see BERICHT-s6-o1): M7 `of_record` ignores the scope -> W8, W9; M8 `of_record` does not encode `/` -> W8; M9 `Switch` opens a new hold -> W9b (G1);
//! M11 a request without scope gets a suffix -> W10 (G2).

mod window_owner_common;

use fs_forms_run::{OpenFrontend, Placement, WhenOpen, WindowId, WindowOpened, WindowOwner};
use proptest::prelude::*;
use window_owner_common::{fake, id, root, tasks};

const P: &str = "net.freesynergy.Tasks";

fn at(
    owner: &fs_forms_run::FakeWindows,
    scope: &str,
    when: WhenOpen,
) -> Result<WindowOpened, fs_forms_run::Coded> {
    owner.open(
        OpenFrontend::new(tasks(), Placement::Window)
            .with_scope(scope)
            .with_when_open(when),
    )
}

#[test]
fn w8_the_id_without_a_scope_is_the_old_one_with_a_scope_it_is_encoded() {
    assert_eq!(
        WindowId::of_record(P, "board", None),
        WindowId::of(P, "board")
    );
    assert_eq!(
        WindowId::of_record(P, "board", Some("urn:uuid:1")).as_str(),
        "net.freesynergy.Tasks/board/urn%3Auuid%3A1"
    );
    // RFC 3986 unreserved stays, `/`, `#`, `%` and non-ASCII are encoded.
    let odd = WindowId::of_record(P, "board", Some("a-b.c_d~e/f#g%h\u{e4}"));
    assert_eq!(
        odd.as_str(),
        "net.freesynergy.Tasks/board/a-b.c_d~e%2Ff%23g%25h%C3%A4"
    );
    let place = WindowId::of(P, "board");
    assert_eq!(
        odd.scope_in(&place),
        Some(Some("a-b.c_d~e/f#g%h\u{e4}".into()))
    );
    assert_eq!(place.scope_in(&place), Some(None));
    assert_eq!(WindowId::of(P, "other").scope_in(&place), None);
}

proptest! {
    /// G3: two different scopes never give one id, and the id reads back to its scope.
    #[test]
    fn w8b_two_scopes_never_share_an_id_and_the_scope_reads_back(
        a in "\\PC{0,12}", b in "[a-z/#%\u{e4}\u{4e2d} :]{0,8}"
    ) {
        let place = WindowId::of(P, "board");
        let (ia, ib) = (WindowId::of_record(P, "board", Some(&a)), WindowId::of_record(P, "board", Some(&b)));
        prop_assert_eq!(ia == ib, a == b);
        prop_assert_eq!(ia.scope_in(&place), Some(Some(a.clone())));
        // a record is never the place itself
        prop_assert_ne!(&ia, &place);
    }
}

#[test]
fn w9_another_record_in_a_window_is_a_second_id_and_a_second_hold() {
    let owner = fake();
    let a = id(at(&owner, "urn:a", WhenOpen::Window).unwrap());
    let b = id(at(&owner, "urn:b", WhenOpen::Window).unwrap());
    assert_ne!(a, b);
    assert_eq!(owner.open_now().len(), 2);
    assert_eq!(owner.holds(), (2, 0));
    assert!(matches!(at(&owner, "urn:b", WhenOpen::Window), Ok(WindowOpened::Raised(i)) if i == b));
}

#[test]
fn w9b_switch_keeps_one_entry_and_moves_the_grip() {
    let owner = fake();
    let a = id(at(&owner, "urn:a", WhenOpen::Window).unwrap());
    let records = owner.records_of(&a).unwrap();
    let comp = owner.composition_of(&a).unwrap();
    owner.invoke(&a, "values", "value.set").unwrap();
    let r = at(&owner, "urn:b", WhenOpen::Switch).unwrap();
    let b = WindowId::of_record(P, "board", Some("urn:b"));
    assert_eq!(
        r,
        WindowOpened::Switched {
            id: b.clone(),
            from: Some("urn:a".into())
        }
    );
    // P3+: the grip moves to the new record and keeps its own connection: no second hold, none dropped
    assert_eq!(owner.holds(), (1, 0));
    assert_eq!(owner.open_now().len(), 1);
    let now = &owner.open_now()[0];
    assert_eq!((&now.id, now.scope.as_deref()), (&b, Some("urn:b")));
    assert!(owner.composition_of(&a).is_none());
    assert_eq!(owner.composition_of(&b).unwrap(), comp);
    // the records of the old record are shown nowhere now; the new ones count from zero
    assert!(records.upgrade().is_none());
    owner.invoke(&b, "values", "value.set").unwrap();
    assert_eq!(owner.records_of(&b).unwrap().upgrade().unwrap().calls(), 1);
    owner.close(&b).unwrap();
    assert_eq!(owner.holds(), (1, 1));
}

#[test]
fn w10_no_scope_twice_is_the_old_id_and_raised() {
    let owner = fake();
    let r = root(&owner);
    assert_eq!(r, WindowId("net.freesynergy.Tasks/board".into()));
    let again = owner
        .open(OpenFrontend::new(tasks(), Placement::Window))
        .unwrap();
    assert_eq!(again, WindowOpened::Raised(r.clone()));
    assert_eq!(owner.raised(&r), 1);
    assert_eq!((owner.open_now().len(), owner.holds()), (1, (1, 0)));
}
