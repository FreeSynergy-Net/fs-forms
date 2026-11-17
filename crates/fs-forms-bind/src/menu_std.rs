//! The standard menus of the shell as an INPUT of the host (A-P4): read from `standard.rs` of fs-components-commands by the host,
//! never copied into the plan.
//!
//! Pattern: Value Object (input snapshot).

use fs_components_commands::{CommandId, standard};

/// One entry of a standard menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StdEntry {
    /// Name without the `app.` prefix.
    pub name: String,
    /// Stored shortcut text (`Primary+Q`), if the shell binds one.
    pub shortcut: Option<String>,
}

/// The standard menus by name (`program`, `help`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StandardMenus {
    /// Menus in shell order.
    pub menus: Vec<(String, Vec<StdEntry>)>,
}

impl StandardMenus {
    /// The entries of one menu.
    #[must_use]
    pub fn entries(&self, which: &str) -> &[StdEntry] {
        self.menus
            .iter()
            .find(|(w, _)| w == which)
            .map_or(&[], |(_, e)| e.as_slice())
    }

    /// What the host passes: read from the shell's own fixed commands (`standard_commands()`), grouped as its menu bar groups them.
    #[must_use]
    pub fn from_shell() -> Self {
        let cmds = standard::standard_commands();
        let entry = |id: &str| {
            let shortcut = cmds
                .get(&CommandId::new(id))
                .and_then(|c| c.shortcuts.first())
                .map(fs_components_commands::Shortcut::aria);
            StdEntry {
                name: id.trim_start_matches(standard::PREFIX).to_owned(),
                shortcut,
            }
        };
        Self {
            menus: vec![
                (
                    "program".to_owned(),
                    vec![entry(standard::SETTINGS), entry(standard::QUIT)],
                ),
                (
                    "help".to_owned(),
                    vec![entry(standard::HELP), entry(standard::ABOUT)],
                ),
            ],
        }
    }
}
