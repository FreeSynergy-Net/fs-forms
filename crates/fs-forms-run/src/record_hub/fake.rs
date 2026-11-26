//! The double of a module for the hub: it keeps a revision per record, takes writes from the links and tells EVERY watching link of the
//! record (the writer's own and every neighbour's) through its sink, like a real `FormSource.watch`.
//!
//! Pattern: Fake (test double) – the same contract as the real module (revision, `conflict`, `watch` to every connection), in memory.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use fs_components_kanban::Answer;
use fs_objects::UndoState;

use super::port::{ChangeSink, Link, Module};
use crate::records::{Records, SideItem};
use crate::view::Coded;

/// How many held connections were made and how many fell.
#[derive(Debug, Default)]
pub struct HoldCounts {
    attached: AtomicUsize,
    detached: AtomicUsize,
}

impl HoldCounts {
    /// `(attached, detached)`.
    #[must_use]
    pub fn get(&self) -> (usize, usize) {
        (
            self.attached.load(Ordering::SeqCst),
            self.detached.load(Ordering::SeqCst),
        )
    }
}

/// Pattern: RAII (test double) – stands for the held connection to a module (`fs_rpc::hold`, not built yet): attached when made,
/// detached when dropped.
#[derive(Debug)]
pub struct FakeHold {
    counts: Arc<HoldCounts>,
}

impl FakeHold {
    /// A hold, counted as attached; it counts as detached when dropped.
    #[must_use]
    pub fn attach(counts: &Arc<HoldCounts>) -> Self {
        counts.attached.fetch_add(1, Ordering::SeqCst);
        Self {
            counts: Arc::clone(counts),
        }
    }
}

impl Drop for FakeHold {
    fn drop(&mut self) {
        self.counts.detached.fetch_add(1, Ordering::SeqCst);
    }
}

type Scope = Option<String>;

struct Reg {
    id: usize,
    scope: Arc<Mutex<Scope>>,
    sink: Weak<dyn ChangeSink>,
}

/// Makes the stand-in records of one record of the module.
pub type Factory = Box<dyn Fn(Option<&str>) -> Arc<dyn Records> + Send + Sync>;

/// Pattern: Fake (test double) – one module with a revision per record and a `watch` to every connection.
pub struct FakeModule {
    me: Weak<FakeModule>,
    counts: Arc<HoldCounts>,
    factory: Factory,
    stores: Mutex<HashMap<Scope, Weak<dyn Records>>>,
    revs: Mutex<HashMap<Scope, u64>>,
    regs: Mutex<Vec<Reg>>,
    /// The gate of the writes: checking the seen revision and taking the change are ONE step under it (no gap between check and write).
    gate: Mutex<()>,
    next: AtomicUsize,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

impl FakeModule {
    /// A module whose records come from `factory`; its connections are counted in `counts`.
    #[must_use]
    pub fn new(counts: Arc<HoldCounts>, factory: Factory) -> Arc<Self> {
        Arc::new_cyclic(|me| Self {
            me: me.clone(),
            counts,
            factory,
            stores: Mutex::new(HashMap::new()),
            revs: Mutex::new(HashMap::new()),
            regs: Mutex::new(Vec::new()),
            gate: Mutex::new(()),
            next: AtomicUsize::new(0),
        })
    }

    /// The module took a write of connection `from` on `scope`: the revision grows and EVERY connection watching the record is told,
    /// the writer's own and the neighbours' alike (no shortcut for "the same process").
    fn wrote(&self, scope: &Scope, rev: u64) {
        let sinks: Vec<Arc<dyn ChangeSink>> = lock(&self.regs)
            .iter()
            .filter(|r| *lock(&r.scope) == *scope)
            .filter_map(|r| r.sink.upgrade())
            .collect();
        for s in sinks {
            s.changed(rev);
        }
    }

    /// Counts the revision of `scope` one up (called under the gate, in the same step as the check that let the write in).
    fn bump(&self, scope: &Scope) -> u64 {
        let mut revs = lock(&self.revs);
        let r = revs.entry(scope.clone()).or_insert(0);
        *r += 1;
        *r
    }

