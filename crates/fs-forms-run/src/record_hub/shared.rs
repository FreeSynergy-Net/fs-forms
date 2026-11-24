//! The one object behind every grip of a key: the records of the module, wrapped so that every accepted change reads them again and
//! counts the generation (the own-change path; the `watch` of the module takes the same way once it exists).
//!
//! Pattern: Observer (the views are told by the generation) around a Decorator that adds nothing but the re-read.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use fs_components_kanban::{Answer, Operation};

use super::RecordKey;
use crate::form_data::FormData;
use crate::records::{Records, SideItem};

/// The shared records of one key, with their generation.
pub(super) struct Shared {
    key: RecordKey,
    inner: Arc<dyn Records>,
    generation: AtomicU64,
}

impl Shared {
    pub(super) fn new(key: RecordKey, inner: Arc<dyn Records>) -> Self {
        Self {
            key,
            inner,
            generation: AtomicU64::new(0),
        }
    }

    pub(super) fn key(&self) -> &RecordKey {
        &self.key
    }

    pub(super) fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    /// After an accepted call the module has written: read again and count. If reading fails the records say so themselves
    /// ([`Records::stale`]); the generation counts anyway, because the module HAS changed and the views must ask again.
    fn after(&self, answer: Answer) -> Answer {
        if answer == Answer::Accepted {
            let _ = self.inner.reread();
            self.generation.fetch_add(1, Ordering::SeqCst);
        }
        answer
    }
}

impl Records for Shared {
    fn side(&self) -> Vec<SideItem> {
        self.inner.side()
    }
    fn forms(&self) -> Arc<Vec<FormData>> {
        self.inner.forms()
    }
    fn command_of(&self, form: &str, channel: &str) -> Option<String> {
        self.inner.command_of(form, channel)
    }
    fn can_channel(&self, form: &str, channel: &str, record: Option<&str>) -> bool {
        self.inner.can_channel(form, channel, record)
    }
    fn invoke(
        &self,
        form: &str,
        channel: &str,
        record: Option<&str>,
        values: &[(String, String)],
    ) -> Answer {
        self.after(self.inner.invoke(form, channel, record, values))
    }
    fn is_sample(&self) -> bool {
        self.inner.is_sample()
    }
    fn stale(&self) -> Option<String> {
        self.inner.stale()
    }
    fn unavailable(&self) -> Option<String> {
        self.inner.unavailable()
    }
    fn can(&self, op: Operation) -> bool {
        self.inner.can(op)
    }
    fn add_card(&self, column: &str, title: &str) -> Answer {
        self.after(self.inner.add_card(column, title))
    }
    fn move_card(&self, key: &str, to: &str) -> Answer {
        self.after(self.inner.move_card(key, to))
    }
    fn add_column(&self, title: &str) -> Answer {
        self.after(self.inner.add_column(title))
    }
    fn rename_column(&self, column: &str, name: &str) -> Answer {
        self.after(self.inner.rename_column(column, name))
    }
    fn set_limit(&self, column: &str, limit: Option<u32>) -> Answer {
        self.after(self.inner.set_limit(column, limit))
    }
    fn reread(&self) -> Result<(), crate::view::Coded> {
        let r = self.inner.reread();
        self.generation.fetch_add(1, Ordering::SeqCst);
        r
    }
}
