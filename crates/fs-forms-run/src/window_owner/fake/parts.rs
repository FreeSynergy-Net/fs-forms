//! The counting part of the double (`FakeWindows`): the stand-in records of one record of the module.
//!
//! Pattern: Fake (test double) – counting parts of the double, held in memory.

use std::sync::atomic::{AtomicUsize, Ordering};

use fs_components_kanban::Answer;

use crate::records::{Records, SideItem};
use crate::view::Coded;

/// Pattern: Fake (test double) – stand-in for the records of one module that counts the calls it takes.
#[derive(Debug, Default)]
pub struct CountingRecords {
    calls: AtomicUsize,
    rereads: AtomicUsize,
}

impl CountingRecords {
    /// How often these records were read again.
    #[must_use]
    pub fn rereads(&self) -> usize {
        self.rereads.load(Ordering::SeqCst)
    }

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

    fn reread(&self) -> Result<(), Coded> {
        self.rereads.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
