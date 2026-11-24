//! The grip of ONE view: its own connection (the hold), its own seen revision and its own generation, over the shared read cache.
//! Reads go to the cache; writes go ONLY through the own connection and only when the view saw the latest revision.
//!
//! Pattern: Proxy (per view) – reads from the shared cache, writes through the own link with optimistic concurrency (`rev`); Observer –
//! the generation counts what the view's own `watch` told.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError, Weak};

use fs_components_kanban::{Answer, Operation};

use super::port::{ChangeSink, Link};
use super::shared::Shared;
use crate::form_data::FormData;
use crate::records::{Records, SideItem};

/// What the own `watch` of a view does: read the cache again (once per revision) and count the generation of THIS view.
#[derive(Default)]
pub(super) struct GripSink {
    cache: Mutex<Weak<Shared>>,
    generation: AtomicU64,
}

impl GripSink {
    pub(super) fn point_at(&self, shared: &Arc<Shared>) {
        *self.cache.lock().unwrap_or_else(PoisonError::into_inner) = Arc::downgrade(shared);
    }

    pub(super) fn bump(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
    }

    pub(super) fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }
}

impl ChangeSink for GripSink {
    fn changed(&self, rev: u64) {
        let shared = self
            .cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .upgrade();
        if let Some(s) = shared {
            s.observe(rev);
        }
        self.bump();
    }
}

/// Everything one view holds; the `Records` it hands out is this object.
pub(super) struct Inner {
    pub(super) link: Arc<dyn Link>,
    pub(super) sink: Arc<GripSink>,
    pub(super) shared: Mutex<Arc<Shared>>,
    pub(super) seen: AtomicU64,
}

impl Inner {
    pub(super) fn shared(&self) -> Arc<Shared> {
        Arc::clone(&self.shared.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// A write on the own connection: refused with `conflict` when the module is ahead of what this view saw (the same rule as between
    /// two people); after an accepted write the view has seen its own change.
    fn write(&self, f: impl FnOnce(&dyn Records) -> Answer) -> Answer {
        let scope = self.shared().key().scope.clone();
        if self.seen.load(Ordering::SeqCst) < self.link.rev(scope.as_deref()) {
            return Answer::Refused("conflict".into());
        }
        let answer = f(&*self.link.writer(scope.as_deref()));
        if answer == Answer::Accepted {
            self.seen
                .store(self.link.rev(scope.as_deref()), Ordering::SeqCst);
        }
        answer
    }
}

impl Records for Inner {
    fn side(&self) -> Vec<SideItem> {
        self.shared().side()
    }
    /// What the view draws from: it has now seen the revision of this state.
    fn forms(&self) -> Arc<Vec<FormData>> {
        let shared = self.shared();
        self.seen.fetch_max(shared.rev(), Ordering::SeqCst);
        shared.forms()
    }
    fn command_of(&self, form: &str, channel: &str) -> Option<String> {
        self.shared().command_of(form, channel)
    }
    fn can_channel(&self, form: &str, channel: &str, record: Option<&str>) -> bool {
        self.shared().can_channel(form, channel, record)
    }
    fn invoke(
        &self,
        form: &str,
        channel: &str,
        record: Option<&str>,
        values: &[(String, String)],
    ) -> Answer {
        self.write(|w| w.invoke(form, channel, record, values))
    }
    fn is_sample(&self) -> bool {
        self.shared().is_sample()
    }
    fn stale(&self) -> Option<String> {
        self.shared().stale()
    }
    fn unavailable(&self) -> Option<String> {
        self.shared().unavailable()
    }
    fn can(&self, op: Operation) -> bool {
        self.shared().can(op)
    }
    fn add_card(&self, column: &str, title: &str) -> Answer {
        self.write(|w| w.add_card(column, title))
    }
    fn move_card(&self, key: &str, to: &str) -> Answer {
        self.write(|w| w.move_card(key, to))
    }
    fn add_column(&self, title: &str) -> Answer {
        self.write(|w| w.add_column(title))
    }
    fn rename_column(&self, column: &str, name: &str) -> Answer {
        self.write(|w| w.rename_column(column, name))
    }
    fn set_limit(&self, column: &str, limit: Option<u32>) -> Answer {
        self.write(|w| w.set_limit(column, limit))
    }
}
