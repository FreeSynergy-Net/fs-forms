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

/// Keys a description may not bind (owner wins; c0, 18.11.): exactly these, WITHOUT further modifiers: Tab, F1, F6, F10, Escape,
/// Primary+Q (Primary = Ctrl on Linux/Windows, Cmd on the Mac), plus Shift+Tab and Shift+F6 (backwards, for accessibility). Shift+F1, Ctrl+F6 and the like are free. `delete` is
/// NOT reserved: it is an ordinary command that runs only in a list. The list exists ONCE: [`RESERVED`]; this function and the
/// stored `KeyPlan.reserved` both follow it. Until the commands crate carries the constant, this is the local copy.
#[must_use]
pub fn reserved(sc: &Shortcut, platform: Platform) -> bool {
    // `Primary` is Ctrl on Linux and Windows and Cmd (Meta) on the Mac; every other modifier counts as itself, so
    // Ctrl+Meta+Q is free everywhere and Meta+Q is reserved on the Mac only.
    RESERVED
        .iter()
        .filter_map(|t| Shortcut::parse(t).ok())
        .any(|r| r.same_as(sc, platform))
}

/// The one list of reserved keys, as text.
pub const RESERVED: [&str; 8] = [
    "Tab",
    "Shift+Tab",
    "F1",
    "F6",
    "Shift+F6",
    "F10",
    "Escape",
    "Primary+Q",
];

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
            reserved: RESERVED.iter().map(|s| (*s).to_owned()).collect(),
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
