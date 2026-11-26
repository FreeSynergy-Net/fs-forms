//! A write accepted by the gate sets the menu "Edit" again (S3-9 package C round 2, MUST 1), and the window draws again only after a
//! step that was taken (K4).
//!
//! Tests: A1 a card moved through the gate of the window unlocks "Undo" (real `AppWidget`, `choose`); A2 a refused write leaves it
//! locked; A3 `wake` comes after `Done` and not after `Refused` (a click on a locked line never reaches the handler, so `Nothing` has
//! no path through `choose`; it is covered by `history::answer` in `tests/history.rs`).
//!
//! Pattern: none (tests).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use fs_components_app::AppWidget;
use fs_components_core::form_write::{FormWrite, Gesture};
use fs_components_kanban::{Answer, Reason};
use fs_components_words::Locale;
use fs_objects::UndoState;

use super::window_parts;
use crate::after_write::AfterWrite;
use crate::compose::{Composition, HostState};
use crate::history::{Edit, edit_menu_for};
use crate::records::{Records, SideItem};
use crate::repaint::Repaint;
use crate::style::StyleCell;
use crate::view::{ActionView, View};
use crate::words::words;
use crate::write_gate::WriteGate;

/// Records whose history grows with every accepted move; `reply` is the answer of a move and of a step back.
struct Moving {
    undoable: Mutex<usize>,
    reply: Answer,
}

impl Moving {
    fn new(undoable: usize, reply: Answer) -> Arc<Self> {
        Arc::new(Self {
            undoable: Mutex::new(undoable),
            reply,
        })
    }
}

impl Records for Moving {
    fn side(&self) -> Vec<SideItem> {
        Vec::new()
    }
    fn history(&self) -> UndoState {
        UndoState {
            undoable: *self.undoable.lock().unwrap(),
            redoable: 0,
        }
    }
    fn channel_of(&self, _form: &str, command: &str) -> Option<String> {
        (command == "card.move").then(|| "move".to_owned())
    }
    fn can_channel(&self, _form: &str, _channel: &str, _record: Option<&str>) -> bool {
        true
    }
    fn invoke(&self, _: &str, _: &str, _: Option<&str>, _: &[(String, String)]) -> Answer {
        if self.reply == Answer::Accepted {
            *self.undoable.lock().unwrap() += 1;
        }
        self.reply.clone()
    }
    fn step_back(&self, _redo: bool) -> Answer {
        self.reply.clone()
    }
}

#[derive(Default)]
struct Wakes(AtomicUsize);

impl Repaint for Wakes {
    fn wake(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

fn en() -> Locale {
    Locale::from_posix("en").expect("a locale")
}

fn gate(records: Arc<dyn Records>, after: AfterWrite) -> WriteGate {
    WriteGate {
        records,
        actions: Arc::new(vec![ActionView {
            trigger: "drop".into(),
            at: Some("block:board/main".into()),
            command: "card.move".into(),
            open: None,
            confirm: false,
        }]),
        form: "cards-in-columns".into(),
        block: "board".into(),
        place: "block:board/main".into(),
        after,
    }
}

fn undo_locked(w: &AppWidget) -> bool {
    w.with_shell(|s| {
        s.menus()
            .iter()
            .flat_map(|m| m.entries.iter())
            .find(|e| e.id == "app.undo")
            .expect("the line Undo")
            .disabled
    })
}

fn window(records: &Arc<Moving>) -> (AppWidget, AfterWrite) {
    let loc = en();
    let comp = Composition {
        state: HostState::Ready,
        screens: Vec::new(),
        lacking: Vec::new(),
        notices: Vec::new(),
    };
    window_parts(
        &words(&loc),
        &loc,
        &View::default(),
        &comp,
        records.clone(),
        Arc::new(StyleCell::default()),
        Arc::new(crate::clock::SystemClock),
    )
    .expect("a window")
}

#[test]
fn a1_a_card_moved_through_the_gate_unlocks_undo_in_the_real_window() {
    let records = Moving::new(0, Answer::Accepted);
    let (w, after) = window(&records);
    assert!(undo_locked(&w), "no step yet");
    assert!(!w.choose("app.undo"), "locked: a click does nothing");
    gate(records.clone(), after)
        .fire(Gesture::new("drop"), Some("FS-1"), &[])
        .expect("the move is accepted");
    assert!(!undo_locked(&w), "the move is a step: Undo is open");
    assert!(w.choose("app.undo"), "and the click reaches the records");
}

#[test]
fn a2_a_refused_write_leaves_undo_locked() {
    let records = Moving::new(0, Answer::Refused(Reason::new("conflict", "")));
    let (w, after) = window(&records);
    assert!(
        gate(records, after)
            .fire(Gesture::new("drop"), Some("FS-1"), &[])
            .is_err()
    );
    assert!(undo_locked(&w));
}

#[test]
fn a3_the_window_wakes_after_a_step_that_was_taken_not_after_a_refusal() {
    let wakes_after = |reply: Answer| {
        let records = Moving::new(1, reply);
        let (w, _) = window(&records);
        let wakes = Arc::new(Wakes::default());
        let for_menu = records.clone();
        let edit = Edit {
            records: records.clone(),
            locale: en(),
            repaint: wakes.clone(),
            menus: Arc::new(move || vec![edit_menu_for(&*for_menu, &en())]),
            inner: Arc::new(|_| {}),
            app: Arc::default(),
            after: AfterWrite::default(),
        };
        edit.install(&w).expect("installed");
        assert!(w.choose("app.undo"));
        wakes.0.load(Ordering::SeqCst)
    };
    assert_eq!(wakes_after(Answer::Accepted), 1, "a step taken wakes once");
    assert_eq!(
        wakes_after(Answer::Refused(Reason::new("conflict", ""))),
        0,
        "a refusal wakes nothing"
    );
}
