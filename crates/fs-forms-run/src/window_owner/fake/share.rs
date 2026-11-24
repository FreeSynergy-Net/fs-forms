//! The sharing of the double: one counting read cache per (program, record) through the same `RecordHub` the real owner uses; every
//! entry holds a grip with its OWN connection to the fake module of its program.
//!
//! Pattern: Identity Map (read cache, see `record_hub`) – the double has no records of its own per window and no shared connection.

use std::sync::{Arc, Weak};

use super::FakeWindows;
use super::parts::CountingRecords;
use crate::record_hub::fake::FakeModule;
use crate::record_hub::{Module, RecordGrip, RecordKey};
use crate::records::Records;
use crate::view::Coded;

impl FakeWindows {
    /// The fake module of `range` (made on first use, gone when no grip or entry holds it).
    fn module_of(&self, range: &str) -> Arc<FakeModule> {
        let mut modules = self
            .modules
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(m) = modules.get(range).and_then(Weak::upgrade) {
            return m;
        }
        let typed = Arc::clone(&self.typed);
        let name = range.to_owned();
        let made = FakeModule::new(
            Arc::clone(&self.counts),
            Box::new(move |scope| {
                let made = Arc::new(CountingRecords::default());
                typed
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .insert(
                        RecordKey {
                            range: name.clone(),
                            scope: scope.map(str::to_owned),
                        },
                        Arc::downgrade(&made),
                    );
                made as Arc<dyn Records>
            }),
        );
        modules.insert(range.to_owned(), Arc::downgrade(&made));
        made
    }

    /// A grip with its own new connection on (`range`, `scope`), and the module it connected to.
    pub(super) fn grip_for(
        &self,
        range: &str,
        scope: Option<&str>,
    ) -> Result<(RecordGrip, Arc<dyn Module>), Coded> {
        let module: Arc<dyn Module> = self.module_of(range);
        let key = RecordKey {
            range: range.to_owned(),
            scope: scope.map(str::to_owned),
        };
        let grip = self.hub.attach(&key, &module)?;
        Ok((grip, module))
    }

    /// The generation of the open frontend: how often ITS OWN `watch` told it of a change (a view that saw another number draws again).
    /// `None` when not open.
    #[must_use]
    pub fn generation_of(&self, id: &crate::window_owner::WindowId) -> Option<u64> {
        self.lock().get(id).map(|e| e.grip.generation())
    }

    /// The grip of an open frontend (its records, cache, seen revision), for the tests. `None` when not open.
    #[must_use]
    pub fn grip_of(&self, id: &crate::window_owner::WindowId) -> Option<RecordGrip> {
        self.lock().get(id).map(|e| e.grip.clone())
    }

    /// How many read caches are alive now (one per record, however many frontends show it).
    #[must_use]
    pub fn records_alive(&self) -> usize {
        self.hub.live()
    }

    pub(super) fn typed(&self) -> std::sync::MutexGuard<'_, super::Typed> {
        self.typed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
