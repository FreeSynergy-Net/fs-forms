//! Undo and redo in the window (S3-9 package C): the menu "Edit", the shortcuts and the one function that answers them.
//!
//! Tests: H1 undo and redo reach `step_back(false)` / `step_back(true)` once each; H2 no step = nothing asked, said in the status line;
//! H3 a refusal shows its code in the status line; H4 a foreign id is not answered; H5 the lines are locked at 0 steps and carry the
//! words and the shortcut of the standard commands; H6 a key in a text field, a list or a tree never runs the commands (the board and
//! the text take Ctrl+Z themselves), nowhere in particular it does; H7 the real window: a click on `app.undo` reaches the records once
//! and refreshes the locked lines.
//!
//! Mutants: M1 `answer` calls `step_back` without looking at the depth -> H2; M2 `redo` mapped to `step_back(false)` -> H1; M3 a
//! refusal is dropped -> H3; M4 `keymap` without `only_in(Global)` -> H6.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use fs_components_app::{Status, shell_words};
use fs_components_commands::{FocusContext, KeyPress, Outcome, Platform, Shortcut};
use fs_components_kanban::{Answer, Reason};
use fs_components_words::Locale;
use fs_forms_run::history::{Said, answer, edit_menu, edit_menu_for, keymap, status_of};
use fs_forms_run::{
    Composition, HostState, NoRecords, Node, Records, ScreenTree, SideItem, UndoState, View,
};

/// Records with a history of `undoable`/`redoable` steps that count the calls of `step_back`.
struct Steps {
    state: UndoState,
    back: AtomicUsize,
    forward: AtomicUsize,
    reply: Answer,
}

impl Steps {
    fn new(undoable: usize, redoable: usize, reply: Answer) -> Arc<Self> {
        Arc::new(Self {
            state: UndoState { undoable, redoable },
            back: AtomicUsize::new(0),
            forward: AtomicUsize::new(0),
            reply,
        })
    }
    fn calls(&self) -> (usize, usize) {
        (
            self.back.load(Ordering::SeqCst),
            self.forward.load(Ordering::SeqCst),
        )
    }
}

impl Records for Steps {
    fn side(&self) -> Vec<SideItem> {
        Vec::new()
    }
    fn history(&self) -> UndoState {
        self.state
    }
    fn step_back(&self, redo: bool) -> Answer {
        let n = if redo { &self.forward } else { &self.back };
        n.fetch_add(1, Ordering::SeqCst);
        self.reply.clone()
    }
}

fn en() -> Locale {
    Locale::from_posix("en").expect("a locale")
}

fn press(text: &str) -> KeyPress {
    let sc = Shortcut::parse(text).expect("a readable key");
    KeyPress::new(sc.modifiers(), sc.key()).expect("a key press")
}

#[test]
fn h1_undo_and_redo_reach_step_back_once_each() {
    let r = Steps::new(2, 1, Answer::Accepted);
    assert_eq!(answer(&*r, "app.undo"), Said::Done { redo: false });
    assert_eq!(r.calls(), (1, 0));
    assert_eq!(answer(&*r, "app.redo"), Said::Done { redo: true });
    assert_eq!(r.calls(), (1, 1));
}

#[test]
fn h2_no_step_asks_nothing_and_says_so() {
    let r = Steps::new(0, 0, Answer::Accepted);
    let said = answer(&*r, "app.undo");
    assert_eq!(said, Said::Nothing { redo: false });
    assert_eq!(r.calls(), (0, 0), "step_back is not asked without a step");
    assert_eq!(
        status_of(&said, &en()),
        Some(Status::Info(
            "Nothing to undo (host-notice-undo-nothing)".into()
        ))
    );
    // records that say nothing about a history (the default) are the same
    assert_eq!(answer(&NoRecords, "app.redo"), Said::Nothing { redo: true });
}

#[test]
fn h3_a_refusal_shows_its_code_never_silent() {
    let r = Steps::new(1, 0, Answer::Refused(Reason::new("conflict", "")));
    let said = answer(&*r, "app.undo");
    assert_eq!(
        said,
        Said::Refused {
            redo: false,
            code: "conflict".into()
        }
    );
    let Some(Status::Problem(text)) = status_of(&said, &en()) else {
        panic!("a refusal is a problem line");
    };
    assert!(text.contains("conflict"), "the code is in the line: {text}");
}

#[test]
fn h4_a_foreign_id_is_not_answered() {
    let r = Steps::new(3, 3, Answer::Accepted);
    for id in ["layout.frontend.board", "app.quit", "", "undo"] {
        assert_eq!(answer(&*r, id), Said::NotMine, "{id}");
        assert_eq!(status_of(&Said::NotMine, &en()), None);
    }
    assert_eq!(r.calls(), (0, 0));
}

