//! The two holes the review of stage 6 (R3) found in the hub, closed: S1 the proxy of a view passes on `history`, `step_back` and
//! `changed` (ONE counter: the generation of the view is what `Records::changed` says, so a `ChangeWatch` sees a neighbour's write and
//! undo goes over the own connection with the seen revision); S2 the module itself checks the seen revision (`Link::writer(scope,
//! seen)`), not only the client.
//!
//! Tests: S1a a neighbour writes, `ChangeWatch::grew` of the other view is true once and `changed()` equals `generation()`; S1b
//! `history` is read through the cache; S1c `step_back` with an old revision is `conflict` and changes nothing, with the seen
//! revision it is taken; S2 a writer made at an old revision is refused by the module even when the client check is bypassed.
//!
//! Mutants (all four run, each red as named): M1 `Inner::changed` stays the default 0 -> S1a; M2 `Inner::step_back` writes without
//! `rev` (straight through the link at the module's revision) -> S1c; M3 `Inner::history` stays the default -> S1b; M4 the module
//! drops the check of `seen` in `guarded` -> S2 (S1c stays green: the client checks first).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use fs_components_kanban::Answer;
use fs_forms_run::{
    ChangeSink, ChangeWatch, FakeModule, HoldCounts, Module, RecordHub, RecordKey, Records,
    SampleBoard, UndoState,
};

/// A store with an undo depth: every accepted move is one step; a step back is taken once per call.
#[derive(Default)]
struct Undoable {
    board: SampleBoard,
    steps: AtomicUsize,
    undone: AtomicUsize,
}

impl Records for Undoable {
    fn side(&self) -> Vec<fs_forms_run::SideItem> {
        Vec::new()
    }
    fn invoke(
        &self,
        form: &str,
        channel: &str,
        record: Option<&str>,
        values: &[(String, String)],
    ) -> Answer {
        let a = self.board.invoke(form, channel, record, values);
        if a == Answer::Accepted {
            self.steps.fetch_add(1, Ordering::SeqCst);
        }
        a
    }
    fn history(&self) -> UndoState {
        UndoState {
            undoable: self.steps.load(Ordering::SeqCst),
            redoable: 0,
        }
    }
    fn step_back(&self, _redo: bool) -> Answer {
        self.undone.fetch_add(1, Ordering::SeqCst);
        Answer::Accepted
    }
}

fn key() -> RecordKey {
    RecordKey {
        range: "net.freesynergy.Tasks".into(),
        scope: Some("urn:a".into()),
    }
}

fn module() -> (Arc<FakeModule>, Arc<Undoable>) {
    let store = Arc::new(Undoable::default());
    let handed = Arc::clone(&store);
    let m = FakeModule::new(
        Arc::new(HoldCounts::default()),
        Box::new(move |_| Arc::clone(&handed) as Arc<dyn Records>),
    );
    (m, store)
}

#[test]
fn s1a_a_neighbours_write_makes_the_change_watch_of_the_other_view_grow_once() {
    let (m, _) = module();
    let m: Arc<dyn Module> = m;
    let hub = RecordHub::new();
    let a = hub.attach(&key(), &m).unwrap();
    let b = hub.attach(&key(), &m).unwrap();
    let mut watch = ChangeWatch::new();
    assert!(!watch.grew(&*b.records()), "nothing happened yet");
    assert_eq!(
        fs_forms_run::sample_move_card(&*a.records(), "FS-1", "done"),
        Answer::Accepted
    );
    assert!(watch.grew(&*b.records()), "the neighbour's write is seen");
    assert!(!watch.grew(&*b.records()), "only once per change");
    assert_eq!(b.records().changed(), b.generation(), "ONE counter");
}

#[test]
fn s1b_the_history_is_read_through_the_cache() {
    let (m, _) = module();
    let m: Arc<dyn Module> = m;
    let hub = RecordHub::new();
    let a = hub.attach(&key(), &m).unwrap();
    assert_eq!(a.records().history().undoable, 0);
    assert_eq!(
        fs_forms_run::sample_move_card(&*a.records(), "FS-1", "done"),
        Answer::Accepted
    );
    assert_eq!(a.records().history().undoable, 1);
    assert_eq!(a.cache().history().undoable, 1, "the cache reads it too");
}

#[test]
fn s1c_a_step_back_goes_over_the_own_connection_with_the_seen_revision() {
    let (m, store) = module();
    let m: Arc<dyn Module> = m;
    let hub = RecordHub::new();
    let a = hub.attach(&key(), &m).unwrap();
    let b = hub.attach(&key(), &m).unwrap();
    assert_eq!(
        fs_forms_run::sample_move_card(&*a.records(), "FS-1", "done"),
        Answer::Accepted
    );
    assert_eq!(
        b.records().step_back(false),
        fs_forms_run::refused("conflict"),
        "b has not seen the change of a"
    );
    assert_eq!(store.undone.load(Ordering::SeqCst), 0, "nothing was undone");
    let _ = b.records().forms();
    assert_eq!(b.records().step_back(false), Answer::Accepted);
    assert_eq!(store.undone.load(Ordering::SeqCst), 1);
}

struct Mute;
impl ChangeSink for Mute {
    fn changed(&self, _rev: u64) {}
}

#[test]
fn s2_the_module_refuses_a_writer_made_at_an_old_revision_without_the_client() {
    let (m, store) = module();
    let m: Arc<dyn Module> = m;
    let link = m.connect(Arc::new(Mute)).unwrap();
    let other = m.connect(Arc::new(Mute)).unwrap();
    link.watch(Some("urn:a"));
    let first = link.writer(Some("urn:a"), link.rev(Some("urn:a")));
    assert_eq!(
        fs_forms_run::sample_move_card(&*first, "FS-1", "done"),
        Answer::Accepted
    );
    // `other` still holds a writer made at revision 0: no client in between checks it
    let stale = other.writer(Some("urn:a"), 0);
    assert_eq!(
        fs_forms_run::sample_move_card(&*stale, "FS-2", "done"),
        fs_forms_run::refused("conflict")
    );
    assert_eq!(
        store.steps.load(Ordering::SeqCst),
        1,
        "the stale write did not happen"
    );
    let fresh = other.writer(Some("urn:a"), other.rev(Some("urn:a")));
    assert_eq!(
        fs_forms_run::sample_move_card(&*fresh, "FS-2", "done"),
        Answer::Accepted
    );
}
