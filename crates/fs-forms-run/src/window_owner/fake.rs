//! The double of the [`WindowOwner`] (S6-2): it keeps counters instead of windows. A "window" is an entry with a composition, a
//! counting stand-in for the module records and a [`FakeHold`] that counts attach and detach, so the conformance tests
//! (`tests/window_owner.rs`) can prove the contract without a window, a socket or `fs_rpc::hold`.
//!
//! Pattern: Fake (test double) – the same contract as the real owner, held in memory.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Weak};

use fs_components_kanban::Answer;
use fs_forms_layout::FrontendKey;

use super::{OpenFrontend, OpenedFrontend, Placement, WindowId, WindowOwner};
use crate::catalog::TestCatalog;
use crate::compose::{Composition, compose};
use crate::records::{Records, SideItem};
use crate::view::{Coded, View};

/// Pattern: Fake (test double) – stand-in for the records of one module that counts the calls it takes.
#[derive(Debug, Default)]
pub struct CountingRecords {
    calls: AtomicUsize,
}

impl CountingRecords {
    /// How many calls these records took.
    #[must_use]
    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl Records for CountingRecords {
    fn side(&self) -> Vec<SideItem> {
        Vec::new()
    }

    fn invoke(
        &self,
        _form: &str,
        _channel: &str,
        _record: Option<&str>,
        _values: &[(String, String)],
    ) -> Answer {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Answer::Accepted
    }
}

/// Pattern: RAII (test double) – stands for the held connection to a module (`fs_rpc::hold`, not built yet): attached when made,
/// detached when dropped.
#[derive(Debug)]
pub struct FakeHold {
    detached: Arc<AtomicUsize>,
}

impl FakeHold {
    fn attach(attached: &AtomicUsize, detached: &Arc<AtomicUsize>) -> Self {
        attached.fetch_add(1, Ordering::SeqCst);
        Self {
            detached: Arc::clone(detached),
        }
    }
}

impl Drop for FakeHold {
    fn drop(&mut self) {
        self.detached.fetch_add(1, Ordering::SeqCst);
    }
}

/// What one open frontend holds; dropping the entry drops all of it.
struct Entry {
    key: FrontendKey,
    mode: Placement,
    switchable: bool,
    raised: usize,
    parent: Option<WindowId>,
    composition: Composition,
    records: Arc<CountingRecords>,
    _hold: FakeHold,
}

/// Pattern: Fake (test double) – a [`WindowOwner`] that keeps counters instead of windows.
///
/// Abweichung: besides the contract it has reading helpers for the tests (`composition_of`, `records_of`, `invoke`, `holds`).
pub struct FakeWindows {
    catalog: TestCatalog,
    known: HashMap<FrontendKey, View>,
    refusing: Vec<FrontendKey>,
    attached: AtomicUsize,
    detached: Arc<AtomicUsize>,
    open: Mutex<HashMap<WindowId, Entry>>,
}

impl FakeWindows {
    /// A fake that composes with `catalog` and knows no frontend yet.
    #[must_use]
    pub fn new(catalog: TestCatalog) -> Self {
        Self {
            catalog,
            known: HashMap::new(),
            refusing: Vec::new(),
            attached: AtomicUsize::new(0),
            detached: Arc::new(AtomicUsize::new(0)),
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
        self.lock().get(id).map(|e| Arc::downgrade(&e.records))
    }

    /// A call of the child frontend `id`: it reaches the records of that frontend only. `None` when `id` is not open.
    #[must_use]
    pub fn invoke(&self, id: &WindowId, form: &str, channel: &str) -> Option<Answer> {
        let records = self.lock().get(id).map(|e| Arc::clone(&e.records))?;
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
        (
            self.attached.load(Ordering::SeqCst),
            self.detached.load(Ordering::SeqCst),
        )
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<WindowId, Entry>> {
        self.open
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl WindowOwner for FakeWindows {
    fn open(&self, req: OpenFrontend) -> Result<WindowId, Coded> {
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
        let id = WindowId::of(&program, &elem);
        if let Some(e) = open.get_mut(&id) {
            e.raised += 1; // brought to the front, the mode stays
            return Ok(id);
        }
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
        let entry = Entry {
            key: req.key.clone(),
            mode: req.mode,
            switchable: req.switchable,
            raised: 0,
            parent,
            composition: compose(view, &self.catalog),
            records: Arc::new(CountingRecords::default()),
            _hold: FakeHold::attach(&self.attached, &self.detached),
        };
        open.insert(id.clone(), entry);
        Ok(id)
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
        let mut now: Vec<OpenedFrontend> = self
            .lock()
            .iter()
            .map(|(id, e)| OpenedFrontend {
                id: id.clone(),
                key: e.key.clone(),
                mode: e.mode,
                parent: e.parent.clone(),
            })
            .collect();
        now.sort_by(|a, b| a.id.cmp(&b.id));
        now
    }
}
