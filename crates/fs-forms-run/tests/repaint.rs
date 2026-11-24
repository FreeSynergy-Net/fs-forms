//! S3-6b: a wake sets the rebuild flag of the Blinc frame loop (the trigger of the window being built again; the log line `rebuild_source` and
//! a real window are NOT checked, picture 2); the build
//! reads the records again only when the change counter grew. Own test binary: the Blinc context is a process global.
//!
//! Mutant: `BlincRepaint::wake` without `request_rebuild` -> `wake_sets_the_rebuild_flag` red; `read_if_grew` reading without
//! `grew` -> `a_wake_without_a_change_reads_nothing` red.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use fs_components::blinc::core::{BlincContextState, HookState, ReactiveGraph};
use fs_forms_run::{BlincRepaint, ChangeCounter, ChangeWatch, Coded, Records, Repaint, SideItem};

struct Counting {
    counter: ChangeCounter,
    reads: AtomicUsize,
}

impl Records for Counting {
    fn side(&self) -> Vec<SideItem> {
        Vec::new()
    }
    fn changed(&self) -> u64 {
        self.counter.get()
    }
    fn reread(&self) -> Result<(), Coded> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[test]
fn wake_sets_the_rebuild_flag_from_another_thread_and_a_change_reads_once() {
    let flag = Arc::new(AtomicBool::new(false));
    BlincContextState::init(
        Arc::new(Mutex::new(ReactiveGraph::new())),
        Arc::new(Mutex::new(HookState::default())),
        Arc::clone(&flag),
    );
    let repaint: Arc<dyn Repaint> = Arc::new(BlincRepaint);
    assert!(!flag.load(Ordering::SeqCst));

    // a wake without a change: the frame loop builds again, the records are not read
    let records = Counting {
        counter: ChangeCounter::new(),
        reads: AtomicUsize::new(0),
    };
    let mut watch = ChangeWatch::new();
    let r = Arc::clone(&repaint);
    std::thread::spawn(move || r.wake()).join().unwrap();
    assert!(
        flag.swap(false, Ordering::SeqCst),
        "wake must request a rebuild"
    );
    assert_eq!(watch.read_if_grew(&records), Ok(false));
    assert_eq!(records.reads.load(Ordering::SeqCst), 0);

    // a change then a wake: rebuild and exactly one read
    records.counter.bump();
    repaint.wake();
    assert!(flag.swap(false, Ordering::SeqCst));
    assert_eq!(watch.read_if_grew(&records), Ok(true));
    assert_eq!(watch.read_if_grew(&records), Ok(false));
    assert_eq!(records.reads.load(Ordering::SeqCst), 1);
}
