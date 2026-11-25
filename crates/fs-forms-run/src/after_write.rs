//! The hook "a write was accepted": the window sets it, the gate of the board fires it after every accepted gesture, so the menu
//! "Edit" is set again (a card moved = "Undo" is no longer locked).
//!
//! Pattern: Observer (one observer: the window; the subject is the [`crate::write_gate::WriteGate`]). The cell is shared, so a
//! page that is drawn again (another frontend) keeps reaching the same window.

use std::sync::{Arc, Mutex, PoisonError};

type Action = Arc<dyn Fn() + Send + Sync>;

/// The shared cell of the hook; empty = nobody listens.
#[derive(Clone, Default)]
pub struct AfterWrite(Arc<Mutex<Option<Action>>>);

impl AfterWrite {
    /// Sets (or replaces) what runs after an accepted write.
    pub fn set(&self, action: impl Fn() + Send + Sync + 'static) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Some(Arc::new(action));
    }

    /// Lets go of the action (the window is closed).
    pub fn clear(&self) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }

    /// Runs the action, if one is set; the lock is not held while it runs.
    pub fn fire(&self) {
        let action = self
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        if let Some(action) = action {
            action();
        }
    }
}
