//! The hub of the shared records (stage 6, P3 = A1 of NACHTRAG-2): two views of the same record share ONE `Records`, so after a change
//! of one the other shows the same; another record shares nothing; the last grip that falls lets go of everything.
//!
//! Tests: GA1 two grips of one (range, scope) are the same records, one hold, a change through one is a new generation and the new
//! state in the other; GA2 another scope is another records and another generation, a change in A leaves B alone; GA3 one of two
//! grips falls: still attached; the last falls: no strong holder, hold detached, the hub is empty. The same three on the double
//! `FakeWindows` in `window_owner_share.rs`.
//!
//! Mutants (each makes the named test fail): H1 the key leaves out the `scope` -> GA2; H2 `attach` always opens new records -> GA1;
//! H3 the hub leaks a strong `Arc` -> GA3; H4 an accepted change does not count the generation -> GA1, GA2.

use std::sync::Arc;

use fs_components_kanban::Answer;
use fs_forms_run::{RecordHub, RecordKey, Records, SampleBoard};

fn key(scope: Option<&str>) -> RecordKey {
    RecordKey {
        range: "net.freesynergy.Tasks".into(),
        scope: scope.map(str::to_owned),
    }
}

#[allow(clippy::unnecessary_wraps)] // the shape `attach` asks for
fn board() -> Result<Arc<dyn Records>, fs_forms_run::Coded> {
    Ok(Arc::new(SampleBoard::default()))
}

#[test]
fn ga1_two_views_of_one_record_share_one_records_and_show_the_change() {
    let hub = RecordHub::new();
    let a = hub.attach(key(Some("urn:a")), board).unwrap();
    let b = hub
        .attach(key(Some("urn:a")), || panic!("opened twice"))
        .unwrap();
    assert!(Arc::ptr_eq(&a.records(), &b.records()));
    let before = b.records().forms();
    assert_eq!((a.generation(), b.generation()), (0, 0));
    assert_eq!(a.records().move_card("FS-1", "done"), Answer::Accepted);
    assert_eq!((a.generation(), b.generation()), (1, 1));
    assert_ne!(b.records().forms(), before, "the other view shows the move");
    assert_eq!(a.records().forms(), b.records().forms());
}

#[test]
fn ga2_another_scope_shares_nothing() {
    let hub = RecordHub::new();
    let a = hub.attach(key(Some("urn:a")), board).unwrap();
    let b = hub.attach(key(Some("urn:b")), board).unwrap();
    let none = hub.attach(key(None), board).unwrap();
    assert!(!Arc::ptr_eq(&a.records(), &b.records()));
    assert!(!Arc::ptr_eq(&a.records(), &none.records()));
    let (untouched, other) = (b.records().forms(), none.records().forms());
    assert_eq!(a.records().move_card("FS-1", "done"), Answer::Accepted);
    assert_eq!(
        (a.generation(), b.generation(), none.generation()),
        (1, 0, 0)
    );
    assert_eq!(b.records().forms(), untouched);
    assert_eq!(none.records().forms(), other);
}

#[test]
fn ga3_the_last_grip_that_falls_lets_the_hub_go() {
    let hub = RecordHub::new();
    let a = hub.attach(key(Some("urn:a")), board).unwrap();
    let b = hub.attach(key(Some("urn:a")), board).unwrap();
    let weak = Arc::downgrade(&a.records());
    assert_eq!(hub.live(), 1);
    drop(a);
    assert!(weak.upgrade().is_some(), "one view is still attached");
    assert_eq!(hub.live(), 1);
    drop(b);
    assert!(weak.upgrade().is_none(), "no strong holder is left");
    assert_eq!(hub.live(), 0);
    // a later view of the same record opens new records
    let again = hub.attach(key(Some("urn:a")), board).unwrap();
    assert_eq!(again.generation(), 0);
}

#[test]
fn a_failed_open_registers_nothing() {
    let hub = RecordHub::new();
    let r = hub.attach(key(None), || {
        Err(fs_forms_run::Coded::new("host-notice-frontend-refused"))
    });
    assert!(r.is_err());
    assert_eq!(hub.live(), 0);
}
