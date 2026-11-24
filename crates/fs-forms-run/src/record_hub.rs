//! The hub that lets every view of one record share ONE [`Records`]: two windows (or two frontends) that show the same record of the
//! same module read through the same object, so after a change both show the same. The module stays the only source of truth; the
//! hub keeps the last state it read (one copy per process, read again after every change, never written by itself).
//!
//! Pattern: Identity Map – one `Records` per (range, scope) in the process; the views share it and keep no data of their own.
//! (The Multiton of the views is O1: one surface per id; this is the one object below them.)

mod shared;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError, Weak};

use self::shared::Shared;
use crate::records::Records;
use crate::view::Coded;

/// What identifies the shared records: the range of the module (the program whose records they are) and the record (`scope`, an
/// AS2 id; `None` = no record chosen). Another `scope` is another record and shares nothing.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RecordKey {
    /// The range of the module (its program id).
    pub range: String,
    /// The record shown (AS2 id), `None` without one.
    pub scope: Option<String>,
}

/// Pattern: Identity Map – the process-wide table from [`RecordKey`] to the one shared records; it keeps only a `Weak`, so the
/// last [`RecordGrip`] that falls lets go of everything.
#[derive(Default)]
pub struct RecordHub {
    open: Mutex<HashMap<RecordKey, Weak<Shared>>>,
}

impl RecordHub {
    /// An empty hub.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The grip on the shared records of `key`; `open` runs only when the key is not open yet (it runs under the lock of the hub
    /// and must not call the hub). The records come back wrapped: an accepted call reads them again and counts the generation.
    ///
    /// # Errors
    /// What `open` says; then nothing is registered.
    pub fn attach(
        &self,
        key: RecordKey,
        open: impl FnOnce() -> Result<Arc<dyn Records>, Coded>,
    ) -> Result<RecordGrip, Coded> {
        let mut map = self.open.lock().unwrap_or_else(PoisonError::into_inner);
        map.retain(|_, w| w.strong_count() > 0);
        if let Some(shared) = map.get(&key).and_then(Weak::upgrade) {
            return Ok(RecordGrip { shared });
        }
        let shared = Arc::new(Shared::new(key.clone(), open()?));
        map.insert(key, Arc::downgrade(&shared));
        Ok(RecordGrip { shared })
    }

    /// How many records are open now (the keys with a grip alive).
    #[must_use]
    pub fn live(&self) -> usize {
        let map = self.open.lock().unwrap_or_else(PoisonError::into_inner);
        map.values().filter(|w| w.strong_count() > 0).count()
    }
}

/// Pattern: RAII + Observer – one view's hold on the shared records (like `fs_rpc::hold`): the last grip that falls closes records,
/// hold and (later) the `watch` subscription; [`RecordGrip::generation`] is what a view compares to know it must draw again.
pub struct RecordGrip {
    shared: Arc<Shared>,
}

impl RecordGrip {
    /// The shared records (the same object for every grip of the key).
    #[must_use]
    pub fn records(&self) -> Arc<dyn Records> {
        Arc::clone(&self.shared) as Arc<dyn Records>
    }

    /// How often the records changed and were read again; a view that saw another number draws again.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.shared.generation()
    }

    /// The key these records are shared under.
    #[must_use]
    pub fn key(&self) -> &RecordKey {
        self.shared.key()
    }
}
