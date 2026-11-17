//! Step 6 of `Bind::build`: the menu tree, standard menus, shortcuts (invalid, reserved, conflicts).
//!
//! Pattern: Builder (recursive Composite walk).

use crate::build::Pool;
use crate::codes::{BindCode, Part, PartLock};
use crate::commands::command_plan;
use crate::keys::Shortcut;
use crate::menu::{MenuEntry, MenuNode, MenuPlan, standard_entries};
use crate::model::{MenuIn, MenuKind};
use crate::plan::SourcePlan;
use crate::providers::HostCaps;

struct Walk<'a> {
    sources: &'a [SourcePlan],
    host: HostCaps,
    locks: Vec<PartLock>,
    keys: Vec<(String, String, String)>,
    standards: Vec<String>,
}

pub(crate) fn build_menu(
    menu: &[MenuIn],
    sources: &[SourcePlan],
    host: HostCaps,
    pool: &mut Pool<'_>,
) -> MenuPlan {
    let mut w = Walk {
        sources,
        host,
        locks: Vec::new(),
        keys: Vec::new(),
        standards: Vec::new(),
    };
    let roots = w.nodes(menu, pool);
    // Standard shell shortcuts take part in conflicts, as pseudo commands.
    let mut all = w.keys.clone();
    for which in &w.standards {
        for (name, key) in standard_entries(which) {
            if let Some(k) = key.and_then(Shortcut::parse) {
                all.push((k.norm, format!("standard:{name}"), String::new()));
            }
        }
    }
    let mut seen: Vec<&str> = Vec::new();
    for (norm, _, elem) in &w.keys {
        if seen.contains(&norm.as_str()) {
            continue;
        }
        let names: Vec<&str> = all
            .iter()
            .filter(|(n, _, _)| n == norm)
            .map(|(_, c, _)| c.as_str())
            .collect();
        if names.len() > 1 {
            seen.push(norm);
            let c = BindCode::ShortcutConflict
                .coded()
                .with("shortcut", norm.clone())
                .with("commands", names.join(","));
            w.locks.push(PartLock::locked(Part::Menu(elem.clone()), c));
        }
    }
    MenuPlan {
        roots,
        locks: w.locks,
    }
}

impl Walk<'_> {
    fn nodes(&mut self, list: &[MenuIn], pool: &mut Pool<'_>) -> Vec<MenuNode> {
        list.iter().map(|n| self.node(n, pool)).collect()
    }

    fn node(&mut self, n: &MenuIn, pool: &mut Pool<'_>) -> MenuNode {
        for c in pool.take(&n.elem) {
            self.locks
                .push(PartLock::locked(Part::Menu(n.elem.clone()), c));
        }
        let entry = match &n.kind {
            MenuKind::Menu(l) => MenuEntry::Menu {
                label_key: l.clone(),
            },
            MenuKind::Standard(w) => {
                if self.standards.contains(w) {
                    self.locks.push(PartLock::locked(
                        Part::Menu(n.elem.clone()),
                        BindCode::StandardTwice.coded().with("standard", w.clone()),
                    ));
                } else {
                    self.standards.push(w.clone());
                }
                MenuEntry::Standard { which: w.clone() }
            }
            MenuKind::View(v) => MenuEntry::View { view: v.clone() },
            MenuKind::Separator => MenuEntry::Separator,
            MenuKind::Command {
                command,
                on_selected,
                shortcut,
            } => {
                let src_lock = self
                    .sources
                    .get(command.source as usize)
                    .and_then(|s| s.status.as_ref().err());
                let cmd = command_plan(command, self.host, src_lock);
                let shortcut = shortcut
                    .as_deref()
                    .and_then(|t| self.shortcut(&n.elem, t, &cmd.name));
                MenuEntry::Command {
                    command: cmd,
                    on_selected: *on_selected,
                    shortcut,
                }
            }
        };
        MenuNode {
            elem: n.elem.clone(),
            entry,
            children: self.nodes(&n.children, pool),
        }
    }

    fn shortcut(&mut self, elem: &str, text: &str, command: &str) -> Option<Shortcut> {
        let part = Part::Menu(elem.to_owned());
        let Some(sc) = Shortcut::parse(text) else {
            let c = BindCode::ShortcutInvalid
                .coded()
                .with("elem", elem)
                .with("text", text);
            self.locks.push(PartLock::locked(part, c));
            return None;
        };
        if sc.reserved() {
            let c = BindCode::KeyReserved
                .coded()
                .with("elem", elem)
                .with("shortcut", sc.norm.clone())
                .with("owner", "window");
            self.locks.push(PartLock::locked(part, c));
            return None;
        }
        self.keys
            .push((sc.norm.clone(), command.to_owned(), elem.to_owned()));
        Some(sc)
    }
}
