//! The menus of a window as data: the menu of the program (what its description says) and the menu `View` with one radio line per
//! frontend of the program (the rows come from the catalog, [`FrontendRow`] of `fs-components-commands`; the line of the chosen one
//! is on). Nothing is drawn here: the shell of `fs-components-app` draws what these lines say.
//!
//! Pattern: Builder (data in, menu model out); the id of a line is the id of its command (`layout.frontend.<frontend>`), so a click
//! reaches the owner as the command `layout.frontend`.

use fs_components_app::{Menu, MenuEntry};
use fs_components_commands::{FRONTEND, FrontendRow, frontend_line_id};
use fs_components_mf2::Mf2Texts;

use crate::view::{MenuItem, MenuState, View};

fn menu_entry(texts: &Mf2Texts, item: &MenuItem) -> Option<MenuEntry> {
    match item.state {
        MenuState::Hidden => None, // rights come before "coming": a command she may not run is not shown at all
        MenuState::Enabled => Some(MenuEntry::new(&item.id, &item.label)),
        MenuState::Coming => Some(
            MenuEntry::new(
                &item.id,
                format!("{} ({})", item.label, texts.plain("host-menu-coming")),
            )
            .disabled(),
        ),
    }
}

/// The menu of the program (what the description says); `None` when it has no line to show.
pub(crate) fn program_menu(texts: &Mf2Texts, view: &View) -> Option<Menu> {
    let entries: Vec<MenuEntry> = view
        .menu
        .iter()
        .filter_map(|m| menu_entry(texts, m))
        .collect();
    (!entries.is_empty()).then(|| Menu::new(texts.plain("host-title"), entries))
}

/// The prefix of the id of a frontend line: what follows is the name of the frontend.
pub(crate) fn line_prefix() -> String {
    format!("{FRONTEND}.")
}

/// The menu `View`: one radio line per row, the row `chosen` on. Fewer than two rows is no choice: no menu.
pub(crate) fn frontend_menu(texts: &Mf2Texts, rows: &[FrontendRow], chosen: &str) -> Option<Menu> {
    (rows.len() > 1).then(|| {
        let entries = rows
            .iter()
            .map(|r| MenuEntry::new(frontend_line_id(r).as_str(), &r.id).radio(r.id == chosen))
            .collect();
        Menu::new(texts.plain("host-menu-view"), entries)
    })
}

/// The menus of a window, in the order of the title bar: the program's, "Edit" (undo, redo), "View".
pub(crate) fn menus_of(
    texts: &Mf2Texts,
    view: &View,
    rows: &[FrontendRow],
    chosen: &str,
    edit: Menu,
) -> Vec<Menu> {
    program_menu(texts, view)
        .into_iter()
        .chain([edit])
        .chain(frontend_menu(texts, rows, chosen))
        .collect()
}