#[test]
fn h5_the_lines_are_locked_without_a_step_and_use_the_standard_words_and_keys() {
    let loc = en();
    let say = shell_words(&loc);
    let names = fs_components_app::key_names(&loc);
    let menu = |u, r| {
        edit_menu(
            "Edit",
            UndoState {
                undoable: u,
                redoable: r,
            },
            &say,
            Platform::Linux,
            &names,
        )
    };
    let m = menu(0, 1);
    let ids: Vec<&str> = m.entries.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, ["app.undo", "app.redo"]);
    assert_eq!(m.entries[0].label, "Undo", "the shell's word, not the key");
    assert_eq!(m.entries[1].label, "Redo");
    assert!(m.entries[0].disabled && !m.entries[1].disabled);
    let m = menu(2, 0);
    assert!(!m.entries[0].disabled && m.entries[1].disabled);
    let de = Locale::from_posix("de").expect("a locale");
    let m = edit_menu_for(&NoRecords, &de);
    assert_eq!(m.entries[0].label, "Rückgängig");
    assert!(m.entries.iter().all(|e| e.disabled), "no history = locked");
}

#[test]
fn h6_text_list_and_tree_keep_their_keys_nowhere_in_particular_runs_the_commands() {
    let map = keymap(Platform::Linux);
    for context in [
        FocusContext::TextField,
        FocusContext::List,
        FocusContext::Tree,
    ] {
        for key in ["Primary+Z", "Primary+Shift+Z", "Primary+Y"] {
            assert_eq!(
                map.dispatch(&press(&key.replace("Primary", "Control")), context),
                Outcome::NoMatch,
                "{key} in {context:?} is not ours (no second trigger)"
            );
        }
    }
    let run = |key: &str| map.dispatch(&press(key), FocusContext::Global);
    assert_eq!(run("Control+Z"), Outcome::Run("app.undo".into()));
    assert_eq!(run("Control+Shift+Z"), Outcome::Run("app.redo".into()));
    assert_eq!(run("Control+Y"), Outcome::Run("app.redo".into()));
}

fn one_screen() -> Composition {
    Composition {
        state: HostState::Ready,
        screens: vec![ScreenTree {
            page: "main".to_owned(),
            frame: "workbench".to_owned(),
            nodes: vec![Node::Locked {
                place: None,
                codes: Vec::new(),
            }],
        }],
        lacking: Vec::new(),
        notices: Vec::new(),
    }
}

#[test]
fn h7_a_click_in_the_real_window_reaches_the_records_and_says_what_happened() {
    let loc = en();
    let texts = fs_forms_run::words(&loc);
    let r = Steps::new(1, 0, Answer::Accepted);
    let w = fs_forms_run::app_with(&texts, &loc, &View::default(), &one_screen(), r.clone())
        .expect("a window");
    let locked = |w: &fs_components_app::AppWidget| {
        w.with_shell(|s| {
            let edit = s
                .menus()
                .iter()
                .find(|m| m.entries.iter().any(|e| e.id == "app.undo"));
            edit.expect("menu Edit")
                .entries
                .iter()
                .map(|e| e.disabled)
                .collect::<Vec<_>>()
        })
    };
    assert_eq!(locked(&w), [false, true]);
    assert!(w.choose("app.undo"), "the line is there and enabled");
    assert_eq!(r.calls(), (1, 0));
    let said = w.with_shell(|s| s.status().cloned());
    assert_eq!(
        said,
        Some(Status::Info("Undone (host-notice-undo-done)".into()))
    );
    // a refusing record: the code is in the line
    let bad = Steps::new(1, 0, Answer::Refused(Reason::new("conflict", "")));
    let w = fs_forms_run::app_with(&texts, &loc, &View::default(), &one_screen(), bad.clone())
        .expect("a window");
    assert!(w.choose("app.undo"));
    let said = w.with_shell(|s| s.status().cloned());
    let Some(Status::Problem(text)) = said else {
        panic!("a refusal is a problem line")
    };
    assert!(text.contains("conflict"), "{text}");
    // records without a history: the lines are locked, a click does nothing
    let w = fs_forms_run::app_with(
        &texts,
        &loc,
        &View::default(),
        &one_screen(),
        Arc::new(NoRecords),
    )
    .expect("a window");
    assert_eq!(locked(&w), [true, true]);
    assert!(!w.choose("app.undo"), "a locked line does not run");
}
