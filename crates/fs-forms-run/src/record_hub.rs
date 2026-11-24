//! The read cache that lets every view of one record show the same: two windows (or two frontends) that show the same record of the
//! same module read through ONE cached [`Records`], so after a change both show the same. The module stays the only source of truth.
//!
//! The hub is ONLY a read cache. It has no write path. Every view (grip) holds its OWN connection to the module (its hold), watches
//! the record itself and writes only through that connection with the revision it saw. A neighbour window, even of the same process or
//! the same person, learns of a change only through its own `watch` (stage 6 NACHTRAG-2 A+: two instances are two participants).
//!
//! Pattern: Identity Map (as a read cache only) + Observer over `watch` – one cached `Records` per (range, scope); the views share
//! the cache, never a connection. (The Multiton of the views is O1: one surface per id; this is the one object below them.)
//!
//! Where the instance id joins later: the instance IS the connection ([`Link`], the hold; the module knows it as `PerConnection` plus
//! `origin.device`); `WindowId` stays the display name of the place. `introduce` per link and the presence display come with
//! stage 8; nothing of it is built here, only no two links are ever merged.

pub mod fake;
mod grip;
mod port;
mod shared;

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError, Weak};

use self::grip::{GripSink, Inner};
pub use self::port::{ChangeSink, Link, Module};
use self::shared::Shared;
use crate::records::Records;
use crate::repaint::Repaint;
use crate::view::Coded;

/// What identifies the cached records: the range of the module (the program whose records they are) and the record (`scope`, an AS2
/// id; `None` = no record chosen). Another `scope` is another record and shares nothing.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RecordKey {
    /// The range of the module (its program id).
    pub range: String,
    /// The record shown (AS2 id), `None` without one.
    pub scope: Option<String>,
}

/// Pattern: Identity Map (read cache) – the process-wide table from [`RecordKey`] to the one cached records; it keeps only a `Weak`, so
/// the last [`RecordGrip`] that falls lets go of the cache.
#[derive(Default)]
pub struct RecordHub {
    open: Mutex<HashMap<RecordKey, Weak<Shared>>>,
    /// Told after every change a view's own `watch` reports (the window of that view draws again); none = nobody to wake.
    repaint: Option<Arc<dyn Repaint>>,
}

impl RecordHub {
    /// An empty hub.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A hub whose views wake their window through `repaint` after their own `watch` told them of a change.
    #[must_use]
    pub fn with_repaint(repaint: Arc<dyn Repaint>) -> Self {
        Self {
            open: Mutex::default(),
            repaint: Some(repaint),
        }
    }

    /// A grip with its OWN new connection to `module`, watching `key`; the cache of the key is made only when it is not open yet
    /// (under the lock of the hub; `module.records` must not call the hub).
    ///
    /// # Errors
    /// What the connection says; then nothing is registered.
    pub fn attach(&self, key: &RecordKey, module: &Arc<dyn Module>) -> Result<RecordGrip, Coded> {
        let sink = Arc::new(GripSink::with_repaint(self.repaint.clone()));
        let link = module.connect(Arc::clone(&sink) as Arc<dyn ChangeSink>)?;
        let shared = self.cache_of(key, module, &link);
        sink.point_at(&shared);
        Self::watch_then_read(&link, key.scope.as_deref(), &shared, &sink);
        let seen = AtomicU64::new(shared.rev());
        Ok(RecordGrip {
            inner: Arc::new(Inner {
                link,
                sink,
                shared: Mutex::new(shared),
                seen,
            }),
        })
    }

    /// The same grip, the same connection (the hold stays), now on another record of the same module: the cache of the new key is
    /// taken first, then the old one is let go (it falls when no other view shows it). On an error nothing changes.
    ///
    /// # Errors
    /// `host-notice-record-other-range` when `key` belongs to another module.
    pub fn rebind(
        &self,
        grip: &RecordGrip,
        key: RecordKey,
        module: &Arc<dyn Module>,
    ) -> Result<(), Coded> {
        let inner = &grip.inner;
        if inner.shared().key().range != key.range {
            return Err(Coded::new("host-notice-record-other-range").with("range", key.range));
        }
        let shared = self.cache_of(&key, module, &inner.link);
        inner.sink.point_at(&shared);
        Self::watch_then_read(&inner.link, key.scope.as_deref(), &shared, &inner.sink);
        inner.seen.store(shared.rev(), Ordering::SeqCst);
        *inner.shared.lock().unwrap_or_else(PoisonError::into_inner) = shared;
        inner.sink.bump(); // the view shows another record: it draws again
        Ok(())
    }

    /// First `watch`, then read the revision (subscribe-then-read): a write between the first read of the cache and the `watch` would
    /// never be told to the view; so after the `watch` the module is asked once more and a newer revision is taken as told.
    fn watch_then_read(
        link: &Arc<dyn Link>,
        scope: Option<&str>,
        shared: &Shared,
        sink: &GripSink,
    ) {
        link.watch(scope);
        let now = link.rev(scope);
        if now > shared.rev() {
            sink.changed(now);
        }
    }

    fn cache_of(
        &self,
        key: &RecordKey,
        module: &Arc<dyn Module>,
        link: &Arc<dyn Link>,
    ) -> Arc<Shared> {
        let mut map = self.open.lock().unwrap_or_else(PoisonError::into_inner);
        map.retain(|_, w| w.strong_count() > 0);
        if let Some(shared) = map.get(key).and_then(Weak::upgrade) {
            return shared;
        }
        let rev = link.rev(key.scope.as_deref());
        let shared = Arc::new(Shared::new(
            key.clone(),
            module.records(key.scope.as_deref()),
            rev,
        ));
        map.insert(key.clone(), Arc::downgrade(&shared));
        shared
    }

    /// How many caches are open now (the keys with a grip alive).
    #[must_use]
    pub fn live(&self) -> usize {
        let map = self.open.lock().unwrap_or_else(PoisonError::into_inner);
        map.values().filter(|w| w.strong_count() > 0).count()
    }
}

/// Pattern: RAII + Proxy – ONE view's hold: its own connection, its seen revision and its generation. Dropping it detaches the
/// connection once the last handle falls (`Clone` is another handle on the SAME grip, not a second view); the cache falls with the
/// last grip that shows it.
#[derive(Clone)]
pub struct RecordGrip {
    inner: Arc<Inner>,
}

impl RecordGrip {
    /// What the view reads and writes: reads come from the shared cache, writes go only through the own connection and are refused
    /// with `conflict` when the view has not seen the latest revision.
    #[must_use]
    pub fn records(&self) -> Arc<dyn Records> {
        Arc::clone(&self.inner) as Arc<dyn Records>
    }

    /// The shared read cache of the key: the same object for every grip of the key, and it cannot write.
    #[must_use]
    pub fn cache(&self) -> Arc<dyn Records> {
        self.inner.shared() as Arc<dyn Records>
    }

    /// How often THIS view's own `watch` told it of a change; a view that saw another number draws again.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.inner.sink.generation()
    }

    /// The revision this view has seen (what its next write is checked against).
    #[must_use]
    pub fn seen(&self) -> u64 {
        self.inner.seen.load(Ordering::SeqCst)
    }

    /// The key the cache is shared under.
    #[must_use]
    pub fn key(&self) -> RecordKey {
        self.inner.shared().key().clone()
    }
}
