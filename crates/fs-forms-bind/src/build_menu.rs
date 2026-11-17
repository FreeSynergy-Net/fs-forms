//! Step 6 of `Bind::build`: the menu tree, standard menus, shortcuts (invalid, reserved, conflicts).
//!
//! Pattern: Builder (recursive Composite walk). Shortcuts and conflicts come from `fs-components-commands`, not from a copy.

use fs_components_commands::{Command, Commands, Platform, Shortcut, conflicts};

use crate::build::Pool;
use crate::codes::{BindCode, Part, PartLock};
use crate::commands::command_plan;
use crate::keys::reserved;
use crate::menu::{MenuEntry, MenuNode, MenuPlan};
use crate::menu_std::StandardMenus;
use crate::model::{MenuIn, MenuKind};
use crate::plan::SourcePlan;
use crate::providers::HostCaps;

struct Walk<'a> {
    sources: &'a [SourcePlan],
    host: HostCaps,
    locks: Vec<PartLock>,
    keys: Vec<(String, Shortcut, String)>,
    standards: Vec<String>,
}

pub(crate) struct MenuCtx<'a> {
    pub sources: &'a [SourcePlan],
    pub host: HostCaps,
    pub platform: Platform,
    pub standard: &'a StandardMenus,
}

pub(crate) fn build_menu(menu: &[MenuIn], ctx: &MenuCtx<'_>, pool: &mut Pool<'_>) -> MenuPlan {
    let mut w = Walk {
        sources: ctx.sources,
        host: ctx.host,
        locks: Vec::new(),
        keys: Vec::new(),
        standards: Vec::new(),
    };
    let roots = w.nodes(menu, pool);
    // The standard shortcuts of the shell take part in the conflict check as commands of their own.
    let mut all = Commands::new();
    for (name, sc, _) in &w.keys {
        all = all.with(Command::new(name.as_str(), name.as_str()).shortcut(sc.clone()));
    }
    for which in &w.standards {
        for e in ctx.standard.entries(which) {
            if let Some(sc) = e.shortcut.as_deref().and_then(|t| Shortcut::parse(t).ok()) {
                let id = format!("app.{}", e.name);
                all = all.with(Command::new(id.as_str(), id.as_str()).shortcut(sc));
            }
        }
    }
    for c in conflicts(&all, ctx.platform) {
        let ids: Vec<&str> = c
            .commands
            .iter()
            .map(fs_components_commands::CommandId::as_str)
            .collect();
        let elem = w
            .keys
            .iter()
            .find(|(n, _, _)| ids.contains(&n.as_str()))
            .map(|(_, _, e)| e.clone())
            .unwrap_or_default();
        let code = BindCode::ShortcutConflict
            .coded()
            .with("shortcut", c.shortcut.aria())
            .with("commands", ids.join(","));
        w.locks.push(PartLock::locked(Part::Menu(elem), code));
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
                    let c = BindCode::StandardTwice.coded().with("standard", w.clone());
                    self.locks
                        .push(PartLock::locked(Part::Menu(n.elem.clone()), c));
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
                let id = crate::commands::command_id(&cmd);
                let shortcut = shortcut
                    .as_deref()
                    .and_then(|t| self.shortcut(&n.elem, t, &id));
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

    fn shortcut(&mut self, elem: &str, text: &str, id: &str) -> Option<Shortcut> {
        let part = Part::Menu(elem.to_owned());
        let Ok(sc) = Shortcut::parse(text) else {
            let c = BindCode::ShortcutInvalid
                .coded()
                .with("elem", elem)
                .with("text", text);
            self.locks.push(PartLock::locked(part, c));
            return None;
        };
        if reserved(&sc) {
            let c = BindCode::KeyReserved
                .coded()
                .with("elem", elem)
                .with("shortcut", sc.aria())
                .with("owner", "window");
            self.locks.push(PartLock::locked(part, c));
            return None;
        }
        self.keys.push((id.to_owned(), sc.clone(), elem.to_owned()));
        Some(sc)
    }
}
