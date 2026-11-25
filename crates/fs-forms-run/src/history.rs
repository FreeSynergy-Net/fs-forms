//! Undo and redo in the window: the menu "Edit" (two lines from the standard commands `app.undo`/`app.redo`), their shortcuts and the
//! one function that answers them with the records of the window. The ids, the words and the shortcuts are the standard ones of
//! `fs-components-commands` (`standard_commands()`); nothing is made up here.
//!
//! Pattern: Command (invoker = the one handler of `set_menus`, which a shortcut reaches too; receiver = [`Records::step_back`]) with a
//! Builder for the menu (the state of the history in, a menu model out; Forms draws it). A refusal is said in the status line with its
//! code, never silent. Deviation: the keys of the text field (`Primary+Z` in a field) and of a list or tree belong to those, so the
//! keymap lets the two commands count only in the context `Global` (the board takes its Ctrl+Z itself, no second trigger).

use std::sync::{Arc, Mutex, PoisonError};

use fs_components_app::{AppError, AppWidget, Menu, MenuEntry, Status, key_names, shell_words};
use fs_components_commands::standard::{REDO, UNDO};
use fs_components_commands::{
    CommandId, Commands, FocusContext, KeyNames, Keymap, Platform, TextKey, standard_commands,
};
use fs_components_kanban::Answer;
use fs_components_words::Locale;
use fs_objects::UndoState;

use crate::after_write::AfterWrite;
use crate::parts::line;
use crate::records::Records;
use crate::repaint::Repaint;
use crate::view::Coded;
use crate::words::words;

/// What the records said to one command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Said {
    /// Not a command of the history: the caller goes on with it.
    NotMine,
    /// The history has no step in that direction; `step_back` was not asked.
    Nothing {
        /// `true` for redo.
        redo: bool,
    },
    /// The step was taken: draw again.
    Done {
        /// `true` for redo.
        redo: bool,
    },
    /// The records refused (or want an answer first); the code is shown.
    Refused {
        /// `true` for redo.
        redo: bool,
        /// The code of the reason.
        code: String,
    },
}

/// The two commands of the history, with their shortcuts and words.
#[must_use]
pub fn commands() -> Commands {
    standard_commands()
        .iter()
        .filter(|c| c.id.as_str() == UNDO || c.id.as_str() == REDO)
        .cloned()
        .fold(Commands::new(), Commands::with)
}

/// The shortcuts of the two commands for `platform`: they count only while the keyboard is nowhere in particular (`Global`).
#[must_use]
pub fn keymap(platform: Platform) -> Keymap {
    [UNDO, REDO]
        .into_iter()
        .map(CommandId::new)
        .fold(Keymap::from_commands(&commands(), platform), |map, id| {
            map.only_in(&id, FocusContext::Global)
        })
}

/// The menu `label` with a line for undo and one for redo; a direction without a step is locked.
#[must_use]
pub fn edit_menu(
    label: &str,
    state: UndoState,
    say: &dyn Fn(&TextKey) -> String,
    platform: Platform,
    names: &KeyNames,
) -> Menu {
    let entries = commands()
        .iter()
        .map(|c| {
            let redo = c.id.as_str() == REDO;
            let mut entry = MenuEntry::new(c.id.as_str(), say(&c.label));
            if let Some(icon) = &c.icon {
                entry = entry.icon(icon);
            }
            if let Some(first) = c.shortcuts.first() {
                let shown = first.display(platform, names);
                entry = entry.shortcut(shown.clone(), shown);
            }
            if steps(state, redo) == 0 {
                entry = entry.disabled();
            }
            entry
        })
        .collect();
    Menu::new(label, entries)
}

fn steps(state: UndoState, redo: bool) -> usize {
    if redo { state.redoable } else { state.undoable }
}

/// The menu "Edit" for the records as they stand now, in `locale`.
#[must_use]
pub fn edit_menu_for(records: &dyn Records, locale: &Locale) -> Menu {
    edit_menu(
        &words(locale).plain("host-menu-edit"),
        records.history(),
        &shell_words(locale),
        Platform::current(),
        &key_names(locale),
    )
}

/// Answers the command `id` with `records`: `app.undo` and `app.redo` take one step back or forward; any other id is `NotMine`.
#[must_use]
pub fn answer(records: &dyn Records, id: &str) -> Said {
    let redo = match id {
        UNDO => false,
        REDO => true,
        _ => return Said::NotMine,
    };
    if steps(records.history(), redo) == 0 {
        return Said::Nothing { redo };
    }
    match records.step_back(redo) {
        Answer::Accepted => Said::Done { redo },
        Answer::Refused(reason) => Said::Refused {
            redo,
            code: reason.code,
        },
        // a step back asks no question; if the records ask one, it is shown as a refusal with its own code, never dropped
        Answer::Ask(_) => Said::Refused {
            redo,
            code: "ask-unsupported".to_owned(),
        },
    }
}

