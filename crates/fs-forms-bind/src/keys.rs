//! Shortcuts, reserved keys and the keymap of the current selection.
//!
//! The key CHAIN (Ime -> field -> composite -> workbench -> window) is NOT here: it belongs, additively, to
//! `fs-components-commands` (field blocks sit below fs-forms). bind only says which keys a description may not bind and
//! hands the window a keymap whose entries follow `command_state`. The keymap is a narrow mirror until bind may depend on
//! `fs_components_commands::Keymap` (question 91).
//!
//! Pattern: Chain of Responsibility (last link) + Specification.

use crate::commands::{Shown, command_id, command_state};
use crate::menu::{MenuEntry, MenuPlan};
use crate::types::RecordView;

/// A parsed shortcut.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shortcut {
    /// Text as written.
    pub text: String,
    /// Normalised form (`ctrl+shift+n`).
    pub norm: String,
    /// Editing keys of a text field: the shortcut runs only in a list, never while typing.
    pub list_only: bool,
}

const MODS: [&str; 4] = ["ctrl", "alt", "shift", "meta"];
const NAMED: [&str; 14] = [
    "enter",
    "tab",
    "space",
    "escape",
    "delete",
    "backspace",
    "home",
    "end",
    "up",
    "down",
    "left",
    "right",
    "pageup",
    "pagedown",
];
/// Keys of the window and of text input a description may not bind (owner wins).
const RESERVED: [&str; 3] = ["tab", "shift+tab", "f6"];

impl Shortcut {
    /// Parses `ctrl+shift+n`, `f5`, `delete`. `None` = not readable.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let lower = text.trim().to_lowercase();
        let parts: Vec<&str> = lower.split('+').collect();
        let (key, mods) = parts.split_last()?;
        let named = NAMED.contains(key)
            || (key.len() == 1 && key.chars().all(|c| c.is_ascii_alphanumeric()))
            || (key.starts_with('f')
                && key[1..].parse::<u8>().is_ok_and(|n| (1..=12).contains(&n)));
        if !named || mods.iter().any(|m| !MODS.contains(m)) {
            return None;
        }
        let mut sorted: Vec<&str> = MODS.iter().copied().filter(|m| mods.contains(m)).collect();
        sorted.push(key);
        let list_only = mods.is_empty()
            && [
                "delete",
                "backspace",
                "home",
                "end",
                "left",
                "right",
                "space",
                "enter",
            ]
            .contains(key);
        Some(Self {
            text: text.to_owned(),
            norm: sorted.join("+"),
            list_only,
        })
    }

    /// Is the key owned by the window or the text input?
    #[must_use]
    pub fn reserved(&self) -> bool {
        RESERVED.contains(&self.norm.as_str())
    }
}

/// Keys a description may not bind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyPlan {
    /// Reserved keys (normalised).
    pub reserved: Vec<String>,
}

impl Default for KeyPlan {
    fn default() -> Self {
        Self {
            reserved: RESERVED.iter().map(|s| (*s).to_owned()).collect(),
        }
    }
}

/// Where the key was pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    /// Anywhere.
    Global,
    /// In a list or board.
    List,
    /// In a text field.
    TextField,
}

/// The answer of the keymap.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Run the command with this id.
    Run(String),
    /// Known but disabled, with the lock or "coming" as reason.
    Disabled(String),
    /// Nothing bound (also for commands hidden by `may`).
    NoMatch,
    /// Two entries share the key; nothing runs.
    Conflict,
}

#[derive(Clone, Debug)]
struct KeyEntry {
    norm: String,
    list_only: bool,
    id: String,
    state: Result<(), String>,
}

/// The keymap for one selection.
#[derive(Clone, Debug, Default)]
pub struct Keymap {
    entries: Vec<KeyEntry>,
}

impl KeyPlan {
    /// The keymap for the selection: hidden commands are left out (their key does nothing); coming/locked ones answer
    /// `Disabled`; keys that failed `parse` or are reserved never get here.
    #[must_use]
    pub fn keymap_for(&self, menu: &MenuPlan, selection: Option<&RecordView>) -> Keymap {
        let mut entries = Vec::new();
        for entry in menu.entries() {
            let MenuEntry::Command {
                command,
                shortcut: Some(sc),
                ..
            } = entry
            else {
                continue;
            };
            let state = match command_state(command, selection) {
                Shown::Hidden => continue,
                Shown::Enabled => Ok(()),
                Shown::Coming => Err(format!("coming:{}", command.name)),
                Shown::Locked(l) => Err(l.cause.coded().code.clone()),
            };
            entries.push(KeyEntry {
                norm: sc.norm.clone(),
                list_only: sc.list_only,
                id: command_id(command),
                state,
            });
        }
        Keymap { entries }
    }
}

impl Keymap {
    /// What a press does in a focus context.
    #[must_use]
    pub fn dispatch(&self, norm: &str, focus: Focus) -> Outcome {
        let hits: Vec<&KeyEntry> = self
            .entries
            .iter()
            .filter(|e| e.norm == norm && !(e.list_only && focus == Focus::TextField))
            .collect();
        match hits.as_slice() {
            [] => Outcome::NoMatch,
            [one] => match &one.state {
                Ok(()) => Outcome::Run(one.id.clone()),
                Err(why) => Outcome::Disabled(why.clone()),
            },
            _ => Outcome::Conflict,
        }
    }
}
