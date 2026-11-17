//! The menu plan: the tree of the description (order `seq`, never sorted, never invented) with commands bound.
//!
//! Pattern: Composite. Entries come only from the description; standard menus are expanded for display only.

use crate::codes::PartLock;
use crate::commands::{Shown, command_id, command_state};
use crate::keys::Shortcut;
use crate::plan::CommandPlan;
use crate::types::RecordView;

/// The menu tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuPlan {
    /// Roots in description order.
    pub roots: Vec<MenuNode>,
    /// Menu-level locks (conflicts, invalid or reserved shortcuts).
    pub locks: Vec<PartLock>,
}

/// A node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuNode {
    /// Element id.
    pub elem: String,
    /// What it is.
    pub entry: MenuEntry,
    /// Sub nodes.
    pub children: Vec<MenuNode>,
}

/// An entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuEntry {
    /// Submenu.
    Menu {
        /// Label key.
        label_key: String,
    },
    /// Standard menu of the shell.
    Standard {
        /// `program` or `help`.
        which: String,
    },
    /// A command.
    Command {
        /// The command.
        command: CommandPlan,
        /// Needs a selection.
        on_selected: bool,
        /// Usable shortcut (invalid or reserved ones are dropped and locked).
        shortcut: Option<Shortcut>,
    },
    /// View switch.
    View {
        /// View name.
        view: String,
    },
    /// Separator.
    Separator,
}

/// One arranged item of the menu for a selection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arranged {
    /// Command id.
    pub id: String,
    /// How it appears (never `Hidden`).
    pub shown: Shown,
}

/// Standard entries of the shell menus. [nicht belegt] against `standard.rs` of fs-components-app; copied from the plan §1.8.
#[must_use]
pub fn standard_entries(which: &str) -> &'static [(&'static str, Option<&'static str>)] {
    match which {
        "program" => &[
            ("settings", None),
            ("language", None),
            ("quit", Some("ctrl+q")),
        ],
        "help" => &[("help", Some("f1")), ("about", None)],
        _ => &[],
    }
}

impl MenuPlan {
    fn walk<'a>(nodes: &'a [MenuNode], out: &mut Vec<&'a MenuEntry>) {
        for n in nodes {
            out.push(&n.entry);
            Self::walk(&n.children, out);
        }
    }

    /// Every entry in tree order.
    #[must_use]
    pub fn entries(&self) -> Vec<&MenuEntry> {
        let mut out = Vec::new();
        Self::walk(&self.roots, &mut out);
        out
    }

    /// Every bound command in tree order.
    #[must_use]
    pub fn commands(&self) -> Vec<&CommandPlan> {
        self.entries()
            .into_iter()
            .filter_map(|e| {
                if let MenuEntry::Command { command, .. } = e {
                    Some(command)
                } else {
                    None
                }
            })
            .collect()
    }

    /// The command entries for a selection: hidden ones are left out.
    #[must_use]
    pub fn arrangement(&self, selection: Option<&RecordView>) -> Vec<Arranged> {
        self.commands()
            .into_iter()
            .map(|c| Arranged {
                id: command_id(c),
                shown: command_state(c, selection),
            })
            .filter(|a| a.shown != Shown::Hidden)
            .collect()
    }

    /// Is the node shown for the selection? A menu whose entries are ALL hidden for lack of right is dropped entirely (c0, question 88);
    /// entries locked by a missing source or "coming" keep it visible.
    #[must_use]
    pub fn node_visible(node: &MenuNode, selection: Option<&RecordView>) -> bool {
        match &node.entry {
            MenuEntry::Command { command, .. } => {
                command_state(command, selection) != Shown::Hidden
            }
            MenuEntry::Menu { .. } if !node.children.is_empty() => node
                .children
                .iter()
                .any(|c| Self::node_visible(c, selection)),
            _ => true,
        }
    }

    /// The labels of the top-level menus visible for the selection (standard menus included).
    #[must_use]
    pub fn visible_roots(&self, selection: Option<&RecordView>) -> Vec<String> {
        let label = |n: &MenuNode| match &n.entry {
            MenuEntry::Menu { label_key } => label_key.clone(),
            MenuEntry::Standard { which } => format!("standard:{which}"),
            _ => n.elem.clone(),
        };
        self.roots
            .iter()
            .filter(|n| Self::node_visible(n, selection))
            .map(label)
            .collect()
    }

    /// The tree as plain words, one line per node, depth by two spaces.
    #[must_use]
    pub fn outline(&self) -> Vec<String> {
        fn go(nodes: &[MenuNode], depth: usize, out: &mut Vec<String>) {
            for n in nodes {
                let pad = "  ".repeat(depth);
                match &n.entry {
                    MenuEntry::Menu { label_key } => out.push(format!("{pad}{label_key}")),
                    MenuEntry::Standard { which } => {
                        out.push(format!("{pad}standard:{which}"));
                        for (name, key) in standard_entries(which) {
                            out.push(format!(
                                "{pad}  {name}{}",
                                key.map_or(String::new(), |k| format!(" [{k}]"))
                            ));
                        }
                    }
                    MenuEntry::Command {
                        command, shortcut, ..
                    } => {
                        let k = shortcut
                            .as_ref()
                            .map_or(String::new(), |s| format!(" [{}]", s.text));
                        out.push(format!("{pad}{}{k}", command.name));
                    }
                    MenuEntry::View { view } => out.push(format!("{pad}view:{view}")),
                    MenuEntry::Separator => out.push(format!("{pad}-")),
                }
                go(&n.children, depth + 1, out);
            }
        }
        let mut out = Vec::new();
        go(&self.roots, 0, &mut out);
        out
    }
}
