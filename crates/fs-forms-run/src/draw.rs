//! The blocks of `fs-components` this host draws as elements of a page, and the data each one is given. Which block has which widget is the catalog's (`widget` of the contract); `board` is the kanban
//! board, `sidebar` a tree of the side list, `tree` and `table` draw the forms `tree` and `members` by channel. The assignment is one table keyed by the WIDGET the catalog names (and three compositions of general blocks); nothing is painted here and nothing is decided here.
//! A control of the board is unlocked only when the description has an action for it and the records can do it (see `events`);
//! every other one is drawn locked with its hint, never silently.
//!
//! Pattern: Abstract Factory over `fs-components` (block name in, element out), data through the port `Records`.

use std::sync::{Arc, Mutex};

use fs_components::blinc::layout::div::Div;
use fs_components::blinc::theme::{ColorScheme, ThemeState};
use fs_components_fields::Say;
use fs_components_styles::StyleId;
use fs_components_widgets::dialogs::Labels;
use fs_components_workbenches::people::PeopleWidget;

use crate::events::Ask;
use crate::form_data::FormRow;
use crate::records::Records;
use crate::view::ActionView;

mod adapters;
mod compositions;

use adapters::{board, contact_row, field_card};
use compositions::{sidebar, table, tree};

/// The least height of the board, in pixels.
pub(crate) const MIN_H: f32 = 380.0;

/// The people workbench with the rows it was made from, shared by the clones of a page's context.
pub(crate) type PeopleSlot = Arc<Mutex<Option<(Vec<FormRow>, PeopleWidget)>>>;

/// What a drawn block needs besides its name.
#[derive(Clone)]
pub(crate) struct Ctx {
    /// The style of the window NOW (asked at every drawing).
    pub style: Arc<dyn Fn() -> StyleId + Send + Sync>,
    pub records: Arc<dyn Records>,
    pub say: Arc<dyn Say>,
    /// The `[[action]]`s of the description: only what they name is unlocked on a board.
    pub actions: Arc<Vec<ActionView>>,
    /// The open question to the reader, if any.
    pub ask: Arc<Ask>,
    /// The words of the buttons of a dialog.
    pub labels: Labels,
    /// The people workbench of this page with the rows it was made from: its state (selection, search, "more") lives in the widget, so the widget is kept over a rebuild and made anew only when the rows change.
    pub people: PeopleSlot,
}

/// Whether the reader of this window sees the dark scheme.
fn dark() -> bool {
    ThemeState::try_get().is_some_and(|t| t.scheme() == ColorScheme::Dark)
}

/// The function that draws a block as an element: width and height in, element out.
type Drawing = fn(&Ctx, f32, f32) -> Div;

/// The host's own compositions of the general blocks: the contract names no widget for them (scope `general`), the host puts
/// them together from the records. A block with a widget in the contract is NOT listed here (see [`ADAPTERS`]).
const COMPOSITIONS: &[(&str, Drawing)] = &[("sidebar", sidebar), ("tree", tree), ("table", table)];

/// Which function puts a widget of `fs-components` behind a block. The catalog says which widget a block has (`widget` of the
/// block file, `fs-components-catalog`); this table is keyed by the WIDGET, never by a block, so a block that gets a widget in the
/// contract is drawn as soon as its widget has an adapter here and a block that is renamed or added needs nothing. A widget needs
/// data of its own kind, so the adapter, not a name, says where it takes it from; a widget without an adapter is `not built`
/// (listed by `--list-blocks`), never silently dropped.
const ADAPTERS: &[(&str, Drawing)] = &[
    ("fs-components-kanban", board),
    ("PeopleWidget", contact_row),
    ("FieldCard", field_card),
];

/// The function that draws the block: the host's composition of a general block, or the adapter of the widget the catalog names.
fn drawing(name: &str) -> Option<Drawing> {
    if let Some((_, f)) = COMPOSITIONS.iter().find(|(n, _)| *n == name) {
        return Some(*f);
    }
    let widget = crate::blocks::widget_of(name)?;
    ADAPTERS.iter().find(|(w, _)| *w == widget).map(|(_, f)| *f)
}

/// Whether a function draws the block.
pub(crate) fn has(name: &str) -> bool {
    drawing(name).is_some()
}

/// Why a block of the catalog is not drawn, in words for the list: its widget has no adapter, or the contract names none.
pub(crate) fn missing(name: &str) -> String {
    match crate::blocks::widget_of(name) {
        Some(w) => format!("widget {w} has no adapter"),
        None => "no widget in the contract".to_owned(),
    }
}

/// The block as an element, or `None` when this host has no drawing for it (the caller says so).
pub(crate) fn block(name: &str, ctx: &Ctx, width: f32, height: f32) -> Option<Div> {
    drawing(name).map(|draw| draw(ctx, width, height))
}

#[cfg(test)]
mod tests {
    use super::ADAPTERS;

    #[test]
    fn every_adapter_names_a_widget_of_the_catalog() {
        for (widget, _) in ADAPTERS {
            assert!(
                crate::blocks::names_widget(widget),
                "adapter {widget} matches no widget in the catalog"
            );
        }
    }
}
