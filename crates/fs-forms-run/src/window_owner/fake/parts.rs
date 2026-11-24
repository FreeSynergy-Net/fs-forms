//! The counting parts of the double (`FakeWindows`): stand-in records and the stand-in for the held module connection.
//!
//! Pattern: Fake (test double) – counting parts of the double, held in memory.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use fs_components_kanban::Answer;

use crate::records::{Records, SideItem};

/// Pattern: Fake (test double) – stand-in for the records of one module that counts the calls it takes.
#[derive(Debug, Default)]
pub struct CountingRecords {
    calls: AtomicUsize,
}

impl CountingRecords {
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
}

/// Pattern: RAII (test double) – stands for the held connection to a module (`fs_rpc::hold`, not built yet): attached when made,
/// detached when dropped.
#[derive(Debug)]
pub struct FakeHold {
    detached: Arc<AtomicUsize>,
}

impl FakeHold {
    pub(super) fn attach(attached: &AtomicUsize, detached: &Arc<AtomicUsize>) -> Self {
        attached.fetch_add(1, Ordering::SeqCst);
        Self {
            detached: Arc::clone(detached),
        }
    }
}

impl Drop for FakeHold {
    fn drop(&mut self) {
        self.detached.fetch_add(1, Ordering::SeqCst);
    }
}
