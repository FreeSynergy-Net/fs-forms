//! A checked description becomes the [`View`] of the host: pages with their placements in order, the program menu, the title.
//! No provider is asked in this version (the catalog and the providers of `fs-forms-bind` are not wired), so no part is locked
//! for a data reason and the window carries ONE notice that says so; nothing is made to look connected.
//!
//! Pattern: Anti-corruption mirror (the model of the layout in, the narrow view out).

use fs_forms_layout::{ActionTarget, Item, Layout, MenuDoc, MenuKind, Trigger};

use crate::view::{ActionView, Coded, MenuItem, MenuState, PageView, PlaceView, View};

/// The text of an MF2 key in `lang`, else in English, else the key itself (a visible gap, not a blank).
fn text(layout: &Layout, key: &str, lang: &str) -> String {
    let find = |l: &str| {
        layout
            .doc()
            .messages
            .iter()
            .find(|m| m.v.key == key && m.v.lang == l)
            .map(|m| m.v.message.clone())
    };
    find(lang)
        .or_else(|| find("en"))
        .unwrap_or_else(|| key.to_owned())
}

fn menu_items(layout: &Layout, entries: &[Item<MenuDoc>], lang: &str, out: &mut Vec<MenuItem>) {
    for e in entries {
        match e.v.kind {
            MenuKind::Command => {
                if let Some((_, name)) = &e.v.command {
                    out.push(MenuItem {
                        id: name.clone(),
                        label: name.clone(),
                        // no provider is connected: the command is there but does not run yet
                        state: MenuState::Coming,
                    });
                }
            }
            MenuKind::View => {
                if let Some(view) = &e.v.view {
                    let label =
                        e.v.label_key
                            .as_deref()
                            .map_or_else(|| view.clone(), |k| text(layout, k, lang));
                    out.push(MenuItem {
                        id: view.clone(),
                        label,
                        state: MenuState::Coming,
                    });
                }
            }
            _ => {}
        }
        menu_items(layout, &e.v.children, lang, out);
    }
}

fn trigger_name(t: Trigger) -> &'static str {
    match t {
        Trigger::Button => "button",
        Trigger::Menu => "menu",
        Trigger::Key => "key",
        Trigger::Drop => "drop",
        Trigger::QuickAdd => "quick-add",
    }
}

/// The view of `layout` for a reader of `lang`.
#[must_use]
pub fn view_of(layout: &Layout, lang: &str) -> View {
    let doc = layout.doc();
    let pages = doc
        .pages
        .iter()
        .zip(0u32..)
        .map(|(p, seq)| PageView {
            page: p.v.page.clone(),
            seq,
            frame: p.v.frame.clone(),
            locks: Vec::new(),
            places: p
                .v
                .blocks
                .iter()
                .zip(0u32..)
                .map(|(b, seq)| PlaceView {
                    place: b.v.place.clone(),
                    seq,
                    block: b.v.block.clone(),
                    title: b.v.title_key.as_deref().map(|k| text(layout, k, lang)),
                    locks: Vec::new(),
                    slot_locks: Vec::new(),
                })
                .collect(),
        })
        .collect();
    let mut menu = Vec::new();
    menu_items(layout, &doc.menu, lang, &mut menu);
    let mut notices = vec![Coded::new("host-notice-no-providers")];
    if let Some(name) = crate::style::unknown(doc.style.as_deref()) {
        // said, never silent: the window then uses the style set for the program
        notices.push(Coded::new("host-notice-style-unknown").with("style", name));
    }
    let actions = doc
        .actions
        .iter()
        .filter_map(|a| match &a.v.target {
            ActionTarget::Command { command, .. } => Some(ActionView {
                trigger: trigger_name(a.v.trigger).to_owned(),
                command: command.clone(),
            }),
            ActionTarget::Workflow(_) => None,
        })
        .collect();
    View {
        actions,
        sources: doc.sources.iter().map(|s| s.v.range.clone()).collect(),
        frontend: doc.key.frontend.clone(),
        title: text(layout, &doc.title_key, lang),
        style: doc.style.clone(),
        pages,
        menu,
        notices,
    }
}
