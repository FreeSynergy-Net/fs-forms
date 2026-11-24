//! The wake-up of a window: the one port a thread that learns of a change (the sink of a `watch`) calls so that the window draws
//! the records again. The port only wakes; it compares nothing and draws nothing. The window compares the change counter
//! ([`crate::ChangeWatch`]) when it is built again and reads the records again only if it grew ([`crate::ChangeWatch::read_if_grew`]).
//!
//! Pattern: Adapter (port `Repaint` with the Blinc adapter `BlincRepaint`) – the host and the sinks know only the port; what
//! Blinc needs to build a window again stays in one place.

use fs_components::blinc::animation::try_get_scheduler;
use fs_components::blinc::core::BlincContextState;

/// Wakes the window of the interface so it is built and drawn again. Callable from any thread, any number of times (a wake that
/// finds nothing changed costs one build, no more).
///
/// Pattern: Adapter (port) – the interface of the thread that owns the window, behind one call.
pub trait Repaint: Send + Sync {
    /// Asks for a new build and a new frame of the window. Never blocks, never fails (without a window nothing happens).
    fn wake(&self);
}

/// The Blinc adapter of [`Repaint`]: sets the rebuild flag of the frame loop (`blinc_core::request_rebuild`, an atomic flag, the
/// one `ctx.request_rebuild` sets) and wakes the loop (`SchedulerHandle::request_redraw`). Without Blinc started yet both are skipped.
#[derive(Clone, Copy, Debug, Default)]
pub struct BlincRepaint;

impl Repaint for BlincRepaint {
    fn wake(&self) {
        if let Some(state) = BlincContextState::try_get() {
            state.request_rebuild();
        }
        if let Some(scheduler) = try_get_scheduler() {
            scheduler.request_redraw();
        }
    }
}
