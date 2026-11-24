//! S3-6a: the history of records is locked by default, and the change counter moves a watch exactly once per change.
//!
//! Tests: example-based. Mutant: `Records::history` default with a step -> red; `grew` not taking the value -> red.

use fs_components_kanban::Answer;
use fs_forms_run::{ChangeCounter, ChangeWatch, NoRecords, Records, SideItem};

struct Counting(ChangeCounter);

impl Records for Counting {
    fn side(&self) -> Vec<SideItem> {
        Vec::new()
    }
    fn changed(&self) -> u64 {
        self.0.get()
    }
}

#[test]
fn history_and_step_back_are_locked_by_default() {
    let h = NoRecords.history();
    assert_eq!((h.undoable, h.redoable), (0, 0));
    assert_eq!(
        NoRecords.step_back(false),
        Answer::Refused("unsupported".into())
    );
    assert_eq!(
        NoRecords.step_back(true),
        Answer::Refused("unsupported".into())
    );
    assert_eq!(NoRecords.changed(), 0);
}

#[test]
fn watch_sees_each_change_once_and_clones_share_the_counter() {
    let counter = ChangeCounter::new();
    let records = Counting(counter.clone());
    let mut watch = ChangeWatch::new();
    assert!(!watch.grew(&records));
    counter.bump();
    assert_eq!(counter.get(), 1);
    assert!(watch.grew(&records));
    assert!(!watch.grew(&records));
    counter.bump();
    counter.bump();
    assert!(watch.grew(&records));
    assert!(!watch.grew(&records));
}
