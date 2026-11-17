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

/// Keys a description may not bind (owner wins). Until the commands crate carries the constant `RESERVED` (decided in the draft,
/// section 13) this is the local copy: Tab and Shift+Tab leave the field, F6 changes the area.
#[must_use]
pub fn reserved(sc: &Shortcut) -> bool {
    let m = sc.modifiers();
    let plain = !(m.primary || m.control || m.alt || m.meta);
    (sc.key() == "Tab" && plain) || sc.key() == "F6"
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
            reserved: ["Tab", "Shift+Tab", "F6"]
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
