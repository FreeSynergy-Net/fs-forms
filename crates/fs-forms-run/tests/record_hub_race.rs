//! Two gaps of the hub between "the module took it" and "the view knows it" (review of O4a, S1 and S5).
//!
//! Tests: R1 after an accepted write the view takes the revision of ITS OWN change as seen, not the revision the module has by then
//! (a neighbour wrote in between): the next write of the view is `conflict` until it has read; R2 a write between the first read of
//! the cache and the `watch` is not lost: `attach` watches first and then asks the module again, so the view is told (cache, seen,
//! generation).
//!
//! Mutants (each run, each red as named): M1 `Inner::write` takes `link.rev(..)` after the write as seen -> R1; M2 `attach` does not
//! ask again after the `watch` -> R2.

use std::sync::Arc;

use fs_components_kanban::Answer;
use fs_forms_run::{
    ChangeSink, FakeModule, HoldCounts, Link, Module, RecordHub, RecordKey, Records, SampleBoard,
    refused,
};

fn key() -> RecordKey {
    RecordKey {
        range: "net.freesynergy.Tasks".into(),
        scope: Some("urn:a".into()),
    }
}

/// The neighbour: a second own connection to the inner module that writes when told to.
fn neighbour_write(inner: &Arc<dyn Module>) {
    let link = inner.connect(Arc::new(Mute)).unwrap();
    let w = link.writer(Some("urn:a"), link.rev(Some("urn:a")));
    assert_eq!(w.add_card("todo", "from the neighbour"), Answer::Accepted);
}

struct Mute;
impl ChangeSink for Mute {
    fn changed(&self, _rev: u64) {}
}

/// A module whose links let the neighbour write at a chosen moment: right after an own write (`after_write`) or inside `watch`
/// before the watching begins (`in_watch`).
struct Racing {
    inner: Arc<dyn Module>,
    after_write: bool,
    in_watch: bool,
}

impl Module for Racing {
    fn records(&self, scope: Option<&str>) -> Arc<dyn Records> {
        self.inner.records(scope)
    }
    fn connect(&self, sink: Arc<dyn ChangeSink>) -> Result<Arc<dyn Link>, fs_forms_run::Coded> {
        Ok(Arc::new(RacingLink {
            inner: self.inner.connect(sink)?,
            module: Arc::clone(&self.inner),
            after_write: self.after_write,
            in_watch: self.in_watch,
        }))
    }
}

struct RacingLink {
    inner: Arc<dyn Link>,
    module: Arc<dyn Module>,
    after_write: bool,
    in_watch: bool,
}

impl Link for RacingLink {
    fn watch(&self, scope: Option<&str>) {
        if self.in_watch {
            neighbour_write(&self.module); // written before this connection listens
        }
        self.inner.watch(scope);
    }
    fn rev(&self, scope: Option<&str>) -> u64 {
        self.inner.rev(scope)
    }
    fn writer(&self, scope: Option<&str>, seen: u64) -> Arc<dyn Records> {
        self.inner.writer(scope, seen)
    }
    fn write(
        &self,
        scope: Option<&str>,
        seen: u64,
        change: &mut dyn FnMut(&dyn Records) -> Answer,
    ) -> (Answer, Option<u64>) {
        let got = self.inner.write(scope, seen, change);
        if self.after_write {
            neighbour_write(&self.module); // the module takes the neighbour's write right after ours
        }
        got
    }
}

fn racing(after_write: bool, in_watch: bool) -> Arc<dyn Module> {
    let m = FakeModule::new(
        Arc::new(HoldCounts::default()),
        Box::new(|_| Arc::new(SampleBoard::default()) as Arc<dyn Records>),
    );
    Arc::new(Racing {
        inner: m,
        after_write,
        in_watch,
    })
}

#[test]
fn r1_the_view_takes_the_revision_of_its_own_change_as_seen() {
    let m = racing(true, false);
    let hub = RecordHub::new();
    let a = hub.attach(&key(), &m).unwrap();
    assert_eq!(a.records().move_card("FS-1", "done"), Answer::Accepted);
    assert_eq!(
        a.seen(),
        1,
        "its own change, not the neighbour's revision 2"
    );
    assert_eq!(
        a.records().move_card("FS-2", "done"),
        refused("conflict"),
        "it has not seen the neighbour's change"
    );
    let _ = a.records().forms(); // reads: now it has
    assert_eq!(a.seen(), 2);
}

#[test]
fn r2_a_write_between_the_first_read_and_the_watch_is_not_lost() {
    let m = racing(false, true);
    let hub = RecordHub::new();
    let a = hub.attach(&key(), &m).unwrap();
    assert_eq!(a.cache().history().undoable, 0);
    assert_eq!(a.seen(), 1, "the cache and the view know revision 1");
    assert_eq!(a.generation(), 1, "the view was told, it draws again");
}
