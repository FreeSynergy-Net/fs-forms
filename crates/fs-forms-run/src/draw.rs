//! The blocks of `fs-components` this host draws as elements of a page, and the data each one is given: `board` is the kanban
//! board, `sidebar` a tree of the side list. The assignment is one table (block name to drawing function); nothing is painted here and nothing is decided here.
//! A control of the board is unlocked only when the description has an action for it and the records can do it (see `events`);
//! every other one is drawn locked with its hint, never silently.
//!
//! Pattern: Abstract Factory over `fs-components` (block name in, element out), data through the port `Records`.

use std::sync::Arc;

use fs_components::blinc::layout::div::{Div, div};
use fs_components::blinc::theme::{ColorScheme, ThemeState};
use fs_components_fields::{Look, Say};
use fs_components_kanban::KanbanBoard;
use fs_components_styles::StyleId;
use fs_components_widgets::dialogs::{Labels, modal};
use fs_components_widgets::{TreeNode, TreeView};

use crate::events::{Ask, Wiring};
use crate::records::Records;
use crate::view::ActionView;

/// The least height of the board, in pixels.
pub(crate) const MIN_H: f32 = 380.0;

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
}

/// Whether the reader of this window sees the dark scheme.
fn dark() -> bool {
    ThemeState::try_get().is_some_and(|t| t.scheme() == ColorScheme::Dark)
}

/// The function that draws a block as an element: width and height in, element out.
type Drawing = fn(&Ctx, f32, f32) -> Div;

/// Which function draws which block of the catalog. This is an assignment, not a decision: whether a block exists and what it
/// takes is the catalog's (`fs-components-catalog`, see `blocks`); here stands only the component that stands behind its name.
const DRAWINGS: &[(&str, Drawing)] = &[("board", board), ("sidebar", sidebar)];

/// Whether a function draws the block.
pub(crate) fn has(name: &str) -> bool {
    DRAWINGS.iter().any(|(n, _)| *n == name)
}

/// The block as an element, or `None` when this host has no drawing for it (the caller says so).
pub(crate) fn block(name: &str, ctx: &Ctx, width: f32, height: f32) -> Option<Div> {
    DRAWINGS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, draw)| draw(ctx, width, height))
}

fn board(ctx: &Ctx, width: f32, height: f32) -> Div {
    let wiring = Arc::new(Wiring {
        actions: ctx.actions.clone(),
        records: ctx.records.clone(),
        ask: ctx.ask.clone(),
        say: ctx.say.clone(),
        labels: ctx.labels.clone(),
    });
    let board = KanbanBoard::new(
        wiring.clone(),
        ctx.say.clone(),
        Look::new((ctx.style)(), dark()),
    )
    .events(wiring);
    let height = height.max(MIN_H);
    let ask = ctx.ask.clone();
    // the question to the reader (a column name) stands over the board while it is open
    let area = ctx.ask.live.region("ask", move || {
        let page = board.build(width, height);
        match ask.current() {
            Some(dialog) => {
                dialog.focus();
                modal(page, dialog.build())
            }
            None => div().w(width).h(height).child(page),
        }
    });
    div().w(width).h(height).child(area)
}

fn sidebar(ctx: &Ctx, width: f32, _height: f32) -> Div {
    let nodes: Vec<TreeNode> = ctx
        .records
        .side()
        .into_iter()
        .map(|i| TreeNode::new(i.key, i.label))
        .collect();
    div().w(width).child(TreeView::new(nodes).build())
}
