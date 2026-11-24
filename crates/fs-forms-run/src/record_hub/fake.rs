//! The double of a module for the hub: it keeps a revision per record, takes writes from the links and tells EVERY watching link of the
//! record (the writer's own and every neighbour's) through its sink, like a real `FormSource.watch`.
//!
//! Pattern: Fake (test double) – the same contract as the real module (revision, `conflict`, `watch` to every connection), in memory.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use fs_components_kanban::Answer;

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
            next: AtomicUsize::new(0),
        })
    }

    /// The module took a write of connection `from` on `scope`: the revision grows and EVERY connection watching the record is told,
    /// the writer's own and the neighbours' alike (no shortcut for "the same process").
    fn wrote(&self, scope: &Scope, _from: usize) {
        let rev = {
            let mut revs = lock(&self.revs);
            let r = revs.entry(scope.clone()).or_insert(0);
            *r += 1;
            *r
        };
        let sinks: Vec<Arc<dyn ChangeSink>> = lock(&self.regs)
            .iter()
            .filter(|r| *lock(&r.scope) == *scope)
            .filter_map(|r| r.sink.upgrade())
            .collect();
        for s in sinks {
            s.changed(rev);
        }
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

    fn writer(&self, scope: Option<&str>) -> Arc<dyn Records> {
        Arc::new(LinkWriter {
            module: Arc::clone(&self.module),
            id: self.id,
            scope: scope.map(str::to_owned),
            store: self.module.records(scope),
        })
    }
}

/// The writing side on one connection: an accepted change makes the module count a revision and tell the watchers.
struct LinkWriter {
    module: Arc<FakeModule>,
    id: usize,
    scope: Scope,
    store: Arc<dyn Records>,
}

impl LinkWriter {
    fn done(&self, answer: Answer) -> Answer {
        if answer == Answer::Accepted {
            self.module.wrote(&self.scope, self.id);
        }
        answer
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
        self.done(self.store.invoke(form, channel, record, values))
    }
    fn add_card(&self, column: &str, title: &str) -> Answer {
        self.done(self.store.add_card(column, title))
    }
    fn move_card(&self, key: &str, to: &str) -> Answer {
        self.done(self.store.move_card(key, to))
    }
    fn add_column(&self, title: &str) -> Answer {
        self.done(self.store.add_column(title))
    }
    fn rename_column(&self, column: &str, name: &str) -> Answer {
        self.done(self.store.rename_column(column, name))
    }
    fn set_limit(&self, column: &str, limit: Option<u32>) -> Answer {
        self.done(self.store.set_limit(column, limit))
    }
}
