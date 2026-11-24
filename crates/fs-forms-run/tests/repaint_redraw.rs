//! S3-6b: a wake asks the scheduler of the frame loop for a redraw (the loop would not wake otherwise). Own test binary: the
//! global scheduler is set once per process. Not checked in a real window (picture 2).
//!
//! Mutant: `request_redraw` removed from `BlincRepaint::wake` -> red.

use fs_components::blinc::animation::{AnimationScheduler, set_global_scheduler};
use fs_forms_run::{BlincRepaint, Repaint};

#[test]
fn wake_asks_the_scheduler_for_a_redraw() {
    let scheduler = AnimationScheduler::new();
    set_global_scheduler(scheduler.handle());
    assert!(!scheduler.take_needs_redraw());
    BlincRepaint.wake();
    assert!(scheduler.take_needs_redraw(), "wake must request a redraw");
    assert!(!scheduler.take_needs_redraw());
}
