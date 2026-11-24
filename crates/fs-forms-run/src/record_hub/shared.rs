//! The read cache behind the grips of one key: one object per (range, scope), the last state read from the module. It only reads: it
//! has no write path, and a change reaches it only through [`Shared::observe`], which a view's own `watch` calls.
//!
//! Pattern: Identity Map (read cache) – around a Decorator that exposes the reading half of the records and nothing else.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use fs_components_kanban::Operation;

use super::RecordKey;
use crate::form_data::FormData;
use crate::records::{Records, SideItem};

/// The cache of one key, with the revision of the module it shows.
pub(super) struct Shared {
    key: RecordKey,
    cache: Arc<dyn Records>,
    rev: AtomicU64,
}

impl Shared {
    pub(super) fn new(key: RecordKey, cache: Arc<dyn Records>, rev: u64) -> Self {
        Self {
            key,
            cache,
            rev: AtomicU64::new(rev),
        }
    }

    pub(super) fn key(&self) -> &RecordKey {
        &self.key
    }

    pub(super) fn rev(&self) -> u64 {
        self.rev.load(Ordering::SeqCst)
    }

    /// The module says the record is at `rev` (told by a `watch`): read again, once per revision however many views were told. If
    /// reading fails the records say so themselves ([`Records::stale`]).
    pub(super) fn observe(&self, rev: u64) {
        if self.rev.fetch_max(rev, Ordering::SeqCst) < rev {
            let _ = self.cache.reread();
        }
    }
}

/// Reading only: every write method keeps the default of the trait ("cannot"), so nobody writes through the cache.
impl Records for Shared {
    fn side(&self) -> Vec<SideItem> {
        self.cache.side()
    }
    fn forms(&self) -> Arc<Vec<FormData>> {
        self.cache.forms()
    }
    fn command_of(&self, form: &str, channel: &str) -> Option<String> {
        self.cache.command_of(form, channel)
    }
    fn can_channel(&self, form: &str, channel: &str, record: Option<&str>) -> bool {
        self.cache.can_channel(form, channel, record)
    }
    fn is_sample(&self) -> bool {
        self.cache.is_sample()
    }
    fn stale(&self) -> Option<String> {
        self.cache.stale()
    }
    fn unavailable(&self) -> Option<String> {
        self.cache.unavailable()
    }
    fn can(&self, op: Operation) -> bool {
        self.cache.can(op)
    }
}
