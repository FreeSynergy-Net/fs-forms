//! The change counter of records: the one number a window compares to know that the records changed behind its back (another
//! window, another program) and that it should read them again. It only counts and compares; it draws nothing and wakes nothing
//! (the wake-up is a separate port, S3-6b).
//!
//! Pattern: Observer (pull variant: the observer asks the counter instead of being called; nothing is pushed, a wake-up is a
//! separate port) with `ChangeWatch` as the observer and `ChangeCounter` as the subject.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::records::Records;

/// A counter that only grows. Clones share it, so a thread that learns of a change (the sink of a watch) and the records that
/// report it ([`Records::changed`]) hold the same one.
#[derive(Clone, Debug, Default)]
pub struct ChangeCounter(Arc<AtomicU64>);

impl ChangeCounter {
    /// A counter at 0.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Counts one change.
    pub fn bump(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }

    /// The value now.
    #[must_use]
    pub fn get(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

/// The last counter value a window saw: [`ChangeWatch::grew`] says once per change that the records must be read again.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChangeWatch {
    seen: u64,
}

impl ChangeWatch {
    /// A watch that has seen nothing (records that never change stay at 0 and never trigger).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the counter of `records` differs from the last value seen; the value is taken, so the next call is `false` until
    /// the counter moves again.
    pub fn grew(&mut self, records: &dyn Records) -> bool {
        let now = records.changed();
        let moved = now != self.seen;
        self.seen = now;
        moved
    }
}
