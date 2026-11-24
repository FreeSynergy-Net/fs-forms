//! The wake-up of a window: the one port a thread that learns of a change (the sink of a `watch`) calls so that the window draws
//! the records again. The port only wakes; it compares nothing and draws nothing. The window compares the change counter
//! ([`crate::ChangeWatch`]) when it is built again and reads the records again only if it grew ([`crate::ChangeWatch::read_if_grew`]).
//!
//! Pattern: Ports and Adapters – the host and the sinks know only the port; what Blinc needs to build a window again stays in one place.

use fs_components::blinc::animation::try_get_scheduler;
use fs_components::blinc::core::BlincContextState;

/// Pattern: Port (Ports and Adapters) – wakes the window of the interface so it is built and drawn again. Callable from any
/// thread, any number of times (a wake that finds nothing changed costs one build, no more).
///
/// **Order: first count, then wake** ([`crate::ChangeCounter::bump`], then `wake`). The other way round loses a change: the frame
/// takes the flag, builds, [`crate::ChangeWatch::grew`] still sees the old counter, the `bump` comes after and nobody wakes again,
/// so the change stays unseen until some other rebuild. Counting first is safe (the take of the flag lies after the `bump`).
pub trait Repaint: Send + Sync {
    /// Asks for a new build and a new frame of the window. Never blocks and never panics on its own (without a window nothing
    /// happens; Blinc takes a lock inside, a poisoned lock there is Blinc's).
    fn wake(&self);
}

/// The Blinc adapter of [`Repaint`]: calls `BlincContextState::request_rebuild` (an atomic flag, the one `ctx.request_rebuild`
/// sets, through `try_get`, so nothing panics before Blinc started) and `SchedulerHandle::request_redraw` (wakes the loop).
///
/// Not checked in a real window yet (picture 2). Two limits to know: all windows of ONE process share this one flag and the first
/// window that checks it takes it, so only that one is built again (a risk for picture 2 if A and B are windows of the same
/// process, none if they are separate processes); and the flag is the one of the context Blinc's window created, so the context
/// must not have been created earlier by something else with another flag.
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
