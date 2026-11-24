//! S3-6b: a wake before Blinc started (no context, no scheduler) does nothing and does not panic. Own test binary: both are
//! process globals and must stay unset here.
//!
//! Mutant: `try_get()` replaced by `Some(get())` or `try_get_scheduler` by `get_scheduler` -> red (panic).

use fs_forms_run::{BlincRepaint, Repaint};

#[test]
fn wake_before_blinc_started_does_not_panic() {
    BlincRepaint.wake();
    BlincRepaint.wake();
}
