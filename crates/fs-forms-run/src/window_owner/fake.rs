//! The double of the [`WindowOwner`] (S6-2): it keeps counters instead of windows. A "window" is an entry with a composition, a
//! grip with its OWN connection ([`FakeHold`] counts attach and detach) to a fake module over a shared read cache, so the conformance tests
//! (`tests/window_owner.rs`) can prove the contract without a window, a socket or `fs_rpc::hold`.
//!
//! Pattern: Fake (test double) – the same contract as the real owner, held in memory.

mod parts;
mod share;
mod switch;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};

use fs_components_kanban::Answer;
use fs_forms_layout::FrontendKey;

pub use self::parts::CountingRecords;
use super::open::{at_place, exact};
use super::{
    Decision, OpenFrontend, Opened, OpenedFrontend, Placement, WhenOpen, WindowId, WindowOwner,
    decide,
};
use crate::catalog::TestCatalog;
use crate::compose::{Composition, compose};
pub use crate::record_hub::fake::FakeHold;
use crate::record_hub::fake::{FakeModule, HoldCounts};
use crate::record_hub::{Module, RecordGrip, RecordHub, RecordKey};
use crate::view::{Coded, View};

/// What one open frontend holds; dropping the entry drops all of it.
struct Entry {
    key: FrontendKey,
    place: (String, String),
    mode: Placement,
    switchable: bool,
    raised: usize,
    parent: Option<WindowId>,
    scope: Option<String>,
    composition: Composition,
    grip: RecordGrip,
    module: Arc<dyn Module>,
}

type Typed = HashMap<RecordKey, Weak<CountingRecords>>;

type Modules = HashMap<String, Weak<FakeModule>>;

type Open = HashMap<WindowId, Entry>;

fn snapshot(open: &Open) -> Vec<OpenedFrontend> {
    let mut now: Vec<OpenedFrontend> = open
        .iter()
        .map(|(id, e)| OpenedFrontend {
            id: id.clone(),
            key: e.key.clone(),
            mode: e.mode,
            parent: e.parent.clone(),
            scope: e.scope.clone(),
        })
        .collect();
    now.sort_by(|a, b| a.id.cmp(&b.id));
    now
}

/// Pattern: Fake (test double) – a [`WindowOwner`] that keeps counters instead of windows.
///
/// Abweichung: besides the contract it has reading helpers for the tests (`composition_of`, `records_of`, `invoke`, `holds`).
pub struct FakeWindows {
    catalog: TestCatalog,
    known: HashMap<FrontendKey, View>,
    refusing: Vec<FrontendKey>,
    counts: Arc<HoldCounts>,
    hub: RecordHub,
    typed: Arc<Mutex<Typed>>,
    modules: Mutex<Modules>,
    open: Mutex<Open>,
}

impl FakeWindows {
    /// A fake that composes with `catalog` and knows no frontend yet.
    #[must_use]
    pub fn new(catalog: TestCatalog) -> Self {
        Self {
            catalog,
            known: HashMap::new(),
            refusing: Vec::new(),
            counts: Arc::new(HoldCounts::default()),
            hub: RecordHub::new(),
            typed: Arc::new(Mutex::new(HashMap::new())),
            modules: Mutex::new(HashMap::new()),
            open: Mutex::new(HashMap::new()),
        }
    }

    /// The description of a frontend this fake can open.
    #[must_use]
    pub fn knowing(mut self, key: FrontendKey, view: View) -> Self {
        self.known.insert(key, view);
        self
    }

    /// The module of `key` says no on connecting (`host-notice-frontend-refused`).
    #[must_use]
    pub fn refusing(mut self, key: FrontendKey) -> Self {
        self.refusing.push(key);
        self
    }

    /// The composition of an open frontend (what a panel puts in its parent, what a window shows).
    #[must_use]
    pub fn composition_of(&self, id: &WindowId) -> Option<Composition> {
        self.lock().get(id).map(|e| e.composition.clone())
    }

    /// A weak handle to the records of an open frontend: it is dead once the frontend is closed.
    #[must_use]
    pub fn records_of(&self, id: &WindowId) -> Option<Weak<CountingRecords>> {
        let open = self.lock();
        let e = open.get(id)?;
        let key = RecordKey {
            range: e.key.extends.clone(),
            scope: e.scope.clone(),
        };
        drop(open);
        self.typed().get(&key).cloned()
    }

    /// A call of the child frontend `id`: it reaches the records of that frontend only. `None` when `id` is not open.
    #[must_use]
    pub fn invoke(&self, id: &WindowId, form: &str, channel: &str) -> Option<Answer> {
        let records = self.lock().get(id).map(|e| e.grip.records())?;
        Some(records.invoke(form, channel, None, &[]))
    }

    /// How often an open of this id found it already open (it is brought to the front, the mode stays).
    #[must_use]
    pub fn raised(&self, id: &WindowId) -> usize {
        self.lock().get(id).map_or(0, |e| e.raised)
    }