/// The status line for an answer; `None` for `NotMine`.
#[must_use]
pub fn status_of(said: &Said, locale: &Locale) -> Option<Status> {
    let texts = words(locale);
    let pick = |redo: bool, undo_key: &str, redo_key: &str| {
        Coded::new(if redo { redo_key } else { undo_key })
    };
    match said {
        Said::NotMine => None,
        Said::Nothing { redo } => Some(Status::Info(line(
            &texts,
            &pick(
                *redo,
                "host-notice-undo-nothing",
                "host-notice-redo-nothing",
            ),
        ))),
        Said::Done { redo } => Some(Status::Info(line(
            &texts,
            &pick(*redo, "host-notice-undo-done", "host-notice-redo-done"),
        ))),
        Said::Refused { redo, code } => Some(Status::Problem(line(
            &texts,
            &pick(
                *redo,
                "host-notice-undo-refused",
                "host-notice-redo-refused",
            )
            .with("code", code.clone()),
        ))),
    }
}

/// What a window needs to answer the history: its records, its language, the wake-up, all its menus as they are now and the handler
/// of every other line.
#[derive(Clone)]
pub struct Edit {
    /// The records of the window.
    pub records: Arc<dyn Records>,
    /// The language of the window.
    pub locale: Locale,
    /// Wakes the window so it draws again after a step.
    pub repaint: Arc<dyn Repaint>,
    /// All menus of the window as they are now (the program's, the one for the frontends, and `edit_menu_for`).
    pub menus: Arc<dyn Fn() -> Vec<Menu> + Send + Sync>,
    /// The handler of every line that is not undo or redo.
    pub inner: Arc<dyn Fn(&str) + Send + Sync>,
    /// The window the handler answers in (set by [`Edit::install`]). An owner that closes windows gives the cell of an
    /// [`Attached`], so the window is not kept alive by its own handler; a one-window host gives `Arc::default()`.
    pub app: Arc<Mutex<Option<AppWidget>>>,
    /// The hook of the gate of the board ([`crate::draw::Ctx`]); [`Edit::install`] sets it so the lines follow every accepted write.
    pub after: AfterWrite,
}

/// The owner of the cell of a window's handler: when it drops (the window is closed), the handler lets the window go, so the window,
/// its records and its grip at the module are freed and the grip signs off (no cycle window - menu handler - window).
///
/// Pattern: RAII guard.
#[derive(Default)]
pub struct Attached(Arc<Mutex<Option<AppWidget>>>, AfterWrite);

impl Attached {
    /// The cell to put into [`Edit::app`].
    #[must_use]
    pub fn cell(&self) -> Arc<Mutex<Option<AppWidget>>> {
        Arc::clone(&self.0)
    }

    /// The hook to put into [`Edit::after`] and into the drawing context of the pages of this window.
    #[must_use]
    pub fn hook(&self) -> AfterWrite {
        self.1.clone()
    }
}

impl Drop for Attached {
    fn drop(&mut self) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = None;
        self.1.clear();
    }
}

impl Edit {
    /// Puts the menus, the one handler and the shortcuts into `app` (call again to refresh the locked lines) and sets the hook
    /// [`Edit::after`]: after every accepted write of the board the lines are set again, so "Undo" is open after a card was moved.
    ///
    /// # Errors
    /// [`AppError::DuplicateMenuEntry`] when two lines share an id; nothing changes then.
    pub fn install(&self, app: &AppWidget) -> Result<(), AppError> {
        *self.app.lock().unwrap_or_else(PoisonError::into_inner) = Some(app.clone());
        self.menus_into(app)?;
        // the hook holds a copy with a hook of its own that nobody sets (no cycle hook - copy - hook)
        let again = Self {
            after: AfterWrite::default(),
            ..self.clone()
        };
        self.after.set(move || {
            let app = again
                .app
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone();
            if let Some(app) = app {
                // a failure to refresh leaves the old lines; the write itself is done
                let _ = again.menus_into(&app);
            }
        });
        Ok(())
    }

    fn menus_into(&self, app: &AppWidget) -> Result<(), AppError> {
        app.set_menus((self.menus)(), self.handler())?;
        let say = shell_words(&self.locale);
        app.set_shortcuts(
            keymap(Platform::current()),
            move |key: &TextKey| say(key),
            self.locale.clone(),
        );
        Ok(())
    }

    fn handler(&self) -> impl Fn(&str) + Send + Sync + 'static + use<> {
        let edit = self.clone();
        move |chosen: &str| {
            let Some(app) = edit
                .app
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
            else {
                return; // the window is closed
            };
            let said = answer(&*edit.records, chosen);
            let Some(status) = status_of(&said, &edit.locale) else {
                (edit.inner)(chosen);
                return;
            };
            if matches!(said, Said::Done { .. }) {
                edit.repaint.wake();
            }
            // the locked lines follow the new depth of the history; a failure to refresh leaves the old menus, the status still shows
            let _ = edit.menus_into(&app);
            app.set_status(Some(status));
        }
    }
}
