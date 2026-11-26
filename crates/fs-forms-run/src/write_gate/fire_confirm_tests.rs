//! An action with `confirm` is not carried out on the gesture path: no dialog exists yet (G8), so it is refused with its own code.
//!
//! Pattern: none (tests).

use std::sync::Arc;

use fs_components_core::form_write::{FormWrite, Gesture};

use super::fire_tests::{PLACE, action, gate};
use crate::write::fixture::{FakeWriter, REMOVE, first};

#[test]
fn an_action_with_confirm_is_refused_with_its_own_code_and_the_module_is_not_asked() {
    let w = Arc::new(FakeWriter::default());
    let mut delete = action(Some(PLACE), "menu", REMOVE);
    delete.confirm = true;
    let g = gate(vec![delete], &w);
    let may = g
        .may_fire(Gesture::menu_entry(REMOVE), Some(&first()))
        .unwrap_err();
    assert_eq!(may.code, "confirm-unsupported");
    let refused = g
        .fire(Gesture::menu_entry(REMOVE), Some(&first()), &[])
        .unwrap_err();
    assert_eq!(refused.code, "confirm-unsupported");
    assert!(w.calls.lock().unwrap().is_empty());
}

#[test]
fn an_action_without_confirm_still_runs() {
    let w = Arc::new(FakeWriter::default());
    let g = gate(vec![action(Some(PLACE), "menu", REMOVE)], &w);
    g.fire(Gesture::menu_entry(REMOVE), Some(&first()), &[])
        .expect("fires");
    assert_eq!(w.calls.lock().unwrap().len(), 1);
}