    /// `(attached, detached)`: how many held connections were made and how many fell.
    #[must_use]
    pub fn holds(&self) -> (usize, usize) {
        self.counts.get()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Open> {
        self.open
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn fresh(
        &self,
        req: &OpenFrontend,
        parent: Option<WindowId>,
        place: (String, String),
    ) -> Result<Entry, Coded> {
        let missing = || {
            Coded::new("host-notice-frontend-missing")
                .with("extends", req.key.extends.clone())
                .with("frontend", req.key.frontend.clone())
        };
        let view = self.known.get(&req.key).ok_or_else(missing)?;
        if self.refusing.contains(&req.key) {
            return Err(Coded::new("host-notice-frontend-refused")
                .with("extends", req.key.extends.clone())
                .with("frontend", req.key.frontend.clone()));
        }
        let (grip, module) = self.grip_for(&req.key.extends, req.scope.as_deref())?;
        Ok(Entry {
            key: req.key.clone(),
            place,
            mode: req.mode,
            switchable: req.switchable,
            raised: 0,
            parent,
            scope: req.scope.clone(),
            composition: compose(view, &self.catalog),
            grip,
            module,
        })
    }
}

impl WindowOwner for FakeWindows {
    fn open(&self, req: OpenFrontend) -> Result<Opened, Coded> {
        let mut open = self.lock();
        let (program, elem, parent) = match &req.parent {
            Some((pid, elem)) => {
                let p = open.get(pid).ok_or_else(|| {
                    Coded::new("host-notice-window-unknown").with("id", pid.as_str())
                })?;
                (
                    p.key.extends.clone(),
                    elem.as_str().to_owned(),
                    Some(pid.clone()),
                )
            }
            None => (req.key.extends.clone(), req.key.frontend.clone(), None),
        };
        let id = WindowId::of_record(&program, &elem, req.scope.as_deref());
        let now = snapshot(&open);
        let place = (program, elem);
        if let Some(same) = exact(&now, &req, &place).cloned() {
            if let Some(e) = open.get_mut(&same) {
                e.raised += 1; // brought to the front, the mode and the record stay
            }
            return Ok(Opened::Raised(same));
        }
        let at = at_place(&now, &req, &place).cloned();
        match (decide(&now, &req), at) {
            (Decision::Open(WhenOpen::Switch), Some(at)) => {
                // Same place, other record: the grip (and its own connection, the hold) STAYS; it now watches and reads the new record
                // (same module). The cache of the new one is taken BEFORE the old falls, so a failure leaves the place as it was.
                let range = open[&at].key.extends.clone();
                let mut e = open.remove(&at).expect("at_place names an open entry");
                let key = RecordKey {
                    range,
                    scope: req.scope.clone(),
                };
                if let Err(err) = self.hub.rebind(&e.grip, key, &e.module) {
                    open.insert(at, e);
                    return Err(err);
                }
                let id = WindowId::of_record(&e.place.0, &e.place.1, req.scope.as_deref());
                let from = std::mem::replace(&mut e.scope, req.scope.clone());
                for c in open.values_mut().filter(|c| c.parent.as_ref() == Some(&at)) {
                    c.parent = Some(id.clone());
                }
                open.insert(id.clone(), e);
                Ok(Opened::Switched { id, from })
            }
            (Decision::Ask { at }, _) => {
                Err(Coded::new("host-notice-open-unanswered").with("at", at.as_str()))
            }
            _ => {
                if open.contains_key(&id) {
                    return Err(Coded::new("host-notice-window-id-taken").with("id", id.as_str()));
                }
                let entry = self.fresh(&req, parent, place)?;
                open.insert(id.clone(), entry);
                Ok(Opened::New(id))
            }
        }
    }

    fn close(&self, id: &WindowId) -> Result<(), Coded> {
        let mut open = self.lock();
        if !open.contains_key(id) {
            return Err(Coded::new("host-notice-window-unknown").with("id", id.as_str()));
        }
        // Every frontend whose parent chain reaches `id` goes with it.
        let mut doomed = vec![id.clone()];
        loop {
            let more: Vec<WindowId> = open
                .iter()
                .filter(|(k, e)| {
                    !doomed.contains(k) && e.parent.as_ref().is_some_and(|p| doomed.contains(p))
                })
                .map(|(k, _)| k.clone())
                .collect();
            if more.is_empty() {
                break;
            }
            doomed.extend(more);
        }
        let gone: Vec<Entry> = doomed.iter().filter_map(|d| open.remove(d)).collect();
        drop(open);
        drop(gone);
        Ok(())
    }

    fn place(&self, id: &WindowId, placement: Placement) -> Result<(), Coded> {
        let mut open = self.lock();
        let e = open
            .get_mut(id)
            .ok_or_else(|| Coded::new("host-notice-window-unknown").with("id", id.as_str()))?;
        if !e.switchable {
            return Err(Coded::new("host-notice-placement-locked").with("id", id.as_str()));
        }
        e.mode = placement;
        Ok(())
    }

    fn open_now(&self) -> Vec<OpenedFrontend> {
        snapshot(&self.lock())
    }

    fn use_frontend(&self, id: &WindowId, frontend: &str) -> Result<(), Coded> {
        self.switch_frontend(id, frontend)
    }
}
