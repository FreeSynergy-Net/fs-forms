//! The blocks of `fs-components` this host draws as elements of a page, and the data each one is given: `board` is the kanban
//! board, `sidebar` a tree of the side list. The assignment is one table (block name to drawing function); nothing is painted here and nothing is decided here.
//! No operation of the board is connected (no provider), so every move is drawn locked with its hint, never silently.
//!
//! Pattern: Abstract Factory over `fs-components` (block name in, element out), data through the port `Records`.

use std::sync::Arc;

use fs_components::blinc::layout::div::{Div, div};
use fs_components::blinc::theme::{ColorScheme, ThemeState};
use fs_components_fields::{Look, Say};
use fs_components_kanban::{FixedBoard, KanbanBoard};
use fs_components_styles::StyleId;
use fs_components_widgets::{TreeNode, TreeView};

use crate::records::Records;

/// The least height of the board, in pixels.
pub(crate) const MIN_H: f32 = 380.0;

/// What a drawn block needs besides its name.
#[derive(Clone)]
pub(crate) struct Ctx {
    /// The style of the window NOW (asked at every drawing).
    pub style: Arc<dyn Fn() -> StyleId + Send + Sync>,
    pub records: Arc<dyn Records>,
    pub say: Arc<dyn Say>,
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
    let source = Arc::new(FixedBoard {
        columns: ctx.records.columns(),
        ops: Vec::new(),
    });
    let board = KanbanBoard::new(source, ctx.say.clone(), Look::new((ctx.style)(), dark()));
    let height = height.max(MIN_H);
    div().w(width).h(height).child(board.build(width, height))
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
