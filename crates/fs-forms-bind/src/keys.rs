//! Reserved keys and the keymap of the current selection, on the REAL `fs_components_commands::Keymap` (A-P3).
//!
//! The key CHAIN (Ime, field, composite, workbench, window) is not here: it is an additive extension of
//! `fs-components-commands`. bind only says which keys a description may not bind and hands the window a keymap whose entries
//! follow `command_state`.
//!
//! Pattern: Adapter (plan commands to the commands crate) + Specification (reserved, list-only).

use fs_components_commands::{Command, CommandId, Commands, Keymap, Platform, Shortcut};

use crate::commands::{Shown, command_id, command_state};
use crate::menu::{MenuEntry, MenuPlan};
use crate::types::RecordView;

/// Keys a description may not bind (owner wins; c0, 18.11.): the global, fixed shortcuts Primary+Q (quit), F1 (help), F6 (area
/// change), F10 (menu), Escape, plus Tab and Shift+Tab (focus). `delete` is NOT reserved: it is an ordinary command that runs only in a list.
/// Until the commands crate carries the one constant `RESERVED` this is the local copy.
#[must_use]
pub fn reserved(sc: &Shortcut) -> bool {
    let m = sc.modifiers();
    let plain = !(m.primary || m.control || m.alt || m.meta);
    let key = sc.key();
    (key == "Tab" && plain)
        || key == "F6"
        || (["F1", "F10", "Escape"].contains(&key) && plain)
        || ((m.primary || m.control || m.meta) && key == "Q")
}

/// Editing keys of a text field: bound without a modifier they run only in a list, never while typing.
#[must_use]
pub fn list_only(sc: &Shortcut) -> bool {
    let m = sc.modifiers();
    let none = !(m.primary || m.control || m.alt || m.shift || m.meta);
    none && [
        "Delete",
        "Backspace",
        "Home",
        "End",
        "ArrowLeft",
        "ArrowRight",
        "Space",
        "Enter",
    ]
    .contains(&sc.key())
}

/// Keys the description may not bind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyPlan {
    /// Reserved keys as stored text.
    pub reserved: Vec<String>,
}

impl Default for KeyPlan {
    fn default() -> Self {
        Self {
            reserved: ["Tab", "Shift+Tab", "F6", "F1", "F10", "Escape", "Primary+Q"]
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
        }
    }
}

impl KeyPlan {
    /// The keymap for the selection: hidden commands are left out (their key does nothing); coming and locked ones answer
    /// `Disabled` with the reason; editing keys run only in a list.
    #[must_use]
    pub fn keymap_for(
        &self,
        menu: &MenuPlan,
        selection: Option<&RecordView>,
        platform: Platform,
    ) -> Keymap {
        let mut commands = Commands::new();
        let mut scoped: Vec<CommandId> = Vec::new();
        let clashing = clashing_ids(menu, platform);
        for entry in menu.entries() {
            let MenuEntry::Command {
                command,
                shortcut: Some(sc),
                ..
            } = entry
            else {
                continue;
            };
            let id = command_id(command);
            let base = Command::new(id.as_str(), id.as_str()).shortcut(sc.clone());
            let cmd = match command_state(command, selection) {
                // Hidden by right but sharing its key with another entry: the clash stays a clash, so the other one does
                // not run alone (c0, K6: no silent hiding of a conflict).
                Shown::Hidden if clashing.iter().any(|c| c == id.as_str()) => {
                    base.disabled("bind-error-shortcut-conflict")
                }
                Shown::Hidden => continue,
                Shown::Enabled => base,
                Shown::Coming => base.unsupported("bind-error-coming"),
                Shown::Locked(l) => base.disabled(l.cause.coded().code.as_str()),
            };
            if list_only(sc) {
                scoped.push(cmd.id.clone());
            }
            commands = commands.with(cmd);
        }
        let mut map = Keymap::from_commands(&commands, platform);
        for id in &scoped {
            map = map.only_in(id, fs_components_commands::FocusContext::List);
        }
        map
    }
}

/// Ids of the commands whose shortcut is shared with another entry of the menu (all entries, hidden ones too).
fn clashing_ids(menu: &MenuPlan, platform: Platform) -> Vec<String> {
    let mut all = Commands::new();
    for entry in menu.entries() {
        if let MenuEntry::Command {
            command,
            shortcut: Some(sc),
            ..
        } = entry
        {
            let id = command_id(command);
            all = all.with(Command::new(id.as_str(), id.as_str()).shortcut(sc.clone()));
        }
    }
    fs_components_commands::conflicts(&all, platform)
        .into_iter()
        .flat_map(|c| c.commands)
        .map(|id| id.as_str().to_owned())
        .collect()
}