    fn rev_of(&self, scope: &Scope) -> u64 {
        lock(&self.revs).get(scope).copied().unwrap_or(0)
    }
}

impl Module for FakeModule {
    fn records(&self, scope: Option<&str>) -> Arc<dyn Records> {
        let key = scope.map(str::to_owned);
        let mut stores = lock(&self.stores);
        if let Some(s) = stores.get(&key).and_then(Weak::upgrade) {
            return s;
        }
        let made = (self.factory)(scope);
        stores.insert(key, Arc::downgrade(&made));
        made
    }

    fn connect(&self, sink: Arc<dyn ChangeSink>) -> Result<Arc<dyn Link>, Coded> {
        let module = self.me.upgrade().expect("a module lives while it is used");
        let id = self.next.fetch_add(1, Ordering::SeqCst);
        let scope = Arc::new(Mutex::new(None));
        lock(&self.regs).push(Reg {
            id,
            scope: Arc::clone(&scope),
            sink: Arc::downgrade(&sink),
        });
        Ok(Arc::new(FakeLink {
            module,
            id,
            scope,
            _hold: FakeHold::attach(&self.counts),
        }))
    }
}

/// One own connection of a view to the [`FakeModule`].
struct FakeLink {
    module: Arc<FakeModule>,
    id: usize,
    scope: Arc<Mutex<Scope>>,
    _hold: FakeHold,
}

impl Drop for FakeLink {
    fn drop(&mut self) {
        lock(&self.module.regs).retain(|r| r.id != self.id);
    }
}

impl Link for FakeLink {
    fn watch(&self, scope: Option<&str>) {
        *lock(&self.scope) = scope.map(str::to_owned);
    }

    fn rev(&self, scope: Option<&str>) -> u64 {
        self.module.rev_of(&scope.map(str::to_owned))
    }

    fn writer(&self, scope: Option<&str>, seen: u64) -> Arc<dyn Records> {
        Arc::new(LinkWriter {
            module: Arc::clone(&self.module),
            seen,
            scope: scope.map(str::to_owned),
            store: self.module.records(scope),
        })
    }

    fn write(
        &self,
        scope: Option<&str>,
        seen: u64,
        change: &mut dyn FnMut(&dyn Records) -> Answer,
    ) -> (Answer, Option<u64>) {
        LinkWriter {
            module: Arc::clone(&self.module),
            seen,
            scope: scope.map(str::to_owned),
            store: self.module.records(scope),
        }
        .guarded_rev(change)
    }
}

/// The writing side on one connection: an accepted change makes the module count a revision and tell the watchers.
struct LinkWriter {
    module: Arc<FakeModule>,
    seen: u64,
    scope: Scope,
    store: Arc<dyn Records>,
}

impl LinkWriter {
    /// The module takes the change only when the record is still at the revision this view saw: checked and written under the gate,
    /// then (outside it) every watching connection is told.
    fn guarded(&self, change: impl FnOnce(&dyn Records) -> Answer) -> Answer {
        let mut change = Some(change);
        self.guarded_rev(&mut |s| {
            change
                .take()
                .map_or_else(|| crate::records::refused("conflict"), |c| c(s))
        })
        .0
    }

    /// The same, and the revision the accepted change made (`None` when refused).
    fn guarded_rev(&self, change: &mut dyn FnMut(&dyn Records) -> Answer) -> (Answer, Option<u64>) {
        let (answer, rev) = {
            let _gate = lock(&self.module.gate);
            if self.module.rev_of(&self.scope) > self.seen {
                return (crate::records::refused("conflict"), None);
            }
            let answer = change(&*self.store);
            let rev = (answer == Answer::Accepted).then(|| self.module.bump(&self.scope));
            (answer, rev)
        };
        if let Some(rev) = rev {
            self.module.wrote(&self.scope, rev);
        }
        (answer, rev)
    }
}

impl Records for LinkWriter {
    fn side(&self) -> Vec<SideItem> {
        Vec::new()
    }
    fn invoke(
        &self,
        form: &str,
        channel: &str,
        record: Option<&str>,
        values: &[(String, String)],
    ) -> Answer {
        self.guarded(|s| s.invoke(form, channel, record, values))
    }
    fn invoke_command(&self, form: &str, command: &str, record: Option<&str>) -> Answer {
        self.guarded(|s| s.invoke_command(form, command, record))
    }
    fn history(&self) -> UndoState {
        self.store.history()
    }
    fn step_back(&self, redo: bool) -> Answer {
        self.guarded(|s| s.step_back(redo))
    }
}
