//! The hub as a READ CACHE (stage 6, P3+ of NACHTRAG-2, A+: two instances are two participants): views of the same record share one
//! cached `Records`, but each has its OWN connection (hold), watches itself and writes only through its own link with the revision it
//! saw. The cache has no write path.
//!
//! Tests: GA1 two grips of one (range, scope) share the cache (`ptr_eq`), the module counts TWO holds, a change through one shows in
//! the other; GA2 another scope is another cache and another generation; GA3 one of two grips falls: the cache stays and one hold is
//! left; the last falls: no strong holder, both holds detached; GA5 the neighbour learns of a change ONLY through its own `watch`;
//! GA6 two windows, one record, the second writes with an old revision: `conflict`; and the cache itself cannot be written through.
//!
//! Mutants (each makes the named test fail): H1 the key leaves out the `scope` -> GA2; H2 `attach` always makes a new cache -> GA1,
//! GA3; H3 the hub leaks a strong `Arc` -> GA3; H5 the module does not tell the holds (no broadcast) -> GA1, GA5; H5b it tells only the
//! writer's own hold -> GA5; H6 the revision is ignored on writing -> GA6; H7 the grip writes through the shared cache -> GA5, GA6.

use std::sync::Arc;

use fs_components_kanban::Answer;
use fs_forms_run::{
    Coded, FakeModule, HoldCounts, Module, RecordHub, RecordKey, Records, SampleBoard,
};

fn key(scope: Option<&str>) -> RecordKey {
    RecordKey {
        range: "net.freesynergy.Tasks".into(),
        scope: scope.map(str::to_owned),
    }
}

fn module() -> (Arc<dyn Module>, Arc<HoldCounts>) {
    let counts = Arc::new(HoldCounts::default());
    let made = FakeModule::new(
        Arc::clone(&counts),
        Box::new(|_| Arc::new(SampleBoard::default()) as Arc<dyn Records>),
    );
    (made, counts)
}

#[test]
fn ga1_two_views_share_the_cache_and_each_has_its_own_hold() {
    let (m, counts) = module();
    let hub = RecordHub::new();
    let a = hub.attach(&key(Some("urn:a")), &m).unwrap();
    let b = hub.attach(&key(Some("urn:a")), &m).unwrap();
    assert!(Arc::ptr_eq(&a.cache(), &b.cache()));
    assert_eq!(counts.get(), (2, 0), "two views, two holds");
    let before = b.records().forms();
    assert_eq!((a.generation(), b.generation()), (0, 0));
    assert_eq!(a.records().move_card("FS-1", "done"), Answer::Accepted);
    assert_eq!((a.generation(), b.generation()), (1, 1));
    assert_ne!(b.records().forms(), before, "the other view shows the move");
    assert_eq!(a.records().forms(), b.records().forms());
}

#[test]
fn ga2_another_scope_shares_nothing() {
    let (m, _) = module();
    let hub = RecordHub::new();
    let a = hub.attach(&key(Some("urn:a")), &m).unwrap();
    let b = hub.attach(&key(Some("urn:b")), &m).unwrap();
    let none = hub.attach(&key(None), &m).unwrap();
    assert!(!Arc::ptr_eq(&a.cache(), &b.cache()));
    assert!(!Arc::ptr_eq(&a.cache(), &none.cache()));
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
fn ga3_the_last_grip_that_falls_lets_cache_and_holds_go() {
    let (m, counts) = module();
    let hub = RecordHub::new();
    let a = hub.attach(&key(Some("urn:a")), &m).unwrap();
    let b = hub.attach(&key(Some("urn:a")), &m).unwrap();
    let weak = Arc::downgrade(&a.cache());
    assert_eq!(hub.live(), 1);
    drop(a);
    assert!(weak.upgrade().is_some(), "one view is still attached");
    assert_eq!((hub.live(), counts.get()), (1, (2, 1)));
    drop(b);
    assert!(weak.upgrade().is_none(), "no strong holder is left");
    assert_eq!((hub.live(), counts.get()), (0, (2, 2)));
    let again = hub.attach(&key(Some("urn:a")), &m).unwrap();
    assert_eq!(again.generation(), 0);
}

#[test]
fn ga5_the_neighbour_learns_only_through_its_own_watch() {
    let (m, _) = module();
    let hub = RecordHub::new();
    let a = hub.attach(&key(Some("urn:a")), &m).unwrap();
    let b = hub.attach(&key(Some("urn:a")), &m).unwrap();
    assert_eq!(b.records().add_card("todo", "x"), Answer::Accepted);
    // the writer is told by its own hold like everyone else, no counting at the writing place
    assert_eq!((a.generation(), b.generation()), (1, 1));
    // a write that is not accepted tells nobody
    assert_ne!(a.records().move_card("NOPE", "nowhere"), Answer::Accepted);
    assert_eq!((a.generation(), b.generation()), (1, 1));
}

#[test]
fn ga6_a_second_window_writing_with_an_old_revision_gets_conflict() {
    let (m, _) = module();
    let hub = RecordHub::new();
    let a = hub.attach(&key(Some("urn:a")), &m).unwrap();
    let b = hub.attach(&key(Some("urn:a")), &m).unwrap();
    assert_eq!(a.records().move_card("FS-1", "done"), Answer::Accepted);
    assert_eq!((a.seen(), b.seen()), (1, 0));
    assert_eq!(
        b.records().move_card("FS-2", "done"),
        fs_forms_run::refused("conflict"),
        "the same rule as between two people, though both are one person"
    );
    let _ = b.records().forms(); // b has seen the new state now
    assert_eq!(b.seen(), 1);
    assert_eq!(b.records().move_card("FS-2", "done"), Answer::Accepted);
}

#[test]
fn the_cache_cannot_be_written_through() {
    let (m, _) = module();
    let hub = RecordHub::new();
    let a = hub.attach(&key(Some("urn:a")), &m).unwrap();
    let before = a.cache().forms();
    assert_ne!(a.cache().move_card("FS-1", "done"), Answer::Accepted);
    assert_eq!((a.generation(), a.cache().forms()), (0, before));
}

#[test]
fn rebind_keeps_the_hold_and_moves_the_view_to_the_other_record() {
    let (m, counts) = module();
    let hub = RecordHub::new();
    let a = hub.attach(&key(Some("urn:a")), &m).unwrap();
    let old = Arc::downgrade(&a.cache());
    hub.rebind(&a, key(Some("urn:b")), &m).unwrap();
    assert_eq!(counts.get(), (1, 0), "the hold stays, no second connection");
    assert!(
        old.upgrade().is_none(),
        "the old cache is shown nowhere now"
    );
    assert_eq!((a.key(), a.generation()), (key(Some("urn:b")), 1));
    let mut other = key(Some("urn:b"));
    other.range = "other.Program".into();
    assert!(hub.rebind(&a, other, &m).is_err());
    assert_eq!(a.key(), key(Some("urn:b")), "an error changes nothing");
}

struct Refusing;

impl Module for Refusing {
    fn records(&self, _scope: Option<&str>) -> Arc<dyn Records> {
        Arc::new(SampleBoard::default())
    }
    fn connect(
        &self,
        _sink: Arc<dyn fs_forms_run::ChangeSink>,
    ) -> Result<Arc<dyn fs_forms_run::Link>, Coded> {
        Err(Coded::new("host-notice-frontend-refused"))
    }
}

#[test]
fn a_refused_connection_registers_nothing() {
    let hub = RecordHub::new();
    let m: Arc<dyn Module> = Arc::new(Refusing);
    assert!(hub.attach(&key(None), &m).is_err());
    assert_eq!(hub.live(), 0);
}
