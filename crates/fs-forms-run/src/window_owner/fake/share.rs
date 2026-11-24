//! The sharing of the double: one counting `Records` (and one `FakeHold`) per (program, record), through the same `RecordHub` the real
//! owner uses; the entries only hold a grip.
//!
//! Pattern: Identity Map (see `record_hub`) – the double has no records of its own per window.

use std::sync::{Arc, Weak};

use super::FakeWindows;
use super::parts::{CountingRecords, FakeHold};
use crate::record_hub::{RecordGrip, RecordKey};
use crate::records::Records;
use crate::view::Coded;

impl FakeWindows {
    /// The grip on the shared records of (`range`, `scope`) and the typed handle of the same object (for the test helpers).
    pub(super) fn grip_for(
        &self,
        range: &str,
        scope: Option<&str>,
    ) -> Result<(RecordGrip, Arc<CountingRecords>), Coded> {
        let key = RecordKey {
            range: range.to_owned(),
            scope: scope.map(str::to_owned),
        };
        let grip = self.hub.attach(key.clone(), || {
            let made = Arc::new(CountingRecords::held(FakeHold::attach(
                &self.attached,
                &self.detached,
            )));
            self.typed().insert(key.clone(), Arc::downgrade(&made));
            Ok(made as Arc<dyn Records>)
        })?;
        let counting = self
            .typed()
            .get(&key)
            .and_then(Weak::upgrade)
            .expect("the grip keeps the records alive");
        Ok((grip, counting))
    }

    /// The generation of the shared records an open frontend shows (a view that saw another number draws again). `None` when not open.
    #[must_use]
    pub fn generation_of(&self, id: &crate::window_owner::WindowId) -> Option<u64> {
        self.lock().get(id).map(|e| e.grip.generation())
    }

    /// How many shared records are alive now (one per record, however many frontends show it).
    #[must_use]
    pub fn records_alive(&self) -> usize {
        self.hub.live()
    }

    fn typed(&self) -> std::sync::MutexGuard<'_, super::Typed> {
        self.typed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
