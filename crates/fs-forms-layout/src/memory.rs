//! `MemoryLayoutStore`: the fake of the port, in memory. Every write runs on a copy of the state and replaces it only when it
//! succeeded and mode is `Apply`: one transaction each, "error means nothing changed, stamp equal" (D3) for free.
//! The traits are implemented in `memory_read`, `memory_write`, `memory_overlay` and `memory_admin`.
//!
//! Pattern: Fake (a second adapter of the port) with a unit of work (copy, change, swap).

use std::cell::RefCell;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::check::CatalogRules;
use crate::codes::LayoutCode;
use crate::ids::{ElemId, FrontendKey, LayoutId, OverlayId, PackageRef, Rev, Who};
use crate::layout::Layout;
use crate::limits::Limits;
use crate::overlay::{Conflict, ElemLock, Overlay};
use crate::port::Pending;
use crate::shim::{Coded, ObjectId, Reference, Utc};
use crate::types::{LogAction, LogEntry, Origin};

/// One stored layout row. A tombstone has `deleted` set and no layout; its key stays as the identity.
#[derive(Clone)]
pub(crate) struct Stored {
    pub id: LayoutId,
    pub key: FrontendKey,
    pub origin: Origin,
    pub package: Option<PackageRef>,
    pub owner: Option<Reference>,
    pub rev: Rev,
    pub updated: Utc,
    pub deleted: Option<Utc>,
    pub layout: Option<Layout>,
}

#[derive(Clone, Default)]
pub(crate) struct State {
    pub stamp: u64,
    pub next: u64,
    pub layouts: Vec<Stored>,
    pub overlays: Vec<Overlay>,
    pub conflicts: Vec<Conflict>,
    pub locks: Vec<(LayoutId, ElemLock)>,
    pub log: Vec<LogEntry>,
}

/// The in-memory store. `actor` is the person of the service; `instance` switches the admin port on.
pub struct MemoryLayoutStore {
    pub(crate) st: RefCell<State>,
    pub(crate) catalog: Box<dyn CatalogRules>,
    pub(crate) limits: Limits,
    pub(crate) actor: Reference,
    pub(crate) instance: bool,
    clock: Box<dyn Fn() -> Utc>,
}

pub(crate) type Res<T> = Result<T, Coded<LayoutCode>>;

pub(crate) fn ready<'a, T: 'a>(r: Res<T>) -> Pending<'a, T> {
    Box::pin(std::future::ready(r))
}

impl MemoryLayoutStore {
    /// A store whose clock is the system clock (the service sets all times; callers never do).
    #[must_use]
    pub fn new(catalog: Box<dyn CatalogRules>, limits: Limits, actor: Reference) -> Self {
        let clock = || {
            let ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_millis());
            Utc(i64::try_from(ms).unwrap_or(i64::MAX))
        };
        Self {
            st: RefCell::default(),
            catalog,
            limits,
            actor,
            instance: false,
            clock: Box::new(clock),
        }
    }
    /// Replaces the clock (tests feed a counter).
    #[must_use]
    pub fn with_clock(mut self, clock: Box<dyn Fn() -> Utc>) -> Self {
        self.clock = clock;
        self
    }
    /// Makes this the instance service: the admin port works.
    #[must_use]
    pub fn as_instance(mut self) -> Self {
        self.instance = true;
        self
    }
    pub(crate) fn now(&self) -> Utc {
        (self.clock)()
    }

    /// Like [`tx`](Self::tx) for writes of a person: an acting party that is not the person it acts for (an AI on behalf)
    /// is refused until the commission can be checked (`unsupported{what=on-behalf}`); an AI is never the owner.
    pub(crate) fn tx_for<T>(
        &self,
        who: &Who,
        apply: bool,
        f: impl FnOnce(&mut State, Utc) -> Res<(T, bool)>,
    ) -> Res<T> {
        if who.actor != who.author {
            return Err(Coded::new(LayoutCode::Unsupported).with("what", "on-behalf"));
        }
        self.tx(apply, f)
    }

    /// Runs a change on a copy. Commits (stamp + 1) only when `f` returns `Ok((value, true))` and `apply` is set.
    pub(crate) fn tx<T>(
        &self,
        apply: bool,
        f: impl FnOnce(&mut State, Utc) -> Res<(T, bool)>,
    ) -> Res<T> {
        let mut copy = self.st.borrow().clone();
        let (value, ok) = f(&mut copy, self.now())?;
        if ok && apply {
            copy.stamp += 1;
            *self.st.borrow_mut() = copy;
        }
        Ok(value)
    }
}

impl State {
    pub(crate) fn new_layout_id(&mut self) -> LayoutId {
        self.next += 1;
        LayoutId(ObjectId(format!("layout-{:06}", self.next)))
    }
    pub(crate) fn new_overlay_id(&mut self) -> OverlayId {
        self.next += 1;
        OverlayId(ObjectId(format!("overlay-{:06}", self.next)))
    }
    pub(crate) fn live(&self, id: &LayoutId) -> Option<usize> {
        self.layouts
            .iter()
            .position(|s| &s.id == id && s.deleted.is_none())
    }
    pub(crate) fn live_key(&self, key: &FrontendKey) -> Option<usize> {
        self.layouts
            .iter()
            .position(|s| &s.key == key && s.deleted.is_none())
    }
    pub(crate) fn locks_of(&self, id: &LayoutId) -> Vec<ElemLock> {
        self.locks
            .iter()
            .filter(|(l, _)| l == id)
            .map(|(_, k)| k.clone())
            .collect()
    }
    /// Appends one log row (never changed afterwards).
    pub(crate) fn log(&mut self, at: Utc, who: &Who, action: LogAction) -> &mut LogEntry {
        let id = ObjectId(format!("log-{:06}", self.log.len() + 1));
        self.log.push(LogEntry {
            id,
            at,
            who: who.clone(),
            os_user: None,
            action,
            layout: None,
            overlay: None,
            elem: None,
            rev_before: None,
            rev_after: None,
            package: None,
            count: None,
            reason: None,
        });
        self.log
            .last_mut()
            .unwrap_or_else(|| unreachable!("just pushed"))
    }
}

pub(crate) fn unknown(what: &str, id: &str) -> Coded<LayoutCode> {
    Coded::new(LayoutCode::Unknown)
        .with("what", what)
        .with("id", id)
}
pub(crate) fn rev_changed(id: &str, expected: Rev, found: Rev) -> Coded<LayoutCode> {
    Coded::new(LayoutCode::RevChanged)
        .with("id", id)
        .with("expected", expected.0)
        .with("found", found.0)
}
pub(crate) fn elem_str(e: &Option<ElemId>) -> &str {
    e.as_ref().map_or("", ElemId::as_str)
}
